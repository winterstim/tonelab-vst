import { EFFECTS_METADATA } from '../config/effects';
import { buildApiUrl, getWebBaseUrl } from '../config/runtime';
import { openExternalUrl } from '../utils/externalNavigation';

const ACCESS_TOKEN_STORAGE_KEY = 'tonelab.api.access_token';
const REFRESH_TOKEN_STORAGE_KEY = 'tonelab.api.refresh_token';

const AUTH_TIMEOUT_MS = 300000;
const AUTH_POLL_INTERVAL_MS = 1500;
const REQUEST_TIMEOUT_MS = 45000;
const NETWORK_RETRY_DELAY_MS = 700;
const NETWORK_MAX_ATTEMPTS = 2;

const EFFECT_ALIASES = {
    overdrive: 'Overdrive',
    distortion: 'Overdrive',
    amp_sim: 'Overdrive',
    amp: 'Overdrive',
    delay: 'Delay',
    echo: 'Delay',
    reverb: 'Reverb',
    hall: 'Reverb',
    noisegate: 'NoiseGate',
    noise_gate: 'NoiseGate',
    gate: 'NoiseGate',
    compressor: 'NoiseGate',
    eq: 'Equalizer',
    threebandeq: 'Equalizer',
    three_band_eq: 'Equalizer',
    equalizer: 'Equalizer'
};

const PARAM_ALIASES = {
    Overdrive: {
        amount: 'drive',
        distortion: 'drive',
        gain: 'output_gain',
        level: 'output_gain',
        output: 'output_gain',
        volume: 'output_gain'
    },
    Delay: {
        time: 'time_ms',
        delay: 'time_ms',
        delay_ms: 'time_ms',
        repeats: 'feedback',
        wet: 'mix'
    },
    Reverb: {
        size: 'room_size',
        decay: 'room_size',
        pre_delay: 'pre_delay_ms',
        predelay: 'pre_delay_ms',
        wet: 'mix'
    },
    NoiseGate: {
        threshold: 'threshold_db',
        attack: 'attack_ms',
        release: 'release_ms',
        decay: 'release_ms'
    },
    Equalizer: {
        bass: 'low_gain',
        low: 'low_gain',
        mids: 'mid_gain',
        mid: 'mid_gain',
        treble: 'high_gain',
        high: 'high_gain'
    }
};

function getStorage() {
    if (typeof window === 'undefined') return null;

    try {
        return window.localStorage || null;
    } catch {
        return null;
    }
}

function throwIfAborted(signal) {
    if (signal?.aborted) {
        throw new DOMException('Operation aborted', 'AbortError');
    }
}

