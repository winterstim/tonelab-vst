import { afterEach, describe, expect, it } from 'vitest';
import {
    RuntimeEnvironment,
    buildApiUrl,
    buildWebUrl,
    detectRuntimeEnvironment,
    getApiBaseUrl,
    getWebBaseUrl
} from './runtime';

const originalWindow = globalThis.window;

afterEach(() => {
    if (originalWindow === undefined) {
        delete globalThis.window;
    } else {
        globalThis.window = originalWindow;
    }
});

describe('runtime config', () => {
    it('uses local defaults when runtime values are missing', () => {
        if (originalWindow === undefined) {
            delete globalThis.window;
        } else {
            globalThis.window = {};
        }

        expect(getApiBaseUrl()).toBe('https://api.tonelab.dev/api/v1');
        expect(getWebBaseUrl()).toBe('https://vst.tonelab.dev');
        expect(buildApiUrl('/health')).toBe('https://api.tonelab.dev/api/v1/health');
        expect(buildWebUrl('/docs')).toBe('https://vst.tonelab.dev/docs');
    });

    it('uses injected runtime values when present', () => {
        globalThis.window = {
            TONELAB_API_BASE_URL: 'https://api.example.com',
            TONELAB_WEB_BASE_URL: 'https://app.example.com/'
        };

        expect(getApiBaseUrl()).toBe('https://api.example.com');
        expect(getWebBaseUrl()).toBe('https://app.example.com');
        expect(buildApiUrl('vst/version/current')).toBe('https://api.example.com/vst/version/current');
    });

    it('applies API prefix only when explicitly provided', () => {
        globalThis.window = {
            TONELAB_API_BASE_URL: 'https://api.example.com',
            TONELAB_API_PREFIX: '/api/v1'
        };

        expect(getApiBaseUrl()).toBe('https://api.example.com/api/v1');
        expect(buildApiUrl('/health')).toBe('https://api.example.com/api/v1/health');
    });

    it('detects embedded VST runtime using ipc bridge', () => {
        globalThis.window = {
            ipc: { postMessage: () => {} },
            location: { hostname: 'localhost' }
        };

        expect(detectRuntimeEnvironment()).toBe(RuntimeEnvironment.VST_EMBEDDED);
    });

    it('detects browser dev runtime on localhost', () => {
        globalThis.window = {
            location: { hostname: 'localhost' }
        };

        expect(detectRuntimeEnvironment()).toBe(RuntimeEnvironment.BROWSER_DEV);
    });

    it('supports explicit runtime override', () => {
        globalThis.window = {
            TONELAB_RUNTIME_ENV: 'browser-web',
            location: { hostname: 'localhost' }
        };

        expect(detectRuntimeEnvironment()).toBe(RuntimeEnvironment.BROWSER_WEB);
    });
});
