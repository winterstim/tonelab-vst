#![allow(unexpected_cfgs)]

use nih_plug::prelude::*;
use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
use wry::{
    http::{Request, Uri},
    WebView, WebViewBuilder,
};

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

struct ViewWrapper<'a>(&'a ParentWindowHandle);

impl<'a> HasWindowHandle for ViewWrapper<'a> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        use raw_window_handle::AppKitWindowHandle;
        use raw_window_handle_05::{HasRawWindowHandle as Rwh05, RawWindowHandle as Raw05};

        let handle_05 = Rwh05::raw_window_handle(self.0);

        match handle_05 {
            Raw05::AppKit(kit05) => {
                let kit06 = AppKitWindowHandle::new(
                    std::ptr::NonNull::new(kit05.ns_view)
                        .expect("ViewWrapper: Null NSView from RawWindowHandle"),
                );
                let raw = RawWindowHandle::AppKit(kit06);
                unsafe { Ok(WindowHandle::borrow_raw(raw)) }
            }
            Raw05::Win32(win05) => {
                use raw_window_handle::Win32WindowHandle;
                let mut win06 = Win32WindowHandle::new(
                    std::num::NonZeroIsize::new(win05.hwnd as isize)
                        .expect("ViewWrapper: Null HWND"),
                );
                win06.hinstance = std::num::NonZeroIsize::new(win05.hinstance as isize);
                let raw = RawWindowHandle::Win32(win06);
                unsafe { Ok(WindowHandle::borrow_raw(raw)) }
            }
            Raw05::Xlib(xlib05) => {
                use raw_window_handle::XlibWindowHandle;
                let xlib06 = XlibWindowHandle::new(xlib05.window);
                let raw = RawWindowHandle::Xlib(xlib06);
                unsafe { Ok(WindowHandle::borrow_raw(raw)) }
            }
            Raw05::Xcb(xcb05) => {
                use raw_window_handle::XcbWindowHandle;
                let xcb06 = XcbWindowHandle::new(
                    std::num::NonZeroU32::new(xcb05.window).expect("ViewWrapper: Null XCB window"),
                );
                let raw = RawWindowHandle::Xcb(xcb06);
                unsafe { Ok(WindowHandle::borrow_raw(raw)) }
            }
            Raw05::Wayland(wayland05) => {
                use raw_window_handle::WaylandWindowHandle;
                let wayland06 = WaylandWindowHandle::new(
                    std::ptr::NonNull::new(wayland05.surface)
                        .expect("ViewWrapper: Null Wayland surface"),
                );
                let raw = RawWindowHandle::Wayland(wayland06);
                unsafe { Ok(WindowHandle::borrow_raw(raw)) }
            }
            _ => panic!("Unsupported platform for this shim"),
        }
    }
}

pub mod device;
pub mod evergreen;
use evergreen::EvergreenEngine;

const PLUGIN_VENDOR_URL: &str = match option_env!("TONELAB_VENDOR_URL") {
    Some(value) => value,
    None => "https://tonelab.dev",
};

const PLUGIN_SUPPORT_EMAIL: &str = match option_env!("TONELAB_SUPPORT_EMAIL") {
    Some(value) => value,
    None => "tonelabvst@gmail.com",
};

const DEFAULT_API_BASE_URL: &str = match option_env!("TONELAB_DEFAULT_API_BASE_URL") {
    Some(value) => value,
    None => "https://robust-dulciana-tonelab-49d88bd9.koyeb.app/api/v1",
};

const DEFAULT_WEB_BASE_URL: &str = match option_env!("TONELAB_DEFAULT_WEB_BASE_URL") {
    Some(value) => value,
    None => "https://tonelab.dev",
};

const DEFAULT_API_PREFIX: &str = match option_env!("TONELAB_DEFAULT_API_PREFIX") {
    Some(value) => value,
    None => "",
};

