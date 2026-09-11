use super::{
    probe_local_toolchain, ExecutableNames, HomebrewPackageDefinition, LocalToolchainResolution,
    ManagedProviderDefinition, PlatformToolchainDefinition, ProgressReporter, ToolInstallProgress,
    ToolPaths, ToolStatus,
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

const HOMEBREW_INSTALL_URL: &str = "https://brew.sh/";

#[derive(Debug, Clone)]
pub struct HomebrewInstallation {
    pub brew: PathBuf,
    pub prefix: PathBuf,
    pub paths: ToolPaths,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessOutput {
    success: bool,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

trait ProcessRunner {
    fn run(&self, program: &Path, args: &[OsString]) -> Result<ProcessOutput, String>;
}

struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, program: &Path, args: &[OsString]) -> Result<ProcessOutput, String> {
        let mut command = Command::new(program);
        command.args(args);
        let output = if args == [OsString::from("--prefix")] {
            super::probe::run_bounded_probe_with_timeout(
                &mut command,
                "Homebrew prefix query",
                std::time::Duration::from_secs(10),
            )?
        } else {
            command.output().map_err(|error| {
                format!(
                    "Failed to run Homebrew executable {}: {error}",
                    program.display()
                )
            })?
        };
        Ok(ProcessOutput {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HomebrewAction {
    Install,
    Upgrade,
    Reinstall,
    Outdated,
    Prefix,
}

impl HomebrewAction {
    fn label(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Upgrade => "upgrade",
            Self::Reinstall => "reinstall",
            Self::Outdated => "outdated check",
            Self::Prefix => "prefix query",
        }
    }
}

pub fn locate_homebrew(
    definition: &PlatformToolchainDefinition,
) -> Result<HomebrewInstallation, String> {
    let path_directories = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let env_prefix = env::var_os("HOMEBREW_PREFIX").map(PathBuf::from);
    locate_homebrew_with(
        definition,
        &path_directories,
        env_prefix.as_deref(),
        Path::is_file,
        &SystemProcessRunner,
    )?
    .ok_or_else(homebrew_missing_error)
}

pub fn probe_homebrew_toolchain(
    definition: &PlatformToolchainDefinition,
) -> Result<Vec<ToolStatus>, String> {
    let path_directories = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let env_prefix = env::var_os("HOMEBREW_PREFIX").map(PathBuf::from);
    probe_homebrew_with(
        definition,
        &path_directories,
        env_prefix.as_deref(),
        Path::is_file,
        &SystemProcessRunner,
    )
}

pub fn check_homebrew_updates(
    definition: &PlatformToolchainDefinition,
) -> Result<Vec<ToolStatus>, String> {
    let installation = locate_homebrew(definition)?;
    check_homebrew_updates_with(
        definition,
        &installation,
        &SystemProcessRunner,
        &probe_installation,
    )
}

pub fn reconcile_homebrew_toolchain(
    definition: &PlatformToolchainDefinition,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ToolStatus>, String> {
    let installation = locate_homebrew(definition)?;
    reconcile_homebrew_with(
        definition,
        &installation,
        reporter,
        &SystemProcessRunner,
        &probe_installation,
    )
}

pub fn reinstall_homebrew_toolchain(
    definition: &PlatformToolchainDefinition,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ToolStatus>, String> {
    let installation = locate_homebrew(definition)?;
    reinstall_homebrew_with(
        definition,
        &installation,
        reporter,
        &SystemProcessRunner,
        &probe_installation,
    )
}

fn homebrew_packages(
    definition: &PlatformToolchainDefinition,
) -> Result<&[HomebrewPackageDefinition], String> {
    match &definition.provider {
        ManagedProviderDefinition::Homebrew { packages } => Ok(packages),
        ManagedProviderDefinition::ArchiveManifest { .. } => Err(format!(
            "Platform {} does not use the Homebrew provider",
            definition.target
        )),
    }
}

fn homebrew_candidates(path_directories: &[PathBuf], env_prefix: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = path_directories
        .iter()
        .map(|directory| directory.join("brew"))
        .collect::<Vec<_>>();
    if let Some(prefix) = env_prefix {
        candidates.push(prefix.join("bin/brew"));
    }
    candidates.push(PathBuf::from("/opt/homebrew/bin/brew"));
    candidates.push(PathBuf::from("/usr/local/bin/brew"));
    deduplicate_paths(candidates)
}

fn deduplicate_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = BTreeSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn find_homebrew_with<F>(
    path_directories: &[PathBuf],
    env_prefix: Option<&Path>,
    mut is_file: F,
) -> Option<PathBuf>
where
    F: FnMut(&Path) -> bool,
{
    homebrew_candidates(path_directories, env_prefix)
        .into_iter()
        .find(|candidate| candidate.is_absolute() && is_file(candidate))
}

fn locate_homebrew_with<F>(
    definition: &PlatformToolchainDefinition,
    path_directories: &[PathBuf],
    env_prefix: Option<&Path>,
    is_file: F,
    runner: &dyn ProcessRunner,
) -> Result<Option<HomebrewInstallation>, String>
where
    F: FnMut(&Path) -> bool,
{
    homebrew_packages(definition)?;
    let Some(brew) = find_homebrew_with(path_directories, env_prefix, is_file) else {
        return Ok(None);
    };
    let output = runner.run(&brew, &[OsString::from("--prefix")])?;
    require_success(HomebrewAction::Prefix, &output)?;
    let prefix = first_nonempty_line(&output.stdout)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            format!(
                "Homebrew prefix query returned no absolute prefix from {}",
                brew.display()
            )
        })?;
    let paths = tool_paths_for_prefix(&prefix, &definition.executable_names);
    Ok(Some(HomebrewInstallation {
        brew,
        prefix,
        paths,
    }))
}

fn tool_paths_for_prefix(prefix: &Path, names: &ExecutableNames) -> ToolPaths {
    let bin = prefix.join("bin");
    ToolPaths {
        root: prefix.to_path_buf(),
        yt_dlp: bin.join(&names.yt_dlp),
        ffmpeg: bin.join(&names.ffmpeg),
        ffmpeg_dir: bin.clone(),
        ffprobe: bin.join(&names.ffprobe),
        deno: bin.join(&names.deno),
    }
}

fn probe_homebrew_with<F>(
    definition: &PlatformToolchainDefinition,
    path_directories: &[PathBuf],
    env_prefix: Option<&Path>,
    is_file: F,
    runner: &dyn ProcessRunner,
) -> Result<Vec<ToolStatus>, String>
where
    F: FnMut(&Path) -> bool,
{
    match locate_homebrew_with(definition, path_directories, env_prefix, is_file, runner)? {
        Some(installation) => Ok(probe_installation(&installation)),
        None => Ok(provider_missing_statuses(definition)),
    }
}

fn probe_installation(installation: &HomebrewInstallation) -> Vec<ToolStatus> {
    let mut statuses = probe_local_toolchain(&LocalToolchainResolution {
        yt_dlp: Some(installation.paths.yt_dlp.clone()),
        ffmpeg: Some(installation.paths.ffmpeg.clone()),
        ffprobe: Some(installation.paths.ffprobe.clone()),
        deno: Some(installation.paths.deno.clone()),
    });
    statuses.push(super::probe_executable(
        "aria2c",
        &installation.prefix.join("bin/aria2c"),
    ));
    statuses
}

fn provider_missing_statuses(definition: &PlatformToolchainDefinition) -> Vec<ToolStatus> {
    [
        ("yt-dlp", definition.executable_names.yt_dlp.as_str()),
        ("ffmpeg", definition.executable_names.ffmpeg.as_str()),
        ("ffprobe", definition.executable_names.ffprobe.as_str()),
        ("deno", definition.executable_names.deno.as_str()),
        ("aria2c", "aria2c"),
    ]
    .into_iter()
    .map(|(name, executable)| ToolStatus {
        name: name.to_string(),
        relative_path: executable.to_string(),
        full_path: String::new(),
        availability: "provider_missing".to_string(),
        version: None,
        expected_version: None,
        error: Some(homebrew_missing_error()),
    })
    .collect()
}

fn homebrew_missing_error() -> String {
    format!("Homebrew was not found. Install Homebrew from {HOMEBREW_INSTALL_URL}")
}

fn check_homebrew_updates_with<F>(
    definition: &PlatformToolchainDefinition,
    installation: &HomebrewInstallation,
    runner: &dyn ProcessRunner,
    probe: &F,
) -> Result<Vec<ToolStatus>, String>
where
    F: Fn(&HomebrewInstallation) -> Vec<ToolStatus>,
{
    let mut statuses = probe(installation);
    let outdated = outdated_formulae_with(definition, installation, runner)?;
    mark_outdated_tools(definition, &outdated, &mut statuses);
    Ok(statuses)
}

fn outdated_formulae_with(
    definition: &PlatformToolchainDefinition,
    installation: &HomebrewInstallation,
    runner: &dyn ProcessRunner,
) -> Result<BTreeSet<String>, String> {
    let formulas = unique_formulae(homebrew_packages(definition)?);
    let mut args = vec![
        OsString::from("outdated"),
        OsString::from("--formula"),
        OsString::from("--json=v2"),
    ];
    args.extend(formulas.iter().map(OsString::from));
    let output = runner.run(&installation.brew, &args)?;
    require_success(HomebrewAction::Outdated, &output)?;
    parse_outdated_formulae(&String::from_utf8_lossy(&output.stdout))
}

#[derive(Deserialize)]
struct OutdatedResponse {
    formulae: Vec<OutdatedFormula>,
}

#[derive(Deserialize)]
struct OutdatedFormula {
    name: String,
}

fn parse_outdated_formulae(json: &str) -> Result<BTreeSet<String>, String> {
    let response: OutdatedResponse = serde_json::from_str(json)
        .map_err(|error| format!("Invalid Homebrew outdated response: {error}"))?;
    Ok(response
        .formulae
        .into_iter()
        .map(|formula| formula.name)
        .collect())
}

fn mark_outdated_tools(
    definition: &PlatformToolchainDefinition,
    outdated: &BTreeSet<String>,
    statuses: &mut [ToolStatus],
) {
    let Ok(packages) = homebrew_packages(definition) else {
        return;
    };
    for package in packages {
        if !outdated.contains(&package.formula) {
            continue;
        }
        for executable in &package.executables {
            if let Some(status_name) = status_name_for_executable(definition, executable) {
                if let Some(status) = statuses
                    .iter_mut()
                    .find(|status| status.name == status_name && status.availability == "available")
                {
                    status.availability = "outdated".to_string();
                }
            }
        }
    }
}

fn status_name_for_executable<'a>(
    definition: &'a PlatformToolchainDefinition,
    executable: &str,
) -> Option<&'a str> {
    let names = &definition.executable_names;
    [
        ("yt-dlp", names.yt_dlp.as_str()),
        ("ffmpeg", names.ffmpeg.as_str()),
        ("ffprobe", names.ffprobe.as_str()),
        ("deno", names.deno.as_str()),
        ("aria2c", "aria2c"),
    ]
    .into_iter()
    .find_map(|(status_name, candidate)| (candidate == executable).then_some(status_name))
}

fn executable_name_for_status<'a>(
    definition: &'a PlatformToolchainDefinition,
    status_name: &str,
) -> Option<&'a str> {
    let names = &definition.executable_names;
    match status_name {
        "yt-dlp" => Some(&names.yt_dlp),
        "ffmpeg" => Some(&names.ffmpeg),
        "ffprobe" => Some(&names.ffprobe),
        "deno" => Some(&names.deno),
        "aria2c" => Some("aria2c"),
        _ => None,
    }
}

