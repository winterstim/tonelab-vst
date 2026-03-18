use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use wasmtime::{Engine, Instance, Memory, Module, Store, TypedFunc};

const ENV_ENABLED: &str = "TONELAB_EVERGREEN_ENABLED";
const ENV_SYNC_URL: &str = "TONELAB_EVERGREEN_SYNC_URL";
const ENV_ALLOW_UNSIGNED: &str = "TONELAB_EVERGREEN_ALLOW_UNSIGNED";
const ENV_PUBLIC_KEY_B64: &str = "TONELAB_EVERGREEN_ED25519_PUBLIC_KEY_B64";
const EMBEDDED_PUBLIC_KEY_B64: Option<&str> = option_env!("TONELAB_EVERGREEN_PUBLIC_KEY_B64");
const DEFAULT_SYNC_URL: &str = "http://localhost:8080/vst/sync";
const CACHE_DIR_NAME: &str = "evergreen_cache";
const CACHE_MANIFEST_FILE: &str = "sync_manifest.json";
const CACHE_WASM_FILE: &str = "engine.wasm";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncAssets {
    #[serde(default)]
    pub icons_url: String,
    #[serde(default)]
    pub web_ui_url: String,
    #[serde(default)]
    pub effects_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub version: String,
    pub wasm_url: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub assets: SyncAssets,
}

pub struct EvergreenEngine {
    cache: CacheManager,
    runtime: Option<WasmRuntime>,
    active_manifest: Option<SyncManifest>,
    last_error: Option<String>,
    sample_rate: f32,
}