const TOKEN_STORAGE_KEY: &str = "tonelab_auth_token";
#[cfg(target_os = "windows")]
const WINDOWS_TOKEN_REGISTRY_PATH: &str = "Software\\Tonelab\\VST";
#[cfg(target_os = "windows")]
const WINDOWS_TOKEN_REGISTRY_VALUE: &str = "auth_token";
const DEFAULT_LOG_FILE_NAME: &str = "tonelab_vst.log";
const ENV_API_BASE_URL: &str = "TONELAB_API_BASE_URL";
const ENV_WEB_BASE_URL: &str = "TONELAB_WEB_BASE_URL";
const ENV_FRONTEND_URL: &str = "FRONTEND_URL";
const ENV_API_PREFIX: &str = "TONELAB_API_PREFIX";
const ENV_EVERGREEN_WEB_UI_URL: &str = "TONELAB_EVERGREEN_WEB_UI_URL";
const ENV_ALLOWED_EXTERNAL_HOSTS: &str = "TONELAB_ALLOWED_EXTERNAL_HOSTS";
const ENV_ENABLE_DEVTOOLS: &str = "TONELAB_ENABLE_DEVTOOLS";
const ENV_LOG_FILE_PATH: &str = "TONELAB_LOG_FILE_PATH";
const DEFAULT_EVERGREEN_WEB_UI_URL: &str = "";

fn read_env_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn normalize_api_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed)
    }
}