fn reconcile_homebrew_with<F>(
    definition: &PlatformToolchainDefinition,
    installation: &HomebrewInstallation,
    reporter: &dyn ProgressReporter,
    runner: &dyn ProcessRunner,
    probe: &F,
) -> Result<Vec<ToolStatus>, String>
where
    F: Fn(&HomebrewInstallation) -> Vec<ToolStatus>,
{
    let statuses = probe(installation);
    let missing_executables = statuses
        .iter()
        .filter(|status| status.availability == "missing")
        .filter_map(|status| executable_name_for_status(definition, &status.name))
        .collect::<BTreeSet<_>>();

    if !missing_executables.is_empty() {
        let formulas = formulas_for_executables(definition, &missing_executables)?;
        emit_action_progress(reporter, HomebrewAction::Install, &formulas);
        run_homebrew_action(installation, HomebrewAction::Install, &formulas, runner)?;
        return final_probe(installation, reporter, probe);
    }

    let outdated = outdated_formulae_with(definition, installation, runner)?;
    let available_formulae = formulas_in_definition_order(definition, &outdated)?;
    if available_formulae.is_empty() {
        return require_ready(statuses);
    }

    emit_action_progress(reporter, HomebrewAction::Upgrade, &available_formulae);
    run_homebrew_action(
        installation,
        HomebrewAction::Upgrade,
        &available_formulae,
        runner,
    )?;
    final_probe(installation, reporter, probe)
}