impl EvergreenEngine {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            cache: CacheManager::new(data_dir.join(CACHE_DIR_NAME)),
            runtime: None,
            active_manifest: None,
            last_error: None,
            sample_rate: 44_100.0,
        }
    }

    pub fn bootstrap(&mut self) -> Result<(), String> {
        if !evergreen_enabled() {
            self.runtime = None;
            self.active_manifest = None;
            self.last_error = None;
            return Ok(());
        }

        self.cache.ensure()?;
        let sync_url = resolve_sync_url();

        match self.try_online_sync(&sync_url) {
            Ok((manifest, runtime)) => {
                let runtime = runtime;
                let _ = runtime.set_sample_rate(self.sample_rate);
                self.active_manifest = Some(manifest);
                self.runtime = Some(runtime);
                self.last_error = None;
                Ok(())
            }
            Err(online_err) => match self.try_cached_sync() {
                Ok((manifest, runtime)) => {
                    let runtime = runtime;
                    let _ = runtime.set_sample_rate(self.sample_rate);
                    self.active_manifest = Some(manifest);
                    self.runtime = Some(runtime);
                    self.last_error = Some(format!(
                        "Online Evergreen sync failed, using cache instead: {}",
                        online_err
                    ));
                    Ok(())
                }
                Err(cache_err) => {
                    let err = format!(
                        "Evergreen sync failed online and from cache. online='{}' cache='{}'",
                        online_err, cache_err
                    );
                    self.last_error = Some(err.clone());
                    Err(err)
                }
            },
        }
    }

    pub fn web_ui_url(&self) -> Option<&str> {
        self.active_manifest
            .as_ref()
            .map(|manifest| manifest.assets.web_ui_url.as_str())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn icons_url(&self) -> Option<&str> {
        self.active_manifest
            .as_ref()
            .map(|manifest| manifest.assets.icons_url.as_str())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn effects_url(&self) -> Option<&str> {
        self.active_manifest
            .as_ref()
            .map(|manifest| manifest.assets.effects_url.as_str())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn active_version(&self) -> Option<&str> {
        self.active_manifest
            .as_ref()
            .map(|manifest| manifest.version.as_str())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn has_runtime(&self) -> bool {
        self.runtime.is_some()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    #[allow(dead_code)]
    pub fn set_param(&mut self, effect_idx: i32, key: &str, value: f32) -> Result<(), String> {
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| "WASM runtime is not loaded".to_string())?;
        runtime.set_param_json(effect_idx, key, value)
    }

    pub fn process_frame(&mut self, l: f32, r: f32) -> Result<(f32, f32), String> {
        let input = [l, r];
        let mut output = [0.0f32, 0.0f32];
        self.process_interleaved_stereo(&input, &mut output)?;
        Ok((output[0], output[1]))
    }

    pub fn process_interleaved_stereo(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<(), String> {
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| "WASM runtime is not loaded".to_string())?;
        runtime.process_interleaved_stereo(input, output)
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.clamp(8_000.0, 192_000.0);
        if let Some(runtime) = self.runtime.as_mut() {
            let _ = runtime.set_sample_rate(self.sample_rate);
        }
    }

    pub fn sync_chain_json(&mut self, chain_json: &str) -> Result<(), String> {
        let runtime = self
            .runtime
            .as_mut()
            .ok_or_else(|| "WASM runtime is not loaded".to_string())?;
        runtime.set_chain_json(chain_json)
    }

    fn try_online_sync(&self, sync_url: &str) -> Result<(SyncManifest, WasmRuntime), String> {
        let manifest = fetch_sync_manifest(sync_url)?;
        if manifest.wasm_url.trim().is_empty() {
            return Err("sync manifest has empty wasm_url".to_string());
        }

        let wasm_bytes = download_bytes(&manifest.wasm_url)?;
        verify_bundle_signature(&wasm_bytes, &manifest.signature)?;

        self.cache.write_manifest(&manifest)?;
        self.cache.write_wasm(&wasm_bytes)?;

        let runtime = WasmRuntime::from_bytes(&wasm_bytes)?;
        runtime.smoke_test()?;
        Ok((manifest, runtime))
    }

    fn try_cached_sync(&self) -> Result<(SyncManifest, WasmRuntime), String> {
        let manifest = self.cache.read_manifest()?;
        let wasm_bytes = self.cache.read_wasm()?;
        verify_bundle_signature(&wasm_bytes, &manifest.signature)?;
        let runtime = WasmRuntime::from_bytes(&wasm_bytes)?;
        runtime.smoke_test()?;
        Ok((manifest, runtime))
    }
}

#[derive(Debug)]
struct CacheManager {
    root: PathBuf,
}

impl CacheManager {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn ensure(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.root).map_err(|e| {
            format!(
                "failed to create evergreen cache directory '{}': {}",
                self.root.display(),
                e
            )
        })
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join(CACHE_MANIFEST_FILE)
    }

    fn wasm_path(&self) -> PathBuf {
        self.root.join(CACHE_WASM_FILE)
    }

    fn write_manifest(&self, manifest: &SyncManifest) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(manifest)
            .map_err(|e| format!("failed to serialize sync manifest: {}", e))?;
        std::fs::write(self.manifest_path(), bytes)
            .map_err(|e| format!("failed to write sync manifest cache: {}", e))
    }

    fn read_manifest(&self) -> Result<SyncManifest, String> {
        let bytes = std::fs::read(self.manifest_path())
            .map_err(|e| format!("failed to read cached sync manifest: {}", e))?;
        serde_json::from_slice::<SyncManifest>(&bytes)
            .map_err(|e| format!("failed to parse cached sync manifest: {}", e))
    }

    fn write_wasm(&self, wasm_bytes: &[u8]) -> Result<(), String> {
        std::fs::write(self.wasm_path(), wasm_bytes)
            .map_err(|e| format!("failed to write cached wasm bundle: {}", e))
    }

    fn read_wasm(&self) -> Result<Vec<u8>, String> {
        std::fs::read(self.wasm_path()).map_err(|e| format!("failed to read cached wasm: {}", e))
    }
}

struct WasmRuntime {
    inner: Mutex<WasmRuntimeInner>,
}

struct WasmRuntimeInner {
    store: Store<()>,
    memory: Memory,
    alloc_samples: TypedFunc<i32, i32>,
    alloc_bytes: TypedFunc<i32, i32>,
    process: TypedFunc<(i32, i32, i32), ()>,
    set_sample_rate: TypedFunc<f32, i32>,
    set_chain_json: TypedFunc<(i32, i32), i32>,
    set_param_json: TypedFunc<(i32, i32), i32>,
}

impl WasmRuntime {
    fn from_bytes(wasm_bytes: &[u8]) -> Result<Self, String> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("wasm module load failed: {}", e))?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| format!("wasm instance creation failed: {}", e))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| "wasm export 'memory' is missing".to_string())?;
        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|e| format!("wasm export 'alloc' is missing or invalid: {}", e))?;
        let alloc_bytes = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc_bytes")
            .unwrap_or_else(|_| alloc.clone());
        let process = instance
            .get_typed_func::<(i32, i32, i32), ()>(&mut store, "process")
            .map_err(|e| format!("wasm export 'process' is missing or invalid: {}", e))?;
        let set_sample_rate = instance
            .get_typed_func::<f32, i32>(&mut store, "set_sample_rate")
            .map_err(|e| format!("wasm export 'set_sample_rate' is missing or invalid: {}", e))?;
        let set_chain_json = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "set_chain_json")
            .map_err(|e| format!("wasm export 'set_chain_json' is missing or invalid: {}", e))?;
        let set_param_json = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "set_param_json")
            .map_err(|e| format!("wasm export 'set_param_json' is missing or invalid: {}", e))?;

        Ok(Self {
            inner: Mutex::new(WasmRuntimeInner {
                store,
                memory,
                alloc_samples: alloc,
                alloc_bytes,
                process,
                set_sample_rate,
                set_chain_json,
                set_param_json,
            }),
        })
    }

    fn smoke_test(&self) -> Result<(), String> {
        let input = [0.0f32, 0.0f32, 0.2f32, -0.2f32, -0.4f32, 0.4f32];
        let mut output = [0.0f32; 6];
        self.process_interleaved_stereo(&input, &mut output)?;
        if output.iter().all(|sample| sample.is_finite()) {
            Ok(())
        } else {
            Err("wasm smoke test produced non-finite samples".to_string())
        }
    }

    fn process_interleaved_stereo(&self, input: &[f32], output: &mut [f32]) -> Result<(), String> {
        if input.len() != output.len() {
            return Err("wasm runtime expected output length to match input length".to_string());
        }
        if input.len() % 2 != 0 {
            return Err(
                "wasm runtime expects interleaved stereo input (even sample count)".to_string(),
            );
        }

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "failed to lock wasm runtime".to_string())?;
        let frame_count = (input.len() / 2) as i32;
        let alloc = inner.alloc_samples.clone();
        let process = inner.process.clone();
        let memory = inner.memory;

        let input_ptr = alloc
            .call(&mut inner.store, input.len() as i32)
            .map_err(|e| format!("wasm alloc(input) failed: {}", e))?;
        let output_ptr = alloc
            .call(&mut inner.store, output.len() as i32)
            .map_err(|e| format!("wasm alloc(output) failed: {}", e))?;

        {
            let memory_data = memory.data_mut(&mut inner.store);
            write_f32_slice(memory_data, input_ptr as usize, input)?;
        }

        process
            .call(&mut inner.store, (input_ptr, output_ptr, frame_count))
            .map_err(|e| format!("wasm process call failed: {}", e))?;

        {
            let memory_data = memory.data(&inner.store);
            read_f32_slice(memory_data, output_ptr as usize, output)?;
        }

        Ok(())
    }

    fn set_sample_rate(&self, sample_rate: f32) -> Result<(), String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "failed to lock wasm runtime".to_string())?;
        let set_sample_rate = inner.set_sample_rate.clone();
        let status = set_sample_rate
            .call(&mut inner.store, sample_rate)
            .map_err(|e| format!("wasm set_sample_rate failed: {}", e))?;
        if status == 0 {
            Ok(())
        } else {
            Err(format!("wasm set_sample_rate returned status {}", status))
        }
    }

    fn set_chain_json(&self, chain_json: &str) -> Result<(), String> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "failed to lock wasm runtime".to_string())?;
        let alloc_bytes = inner.alloc_bytes.clone();
        let set_chain_json = inner.set_chain_json.clone();
        let memory = inner.memory;
        let payload = chain_json.as_bytes();
        let payload_len_i32 =
            i32::try_from(payload.len()).map_err(|_| "chain JSON payload too large".to_string())?;

        let payload_ptr = alloc_bytes
            .call(&mut inner.store, payload_len_i32)
            .map_err(|e| format!("wasm alloc(chain_json) failed: {}", e))?;

        {
            let memory_data = memory.data_mut(&mut inner.store);
            write_byte_slice(memory_data, payload_ptr as usize, payload)?;
        }

        let status = set_chain_json
            .call(&mut inner.store, (payload_ptr, payload_len_i32))
            .map_err(|e| format!("wasm set_chain_json failed: {}", e))?;
        if status == 0 {
            Ok(())
        } else {
            Err(format!("wasm set_chain_json returned status {}", status))
        }
    }

    fn set_param_json(&self, effect_idx: i32, key: &str, value: f32) -> Result<(), String> {
        let payload = serde_json::json!({
            "index": effect_idx,
            "param_key": key,
            "value": value
        });
        let payload = serde_json::to_vec(&payload)
            .map_err(|e| format!("failed to serialize wasm param JSON payload: {}", e))?;

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "failed to lock wasm runtime".to_string())?;
        let alloc_bytes = inner.alloc_bytes.clone();
        let set_param_json = inner.set_param_json.clone();
        let memory = inner.memory;
        let payload_len_i32 =
            i32::try_from(payload.len()).map_err(|_| "param JSON payload too large".to_string())?;

        let payload_ptr = alloc_bytes
            .call(&mut inner.store, payload_len_i32)
            .map_err(|e| format!("wasm alloc(param_json) failed: {}", e))?;

        {
            let memory_data = memory.data_mut(&mut inner.store);
            write_byte_slice(memory_data, payload_ptr as usize, &payload)?;
        }

        let status = set_param_json
            .call(&mut inner.store, (payload_ptr, payload_len_i32))
            .map_err(|e| format!("wasm set_param_json failed: {}", e))?;
        if status == 0 {
            Ok(())
        } else {
            Err(format!("wasm set_param_json returned status {}", status))
        }
    }
}

