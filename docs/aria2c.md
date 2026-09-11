# Required aria2c tool with optional download usage

Settings → Toolchain contains the off-by-default Use aria2c toggle and integer parallelism from 1 to 16. Tool installation, paths and verification share the existing Toolchain controls. aria2c is always required. Homebrew manages the `aria2` formula through the same install, version verification, update and reinstall actions as other required tools. Custom setups and current Windows archives require a configured or PATH executable. The usage switch never changes installation requirements.

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

The frontend keeps a draft for usage and parallelism until Save. Selecting a custom executable saves immediately; the shared Use PATH resets executable overrides. Failed saves retain edits, and unrelated AppState refreshes preserve dirty drafts. English and Chinese copy describe the same controls.

## Discovery

Managed Homebrew mode always uses its resolved prefix plus `bin/aria2c`; legacy custom overrides cannot affect managed operation. Custom toolchains and Windows use the following discovery order:

1. Explicit absolute path, when set. Its failure is authoritative.
2. Executable candidates in inherited PATH; relative PATH entries resolve against the working directory.
3. macOS: the existing Homebrew resolver's prefix plus `bin/aria2c`.
4. macOS: `/opt/homebrew/bin/aria2c`.
5. macOS: `/usr/local/bin/aria2c`.

Windows selects `aria2c.exe`; Unix selects `aria2c`. Automatic discovery skips non-executable files. Keep symlink invocation names intact because yt-dlp identifies the external downloader by basename. The version probe must succeed and identify aria2. Both version probing and Homebrew prefix queries have ten-second deadlines; timeout terminates and reaps the probe.

AppState retrieval does not probe aria2c. Startup tool verification, Verify tools, enabled Save, metadata preflight and every download verify aria2c, regardless of the usage switch. Disabled saves remain possible to repair settings; saving refreshes tool readiness, which stays incomplete until the executable is available.

## Download lifecycle

The backend snapshots saved configuration when a download starts. It requires a working aria2c before spawning yt-dlp. Only enabled usage adds four discrete arguments before the URL:

```text
--downloader
<absolute aria2c executable>
--downloader-args
aria2c:-j N -x N -s N --enable-rpc=true --rpc-listen-all=false --rpc-listen-port=PORT --rpc-secret=TOKEN --stop-with-process=APP_PID
```

Arguments never pass through a shell. No raw argument field is provided. The same N sets concurrent items (`-j`), connections per server per item (`-x`) and splits (`-s`); this is not a total connection cap or an application-level video queue. yt-dlp can print its defaults before the overriding arguments; the generated user overrides still apply.

Metadata extraction verifies required aria2c availability without adding downloader arguments. Disabled downloads retain their previous arguments. Protocol eligibility remains with yt-dlp, so enabling aria2c does not prove every selected format uses it. Existing indeterminate progress remains available when numeric updates are absent.

Unix downloads own a process group. Cancellation signals TERM, waits up to two seconds, then uses KILL if members remain. Windows enumerates descendants and uses `taskkill /T /F`, including when the parent has already exited. Cancellation and spawn registration share a mutex, as do cancellation and the final completion decision. The group stays registered during cancellation cleanup.

## Verification

Run:

```sh
bun run test
bun run build
cargo test --manifest-path src-tauri/Cargo.toml --lib --bins --tests
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Rust tests use temporary native executables without requiring aria2c. They cover version validation, Homebrew timeout, path discovery, save/reload/recovery, command construction, early/late cancellation, and TERM-resistant child cleanup on Unix. Frontend tests cover invalid inputs, failed saves, dirty drafts.

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

### Required-tool correction (2026-09-11)

The required-tool rule supersedes the original optional-installation design. Homebrew missing-formula installation and update mapping have regression tests. The native Settings check shows aria2c in the required tool list with usage off. Missing configured executables block readiness regardless of usage; repairing and saving settings refreshes readiness. Windows archive delivery is a separate pending distribution decision; this change requires a configured executable there without claiming the existing archives contain it.

Correction verification: 208 frontend tests and 109 Rust tests (106 library, 2 binary, 1 integration) pass; build, formatting and Clippy with warnings denied pass. Native Homebrew Settings verifies all five executables with usage off. An unavailable configured aria2c blocks readiness, and Use PATH + Save restores readiness without enabling usage. No package installation is suggested for a configuration-only failure. Independent review passed after that remediation fix.

### Toolchain UI consolidation (2026-09-11)

Removed the standalone downloader section and duplicate executable/status controls. Usage and parallelism remain under Toolchain. Custom executable selection saves only the path, preserving unsaved usage options; the shared Use PATH resets it. Managed Homebrew uses its own aria2c regardless of legacy custom overrides.

Verification: 208 frontend tests, 110 Rust tests (107 library, 2 binary, 1 integration), production build, formatting and Clippy pass. Independent review passed. Rebuilt and reopened the release app; native Settings confirms the consolidated controls and all five required executables available with usage off.

### RPC progress (2026-09-11)

Enabled downloads use an available loopback port and fresh 256-bit OS-random token per application download. The Rust download worker polls aria2c HTTP JSON-RPC at a target interval of 250 ms. Requests bypass proxies and redirects, have one-second timeouts and 1 MiB response limits; each polling cycle has a two-second network deadline. Waiting/stopped queue pages rotate across cycles so large fragment queues cannot monopolize the worker. Tokens are not saved, sent to the UI or included in captured stderr diagnostics.

The existing progress event/UI receives percentage, speed and ETA from RPC while aria2c is active. The yt-dlp parser remains active for native fallback and final output paths. Unknown sizes or incomplete fragment history produce indeterminate percentage with available speed. A percentage describes the current aria2c invocation, not the combined video/audio/postprocessing job.

RPC-enabled aria2c stays alive after finishing. The monitor authenticates each session, waits for terminal work and empty active/waiting queues, rechecks, then requests graceful shutdown. It reconnects for later audio/video transfers and retries. yt-dlp's final exit remains the authority for operation success. Merge/postprocessing reporting is unchanged.

A connected RPC session gets a ten-second retry window; timeouts do not become an unlimited wait after shutdown. A closed listener is expected between subprocesses. Cancellation blocks further progress updates and terminates the process tree. RPC failure and nonzero parent exit clean descendants before output-reader joins, avoiding hangs from inherited pipes.

Automated coverage includes progress arithmetic, unknown totals, malformed/oversized replies, authentication, secret redaction, stopped-history eviction, bounded queue pagination, session changes, shutdown timeouts, slow extraction and parent-exit cleanup. Windows descendant selection is tested with a synthetic process graph; native Windows cleanup remains unverified.

Run the optional real-tool integration test with yt-dlp, aria2c, FFmpeg, ffprobe and python3 on PATH:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --lib real_rpc_download_lifecycle -- --ignored --nocapture
```

The test generates disposable local media and exercises native downloading, aria2c HTTP, separate video/audio plus merge (checked with ffprobe), HLS, native fallback with aria2c enabled, unknown-length HTTP, HTTP 404, broken fragments, fragment retry recovery, cancellation and fast completion. It checks process/listener cleanup. These are backend integration tests, not a native UI test or a Windows runtime test.
