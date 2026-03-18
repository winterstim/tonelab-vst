# Deploy Guide (Vercel + Google Cloud Run)

This repo uses the Evergreen architecture:
- VST bundle contains only the Rust host.
- DSP ships as `engine.wasm` from the backend `/vst/sync`.
- UI ships from a remote URL (`assets.web_ui_url`).
- Effect metadata ships from `assets.effects_url`.

## Frontend (Vercel)

Recommended: deploy the UI as its own Vercel project using the `ui` directory.

Settings:
- Root Directory: `ui`
- Build Command: `npm run build`
- Output Directory: `dist`

Environment variables (Vercel Project Settings):
- `VITE_TONELAB_API_BASE_URL` = `https://<your-api-domain>/api/v1`
- `VITE_TONELAB_WEB_BASE_URL` = `https://<your-ui-domain>`
- `VITE_TONELAB_API_PREFIX` = (optional, usually empty)
- `VITE_TONELAB_DEFAULT_API_BASE_URL` = same as API base
- `VITE_TONELAB_DEFAULT_WEB_BASE_URL` = same as UI base

After deploy, set the backend env:
- `EVERGREEN_WEB_UI_URL` = the Vercel URL

## Backend (Google Cloud Run)

Deploy from the `backend` directory.

One-time login & project config:
```bash
gcloud auth login
gcloud config set project <YOUR_PROJECT_ID>
```

Deploy:
```bash
cd backend
gcloud run deploy tonelab-evergreen-backend \
  --source . \
  --region <YOUR_REGION> \
  --allow-unauthenticated
```

Set runtime environment variables (Cloud Run console or CLI):
- `EVERGREEN_PUBLIC_BASE_URL` = Cloud Run service URL (no trailing slash)
- `EVERGREEN_WEB_UI_URL` = Vercel UI URL
- `EVERGREEN_VERSION` = your version (optional)
- `EVERGREEN_RATE_LIMIT_RPS` = `60` (optional)
- `EVERGREEN_RATE_LIMIT_BURST` = `120` (optional)
- `EVERGREEN_RATE_LIMIT_TTL` = `2m` (optional)

Cloud Run provides `PORT` automatically and the backend respects it.

## Updating DSP / Effects

When you update DSP or metadata:
1. Build wasm: `cargo build --manifest-path wasm-engine-rust/Cargo.toml --target wasm32-unknown-unknown --release`
2. Copy: `cp wasm-engine-rust/target/wasm32-unknown-unknown/release/tonelab_wasm_engine.wasm backend/assets/engine.wasm`
3. Sign: `backend/security/sign_bundle.sh`
4. Update `backend/assets/effects_manifest.json` if effect metadata changed
5. Redeploy backend (Cloud Run) so `/assets/*` is updated

The VST bundle does not change for DSP/UI updates.
