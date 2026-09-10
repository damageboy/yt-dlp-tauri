# aria2c downloader requirements

> Installation and disabled-operation requirements are superseded by `aria2c-required-tool` following the user correction on 2026-09-11.

## ADDED Requirements

### Requirement: Optional persistent configuration

The application SHALL default to `{"schemaVersion":1,"enabled":false,"executablePath":null,"parallelConnections":16}` in `state/aria2c.json`. It SHALL reject unsupported schema versions, unknown fields, relative configured paths, and non-integral/out-of-range parallelism. A malformed file SHALL produce disabled defaults and a visible load error.

#### Scenario: Disabled recovery
- **WHEN** a saved executable disappears and the user saves with aria2c disabled
- **THEN** the disabled configuration persists without executing aria2c, and ordinary downloads remain available.

#### Scenario: Failed save
- **WHEN** validation, executable verification, or the pre-commit file write fails
- **THEN** the previous file and in-memory configuration remain unchanged.

### Requirement: Executable discovery and verification

The application SHALL resolve an explicit path first, then inherited PATH, then on macOS the existing resolved Homebrew prefix's `bin/aria2c`, then `/opt/homebrew/bin/aria2c` and `/usr/local/bin/aria2c`. Explicit-path failure SHALL be authoritative. A discovered path SHALL remain an absolute invocation path without resolving away the `aria2c` symlink basename. Unix uses `aria2c`; Windows uses `aria2c.exe`.

#### Scenario: Finder launch with nonstandard Homebrew prefix
- **WHEN** PATH contains no aria2c and the existing Homebrew resolver finds a custom prefix
- **THEN** discovery checks that prefix before standard Homebrew paths.

#### Scenario: Hung executable
- **WHEN** an enabled Save or download encounters a version probe that exceeds ten seconds
- **THEN** the probe is terminated and reaped, an actionable error appears, and yt-dlp is not started.

### Requirement: Explicit downloader selection

The application SHALL append four discrete arguments to enabled video downloads: `--downloader`, the resolved absolute executable, `--downloader-args`, and `aria2c:-j N -x N -s N`. The backend SHALL generate arguments from validated settings without a shell. The UI SHALL call the setting Parallelism and explain its three-option mapping rather than claiming a total connection limit.

#### Scenario: Disabled or metadata-only operation
- **WHEN** aria2c is disabled, or the app only parses metadata
- **THEN** no aria2c resolution or probe occurs for that operation and no downloader arguments are added.

#### Scenario: Protocol uses another downloader
- **WHEN** yt-dlp selects a protocol ineligible for aria2c
- **THEN** yt-dlp retains downloader selection; the app does not claim aria2c was used solely because it was enabled.

### Requirement: Settings draft and status

Settings SHALL expose Use aria2c, Choose executable, Use PATH, Parallelism, Refresh status, and Save. Choose and Use PATH SHALL edit the draft only. Explicit status inspection SHALL not persist changes. Save SHALL validate and persist; failed saves SHALL retain the draft and display an error. All new labels and stable errors SHALL be localized in English and Chinese. Settings SHALL avoid probing during general startup/AppState refresh.

#### Scenario: Late inspection response
- **WHEN** the user changes the draft while an older status inspection is pending
- **THEN** the older response cannot replace status for the newer draft.

### Requirement: Download lifecycle

Downloads SHALL snapshot saved configuration at start, preserve existing output/merge/cookie behavior, and retain indeterminate progress when percentages are unavailable. Completion SHALL follow successful process exit and the existing output-path event.

#### Scenario: Cancellation
- **WHEN** the user cancels an enabled download
- **THEN** Windows terminates the yt-dlp tree and Unix terminates the dedicated yt-dlp process group, including aria2c; the UI retains its cancellation result.
