# aria2c RPC progress design

## Scope and approach

Add a small backend RPC subsystem beside existing yt-dlp stdout reporting. yt-dlp continues to launch aria2c, choose protocols/formats, retry downloads, merge/remux and report the final output path. The app observes aria2c over HTTP JSON-RPC and requests graceful shutdown when its queue is exhausted.

Use HTTP polling with the existing blocking `reqwest` dependency. WebSockets add connection state while continuous byte/speed updates still require polling. Parsing aria2c's console output loses the structured status and queue lifecycle needed here. Neither alternative is planned.

## Session creation and arguments

For each application download with aria2c enabled:

1. Generate an ephemeral cryptographically random token. Use a small direct OS-random dependency if needed; never derive a token from timestamps or process IDs.
2. Select an available non-privileged loopback port. 16800 is the example port, not a persisted setting. Port availability checks cannot reserve a port through aria2c startup; explicitly handle bind conflicts and authentication failure rather than attach to an unrelated server.
3. Extend the existing `aria2c:-j N -x N -s N` argument string with `--enable-rpc=true --rpc-listen-all=false --rpc-listen-port=<port> --rpc-secret=<token>` and `--stop-with-process=<app-pid>` for app-exit cleanup.
4. Start the monitor with the yt-dlp operation; discard its token/session state at the end. Disabled downloads allocate no RPC resources and retain their current argument vector.

Bind only to loopback. The backend client uses `127.0.0.1`, disables proxies and redirects, sets a one-second request timeout and limits response size. Include `token:<token>` in every RPC call. Do not log tokens, authenticated request bodies, raw command arguments, cookies or media URLs from RPC responses. RPC credentials do not go to the frontend or saved settings.

## Monitor lifecycle

Use a 250 ms target polling interval in the existing download worker, interleaved with child-exit checks. Each cycle has a two-second network deadline and reads at most one waiting/stopped page per queue, with rotating offsets. No separate monitor thread is needed. Stop progress routing and join stdout/stderr readers before clearing `DownloadProcessState` or emitting final completion/failure.

The lifecycle is:

```text
Awaiting aria2c → Connected(sessionId) → Queue exhausted → Shutdown requested
       ↑                                                        |
       └──────────── next aria2c invocation / retry ──────────────┘

Any state → yt-dlp exit / cancellation → monitor stopped and joined
```

- RPC absence before the first connection is expected during extraction or native fallback. Do not apply a ten-second timer starting at yt-dlp launch: extraction can take longer and aria2c might never be selected.
- Authenticate and obtain `aria2.getSessionInfo` before accepting data. A new session ID resets transfer IDs, completed counters and shutdown state, even if no connection-refused interval was observed.
- yt-dlp may invoke separate aria2c processes for video, audio and retries. Continue monitoring until the parent operation ends, reconnecting across these boundaries.
- After a connected session, transient RPC failures retry. Persistent failure for ten seconds while that session has not been intentionally shut down ends the operation with a specific progress-connection error and cleans up the process tree. Do not allow a lost RPC client to leave an RPC-enabled child waiting forever.
- After intentional shutdown, wait for the next session without treating normal extraction/native/postprocessing gaps as RPC failures. Repeated successful contact with the same session for ten seconds after shutdown is a shutdown failure, not a new transfer.
- Startup/bind/authentication errors must preserve the yt-dlp/aria2c error outcome. Never send shutdown to an unauthenticated or foreign session.

## Progress projection and source ownership

Read `aria2.getGlobalStat`, `aria2.tellActive`, and paginated waiting/stopped records. Request only the keys required: session/GID identity, status, total/completed byte counts, speed and error information. RPC numeric fields are decimal strings; validate them before arithmetic.

