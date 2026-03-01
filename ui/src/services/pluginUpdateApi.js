import packageJson from '../../package.json';
import { buildApiUrl } from '../config/runtime';
import { openExternalUrl } from '../utils/externalNavigation';

const REQUEST_TIMEOUT_MS = 20000;

const VERSION_ENDPOINT = '/vst/version/current';
const INSTALLER_ENDPOINT_PREFIX = '/vst/installer';

const DEFAULT_PLUGIN_VERSION = '0.0.0';
const UNKNOWN_LOCAL_BUILD = null;
const PACKAGE_VERSION = typeof packageJson?.version === 'string'
    ? packageJson.version.trim()
    : '';

function readFirstStringValue(candidates) {
    for (const candidate of candidates) {
        if (typeof candidate === 'string' && candidate.trim()) {
            return candidate.trim();
        }
    }
    return '';
}

function parseBuildNumber(value) {
    if (typeof value === 'number' && Number.isFinite(value)) {
        return Math.trunc(value);
    }
    if (typeof value === 'string' && value.trim()) {
        const parsed = Number.parseInt(value.trim(), 10);
        if (Number.isFinite(parsed)) return parsed;
    }
    return null;
}

function getPluginVersion() {
    const envVersion = readFirstStringValue([import.meta.env.VITE_TONELAB_PLUGIN_VERSION]);
    const windowVersion = typeof window !== 'undefined'
        ? readFirstStringValue([window.TONELAB_PLUGIN_VERSION])
        : '';
    return windowVersion || envVersion || PACKAGE_VERSION || DEFAULT_PLUGIN_VERSION;
}

function getPluginBuildNumber() {
    const envBuild = parseBuildNumber(import.meta.env.VITE_TONELAB_PLUGIN_BUILD_NUMBER);
    const windowBuild = typeof window !== 'undefined'
        ? parseBuildNumber(window.TONELAB_PLUGIN_BUILD_NUMBER)
        : null;
    return windowBuild ?? envBuild ?? UNKNOWN_LOCAL_BUILD;
}

function parseVersion(version) {
    if (typeof version !== 'string') return null;
    const cleaned = version.trim().replace(/^v/i, '');
    if (!cleaned) return null;

    const parts = cleaned.split('.');
    if (!parts.length) return null;

    const numericParts = parts.map((part) => {
        const match = part.match(/^\d+/);
        if (!match) return null;
        return Number(match[0]);
    });

    if (numericParts.some((part) => part === null)) return null;
    return numericParts;
}

function isVersionGreater(a, b) {
    const av = parseVersion(a);
    const bv = parseVersion(b);
    if (!av || !bv) return false;

    const maxLength = Math.max(av.length, bv.length);
    for (let i = 0; i < maxLength; i += 1) {
        const ai = av[i] ?? 0;
        const bi = bv[i] ?? 0;
        if (ai > bi) return true;
        if (ai < bi) return false;
    }

    return false;
}

function getNavigatorPlatform() {
    if (typeof navigator === 'undefined') return '';

    const uaDataPlatform = navigator.userAgentData?.platform;
    if (typeof uaDataPlatform === 'string' && uaDataPlatform.trim()) {
        return uaDataPlatform.trim();
    }

    if (typeof navigator.platform === 'string' && navigator.platform.trim()) {
        return navigator.platform.trim();
    }

    return typeof navigator.userAgent === 'string' ? navigator.userAgent : '';
}

function detectPlatform() {
    const override = readFirstStringValue([
        typeof window !== 'undefined' ? window.TONELAB_PLATFORM : '',
        import.meta.env.VITE_TONELAB_PLATFORM
    ]).toLowerCase();
    if (override === 'darwin' || override === 'windows' || override === 'linux') {
        return override;
    }

    const raw = getNavigatorPlatform().toLowerCase();
    if (raw.includes('mac') || raw.includes('darwin')) return 'darwin';
    if (raw.includes('win')) return 'windows';
    if (raw.includes('linux')) return 'linux';
    return 'darwin';
}

