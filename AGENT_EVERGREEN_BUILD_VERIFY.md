# Evergreen Build + Verification Runbook (For AI Agents)

This file is the canonical build/verify flow for the Evergreen architecture in this repo.

Goal: run Tonelab VST as a plugin that fetches DSP wasm/assets from backend, verifies signatures, caches locally, and keeps working without forcing users to reinstall on every DSP update.

Architecture note:
- The Rust VST host no longer maps effect names/types (`Overdrive`, `Delay`, etc.).
- Chain + param updates are forwarded to wasm via JSON ABI (`set_chain_json`, `set_param_json`).
- New effect types are expected to be handled by backend-delivered wasm runtime, without host-plugin updates.
- UI effect metadata is delivered by backend (`assets.effects_url`) and loaded at runtime.

## 0) One-Command Orchestration (Preferred)

Use the orchestration script:

```bash
./scripts/evergreen_up.sh
```

This runs backup -> full wasm build -> signing -> plugin install -> backend start -> optional UI dev server start.

Useful options:

```bash
./scripts/evergreen_up.sh --skip-backup --skip-install
./scripts/evergreen_up.sh --no-ui
./scripts/evergreen_up.sh --no-backend
```

Stop backend:

```bash
./scripts/evergreen_down.sh
```

Important:
- `evergreen_up.sh` now restarts backend and kills stale listeners on the sync port.
- Backend is launched from `backend/.bin/tonelab_evergreen_backend` (not `go run`), so PID handling is stable.

## 1) Preconditions

- Run from repo root: `/Users/timwinters/Desktop/tonelab/vst/tonelab-vst`
- Required tools:
  - `rustup`, `cargo`
  - `go`
  - `node`, `npm`
  - `openssl`
- Any DAW with VST3 support installed locally.

## 2) Safety Backup (Recommended)

```bash
./scripts/backup_all.sh
```

Archive appears in `backups/`.

## 3) Build Full DSP WASM (1:1 with Rust DSP)

The wasm engine is built from local DSP code:
- `wasm-engine-rust/src/dsp_core/mod.rs`
- `wasm-engine-rust/src/dsp_core/effects.rs`

Commands:

```bash
rustup target add wasm32-unknown-unknown
cargo build --manifest-path wasm-engine-rust/Cargo.toml --target wasm32-unknown-unknown --release
cp wasm-engine-rust/target/wasm32-unknown-unknown/release/tonelab_wasm_engine.wasm backend/assets/engine.wasm
```

## 4) Sign DSP Bundle (Ed25519)

```bash
backend/security/sign_bundle.sh
```

Outputs:
- `backend/assets/engine.wasm.sig.b64`
- `backend/security/dev_ed25519_public_key.b64`
- private key in `backend/security/keys/` (ignored by git)

## 5) Run/Deploy UI

```bash
cd ui
npm run build
cd ..
```

Important: plugin no longer embeds `ui/dist/index.html`. It loads the UI URL from `/vst/sync` (`assets.web_ui_url`).
For local development, `./scripts/evergreen_up.sh` also starts `npm run dev` and backend advertises `http://localhost:5173`.

## 6) Start Evergreen Backend

Preferred:

```bash
./scripts/evergreen_up.sh --skip-backup --skip-install --no-ui
```

Manual fallback:

```bash
cd backend
mkdir -p .bin
go build -o .bin/tonelab_evergreen_backend .
EVERGREEN_ASSETS_DIR="$(pwd)/assets" \
EVERGREEN_SIGNATURE_FILE="engine.wasm.sig.b64" \
EVERGREEN_WASM_SIGNATURE_B64="" \
EVERGREEN_WEB_UI_URL="http://localhost:5173" \
./.bin/tonelab_evergreen_backend
```

Backend should serve:
- `GET http://localhost:8080/vst/sync`
- `GET http://localhost:8080/assets/engine.wasm`
- `GET http://localhost:8080/assets/icons.zip`
- `GET http://localhost:8080/assets/effects_manifest.json`

## 7) Optional Runtime Env Overrides

The one-command script already configures defaults. Manual overrides are optional:

```bash
cd /Users/timwinters/Desktop/tonelab/vst/tonelab-vst
export TONELAB_EVERGREEN_SYNC_URL=http://localhost:8080/vst/sync
export TONELAB_EVERGREEN_ALLOW_UNSIGNED=false
export TONELAB_LOG_FILE_PATH=/tmp/tonelab_vst.log
```

Notes:
- Public key verification works without runtime env because the key is embedded at build/install time.
- Runtime env key (`TONELAB_EVERGREEN_ED25519_PUBLIC_KEY_B64`) can still override if needed.

## 8) Build + Install Plugin

```bash
cargo run -p xtask -- install --release
```

## 9) Launch Any DAW

Open your preferred DAW normally, rescan plugins if needed, and load `Tonelab VST` from VST3 plugins.

## 10) Verification Checklist

### 10.1 Backend sync contract

```bash
curl -s http://localhost:8080/vst/sync
```

Expected JSON fields:
- `version`
- `wasm_url`
- `signature`
- `assets.icons_url`
- `assets.web_ui_url`
- `assets.effects_url`

Also verify signature consistency (critical):

```bash
asset_sig=$(cat backend/assets/engine.wasm.sig.b64)
sync_sig=$(curl -s http://localhost:8080/vst/sync | jq -r '.signature')
test "$asset_sig" = "$sync_sig" && echo "signature ok" || echo "signature mismatch"
```

### 10.2 Plugin log confirms Evergreen bootstrap

```bash
tail -n 200 /tmp/tonelab_vst.log
```

Look for:
- `Evergreen bundle active: ...`
- no persistent bootstrap/signature errors

### 10.3 Runtime behavior

Inside plugin UI:
- activate a chain
- move knobs/params
- audio should process without plugin rebuild

## 11) Offline Cache Validation

1. Start backend, open plugin once (prime cache).
2. Stop backend.
3. Reopen DAW/plugin.
4. Expect plugin to continue using cached wasm bundle.

If it fails both online and cache, check logs in `/tmp/tonelab_vst.log`.

## 12) CI/Agent Quick Smoke Commands

From repo root:

```bash
cargo check
cd backend && go build ./... && cd ..
cd ui && npm test && npm run build && cd ..
```

## 13) Common Failure Modes

- `WASM runtime is not loaded`: sync/bootstrap/signature failure.
- `Ed25519 signature verification failed`: wrong public key or stale signature.
  - First check: `backend/assets/engine.wasm.sig.b64` must match `/vst/sync` `signature`.
  - If mismatch: restart backend via `./scripts/evergreen_up.sh --skip-backup --skip-install --no-ui`.
- UI changes not visible in plugin: wrong `assets.web_ui_url` in `/vst/sync`, or stale browser/service-worker cache on UI host.
- No backend sync: backend not running or wrong `TONELAB_EVERGREEN_SYNC_URL`.
- macOS codesign detritus warning: rerun `./scripts/evergreen_up.sh` (it cleans `xattr` and `._*` sidecars before install).

## 14) Log Capture (for agent handoff)

Plugin log:

```bash
tail -n 200 /tmp/tonelab_vst.log
```

Backend log:

```bash
tail -n 200 /tmp/tonelab_evergreen_backend.log
```

UI dev server log:

```bash
tail -n 200 /tmp/tonelab_evergreen_ui.log
```

## 15) Security Notes

- Do not commit files under `backend/security/keys/`.
- Use strict mode in realistic testing:
  - `TONELAB_EVERGREEN_ALLOW_UNSIGNED=false`
- Rotate signing keys for production; dev key is local-only.
