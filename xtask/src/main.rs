use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const HELP_TEXT: &str = r#"Tonelab installer/updater helpers

Usage:
  cargo run -p xtask -- install [options]
  cargo run -p xtask -- update [options]
  cargo run -p xtask -- doctor [options]

Options:
  --release              Build release bundle (default)
  --debug                Build debug bundle
  --target <triple>      Build target triple (e.g. x86_64-apple-darwin)
  --dest <path>          Override install VST3 root directory
  --bundle <path>        Use an existing .vst3 bundle path
  --skip-build           Skip build step and install found/provided bundle
  --force                Force reinstall even when already up-to-date
  -h, --help             Show this help

Override env vars:
  TONELAB_VST3_DIR       Highest priority install dir override
  VST3_INSTALL_DIR       Generic install dir override
"#;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum InstallerMode {
    Install,
    Update,
    Doctor,
}

#[derive(Debug, Clone)]
struct InstallerOptions {
    release: bool,
    target: Option<String>,
    dest: Option<PathBuf>,
    bundle: Option<PathBuf>,
    skip_build: bool,
    force: bool,
}

impl Default for InstallerOptions {
    fn default() -> Self {
        Self {
            release: true,
            target: None,
            dest: None,
            bundle: None,
            skip_build: false,
            force: false,
        }
    }
}

#[derive(Debug, Clone)]
struct WorkspaceMetadata {
    package_name: String,
    package_version: String,
    workspace_root: PathBuf,
    target_directory: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallerState {
    package_name: String,
    package_version: String,
    target: Option<String>,
    bundle_name: String,
    source_hash: String,
    install_root: String,
    installed_bundle_path: String,
    updated_at_unix: u64,
}

fn main() -> Result<()> {
    let mut args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        return nih_plug_xtask::main();
    }

    let command = args[1].clone();
    let remaining = args.split_off(2);

    match command.as_str() {
        "install" => run_installer(InstallerMode::Install, &remaining),
        "update" => run_installer(InstallerMode::Update, &remaining),
        "doctor" => run_installer(InstallerMode::Doctor, &remaining),
        "--help" | "-h" | "help" => {
            println!("{HELP_TEXT}");
            Ok(())
        }
        _ => nih_plug_xtask::main(),
    }
}

fn run_installer(mode: InstallerMode, raw_args: &[String]) -> Result<()> {
    let options = parse_options(raw_args)?;
    let metadata = workspace_metadata()?;
    run_installer_with_metadata(mode, options, &metadata)
}

fn run_installer_with_metadata(
    mode: InstallerMode,
    options: InstallerOptions,
    metadata: &WorkspaceMetadata,
) -> Result<()> {
    if mode == InstallerMode::Doctor {
        run_doctor(metadata, &options)?;
        return Ok(());
    }

    if !options.skip_build && options.bundle.is_none() {
        build_bundle(metadata, &options)?;
    }

    let source_bundle = resolve_source_bundle(metadata, &options)?;
    let bundle_name = source_bundle
        .file_name()
        .ok_or_else(|| {
            anyhow!(
                "Could not derive bundle name from {}",
                source_bundle.display()
            )
        })?
        .to_string_lossy()
        .to_string();
    let source_hash = hash_path(&source_bundle)?;

    let install_root = resolve_install_root(options.dest.as_deref())?;
    let destination_bundle = install_root.join(&bundle_name);
    let state_path = state_file_path(&metadata.package_name)?;

    if mode == InstallerMode::Update && !options.force {
        if let Some(state) = read_state(&state_path)? {
            if is_up_to_date(&state, metadata, &source_hash, &destination_bundle) {
                println!("Already up-to-date: {}", destination_bundle.display());
                return Ok(());
            }
        }
    }

    if mode == InstallerMode::Install && !options.force && destination_bundle.exists() {
        println!(
            "Bundle already exists at {}. Reinstalling. Use --force to suppress this warning.",
            destination_bundle.display()
        );
    }

    install_bundle(&source_bundle, &destination_bundle)?;

    #[cfg(target_os = "macos")]
    patch_info_plist_for_ats(&destination_bundle)?;

    run_post_install_hooks(&destination_bundle)?;

    let state = InstallerState {
        package_name: metadata.package_name.clone(),
        package_version: metadata.package_version.clone(),
        target: options.target.clone(),
        bundle_name,
        source_hash,
        install_root: install_root.to_string_lossy().to_string(),
        installed_bundle_path: destination_bundle.to_string_lossy().to_string(),
        updated_at_unix: unix_now()?,
    };

    write_state(&state_path, &state)?;
    println!("Installed: {}", destination_bundle.display());
    println!("State file: {}", state_path.display());
    Ok(())
}

