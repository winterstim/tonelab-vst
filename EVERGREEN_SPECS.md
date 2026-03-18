# Evergreen VST: Technical Specification & Implementation Guide

This document outlines the architecture and implementation steps for transitioning the Tonelab VST to a backend-driven "Evergreen" model.

## Core Architecture

The system consists of three main layers:
1. **Rust Host Shell**: A minimal VST3/AU wrapper that handles DAW communication and WASM execution.
2. **Go Backend**: The central "brain" that manages DSP logic delivery, AI generation, and asset distribution.
3. **Web UI (Vite)**: A hybrid frontend that runs on `vst.tonelab.dev` and communicates with the Rust host via a WebView bridge.

## 1. Go Backend (Delivery & Logic)

The backend is responsible for serving:
- **WASM Modules**: Symmetrically compiled DSP effects (Overdrive, Delay, etc.).
- **Asset Manifests**: JSON describing icons, names, and parameters (effects manifest).
- **Dynamic Formulas**: Math expressions for non-WASM routing.

### Key Endpoints (Proposed)
- `GET /vst/sync`: Returns latest WASM/Asset bundle based on client version.
- `GET /assets/*`: Static icons and resources.

## 2. Thin Rust Host (Execution)

Rust is stripped of hardcoded effects. Its role is reduced to:
- **`WasmRuntime`**: Loading and executing `.wasm` files.
- **`WebViewBridge`**: Messaging interface between the Web UI and the Audio Thread.
- **`CacheManager`**: Local persistence of logic for offline use.

## 3. Web UI (Interaction)

The UI detects its environment and behaves accordingly.

---

## Technical Contracts (The "Glue")

To ensure any agent can implement this, we follow these strict contracts:

### A. JS-to-Rust Bridge (JSON)
Message sent via `window.ipc.postMessage(json)`:
```json
{
  "type": "sync_chain",
  "data": [
    { "type": "Overdrive", "params": { "gain": 0.5, "tone": 0.5 } },
    { "type": "Delay", "params": { "time": 0.3, "feedback": 0.4 } }
  ]
}
```

### B. Go-to-Host Sync (JSON)
Endpoint `GET /vst/sync`:
```json
{
  "version": "1.0.0",
  "wasm_url": "https://assets.tonelab.dev/v1/engine.wasm",
  "signature": "base64_ed25519_sig",
  "assets": {
    "icons_url": "https://assets.tonelab.dev/v1/icons.zip",
    "web_ui_url": "https://vst.tonelab.dev",
    "effects_url": "https://assets.tonelab.dev/v1/effects_manifest.json"
  }
}
```

### C. WASM Engine Interface
The `.wasm` module must export:
- `fn alloc(size: i32) -> *mut f32`
- `fn alloc_bytes(size: i32) -> *mut u8`
- `fn process(input_ptr: *mut f32, output_ptr: *mut f32, samples: i32)`
- `fn set_sample_rate(sample_rate: f32) -> i32`
- `fn set_chain_json(ptr: i32, len: i32) -> i32`
- `fn set_param_json(ptr: i32, len: i32) -> i32`

---

## Implementation Roadmap

### Phase 1: Local Backend & UI Bridge
- [x] Initialize Go server on `localhost:8080`.
- [x] Configure Vite dev server on `localhost:5173`.
- [x] Implement environment detection in React.

### Phase 2: WASM Migration
- [x] Port DSP logic to Rust WASM (`wasm-engine-rust/src/dsp_core/*`).
- [x] Implement `Wasmtime` integration in Rust host.

### Phase 3: Offline & Security
- [x] Implement local caching in Rust.
- [x] Add Ed25519 signature verification for incoming bundles.
