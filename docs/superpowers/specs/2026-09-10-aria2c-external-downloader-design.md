# aria2c External Downloader Design

**Status:** Planning complete; implementation not started

**Implementation:** [OpenSpec checklist](../../../openspec/changes/aria2c-external-downloader/tasks.md)

**Date:** 2026-09-10

## Summary

Add optional aria2c integration for video downloads. Users can enable aria2c in
Settings, choose an absolute executable or let the app discover it, and set one
bounded parallelism value. When enabled and validated, the backend passes
aria2c to yt-dlp as an external downloader using direct process arguments.

aria2c remains a user-managed optional dependency. The app does not download,
bundle, update, or execute it through a shell.

## Goals

- Keep aria2c disabled by default.
- Discover a user-installed aria2c from a configured path or supported search
  locations.
- Let users select an executable, return to PATH discovery, enable or disable the
  integration, and set parallelism from 1 through 16.
- Validate aria2c before an enabled configuration is saved and immediately before a
  download starts.
- Apply aria2c only to the existing video-download command; metadata parsing remains
  unchanged.
- Preserve the exact current yt-dlp command when aria2c is disabled.
- Cancel yt-dlp and its aria2c child together on every supported platform.
- Keep executable paths, video URLs, and downloader options out of shell strings.

## Non-goals

- Bundling, installing, updating, or redistributing aria2c.
- Adding aria2c to the managed yt-dlp/FFmpeg/Deno toolchain.
- Accepting arbitrary aria2c or yt-dlp downloader arguments from the UI.
- Adding an application-level queue for downloading multiple videos concurrently.
- Changing metadata extraction, preview loading, cookie selection, output naming,
  FFmpeg behavior, or Deno behavior.
- Guaranteeing that every site or yt-dlp protocol uses aria2c; yt-dlp retains control
  over downloader eligibility and fallback behavior.

## User Experience

Settings gains an **aria2c external downloader** section with:

- **Use aria2c** toggle, off by default;
- executable source/status showing the configured or discovered absolute path;
- **Choose executable** action using the existing native file-picker pattern;
- **Use PATH** action, which clears the configured path and reruns discovery;
- **Parallelism** whole-number input with default `16`, minimum `1`,
  maximum `16`, and step `1`;
- **Refresh status** action for explicit inspection;
- explicit **Save** action.

The section distinguishes these states:

- disabled or not yet checked;
- available from a configured path;
- available from PATH or a Finder-safe macOS location;
- not found;
- configured executable invalid or no longer available;
- executable found but version probe failed.

Choosing an executable stores its absolute path in the draft form. **Use PATH** sets
the draft path to `null`; it does not store whichever discovery result happened to
win. Saving with **Use aria2c** enabled succeeds only if resolution and the version
probe succeed. A failed enabled save leaves the previously persisted configuration
unchanged and shows an actionable error. Saving with the toggle disabled must remain
possible even if a previously configured executable has disappeared.

The Parallelism setting maps one N to `-j N -x N -s N`. It is not a total
connection cap: `-j` limits concurrent items, while `-x` and `-s` apply within each
item. It does not control how many top-level videos the app downloads at once.
The persisted key remains `parallelConnections`.

All new labels, statuses, validation messages, and errors are provided in the
existing English and Chinese UI variants.

## Persisted Configuration

Store the backend-owned configuration at `aria2c.json` in the existing application
state directory. Use a strict, versioned schema:

```json
{
  "schemaVersion": 1,
  "enabled": false,
  "executablePath": null,
  "parallelConnections": 16
}
```

The Rust model is conceptually:

```rust
struct Aria2cConfig {
    schema_version: u32,
    enabled: bool,
    executable_path: Option<PathBuf>,
    parallel_connections: u8,
}
```

Contracts:

- `schemaVersion` must equal `1`.
- Unknown fields and unsupported schema versions are rejected.
- `executablePath`, when present, must be absolute.
- `parallelConnections` must be in `1..=16`.
- Missing configuration produces the defaults shown above.
- Malformed configuration must never enable aria2c implicitly. The backend exposes
  the load error and uses disabled safe defaults until the user saves a valid value.
- Save writes and syncs a same-directory temporary file, reuses the existing
  platform atomic-replacement primitive, then updates memory under a serialized
  commit lock. Pre-commit failures preserve the prior valid file and memory.

`AppState` exposes saved configuration from managed `Aria2cState`; inspection
returns computed status without persisting it.
The frontend reads and updates it through focused Tauri commands rather than reading
or writing the JSON file directly.

## Resolution and Validation

Resolution is backend-owned and deterministic.

### Executable names

- Windows: `aria2c.exe`
- macOS and other Unix builds: `aria2c`

### Resolution order

1. `executablePath` when configured;
2. inherited process `PATH`;
3. on macOS only, `<resolved Homebrew prefix>/bin/aria2c`, using the existing Homebrew resolver;
4. on macOS only, `/opt/homebrew/bin/aria2c`;
5. on macOS only, `/usr/local/bin/aria2c`.