fn write_f32_slice(memory: &mut [u8], ptr: usize, values: &[f32]) -> Result<(), String> {
    let byte_len = values
        .len()
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(|| "input byte length overflow".to_string())?;
    let end = ptr
        .checked_add(byte_len)
        .ok_or_else(|| "input pointer overflow".to_string())?;
    if end > memory.len() {
        return Err("input write out of wasm memory bounds".to_string());
    }

    for (index, value) in values.iter().enumerate() {
        let offset = ptr + index * std::mem::size_of::<f32>();
        memory[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    Ok(())
}

fn write_byte_slice(memory: &mut [u8], ptr: usize, values: &[u8]) -> Result<(), String> {
    let end = ptr
        .checked_add(values.len())
        .ok_or_else(|| "byte payload pointer overflow".to_string())?;
    if end > memory.len() {
        return Err("byte payload write out of wasm memory bounds".to_string());
    }
    memory[ptr..end].copy_from_slice(values);
    Ok(())
}

fn read_f32_slice(memory: &[u8], ptr: usize, output: &mut [f32]) -> Result<(), String> {
    let byte_len = output
        .len()
        .checked_mul(std::mem::size_of::<f32>())
        .ok_or_else(|| "output byte length overflow".to_string())?;
    let end = ptr
        .checked_add(byte_len)
        .ok_or_else(|| "output pointer overflow".to_string())?;
    if end > memory.len() {
        return Err("output read out of wasm memory bounds".to_string());
    }

    for (index, slot) in output.iter_mut().enumerate() {
        let offset = ptr + index * std::mem::size_of::<f32>();
        let bytes = [
            memory[offset],
            memory[offset + 1],
            memory[offset + 2],
            memory[offset + 3],
        ];
        *slot = f32::from_le_bytes(bytes);
    }
    Ok(())
}

fn fetch_sync_manifest(sync_url: &str) -> Result<SyncManifest, String> {
    let bytes = download_bytes(sync_url)?;
    let manifest = serde_json::from_slice::<SyncManifest>(&bytes)
        .map_err(|e| format!("failed to parse sync manifest JSON: {}", e))?;
    if manifest.version.trim().is_empty() {
        return Err("sync manifest has empty version".to_string());
    }
    Ok(manifest)
}

fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(map_ureq_error)?;

    let mut reader = response.into_reader();
    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .map_err(|e| format!("failed to read response body from '{}': {}", url, e))?;
    Ok(buffer)
}

fn map_ureq_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, _) => format!("HTTP status {}", code),
        ureq::Error::Transport(err) => format!("transport error: {}", err),
    }
}