fn reinstall_homebrew_with<F>(
    definition: &PlatformToolchainDefinition,
    installation: &HomebrewInstallation,
    reporter: &dyn ProgressReporter,
    runner: &dyn ProcessRunner,
    probe: &F,
) -> Result<Vec<ToolStatus>, String>
where
    F: Fn(&HomebrewInstallation) -> Vec<ToolStatus>,
{
    let formulas = unique_formulae(homebrew_packages(definition)?);
    emit_action_progress(reporter, HomebrewAction::Reinstall, &formulas);
    run_homebrew_action(installation, HomebrewAction::Reinstall, &formulas, runner)?;
    final_probe(installation, reporter, probe)
}

fn formulas_for_executables(
    definition: &PlatformToolchainDefinition,
    executables: &BTreeSet<&str>,
) -> Result<Vec<String>, String> {
    let packages = homebrew_packages(definition)?;
    Ok(unique_formulae(
        &packages
            .iter()
            .filter(|package| {
                package
                    .executables
                    .iter()
                    .any(|executable| executables.contains(executable.as_str()))
            })
            .cloned()
            .collect::<Vec<_>>(),
    ))
}

fn formulas_in_definition_order(
    definition: &PlatformToolchainDefinition,
    selected: &BTreeSet<String>,
) -> Result<Vec<String>, String> {
    Ok(unique_formulae(homebrew_packages(definition)?)
        .into_iter()
        .filter(|formula| selected.contains(formula))
        .collect())
}

