# Tonelab VST (Evergreen Architecture)

Tonelab is a VST3 plugin with a thin native host and remotely delivered runtime assets.

## What Is Evergreen

- The VST bundle contains host/runtime glue and audio I/O.
- DSP is delivered as signed `engine.wasm` from backend `/vst/sync`.
- UI is loaded from `assets.web_ui_url` (local dev or your future production domain).
- Effect metadata is delivered from `assets.effects_url` and drives the UI dynamically.
- No embedded dashboard HTML is required in the plugin bundle.
- The host does not map hardcoded effect names/types; chain and param updates are forwarded to wasm as JSON payloads.

## Repository Layout

- `src/`: VST host/plugin code (Rust)
- `wasm-engine-rust/`: DSP wasm engine source (Rust)
- `backend/`: sync API + assets server (Go)
- `ui/`: remote dashboard app (Vite/React)
- `scripts/evergreen_up.sh`: build/install/start pipeline
- `scripts/evergreen_down.sh`: stop backend/UI started by script
- `AGENT_EVERGREEN_BUILD_VERIFY.md`: detailed verification runbook

## Quick Local Start

From repo root:

```bash
./scripts/evergreen_up.sh --skip-backup
```

This does, in order:

1. Builds `wasm-engine-rust` for `wasm32-unknown-unknown`
2. Copies wasm to `backend/assets/engine.wasm`
3. Signs wasm (`engine.wasm.sig.b64`)
4. Builds and installs VST3 via `xtask`
5. Starts backend (`/vst/sync`) and UI dev server

Then open any DAW and scan/load `Tonelab VST v0.2.0`.

## Manual Verification

```bash
cargo check
cargo test
cd backend && go test ./... && cd ..
cd ui && npm test -- --run && npm run build && cd ..
```

Live sync check (while backend is running):

```bash
curl -s http://localhost:8080/vst/sync | jq .
asset_sig=$(cat backend/assets/engine.wasm.sig.b64)
sync_sig=$(curl -s http://localhost:8080/vst/sync | jq -r '.signature')
test "$asset_sig" = "$sync_sig" && echo "signature ok" || echo "signature mismatch"
```

## Deployment Model

- Put UI on your domain (e.g. `https://app.example.com`).
- Set backend `EVERGREEN_WEB_UI_URL` to that UI URL.
- Backend returns UI URL + signed wasm URLs in `/vst/sync`.
- Plugin loads UI remotely and applies DSP via wasm runtime.

## Notes

- Cached evergreen assets are used only after a successful signed load.
- If backend is unavailable before first successful sync, DSP runtime is unavailable.
- Security material under `backend/security/keys/` is local/dev-only and ignored.

## License

See `LICENSE`.
