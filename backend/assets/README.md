# Evergreen Backend Assets

- `engine.wasm`: WASM DSP bundle consumed by the Rust host via `/vst/sync`.
- `engine.wasm.sig.b64`: Ed25519 signature in Base64 (optional in dev, required in strict mode).
- `icons.zip`: optional icon pack URL referenced from `/vst/sync` (`assets.icons_url`).
- `effects_manifest.json`: canonical UI/effect metadata served from backend (`assets.effects_url`).
- `icons/*.svg`: per-effect icons referenced by `effects_manifest.json`.

Generate/update `engine.wasm` from Rust DSP sources in `wasm-engine-rust/src/dsp_core/`:

```bash
rustup target add wasm32-unknown-unknown
cargo build --manifest-path ../wasm-engine-rust/Cargo.toml --target wasm32-unknown-unknown --release
cp ../wasm-engine-rust/target/wasm32-unknown-unknown/release/tonelab_wasm_engine.wasm ./engine.wasm
../backend/security/sign_bundle.sh
```
