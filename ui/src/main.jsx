import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import '@xyflow/react/dist/style.css';
import './index.css'
import App from './App.jsx'
import { detectRuntimeEnvironment } from './config/runtime';
import { loadEffectsMetadata } from './config/effects';

const runtimeEnvironment = detectRuntimeEnvironment();
if (typeof window !== 'undefined') {
  window.TONELAB_RUNTIME_ENV = runtimeEnvironment;
  document.documentElement.dataset.tonelabRuntime = runtimeEnvironment;
}

function registerServiceWorker() {
  if (typeof window === 'undefined') return;
  if (!('serviceWorker' in navigator)) return;

  const { protocol, hostname } = window.location;
  const isLocalDev = hostname === 'localhost' || hostname === '127.0.0.1';
  if (protocol !== 'https:' && !isLocalDev) return;

  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      // Service worker is an optimization; app should work without it.
    });
  });
}

registerServiceWorker();

function bootstrapEvergreenAssets() {
  if (typeof window === 'undefined') return;
  const iconsUrl =
    typeof window.TONELAB_EVERGREEN_ICONS_URL === 'string'
      ? window.TONELAB_EVERGREEN_ICONS_URL.trim()
      : '';
  if (!iconsUrl) return;

  try {
    const resolved = new URL(iconsUrl);
    const basePath = `${resolved.origin}${resolved.pathname.substring(
      0,
      resolved.pathname.lastIndexOf('/'),
    )}`;
    window.TONELAB_ASSETS_BASE_URL = basePath;
  } catch (_) {
    // Keep app running even if backend returned malformed icons URL.
  }

  fetch(iconsUrl).catch(() => {
    // Icons pack fetch is best-effort and should not break UI startup.
  });
}

async function startApplication() {
  bootstrapEvergreenAssets();

  try {
    await loadEffectsMetadata();
  } catch (error) {
    if (typeof window !== 'undefined') {
      window.TONELAB_EFFECTS_BOOTSTRAP_ERROR = error?.message || 'Failed to load effects metadata';
    }
    console.error('[tonelab] Failed to load effects metadata', error);
  }

  createRoot(document.getElementById('root')).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

startApplication();