function detectArchitecture() {
    const override = readFirstStringValue([
        typeof window !== 'undefined' ? window.TONELAB_ARCH : '',
        import.meta.env.VITE_TONELAB_ARCH
    ]).toLowerCase();
    if (override === 'arm64' || override === 'amd64') {
        return override;
    }

    if (typeof navigator === 'undefined') return 'arm64';

    const uaDataArch = typeof navigator.userAgentData?.architecture === 'string'
        ? navigator.userAgentData.architecture.toLowerCase()
        : '';
    if (uaDataArch.includes('arm')) return 'arm64';
    if (uaDataArch.includes('x86') || uaDataArch.includes('amd')) return 'amd64';

    const ua = (navigator.userAgent || '').toLowerCase();
    if (ua.includes('arm64') || ua.includes('aarch64') || ua.includes('arm')) return 'arm64';
    if (ua.includes('x86_64') || ua.includes('amd64') || ua.includes('x64') || ua.includes('win64')) {
        return 'amd64';
    }

    return 'arm64';
}

function getInstallerOs(platform) {
    if (platform === 'darwin') return 'darwin';
    if (platform === 'windows') return 'windows';
    return 'linux';
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
    } finally {
        clearTimeout(timeout);
        if (signal) {
            signal.removeEventListener('abort', onAbort);
        }
    }
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

function hasNewBuild(remoteBuildNumber, localBuildNumber) {
    if (!Number.isFinite(remoteBuildNumber) || !Number.isFinite(localBuildNumber)) return false;
    return remoteBuildNumber > localBuildNumber;
}

function hasNewVersion(remoteVersion, localVersion) {
    if (!remoteVersion || !localVersion) return false;
    return isVersionGreater(remoteVersion, localVersion);
}

function resolveUpdateAvailable(remote, local) {
    if (hasNewBuild(remote.buildNumber, local.buildNumber)) return true;
    return hasNewVersion(remote.version, local.version);
}

function extractRelease(payload) {
    if (payload && typeof payload === 'object') return payload;
    return {};
}

function buildUpdateResult({
    platform,
    arch,
    localVersion,
    localBuildNumber,
    release
}) {
    const remoteVersion = readFirstStringValue([release.version]);
    const remoteBuildNumber = parseBuildNumber(release.build_number);
    const downloadUrl = readFirstStringValue([release.download_url]);
    const isCritical = release?.is_critical === true;

    return {
        platform,
        arch,
        installerOs: getInstallerOs(platform),
        currentVersion: localVersion,
        currentBuildNumber: localBuildNumber,
        latestVersion: remoteVersion,
        latestBuildNumber: remoteBuildNumber,
        downloadUrl,
        isCritical,
        updateAvailable: resolveUpdateAvailable(
            {
                version: remoteVersion,
                buildNumber: remoteBuildNumber
            },
            {
                version: localVersion,
                buildNumber: localBuildNumber
            }
        )
    };
}

function buildInstallerUrl(platform) {
    const installerOs = getInstallerOs(platform);
    const url = new URL(buildApiUrl(`${INSTALLER_ENDPOINT_PREFIX}/${installerOs}`));
    return url.toString();
}

export async function fetchPluginUpdateInfo({ signal } = {}) {
    const platform = detectPlatform();
    const arch = detectArchitecture();
    const localVersion = getPluginVersion();
    const localBuildNumber = getPluginBuildNumber();

    const url = new URL(buildApiUrl(VERSION_ENDPOINT));
    url.searchParams.set('platform', platform);

    const response = await fetchWithTimeout(
        url.toString(),
        {
            method: 'GET',
            headers: {
                Accept: 'application/json'
            },
            credentials: 'include'
        },
        signal
    );

    if (response.status === 404) {
        return {
            platform,
            arch,
            installerOs: getInstallerOs(platform),
            currentVersion: localVersion,
            currentBuildNumber: localBuildNumber,
            latestVersion: '',
            latestBuildNumber: null,
            downloadUrl: '',
            isCritical: false,
            updateAvailable: false
        };
    }

    if (!response.ok) {
        const message = await extractErrorMessage(response);
        throw new Error(`Failed to check plugin version: ${message}`);
    }

    const payload = await parseJsonResponse(response);
    const release = extractRelease(payload);

    return buildUpdateResult({
        platform,
        arch,
        localVersion,
        localBuildNumber,
        release
    });
}

export async function installPluginUpdate(updateInfo = {}) {
    const platform = updateInfo.platform || detectPlatform();
    const installerUrl = buildInstallerUrl(platform);

    openExternalUrl(installerUrl);

    return {
        installerUrl,
        message: 'Installer opened. Complete update and restart Tonelab.'
    };
}
