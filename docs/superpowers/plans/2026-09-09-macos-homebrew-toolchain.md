# macOS Homebrew Toolchain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make yt-dlp-tauri run and produce native bundles on macOS, using Homebrew for the managed toolchain while retaining custom executable paths and all existing Windows archive behavior.

**Architecture:** Add a bundled platform catalog that maps OS/architecture pairs to an archive-manifest or Homebrew managed provider. Keep persisted source values as `managed`/`local`, route backend operations through the selected provider, and send provider labels/capabilities to the frontend so Windows displays Managed/Local while macOS displays Homebrew/Custom.

**Tech Stack:** Rust 2021, Tauri 2, serde/serde_json, std::process, vanilla TypeScript, Vite, Node test runner, GitHub Actions, Homebrew.

**Spec:** `docs/superpowers/specs/2026-09-09-macos-homebrew-toolchain-design.md`

## Global Constraints

- Preserve the existing Windows x64 archive manifest, SHA-256 verification, atomic activation, NSIS release, and toolchain publication workflows.
- Support runtime definitions for `windows/x86_64`, `macos/aarch64`, and `macos/x86_64`; reject other pairs with an actionable error.
- Keep persisted tool source values exactly `managed` and `local`.
- On macOS, `managed` means Homebrew and `local` is displayed as Custom.
- Never interpolate a user-controlled value into a shell command; invoke the resolved `brew` executable with an argument array.
- Formula names come only from the validated bundled platform definition.
- Do not silently install Homebrew or execute its remote bootstrap script.
- Do not commit or bundle third-party macOS tool binaries.
- macOS `.app` and `.dmg` outputs remain unsigned and unnotarized development artifacts.
- Keep downloads at `~/Downloads/yt-dlp-tauri`; store macOS state and logs under `~/Library/Application Support/yt-dlp-tauri`.
- Do not stage the pre-existing untracked `.pi/` directory.

---

## File Structure

### New files

- `src-tauri/platform-toolchains.json` — schema-versioned OS/architecture, provider, executable, formula, label, capability, and default-source definitions.
- `src-tauri/src/toolchain/platform.rs` — catalog parsing, validation, platform selection, and serializable frontend presentation.
- `src-tauri/src/toolchain/homebrew.rs` — Brew discovery, direct command execution, formula state parsing, tool path resolution, and package operations.
- `src/platform-toolchain.ts` — frontend platform/provider types and pure UI decision helpers.
- `tests/platform-toolchain-ui.test.ts` — provider label, action, summary-mode, and executable-picker tests.
- `tests/macos-support.test.ts` — macOS Tauri configuration and CI contract tests.
- `src-tauri/tauri.macos.conf.json` — macOS-specific `app`/`dmg` targets and icon configuration.

### Modified files

- `src-tauri/src/toolchain/mod.rs` — register/export the platform and Homebrew modules while leaving archive-only helpers intact.
- `src-tauri/src/toolchain/local.rs` — resolve custom tools from platform executable names and Finder-safe search directories.
- `src-tauri/src/lib.rs` — provider dispatch, AppState platform metadata, source defaults, macOS paths, and macOS folder opening.
- `src/toolchain.ts` — recognize Homebrew status/summary behavior without changing archive summaries.
- `src/main.ts` — render provider-specific labels/actions, request confirmation for Brew mutations, branch update checks, and remove the `.exe` picker restriction on macOS.
- `index.html` — add stable wrappers for provider-specific managed details and guidance.
- `src-tauri/tauri.conf.json` — bundle the platform catalog alongside the existing archive manifest.
- `.github/workflows/ci.yml` — add native macOS tests and `.app` build verification.
- `README.md`, `README_zh.md`, `PRODUCT.md`, `src-tauri/Tools/README.md`, `package.json`, `CHANGELOG.md` — document and describe macOS/Homebrew support.

---

### Task 1: Add the validated platform toolchain catalog

**Files:**

- Create: `src-tauri/platform-toolchains.json`
- Create: `src-tauri/src/toolchain/platform.rs`
- Modify: `src-tauri/src/toolchain/mod.rs:1-29, 123-270, 329-347`
- Modify: `src-tauri/tauri.conf.json:29-39`
- Test: inline Rust tests in `src-tauri/src/toolchain/platform.rs`

**Interfaces:**

- Consumes: `ToolchainSource` from `toolchain/local.rs`.
- Produces: `parse_platform_catalog(json: &str) -> Result<PlatformCatalog, String>`.
- Produces: `platform_definition_from(catalog: &PlatformCatalog, os: &str, arch: &str) -> Result<PlatformToolchainDefinition, String>`.
- Produces: `bundled_platform_catalog() -> Result<PlatformCatalog, String>`.
- Produces: `ManagedProviderDefinition::{ArchiveManifest { manifest_target }, Homebrew { packages }}`.
- Produces: `PlatformToolchainDefinition::{target, os, arch, provider, source_labels, default_source, executable_names, capabilities}`.
- Produces: `PlatformPresentation`, serialized with camelCase fields for `AppState`.

- [ ] **Step 1: Write failing catalog parsing and selection tests**

Add tests that prove the catalog is data-driven and rejects malformed definitions:

```rust
#[test]
fn bundled_catalog_selects_windows_and_both_macos_architectures() {
    let catalog = bundled_platform_catalog().expect("bundled catalog should parse");

    let windows = platform_definition_from(&catalog, "windows", "x86_64").unwrap();
    assert_eq!(windows.target, "win-x64");
    assert_eq!(windows.executable_names.yt_dlp, "yt-dlp.exe");
    assert_eq!(windows.source_labels.managed, "Managed");

    let arm = platform_definition_from(&catalog, "macos", "aarch64").unwrap();
    assert_eq!(arm.target, "macos-arm64");
    assert_eq!(arm.executable_names.yt_dlp, "yt-dlp");
    assert_eq!(arm.source_labels.managed, "Homebrew");
    assert_eq!(arm.source_labels.local, "Custom");

    let intel = platform_definition_from(&catalog, "macos", "x86_64").unwrap();
    assert_eq!(intel.target, "macos-x64");
}

#[test]
fn catalog_rejects_duplicate_platform_pairs() {
    let json = r#"{
      "schemaVersion": 1,
      "targets": [
        {"target":"one","os":"macos","arch":"aarch64","provider":{"kind":"homebrew","packages":[{"formula":"yt-dlp","executables":["yt-dlp"]}]},"sourceLabels":{"managed":"Homebrew","local":"Custom"},"defaultSource":"managed","executableNames":{"ytDlp":"yt-dlp","ffmpeg":"ffmpeg","ffprobe":"ffprobe","deno":"deno"},"capabilities":{"install":true,"update":true,"reinstall":true}},
        {"target":"two","os":"macos","arch":"aarch64","provider":{"kind":"homebrew","packages":[{"formula":"yt-dlp","executables":["yt-dlp"]}]},"sourceLabels":{"managed":"Homebrew","local":"Custom"},"defaultSource":"managed","executableNames":{"ytDlp":"yt-dlp","ffmpeg":"ffmpeg","ffprobe":"ffprobe","deno":"deno"},"capabilities":{"install":true,"update":true,"reinstall":true}}
      ]
    }"#;

    assert!(parse_platform_catalog(json)
        .unwrap_err()
        .contains("duplicate platform mapping"));
}

#[test]
fn unsupported_platform_error_names_os_and_architecture() {
    let catalog = bundled_platform_catalog().unwrap();
    let error = platform_definition_from(&catalog, "linux", "x86_64").unwrap_err();
    assert!(error.contains("linux-x86_64"));
}
```

Also cover unsupported schema versions, duplicate target IDs, empty executable names, empty Homebrew formulas/executable lists, an archive provider whose `manifestTarget` is empty, and a default source other than `managed`/`local`.

