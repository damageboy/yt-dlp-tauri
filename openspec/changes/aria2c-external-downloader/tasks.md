# aria2c External Downloader Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement this plan task by task. Steps use checkbox syntax for tracking. Use subagents only if separately authorized.

**Goal:** Add optional aria2c downloads with executable discovery, persistent enablement, and configurable parallelism.

**Architecture:** A focused Rust aria2c module owns configuration and resolution. Tauri owns saved state and snapshots it for downloads. Settings edits a draft and explicitly saves. yt-dlp continues to own transfer protocol selection and FFmpeg postprocessing.

**Tech Stack:** Rust 2021, Tauri 2, serde, vanilla TypeScript, Vite, Node test runner; existing platform process and Windows filesystem support.

**Spec:** [requirements](specs/aria2c-downloader/spec.md), [detailed design](../../../docs/superpowers/specs/2026-09-10-aria2c-external-downloader-design.md).

## Execution status (2026-09-10)

- [x] Configuration, strict validation and exact downloader arguments implemented.
- [x] Discovery and bounded inspection implemented; custom-prefix precedence tested.
- [x] Atomic persistence and Tauri commands implemented; recovery and concurrent saves tested.
- [x] Download integration and process-tree cancellation implemented; independent review findings fixed and retested.
- [x] Bilingual Settings implemented; native save, validation, recovery and restart checks passed.
- [x] Documentation, automated checks, native HTTP N=1/N=16 transfers, cancellation/retry and direct aria2c YouTube merge verified.
- [ ] Verify a native GUI launch with demonstrably minimal PATH.
- [ ] Run Windows-native discovery and cancellation acceptance.