fn parse_options(args: &[String]) -> Result<InstallerOptions> {
    let mut options = InstallerOptions::default();
    let mut idx = 0usize;

    while idx < args.len() {
        let arg = &args[idx];
        match arg.as_str() {
            "--release" => options.release = true,
            "--debug" => options.release = false,
            "--target" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| anyhow!("--target expects a value"))?;
                options.target = Some(value.clone());
            }
            "--dest" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| anyhow!("--dest expects a value"))?;
                options.dest = Some(PathBuf::from(value));
            }
            "--bundle" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| anyhow!("--bundle expects a value"))?;
                options.bundle = Some(PathBuf::from(value));
            }
            "--skip-build" => options.skip_build = true,
            "--force" => options.force = true,
            "-h" | "--help" => {
                println!("{HELP_TEXT}");
                std::process::exit(0);
            }
            unknown => {
                bail!("Unknown option: {unknown}\n\n{HELP_TEXT}");
            }
        }
        idx += 1;
    }

    Ok(options)
}

fn workspace_metadata() -> Result<WorkspaceMetadata> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("Failed to run `cargo metadata`")?;

    if !output.status.success() {
        bail!("`cargo metadata` failed with status {}", output.status);
    }

    let value: Value =
        serde_json::from_slice(&output.stdout).context("Invalid cargo metadata JSON")?;

    let workspace_root = value
        .get("workspace_root")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("Missing workspace_root in cargo metadata"))?;

    let target_directory = value
        .get("target_directory")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("Missing target_directory in cargo metadata"))?;

    let packages = value
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Missing packages in cargo metadata"))?;

    let root_package_id = value
        .get("resolve")
        .and_then(|resolve| resolve.get("root"))
        .and_then(Value::as_str);

    let workspace_manifest = workspace_root.join("Cargo.toml");
    let workspace_manifest_str = workspace_manifest.to_string_lossy().to_string();

    let package = root_package_id
        .and_then(|root_id| {
            packages
                .iter()
                .find(|pkg| pkg.get("id").and_then(Value::as_str) == Some(root_id))
        })
        .or_else(|| {
            packages.iter().find(|pkg| {
                pkg.get("manifest_path")
                    .and_then(Value::as_str)
                    .map(|manifest_path| manifest_path == workspace_manifest_str)
                    .unwrap_or(false)
            })
        })
        .or_else(|| {
            packages
                .iter()
                .find(|pkg| pkg.get("name").and_then(Value::as_str) != Some("xtask"))
        })
        .or_else(|| packages.first())
        .ok_or_else(|| anyhow!("Could not determine root package from cargo metadata"))?;

    let package_name = package
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing root package name in cargo metadata"))?
        .to_string();

    let package_version = package
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Missing root package version in cargo metadata"))?
        .to_string();

    Ok(WorkspaceMetadata {
        package_name,
        package_version,
        workspace_root,
        target_directory,
    })
}

