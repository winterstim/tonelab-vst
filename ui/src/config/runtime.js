const LOCAL_DEFAULT_API_BASE = 'https://api.tonelab.dev/api/v1';
const LOCAL_DEFAULT_WEB_BASE = 'https://vst.tonelab.dev';
const DEFAULT_API_PREFIX = '';
const RUNTIME_ENV_VST = 'vst-embedded';
const RUNTIME_ENV_DEV_BROWSER = 'browser-dev';
const RUNTIME_ENV_WEB = 'browser-web';
const RUNTIME_ENV_SSR = 'ssr';

function readFirstStringValue(candidates) {
    for (const candidate of candidates) {
        if (typeof candidate === 'string' && candidate.trim()) {
            return candidate.trim();
        }
    }
    return '';
}

function readWindowString(key) {
    if (typeof window === 'undefined') return '';
    const value = window[key];
    return typeof value === 'string' ? value : '';
}

function normalizeOrigin(value, fallback) {
    const trimmed = (value || '').trim().replace(/\/+$/, '');
    return trimmed || fallback;
}

function normalizeApiPrefix(prefix) {
    const trimmed = (prefix || '').trim();
    if (!trimmed) return DEFAULT_API_PREFIX;
    if (trimmed.startsWith('/')) return trimmed.replace(/\/+$/, '');
    return `/${trimmed.replace(/\/+$/, '')}`;
}

function getApiPrefix() {
    return normalizeApiPrefix(
        readFirstStringValue([
            import.meta.env.VITE_TONELAB_API_PREFIX,
            readWindowString('TONELAB_API_PREFIX')
        ])
    );
}

const DEFAULT_API_BASE = normalizeOrigin(
    readFirstStringValue([import.meta.env.VITE_TONELAB_DEFAULT_API_BASE_URL]),
    LOCAL_DEFAULT_API_BASE
);

const DEFAULT_WEB_BASE = normalizeOrigin(
    readFirstStringValue([import.meta.env.VITE_TONELAB_DEFAULT_WEB_BASE_URL]),
    LOCAL_DEFAULT_WEB_BASE
);

export function getApiBaseUrl() {
    const configuredBase = readFirstStringValue([
        import.meta.env.VITE_TONELAB_API_BASE_URL,
        readWindowString('TONELAB_API_BASE_URL')
    ]);

    const origin = normalizeOrigin(configuredBase, DEFAULT_API_BASE);
    const apiPrefix = getApiPrefix();
    if (!apiPrefix) return origin;
    if (origin.endsWith(apiPrefix)) return origin;
    return `${origin}${apiPrefix}`;
}

export function getWebBaseUrl() {
    const configuredBase = readFirstStringValue([
        import.meta.env.VITE_TONELAB_WEB_BASE_URL,
        readWindowString('TONELAB_WEB_BASE_URL')
    ]);

    return normalizeOrigin(configuredBase, DEFAULT_WEB_BASE);
}

export function buildApiUrl(path) {
    const normalizedPath = path.startsWith('/') ? path : `/${path}`;
    return `${getApiBaseUrl()}${normalizedPath}`;
}

export function buildWebUrl(path) {
    const normalizedPath = path.startsWith('/') ? path : `/${path}`;
    return `${getWebBaseUrl()}${normalizedPath}`;
}

export function getIsInsideVST() {
    return detectRuntimeEnvironment() === RUNTIME_ENV_VST;
}

export function getAssetsBaseUrl() {
    const configuredBase = readFirstStringValue([
        import.meta.env.VITE_TONELAB_ASSETS_BASE_URL,
        readWindowString('TONELAB_ASSETS_BASE_URL')
    ]);
    return normalizeOrigin(configuredBase, 'https://assets.tonelab.dev');
}

export function getEvergreenIconsUrl() {
    return readFirstStringValue([readWindowString('TONELAB_EVERGREEN_ICONS_URL')]);
}

export function getEvergreenEffectsUrl() {
    return readFirstStringValue([readWindowString('TONELAB_EVERGREEN_EFFECTS_URL')]);
}

export function detectRuntimeEnvironment() {
    if (typeof window === 'undefined') return RUNTIME_ENV_SSR;

    const runtimeOverride = readWindowString('TONELAB_RUNTIME_ENV').toLowerCase();
    if (
        runtimeOverride === RUNTIME_ENV_VST ||
        runtimeOverride === RUNTIME_ENV_DEV_BROWSER ||
        runtimeOverride === RUNTIME_ENV_WEB
    ) {
        return runtimeOverride;
    }

    const hasEmbeddedBridge = !!(
        window.ipc?.postMessage ||
        window.chrome?.webview ||
        window.webkit?.messageHandlers
    );
    if (hasEmbeddedBridge) return RUNTIME_ENV_VST;

    const hostname = (window.location?.hostname || '').toLowerCase();
    if (hostname === 'localhost' || hostname === '127.0.0.1') {
        return RUNTIME_ENV_DEV_BROWSER;
    }

    return RUNTIME_ENV_WEB;
}

export const RuntimeEnvironment = {
    VST_EMBEDDED: RUNTIME_ENV_VST,
    BROWSER_DEV: RUNTIME_ENV_DEV_BROWSER,
    BROWSER_WEB: RUNTIME_ENV_WEB,
    SSR: RUNTIME_ENV_SSR
};