Detailed results and limitations: [verification record](../../../docs/aria2c.md#macos-results-2026-09-10).

The detailed steps below preserve the original proposed sequence. Their unchecked boxes are historical planning detail, not the current task status. Implementation uses a per-fixture mode file instead of environment variables, a shared native fixture helper, `Aria2cSettingsDraft` for UI state, and an extracted production command builder. Checkpoint commits are consolidated after final verification. Metadata isolation follows separate command paths and was also observed during native parsing.

## Global constraints

- Off by default. No automatic aria2c installation, managed-toolchain manifest change, raw arguments, concurrent-video queue, or new production dependency.
- Persist schema 1, `enabled: false`, `executablePath: null`, `parallelConnections: 16`; accept integer 1–16. UI label is **Parallelism**. The JSON key stays `parallelConnections` for consistency with the original design.
- One N maps to `-j N -x N -s N`. It is not a total connection cap. Do not promise a speedup or aria2c use for every protocol.
- Explicit executable > PATH > macOS resolved Homebrew prefix > Apple Silicon standard path > Intel standard path. A failed explicit selection never falls back.
- Probe only at explicit Settings inspection, enabled Save, and enabled download. Startup, metadata, disabled saves, and disabled downloads perform no aria2c probe.
- Use the existing ten-second version-probe deadline; kill and reap on timeout. Homebrew prefix discovery must also have a ten-second deadline.
- No shell interpolation. Preserve absolute symlink invocation paths named `aria2c`/`aria2c.exe`; do not canonicalize them to versioned Cellar names.
- Existing metadata and disabled download argument vectors remain unchanged.
- Preserve pre-existing changes in `src/toolchain.ts`, `tests/preview-html.test.ts`, and `tests/toolchain.test.ts`, plus `.pi/` and root `AGENTS.md`. Capture status/diff before implementation; stage only owned changes.
- Follow root-to-leaf documentation lookup before source reads/edits. `kb` and OpenSpec CLI were unavailable during planning; use directory records and manually maintained OpenSpec files unless availability changes.
- Run tests before each implementation step, verify that intended assertions fail, then implement and rerun. A test command selecting zero tests is not a successful red/green check.

## Current-code evidence

- `src-tauri/src/lib.rs`: `download_video` constructs yt-dlp directly; `parse_metadata` is separate; `kill_process_tree` currently targets only the positive PID on Unix.
- `src-tauri/src/toolchain/homebrew.rs`: `locate_homebrew` already understands PATH, HOMEBREW_PREFIX, standard locations, and `brew --prefix`. Reuse it instead of another brew locator.
- `src-tauri/src/toolchain/probe.rs`: `probe_executable` already implements ten-second version probes and Windows hidden-process behavior. Reuse its timeout path.
- `src-tauri/src/toolchain/activation.rs`: `atomic_replace` handles Unix rename and Windows replacement; reuse that primitive rather than copying Windows APIs.
- `src/main.ts`: `updateDownloadProgress` already removes the progress value for an unknown percentage. Preserve this behavior; no RPC progress subsystem is needed.
- `package.json`: `test` runs Node's test runner, not Bun's native runner. Use `bun run test` or `npm test`.

## File map

| File | Planned responsibility |
| --- | --- |
| `src-tauri/src/aria2c.rs` (new) | Config, saved state, inspection, discovery, argument generation; inline behavioral tests |
| `src-tauri/src/lib.rs` | Register module/state/commands; snapshot saved settings; download args and process groups |
| `src-tauri/src/toolchain/mod.rs` | Crate-private re-exports of reusable probe and atomic replacement helpers |
| `src-tauri/src/toolchain/probe.rs` | Expose bounded runner to sibling Homebrew module; preserve existing probe behavior |
| `src-tauri/src/toolchain/homebrew.rs` | Bound `brew --prefix`; retain install/update behavior |
| `src-tauri/src/toolchain/activation.rs` | Make atomic replacement crate-visible; use neutral state-file error wording |
| `src/aria2c-settings.ts` (new) | Wire types, numeric validation, draft update helpers |
| `src/main.ts`, `index.html`, `src/styles.css` | Settings, commands, bilingual copy, status and busy state |
| `tests/aria2c-settings.test.ts` (new) | Numeric/draft behavior, UI and translation contracts |
| `src-tauri/tests/fixtures/aria2c_probe.rs` (new) | Compiled process fixture for version, timeout, argv capture and child lifecycle tests |
| `README.md`, `README_zh.md` | Optional installation and accurate setting semantics |
| `docs/aria2c.md` (new) | State, discovery, lifecycle, and reproducible manual checks |

Add/update per-file rows in the nearest directory `AGENTS.md` for every implementation file touched. Create missing directory records; root remains doctrine only. No implementation file listed above is created by this planning task.

## Task 1 — Configuration and argument contract

**Consumes:** serde, native `PathBuf`/`OsString`.
**Produces:** `Aria2cConfig`, `Aria2cStatus`, `Aria2cSource`, configuration parser and argument builder in `aria2c.rs`.

- [ ] Add `mod aria2c;` to `lib.rs` before running the first new tests so Rust discovers them. Add tests with native absolute paths built from `std::env::temp_dir()`, not hard-coded Unix paths in Windows tests.

```rust
#[test]
fn defaults_and_invalid_ranges() {
    let config = Aria2cConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.parallel_connections, 16);
    assert_eq!(config.executable_path, None);
    for n in [0, 17] {
        let json = format!(r#"{{"schemaVersion":1,"enabled":false,"executablePath":null,"parallelConnections":{n}}}"#);
        assert!(parse_aria2c_config(&json).is_err());
    }
}
```

- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml aria2c::tests --lib`; confirm missing definitions cause the intended failure.
- [ ] Implement strict camelCase serde configuration with `deny_unknown_fields`, validation and defaults:

```rust
pub struct Aria2cConfig {
    pub schema_version: u32,
    pub enabled: bool,
    pub executable_path: Option<PathBuf>,
    pub parallel_connections: u8,
}
pub enum Aria2cSource { Configured, Path, HomebrewPrefix, HomebrewAppleSilicon, HomebrewIntel }
pub struct Aria2cStatus {
    pub source: Option<Aria2cSource>,
    pub executable_path: Option<PathBuf>,
    pub available: bool,
    pub version: Option<String>,
    pub error_code: Option<String>,
    pub error: Option<String>,
}
```

`Aria2cStatus` serializes camelCase; sources serialize kebab-case. Codes are `not-found`, `invalid-path`, `probe-failed`, `timeout`; UI translates codes and may display sanitized native details. No persisted status.

- [ ] Add tests for schema 2, unknown fields, missing required fields, fractional and negative N, relative executable paths, valid round-trip, N=1 and N=16, disabled empty args, and paths containing spaces. A missing optional `executablePath` is equivalent to null.
- [ ] Implement `parse_aria2c_config(&str) -> Result<Aria2cConfig, String>` and `aria2c_downloader_args(&Aria2cConfig, &Aria2cStatus) -> Result<Vec<OsString>, String>`. Validate config; return empty when disabled; require available status/path when enabled. The enabled result is exactly:

```rust
vec![
    OsString::from("--downloader"),
    path.as_os_str().to_owned(),
    OsString::from("--downloader-args"),
    OsString::from(format!("aria2c:-j {n} -x {n} -s {n}")),
]
```

- [ ] Rerun module tests; verify named tests executed. Update records and checkpoint only Task 1 files after review.

## Task 2 — Discovery and bounded inspection

**Consumes:** Task 1 config/status; existing Homebrew resolver and probe helpers.
**Produces:** `inspect_aria2c(&Aria2cConfig) -> Result<Aria2cStatus, String>`.

- [ ] Add failing discovery tests with injected PATH directories, optional Homebrew prefix, and executable predicate. Native path fixtures cover configured precedence, missing explicit path without fallback, PATH precedence, relative PATH entries made absolute, non-adjacent duplicate removal, custom Homebrew prefix, standard fallback order, and Windows `.exe` naming.
- [ ] Run the aria2c module tests and Homebrew tests; establish the new failures without altering global PATH in parallel tests.
- [ ] Implement lazy discovery. If explicit path exists, validate that selection only. Search PATH first; call Homebrew only on macOS after PATH fails. Use `crate::current_platform_definition()` and existing `locate_homebrew` for the prefix, then standard fallback locations if the prefix cannot be resolved. Do not require Homebrew for explicit or PATH selections. Use order-preserving full deduplication, not `Vec::dedup_by` which removes only adjacent duplicates.
- [ ] Re-export `probe_executable` crate-privately from `toolchain/mod.rs`. Inspect the resolved path via `probe_executable("aria2c", path)`. Keep its ten-second timeout, direct argument invocation, and hidden Windows process behavior. Validate that the executable basename identifies aria2c for yt-dlp and that successful version output identifies aria2; surface a useful error for another selected program. Extend the shared probe summary extraction to select the first nonempty stdout line with stderr fallback, covering that change with a regression test. In the aria2c adapter sanitize the summary and cap it to 240 characters. Existing tool availability classifications remain unchanged.
- [ ] Make `run_bounded_probe_with_timeout` visible to sibling modules. In `SystemProcessRunner::run`, use it with ten seconds specifically for `--prefix`; retain existing long-running brew install/update execution. Add a failing timeout test first. This prevents aria2c discovery from hanging inside brew even though its own version probe is bounded.
- [ ] Create the Rust fixture source with modes selected by a per-command environment variable (never mutate the test process environment): normal version, nonzero exit, empty output, sleep beyond deadline, and argv capture. Compile it in a unique temporary directory as `aria2c` or `aria2c.exe`; use RAII cleanup and no real tool dependency. The timeout case uses a short injected deadline in unit tests, then one production-deadline smoke check. Verify timed-out children are reaped.
- [ ] Check disabled download/save paths never call the inspector using an injected closure that panics if invoked. Explicit Refresh may inspect while the toggle is off.
- [ ] Run aria2c, Homebrew and probe tests; confirm existing Homebrew installation and local-toolchain behavior still pass. Update file records and checkpoint Task 2.

## Task 3 — Persistence and Tauri commands

**Consumes:** validated config and inspector.
**Produces:** cloneable `Aria2cState`, `Aria2cSettings`, and three commands.

```rust
pub struct Aria2cSettings {
    pub config: Aria2cConfig,
    pub status: Aria2cStatus,
    pub load_error: Option<String>,
}
```

Serialize camelCase. `Aria2cState::load(PathBuf) -> Self` reads once; `settings() -> Result<Aria2cSettings, String>` returns saved data without probing; `save(Aria2cConfig) -> Result<Aria2cSettings, String>` validates and persists; `snapshot_config() -> Result<Aria2cConfig, String>` clones saved config. Before inspection, status has `available: false` with null version/error/source/path; UI labels this Not checked, not Not found.

- [ ] Write failing temp-directory tests: missing file defaults; malformed file disabled with load error; reload; enabled invalid executable save rejected; disabled stale executable save succeeds without probe; invalid settings retain file and memory; write/replace failure retains prior state; concurrent saves leave memory equal to disk.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml aria2c::tests --lib` and verify failures.
- [ ] Implement schema loading and transactional save. Validate/probe before committing. Serialize pretty JSON plus newline to a unique same-directory temporary file, flush/sync, replace destination, then update in-memory config. Serialize the commit section under the state mutex so two saves cannot invert file/memory ordering. Do not hold the mutex during the external probe. On successful disabled save clear load error and use Not checked status.
- [ ] Reuse `activation.rs::atomic_replace` through a crate-private re-export. Make error wording refer to a state file, retaining existing platform behavior. Remove temporary files on pre-commit failure; never delete the valid destination first. A directory-sync failure after rename is not reported as a rollback-safe failure.
- [ ] Register managed state during existing Tauri setup. Extend `AppState` with `aria2c: Aria2cSettings`; AppState assembly does no executable inspection. Register:

```text
get_aria2c_settings() -> Aria2cSettings
inspect_aria2c_config({ config: Aria2cConfig }) -> Aria2cStatus
save_aria2c_config({ config: Aria2cConfig }) -> Aria2cSettings
```

Rust commands take `tauri::State<'_, Aria2cState>` and return `Result<response, String>`. Run inspection/save on `spawn_blocking`; inspection never persists. Initialization failure yields disabled settings with a load error rather than preventing normal app startup.

- [ ] Test real serialized response shapes and state behavior rather than self-matching Rust source strings. Run aria2c and activation tests, `cargo check --manifest-path src-tauri/Cargo.toml --all-targets`, update records, checkpoint Task 3.

## Task 4 — Download integration and cancellation

**Consumes:** saved snapshot, inspector, generated argument vector.
**Produces:** enabled download selection and reliable child cancellation.

- [ ] Capture a baseline disabled download argv before editing. Add failing behavioral tests for enabled args before URL, disabled exact argv, no aria2c during metadata, and failed enabled validation before yt-dlp spawn. Use the compiled fixture to capture real process arguments, including a tool directory containing spaces.
- [ ] Snapshot `Aria2cConfig` at download command entry before `spawn_blocking`. Do not reread settings after tool discovery. When enabled inspect the snapshot and generate args; otherwise skip discovery and use an empty vector. Insert `.args(aria2c_args)` immediately before existing `.arg(&request.url)`.
- [ ] Add `download_process_command(program: impl AsRef<OsStr>) -> Command` using existing `background_command` and Unix `CommandExt::process_group(0)`. Use it only for the video process. Preserve Windows creation flags.

```rust
fn download_process_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = background_command(program);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
}
```

- [ ] Add failing cancellation tests before changing `kill_process_tree`. Extend the compiled fixture with parent/child modes: parent spawns a child and reports both PIDs; child writes a heartbeat. Test through the production spawn/cancel helper, not just argument text. Wait for child readiness before cancelling and use bounded exit polling with a cleanup guard. Allow for transient zombie reaping when checking PIDs.
- [ ] Keep Windows `taskkill /PID PID /T /F`. On macOS use direct `/bin/kill -TERM -- -PGID`; verify negative-PGID parsing on the native host. Retain PGID until cleanup completes. Wait up to two seconds for group exit; if members remain send KILL to the same group and reap the owned parent. Test a child that ignores TERM. Never signal zero or the application's process group. Report failure if a child remains; do not clear lifecycle state before cleanup is settled.
- [ ] Preserve existing cancellation result and completion semantics, including a cancellation request arriving near process exit. Test that cancellation cannot become a false successful download and that stream-reader joins cannot hang on a surviving child.
- [ ] Reuse the existing indeterminate progress UI. Do not invent percentages or infer actual aria2c use from the toggle. Start with neutral Downloading status; forward existing valid progress events and output-path marker; only finish at successful process completion. Verify error and cancel state leave the UI usable for another download.
- [ ] Run full Rust tests and format/check. Update records and checkpoint Task 4 only after the real process-tree tests pass on macOS; require native Windows verification in Task 6.

## Task 5 — Settings UI

**Consumes:** AppState aria2c data and three Tauri commands.
**Produces:** bilingual persistent settings with isolated drafts.

- [ ] Define matching TypeScript `Aria2cConfig`, `Aria2cStatus`, `Aria2cSettings`, and source union in `src/aria2c-settings.ts`. `executablePath` is `string | null`; status optional values are explicitly nullable. `parallelConnections` is a number validated as an integer.
- [ ] Add failing Node tests for numeric parsing and draft immutability:

```typescript
test("parallelism is an integer from 1 to 16", () => {
  assert.equal(parseParallelConnections("1"), 1);
  assert.equal(parseParallelConnections("16"), 16);
  for (const input of ["", "0", "17", "1.5", "1e1", "NaN"]) {
    assert.equal(parseParallelConnections(input), null);
  }
});
```

- [ ] Run `node --test --experimental-strip-types tests/aria2c-settings.test.ts`; confirm intended failure, then implement:

```typescript
export function parseParallelConnections(value: string): number | null {
  return /^(?:[1-9]|1[0-6])$/u.test(value.trim()) ? Number(value) : null;
}
```

Use object spreading for Choose/Use PATH draft updates. Reuse existing executable picker filters from `src/platform-toolchain.ts`; do not duplicate that helper.

- [ ] Add markup/translation checks matching the existing Node suite and interaction checks for Save/inspection. Render controls with IDs `aria2c-enabled`, `aria2c-path`, `choose-aria2c`, `use-path-aria2c`, `aria2c-parallel`, `refresh-aria2c`, `save-aria2c`, `aria2c-status`. Number input has `min="1" max="16" step="1"`; associate labels and use `role="status" aria-live="polite"`.
- [ ] Integrate draft state in `main.ts`; initialize from AppState once and explicitly refresh when opening Settings. Do not overwrite a dirty draft during unrelated AppState refresh. Choose/Use PATH inspect the new draft but do not save. Refresh inspects current draft. Any edit increments an inspection generation counter; discard responses whose counter is stale. Save serializes the validated complete draft and waits for backend success before updating saved state or showing success.
- [ ] Keep disabled recovery save reachable even with a stale path. Invalid numeric input sets `aria-invalid` and blocks invocation. Disable controls during Save or existing global busy operations. Inspection failure preserves the draft. Changing settings during an active download does not affect that download's snapshot.
- [ ] Provide equivalent English/Chinese labels: Use aria2c, Executable, Choose, Use PATH, Refresh status, Parallelism, Save; statuses Not checked, Disabled, Available, Not found, Invalid executable, Probe failed, Timed out, Load failed, Saved. Helper text explicitly explains N maps to all three controls and does not download N videos at once or cap total connections at N. Show sanitized native error details after localized context.
- [ ] Add focused responsive styling with wrapping paths and keyboard-visible focus. Preserve the existing unknown-percent rendering.
- [ ] Verify interaction outcomes with the running UI: failed save keeps draft; two reversed inspection responses cannot change the current status; reopening/restarting retains saved settings; keyboard access and English/Chinese layout work. Source assertions alone do not prove these outcomes.
- [ ] Run `bun run test` and `bun run build`, update records, checkpoint Task 5.

## Task 6 — Documentation and native acceptance

**Consumes:** passing implementation from Tasks 1–5.
**Produces:** setup documentation and evidence of actual supported-platform behavior.

- [ ] Update both READMEs and `docs/aria2c.md`: `brew install aria2`, Windows user-selected aria2c.exe, discovery precedence, setting meanings, default-off behavior, schema, recovery, supported-protocol limitation and cancellation. State that install/update/reinstall of required tools does not manage aria2c.
- [ ] Run final automated checks once after the last change:

```bash
bun run test
bun run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib --bins --tests
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
git diff --check
```

Classify existing warnings separately from new failures; do not refactor unrelated code merely to clear baseline warnings. Use actual available diagnostics; no dependency on unconfigured `lsp_diagnostics` or `lens_diagnostics` tools.

- [ ] On macOS, test with actual installed yt-dlp/aria2c versions and a controlled local HTTP range server serving a generated media file. Capture the real aria2c invocation and HTTP Range activity for N=1 and N=16. Verify the final file checksum. The fixture must support concurrent Range requests and serve a sufficiently large file; no benchmark threshold or exact connection-count claim.
- [ ] Download a short permitted YouTube example with video/audio merging. Record selected format/protocol, yt-dlp/aria2c versions, whether aria2c actually ran, final media validation, and observed progress. If yt-dlp selects a native fragment downloader, record that; a successful download alone does not validate external downloader use.
- [ ] Cancel during an actual aria2c transfer; verify yt-dlp and aria2c exit and a subsequent download succeeds. Repeat using the TERM-ignoring fixture. Test removal after enabled Save and disabled recovery.
- [ ] Test Finder-style minimal PATH and a nonstandard Homebrew prefix without modifying system installations. Use injected environment/process fixtures for precedence and a real Finder launch for the installed setup. Test a symlink path and a path containing spaces.
- [ ] Run Windows-native executable discovery, save/reload, argv and tree-cancellation tests on a Windows host/CI. Cross-target unit assertions on macOS are not proof of Windows process behavior. If unavailable, report this gate outstanding; do not claim complete cross-platform verification.
- [ ] Run project code-review/code-quality checks before feature commits/merge, with an independent review if authorized. Recheck preserved pre-existing diffs. Stage only owned files. Do not merge or publish as part of planning.

## Acceptance evidence to report

| Requirement | Evidence |
| --- | --- |
| Default-off compatibility | Before/after disabled argv equality; no aria2c probe |
| Correct settings | Restart/save/recovery tests plus visible Settings verification |
| Correct external execution | Actual aria2c argv for N=1/16, path with spaces |
| Finder-safe discovery | Minimal-PATH and custom-prefix results |
| Successful transfer | Controlled HTTP fixture checksum and merged video validation |
| Honest progress | Observed numeric events or working indeterminate state |
| Complete cancellation | Parent/child exit, TERM escalation fixture, subsequent download |
| Windows compatibility | Native Windows test result; otherwise explicit outstanding gate |

## Sources checked during planning

- [aria2 option semantics](https://aria2.github.io/manual/en/html/aria2c.html#options): `-j` limits concurrent items, `-x` limits connections per server per item, `-s` controls splitting.
- [yt-dlp aria2c implementation](https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/downloader/external.py): executable handling, generated defaults, protocol eligibility, output behavior. Recheck the installed version during native testing.
- [Homebrew aria2 formula](https://formulae.brew.sh/formula/aria2): package name differs from executable name.