fn build_bundle(metadata: &WorkspaceMetadata, options: &InstallerOptions) -> Result<()> {
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("bundle")
        .arg(&metadata.package_name)
        .current_dir(&metadata.workspace_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if options.release {
        command.arg("--release");
    }

    if let Some(target) = &options.target {
        command.arg("--target").arg(target);
    }

    let status = command
        .status()
        .context("Failed to execute bundle build command")?;
    if !status.success() {
        bail!("Bundle build failed with status {status}");
    }

    Ok(())
}

fn resolve_source_bundle(
    metadata: &WorkspaceMetadata,
    options: &InstallerOptions,
) -> Result<PathBuf> {
    if let Some(bundle_path) = &options.bundle {
        let canonical = fs::canonicalize(bundle_path).with_context(|| {
            format!(
                "Provided bundle path does not exist: {}",
                bundle_path.display()
            )
        })?;
        if canonical.extension() != Some(OsStr::new("vst3")) {
            bail!(
                "Provided bundle path must end with .vst3: {}",
                canonical.display()
            );
        }
        return Ok(canonical);
    }

    let mut roots = Vec::<PathBuf>::new();
    if let Some(target) = &options.target {
        roots.push(metadata.target_directory.join(target).join("bundled"));
    }
    roots.push(metadata.target_directory.join("bundled"));

    let mut candidates = Vec::<PathBuf>::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in
            fs::read_dir(&root).with_context(|| format!("Failed to read {}", root.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension() == Some(OsStr::new("vst3")) {
                candidates.push(path);
            }
        }
    }

    if candidates.is_empty() {
        bail!(
            "No .vst3 bundles found. Expected at least one artifact under {}/bundled",
            metadata.target_directory.display()
        );
    }

    let preferred_name = format!("{}.vst3", metadata.package_name);
    if let Some(path) = candidates.iter().find(|path| {
        path.file_name()
            .map(|f| f == OsStr::new(&preferred_name))
            .unwrap_or(false)
    }) {
        return Ok(path.clone());
    }

    candidates.sort_by(|a, b| newest_mtime(b).cmp(&newest_mtime(a)));
    Ok(candidates.remove(0))
}

fn resolve_install_root(override_dir: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_dir {
        ensure_writable_dir(path)?;
        return Ok(path.to_path_buf());
    }

    let mut candidates = Vec::<PathBuf>::new();
    if let Some(path) = env::var_os("TONELAB_VST3_DIR") {
        candidates.push(PathBuf::from(path));
    }
    if let Some(path) = env::var_os("VST3_INSTALL_DIR") {
        candidates.push(PathBuf::from(path));
    }

    candidates.extend(default_vst3_dirs());
    dedupe_paths(&mut candidates);

    let mut errors = Vec::<String>::new();
    for candidate in candidates {
        match ensure_writable_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) => errors.push(format!("{} ({error})", candidate.display())),
        }
    }

    bail!(
        "Could not find a writable VST3 install directory.\nTried:\n{}",
        errors.join("\n")
    );
}

fn default_vst3_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::<PathBuf>::new();

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            dirs.push(
                home.join("Library")
                    .join("Audio")
                    .join("Plug-Ins")
                    .join("VST3"),
            );
        }
        dirs.push(PathBuf::from("/Library/Audio/Plug-Ins/VST3"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(common) = env::var_os("COMMONPROGRAMFILES") {
            dirs.push(PathBuf::from(common).join("VST3"));
        }
        if let Some(appdata) = env::var_os("APPDATA") {
            dirs.push(PathBuf::from(appdata).join("VST3"));
        }
        if let Some(local_appdata) = env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local_appdata).join("VST3"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(xdg_data_home).join("vst3"));
        }
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".vst3"));
            dirs.push(home.join(".local").join("share").join("vst3"));
        }
        dirs.push(PathBuf::from("/usr/local/lib/vst3"));
    }

    dirs
}

fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    let mut unique = Vec::<PathBuf>::new();
    for path in paths.drain(..) {
        if !unique.iter().any(|existing| existing == &path) {
            unique.push(path);
        }
    }
    *paths = unique;
}

fn ensure_writable_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create install directory {}", dir.display()))?;

    let test_file = dir.join(".tonelab-vst-write-test");
    let mut file = File::create(&test_file)
        .with_context(|| format!("Directory is not writable: {}", dir.display()))?;
    file.write_all(b"ok")
        .with_context(|| format!("Failed writing to {}", test_file.display()))?;
    drop(file);
    let _ = fs::remove_file(test_file);
    Ok(())
}