Resolve Homebrew lazily after PATH fails. Bound its `brew --prefix` query to ten
seconds. Failure to locate Homebrew does not prevent checking standard fallback
paths. Preserve absolute symlink invocation paths; do not canonicalize away the
`aria2c`/`aria2c.exe` basename yt-dlp uses for downloader identification.

The explicit path is authoritative: when configured, the backend must report its
failure rather than silently switching to a different PATH installation. Finder-safe
Homebrew candidates are fallback search locations because GUI applications may not
inherit the user's shell PATH.

Resolution returns a computed status containing the selected source, absolute path,
availability, parsed version text when available, and an actionable error when
unavailable. The computed status is not persisted.

### Probe

Validate the resolved executable by invoking it directly with:

```text
<absolute-aria2c-path> --version
```

A valid probe requires the process to start, exit successfully, and identify aria2
in its version output. Reuse the existing ten-second version-probe deadline; kill
and reap a timed-out child. Capture a bounded,
user-displayable version summary from stdout or stderr using existing log/error
sanitization conventions.

Probe at these boundaries:

1. explicit Settings inspection (Refresh, Choose executable, or Use PATH);
2. before committing a save that enables aria2c;
3. immediately before constructing/spawning an enabled video download.

General AppState refresh and startup return saved settings without probing.
Disabled Save and disabled downloads never invoke discovery or the version probe.
Before an inspection, display Not checked rather than Not found. An inspection
response for an older draft must not overwrite the current draft status.

The pre-download probe handles executables that are moved, removed, or replaced after
Settings was saved. Failure stops before yt-dlp starts and tells the user to disable
aria2c, select a valid executable, or restore it to PATH.

## Backend Interfaces

Place aria2c configuration, persistence, discovery, probing, and yt-dlp argument
construction in a focused Rust module instead of expanding `lib.rs` with unrelated
helpers. `lib.rs` remains responsible for Tauri command registration, application
state wiring, and the download orchestration boundary.

Expose backend operations equivalent to:

- get persisted configuration plus current resolution status;
- save a complete configuration draft transactionally;
- select an aria2c executable through the existing native picker mechanism;
- resolve and probe the active executable before a download;
- construct the optional yt-dlp argument fragment from validated state.

The implementation plan defines `get_aria2c_settings`, `inspect_aria2c_config`, and
`save_aria2c_config` with `Aria2cConfig`, `Aria2cStatus`, and `Aria2cSettings` payloads.

## yt-dlp Command Construction

When aria2c is disabled, add no downloader-related arguments. Existing command
construction and argument ordering remain unchanged.

When enabled, resolve and probe aria2c, then add these four discrete arguments before
the video URL:

```text
--downloader
<resolved-absolute-aria2c-path>
--downloader-args
aria2c:-j N -x N -s N
```

For example, `N = 16` corresponds to the shell-display form:

```text
yt-dlp --downloader /absolute/path/to/aria2c \
  --downloader-args "aria2c:-j 16 -x 16 -s 16" URL
```

The backend passes each yt-dlp argument through the process API. It does not build or
execute this display form as a shell command. The downloader-argument key remains
`aria2c` even when `--downloader` receives an absolute executable path.

The one UI value intentionally maps to all three aria2c controls:

- `-j N`: concurrent downloads within aria2c;
- `-x N`: connections to one server;
- `-s N`: split count.

No raw argument field is exposed.

## Download and Cancellation Flow

The enabled video-download flow is:

1. snapshot the saved aria2c configuration from `AppState`;
2. resolve the executable using the configured precedence;
3. run `aria2c --version`;
4. add the validated downloader argument fragment to the existing yt-dlp command;
5. spawn yt-dlp and stream progress through the current mechanism;
6. on cancellation, terminate the process tree that contains yt-dlp and aria2c.

Windows retains the existing `taskkill /T` process-tree behavior. On Unix, yt-dlp is
started in its own process group and cancellation targets that group rather than only
the yt-dlp PID. Wait up to two seconds after TERM, then escalate to KILL if group
members remain, retaining group identity until cleanup completes. Preserve existing
cancellation result/error semantics while ensuring an aria2c child is not left running.

Metadata parsing never reads this configuration and never receives aria2c arguments.
Snapshot saved configuration at download command entry so later settings edits cannot
change an already-starting download. Preserve indeterminate progress when yt-dlp does
not emit numeric updates for its external downloader; do not infer actual aria2c use
from the enabled setting. Verify actual subprocess use with the installed yt-dlp
version and selected protocol.

## Error Handling

| Condition | Result |
| --- | --- |
| Disabled download or Save | Preserve current behavior; do not resolve or probe aria2c |
| Enabled save with no executable | Reject save and show discovery guidance |
| Configured path is relative | Reject configuration |
| Parallel value outside `1..=16` or non-integral | Reject in UI and backend |
| Configured executable missing | Do not fall back to PATH; reject enabled save/download |
| `--version` cannot start or exits nonzero | Reject enabled save/download with probe error |
| Executable disappears after save | Fail before spawning yt-dlp |
| Disabled save with stale path | Persist disabled state so user can recover |
| Malformed/unsupported JSON | Use disabled safe defaults and expose load error |
| yt-dlp rejects aria2c for a URL/protocol | Surface the existing download-process error |
| Cancellation requested | Terminate yt-dlp and aria2c process tree/group |

