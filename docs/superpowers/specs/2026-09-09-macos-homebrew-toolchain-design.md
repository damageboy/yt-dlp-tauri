# macOS Homebrew Toolchain Design

## Summary

Add first-class macOS support without bundling macOS copies of yt-dlp, FFmpeg, FFprobe, or Deno. The existing managed-toolchain abstraction will select a platform-specific installation provider:

- Windows x64 continues to install verified project-controlled archives.
- macOS Apple Silicon and Intel use Homebrew formulas.
- Users on either platform can bypass the managed provider and select trusted custom tools.

The initial macOS deliverable is a functional local/CI `.app` and `.dmg`. Public signing and notarization are intentionally deferred because they require Apple distribution credentials.

## Goals

- Launch and initialize the Tauri app on macOS.
- Support both `aarch64-apple-darwin` and `x86_64-apple-darwin` runtime definitions.
- Install, verify, update, and reinstall the required macOS tools through Homebrew.
- Continue supporting absolute custom tool paths.
- Discover Homebrew reliably when the app is launched from Finder rather than a shell.
- Preserve existing Windows behavior, archive integrity, and release workflows.
- Produce native macOS `.app` and `.dmg` bundles from a Mac.
- Add automated macOS verification.

## Non-goals

- Publishing project-hosted macOS tool archives.
- Installing Homebrew silently or executing a remote bootstrap script without user control.
- Shipping a signed or notarized public macOS release in this change.
- Changing the Windows toolchain publication pipeline or archive manifest format.
- Supporting Linux tool installation in this change.

## Current State

The Rust backend and frontend compile on Apple Silicon macOS. However, the current app cannot complete startup or provide a usable macOS bundle because:

- Runtime target selection recognizes only `windows-x86_64` / `win-x64`.
- Local discovery searches only `.exe` names.
- The custom executable picker filters for `.exe` files.
- The default source is the Windows archive-managed toolchain.
- Opening a folder uses `xdg-open` on every non-Windows platform.
- State falls back to `~/.local/share`, rather than the normal macOS Application Support directory.
- Tauri bundling is configured only for NSIS, so a macOS build compiles an executable but emits no `.app` or `.dmg`.
- CI verifies Linux compilation but has no macOS runtime or bundle coverage.

The exact existing compatibility fixture was run successfully against Homebrew installations of `yt-dlp`, `ffmpeg`, `ffprobe`, and `deno` on Apple Silicon.

## Platform Toolchain Definitions

Introduce a bundled, schema-versioned platform definition file separate from `tools-manifest.json`. Keeping these concerns separate preserves the immutable archive and hash guarantees of the existing Windows manifest.

Each target definition declares:

- target identifier;
- supported OS and architecture;
- managed installation provider;
- user-facing source labels;
- default source;
- executable names;
- provider-specific package metadata and capabilities.

