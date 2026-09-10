# Optional aria2c downloads

Settings offers an off-by-default aria2c downloader, executable selection/automatic discovery, status refresh, and integer parallelism from 1 to 16. aria2c is user-managed; macOS installation is `brew install aria2`. The application never installs or upgrades it with required tools.

## State and validation

The existing application data directory contains `state/aria2c.json`:

```json
{
  "schemaVersion": 1,
  "enabled": false,
  "executablePath": null,
  "parallelConnections": 16
}
```

Rust rejects unsupported schemas, unknown fields, relative selected paths and invalid numbers. Missing state uses defaults; malformed state uses disabled defaults and reports its load error. A successful save writes a same-directory temporary file, syncs it, and atomically replaces the destination before updating memory. Failed validation or replacement preserves the previous saved state.

The frontend keeps a draft. Selecting a file, Use PATH and Refresh inspect without saving. Failed saves retain edits, unrelated AppState refreshes preserve dirty drafts, and old inspection responses cannot overwrite newer drafts. English and Chinese copy describe the same controls.

## Discovery

1. Explicit absolute path, when set. Its failure is authoritative.
2. Executable candidates in inherited PATH; relative PATH entries resolve against the working directory.
3. macOS: the existing Homebrew resolver's prefix plus `bin/aria2c`.
4. macOS: `/opt/homebrew/bin/aria2c`.
5. macOS: `/usr/local/bin/aria2c`.

Windows selects `aria2c.exe`; Unix selects `aria2c`. Automatic discovery skips non-executable files. Keep symlink invocation names intact because yt-dlp identifies the external downloader by basename. The version probe must succeed and identify aria2. Both version probing and Homebrew prefix queries have ten-second deadlines; timeout terminates and reaps the probe.

Startup and general AppState retrieval do not probe aria2c. Explicit Settings inspection, enabled Save and enabled download do. Disabled saves and downloads skip inspection entirely.

## Download lifecycle

The backend snapshots saved configuration when a download starts. It validates enabled aria2c before spawning yt-dlp, adding four discrete arguments before the URL:

```text
--downloader
<absolute aria2c executable>
--downloader-args
aria2c:-j N -x N -s N
```

Arguments never pass through a shell. No raw argument field is provided. The same N sets concurrent items (`-j`), connections per server per item (`-x`) and splits (`-s`); this is not a total connection cap or an application-level video queue. yt-dlp can print its defaults before the overriding arguments; the generated user overrides still apply.

Metadata extraction never reads aria2c settings. Disabled downloads retain their previous arguments. Protocol eligibility remains with yt-dlp, so enabling aria2c does not prove every selected format uses it. Existing indeterminate progress remains available when numeric updates are absent.

Unix downloads own a process group. Cancellation signals TERM, waits up to two seconds, then uses KILL if members remain. Windows retains `taskkill /T /F`. Cancellation and spawn registration share a mutex, as do cancellation and the final completion decision. The group stays registered during cancellation cleanup.

## Verification

Run:

```sh
bun run test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml --lib --bins --tests
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Rust tests use temporary native executables without requiring aria2c. They cover version validation, Homebrew timeout, path discovery, save/reload/recovery, command construction, early/late cancellation, and TERM-resistant child cleanup on Unix. Frontend tests cover invalid inputs, failed saves, dirty drafts and reversed inspection responses.

Native acceptance uses an isolated app state directory and a controlled HTTP range server. Compare the downloaded file's checksum with the source for N=1 and N=16, capture the actual aria2c invocation, cancel an active transfer and check that both processes exit, then start another transfer. Also verify saving, restarting, missing-executable recovery, both languages and Finder-style PATH discovery. A native Windows run is required to claim Windows process behavior verified.

### macOS results (2026-09-10)

- Automated: 205 frontend tests and 106 Rust tests pass (103 library, 2 binary, 1 integration). Production build, format check, all-target check and Clippy with warnings denied pass.
- Installed tools: yt-dlp 2026.08.19, aria2 1.37.0, FFmpeg 9.0.1, Deno 2.9.6.
- Native Settings: default disabled, automatic discovery, selected executable containing spaces, invalid 17 rejected, enabled missing-executable save rejected, disabled recovery saved, and restart persistence verified. English and Chinese layouts inspected. Reversed inspection responses and dirty refresh behavior verified with frontend tests.
- Native local transfers: N=1 and N=16 both match source SHA-256 `7b699e002934e1487a6ba1821ff8eb6901dd2ba9fd8b1b0e664c7efa005ef340`. Captured invocation contains the respective `-j/-x/-s` overrides. N=16 produced 16 HTTP Range requests; this observation is not a guaranteed connection count.
- Native cancellation: active aria2c exited after cancellation; subsequent download completed with matching checksum. The Rust process fixture also verifies termination of a child that ignores TERM and cancellation near completion.
- YouTube CLI smoke: `jNQXAC9IVRw`, formats 395+251 (HTTPS), direct installed aria2c at N=1; both transfers completed with aria2c progress output and FFmpeg merged them. FFprobe confirms AV1 video, Opus audio and 19.021-second duration. An earlier logging-wrapper run exited with -9; its cause was not established, so only the direct-executable retry is counted as successful YouTube evidence.
- Discovery precedence and custom Homebrew prefix are covered by injected native tests. Native Settings found the installed Homebrew symlink. A GUI launch with a demonstrably minimal PATH remains unverified: the automation launcher retained PATH despite the test bundle environment override.
- Windows-native discovery and process-tree cancellation remain an outstanding acceptance gate; no Windows run was available. macOS results do not establish Windows runtime behavior.

All native UI tests used a disposable application state directory and local media; existing user settings were preserved.
