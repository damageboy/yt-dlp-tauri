use crate::toolchain::{atomic_replace, locate_homebrew, probe_executable};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub const ARIA2C_CONFIG_FILE: &str = "aria2c.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Aria2cConfig {
    pub schema_version: u32,
    pub enabled: bool,
    pub executable_path: Option<PathBuf>,
    pub parallel_connections: u8,
}

impl Default for Aria2cConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            enabled: false,
            executable_path: None,
            parallel_connections: 16,
        }
    }
}

impl Aria2cConfig {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported aria2c settings schema.".into());
        }
        if !(1..=16).contains(&self.parallel_connections) {
            return Err("aria2c parallelism must be an integer from 1 to 16.".into());
        }
        if self
            .executable_path
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
        {
            return Err("Choose an absolute aria2c executable path.".into());
        }
        Ok(())
    }
}

pub fn parse_aria2c_config(json: &str) -> Result<Aria2cConfig, String> {
    let config: Aria2cConfig =
        serde_json::from_str(json).map_err(|error| format!("Invalid aria2c settings: {error}"))?;
    config.validate()?;
    Ok(config)
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Aria2cSource {
    Configured,
    Path,
    HomebrewPrefix,
    HomebrewAppleSilicon,
    HomebrewIntel,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Aria2cStatus {
    pub source: Option<Aria2cSource>,
    pub executable_path: Option<PathBuf>,
    pub available: bool,
    pub version: Option<String>,
    pub error_code: Option<String>,
    pub error: Option<String>,
}

impl Aria2cStatus {
    pub fn tool_status(&self) -> crate::toolchain::ToolStatus {
        crate::toolchain::ToolStatus {
            name: "aria2c".into(),
            relative_path: "aria2c".into(),
            full_path: self
                .executable_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            availability: if self.available {
                "available"
            } else if matches!(
                self.error_code.as_deref(),
                Some("not-found" | "invalid-path")
            ) {
                "missing"
            } else {
                "cannot_execute"
            }
            .into(),
            version: self.version.clone(),
            expected_version: None,
            error: self.error.clone(),
        }
    }

    fn available(source: Aria2cSource, path: PathBuf, version: String) -> Self {
        Self {
            source: Some(source),
            executable_path: Some(path),
            available: true,
            version: Some(version),
            ..Default::default()
        }
    }
}

pub fn require_aria2c(config: &Aria2cConfig) -> Result<Aria2cStatus, String> {
    let status = inspect_aria2c(config)?;
    if !status.available {
        return Err(status
            .error
            .unwrap_or_else(|| "aria2c is required. Install or configure it in Settings.".into()));
    }
    Ok(status)
}

pub fn aria2c_downloader_args(
    config: &Aria2cConfig,
    status: &Aria2cStatus,
) -> Result<Vec<OsString>, String> {
    config.validate()?;
    if !config.enabled {
        return Ok(Vec::new());
    }
    if !status.available {
        return Err(status.error.clone().unwrap_or_else(|| {
            "aria2c is unavailable. Install or configure it in Settings.".into()
        }));
    }
    let path = status
        .executable_path
        .as_ref()
        .ok_or("aria2c has no resolved executable.")?;
    let n = config.parallel_connections;
    Ok(vec![
        "--downloader".into(),
        path.as_os_str().to_owned(),
        "--downloader-args".into(),
        format!("aria2c:-j {n} -x {n} -s {n}").into(),
    ])
}

fn failure(
    code: &str,
    message: impl Into<String>,
    source: Option<Aria2cSource>,
    path: Option<PathBuf>,
) -> Aria2cStatus {
    Aria2cStatus {
        source,
        executable_path: path,
        error_code: Some(code.into()),
        error: Some(message.into()),
        ..Default::default()
    }
}

fn executable_name(os: &str) -> &'static str {
    if os == "windows" {
        "aria2c.exe"
    } else {
        "aria2c"
    }
}

fn resolve_with(
    config: &Aria2cConfig,
    os: &str,
    directories: &[PathBuf],
    cwd: &Path,
    homebrew_prefix: impl FnOnce() -> Option<PathBuf>,
    is_file: impl Fn(&Path) -> bool,
) -> Result<(Aria2cSource, PathBuf), Aria2cStatus> {
    if let Some(path) = &config.executable_path {
        return if is_file(path) {
            Ok((Aria2cSource::Configured, path.clone()))
        } else {
            Err(failure(
                "invalid-path",
                "Configured aria2c is missing. Install aria2 or choose another executable.",
                Some(Aria2cSource::Configured),
                Some(path.clone()),
            ))
        };
    }
    let name = executable_name(os);
    let mut seen = BTreeSet::new();
    for directory in directories {
        let path = if directory.is_absolute() {
            directory.clone()
        } else {
            cwd.join(directory)
        }
        .join(name);
        if seen.insert(path.clone()) && is_file(&path) {
            return Ok((Aria2cSource::Path, path));
        }
    }
    if os == "macos" {
        let mut candidates = Vec::new();
        if let Some(prefix) = homebrew_prefix().filter(|path| path.is_absolute()) {
            candidates.push((Aria2cSource::HomebrewPrefix, prefix.join("bin/aria2c")));
        }
        candidates.extend([
            (
                Aria2cSource::HomebrewAppleSilicon,
                PathBuf::from("/opt/homebrew/bin/aria2c"),
            ),
            (
                Aria2cSource::HomebrewIntel,
                PathBuf::from("/usr/local/bin/aria2c"),
            ),
        ]);
        for (source, path) in candidates {
            if seen.insert(path.clone()) && is_file(&path) {
                return Ok((source, path));
            }
        }
    }
    Err(failure(
        "not-found",
        "aria2c was not found. Install aria2 or choose its executable in Settings.",
        None,
        None,
    ))
}

fn executable_candidate(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn inspect_aria2c(config: &Aria2cConfig) -> Result<Aria2cStatus, String> {
    config.validate()?;
    let directories = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let resolution = resolve_with(
        config,
        env::consts::OS,
        &directories,
        &cwd,
        || {
            crate::current_platform_definition()
                .ok()
                .and_then(|platform| locate_homebrew(&platform).ok())
                .map(|installation| installation.prefix)
        },
        executable_candidate,
    );
    let (source, path) = match resolution {
        Ok(value) => value,
        Err(status) => return Ok(status),
    };
    if path.file_name().and_then(|name| name.to_str()) != Some(executable_name(env::consts::OS)) {
        return Ok(failure(
            "invalid-path",
            "Select an executable named aria2c (aria2c.exe on Windows).",
            Some(source),
            Some(path),
        ));
    }
    let probe = probe_executable("aria2c", &path);
    if probe.availability != "available" {
        let error = probe
            .error
            .unwrap_or_else(|| "aria2c could not be verified.".into());
        let code = if error.contains("timed out") {
            "timeout"
        } else {
            "probe-failed"
        };
        return Ok(failure(
            code,
            bounded_summary(&error),
            Some(source),
            Some(path),
        ));
    }
    match probe
        .version
        .filter(|version| version.starts_with("aria2 version "))
    {
        Some(version) => Ok(Aria2cStatus::available(
            source,
            path,
            bounded_summary(&version),
        )),
        None => Ok(failure(
            "probe-failed",
            "The selected program did not report an aria2 version.",
            Some(source),
            Some(path),
        )),
    }
}

fn bounded_summary(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .take(240)
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Aria2cSettings {
    pub config: Aria2cConfig,
    pub status: Aria2cStatus,
    pub load_error: Option<String>,
}

#[derive(Clone)]
pub struct Aria2cState {
    path: Option<PathBuf>,
    stored: Arc<Mutex<Aria2cSettings>>,
}

impl Aria2cState {
    pub fn load(path: PathBuf) -> Self {
        let loaded = match fs::read_to_string(&path) {
            Ok(json) => parse_aria2c_config(&json),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Aria2cConfig::default())
            }
            Err(error) => Err(format!("Could not read aria2c settings: {error}")),
        };
        let (config, load_error) = match loaded {
            Ok(config) => (config, None),
            Err(error) => (Aria2cConfig::default(), Some(error)),
        };
        Self {
            path: Some(path),
            stored: Arc::new(Mutex::new(Aria2cSettings {
                config,
                status: Aria2cStatus::default(),
                load_error,
            })),
        }
    }

    pub fn unavailable(error: String) -> Self {
        Self {
            path: None,
            stored: Arc::new(Mutex::new(Aria2cSettings {
                config: Aria2cConfig::default(),
                status: Aria2cStatus::default(),
                load_error: Some(error),
            })),
        }
    }

    pub fn settings(&self) -> Result<Aria2cSettings, String> {
        self.stored
            .lock()
            .map(|settings| settings.clone())
            .map_err(|error| error.to_string())
    }

    pub fn snapshot_config(&self) -> Result<Aria2cConfig, String> {
        Ok(self.settings()?.config)
    }

    pub(crate) fn save_with(
        &self,
        config: Aria2cConfig,
        inspect: impl FnOnce(&Aria2cConfig) -> Result<Aria2cStatus, String>,
    ) -> Result<Aria2cSettings, String> {
        config.validate()?;
        let status = if config.enabled {
            inspect(&config)?
        } else {
            Aria2cStatus::default()
        };
        aria2c_downloader_args(&config, &status)?;
        let path = self
            .path
            .as_ref()
            .ok_or("aria2c settings directory is unavailable.")?;
        let parent = path
            .parent()
            .ok_or("aria2c settings has no parent directory.")?;
        let mut bytes = serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        // Serialize the file commit and memory update together; probes run outside this lock.
        let mut stored = self.stored.lock().map_err(|error| error.to_string())?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let temporary = parent.join(format!(".aria2c-{}-{nonce}.tmp", std::process::id()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| error.to_string())?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|error| error.to_string())?;
            drop(file);
            atomic_replace(&temporary, path)
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        *stored = Aria2cSettings {
            config,
            status,
            load_error: None,
        };
        Ok(stored.clone())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn automatic_discovery_skips_non_executable_files() {
        use std::os::unix::fs::PermissionsExt;
        let root = TestDirectory::new();
        let shadow = root.0.join("shadow");
        fs::create_dir(&shadow).unwrap();
        fs::write(shadow.join("aria2c"), "not executable").unwrap();
        fs::set_permissions(shadow.join("aria2c"), fs::Permissions::from_mode(0o644)).unwrap();
        let good = root.fixture("");
        let result = resolve_with(
            &Aria2cConfig::default(),
            "macos",
            &[shadow, good.parent().unwrap().to_path_buf()],
            &root.0,
            || panic!("Valid PATH executable should win"),
            executable_candidate,
        )
        .unwrap();
        assert_eq!(result.1, good);
    }

    use super::*;
    use crate::test_support::TestDirectory;

    #[test]
    fn resolution_obeys_precedence_and_skips_unneeded_homebrew() {
        let root = TestDirectory::new();
        let dir = root.0.join("bin");
        let config = Aria2cConfig::default();
        let resolution = resolve_with(
            &config,
            "macos",
            std::slice::from_ref(&dir),
            &root.0,
            || panic!("PATH match must skip Homebrew"),
            |_| true,
        )
        .unwrap();
        assert_eq!(resolution, (Aria2cSource::Path, dir.join("aria2c")));
        let prefix = root.0.join("custom-brew");
        let resolution = resolve_with(
            &config,
            "macos",
            &[],
            &root.0,
            || Some(prefix.clone()),
            |_| true,
        )
        .unwrap();
        assert_eq!(
            resolution,
            (Aria2cSource::HomebrewPrefix, prefix.join("bin/aria2c"))
        );
        let explicit = Aria2cConfig {
            executable_path: Some(root.0.join("missing/aria2c")),
            ..config
        };
        let missing = resolve_with(
            &explicit,
            "macos",
            &[dir],
            &root.0,
            || panic!("Explicit path must skip Homebrew"),
            |_| false,
        )
        .unwrap_err();
        assert_eq!(missing.error_code.as_deref(), Some("invalid-path"));
        assert_eq!(missing.executable_path, explicit.executable_path);
        let resolution = resolve_with(
            &Aria2cConfig::default(),
            "windows",
            &[PathBuf::from("relative")],
            &root.0,
            || panic!("Not macOS"),
            |_| true,
        )
        .unwrap();
        assert_eq!(resolution.1, root.0.join("relative/aria2c.exe"));
    }

    #[test]
    fn real_probe_accepts_version_and_rejects_empty_or_wrong_name() {
        let root = TestDirectory::new();
        let exe = root.fixture("stderr");
        let config = Aria2cConfig {
            executable_path: Some(exe.clone()),
            enabled: true,
            ..Default::default()
        };
        let status = inspect_aria2c(&config).unwrap();
        assert!(status.available, "{status:?}");
        assert_eq!(status.version.as_deref(), Some("aria2 version 1.37.0"));
        fs::write(exe.parent().unwrap().join("mode"), "empty").unwrap();
        assert_eq!(
            inspect_aria2c(&config).unwrap().error_code.as_deref(),
            Some("probe-failed")
        );
        let wrong = exe.with_file_name("different-program");
        fs::copy(&exe, &wrong).unwrap();
        let config = Aria2cConfig {
            executable_path: Some(wrong),
            ..config
        };
        assert_eq!(
            inspect_aria2c(&config).unwrap().error_code.as_deref(),
            Some("invalid-path")
        );
    }

    #[test]
    fn save_reload_recovery_and_validation_preserve_state() {
        let root = TestDirectory::new();
        let path = root.0.join("state").join(ARIA2C_CONFIG_FILE);
        let state = Aria2cState::load(path.clone());
        assert_eq!(state.settings().unwrap().config, Aria2cConfig::default());
        let disabled = Aria2cConfig {
            executable_path: Some(root.0.join("gone/aria2c")),
            ..Default::default()
        };
        state
            .save_with(disabled.clone(), |_| panic!("Disabled save must not probe"))
            .unwrap();
        let bytes = fs::read(&path).unwrap();
        let enabled = Aria2cConfig {
            enabled: true,
            ..disabled.clone()
        };
        assert!(state
            .save_with(enabled, |_| Ok(Aria2cStatus::default()))
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(state.snapshot_config().unwrap(), disabled);
        assert_eq!(
            Aria2cState::load(path.clone()).snapshot_config().unwrap(),
            disabled
        );
        fs::write(&path, "{malformed").unwrap();
        let recovery = Aria2cState::load(path);
        assert!(recovery.settings().unwrap().load_error.is_some());
        assert!(!recovery.snapshot_config().unwrap().enabled);
        recovery
            .save_with(Aria2cConfig::default(), |_| panic!("Disabled recovery"))
            .unwrap();
        assert!(recovery.settings().unwrap().load_error.is_none());
    }

    #[test]
    fn disabled_usage_still_requires_an_installed_executable() {
        let root = TestDirectory::new();
        let missing = Aria2cConfig {
            executable_path: Some(root.0.join("missing/aria2c")),
            enabled: false,
            ..Default::default()
        };
        assert!(require_aria2c(&missing).is_err());
        let available = Aria2cConfig {
            executable_path: Some(root.fixture("")),
            ..missing
        };
        let status = require_aria2c(&available).unwrap();
        assert!(aria2c_downloader_args(&available, &status)
            .unwrap()
            .is_empty());
        assert_eq!(status.tool_status().availability, "available");
    }

    #[test]
    fn concurrent_saves_leave_memory_equal_to_disk() {
        let root = TestDirectory::new();
        let path = root.0.join(ARIA2C_CONFIG_FILE);
        let state = Aria2cState::load(path.clone());
        let barrier = Arc::new(std::sync::Barrier::new(16));
        let threads: Vec<_> = (1..=16)
            .map(|parallel_connections| {
                let state = state.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    state
                        .save_with(
                            Aria2cConfig {
                                parallel_connections,
                                ..Default::default()
                            },
                            |_| panic!("Disabled save must not probe"),
                        )
                        .unwrap();
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(
            state.snapshot_config().unwrap(),
            parse_aria2c_config(&fs::read_to_string(path).unwrap()).unwrap()
        );
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
    }

    #[test]
    fn failed_replace_does_not_change_memory_and_cleans_temporary_file() {
        let root = TestDirectory::new();
        let path = root.0.join("blocked.json");
        fs::create_dir(&path).unwrap();
        let state = Aria2cState::load(path);
        let draft = Aria2cConfig {
            parallel_connections: 4,
            ..Default::default()
        };
        assert!(state.save_with(draft, |_| panic!("Disabled save")).is_err());
        assert_eq!(state.snapshot_config().unwrap().parallel_connections, 16);
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
    }

    fn absolute(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    #[test]
    fn defaults_and_schema_validation() {
        let defaults = Aria2cConfig::default();
        assert!(!defaults.enabled);
        assert_eq!(defaults.parallel_connections, 16);
        assert_eq!(defaults.executable_path, None);
        let json = serde_json::to_value(&defaults).unwrap();
        assert_eq!(parse_aria2c_config(&json.to_string()).unwrap(), defaults);
        for (key, value) in [
            ("schemaVersion", serde_json::json!(2)),
            ("parallelConnections", serde_json::json!(0)),
            ("parallelConnections", serde_json::json!(17)),
            ("parallelConnections", serde_json::json!(-1)),
            ("parallelConnections", serde_json::json!(1.5)),
            ("executablePath", serde_json::json!("relative/aria2c")),
            ("rawArgs", serde_json::json!("--anything")),
        ] {
            let mut invalid = json.clone();
            invalid[key] = value;
            assert!(
                parse_aria2c_config(&invalid.to_string()).is_err(),
                "{invalid}"
            );
        }
        let mut missing = json.clone();
        missing.as_object_mut().unwrap().remove("enabled");
        assert!(parse_aria2c_config(&missing.to_string()).is_err());
    }

    #[test]
    fn exact_arguments_preserve_executable_spaces_and_parallelism() {
        let path = absolute("tools with spaces/aria2c");
        let status = Aria2cStatus::available(
            Aria2cSource::Configured,
            path.clone(),
            "aria2 version 1".into(),
        );
        for n in [1, 16] {
            let config = Aria2cConfig {
                enabled: true,
                parallel_connections: n,
                ..Default::default()
            };
            assert_eq!(
                aria2c_downloader_args(&config, &status).unwrap(),
                vec![
                    OsString::from("--downloader"),
                    path.as_os_str().to_owned(),
                    OsString::from("--downloader-args"),
                    OsString::from(if n == 1 {
                        "aria2c:-j 1 -x 1 -s 1"
                    } else {
                        "aria2c:-j 16 -x 16 -s 16"
                    }),
                ]
            );
            assert!(aria2c_downloader_args(&config, &Aria2cStatus::default()).is_err());
        }
        assert!(
            aria2c_downloader_args(&Aria2cConfig::default(), &Aria2cStatus::default())
                .unwrap()
                .is_empty()
        );
    }
}