fn resolve_api_base_url() -> String {
    read_env_non_empty(ENV_API_BASE_URL)
        .map(|value| normalize_base_url(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| normalize_base_url(DEFAULT_API_BASE_URL))
}

fn resolve_web_base_url() -> String {
    read_env_non_empty(ENV_WEB_BASE_URL)
        .or_else(|| read_env_non_empty(ENV_FRONTEND_URL))
        .map(|value| normalize_base_url(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| normalize_base_url(DEFAULT_WEB_BASE_URL))
}

fn resolve_api_prefix() -> String {
    read_env_non_empty(ENV_API_PREFIX)
        .map(|value| normalize_api_prefix(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| normalize_api_prefix(DEFAULT_API_PREFIX))
}

fn is_http_or_https_url(value: &str) -> bool {
    let parsed = match value.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => return false,
    };
    let scheme = parsed
        .scheme_str()
        .map(|entry| entry.to_ascii_lowercase())
        .unwrap_or_default();
    (scheme == "http" || scheme == "https") && parsed.host().is_some()
}

fn resolve_evergreen_ui_url(preferred: Option<&str>) -> String {
    let preferred = preferred
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let env_override = read_env_non_empty(ENV_EVERGREEN_WEB_UI_URL);
    let candidate = preferred
        .or(env_override)
        .unwrap_or_else(|| DEFAULT_EVERGREEN_WEB_UI_URL.to_string());
    if is_http_or_https_url(&candidate) {
        candidate
    } else {
        "about:blank".to_string()
    }
}

fn env_bool(name: &str) -> Option<bool> {
    let value = read_env_non_empty(name)?;
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn webview_devtools_enabled() -> bool {
    env_bool(ENV_ENABLE_DEVTOOLS).unwrap_or(cfg!(debug_assertions))
}

fn normalize_host(host: &str) -> String {
    host.trim().trim_start_matches('.').to_ascii_lowercase()
}

fn parse_url_host(url: &str) -> Option<String> {
    let parsed = url.parse::<Uri>().ok()?;
    parsed.host().map(normalize_host)
}

fn is_host_allowed(host: &str, allowed_hosts: &HashSet<String>) -> bool {
    allowed_hosts
        .iter()
        .any(|allowed| host == allowed || host.ends_with(&format!(".{}", allowed)))
}

fn resolve_allowed_external_hosts() -> HashSet<String> {
    if let Some(configured) = read_env_non_empty(ENV_ALLOWED_EXTERNAL_HOSTS) {
        let parsed: HashSet<String> = configured
            .split(',')
            .map(normalize_host)
            .filter(|host| !host.is_empty())
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    [
        resolve_web_base_url(),
        resolve_api_base_url(),
        <TonelabPlugin as Plugin>::URL.to_string(),
    ]
    .into_iter()
    .filter_map(|url| parse_url_host(&url))
    .collect()
}

fn external_url_for_log(url: &str) -> String {
    let parsed = match url.parse::<Uri>() {
        Ok(value) => value,
        Err(_) => return "<invalid>".to_string(),
    };
    let scheme = parsed.scheme_str().unwrap_or("?");
    let host = parsed.host().unwrap_or("?");
    format!("{}://{}", scheme, host)
}

fn is_allowed_external_url(url: &str) -> bool {
    let parsed = match url.parse::<Uri>() {
        Ok(value) => value,
        Err(_) => return false,
    };

    let scheme = parsed
        .scheme_str()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if scheme != "http" && scheme != "https" {
        return false;
    }

    let host = match parsed.host() {
        Some(value) => normalize_host(value),
        None => return false,
    };

    let allowed_hosts = resolve_allowed_external_hosts();
    if allowed_hosts.is_empty() {
        return false;
    }

    is_host_allowed(&host, &allowed_hosts)
}

fn log_file_path() -> std::path::PathBuf {
    if let Some(path) = read_env_non_empty(ENV_LOG_FILE_PATH) {
        return std::path::PathBuf::from(path);
    }
    std::env::temp_dir().join(DEFAULT_LOG_FILE_NAME)
}

fn open_external_url(url: &str) {
    if !is_allowed_external_url(url) {
        log_to_file(&format!(
            "Blocked external URL outside allowlist: {}",
            external_url_for_log(url)
        ));
        return;
    }

    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();

    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("rundll32.exe")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .spawn();

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();

    if let Err(e) = result {
        eprintln!(
            "Failed to open external URL '{}': {}",
            external_url_for_log(url),
            e
        );
        log_to_file(&format!(
            "Failed to open external URL {}: {}",
            external_url_for_log(url),
            e
        ));
    }
}

pub struct TonelabPlugin {
    params: Arc<TonelabParams>,
    evergreen_engine: Arc<Mutex<EvergreenEngine>>,
    sample_rate: f32,
}

const EDITOR_WIDTH: u32 = 800;
const EDITOR_HEIGHT: u32 = 600;

#[derive(Params)]
struct TonelabParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for TonelabParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Output Gain",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            ),
        }
    }
}

impl Default for TonelabPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(TonelabParams::default()),
            evergreen_engine: Arc::new(Mutex::new(EvergreenEngine::new(get_data_dir()))),
            sample_rate: 44100.0,
        }
    }
}

impl TonelabPlugin {
    fn bypass_sample(sample: f32, gain: f32) -> f32 {
        sample * gain
    }
}

impl Vst3Plugin for TonelabPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"TonelabAudioFx03";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

impl Plugin for TonelabPlugin {
    const NAME: &'static str = concat!("Tonelab VST v", env!("CARGO_PKG_VERSION"));
    const VENDOR: &'static str = "Tonelab";
    const URL: &'static str = PLUGIN_VENDOR_URL;
    const EMAIL: &'static str = PLUGIN_SUPPORT_EMAIL;

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let (evergreen_web_ui_url, evergreen_icons_url, evergreen_effects_url) = self
            .evergreen_engine
            .lock()
            .ok()
            .map(|engine| {
                (
                    engine.web_ui_url().map(|value| value.to_string()),
                    engine.icons_url().map(|value| value.to_string()),
                    engine.effects_url().map(|value| value.to_string()),
                )
            })
            .unwrap_or((None, None, None));