fn unique_formulae(packages: &[HomebrewPackageDefinition]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    packages
        .iter()
        .filter(|package| seen.insert(package.formula.as_str()))
        .map(|package| package.formula.clone())
        .collect()
}

fn homebrew_action_args(action: HomebrewAction, formulas: &[String]) -> Vec<OsString> {
    let command = match action {
        HomebrewAction::Install => "install",
        HomebrewAction::Upgrade => "upgrade",
        HomebrewAction::Reinstall => "reinstall",
        HomebrewAction::Outdated | HomebrewAction::Prefix => {
            unreachable!("query actions use dedicated argument builders")
        }
    };
    std::iter::once(OsString::from(command))
        .chain(formulas.iter().map(OsString::from))
        .collect()
}

fn run_homebrew_action(
    installation: &HomebrewInstallation,
    action: HomebrewAction,
    formulas: &[String],
    runner: &dyn ProcessRunner,
) -> Result<(), String> {
    let output = runner.run(&installation.brew, &homebrew_action_args(action, formulas))?;
    require_success(action, &output)
}

fn require_success(action: HomebrewAction, output: &ProcessOutput) -> Result<(), String> {
    if output.success {
        return Ok(());
    }
    let detail = first_nonempty_line(&output.stderr)
        .or_else(|| first_nonempty_line(&output.stdout))
        .unwrap_or_else(|| "No process output".to_string());
    Err(format!(
        "Homebrew {} failed (exit code {}): {}",
        action.label(),
        output.exit_code.unwrap_or(-1),
        detail
    ))
}

fn first_nonempty_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}

fn emit_action_progress(
    reporter: &dyn ProgressReporter,
    action: HomebrewAction,
    formulas: &[String],
) {
    reporter.emit(ToolInstallProgress {
        percent: None,
        status: format!("Homebrew {}: {}", action.label(), formulas.join(", ")),
        tool: None,
    });
}

