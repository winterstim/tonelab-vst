# Tonelab Rust WASM Engine

This crate builds the Evergreen DSP wasm module from local Rust DSP sources:

- `wasm-engine-rust/src/dsp_core/mod.rs`
- `wasm-engine-rust/src/dsp_core/effects.rs`

Exports:

- `alloc(size: i32) -> i32`
- `process(input_ptr: i32, output_ptr: i32, samples: i32)`
- `set_param(effect_idx: i32, key_hash: i32, value: f32)`

Build and copy into backend assets:

```bash
rustup target add wasm32-unknown-unknown
cargo build --manifest-path wasm-engine-rust/Cargo.toml --target wasm32-unknown-unknown --release
cp wasm-engine-rust/target/wasm32-unknown-unknown/release/tonelab_wasm_engine.wasm backend/assets/engine.wasm
```