        Some(Box::new(TonelabEditor {
            params: self.params.clone(),
            evergreen_engine: self.evergreen_engine.clone(),
            evergreen_web_ui_url,
            evergreen_icons_url,
            evergreen_effects_url,
            scale_factor_bits: AtomicU32::new(1.0f32.to_bits()),
            is_open: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;

        if let Ok(mut evergreen_engine) = self.evergreen_engine.lock() {
            evergreen_engine.set_sample_rate(self.sample_rate);

            if let Err(error) = evergreen_engine.bootstrap() {
                log_to_file(&format!("Evergreen bootstrap failed: {}", error));
            } else {
                if let Some(version) = evergreen_engine.active_version() {
                    log_to_file(&format!("Evergreen bundle active: {}", version));
                }
                if let Some(error) = evergreen_engine.last_error() {
                    log_to_file(error);
                }
            }
        }

        true
    }

    fn reset(&mut self) {
        if let Ok(mut evergreen_engine) = self.evergreen_engine.lock() {
            evergreen_engine.set_sample_rate(self.sample_rate);
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut evergreen_guard = self.evergreen_engine.try_lock().ok();
        let mut evergreen_ready = evergreen_guard
            .as_ref()
            .map(|engine| engine.has_runtime())
            .unwrap_or(false);

        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.value();

            // Safety: We expect exactly 2 channels (stereo).
            // We use into_iter() to get mutable references to the samples avoiding multiple mutable borrows of the container.
            let mut channels = channel_samples.into_iter();
            let (l, r) = match (channels.next(), channels.next()) {
                (Some(l), Some(r)) => (l, r),
                (Some(l), None) => {
                    let in_l = *l;
                    if evergreen_ready {
                        if let Some(engine) = evergreen_guard.as_mut() {
                            match engine.process_frame(in_l, in_l) {
                                Ok((out_l, _)) => {
                                    *l = out_l * gain;
                                    continue;
                                }
                                Err(error) => {
                                    log_to_file(&format!(
                                        "Evergreen process failed, switching to bypass for this block: {}",
                                        error
                                    ));
                                    evergreen_ready = false;
                                }
                            }
                        }
                    }
                    *l = Self::bypass_sample(in_l, gain);
                    continue;
                }
                _ => continue,
            };

            let in_l = *l;
            let in_r = *r;
            if evergreen_ready {
                if let Some(engine) = evergreen_guard.as_mut() {
                    match engine.process_frame(in_l, in_r) {
                        Ok((out_l, out_r)) => {
                            *l = out_l * gain;
                            *r = out_r * gain;
                            continue;
                        }
                        Err(error) => {
                            log_to_file(&format!(
                                "Evergreen process failed, switching to bypass for this block: {}",
                                error
                            ));
                            evergreen_ready = false;
                        }
                    }
                }
            }

            *l = Self::bypass_sample(in_l, gain);
            *r = Self::bypass_sample(in_r, gain);
        }

        ProcessStatus::Normal
    }
}

#[allow(unexpected_cfgs)]
fn save_token_globally(token: &str) {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc::runtime::{Class, Object};
        let cls = Class::get("NSUserDefaults").expect("Failed to get NSUserDefaults class");
        let defaults: *mut Object = msg_send![cls, standardUserDefaults];

        let cls_nsstring = Class::get("NSString").expect("Failed to get NSString class");
        let key_str = std::ffi::CString::new(TOKEN_STORAGE_KEY).expect("CString::new failed");
        let val_str = std::ffi::CString::new(token).expect("CString::new failed");

        let key: *mut Object = msg_send![cls_nsstring, stringWithUTF8String: key_str.as_ptr()];
        let value: *mut Object = msg_send![cls_nsstring, stringWithUTF8String: val_str.as_ptr()];

        let _: () = msg_send![defaults, setObject: value forKey: key];
        let _: () = msg_send![defaults, synchronize];
        log_to_file("Saved token to NSUserDefaults");
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok((key, _)) = hkcu.create_subkey(WINDOWS_TOKEN_REGISTRY_PATH) {
            let _ = key.set_value(WINDOWS_TOKEN_REGISTRY_VALUE, &token);
            log_to_file("Saved token to Registry");
        } else {
            log_to_file("Failed to open Registry key for writing");
        }
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let path = get_token_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, token);
        log_to_file(&format!(
            "Saved token to ~/.tonelab_auth_token: {} chars",
            token.len()
        ));
    }
}