fn install_bundle(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        if destination.is_dir() {
            fs::remove_dir_all(destination).with_context(|| {
                format!(
                    "Failed to remove existing bundle before install: {}",
                    destination.display()
                )
            })?;
        } else {
            fs::remove_file(destination).with_context(|| {
                format!(
                    "Failed to remove existing file before install: {}",
                    destination.display()
                )
            })?;
        }
    }

    copy_dir_recursive(source, destination).with_context(|| {
        format!(
            "Failed to copy bundle from {} to {}",
            source.display(),
            destination.display()
        )
    })?;

    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    if source.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
        return Ok(());
    }

    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let src = entry.path();
        let dst = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn patch_info_plist_for_ats(bundle_path: &Path) -> Result<()> {
    let plist_path = bundle_path.join("Contents/Info.plist");
    if !plist_path.exists() {
        return Ok(());
    }

    let mut content = fs::read_to_string(&plist_path)?;

    // Check if already patched
    if content.contains("NSAppTransportSecurity") {
        return Ok(());
    }

    let ats_xml = r#"    <key>NSAppTransportSecurity</key>
    <dict>
        <key>NSAllowsArbitraryLoads</key>
        <true/>
    </dict>
  </dict>
</plist>"#;

    // Try to replace the closing tags. The indentation in the file is 2 spaces.
    let target = "  </dict>\n</plist>";
    if content.contains(target) {
        content = content.replace(target, ats_xml);
        fs::write(&plist_path, content)?;
        println!("Patched Info.plist with NSAppTransportSecurity");
    } else {
        println!("Warning: Could not patch Info.plist (structure mismatch)");
    }
    Ok(())
}

fn run_post_install_hooks(bundle_path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        run_optional_command("xattr", ["-cr", &bundle_path.to_string_lossy()])?;
        run_optional_command(
            "codesign",
            [
                "--force",
                "--deep",
                "--sign",
                "-",
                &bundle_path.to_string_lossy(),
            ],
        )?;
    }

    Ok(())
}

fn run_optional_command<const N: usize>(program: &str, args: [&str; N]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(exit) if exit.success() => Ok(()),
        Ok(_) => {
            println!(
                "Warning: optional command failed: {} {}",
                program,
                args.join(" ")
            );
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

fn hash_path(path: &Path) -> Result<String> {
    let mut entries = Vec::<PathBuf>::new();
    collect_paths(path, &mut entries)?;
    entries.sort();

    let mut hasher = Sha256::new();
    for entry in entries {
        let relative = entry.strip_prefix(path).unwrap_or(&entry);
        hasher.update(relative.to_string_lossy().as_bytes());
        if entry.is_dir() {
            hasher.update(b"[dir]");
            continue;
        }

        hasher.update(b"[file]");
        let mut file = File::open(&entry)
            .with_context(|| format!("Failed to read file during hashing: {}", entry.display()))?;
        let mut buffer = [0u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_paths(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    out.push(path.to_path_buf());
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            collect_paths(&entry.path(), out)?;
        }
    }
    Ok(())
}

fn newest_mtime(path: &Path) -> u128 {
    let mut newest = 0u128;
    let mut queue = vec![path.to_path_buf()];

    while let Some(current) = queue.pop() {
        if let Ok(metadata) = fs::metadata(&current) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                    newest = newest.max(duration.as_nanos());
                }
            }
            if metadata.is_dir() {
                if let Ok(entries) = fs::read_dir(&current) {
                    for entry in entries.flatten() {
                        queue.push(entry.path());
                    }
                }
            }
        }
    }

    newest
}

fn state_file_path(package_name: &str) -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow!("Could not determine config directory for installer state"))?;
    let state_dir = config_dir.join("tonelab-vst");
    fs::create_dir_all(&state_dir)
        .with_context(|| format!("Failed to create state directory {}", state_dir.display()))?;
    Ok(state_dir.join(format!("{package_name}-installer-state.json")))
}

fn read_state(path: &Path) -> Result<Option<InstallerState>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read state file {}", path.display()))?;
    let state: InstallerState = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse state file {}", path.display()))?;
    Ok(Some(state))
}