fn final_probe<F>(
    installation: &HomebrewInstallation,
    reporter: &dyn ProgressReporter,
    probe: &F,
) -> Result<Vec<ToolStatus>, String>
where
    F: Fn(&HomebrewInstallation) -> Vec<ToolStatus>,
{
    reporter.emit(ToolInstallProgress {
        percent: None,
        status: "Checking Homebrew toolchain compatibility".to_string(),
        tool: None,
    });
    require_ready(probe(installation))
}

fn require_ready(statuses: Vec<ToolStatus>) -> Result<Vec<ToolStatus>, String> {
    if statuses
        .iter()
        .all(|status| status.availability == "available")
    {
        return Ok(statuses);
    }

    let detail = statuses
        .iter()
        .find(|status| status.availability != "available")
        .and_then(|status| status.error.as_deref())
        .unwrap_or("Homebrew toolchain compatibility check failed");
    Err(format!(
        "Homebrew toolchain is not ready after package operation: {detail}"
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn prefix_query_terminates_after_its_deadline() {
        let root = crate::test_support::TestDirectory::new();
        let exe = root.fixture("sleep");
        let start = std::time::Instant::now();
        let result = SystemProcessRunner.run(&exe, &["--prefix".into()]);
        assert!(result.unwrap_err().contains("timed out"));
        assert!(start.elapsed() < std::time::Duration::from_secs(15));
    }

    use super::*;
    use crate::toolchain::{bundled_platform_catalog, platform_definition_from};
    use std::{cell::RefCell, collections::VecDeque, ffi::OsStr};

    #[derive(Default)]
    struct RecordingRunner {
        calls: RefCell<Vec<(PathBuf, Vec<OsString>)>>,
        outputs: RefCell<VecDeque<Result<ProcessOutput, String>>>,
    }

    impl RecordingRunner {
        fn with_outputs(outputs: impl IntoIterator<Item = ProcessOutput>) -> Self {
            Self {
                calls: RefCell::default(),
                outputs: RefCell::new(outputs.into_iter().map(Ok).collect()),
            }
        }
    }

    impl ProcessRunner for RecordingRunner {
        fn run(&self, program: &Path, args: &[OsString]) -> Result<ProcessOutput, String> {
            self.calls
                .borrow_mut()
                .push((program.to_path_buf(), args.to_vec()));
            self.outputs
                .borrow_mut()
                .pop_front()
                .expect("test must provide one output per call")
        }
    }

    #[derive(Default)]
    struct RecordingReporter {
        events: RefCell<Vec<ToolInstallProgress>>,
    }

    impl ProgressReporter for RecordingReporter {
        fn emit(&self, progress: ToolInstallProgress) {
            self.events.borrow_mut().push(progress);
        }
    }

    fn macos_definition() -> PlatformToolchainDefinition {
        let catalog = bundled_platform_catalog().unwrap();
        platform_definition_from(&catalog, "macos", "aarch64").unwrap()
    }

    fn installation() -> HomebrewInstallation {
        let prefix = PathBuf::from("/opt/homebrew");
        HomebrewInstallation {
            brew: prefix.join("bin/brew"),
            paths: tool_paths_for_prefix(&prefix, &macos_definition().executable_names),
            prefix,
        }
    }

    fn output(success: bool, exit_code: Option<i32>, stdout: &str, stderr: &str) -> ProcessOutput {
        ProcessOutput {
            success,
            exit_code,
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    fn status(name: &str, availability: &str) -> ToolStatus {
        ToolStatus {
            name: name.to_string(),
            relative_path: name.to_string(),
            full_path: format!("/opt/homebrew/bin/{name}"),
            availability: availability.to_string(),
            version: Some("test version".to_string()),
            expected_version: None,
            error: None,
        }
    }

    fn available_statuses() -> Vec<ToolStatus> {
        ["yt-dlp", "ffmpeg", "ffprobe", "deno"]
            .into_iter()
            .map(|name| status(name, "available"))
            .collect()
    }

    fn os_args<const N: usize>(values: [&str; N]) -> Vec<OsString> {
        values.into_iter().map(OsString::from).collect()
    }

    #[test]
    #[cfg(unix)]
    fn brew_discovery_prefers_path_then_environment_then_standard_prefixes() {
        let path_brew = PathBuf::from("/custom/bin/brew");
        let env_prefix = PathBuf::from("/env/homebrew");
        let existing = [path_brew.clone(), env_prefix.join("bin/brew")];

        let found =
            find_homebrew_with(&[PathBuf::from("/custom/bin")], Some(&env_prefix), |path| {
                existing.contains(&path.to_path_buf())
            });

        assert_eq!(found, Some(path_brew));
    }

    #[test]
    #[cfg(unix)]
    fn brew_discovery_finds_apple_silicon_with_minimal_path() {
        let found = find_homebrew_with(&[PathBuf::from("/usr/bin")], None, |path| {
            path == Path::new("/opt/homebrew/bin/brew")
        });
        assert_eq!(found, Some(PathBuf::from("/opt/homebrew/bin/brew")));
    }

    #[test]
    fn brew_discovery_deduplicates_without_changing_precedence() {
        let candidates = homebrew_candidates(
            &[
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
            ],
            Some(Path::new("/opt/homebrew")),
        );
        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/opt/homebrew/bin/brew"),
                PathBuf::from("/usr/local/bin/brew"),
            ]
        );
    }

    #[test]
    fn package_actions_are_direct_argument_arrays() {
        let formulas = vec![
            "yt-dlp".to_string(),
            "ffmpeg".to_string(),
            "deno".to_string(),
        ];
        assert_eq!(
            homebrew_action_args(HomebrewAction::Install, &formulas),
            os_args(["install", "yt-dlp", "ffmpeg", "deno"]),
        );
        assert_eq!(
            homebrew_action_args(HomebrewAction::Upgrade, &formulas),
            os_args(["upgrade", "yt-dlp", "ffmpeg", "deno"]),
        );
        assert_eq!(
            homebrew_action_args(HomebrewAction::Reinstall, &formulas),
            os_args(["reinstall", "yt-dlp", "ffmpeg", "deno"]),
        );
    }

    #[test]
    fn outdated_ffmpeg_marks_both_media_executables() {
        let definition = macos_definition();
        let outdated =
            parse_outdated_formulae(r#"{"formulae":[{"name":"ffmpeg"}],"casks":[]}"#).unwrap();
        let mut statuses = available_statuses();

        mark_outdated_tools(&definition, &outdated, &mut statuses);

        assert!(outdated.contains("ffmpeg"));
        assert_eq!(statuses[0].availability, "available");
        assert_eq!(statuses[1].availability, "outdated");
        assert_eq!(statuses[2].availability, "outdated");
        assert_eq!(statuses[3].availability, "available");
    }

    #[test]
    #[cfg(unix)]
    fn locating_uses_absolute_brew_and_builds_prefix_bin_paths() {
        let runner =
            RecordingRunner::with_outputs([output(true, Some(0), "\n/opt/homebrew\n", "")]);
        let definition = macos_definition();

        let found = locate_homebrew_with(
            &definition,
            &[PathBuf::from("/usr/bin")],
            None,
            |path| path == Path::new("/opt/homebrew/bin/brew"),
            &runner,
        )
        .unwrap()
        .unwrap();

        assert_eq!(found.brew, PathBuf::from("/opt/homebrew/bin/brew"));
        assert_eq!(found.prefix, PathBuf::from("/opt/homebrew"));
        assert_eq!(
            found.paths.yt_dlp,
            PathBuf::from("/opt/homebrew/bin/yt-dlp")
        );
        assert_eq!(
            found.paths.ffmpeg,
            PathBuf::from("/opt/homebrew/bin/ffmpeg")
        );
        assert_eq!(
            found.paths.ffprobe,
            PathBuf::from("/opt/homebrew/bin/ffprobe")
        );
        assert_eq!(found.paths.deno, PathBuf::from("/opt/homebrew/bin/deno"));
        assert_eq!(
            runner.calls.borrow().as_slice(),
            &[(
                PathBuf::from("/opt/homebrew/bin/brew"),
                os_args(["--prefix"]),
            )]
        );
    }

    #[test]
    fn missing_brew_returns_provider_missing_statuses_with_install_url() {
        let runner = RecordingRunner::default();
        let statuses =
            probe_homebrew_with(&macos_definition(), &[], None, |_| false, &runner).unwrap();

        assert_eq!(statuses.len(), 5);
        assert!(statuses
            .iter()
            .all(|status| status.availability == "provider_missing"));
        assert!(statuses.iter().all(|status| status
            .error
            .as_deref()
            .is_some_and(|error| error.contains("https://brew.sh/"))));
        assert!(runner.calls.borrow().is_empty());
    }

    #[test]
    fn outdated_check_uses_exact_arguments_and_marks_formula_executables() {
        let runner = RecordingRunner::with_outputs([output(
            true,
            Some(0),
            r#"{"formulae":[{"name":"ffmpeg"}],"casks":[]}"#,
            "",
        )]);
        let definition = macos_definition();
        let installation = installation();

        let statuses = check_homebrew_updates_with(&definition, &installation, &runner, &|_| {
            available_statuses()
        })
        .unwrap();

        assert_eq!(statuses[1].availability, "outdated");
        assert_eq!(statuses[2].availability, "outdated");
        assert_eq!(
            runner.calls.borrow().as_slice(),
            &[(
                PathBuf::from("/opt/homebrew/bin/brew"),
                os_args([
                    "outdated",
                    "--formula",
                    "--json=v2",
                    "yt-dlp",
                    "ffmpeg",
                    "deno",
                    "aria2",
                ]),
            )]
        );
    }

    #[test]
    fn malformed_outdated_json_is_rejected() {
        let error = parse_outdated_formulae("not json").unwrap_err();
        assert!(error.contains("Invalid Homebrew outdated response"));
    }

    #[test]
    fn missing_aria2c_installs_aria2_formula() {
        let runner = RecordingRunner::with_outputs([output(true, Some(0), "", "")]);
        let reporter = RecordingReporter::default();
        let mut missing = available_statuses();
        missing.push(status("aria2c", "missing"));
        let mut ready = available_statuses();
        ready.push(status("aria2c", "available"));
        let probes = RefCell::new(VecDeque::from([missing, ready]));
        let statuses = reconcile_homebrew_with(
            &macos_definition(),
            &installation(),
            &reporter,
            &runner,
            &|_| probes.borrow_mut().pop_front().unwrap(),
        )
        .unwrap();
        assert!(statuses
            .iter()
            .all(|status| status.availability == "available"));
        assert_eq!(
            runner.calls.borrow().as_slice(),
            &[(
                PathBuf::from("/opt/homebrew/bin/brew"),
                os_args(["install", "aria2"]),
            )]
        );
    }

    #[test]
    fn aria2_updates_mark_aria2c_outdated() {
        let runner = RecordingRunner::with_outputs([output(
            true,
            Some(0),
            r#"{"formulae":[{"name":"aria2"}],"casks":[]}"#,
            "",
        )]);
        let statuses =
            check_homebrew_updates_with(&macos_definition(), &installation(), &runner, &|_| {
                let mut statuses = available_statuses();
                statuses.push(status("aria2c", "available"));
                statuses
            })
            .unwrap();
        assert_eq!(
            statuses
                .iter()
                .find(|status| status.name == "aria2c")
                .unwrap()
                .availability,
            "outdated"
        );
        assert!(runner.calls.borrow()[0]
            .1
            .contains(&OsString::from("aria2")));
    }

    #[test]
    fn install_uses_only_formulae_for_missing_executables() {
        let runner = RecordingRunner::with_outputs([output(true, Some(0), "", "")]);
        let reporter = RecordingReporter::default();
        let mut final_statuses = available_statuses();
        final_statuses[1].availability = "missing".to_string();
        final_statuses[2].availability = "missing".to_string();
        let probes = RefCell::new(VecDeque::from([final_statuses, available_statuses()]));

        let statuses = reconcile_homebrew_with(
            &macos_definition(),
            &installation(),
            &reporter,
            &runner,
            &|_| probes.borrow_mut().pop_front().unwrap(),
        )
        .unwrap();

        assert!(statuses
            .iter()
            .all(|status| status.availability == "available"));
        assert_eq!(
            runner.calls.borrow().as_slice(),
            &[(
                PathBuf::from("/opt/homebrew/bin/brew"),
                os_args(["install", "ffmpeg"]),
            )]
        );
        assert_eq!(reporter.events.borrow().len(), 2);
    }

    #[test]
    fn upgrade_uses_only_outdated_formulae() {
        let runner = RecordingRunner::with_outputs([
            output(
                true,
                Some(0),
                r#"{"formulae":[{"name":"yt-dlp"},{"name":"deno"}],"casks":[]}"#,
                "",
            ),
            output(true, Some(0), "", ""),
        ]);
        let reporter = RecordingReporter::default();

        reconcile_homebrew_with(
            &macos_definition(),
            &installation(),
            &reporter,
            &runner,
            &|_| available_statuses(),
        )
        .unwrap();

        assert_eq!(
            runner.calls.borrow().as_slice(),
            &[
                (
                    PathBuf::from("/opt/homebrew/bin/brew"),
                    os_args([
                        "outdated",
                        "--formula",
                        "--json=v2",
                        "yt-dlp",
                        "ffmpeg",
                        "deno",
                        "aria2",
                    ]),
                ),
                (
                    PathBuf::from("/opt/homebrew/bin/brew"),
                    os_args(["upgrade", "yt-dlp", "deno"]),
                ),
            ]
        );
    }

    #[test]
    fn reinstall_uses_all_unique_formulae_and_probes_afterward() {
        let runner = RecordingRunner::with_outputs([output(true, Some(0), "", "")]);
        let reporter = RecordingReporter::default();

        reinstall_homebrew_with(
            &macos_definition(),
            &installation(),
            &reporter,
            &runner,
            &|_| available_statuses(),
        )
        .unwrap();

        assert_eq!(
            runner.calls.borrow().as_slice(),
            &[(
                PathBuf::from("/opt/homebrew/bin/brew"),
                os_args(["reinstall", "yt-dlp", "ffmpeg", "deno", "aria2"]),
            )]
        );
        assert_eq!(reporter.events.borrow().len(), 2);
    }

    #[test]
    fn nonzero_action_preserves_stderr_and_never_uses_a_shell() {
        let runner = RecordingRunner::with_outputs([output(
            false,
            Some(17),
            "less useful stdout",
            "formula permission denied\nmore detail",
        )]);
        let error = run_homebrew_action(
            &installation(),
            HomebrewAction::Install,
            &["yt-dlp".to_string()],
            &runner,
        )
        .unwrap_err();

        assert_eq!(
            error,
            "Homebrew install failed (exit code 17): formula permission denied"
        );
        let calls = runner.calls.borrow();
        assert_eq!(calls[0].0, PathBuf::from("/opt/homebrew/bin/brew"));
        assert_eq!(calls[0].1, os_args(["install", "yt-dlp"]));
        assert!(calls.iter().all(|(program, _)| !matches!(
            program.file_name().and_then(OsStr::to_str),
            Some("sh" | "bash" | "zsh")
        )));
    }

    #[test]
    fn system_runner_spawn_error_names_absolute_program() {
        let program = Path::new("/definitely/missing/homebrew-test/brew");
        let error = SystemProcessRunner.run(program, &[]).unwrap_err();

        assert!(error.contains(program.to_str().unwrap()));
    }

    #[test]
    fn final_probe_must_pass_before_package_operation_succeeds() {
        let runner = RecordingRunner::with_outputs([output(true, Some(0), "", "")]);
        let reporter = RecordingReporter::default();
        let error = reinstall_homebrew_with(
            &macos_definition(),
            &installation(),
            &reporter,
            &runner,
            &|_| {
                let mut statuses = available_statuses();
                statuses[0].availability = "cannot_execute".to_string();
                statuses[0].error = Some("compatibility fixture failed".to_string());
                statuses
            },
        )
        .unwrap_err();

        assert!(error.contains("compatibility fixture failed"));
    }
}