#[allow(unexpected_cfgs)]
fn load_token_globally() -> String {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc::runtime::{Class, Object};
        use std::ffi::CStr;

        let cls = Class::get("NSUserDefaults").expect("Failed to get NSUserDefaults class");
        let defaults: *mut Object = msg_send![cls, standardUserDefaults];

        let cls_nsstring = Class::get("NSString").expect("Failed to get NSString class");
        let key_str = std::ffi::CString::new(TOKEN_STORAGE_KEY).expect("CString::new failed");
        let key: *mut Object = msg_send![cls_nsstring, stringWithUTF8String: key_str.as_ptr()];

        let value: *mut Object = msg_send![defaults, objectForKey: key];

        if !value.is_null() {
            let utf8: *const i8 = msg_send![value, UTF8String];
            if !utf8.is_null() {
                let s = CStr::from_ptr(utf8).to_string_lossy().into_owned();
                log_to_file(&format!(
                    "Loaded token from NSUserDefaults: {} chars",
                    s.len()
                ));
                return s;
            }
        }

        String::new()
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(WINDOWS_TOKEN_REGISTRY_PATH) {
            let token: String = key
                .get_value(WINDOWS_TOKEN_REGISTRY_VALUE)
                .unwrap_or_default();
            log_to_file(&format!(
                "Loaded token from Registry: {} chars",
                token.len()
            ));
            return token;
        }
        String::new()
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let path = get_token_path();
        std::fs::read_to_string(path).unwrap_or_default()
    }
}

fn log_to_file(msg: &str) {
    use std::io::Write;
    let path = log_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(file, "{}", msg);
    }
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn get_token_path() -> std::path::PathBuf {
    // simplified path: ~/.tonelab_auth_token
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().join(".tonelab_auth_token"))
        .unwrap_or_else(|| std::env::temp_dir().join("tonelab_vst_token"))
}

fn get_data_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("com", "tonelab", "tonelab_vst")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("tonelab_vst_data"))
}

struct TonelabEditor {
    #[allow(dead_code)] // params are kept alive by Arc but not read directly in Editor struct
    params: Arc<TonelabParams>,
    evergreen_engine: Arc<Mutex<EvergreenEngine>>,
    evergreen_web_ui_url: Option<String>,
    evergreen_icons_url: Option<String>,
    evergreen_effects_url: Option<String>,
    scale_factor_bits: AtomicU32,
    is_open: Arc<AtomicBool>,
}

impl TonelabEditor {
    const WIDTH: u32 = EDITOR_WIDTH;
    const HEIGHT: u32 = EDITOR_HEIGHT;

    fn scale_factor(&self) -> f32 {
        let factor = f32::from_bits(self.scale_factor_bits.load(Ordering::Relaxed));
        if factor.is_finite() && factor > 0.0 {
            factor
        } else {
            1.0
        }
    }

    fn scaled_editor_size(&self) -> (u32, u32) {
        let factor = self.scale_factor();
        let width = ((Self::WIDTH as f32 * factor).round() as u32).max(1);
        let height = ((Self::HEIGHT as f32 * factor).round() as u32).max(1);
        (width, height)
    }
}

struct TonelabEditorHandle {
    #[allow(dead_code)]
    webview: WebView,
    is_open: Arc<AtomicBool>,
}

unsafe impl Send for TonelabEditorHandle {}

impl Drop for TonelabEditorHandle {
    fn drop(&mut self) {
        self.is_open.store(false, Ordering::Release);
    }
}

impl Editor for TonelabEditor {
    #[allow(unexpected_cfgs, deprecated)]
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        _context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let evergreen_engine = self.evergreen_engine.clone();