## Security and Trust Boundaries

- aria2c is an external executable trusted by the user, like a custom tool path.
- The backend accepts only an absolute configured path or a deterministic discovered
  path.
- Version probing and downloading invoke executable paths directly with argument
  arrays.
- URLs, executable paths, and downloader arguments are never interpolated into a
  shell command.
- The app generates the complete aria2c option string from the validated integer; the
  user cannot inject arbitrary command-line text.
- Error and version output follows existing bounded logging and sanitization rules.

## Testing Strategy

### Rust unit tests

- Default config is schema version `1`, disabled, path `null`, and parallelism value
  `16`.
- Valid JSON round-trips; unknown fields, unsupported versions, relative paths,
  non-integral values, and values outside `1..=16` fail.
- Missing config returns defaults; malformed config produces disabled safe state and
  a visible load error.
- Explicit paths take precedence over PATH and do not fall back after failure.
- PATH takes precedence over resolved and standard Homebrew directories.
- A nonstandard resolved Homebrew prefix precedes standard Homebrew locations.
- After the resolved Homebrew prefix, fallback order is `/opt/homebrew/bin` then `/usr/local/bin`.
- Platform executable names are `aria2c.exe` on Windows and `aria2c` on Unix.
- Probe invokes the absolute path with exactly `--version` and classifies launch,
  exit-status, and output results.
- Disabled configuration emits no yt-dlp arguments.
- Enabled `N = 16` emits exactly:

  ```text
  --downloader
  <absolute path>
  --downloader-args
  aria2c:-j 16 -x 16 -s 16
  ```

- Enabled downloads fail before yt-dlp spawn when resolution or probing fails.
- Metadata commands never receive aria2c arguments.
- Windows cancellation retains process-tree termination.
- Unix spawning creates a dedicated process group and cancellation targets it.

Use pure discovery/probe/process abstractions or test doubles so unit tests do not
require aria2c to be installed.

### Frontend tests

- Settings renders the aria2c section and explicit Save action.
- Toggle defaults off.
- Numeric input renders `min="1"`, `max="16"`, and `step="1"` and blocks invalid
  draft values.
- Choose executable and Use PATH update the draft source/status correctly.
- Saving an enabled unavailable executable displays the backend error without
  claiming success.
- Disabled state remains saveable after an executable becomes unavailable.
- English and Chinese strings exist for every new label and status.
- Controls have associated labels, keyboard-accessible actions, and live status/error
  announcements consistent with the current Settings UI.

### Integration and regression verification

Run the existing frontend and Rust suites plus targeted new tests. Verify manually or
with a controlled fixture executable that:

- disabled downloads produce the pre-change yt-dlp argument list;
- enabled downloads pass the absolute executable and generated args;
- cancellation does not leave the fixture downloader running;
- Finder-safe macOS discovery works under a minimal PATH;
- Windows discovery uses `aria2c.exe`.

## Documentation

Update:

- `README.md` with the optional dependency, setup, setting meanings, and command
  mapping;
- `README_zh.md` with equivalent guidance;
- the nearest directory `AGENTS.md` records for every changed or added source, test,
  and documentation file;
- `docs/aria2c.md` with the `aria2c.json` schema, resolution order,
  validation boundaries, and process-group cancellation behavior.

Documentation must clearly distinguish aria2c connection parallelism for one current
video from application-level parallel video downloads.

## Acceptance Criteria

- A fresh installation has aria2c disabled and downloads exactly as before.
- Settings can choose an absolute aria2c executable or return to discovery mode.
- Discovery follows configured path, PATH, and macOS Homebrew fallback precedence.
- Enabling cannot be saved until the resolved executable passes `--version`.
- Parallelism accepts only whole numbers from `1` through `16` and defaults
  to `16`.
- Enabled video downloads pass the resolved absolute path and generated
  `aria2c:-j N -x N -s N` arguments to yt-dlp.
- Metadata parsing never invokes or configures aria2c.
- A missing or invalid enabled executable fails before yt-dlp starts.
- Cancelling an enabled download terminates both yt-dlp and aria2c.
- No shell command is introduced for probing, downloading, or cancellation argument
  construction.
- English and Chinese Settings UI and README documentation describe the feature.
- Existing frontend tests, Rust tests, builds, disabled download behavior, and
  Windows process-tree cancellation remain green.

## Planning verification refinements

- Canonical requirements and executable checklist live in `openspec/changes/aria2c-external-downloader/`.
- Unit tests must use native absolute fixture paths and actually register the new Rust module before the first red test.
- Use behavioral command/process tests; source-string assertions alone cannot prove downloader selection or cancellation.
- Reuse existing picker filters and indeterminate progress rendering. No aria2c RPC subsystem is required.
- Native macOS acceptance covers controlled HTTP Range transfers, actual aria2c argv, final checksum, and YouTube format/protocol selection.
- Windows cancellation requires native Windows execution; report the gate outstanding when a host is unavailable.
