const LOCAL_DEFAULT_API_BASE = 'https://api.tonelab.dev/api/v1';
const LOCAL_DEFAULT_WEB_BASE = 'https://tonelab.dev';
const DEFAULT_API_PREFIX = '';

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