        let device_info = device::get_current_device_info();
        let device_info_json =
            serde_json::to_string(&device_info).unwrap_or_else(|_| "{}".to_string());

        // Load saved token
        let saved_token = load_token_globally();
        let api_base_url = resolve_api_base_url();
        let web_base_url = resolve_web_base_url();
        let api_prefix = resolve_api_prefix();
        let plugin_version = <TonelabPlugin as Plugin>::VERSION;
        let evergreen_web_ui_url = self.evergreen_web_ui_url.clone().unwrap_or_default();
        let evergreen_icons_url = self.evergreen_icons_url.clone().unwrap_or_default();
        let evergreen_effects_url = self.evergreen_effects_url.clone().unwrap_or_default();
        let ui_url = resolve_evergreen_ui_url(Some(&evergreen_web_ui_url));

        let init_script = format!(
            "window.DEVICE_INFO = {}; window.RUST_AUTH_TOKEN = {:?}; window.TONELAB_API_BASE_URL = {:?}; window.TONELAB_WEB_BASE_URL = {:?}; window.TONELAB_API_PREFIX = {:?}; window.TONELAB_PLUGIN_VERSION = {:?}; window.TONELAB_EVERGREEN_WEB_UI_URL = {:?}; window.TONELAB_EVERGREEN_ICONS_URL = {:?}; window.TONELAB_EVERGREEN_EFFECTS_URL = {:?}; window.TONELAB_RUNTIME_ENV = 'vst-embedded';",
            device_info_json,
            saved_token,
            api_base_url,
            web_base_url,
            api_prefix,
            plugin_version,
            evergreen_web_ui_url,
            evergreen_icons_url,
            evergreen_effects_url
        );

        let wrapper = ViewWrapper(&parent);

        // Create a custom WebContext with a writable user data directory.
        // This is CRITICAL for Windows VSTs plugins located in read-only "Program Files" directories.
        let data_dir = get_data_dir();
        let mut context = wry::WebContext::new(Some(data_dir));
        let (scaled_width, scaled_height) = self.scaled_editor_size();