function createProcessId() {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
        try {
            return crypto.randomUUID();
        } catch {
            // Some embedded WebView contexts expose randomUUID but block it as "insecure".
        }
    }
    return `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
}

function sleep(ms, signal) {
    return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
            cleanup();
            resolve();
        }, ms);

        const onAbort = () => {
            cleanup();
            reject(new DOMException('Operation aborted', 'AbortError'));
        };

        const cleanup = () => {
            clearTimeout(timer);
            if (signal) {
                signal.removeEventListener('abort', onAbort);
            }
        };

        if (signal) {
            signal.addEventListener('abort', onAbort, { once: true });
        }
    });
}

async function fetchWithTimeout(url, options = {}, signal, timeoutMs = REQUEST_TIMEOUT_MS) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);

    const onAbort = () => controller.abort();
    if (signal) {
        signal.addEventListener('abort', onAbort, { once: true });
    }

    try {
        return await fetch(url, {
            ...options,
            signal: controller.signal
        });
    } catch (error) {
        if (controller.signal.aborted && !signal?.aborted) {
            const timeoutError = new Error(`Request timed out after ${Math.round(timeoutMs / 1000)}s`);
            timeoutError.code = 'REQUEST_TIMEOUT';
            throw timeoutError;
        }
        throw error;
    } finally {
        clearTimeout(timeout);
        if (signal) {
            signal.removeEventListener('abort', onAbort);
        }
    }
}

function isRetryableNetworkError(error) {
    if (!error) return false;
    if (error.name === 'TypeError') return true;

    const message = String(error.message || '').toLowerCase();
    return (
        message.includes('failed to fetch') ||
        message.includes('network') ||
        message.includes('empty_response') ||
        message.includes('timed out')
    );
}

async function parseJsonResponse(response) {
    const text = await response.text();
    if (!text) return null;

    try {
        return JSON.parse(text);
    } catch {
        return text;
    }
}

async function extractErrorMessage(response) {
    const payload = await parseJsonResponse(response);
    if (payload && typeof payload === 'object' && typeof payload.error === 'string') {
        return payload.error;
    }
    if (typeof payload === 'string' && payload.trim()) {
        return payload.trim();
    }
    return `Request failed with status ${response.status}`;
}

function openSystemBrowser(url) {
    openExternalUrl(url);
}

function getStoredTokens() {
    // Priority: Saved token injected from Rust (via init script)
    // This bypasses localStorage reliability issues in some WebViews
    if (typeof window !== 'undefined' && window.RUST_AUTH_TOKEN) {
        return {
            accessToken: window.RUST_AUTH_TOKEN,
            refreshToken: ''
        };
    }

    const storage = getStorage();
    if (!storage) {
        return { accessToken: '', refreshToken: '' };
    }

    const accessToken = storage.getItem(ACCESS_TOKEN_STORAGE_KEY) || '';
    const refreshToken = storage.getItem(REFRESH_TOKEN_STORAGE_KEY) || '';
    return { accessToken, refreshToken };
}

function setStoredTokens(accessToken, refreshToken = '') {
    const token = accessToken || '';
    remoteLog(`setStoredTokens called. Token length: ${token.length}`);

    // ALWAYS update local memory cache first
    if (typeof window !== 'undefined') {
        window.RUST_AUTH_TOKEN = token;
    }

    // 1. Sync with Rust backend (robust persistence)
    if (typeof window !== 'undefined' && window.ipc && window.ipc.postMessage) {
        remoteLog('Sending save_token IPC message...');
        window.ipc.postMessage(JSON.stringify({
            type: 'save_token',
            token: token
        }));
    }

    // 2. Backup to localStorage (browser/dev mode AND VST fallback)
    const storage = getStorage();
    if (storage) {
        try {
            storage.setItem(ACCESS_TOKEN_STORAGE_KEY, token);
            storage.setItem(REFRESH_TOKEN_STORAGE_KEY, refreshToken || '');
        } catch (e) {
            console.warn('[tonelabApi] Failed to write localStorage', e);
        }
    }
}

export function clearStoredTokens() {
    // Clear Rust backend token
    setStoredTokens('', '');

    // Clear localStorage
    const storage = getStorage();
    if (storage) {
        storage.removeItem(ACCESS_TOKEN_STORAGE_KEY);
        storage.removeItem(REFRESH_TOKEN_STORAGE_KEY);
    }
}

function remoteLog(msg) {
    // Send log to Rust backend for file logging (bypass console issues)
    if (typeof window !== 'undefined' && window.ipc && window.ipc.postMessage) {
        window.ipc.postMessage(JSON.stringify({ type: 'log', message: msg }));
    }

}

export async function authorizeDesktopUser({ signal, onStatus } = {}) {
    throwIfAborted(signal);

    const processId = createProcessId();
    const authUrl = `${getWebBaseUrl()}/auth?process_id=${encodeURIComponent(processId)}`;

    remoteLog(`Starting desktop auth. Process ID: ${processId}`);
    remoteLog(`WebView Origin: ${window.location.origin}`);

    onStatus?.('Opening browser for sign in...');
    openSystemBrowser(authUrl);

    onStatus?.('Waiting for sign in confirmation...');
    const startedAt = Date.now();
    remoteLog('Starting polling loop...');

    while (Date.now() - startedAt < AUTH_TIMEOUT_MS) {
        throwIfAborted(signal);

        try {
            const response = await fetchWithTimeout(
                buildApiUrl('/auth/desktop/token'),
                {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        Accept: 'application/json'
                    },
                    body: JSON.stringify({ process_id: processId }),
                    credentials: 'omit'
                },
                signal
            );

            if (response.status === 404) {
                // legitimate pending state
                await sleep(AUTH_POLL_INTERVAL_MS, signal);
                continue;
            }

            if (!response.ok) {
                console.warn('[tonelabApi] Polling error status:', response.status);
                const message = await extractErrorMessage(response);
                throw new Error(`Authorization error (${response.status}): ${message}`);
            }

            const payload = await parseJsonResponse(response);
            remoteLog('Token received successfully from polling.');

            const accessToken = payload?.access_token;
            if (!accessToken) {
                console.error('[tonelabApi] No access_token in payload:', payload);
                throw new Error('Authorization error: access_token not found');
            }

            const refreshToken = payload?.refresh_token || '';
            setStoredTokens(accessToken, refreshToken);
            onStatus?.('Authorization complete');
            return accessToken;

        } catch (pollError) {
            if (pollError?.name === 'AbortError') throw pollError;
            // If 404/pending, we continue. But if fetch failed (network), we log it.
            console.error('[tonelabApi] Polling iteration failed:', pollError);

            // If it's a network error (e.g. Failed to fetch), we might want to retry rather than crash immediately,
            // but for now let's just rethrow if it's not a known safe error.
            if (pollError.message.includes('Token not found')) {
                // Should have been caught by 404 check above, but just in case
                await sleep(AUTH_POLL_INTERVAL_MS, signal);
                continue;
            }
            throw pollError;
        }
    }

    throw new Error('Authorization timed out. Please try again.');
}

async function ensureAccessToken({ signal, onStatus } = {}) {
    const { accessToken } = getStoredTokens();
    if (accessToken) return accessToken;
    return authorizeDesktopUser({ signal, onStatus });
}

function toNumber(value) {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return value;
    }
    if (value && typeof value === 'object') {
        const nestedCandidates = [value.value, value.amount, value.number];
        for (const candidate of nestedCandidates) {
            const nested = toNumber(candidate);
            if (nested !== null) return nested;
        }
        return null;
    }
    if (typeof value === 'string' && value.trim() !== '') {
        const normalized = value.trim().replace(',', '.');
        const parsed = Number(normalized);
        if (Number.isFinite(parsed)) return parsed;

        // Accept values like "-6 dB", "45%", "320 ms"
        const match = normalized.match(/[-+]?\d*\.?\d+/);
        if (match) {
            const extracted = Number(match[0]);
            if (Number.isFinite(extracted)) return extracted;
        }
    }
    return null;
}

function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
}

function normalizeEffectType(rawType) {
    if (typeof rawType !== 'string' || !rawType.trim()) return null;

    const direct = rawType.trim();
    if (EFFECTS_METADATA[direct]) return direct;

    const directInsensitive = Object.keys(EFFECTS_METADATA).find(
        key => key.toLowerCase() === direct.toLowerCase()
    );
    if (directInsensitive) return directInsensitive;

    const aliasKey = direct.toLowerCase().replace(/[\s-]+/g, '_');
    return EFFECT_ALIASES[aliasKey] || null;
}

function normalizeParamKey(effectType, rawKey) {
    const effectConfig = EFFECTS_METADATA[effectType];
    if (!effectConfig || typeof rawKey !== 'string') return null;

    if (effectConfig.params[rawKey]) return rawKey;

    const keyInsensitive = Object.keys(effectConfig.params).find(
        key => key.toLowerCase() === rawKey.toLowerCase()
    );
    if (keyInsensitive) return keyInsensitive;

    const aliases = PARAM_ALIASES[effectType] || {};
    const aliasKey = rawKey.toLowerCase().replace(/[\s-]+/g, '_');
    if (aliases[aliasKey]) return aliases[aliasKey];

    const compactRaw = rawKey.toLowerCase().replace(/[^a-z0-9]/g, '');
    const normalizedMatch = Object.keys(effectConfig.params).find((key) => {
        const compactKey = key.toLowerCase().replace(/[^a-z0-9]/g, '');
        return compactKey === compactRaw;
    });

    return normalizedMatch || null;
}

function transformParamValue(effectType, paramKey, value, paramConfig) {
    let normalized = value;

    if (paramConfig.max <= 1 && normalized > 1 && normalized <= 100) {
        normalized = normalized / 100;
    }

    if (effectType === 'Delay' && paramKey === 'time_ms' && normalized > 0 && normalized <= 10) {
        normalized = normalized * 1000;
    }

    if (effectType === 'Reverb' && paramKey === 'room_size' && normalized > 1) {
        normalized = normalized / 5;
    }

    return clamp(normalized, paramConfig.min, paramConfig.max);
}

function normalizeParams(effectType, rawParams) {
    const effectConfig = EFFECTS_METADATA[effectType];
    if (!effectConfig) {
        return {
            params: {},
            incomingCount: 0,
            appliedCount: 0,
            unknownKeys: [],
            nonNumericKeys: []
        };
    }

    const params = {};
    let incomingCount = 0;
    let appliedCount = 0;
    const unknownKeys = [];
    const nonNumericKeys = [];

    Object.entries(effectConfig.params).forEach(([key, config]) => {
        params[key] = config.default;
    });

    if (!rawParams || typeof rawParams !== 'object') {
        return {
            params,
            incomingCount,
            appliedCount,
            unknownKeys,
            nonNumericKeys
        };
    }

    Object.entries(rawParams).forEach(([rawKey, rawValue]) => {
        incomingCount += 1;

        const paramKey = normalizeParamKey(effectType, rawKey);
        if (!paramKey) {
            unknownKeys.push(rawKey);
            return;
        }

        const paramConfig = effectConfig.params[paramKey];
        if (!paramConfig) {
            unknownKeys.push(rawKey);
            return;
        }

        const numeric = toNumber(rawValue);
        if (numeric === null) {
            nonNumericKeys.push(rawKey);
            return;
        }

        params[paramKey] = transformParamValue(effectType, paramKey, numeric, paramConfig);
        appliedCount += 1;
    });

    return {
        params,
        incomingCount,
        appliedCount,
        unknownKeys,
        nonNumericKeys
    };
}

function extractRawChain(payload) {
    if (typeof payload === 'string') {
        try {
            return extractRawChain(JSON.parse(payload));
        } catch {
            return [];
        }
    }

    if (Array.isArray(payload)) return payload;
    if (!payload || typeof payload !== 'object') return [];

    if (Array.isArray(payload.chain)) return payload.chain;
    if (Array.isArray(payload.data)) return payload.data;
    if (Array.isArray(payload.data?.chain)) return payload.data.chain;
    if (Array.isArray(payload.data?.effects)) return payload.data.effects;
    if (Array.isArray(payload.effects)) return payload.effects;

    return [];
}

function normalizeChainPayload(payload) {
    const rawChain = extractRawChain(payload);
    if (!Array.isArray(rawChain)) {
        return {
            chain: [],
            issues: ['Payload does not contain a chain array']
        };
    }

    const normalizedChain = [];
    const issues = [];

    rawChain.forEach((item, index) => {
        if (!item || typeof item !== 'object') return;

        const effectType = normalizeEffectType(item.type || item.id || item.effect || item.name);
        if (!effectType) {
            issues.push(`effect[${index}]: unsupported effect type`);
            return;
        }

        const rawParams = item.params || item.parameters || item.config || null;
        const rawParamCount =
            rawParams && typeof rawParams === 'object' ? Object.keys(rawParams).length : 0;

        const normalizedParams = normalizeParams(
            effectType,
            rawParams
        );

        if (rawParamCount === 0) {
            issues.push(`effect[${index}] ${effectType}: params missing in AI response`);
        } else if (normalizedParams.appliedCount === 0) {
            issues.push(
                `effect[${index}] ${effectType}: no usable params (unknown: ${normalizedParams.unknownKeys.join(', ') || 'none'}, non-numeric: ${normalizedParams.nonNumericKeys.join(', ') || 'none'})`
            );
        }

        normalizedChain.push({
            type: effectType,
            params: normalizedParams.params
        });
    });

    return {
        chain: normalizedChain,
        issues
    };
}

async function requestAiChain(prompt, accessToken, signal) {
    const apiUrl = buildApiUrl('/vst/ai/generate-chain');

    remoteLog(`[AI] Request URL: ${apiUrl}`);
    remoteLog(`[AI] Origin: ${typeof window !== 'undefined' ? window.location.origin : 'unknown'}`);
    let response = null;

    for (let attempt = 1; attempt <= NETWORK_MAX_ATTEMPTS; attempt += 1) {
        try {
            response = await fetchWithTimeout(
                apiUrl,
                {
                    method: 'POST',
                    headers: {
                        Authorization: `Bearer ${accessToken}`,
                        'Content-Type': 'application/json',
                        Accept: 'application/json'
                    },
                    body: JSON.stringify({ prompt }),
                    credentials: 'omit',
                    redirect: 'manual'
                },
                signal
            );
            break;
        } catch (error) {
            if (error?.name === 'AbortError') {
                throw error;
            }

            const retryable = isRetryableNetworkError(error);
            if (!retryable || attempt >= NETWORK_MAX_ATTEMPTS) {
                throw new Error(`Network error while contacting AI API: ${error?.message || 'unknown error'}`);
            }

            remoteLog(`[AI] Network error on attempt ${attempt}: ${error?.message || 'unknown error'}. Retrying...`);
            await sleep(NETWORK_RETRY_DELAY_MS, signal);
        }
    }

    if (!response) {
        throw new Error('Network error while contacting AI API: no response');
    }

    if (response.type === 'opaqueredirect' || response.status === 307 || response.status === 401) {
        const authError = new Error('Re-authorization required');
        authError.code = 'AUTH_REQUIRED';
        throw authError;
    }

    if (!response.ok) {
        const message = await extractErrorMessage(response);
        if (response.status === 403) {
            const pricingUrl = 'https://www.tonelab.dev/pricing';
            openExternalUrl(pricingUrl);
            throw new Error(message || 'Active Tonelab AI subscription required. Redirecting to pricing...');
        }
        throw new Error(`AI endpoint error: ${message}`);
    }

    const payload = await parseJsonResponse(response);
    const normalized = normalizeChainPayload(payload);

    remoteLog(`[AI] Raw chain payload type: ${Array.isArray(payload) ? 'array' : typeof payload}`);
    remoteLog(`[AI] Normalized effects: ${normalized.chain.length}, issues: ${normalized.issues.length}`);

    if (normalized.issues.length > 0) {
        throw new Error(`AI returned invalid params format: ${normalized.issues[0]}`);
    }

    return normalized.chain;
}

export async function generateChainFromPrompt(prompt, { signal, onStatus } = {}) {
    const cleanPrompt = (prompt || '').trim();
    if (!cleanPrompt) return [];

    // --- MOCK START: 90s Grunge Raw High-Gain ---
    // TODO: MOCK_REMOVE - Удали этот блок, чтобы вернуть реальную логику API
    const isMockEnabled = true;
    if (isMockEnabled) {
        onStatus?.('Generating 90s Grunge Raw High-Gain mock...');
        onStatus?.('Chain ready');
        return [
            {
                type: 'NoiseGate',
                params: { threshold_db: -45.0, ratio: 10.0, attack_ms: 2.0, release_ms: 50.0 }
            },
            {
                type: 'Overdrive',
                params: { drive: 0.85, mix: 1.0, output_gain: 1.15 }
            },
            {
                type: 'Equalizer',
                params: {
                    low_freq: 120.0, low_gain: 4.0,
                    mid_freq: 850.0, mid_gain: -6.0, mid_q: 1.2,
                    high_freq: 4500.0, high_gain: 5.0
                }
            },
            {
                type: 'Reverb',
                params: { room_size: 0.2, damping: 0.6, width: 0.8, mix: 0.12, pre_delay_ms: 0.0 }
            }
        ];
    }
    // --- MOCK END ---

    onStatus?.('Checking authorization...');
    let accessToken = await ensureAccessToken({ signal, onStatus });

    onStatus?.('Sending request to AI...');
    try {
        const chain = await requestAiChain(cleanPrompt, accessToken, signal);
        onStatus?.('Chain ready');
        return chain;
    } catch (error) {
        if (error?.code !== 'AUTH_REQUIRED') {
            throw error;
        }

        clearStoredTokens();
        onStatus?.('Session expired, re-authorizing...');
        accessToken = await authorizeDesktopUser({ signal, onStatus });
        onStatus?.('Sending request to AI...');

        const chain = await requestAiChain(cleanPrompt, accessToken, signal);
        onStatus?.('Chain ready');
        return chain;
    }
}
