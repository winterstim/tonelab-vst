import {
    getAssetsBaseUrl,
    getEvergreenEffectsUrl,
    getEvergreenIconsUrl
} from './runtime';

export const EFFECTS_METADATA = Object.create(null);

function toFiniteNumber(value, fallback = 0) {
    if (typeof value === 'number' && Number.isFinite(value)) return value;
    if (typeof value === 'string' && value.trim()) {
        const parsed = Number(value.trim());
        if (Number.isFinite(parsed)) return parsed;
    }
    return fallback;
}

function normalizeAliases(rawAliases) {
    if (!Array.isArray(rawAliases)) return [];
    return rawAliases
        .map((value) => (typeof value === 'string' ? value.trim().toLowerCase() : ''))
        .filter(Boolean);
}

function resolveUrl(baseUrl, maybeRelativeUrl) {
    if (typeof maybeRelativeUrl !== 'string' || !maybeRelativeUrl.trim()) return '';
    try {
        return new URL(maybeRelativeUrl.trim(), baseUrl).toString();
    } catch {
        return maybeRelativeUrl.trim();
    }
}

function normalizeParam(rawParam) {
    if (!rawParam || typeof rawParam !== 'object') return null;
    const id = typeof rawParam.id === 'string' ? rawParam.id.trim() : '';
    if (!id) return null;

    const min = toFiniteNumber(rawParam.min, 0);
    const max = toFiniteNumber(rawParam.max, 1);
    const normalizedMax = max <= min ? min + 1 : max;
    const fallbackDefault = min;
    const defaultValue = toFiniteNumber(rawParam.default, fallbackDefault);
    const step = toFiniteNumber(rawParam.step, 0.01);

    return {
        id,
        label: typeof rawParam.label === 'string' && rawParam.label.trim()
            ? rawParam.label.trim()
            : id,
        min,
        max: normalizedMax,
        default: Math.min(normalizedMax, Math.max(min, defaultValue)),
        step: step > 0 ? step : 0.01,
        aliases: normalizeAliases(rawParam.aliases),
    };
}

function normalizeParamMap(rawParams) {
    const params = Object.create(null);
    if (Array.isArray(rawParams)) {
        rawParams.forEach((rawParam) => {
            const normalized = normalizeParam(rawParam);
            if (normalized) {
                params[normalized.id] = normalized;
            }
        });
        return params;
    }

    if (rawParams && typeof rawParams === 'object') {
        Object.entries(rawParams).forEach(([paramId, rawConfig]) => {
            const rawParam = {
                ...(rawConfig && typeof rawConfig === 'object' ? rawConfig : {}),
                id: rawConfig?.id ?? paramId,
            };
            const normalized = normalizeParam(rawParam);
            if (normalized) {
                params[normalized.id] = normalized;
            }
        });
    }
    return params;
}

function normalizeEffect(rawEffect, manifestUrl) {
    if (!rawEffect || typeof rawEffect !== 'object') return null;
    const idCandidate = rawEffect.id ?? rawEffect.type ?? rawEffect.name;
    const id = typeof idCandidate === 'string' ? idCandidate.trim() : '';
    if (!id) return null;

    const params = normalizeParamMap(rawEffect.params);
    if (Object.keys(params).length === 0) return null;

    return {
        id,
        label: typeof rawEffect.label === 'string' && rawEffect.label.trim()
            ? rawEffect.label.trim()
            : id,
        aliases: normalizeAliases(rawEffect.aliases),
        icon_url: resolveUrl(manifestUrl, rawEffect.icon_url || ''),
        params,
    };
}

function normalizeEffectsManifest(payload, manifestUrl) {
    const entries = [];
    if (Array.isArray(payload?.effects)) {
        payload.effects.forEach((rawEffect) => {
            const normalized = normalizeEffect(rawEffect, manifestUrl);
            if (normalized) entries.push(normalized);
        });
    } else if (payload?.effects && typeof payload.effects === 'object') {
        Object.entries(payload.effects).forEach(([effectId, rawEffect]) => {
            const normalized = normalizeEffect(
                {
                    ...(rawEffect && typeof rawEffect === 'object' ? rawEffect : {}),
                    id: rawEffect?.id ?? effectId,
                },
                manifestUrl
            );
            if (normalized) entries.push(normalized);
        });
    }

    if (entries.length === 0) {
        throw new Error('effects manifest is empty or invalid');
    }

    const map = Object.create(null);
    entries.forEach((entry) => {
        map[entry.id] = entry;
    });
    return map;
}

function assignMetadata(map) {
    Object.keys(EFFECTS_METADATA).forEach((key) => {
        delete EFFECTS_METADATA[key];
    });
    Object.assign(EFFECTS_METADATA, map);
}

function deriveManifestUrlFromIcons() {
    const iconsUrl = getEvergreenIconsUrl();
    if (!iconsUrl) return '';
    try {
        const parsed = new URL(iconsUrl);
        const pathname = parsed.pathname || '/';
        const slashIndex = pathname.lastIndexOf('/');
        const directory = slashIndex >= 0 ? pathname.slice(0, slashIndex + 1) : '/';
        return `${parsed.origin}${directory}effects_manifest.json`;
    } catch {
        return '';
    }
}

function resolveManifestUrl(explicitUrl) {
    const direct = (explicitUrl || '').trim() || getEvergreenEffectsUrl();
    if (direct) return direct;

    const fromIcons = deriveManifestUrlFromIcons();
    if (fromIcons) return fromIcons;

    const assetsBase = getAssetsBaseUrl();
    return `${assetsBase}/effects_manifest.json`;
}

export async function loadEffectsMetadata({ signal, manifestUrl } = {}) {
    const url = resolveManifestUrl(manifestUrl);
    if (!url) {
        throw new Error('effects manifest URL is not configured');
    }

    const response = await fetch(url, { signal, cache: 'no-store' });
    if (!response.ok) {
        throw new Error(`failed to fetch effects manifest (${response.status})`);
    }

    const payload = await response.json();
    const metadata = normalizeEffectsManifest(payload, url);
    assignMetadata(metadata);
    return EFFECTS_METADATA;
}

export function getEffectsMetadata() {
    return EFFECTS_METADATA;
}

export function getEffectList() {
    return Object.values(EFFECTS_METADATA);
}

export const validateParam = (effectType, paramId, value) => {
    const effect = EFFECTS_METADATA[effectType];
    if (!effect) return false;
    const paramConfig = effect.params?.[paramId];
    if (!paramConfig) return false;
    if (!Number.isFinite(value)) return false;
    return value >= paramConfig.min && value <= paramConfig.max;
};