        let webview_result = WebViewBuilder::new_as_child(&wrapper)
            .with_web_context(&mut context)
            .with_devtools(webview_devtools_enabled())
            .with_url(&ui_url)
            .with_initialization_script(&init_script)
            // Initialize child bounds from logical editor size * host scale factor.
            // This keeps plugin UI placement/sizing aligned with host expectations on Windows/Linux HiDPI.
            .with_bounds(wry::Rect {
                position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                size: wry::dpi::PhysicalSize::new(scaled_width, scaled_height).into(),
            })
            .with_transparent(false)
            .with_visible(true)
            .with_background_color((30, 30, 30, 255)) // Dark grey background
            .with_ipc_handler(move |req: Request<String>| {
                let msg = req.body();
                let apply_chain = |chain_json: &str, evergreen_engine: &Arc<Mutex<EvergreenEngine>>| {
                    if let Ok(mut engine) = evergreen_engine.lock() {
                        if !engine.has_runtime() {
                            if let Err(error) = engine.bootstrap() {
                                log_to_file(&format!("IPC sync_chain bootstrap failed: {}", error));
                                return;
                            }
                        }
                        if let Err(error) = engine.sync_chain_json(chain_json) {
                            log_to_file(&format!("IPC sync_chain apply failed: {}", error));
                        }
                    } else {
                        log_to_file("IPC sync_chain: failed to lock evergreen engine");
                    }
                };

                if let Ok(value) = serde_json::from_str::<Value>(msg) {
                    if value.is_array() {
                        apply_chain(msg, &evergreen_engine);
                    } else if value.is_object() {
                        if let Some(msg_type) = value.get("type").and_then(|v| v.as_str()) {
                            if msg_type == "sync_chain" {
                                if let Some(chain_data) = value.get("data") {
                                    if let Ok(chain_json) = serde_json::to_string(chain_data) {
                                        apply_chain(&chain_json, &evergreen_engine);
                                    }
                                }
                            } else if msg_type == "param_change" {
                                if let (Some(index), Some(key), Some(val)) = (
                                    value.get("index").and_then(|v| v.as_u64()),
                                    value.get("param_key").and_then(|v| v.as_str()),
                                    value.get("value").and_then(|v| v.as_f64()),
                                ) {
                                    if let Ok(mut engine) = evergreen_engine.lock() {
                                        if !engine.has_runtime() {
                                            if let Err(error) = engine.bootstrap() {
                                                log_to_file(&format!(
                                                    "IPC param_change bootstrap failed: {}",
                                                    error
                                                ));
                                                return;
                                            }
                                        }
                                        if let Err(error) =
                                            engine.set_param(index as i32, key, val as f32)
                                        {
                                            log_to_file(&format!(
                                                "IPC param_change apply failed: index={} key={} error={}",
                                                index, key, error
                                            ));
                                        }
                                    } else {
                                        log_to_file("IPC param_change: failed to lock evergreen engine");
                                    }
                                }
                            } else if msg_type == "open_external_url" {
                                if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                                    open_external_url(url);
                                }
                            } else if msg_type == "log" {
                                if let Some(message) = value.get("message").and_then(|v| v.as_str())
                                {
                                    #[cfg(unix)]
                                    log_to_file(&format!("JS: {}", message));
                                }
                            } else if msg_type == "save_token" {
                                if let Some(token) = value.get("token").and_then(|v| v.as_str()) {
                                    save_token_globally(token);
                                }
                            }
                        }
                    }
                }
            })
            .build();

        match webview_result {
            Ok(webview) => {
                #[cfg(target_os = "macos")]
                unsafe {
                    use cocoa::appkit::NSView;
                    use cocoa::base::id;
                    use raw_window_handle_05::HasRawWindowHandle;

                    let parent_handle = parent.raw_window_handle();
                    if let raw_window_handle_05::RawWindowHandle::AppKit(kit) = parent_handle {
                        let parent_view = kit.ns_view as id;
                        let parent_frame = NSView::frame(parent_view);

                        let subviews: id = msg_send![parent_view, subviews];
                        let count: usize = msg_send![subviews, count];
                        if count > 0 {
                            let webview_nsview: id = msg_send![subviews, lastObject];

                            let _: () = msg_send![webview_nsview, setFrame: parent_frame];

                            let _: () = msg_send![webview_nsview, setAutoresizingMask: 18u64];
                        }
                    }
                }

                self.is_open.store(true, Ordering::Release);
                Box::new(TonelabEditorHandle {
                    webview,
                    is_open: self.is_open.clone(),
                })
            }
            Err(e) => {
                eprintln!("Failed to create webview: {}", e);
                Box::new(())
            }
        }
    }

    fn size(&self) -> (u32, u32) {
        (Self::WIDTH, Self::HEIGHT)
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        if self.is_open.load(Ordering::Acquire) {
            return false;
        }

        if !factor.is_finite() || factor <= 0.0 {
            return false;
        }

        self.scale_factor_bits
            .store(factor.to_bits(), Ordering::Relaxed);
        true
    }
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
    fn param_values_changed(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_api_prefix_handles_common_forms() {
        assert_eq!(normalize_api_prefix("api/v1"), "/api/v1");
        assert_eq!(normalize_api_prefix("/api/v1/"), "/api/v1");
        assert_eq!(normalize_api_prefix(""), "");
    }

    #[test]
    fn host_allowlist_accepts_exact_and_subdomain_matches() {
        let mut allowed = HashSet::new();
        allowed.insert("tonelab.audio".to_string());

        assert!(is_host_allowed("tonelab.audio", &allowed));
        assert!(is_host_allowed("api.tonelab.audio", &allowed));
        assert!(!is_host_allowed("eviltonelab.audio", &allowed));
    }
}

nih_export_vst3!(TonelabPlugin);