- [ ] **Step 2: Run the tests and confirm they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml toolchain::platform::tests --lib
```

Expected: compilation fails because `platform.rs` and its exported interfaces do not exist.

- [ ] **Step 3: Add the platform catalog data**

Create `src-tauri/platform-toolchains.json` with schema version 1 and these exact targets:

```json
{
  "schemaVersion": 1,
  "targets": [
    {
      "target": "win-x64",
      "os": "windows",
      "arch": "x86_64",
      "provider": { "kind": "archive-manifest", "manifestTarget": "win-x64" },
      "sourceLabels": { "managed": "Managed", "local": "Local" },
      "defaultSource": "managed",
      "executableNames": {
        "ytDlp": "yt-dlp.exe",
        "ffmpeg": "ffmpeg.exe",
        "ffprobe": "ffprobe.exe",
        "deno": "deno.exe"
      },
      "capabilities": { "install": true, "update": true, "reinstall": true }
    },
    {
      "target": "macos-arm64",
      "os": "macos",
      "arch": "aarch64",
      "provider": {
        "kind": "homebrew",
        "packages": [
          { "formula": "yt-dlp", "executables": ["yt-dlp"] },
          { "formula": "ffmpeg", "executables": ["ffmpeg", "ffprobe"] },
          { "formula": "deno", "executables": ["deno"] }
        ]
      },
      "sourceLabels": { "managed": "Homebrew", "local": "Custom" },
      "defaultSource": "managed",
      "executableNames": {
        "ytDlp": "yt-dlp",
        "ffmpeg": "ffmpeg",
        "ffprobe": "ffprobe",
        "deno": "deno"
      },
      "capabilities": { "install": true, "update": true, "reinstall": true }
    },
    {
      "target": "macos-x64",
      "os": "macos",
      "arch": "x86_64",
      "provider": {
        "kind": "homebrew",
        "packages": [
          { "formula": "yt-dlp", "executables": ["yt-dlp"] },
          { "formula": "ffmpeg", "executables": ["ffmpeg", "ffprobe"] },
          { "formula": "deno", "executables": ["deno"] }
        ]
      },
      "sourceLabels": { "managed": "Homebrew", "local": "Custom" },
      "defaultSource": "managed",
      "executableNames": {
        "ytDlp": "yt-dlp",
        "ffmpeg": "ffmpeg",
        "ffprobe": "ffprobe",
        "deno": "deno"
      },
      "capabilities": { "install": true, "update": true, "reinstall": true }
    }
  ]
}
```

- [ ] **Step 4: Implement strict parsing, validation, and presentation**

Use owned strings so catalog data, rather than target-name conditionals, supplies executable names:

```rust
const PLATFORM_CATALOG_SCHEMA_VERSION: u32 = 1;
const BUNDLED_PLATFORM_CATALOG: &str = include_str!("../../platform-toolchains.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformCatalog {
    pub schema_version: u32,
    pub targets: Vec<PlatformToolchainDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformToolchainDefinition {
    pub target: String,
    pub os: String,
    pub arch: String,
    pub provider: ManagedProviderDefinition,
    pub source_labels: SourceLabels,
    pub default_source: ToolchainSource,
    pub executable_names: ExecutableNames,
    pub capabilities: ProviderCapabilities,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ManagedProviderDefinition {
    ArchiveManifest { manifest_target: String },
    Homebrew { packages: Vec<HomebrewPackageDefinition> },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HomebrewPackageDefinition {
    pub formula: String,
    pub executables: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutableNames {
    pub yt_dlp: String,
    pub ffmpeg: String,
    pub ffprobe: String,
    pub deno: String,
}
```

Derive `Serialize` for `SourceLabels`, `ProviderCapabilities`, and the frontend-facing presentation. Add a provider-kind serializer that emits exactly `archive-manifest` or `homebrew`. Validation must use sets for duplicate IDs/pairs and reject trimmed-empty names.

Keep the existing `tool_names_for_target` and `tool_paths_for_root` functions archive-specific for activation/legacy Windows paths; remove `tool_target_from` only after all callers use the new catalog in Task 3.

- [ ] **Step 5: Bundle the catalog resource and expose the module**

Add `mod platform;` and the required `pub use platform::{...};` exports in `toolchain/mod.rs`. Add `"platform-toolchains.json"` next to `"tools-manifest.json"` in `bundle.resources`; retaining `include_str!` gives deterministic parsing in tests while the resource keeps the deployment contract explicit.

- [ ] **Step 6: Run catalog and existing toolchain tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml toolchain::platform::tests --lib
cargo test --manifest-path src-tauri/Cargo.toml toolchain::tests --lib
```

Expected: both commands pass; Windows archive target/name tests remain unchanged.

- [ ] **Step 7: Commit the catalog**

```bash
git add src-tauri/platform-toolchains.json src-tauri/src/toolchain/platform.rs src-tauri/src/toolchain/mod.rs src-tauri/tauri.conf.json
git commit -m "feat: define platform toolchain providers"
```

---

### Task 2: Implement the Homebrew provider

**Files:**

- Create: `src-tauri/src/toolchain/homebrew.rs`
- Modify: `src-tauri/src/toolchain/mod.rs:1-29`
- Modify: `src-tauri/src/toolchain/probe.rs:203-217`
- Test: inline Rust tests in `src-tauri/src/toolchain/homebrew.rs`

**Interfaces:**

- Consumes: `PlatformToolchainDefinition`, `HomebrewPackageDefinition`, `ExecutableNames`, `ToolPaths`, `ToolStatus`, `ProgressReporter`, and `probe_local_toolchain`.
- Produces: `locate_homebrew(definition: &PlatformToolchainDefinition) -> Result<HomebrewInstallation, String>`.
- Produces: `probe_homebrew_toolchain(definition: &PlatformToolchainDefinition) -> Result<Vec<ToolStatus>, String>`; missing Brew returns statuses with `availability == "provider_missing"`, not a command error.
- Produces: `check_homebrew_updates(definition: &PlatformToolchainDefinition) -> Result<Vec<ToolStatus>, String>`.
- Produces: `reconcile_homebrew_toolchain(definition: &PlatformToolchainDefinition, reporter: &dyn ProgressReporter) -> Result<Vec<ToolStatus>, String>`.
- Produces: `reinstall_homebrew_toolchain(definition: &PlatformToolchainDefinition, reporter: &dyn ProgressReporter) -> Result<Vec<ToolStatus>, String>`.
- Produces: testable `ProcessRunner`/`ProcessOutput` boundary; production implementation calls `Command::new(program).args(args).output()` directly.

- [ ] **Step 1: Write failing tests for discovery, commands, and formula state**

Cover Finder-safe discovery precedence and exact, non-shell commands:

```rust
#[test]
fn brew_discovery_prefers_path_then_environment_then_standard_prefixes() {
    let path_brew = PathBuf::from("/custom/bin/brew");
    let env_prefix = PathBuf::from("/env/homebrew");
    let existing = [path_brew.clone(), env_prefix.join("bin/brew")];

    let found = find_homebrew_with(
        &[PathBuf::from("/custom/bin")],
        Some(&env_prefix),
        |path| existing.contains(&path.to_path_buf()),
    );

    assert_eq!(found, Some(path_brew));
}

#[test]
fn brew_discovery_finds_apple_silicon_with_minimal_path() {
    let found = find_homebrew_with(&[PathBuf::from("/usr/bin")], None, |path| {
        path == Path::new("/opt/homebrew/bin/brew")
    });
    assert_eq!(found, Some(PathBuf::from("/opt/homebrew/bin/brew")));
}

#[test]
fn package_actions_are_direct_argument_arrays() {
    let formulas = vec!["yt-dlp".to_string(), "ffmpeg".to_string(), "deno".to_string()];
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
    let outdated = parse_outdated_formulae(
        r#"{"formulae":[{"name":"ffmpeg"}],"casks":[]}"#,
    ).unwrap();
    assert!(outdated.contains("ffmpeg"));
}
```

Use a recording fake to assert the executable and arguments without changing installed formulas:

```rust
#[derive(Default)]
struct RecordingRunner {
    calls: RefCell<Vec<(PathBuf, Vec<OsString>)>>,
    outputs: RefCell<VecDeque<Result<ProcessOutput, String>>>,
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
```

Assert the recorded executable is the resolved absolute Brew path, arguments are separate `OsString` values, stderr survives non-zero exits, and no recorded program file name is `sh`, `bash`, or `zsh`. Also test malformed `brew outdated --json=v2` output and missing Brew status text containing `https://brew.sh/`.

- [ ] **Step 2: Run the tests and confirm they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml toolchain::homebrew::tests --lib
```

Expected: compilation fails because the Homebrew module does not exist.

- [ ] **Step 3: Implement Brew discovery and process isolation**

Use this candidate order, deduplicated without changing precedence:

```rust
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
```

Represent process output without platform-specific `ExitStatus` construction in tests:

```rust
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
```

The system runner must use `Command::new(program).args(args).output()` and return an error naming the absolute program when spawn fails.

- [ ] **Step 4: Implement prefix/tool resolution and status probing**

Run `brew --prefix`, trim one non-empty stdout line, and build all four paths under `<prefix>/bin`. Map FFmpeg’s one formula to both `ffmpeg` and `ffprobe`.

```rust
pub struct HomebrewInstallation {
    pub brew: PathBuf,
    pub prefix: PathBuf,
    pub paths: ToolPaths,
}
```

If Brew is missing, `probe_homebrew_toolchain` must return four `provider_missing` statuses so `get_app_state` and Settings still load. If Brew exists, reuse `probe_local_toolchain` so version probes and the deterministic combined compatibility fixture remain identical.

- [ ] **Step 5: Implement install, update, and reinstall operations**

- Install/reconcile: probe executable paths; if any are missing, invoke `brew install` with the unique formulas that provide missing executables.
- Update check: invoke `brew outdated --formula --json=v2 yt-dlp ffmpeg deno`, parse `formulae[].name`, and change each executable supplied by an outdated formula from `available` to `outdated`.
- Upgrade: when reconcile sees no missing tools but outdated formulas exist, invoke `brew upgrade` with only those formulas.
- Reinstall: invoke `brew reinstall yt-dlp ffmpeg deno`.
- Emit progress before command launch and before final probing. Never report success until the shared compatibility probe passes.

Format failure as:

```rust
format!(
    "Homebrew {} failed (exit code {}): {}",
    action.label(),
    output.exit_code.unwrap_or(-1),
    first_nonempty_line(&output.stderr)
        .or_else(|| first_nonempty_line(&output.stdout))
        .unwrap_or("No process output")
)
```

- [ ] **Step 6: Run Homebrew and probe tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml toolchain::homebrew::tests --lib
cargo test --manifest-path src-tauri/Cargo.toml toolchain::probe::tests --lib
```

Expected: all pass without changing the machine’s installed formulas; every package-changing test uses the fake runner.

- [ ] **Step 7: Commit the provider**

```bash
git add src-tauri/src/toolchain/homebrew.rs src-tauri/src/toolchain/mod.rs src-tauri/src/toolchain/probe.rs
git commit -m "feat: add Homebrew toolchain provider"
```

---

### Task 3: Route backend state and commands through the selected provider

**Files:**

- Modify: `src-tauri/src/lib.rs:31-52, 146-401, 620-736, 951-1038, 1307-1328, 1651-1711, 1854-2218`
- Modify: `src-tauri/src/toolchain/mod.rs:1-29, 240-270, 329-347`
- Test: inline Rust tests in `src-tauri/src/lib.rs`

**Interfaces:**

- Consumes: Task 1 platform definitions/presentation and Task 2 Homebrew operations.
- Produces: `AppState.platform: PlatformPresentation`.
- Produces: `check_managed_tool_updates(app: AppHandle, github_access_mode: String) -> Result<ManagedToolUpdateResult, String>`.
- Produces: `ManagedToolUpdateResult { tools, manifest_json, remote_revision }`, serialized as camelCase.
- Produces: `archive_manifest_target(definition: &PlatformToolchainDefinition) -> Result<&str, String>` for archive-only guards.
- Preserves: existing Tauri command names for install/reinstall and the archive-only manifest commands.

- [ ] **Step 1: Write failing tests for source defaults and provider dispatch**

Refactor source parsing to accept the platform default and add:

```rust
#[test]
fn absent_source_uses_platform_default() {
    assert_eq!(
        parse_toolchain_source_state(None, ToolchainSource::Managed).unwrap(),
        ToolchainSource::Managed,
    );
    assert_eq!(
        parse_toolchain_source_state(None, ToolchainSource::Local).unwrap(),
        ToolchainSource::Local,
    );
}

#[test]
fn provider_kind_controls_archive_manifest_access() {
    let catalog = bundled_platform_catalog().unwrap();
    let windows = platform_definition_from(&catalog, "windows", "x86_64").unwrap();
    let macos = platform_definition_from(&catalog, "macos", "aarch64").unwrap();

    assert!(archive_manifest_target(&windows).is_ok());
    assert!(archive_manifest_target(&macos)
        .unwrap_err()
        .contains("Homebrew"));
}
```

Add a serde test confirming `PlatformPresentation` emits `target`, `managedProvider`, `sourceLabels`, `capabilities`, and `executableExtension`; Windows extension is `"exe"`, macOS is `null`.

- [ ] **Step 2: Run the focused tests and confirm failure**

```bash
cargo test --manifest-path src-tauri/Cargo.toml absent_source_uses_platform_default --lib
cargo test --manifest-path src-tauri/Cargo.toml provider_kind_controls_archive_manifest_access --lib
```

Expected: compilation fails because the parser signature and provider helpers are not implemented.

- [ ] **Step 3: Make platform selection a single backend dependency**

Replace `current_tool_target()` with:

```rust
fn current_platform_definition() -> Result<PlatformToolchainDefinition, String> {
    let catalog = bundled_platform_catalog()?;
    let os = env::var("YT_DLP_TOOL_OS").unwrap_or_else(|_| env::consts::OS.to_string());
    let arch = env::var("YT_DLP_TOOL_ARCH").unwrap_or_else(|_| env::consts::ARCH.to_string());
    platform_definition_from(&catalog, &os, &arch)
}
```

Keep `YT_DLP_TOOL_TARGET` only as a backwards-compatible Windows archive override: resolve it against a catalog target and reject an OS/architecture mismatch unless tests explicitly opt into `YT_DLP_TOOL_OS`/`YT_DLP_TOOL_ARCH`. Remove the old hard-coded supported-target error and old `tool_target_from` usage.

Change `read_toolchain_source` to accept the selected definition’s `default_source`. Every command must select the platform once and pass `&PlatformToolchainDefinition` down rather than re-reading environment state in nested helpers.

- [ ] **Step 4: Extend AppState and make startup non-failing without Brew**

Add:

```rust
#[derive(Debug, Serialize)]
struct AppState {
    download_directory: String,
    tools_root: String,
    toolchain_revision: Option<String>,
    toolchain_source: ToolchainSource,
    platform: PlatformPresentation,
    local_toolchain: LocalToolchainConfig,
    local_toolchain_paths: LocalToolchainPaths,
    cookies_file: Option<String>,
}
```

Refactor `build_app_state` to accept `&AppHandle` and `&PlatformToolchainDefinition`. Add an injected `AppHandle` parameter to `set_download_directory`, `reset_download_directory`, `set_cookies_file`, and `clear_cookies_file`; Tauri supplies it without changing frontend invoke payloads. This removes the current `build_app_state(String::new())` calls and lets every returned state use one platform-aware builder.

For archive providers, preserve active revision/root behavior. For Homebrew, expose the discovered Brew prefix when available, otherwise use an empty root and `None` revision. Failure to locate Brew must not fail `get_app_state`.

- [ ] **Step 5: Dispatch managed check/install/reinstall operations**

Use exhaustive provider matches:

```rust
match &platform.provider {
    ManagedProviderDefinition::ArchiveManifest { manifest_target } => {
        // Existing manifest read/probe/install path, unchanged.
    }
    ManagedProviderDefinition::Homebrew { .. } => {
        probe_homebrew_toolchain(&platform)
    }
}
```

For `install_tools`, call existing atomic archive installation on Windows and `reconcile_homebrew_toolchain` on macOS. For `reinstall_tools`, preserve the optional archive manifest behavior and call `reinstall_homebrew_toolchain` for Homebrew. Archive-only commands (`check_tools_with_manifest`, `install_tools_from_manifest`, release-manifest fetching) must return an explicit provider error if invoked on Homebrew rather than trying to find a macOS archive target.

- [ ] **Step 6: Add a unified managed update-check command**

Register `check_managed_tool_updates` in `tauri::generate_handler!`. Return:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedToolUpdateResult {
    tools: Vec<ToolStatus>,
    manifest_json: Option<String>,
    remote_revision: Option<String>,
}
```

On Homebrew, return statuses from `check_homebrew_updates` with both optional fields `None`. On Windows, reuse the existing stable-channel/legacy manifest fetch and remote manifest probing, returning the manifest JSON/revision needed by `install_tools_from_manifest`. Keep GitHub proxy behavior entirely inside the archive branch.

- [ ] **Step 7: Run all Rust library tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
```

Expected: all tests pass on macOS; existing Windows archive tests remain green.

- [ ] **Step 8: Commit provider dispatch**

```bash
git add src-tauri/src/lib.rs src-tauri/src/toolchain/mod.rs
git commit -m "feat: dispatch toolchains by platform provider"
```

---

### Task 4: Make custom discovery and operating-system paths macOS-safe

**Files:**

- Modify: `src-tauri/src/toolchain/local.rs:1-243, 265-437`
- Modify: `src-tauri/src/lib.rs:620-699, 1307-1334, 1621-1663, 1713-1754, 1821-1834, tests after 1846`
- Test: inline Rust tests in both files

**Interfaces:**

- Consumes: `ExecutableNames` from Task 1.
- Produces: `resolve_local_toolchain(config: &LocalToolchainConfig, names: &ExecutableNames, os: &str) -> Result<LocalToolchainResolution, String>`.
- Produces: `tool_search_directories(inherited_path: Option<&OsStr>, homebrew_prefix: Option<&Path>, os: &str) -> Vec<PathBuf>`.
- Produces: pure `app_data_root_from(os, home, local_app_data, xdg_data_home)` and `open_path_program(os)` helpers.

- [ ] **Step 1: Write failing macOS custom-discovery tests**

```rust
#[test]
fn macos_custom_discovery_uses_extensionless_names() {
    let names = ExecutableNames {
        yt_dlp: "yt-dlp".to_string(),
        ffmpeg: "ffmpeg".to_string(),
        ffprobe: "ffprobe".to_string(),
        deno: "deno".to_string(),
    };
    let directory = absolute_path(&["opt", "homebrew", "bin"]);
    let available = [
        directory.join("yt-dlp"),
        directory.join("ffmpeg"),
        directory.join("ffprobe"),
        directory.join("deno"),
    ];

    let resolution = resolve_local_toolchain_with(
        &LocalToolchainConfig::default(),
        &names,
        &[directory],
        |path| available.contains(&path.to_path_buf()),
    ).unwrap();

    assert!(resolution.complete_paths().is_ok());
}

#[test]
fn finder_safe_search_adds_both_standard_homebrew_bins() {
    let directories = tool_search_directories(
        Some(OsStr::new("/usr/bin:/bin")),
        None,
        "macos",
    );
    assert!(directories.contains(&PathBuf::from("/opt/homebrew/bin")));
    assert!(directories.contains(&PathBuf::from("/usr/local/bin")));
}
```

Retain tests proving explicit absolute paths override discovered paths and FFmpeg/FFprobe must share one directory.

- [ ] **Step 2: Write failing macOS filesystem/open tests**

```rust
#[test]
fn macos_state_uses_application_support() {
    assert_eq!(
        app_data_root_from("macos", Some(Path::new("/Users/test")), None, None).unwrap(),
        PathBuf::from("/Users/test/Library/Application Support/yt-dlp-tauri"),
    );
}

#[test]
fn platform_open_programs_are_native() {
    assert_eq!(open_path_program("windows"), "explorer");
    assert_eq!(open_path_program("macos"), "open");
    assert_eq!(open_path_program("linux"), "xdg-open");
}
```

- [ ] **Step 3: Run focused tests and confirm failure**

```bash
cargo test --manifest-path src-tauri/Cargo.toml macos_custom_discovery --lib
cargo test --manifest-path src-tauri/Cargo.toml macos_state_uses_application_support --lib
```

Expected: tests fail because current discovery accepts a target string and non-Windows state/open logic is Linux-specific.

- [ ] **Step 4: Refactor local discovery around executable names**

Pass `&ExecutableNames` from the selected platform definition instead of calling archive-only `tool_names_for_target`. Build the search list from inherited PATH, then `HOMEBREW_PREFIX/bin`, `/opt/homebrew/bin`, and `/usr/local/bin` on macOS. Deduplicate in first-seen order. Do not run `brew` in Custom mode; these directories are only executable candidates.

- [ ] **Step 5: Implement macOS Application Support and folder opening**

Use explicit OS branches in pure helpers and thin side-effect wrappers:

```rust
fn open_path(path: &Path) -> Result<(), String> {
    Command::new(open_path_program(env::consts::OS))
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(to_string)
}
```

`app_data_root_from` must check Windows `LOCALAPPDATA`, then macOS `~/Library/Application Support`, then XDG/fallback for other Unix systems. Keep the existing Downloads default.

- [ ] **Step 6: Run local, path, and complete Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml toolchain::local::tests --lib
cargo test --manifest-path src-tauri/Cargo.toml platform_open_programs_are_native --lib
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Expected: all pass.

- [ ] **Step 7: Commit macOS runtime paths**

```bash
git add src-tauri/src/toolchain/local.rs src-tauri/src/lib.rs
git commit -m "fix: use native macOS tool and data paths"
```

---

### Task 5: Render Homebrew/Custom behavior in the frontend

**Files:**

- Create: `src/platform-toolchain.ts`
- Create: `tests/platform-toolchain-ui.test.ts`
- Modify: `src/toolchain.ts:1-168`
- Modify: `tests/toolchain.test.ts:1-95`
- Modify: `src/main.ts:38-57, 79-421, 441-537, 789-1062, 1406-1593`
- Modify: `index.html:137-207`
- Modify: `tests/preview-html.test.ts:25-48`

**Interfaces:**

- Consumes: camelCase `AppState.platform` from Task 3 and `ManagedToolUpdateResult`.
- Produces: `PlatformPresentation`, `ManagedProviderKind`, and `ProviderCapabilities` TypeScript types.
- Produces: `executablePickerFilters(platform)`, `managedSummaryMode(platform)`, `managedActionConfirmationKey(platform, action)`, and `showsRevision(platform)` pure helpers.
- Produces: frontend `ManagedToolUpdateResult { tools, manifestJson, remoteRevision }`.
- Extends: `ToolStatus.availability` with `provider_missing`; `ToolSummaryMode` with `homebrew`.

- [ ] **Step 1: Write failing pure frontend tests**

Create `tests/platform-toolchain-ui.test.ts`:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import {
  executablePickerFilters,
  managedActionConfirmationKey,
  managedSummaryMode,
  showsRevision,
  type PlatformPresentation,
} from "../src/platform-toolchain.ts";

const macos: PlatformPresentation = {
  target: "macos-arm64",
  managedProvider: "homebrew",
  sourceLabels: { managed: "Homebrew", local: "Custom" },
  defaultSource: "managed",
  executableExtension: null,
  capabilities: { install: true, update: true, reinstall: true },
};

test("macOS uses extensionless pickers and Homebrew confirmations", () => {
  assert.deepEqual(executablePickerFilters(macos), []);
  assert.equal(managedSummaryMode(macos), "homebrew");
  assert.equal(showsRevision(macos), false);
  assert.equal(
    managedActionConfirmationKey(macos, "install"),
    "settings.homebrewInstallConfirm",
  );
});
```

Add a Windows fixture that expects `[{ name: "Executable", extensions: ["exe"] }]`, `managed` summary mode, visible revision, and no Homebrew confirmation key.

Extend `tests/toolchain.test.ts` with Homebrew cases:

```ts
test("missing Homebrew keeps Settings usable without offering a formula action", () => {
  assert.deepEqual(summarizeTools([tool("provider_missing")], "homebrew"), {
    ready: false,
    action: null,
    settingsKey: "settings.homebrewMissing",
    noticeKey: "notice.homebrewMissing",
    eventKey: "event.homebrewMissing",
    tone: "warning",
  });
});

test("Homebrew formulas map missing and outdated tools to install and update", () => {
  assert.equal(summarizeTools([tool("missing")], "homebrew").action, "install");
  assert.equal(summarizeTools([tool("outdated")], "homebrew").action, "update");
});
```

- [ ] **Step 2: Run frontend tests and confirm failure**

```bash
npm test -- --test-name-pattern='macOS uses extensionless|missing Homebrew|Homebrew formulas'
```

Expected: tests fail because the module, status, and summary mode do not exist.

- [ ] **Step 3: Implement pure provider UI helpers and summaries**

Define exact types in `src/platform-toolchain.ts` and keep provider decisions out of DOM-heavy `main.ts`:

```ts
export type ManagedProviderKind = "archive-manifest" | "homebrew";
export type ProviderCapabilities = { install: boolean; update: boolean; reinstall: boolean };
export type PlatformPresentation = {
  target: string;
  managedProvider: ManagedProviderKind;
  sourceLabels: { managed: string; local: string };
  defaultSource: "managed" | "local";
  executableExtension: string | null;
  capabilities: ProviderCapabilities;
};
```

Add Homebrew translation keys to `ToolSummary` unions and preserve every existing archive/local return value.

- [ ] **Step 4: Add provider presentation to AppState and Settings rendering**

Store `state.platform` from `applyAppState`. In `renderToolchainSource`:

- use `state.platform.sourceLabels` as the English fallback, but render translated `settings.homebrewTools`/`settings.customTools` keys in Chinese Homebrew mode and the existing translated keys in Windows mode;
- use Homebrew-specific translated hint text when managed provider is Homebrew;
- hide the revision row for Homebrew and show the provider prefix/root;
- gate update/reinstall visibility with capabilities;
- retain Custom path controls only when source is `local`.

Wrap the revision paragraph with `id="toolchain-revision-row"`, add an `id="managed-provider-guidance"` paragraph, and add a hidden `id="homebrew-help"` button labeled from `action.openHomebrew`. Register the button in `elements`, bind it to `openUrl("https://brew.sh/")`, and extend `preview-html.test.ts` to require all three stable elements.

- [ ] **Step 5: Route updates and package-changing actions**

Replace the frontend’s archive-only update sequence with the Task 3 command:

```ts
const result = await invoke<ManagedToolUpdateResult>("check_managed_tool_updates", {
  githubAccessMode: state.githubAccessMode,
});
const summary = applyToolSummary(
  result.tools,
  result.remoteRevision ? "remote" : managedSummaryMode(state.platform),
  { remoteRevision: result.remoteRevision },
);
state.pendingToolManifestJson = summary.action ? result.manifestJson : null;
```

Before Homebrew install/update/reinstall, require `window.confirm` using action-specific English/Chinese text that names the exact formulas `yt-dlp`, `ffmpeg`, and `deno`. Do not add confirmation to unchanged Windows archive actions.

In `applyToolSummary`, show `homebrew-help` only when any status has `availability === "provider_missing"`; set the guidance text to the translated missing-Brew explanation and let the button open the official `https://brew.sh/` URL. Hide the button for every other result. The absence of Brew must not throw from bootstrap.

- [ ] **Step 6: Remove `.exe` filtering only where appropriate**

Use:

```ts
const filters = executablePickerFilters(state.platform);
const selected = await open({
  multiple: false,
  directory: tool === "ffmpeg",
  ...(tool === "ffmpeg" || filters.length === 0 ? {} : { filters }),
});
```

Windows retains its `.exe` filter; macOS executable selections are extensionless.

- [ ] **Step 7: Run frontend tests and build**

```bash
npm test
npm run build
```

Expected: all tests pass and TypeScript reports no errors.

- [ ] **Step 8: Commit frontend provider behavior**

```bash
git add src/platform-toolchain.ts tests/platform-toolchain-ui.test.ts src/toolchain.ts tests/toolchain.test.ts src/main.ts index.html tests/preview-html.test.ts
git commit -m "feat: expose Homebrew toolchain controls"
```

---

### Task 6: Add macOS bundle and CI contracts

**Files:**

- Create: `src-tauri/tauri.macos.conf.json`
- Create: `tests/macos-support.test.ts`
- Modify: `.github/workflows/ci.yml:1-45`

**Interfaces:**

- Consumes: existing `src-tauri/icons/icon.icns` and bundled platform catalog.
- Produces: native macOS `.app` and `.dmg` targets when `npm run tauri build` runs on macOS.
- Produces: CI job `verify-macos` that builds a native debug `.app` and asserts its directory exists.

- [ ] **Step 1: Write failing configuration contract tests**

Create `tests/macos-support.test.ts`:

```ts
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const macConfig = JSON.parse(readFileSync("src-tauri/tauri.macos.conf.json", "utf8"));
const baseConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const ci = readFileSync(".github/workflows/ci.yml", "utf8");

test("macOS config emits app and dmg bundles with the icns icon", () => {
  assert.deepEqual(macConfig.bundle.targets, ["app", "dmg"]);
  assert.ok(macConfig.bundle.icon.includes("icons/icon.icns"));
  assert.ok(baseConfig.bundle.resources.includes("platform-toolchains.json"));
});

test("CI builds and verifies a native macOS app", () => {
  assert.match(ci, /verify-macos:[\s\S]*?runs-on: macos-15/u);
  assert.match(ci, /npm run tauri build -- --debug --bundles app/u);
  assert.match(ci, /target\/debug\/bundle\/macos\/yt-dlp-tauri\.app/u);
});
```

- [ ] **Step 2: Run the tests and confirm failure**

```bash
node --test --experimental-strip-types tests/macos-support.test.ts
```

Expected: failure because the platform-specific configuration and macOS CI job do not exist.

- [ ] **Step 3: Add the platform-specific Tauri configuration**

Create:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "bundle": {
    "targets": ["app", "dmg"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns"
    ]
  }
}
```

Do not add third-party binaries to resources. The base config keeps `targets: ["nsis"]`; Tauri’s automatic `tauri.macos.conf.json` merge replaces that target list only on macOS.

- [ ] **Step 4: Add the macOS CI job**

Use `macos-15`, Node 24, stable Rust, npm/Rust caches, then run:

```yaml
      - run: npm ci
      - run: npm test
      - run: npm run build
      - run: cargo test --manifest-path ./src-tauri/Cargo.toml --lib --bins --tests
      - run: cargo check --manifest-path ./src-tauri/Cargo.toml --all-targets
      - run: npm run tauri build -- --debug --bundles app
      - run: test -d src-tauri/target/debug/bundle/macos/yt-dlp-tauri.app
```

Keep the existing Linux `verify` job unchanged so cross-platform Rust compilation remains covered.

- [ ] **Step 5: Run the contract test and build a local app bundle**

```bash
node --test --experimental-strip-types tests/macos-support.test.ts
npm run tauri build -- --debug --bundles app
find src-tauri/target/debug/bundle/macos/yt-dlp-tauri.app -maxdepth 2 -type f | sort
```

Expected: test passes, Tauri reports a macOS application bundle, and the final command lists bundle contents.

- [ ] **Step 6: Build the local DMG**

```bash
npm run tauri build -- --debug --bundles dmg
find src-tauri/target/debug/bundle/dmg -maxdepth 1 -name '*.dmg' -print
```

Expected: exactly one unsigned debug DMG is listed.

- [ ] **Step 7: Verify the automatic macOS configuration merge**

```bash
rm -rf src-tauri/target/debug/bundle/macos src-tauri/target/debug/bundle/dmg
npm run tauri build -- --debug
test -d src-tauri/target/debug/bundle/macos/yt-dlp-tauri.app
find src-tauri/target/debug/bundle/dmg -maxdepth 1 -name '*.dmg' -print -quit | grep -q .
```

Expected: plain `tauri build` emits both macOS bundle types, proving `tauri.macos.conf.json` overrides the base NSIS target automatically.

- [ ] **Step 8: Commit packaging and CI**

```bash
git add src-tauri/tauri.macos.conf.json tests/macos-support.test.ts .github/workflows/ci.yml
git commit -m "ci: verify native macOS bundles"
```

---

### Task 7: Document macOS and Homebrew support

**Files:**

- Modify: `README.md:1-209, 234-245`
- Modify: `README_zh.md:1-209, 234-245`
- Modify: `PRODUCT.md:1-33`
- Modify: `src-tauri/Tools/README.md:1-11`
- Modify: `package.json:4-23`
- Modify: `CHANGELOG.md:1-8`

**Interfaces:**

- Consumes: final UI labels, paths, commands, and bundle behavior from Tasks 1-6.
- Produces: matching English/Chinese user instructions and accurate project metadata.

- [ ] **Step 1: Update English documentation and product metadata**

Document these exact operational facts:

```bash
brew install yt-dlp ffmpeg deno
npm ci
npm run tauri dev
npm run tauri build
```

Explain:

- macOS Apple Silicon and Intel runtime definitions are supported;
- Homebrew mode can install/update/reinstall formulas only when Homebrew already exists;
- if Brew is absent, install it from `https://brew.sh/` under the user’s control;
- Custom mode accepts absolute extensionless paths and also searches inherited PATH plus standard Brew prefixes;
- Finder-safe discovery covers `/opt/homebrew/bin` and `/usr/local/bin`;
- state/logs use `~/Library/Application Support/yt-dlp-tauri/`;
- bundles are in `src-tauri/target/release/bundle/macos/` and `src-tauri/target/release/bundle/dmg/`;
- local macOS bundles are unsigned/unnotarized and may require the user to explicitly allow opening them;
- project-hosted archive tools and published releases remain Windows-only.

Change `PRODUCT.md` users from Windows-only to Windows/macOS users, change the README badges/tech-stack tables, and add `macos` plus `homebrew` package keywords without removing `windows`.

- [ ] **Step 2: Update Chinese documentation with equivalent content**

Keep command names, formula names, URLs, and filesystem paths identical to the English documentation. Translate the explanations rather than shortening the supported behavior or security warning.

- [ ] **Step 3: Clarify the tool cache boundary**

Update `src-tauri/Tools/README.md` to state that `Tools/win-x64` is only for project-controlled Windows archives and macOS managed tools are external Homebrew installations, never copied into this directory.

- [ ] **Step 4: Add bilingual Unreleased changelog entries**

Under `## Unreleased`, add Chinese and English bullets covering Homebrew managed tools, Custom paths/Finder-safe discovery, native macOS path handling, `.app`/`.dmg` bundles, and macOS CI. Do not claim signing, notarization, or public macOS release publication.

- [ ] **Step 5: Run documentation-adjacent tests and frontend build**

```bash
npm test
npm run build
```

Expected: all tests and the production frontend build pass.

- [ ] **Step 6: Commit documentation**

```bash
git add README.md README_zh.md PRODUCT.md src-tauri/Tools/README.md package.json CHANGELOG.md
git commit -m "docs: explain macOS Homebrew setup"
```

---

## Final Verification

- [ ] **Step 1: Format and run proactive diagnostics**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Then run `lsp_diagnostics` on all modified Rust/TypeScript files and `lens_diagnostics` with `mode=all`. Expected: no errors or blocking diagnostics.

- [ ] **Step 2: Run the complete automated suite**

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib --bins --tests
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
```

Expected: every command exits 0.

- [ ] **Step 3: Verify the native bundles**

```bash
npm run tauri build -- --debug --bundles app,dmg
test -d src-tauri/target/debug/bundle/macos/yt-dlp-tauri.app
find src-tauri/target/debug/bundle/dmg -maxdepth 1 -name '*.dmg' -print -quit | grep -q .
```

Expected: both bundle assertions pass and no third-party tool binary exists inside the `.app` resources.

- [ ] **Step 4: Exercise Finder-like Homebrew discovery**

Launch the built `.app` through Finder or:

```bash
open src-tauri/target/debug/bundle/macos/yt-dlp-tauri.app
```

Verify Settings shows **Homebrew** and **Custom**, resolves `/opt/homebrew` on Apple Silicon (or `/usr/local` on Intel), displays all four executable paths, and passes Verify tools. Repeat Custom mode with one explicit executable path, then restore auto-detection.

- [ ] **Step 5: Exercise package-operation safety without unnecessary mutation**

Confirm Install/Update/Reinstall each shows the exact formula confirmation before running. Cancel each dialog and verify no Brew command runs. If a disposable Brew environment or genuinely missing formula is available, approve Install once and confirm progress plus post-install compatibility verification.

- [ ] **Step 6: Check repository hygiene and Windows contracts**

```bash
git status --short
git diff --check
node --test --experimental-strip-types tests/release-workflow.test.ts tests/windows-tool-restore-script.test.ts
```

Expected: `.pi/` remains untracked and unstaged, generated `dist/`/`target/` artifacts are ignored, no whitespace errors appear, and Windows release/restore contract tests pass.