Conceptual shape:

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
      "defaultSource": "managed"
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
      "defaultSource": "managed"
    }
  ]
}
```

The Intel macOS entry has the same formulas and executable names with target `macos-x64` and architecture `x86_64`.

The persisted source values remain `managed` and `local` to avoid a Windows settings migration. The selected platform definition determines what `managed` means and how it is presented. On macOS, `managed` means Homebrew; `local` means Custom.

Unsupported OS/architecture pairs return an actionable platform error.

## Backend Components

### Platform definition loader

A focused Rust module will parse and validate the bundled platform definitions. Validation rejects duplicate targets, unknown provider kinds, empty package or executable names, unsupported default sources, and inconsistent OS/architecture mappings.

It exposes the selected target, provider capabilities, executable names, labels, and default source to command handlers. Platform conditionals should be concentrated here rather than repeated throughout `lib.rs`.

### Homebrew provider

A dedicated Homebrew module will own package-manager behavior:

1. Locate `brew` from the inherited `PATH`, `HOMEBREW_PREFIX`, `/opt/homebrew/bin/brew`, or `/usr/local/bin/brew`.
2. Run `brew --prefix` and resolve executables from the returned prefix.
3. Determine installed or outdated formulas using Brew commands with machine-readable output where available.
4. Install missing formulas with direct process arguments equivalent to:

   ```text
   brew install yt-dlp ffmpeg deno
   ```

5. Update outdated formulas with `brew upgrade` and reinstall through `brew reinstall` only after the corresponding user action.
6. Run the existing version probes and deterministic combined compatibility fixture after every package-changing operation.

Rust must invoke the resolved Brew executable directly with an argument array. It must not build a shell command from user input. Formula names come only from the validated bundled definition.

Brew output is captured and translated into the existing progress/status channel. Errors retain useful stderr details for locks, permissions, unavailable formulas, network failures, and process exit codes.

### Missing Homebrew

If Homebrew cannot be found, app startup still succeeds. Tool status explains that Homebrew is required and provides official installation guidance. The app must not silently execute Homebrew's remote installation script. A future, separately reviewed bootstrap action may open Terminal or the official Homebrew site, but it is not required for tool installation support in this change.

### Custom provider

Custom mode retains absolute paths for yt-dlp, an FFmpeg directory containing both FFmpeg and FFprobe, and Deno. macOS uses extensionless executable names and removes the `.exe` picker restriction.

PATH discovery will include the inherited directories plus standard Homebrew binary directories. Explicit paths continue to take precedence over discovered paths. All resolved custom tools pass the same compatibility probe as Homebrew and Windows tools.

### Application state and commands

`AppState` will include the selected platform target, provider kind, display labels, capabilities, and resolved provider root. The frontend uses these fields instead of inferring behavior from the browser user agent.

Existing command names may remain stable, but their implementation delegates by provider:

- check tools: archive manifest probe or Homebrew/custom probe;
- install tools: archive activation or `brew install`;
- check updates: remote archive manifest or `brew outdated`;
- update tools: archive activation or `brew upgrade`;
- reinstall tools: archive reinstall or `brew reinstall`.

Startup chooses the platform definition's default source when no source has been persisted. Existing Windows settings remain valid.

## Frontend Behavior

Settings remain structurally familiar while labels and actions reflect the selected platform definition:

- Windows shows **Managed** and **Local**.
- macOS shows **Homebrew** and **Custom**.
- Homebrew mode shows the Brew prefix and resolved executable paths.
- Custom mode shows selected or discovered absolute paths.
- Package-changing actions display a confirmation immediately before execution.
- Homebrew update/reinstall controls appear only when supported by the current provider state.
- A missing Brew installation produces an actionable status rather than preventing the rest of the app from loading.

The existing tool readiness gate remains: metadata parsing and downloads are disabled until all four executables pass the compatibility check.

## macOS Integration

- Use macOS `open` with the selected path as a direct argument, retain Explorer on Windows, and retain `xdg-open` for future Unix/Linux support.
- Store new macOS state and logs under:

  ```text
  ~/Library/Application Support/yt-dlp-tauri/
  ```

- Keep downloads under:

  ```text
  ~/Downloads/yt-dlp-tauri/
  ```

- Use existing process spawning on Unix. Cancellation sends `SIGTERM` to the active yt-dlp process; process-group improvements are out of scope unless tests reveal orphaned FFmpeg children.

## Packaging

Add `src-tauri/tauri.macos.conf.json`. Tauri automatically merges this platform-specific file with the base configuration on macOS.

The macOS override will:

- build `app` and `dmg` bundle targets;
- include `icons/icon.icns`;
- include the platform toolchain definition as a resource;
- avoid embedding third-party tool binaries.

The base/Windows configuration continues to target NSIS. The release workflow continues publishing Windows only. macOS bundles built locally or in CI are unsigned development artifacts and must be documented as such.

A native build produces the current host architecture. Universal or separately published architecture artifacts can be added later once release signing and notarization are designed.

## Security and Trust Boundaries

- Windows archives retain current URL, size, and SHA-256 verification.
- Homebrew owns formula acquisition, integrity, upgrades, and installation permissions.
- The app displays that Homebrew tools are external, shared system/user packages rather than app-owned revisions.
- Custom tools remain explicitly trusted user selections.
- Brew formula names are bundled constants validated at load time.
- No user-controlled string is interpolated into a shell command.
- Package-changing commands require an explicit UI action and confirmation.
- Video URLs and Cookie files are passed only to the resolved yt-dlp executable, preserving the existing warning about trusted tools.

## Testing

### Rust unit tests

- Map Windows x64, macOS Apple Silicon, and macOS Intel to the correct definitions.
- Reject unsupported OS/architecture pairs and malformed definitions.
- Verify executable names for each platform.
- Verify default source and provider labels.
- Verify Brew discovery precedence and deduplication.
- Verify formula-to-executable resolution, including FFmpeg providing FFprobe.
- Verify direct argument construction for install, upgrade, and reinstall operations.
- Verify no shell interpolation is used.
- Verify missing Brew and failed Brew commands produce actionable errors.
- Verify custom absolute paths override discovery.
- Keep the deterministic combined tool compatibility tests.
- Verify macOS Application Support and `open` command selection with pure platform helpers.

### Frontend tests

- Render Managed/Local labels for Windows and Homebrew/Custom labels for macOS.
- Hide or show provider actions based on capabilities.
- Require confirmation for package-changing actions.
- Render missing-Brew guidance without blocking settings or app initialization.
- Remove executable-extension filtering on macOS while preserving Windows filtering.

### CI and build verification

Add a macOS job that runs:

```bash
npm ci
npm test
npm run build
cargo test --manifest-path ./src-tauri/Cargo.toml --lib --bins --tests
cargo check --manifest-path ./src-tauri/Cargo.toml --all-targets
npm run tauri build -- --debug --bundles app
```

The job verifies a native `.app` is emitted. DMG creation remains covered by local release-candidate verification because it adds little runtime coverage and can be slower in CI.

Windows release and toolchain workflows remain unchanged.

## Documentation

Update English and Chinese project documentation to cover:

- macOS prerequisites and supported architectures;
- `brew install yt-dlp ffmpeg deno`;
- Homebrew versus Custom source behavior;
- Finder-safe Brew discovery;
- local `.app` and `.dmg` build commands and output paths;
- state/log locations;
- unsigned and non-notarized bundle limitations;
- Windows-only scope of project-hosted managed archives and release publishing.

Update product copy and package keywords so the app is no longer described as Windows-only.

## Acceptance Criteria

- The app starts on Apple Silicon and Intel macOS definitions without a target error.
- A Finder-like minimal `PATH` still discovers standard Apple Silicon or Intel Homebrew installations.
- The app can install required formulas through an existing Homebrew installation after user confirmation.
- Homebrew and Custom toolchains both pass the exact compatibility fixture before downloads are enabled.
- Custom extensionless executable selection works on macOS.
- Opening the download folder works through macOS `open`.
- `npm run tauri build` on macOS emits native `.app` and `.dmg` bundles.
- macOS state uses Application Support.
- Existing Windows frontend/Rust tests and NSIS release behavior remain green.
- CI includes a passing macOS `.app` bundle build.
- No third-party macOS tool binaries are committed or bundled.