fn verify_bundle_signature(bundle_bytes: &[u8], signature_b64: &str) -> Result<(), String> {
    let allow_unsigned = env_bool(ENV_ALLOW_UNSIGNED).unwrap_or(cfg!(debug_assertions));
    let public_key_b64 = resolve_public_key_b64();
    let signature = signature_b64.trim();

    if public_key_b64.is_empty() || signature.is_empty() {
        if allow_unsigned {
            return Ok(());
        }
        return Err("missing Ed25519 key/signature and unsigned bundles are disabled".to_string());
    }

    let public_key = base64::engine::general_purpose::STANDARD
        .decode(public_key_b64.as_bytes())
        .map_err(|e| format!("invalid base64 public key: {}", e))?;
    let signature = base64::engine::general_purpose::STANDARD
        .decode(signature.as_bytes())
        .map_err(|e| format!("invalid base64 signature: {}", e))?;

    let verifier = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key);
    verifier
        .verify(bundle_bytes, &signature)
        .map_err(|_| "Ed25519 signature verification failed".to_string())
}

fn resolve_sync_url() -> String {
    read_env_non_empty(ENV_SYNC_URL).unwrap_or_else(|| DEFAULT_SYNC_URL.to_string())
}

fn resolve_public_key_b64() -> String {
    read_env_non_empty(ENV_PUBLIC_KEY_B64)
        .or_else(|| {
            EMBEDDED_PUBLIC_KEY_B64
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn evergreen_enabled() -> bool {
    env_bool(ENV_ENABLED).unwrap_or(true)
}

fn read_env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_bool(name: &str) -> Option<bool> {
    let value = read_env_non_empty(name)?;
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn cache_root_for(data_dir: &Path) -> PathBuf {
    data_dir.join(CACHE_DIR_NAME)
}
