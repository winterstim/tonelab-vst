#![allow(unexpected_cfgs)]

use arc_swap::ArcSwap;
use nih_plug::prelude::*;
use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
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
pub mod dsp;
use dsp::Chain;

const PLUGIN_VENDOR_URL: &str = match option_env!("TONELAB_VENDOR_URL") {
    Some(value) => value,
    None => "https://tonelab-ai.vercel.app",
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
    None => "https://tonelab-ai.vercel.app",
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
const ENV_ALLOWED_EXTERNAL_HOSTS: &str = "TONELAB_ALLOWED_EXTERNAL_HOSTS";
const ENV_ENABLE_DEVTOOLS: &str = "TONELAB_ENABLE_DEVTOOLS";
const ENV_LOG_FILE_PATH: &str = "TONELAB_LOG_FILE_PATH";

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
    let result = std::process::Command::new("explorer.exe").arg(url).spawn();

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
    chain_state: Arc<ArcSwap<Chain>>,
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
        let default_chain = Chain::new();

        Self {
            params: Arc::new(TonelabParams::default()),
            chain_state: Arc::new(ArcSwap::from_pointee(default_chain)),
            sample_rate: 44100.0,
        }
    }
}

impl TonelabPlugin {
    #[inline]
    fn process_stereo_frame(chain: &Chain, in_l: f32, in_r: f32, gain: f32) -> (f32, f32) {
        let (out_l, out_r) = chain.process(in_l, in_r);
        (out_l * gain, out_r * gain)
    }

    #[inline]
    fn process_mono_frame(chain: &Chain, input: f32, gain: f32) -> f32 {
        let (out_l, _) = chain.process(input, input);
        out_l * gain
    }
}

impl Vst3Plugin for TonelabPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"TonelabAudioFx03";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

impl Plugin for TonelabPlugin {
    const NAME: &'static str = "Tonelab VST";
    const VENDOR: &'static str = "Tonelab";
    const URL: &'static str = PLUGIN_VENDOR_URL;
    const EMAIL: &'static str = PLUGIN_SUPPORT_EMAIL;

    const VERSION: &'static str = "0.1.0";

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
        Some(Box::new(TonelabEditor {
            params: self.params.clone(),
            chain_state: self.chain_state.clone(),
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

        let current = self.chain_state.load();
        if let Ok(json) = current.to_json() {
            if let Ok(mut new_chain) = Chain::from_json(&json) {
                new_chain.reset(self.sample_rate);
                self.chain_state.store(Arc::new(new_chain));
            }
        }

        true
    }

    fn reset(&mut self) {
        let current = self.chain_state.load();
        if let Ok(json) = current.to_json() {
            if let Ok(mut new_chain) = Chain::from_json(&json) {
                new_chain.reset(self.sample_rate);
                self.chain_state.store(Arc::new(new_chain));
            }
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let chain_guard = self.chain_state.load();

        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.value();

            // Safety: We expect exactly 2 channels (stereo).
            // We use into_iter() to get mutable references to the samples avoiding multiple mutable borrows of the container.
            let mut channels = channel_samples.into_iter();
            let (l, r) = match (channels.next(), channels.next()) {
                (Some(l), Some(r)) => (l, r),
                (Some(l), None) => {
                    let in_l = *l;
                    *l = Self::process_mono_frame(chain_guard.as_ref(), in_l, gain);
                    continue;
                }
                _ => continue,
            };

            let in_l = *l;
            let in_r = *r;
            let (out_l, out_r) = Self::process_stereo_frame(chain_guard.as_ref(), in_l, in_r, gain);

            *l = out_l;
            *r = out_r;
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
    chain_state: Arc<ArcSwap<Chain>>,
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
        let chain_state = self.chain_state.clone();

        const HTML_CONTENT: &str = include_str!("../ui/dist/index.html");

        let device_info = device::get_current_device_info();
        let device_info_json =
            serde_json::to_string(&device_info).unwrap_or_else(|_| "{}".to_string());

        // Load saved token
        let saved_token = load_token_globally();
        let api_base_url = resolve_api_base_url();
        let web_base_url = resolve_web_base_url();
        let api_prefix = resolve_api_prefix();
        let plugin_version = <TonelabPlugin as Plugin>::VERSION;

        let init_script = format!(
            "window.DEVICE_INFO = {}; window.RUST_AUTH_TOKEN = {:?}; window.TONELAB_API_BASE_URL = {:?}; window.TONELAB_WEB_BASE_URL = {:?}; window.TONELAB_API_PREFIX = {:?}; window.TONELAB_PLUGIN_VERSION = {:?};",
            device_info_json, saved_token, api_base_url, web_base_url, api_prefix, plugin_version
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
            .with_html(HTML_CONTENT)
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
                if let Ok(value) = serde_json::from_str::<Value>(msg) {
                    if value.is_array() {
                        match Chain::from_json(msg) {
                            Ok(mut new_chain) => {
                                new_chain.reset(44100.0);
                                chain_state.store(Arc::new(new_chain));
                            }
                            Err(_e) => {}
                        }
                    } else if value.is_object() {
                        if let Some(msg_type) = value.get("type").and_then(|v| v.as_str()) {
                            if msg_type == "param_change" {
                                if let (Some(index), Some(key), Some(val)) = (
                                    value.get("index").and_then(|v| v.as_u64()),
                                    value.get("param_key").and_then(|v| v.as_str()),
                                    value.get("value").and_then(|v| v.as_f64()),
                                ) {
                                    let current_chain = chain_state.load();
                                    current_chain.set_param(index as usize, key, val as f32);
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
    fn process_stereo_frame_matches_chain_output_on_both_channels() {
        let json = serde_json::json!([
            {
                "type": "Overdrive",
                "params": { "drive": 1.0, "mix": 1.0, "output_gain": 1.0 }
            }
        ])
        .to_string();

        let chain = Chain::from_json(&json).expect("test chain should parse");
        let in_l = 0.35;
        let in_r = -0.52;
        let gain = 0.8;

        let (out_l, out_r) = TonelabPlugin::process_stereo_frame(&chain, in_l, in_r, gain);
        let (expected_l, expected_r) = chain.process(in_l, in_r);

        assert!((out_l - (expected_l * gain)).abs() < 1e-6);
        assert!((out_r - (expected_r * gain)).abs() < 1e-6);
    }

    #[test]
    fn process_mono_frame_matches_left_channel_of_duplicated_input() {
        let json = serde_json::json!([
            {
                "type": "Overdrive",
                "params": { "drive": 0.8, "mix": 0.9, "output_gain": 1.0 }
            }
        ])
        .to_string();

        let chain = Chain::from_json(&json).expect("test chain should parse");
        let input = 0.42;
        let gain = 1.25;

        let out = TonelabPlugin::process_mono_frame(&chain, input, gain);
        let (expected_l, _) = chain.process(input, input);

        assert!((out - (expected_l * gain)).abs() < 1e-6);
    }

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