- For a single transfer with a known positive total: percentage = `100 * completedLength / totalLength`; speed comes from byte/second data; ETA = remaining bytes / nonzero speed.
- For a multi-file/fragment batch: track records by GID within the session and include queued and completed work. Sum bytes only when the complete batch's totals are known. Do not calculate a batch percentage using active transfers alone.
- Waiting fragments often have unknown sizes. Show indeterminate percentage with available speed until a trustworthy denominator exists. Do not invent totals or silently present fragment count as byte percentage.
- Handle stopped-result eviction: aria2c retains only 1000 results by default. Compare observed records with `numStoppedTotal`; missing history invalidates byte aggregation, not shutdown detection. Do not increase retention without bounds.
- Percentage describes the current aria2c invocation, not the complete video-plus-audio-plus-merge job. Reset when the session changes.
- Convert values to existing `DownloadProgress { percent, status, speed, eta, raw }`; `raw` is `None`. No event-schema or frontend redesign is required.
- While an authenticated aria2c transfer is active, RPC owns transfer progress. Keep the yt-dlp reader running for output paths and lifecycle data, and resume its progress as the aria2c session ends. Enabling aria2c in settings alone must never suppress genuine native-download progress.
- An aria2c completed transfer does not mean the application download is complete. Only the successful yt-dlp exit produces final `Completed`. Existing postprocessing behavior stays as-is for this change.

## Queue completion, failure and cleanup

Do not shut down on the first GID completion or on an initially empty active list.

Shutdown eligibility requires an authenticated session, evidence that work was queued/processed (`numStoppedTotal > 0`), and `numActive == 0 && numWaiting == 0`, confirmed again immediately before shutdown. This covers fast transfers completed before the first poll. Paused/queued items are not complete. Failed/stopped items count as terminal for process cleanup, not success.

Call `aria2.shutdown` for that session, stop publishing its progress, then let yt-dlp receive the child exit code and perform its normal retry/skip/error policy. Keep error diagnostics available; never convert an RPC shutdown response of `OK` into download success. A local HTTP-404 experiment confirmed RPC `status=error`, aria2c exit code 3 and yt-dlp exit code 1 after graceful shutdown; fragmented retry behavior still needs integration coverage.

Cancellation continues to terminate the existing process group/tree and overrides concurrent success. Stop the operation-scoped monitor on cancellation and every return path. RPC failure cleanup must use process-tree termination without incorrectly setting the user's cancellation flag; preserve the monitor error as the failure cause. Do not hold the process-state lock across RPC requests or thread joins.

## Files and boundaries

| File | Planned responsibility |
| --- | --- |
| `src-tauri/src/aria2c_rpc.rs` (new) | Ephemeral configuration, HTTP RPC client, session state, progress projection, terminal-queue decisions and unit tests. |
| `src-tauri/src/aria2c.rs` | Append runtime RPC options to existing enabled downloader arguments; preserve disabled arguments and persisted settings. |
| `src-tauri/src/lib.rs` | Own monitor lifetime, route progress sources and integrate cleanup with existing download/cancellation paths. |
| `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` | Declare OS randomness dependency if required; reuse existing HTTP/JSON libraries. |
| `src-tauri/tests/fixtures/aria2c_rpc_probe.rs` (new) | Compiled fake process/RPC server for deterministic cross-platform lifecycle tests. |
| `src-tauri/src/test_support.rs` | Build new fixture using existing isolated temporary-directory pattern. |
| `docs/aria2c.md`, nearest directory `AGENTS.md` files | Record RPC behavior, lifecycle, tests and real-platform verification limits. |

Existing unrelated working-tree edits must remain untouched.

## Evidence

- [aria2 RPC options and API](https://aria2.github.io/manual/en/html/aria2c.html#rpc-options)
- [yt-dlp external downloader lifecycle](https://github.com/yt-dlp/yt-dlp/blob/2026.08.19/yt_dlp/downloader/external.py)
- Session experiments: native vs aria2c structured output; authenticated live RPC through injected yt-dlp arguments; successful shutdown; failed HTTP download preserving nonzero exits. Tests used temporary local HTTP servers, not the Tauri UI.

Implementation refinement: the RPC client lives inside `RpcMonitor`; deterministic RPC fixtures run as local Rust test servers, and real-tool tests use `rpc_media_server.py`. `windows_process_tree.rs` enumerates descendants after parent exit. These replace the planned separate monitor thread and compiled RPC-server fixture.
