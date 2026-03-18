# Evergreen Bundle Signing

Sign `engine.wasm` with Ed25519 and produce:

- `backend/assets/engine.wasm.sig.b64` (sync payload signature)
- `backend/security/dev_ed25519_public_key.b64` (public verification key)

Run:

```bash
cd backend/security
./sign_bundle.sh
```

Then set in the Rust host environment:

```bash
TONELAB_EVERGREEN_ED25519_PUBLIC_KEY_B64=$(cat backend/security/dev_ed25519_public_key.b64)
TONELAB_EVERGREEN_ALLOW_UNSIGNED=false
```

Private keys are generated under `backend/security/keys/` and ignored by git.
