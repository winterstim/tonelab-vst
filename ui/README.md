# Tonelab UI (React WebView Frontend)

This folder contains the frontend used inside the Tonelab VST plugin editor.

The UI is a React application rendered in a `wry` WebView inside the Rust plugin.

## Stack

- React 19
- Vite 7
- XYFlow (`@xyflow/react`) for node graph UI
- Vitest for unit tests

## What The UI Does

- Renders a node-based chain editor for effects.
- Allows wiring and reordering effect cards.
- Sends active chain snapshots to Rust as JSON.
- Sends real-time parameter updates for active nodes.
- Supports optional AI-generated chain creation.
- Shows optional plugin update notice and opens installer URL.

## Key Runtime Contract With Rust

IPC bridge messages are sent via `window.ipc.postMessage(...)`:

1. Full chain payload (array)
```json
[
  { "type": "Overdrive", "params": { "drive": 0.6, "mix": 1.0, "output_gain": 1.0 } }
]
```

2. Parameter update payload
```json
{
  "type": "param_change",
  "index": 0,
  "param_key": "drive",
  "value": 0.7
}
```

3. External URL request
```json
{
  "type": "open_external_url",
  "url": "https://example.com"
}
```

## Setup

Install dependencies:
```bash
npm ci
```

Run dev server:
```bash
npm run dev
```

Build production bundle:
```bash
npm run build
```

Run tests:
```bash
npm test
```

Run lint:
```bash
npm run lint
```

## Embedding Into Plugin

The Rust plugin embeds `ui/dist/index.html` at compile time (`include_str!`).

Important:
- If you change UI code, run `npm run build` before rebuilding Rust plugin.
- The plugin will display whichever `ui/dist/index.html` existed at compile time.

## Environment Variables

Primary UI variables (browser/dev mode):
- `VITE_TONELAB_API_BASE_URL`
- `VITE_TONELAB_WEB_BASE_URL`
- `VITE_TONELAB_API_PREFIX` (optional, default empty)
- `VITE_TONELAB_DEFAULT_API_BASE_URL` (default fallback: `https://robust-dulciana-tonelab-49d88bd9.koyeb.app/api/v1`)
- `VITE_TONELAB_DEFAULT_WEB_BASE_URL` (default fallback: `https://tonelab-ai.vercel.app`)

Runtime variables injected by Rust in embedded VST mode:
- `window.TONELAB_API_BASE_URL`
- `window.TONELAB_WEB_BASE_URL`
- `window.TONELAB_API_PREFIX`

Rust host/runtime environment sources for those injected values:
- `TONELAB_API_BASE_URL`
- `TONELAB_WEB_BASE_URL` (or `FRONTEND_URL` fallback)
- `TONELAB_API_PREFIX`

Optional update-check metadata variables:
- `VITE_TONELAB_PLUGIN_VERSION`
- `VITE_TONELAB_PLUGIN_BUILD_NUMBER`
- `VITE_TONELAB_PLATFORM`
- `VITE_TONELAB_ARCH`

For practical defaults, copy and edit:
- `ui/.env.example`

## AI Service Behavior

`src/services/tonelabApi.js`:
- Handles desktop OAuth flow via external browser.
- Polls for desktop token.
- Stores access/refresh tokens in local storage.
- Calls AI chain generation endpoint and normalizes payload to supported effect schema.

## Update Service Behavior

`src/services/pluginUpdateApi.js`:
- Reads local plugin version/build metadata.
- Calls backend for latest release metadata.
- Compares local vs remote version/build.
- Opens installer URL through IPC external link message.

## Directory Highlights

- `src/App.jsx`: top-level editor behavior and state orchestration.
- `src/components/`: node cards, toolbar, custom edges, overlays.
- `src/config/effects.js`: effect metadata and parameter UI ranges.
- `src/hooks/useBridge.js`: active chain serialization + IPC send.
- `src/services/`: AI and update API clients.
- `src/utils/layout.js`: card/knob layout math.

## Notes

- The UI is designed to work both in WebView and in browser dev mode.
- Features that require IPC (for example, opening external links) use browser fallbacks where possible.