fn write_state(path: &Path, state: &InstallerState) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("State file has no parent directory: {}", path.display()))?;
    fs::create_dir_all(parent)?;
    let payload = serde_json::to_string_pretty(state)?;
    fs::write(path, payload).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn is_up_to_date(
    state: &InstallerState,
    metadata: &WorkspaceMetadata,
    source_hash: &str,
    destination_bundle: &Path,
) -> bool {
    state.package_name == metadata.package_name
        && state.package_version == metadata.package_version
        && state.source_hash == source_hash
        && Path::new(&state.installed_bundle_path).exists()
        && destination_bundle.exists()
}

fn unix_now() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time is before UNIX_EPOCH")?;
    Ok(now.as_secs())
}

fn run_doctor(metadata: &WorkspaceMetadata, options: &InstallerOptions) -> Result<()> {
    println!("Tonelab installer doctor");
    println!("OS: {}  ARCH: {}", env::consts::OS, env::consts::ARCH);
    println!("Workspace root: {}", metadata.workspace_root.display());
    println!(
        "Package: {} {}",
        metadata.package_name, metadata.package_version
    );
    println!("Target dir: {}", metadata.target_directory.display());
    println!(
        "Mode: {}",
        if options.release { "release" } else { "debug" }
    );
    if let Some(target) = &options.target {
        println!("Requested target: {target}");
    }

    println!("\nInstall directory candidates:");
    if let Some(path) = env::var_os("TONELAB_VST3_DIR") {
        println!("- TONELAB_VST3_DIR={}", PathBuf::from(path).display());
    }
    if let Some(path) = env::var_os("VST3_INSTALL_DIR") {
        println!("- VST3_INSTALL_DIR={}", PathBuf::from(path).display());
    }
    for path in default_vst3_dirs() {
        println!("- {}", path.display());
    }

    match resolve_install_root(options.dest.as_deref()) {
        Ok(path) => println!("\nResolved install root: {}", path.display()),
        Err(error) => println!("\nResolved install root: ERROR ({error})"),
    }

    match resolve_source_bundle(metadata, options) {
        Ok(path) => println!("Detected bundle: {}", path.display()),
        Err(error) => println!("Detected bundle: ERROR ({error})"),
    }

    let state_path = state_file_path(&metadata.package_name)?;
    println!("State file: {}", state_path.display());
    match read_state(&state_path)? {
        Some(state) => println!(
            "Installed state: version={}, path={}",
            state.package_version, state.installed_bundle_path
        ),
        None => println!("Installed state: <none>"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn unique_suffix() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drift")
            .as_nanos();
        format!("{}_{}", std::process::id(), nanos)
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("{prefix}_{}", unique_suffix()));
        fs::create_dir_all(&path).expect("failed to create temp directory");
        path
    }

    fn create_fake_bundle(root: &Path, bundle_name: &str, payload: &str) -> PathBuf {
        let bundle = root.join(format!("{bundle_name}.vst3"));
        let binary_dir = bundle.join("Contents").join("MacOS");
        fs::create_dir_all(&binary_dir).expect("failed to create fake bundle structure");
        fs::write(binary_dir.join(bundle_name), payload)
            .expect("failed to write fake plugin binary");
        bundle
    }

    fn remove_path(path: &Path) {
        if !path.exists() {
            return;
        }
        if path.is_dir() {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn install_and_update_flow_is_idempotent() {
        let temp_root = unique_temp_dir("tonelab_xtask_flow");
        let bundle_root = temp_root.join("bundle_source");
        let install_root = temp_root.join("install_dest");
        fs::create_dir_all(&bundle_root).expect("failed to create bundle root");
        fs::create_dir_all(&install_root).expect("failed to create install root");

        let bundle = create_fake_bundle(&bundle_root, "tonelab_vst_test", "version-1");
        let package_name = format!("tonelab_vst_test_{}", unique_suffix());
        let metadata = WorkspaceMetadata {
            package_name: package_name.clone(),
            package_version: "1.2.3".to_string(),
            workspace_root: temp_root.clone(),
            target_directory: temp_root.join("target"),
        };

        let options = InstallerOptions {
            release: true,
            target: None,
            dest: Some(install_root.clone()),
            bundle: Some(bundle.clone()),
            skip_build: true,
            force: false,
        };

        run_installer_with_metadata(InstallerMode::Install, options.clone(), &metadata)
            .expect("install flow failed");
        let installed_bundle = install_root.join("tonelab_vst_test.vst3");
        assert!(installed_bundle.exists(), "installed bundle does not exist");

        let state_path = state_file_path(&package_name).expect("state path should resolve");
        let state_before = read_state(&state_path)
            .expect("state should be readable")
            .expect("state should exist after install");

        run_installer_with_metadata(InstallerMode::Update, options, &metadata)
            .expect("update flow failed");
        let state_after = read_state(&state_path)
            .expect("state should be readable")
            .expect("state should still exist after update");

        assert_eq!(state_before.source_hash, state_after.source_hash);
        assert_eq!(state_before.package_version, state_after.package_version);

        remove_path(&state_path);
        remove_path(&temp_root);
    }

    #[test]
    fn update_reinstalls_when_source_bundle_changes() {
        let temp_root = unique_temp_dir("tonelab_xtask_update");
        let bundle_root = temp_root.join("bundle_source");
        let install_root = temp_root.join("install_dest");
        fs::create_dir_all(&bundle_root).expect("failed to create bundle root");
        fs::create_dir_all(&install_root).expect("failed to create install root");

        let bundle = create_fake_bundle(&bundle_root, "tonelab_vst_test", "version-1");
        let source_binary = bundle
            .join("Contents")
            .join("MacOS")
            .join("tonelab_vst_test");
        let package_name = format!("tonelab_vst_test_{}", unique_suffix());
        let metadata = WorkspaceMetadata {
            package_name: package_name.clone(),
            package_version: "2.0.0".to_string(),
            workspace_root: temp_root.clone(),
            target_directory: temp_root.join("target"),
        };

        let options = InstallerOptions {
            release: true,
            target: None,
            dest: Some(install_root.clone()),
            bundle: Some(bundle.clone()),
            skip_build: true,
            force: false,
        };

        run_installer_with_metadata(InstallerMode::Install, options.clone(), &metadata)
            .expect("initial install should succeed");

        let state_path = state_file_path(&package_name).expect("state path should resolve");
        let before = read_state(&state_path)
            .expect("state should be readable")
            .expect("state should exist");

        fs::write(&source_binary, "version-2").expect("failed to modify source bundle");
        run_installer_with_metadata(InstallerMode::Update, options, &metadata)
            .expect("update should reinstall modified bundle");

        let after = read_state(&state_path)
            .expect("state should be readable")
            .expect("state should exist");
        assert_ne!(before.source_hash, after.source_hash);

        let installed_binary = install_root
            .join("tonelab_vst_test.vst3")
            .join("Contents")
            .join("MacOS")
            .join("tonelab_vst_test");
        let installed_payload =
            fs::read_to_string(installed_binary).expect("installed payload should be readable");
        assert_eq!(installed_payload, "version-2");

        remove_path(&state_path);
        remove_path(&temp_root);
    }

    #[test]
    fn resolve_source_bundle_prefers_package_named_bundle() {
        let temp_root = unique_temp_dir("tonelab_xtask_bundle_detect");
        let bundled_dir = temp_root.join("target").join("bundled");
        fs::create_dir_all(&bundled_dir).expect("failed to create bundled dir");

        let _other = create_fake_bundle(&bundled_dir, "other_plugin", "other");
        let expected = create_fake_bundle(&bundled_dir, "tonelab_vst_pkg", "preferred");

        let metadata = WorkspaceMetadata {
            package_name: "tonelab_vst_pkg".to_string(),
            package_version: "0.9.0".to_string(),
            workspace_root: temp_root.clone(),
            target_directory: temp_root.join("target"),
        };

        let options = InstallerOptions::default();
        let detected =
            resolve_source_bundle(&metadata, &options).expect("bundle detection should succeed");
        assert_eq!(detected.file_name(), expected.file_name());

        remove_path(&temp_root);
    }

    #[test]
    fn parse_options_accepts_expected_flags() {
        let args = vec![
            "--debug".to_string(),
            "--target".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
            "--dest".to_string(),
            "/tmp/tonelab-vst-test".to_string(),
            "--skip-build".to_string(),
            "--force".to_string(),
        ];

        let parsed = parse_options(&args).expect("options should parse");
        assert!(!parsed.release);
        assert_eq!(parsed.target.as_deref(), Some("x86_64-unknown-linux-gnu"));
        assert_eq!(
            parsed.dest.as_deref(),
            Some(Path::new("/tmp/tonelab-vst-test"))
        );
        assert!(parsed.skip_build);
        assert!(parsed.force);
    }

    #[test]
    fn parse_options_rejects_unknown_flag() {
        let args = vec!["--unknown-flag".to_string()];
        let err = parse_options(&args).expect_err("unknown flag should fail");
        assert!(err.to_string().contains("Unknown option"));
    }

    #[test]
    fn parse_options_requires_value_for_target() {
        let args = vec!["--target".to_string()];
        let err = parse_options(&args).expect_err("missing target value should fail");
        assert!(err.to_string().contains("--target expects a value"));
    }

    #[test]
    fn parse_options_requires_value_for_dest() {
        let args = vec!["--dest".to_string()];
        let err = parse_options(&args).expect_err("missing dest value should fail");
        assert!(err.to_string().contains("--dest expects a value"));
    }

    #[test]
    fn parse_options_requires_value_for_bundle() {
        let args = vec!["--bundle".to_string()];
        let err = parse_options(&args).expect_err("missing bundle value should fail");
        assert!(err.to_string().contains("--bundle expects a value"));
    }

    #[test]
    fn resolve_source_bundle_errors_when_no_candidates_exist() {
        let temp_root = unique_temp_dir("tonelab_xtask_empty_candidates");
        let metadata = WorkspaceMetadata {
            package_name: "tonelab_vst".to_string(),
            package_version: "0.1.0".to_string(),
            workspace_root: temp_root.clone(),
            target_directory: temp_root.join("target"),
        };
        let options = InstallerOptions::default();

        let err = resolve_source_bundle(&metadata, &options)
            .expect_err("missing bundled artifacts should return error");
        assert!(err.to_string().contains("No .vst3 bundles found"));

        remove_path(&temp_root);
    }

    #[test]
    fn resolve_source_bundle_rejects_non_vst3_path() {
        let temp_root = unique_temp_dir("tonelab_xtask_bad_bundle");
        let bad_bundle = temp_root.join("plugin.zip");
        fs::write(&bad_bundle, "not a vst3").expect("failed to create fake non-vst3 file");

        let metadata = WorkspaceMetadata {
            package_name: "tonelab_vst".to_string(),
            package_version: "0.1.0".to_string(),
            workspace_root: temp_root.clone(),
            target_directory: temp_root.join("target"),
        };
        let options = InstallerOptions {
            bundle: Some(bad_bundle.clone()),
            ..InstallerOptions::default()
        };

        let err =
            resolve_source_bundle(&metadata, &options).expect_err("non-vst3 path should fail");
        assert!(err.to_string().contains("must end with .vst3"));

        remove_path(&temp_root);
    }

    #[test]
    fn resolve_source_bundle_uses_newest_candidate_when_name_not_found() {
        let temp_root = unique_temp_dir("tonelab_xtask_newest_bundle");
        let bundled_dir = temp_root.join("target").join("bundled");
        fs::create_dir_all(&bundled_dir).expect("failed to create bundled dir");

        let older = create_fake_bundle(&bundled_dir, "aaa_old", "old");
        thread::sleep(Duration::from_secs(1));
        let newer = create_fake_bundle(&bundled_dir, "bbb_new", "new");

        let metadata = WorkspaceMetadata {
            package_name: "not_existing_name".to_string(),
            package_version: "0.1.0".to_string(),
            workspace_root: temp_root.clone(),
            target_directory: temp_root.join("target"),
        };
        let options = InstallerOptions::default();

        let resolved =
            resolve_source_bundle(&metadata, &options).expect("should resolve newest bundle");
        assert_eq!(resolved.file_name(), newer.file_name());
        assert_ne!(resolved.file_name(), older.file_name());

        remove_path(&temp_root);
    }

    #[test]
    fn install_bundle_replaces_existing_destination_bundle() {
        let temp_root = unique_temp_dir("tonelab_xtask_replace_install");
        let source_root = temp_root.join("source");
        let dest_root = temp_root.join("dest");
        fs::create_dir_all(&source_root).expect("failed to create source root");
        fs::create_dir_all(&dest_root).expect("failed to create destination root");

        let source_bundle = create_fake_bundle(&source_root, "plugin_a", "new-version");
        let destination_bundle = create_fake_bundle(&dest_root, "plugin_a", "old-version");

        install_bundle(&source_bundle, &destination_bundle)
            .expect("install should replace destination bundle");

        let payload = fs::read_to_string(
            destination_bundle
                .join("Contents")
                .join("MacOS")
                .join("plugin_a"),
        )
        .expect("destination payload should be readable");
        assert_eq!(payload, "new-version");

        remove_path(&temp_root);
    }

    #[test]
    fn hash_path_changes_when_contents_change() {
        let temp_root = unique_temp_dir("tonelab_xtask_hash_change");
        let bundle = create_fake_bundle(&temp_root, "hash_test", "payload-v1");
        let binary = bundle.join("Contents").join("MacOS").join("hash_test");

        let hash_v1 = hash_path(&bundle).expect("hash must compute");
        fs::write(&binary, "payload-v2").expect("failed to update bundle payload");
        let hash_v2 = hash_path(&bundle).expect("hash must recompute");

        assert_ne!(
            hash_v1, hash_v2,
            "bundle hash must change after file mutation"
        );
        remove_path(&temp_root);
    }

    #[test]
    fn write_and_read_state_roundtrip() {
        let temp_root = unique_temp_dir("tonelab_xtask_state_roundtrip");
        let state_file = temp_root.join("installer_state.json");
        let state = InstallerState {
            package_name: "pkg".to_string(),
            package_version: "1.0.0".to_string(),
            target: Some("x86_64-unknown-linux-gnu".to_string()),
            bundle_name: "pkg.vst3".to_string(),
            source_hash: "abc123".to_string(),
            install_root: "/tmp/vst3".to_string(),
            installed_bundle_path: "/tmp/vst3/pkg.vst3".to_string(),
            updated_at_unix: 123,
        };

        write_state(&state_file, &state).expect("state file should be written");
        let read_back = read_state(&state_file)
            .expect("state file should be readable")
            .expect("state should exist");

        assert_eq!(read_back.package_name, state.package_name);
        assert_eq!(read_back.package_version, state.package_version);
        assert_eq!(read_back.source_hash, state.source_hash);
        assert_eq!(read_back.installed_bundle_path, state.installed_bundle_path);

        remove_path(&temp_root);
    }

    #[test]
    fn is_up_to_date_detects_metadata_or_hash_mismatch() {
        let destination = PathBuf::from("/tmp/tonelab-vst-nonexistent-bundle.vst3");
        let metadata = WorkspaceMetadata {
            package_name: "pkg".to_string(),
            package_version: "1.0.0".to_string(),
            workspace_root: PathBuf::from("/tmp"),
            target_directory: PathBuf::from("/tmp/target"),
        };
        let state = InstallerState {
            package_name: "pkg".to_string(),
            package_version: "1.0.0".to_string(),
            target: None,
            bundle_name: "pkg.vst3".to_string(),
            source_hash: "hash-a".to_string(),
            install_root: "/tmp".to_string(),
            installed_bundle_path: destination.to_string_lossy().to_string(),
            updated_at_unix: 0,
        };

        assert!(!is_up_to_date(&state, &metadata, "hash-a", &destination));

        let bad_version = WorkspaceMetadata {
            package_version: "2.0.0".to_string(),
            ..metadata.clone()
        };
        assert!(!is_up_to_date(&state, &bad_version, "hash-a", &destination));

        assert!(!is_up_to_date(&state, &metadata, "hash-b", &destination));
    }
}
