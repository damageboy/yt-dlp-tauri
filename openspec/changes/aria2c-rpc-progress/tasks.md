# aria2c RPC Progress Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement this plan task-by-task after design approval. Steps use checkbox syntax for tracking.

**Goal:** Show live aria2c progress through its local RPC API while retaining existing yt-dlp progress and completion behavior.

**Architecture:** yt-dlp owns downloading and postprocessing. A Rust HTTP RPC monitor observes each aria2c session, projects progress into the current event type and shuts down exhausted sessions so yt-dlp can continue.

**Tech Stack:** Rust, Tauri 2, blocking reqwest, serde/serde_json, aria2c JSON-RPC; existing TypeScript UI.

**Spec:** [design.md](design.md). Approved by user and implemented; native Windows and direct native-UI verification remain outstanding.

## Global constraints

- Focus only on aria2c download progress. Preserve yt-dlp parsing; no merge/postprocessing redesign.
- Preserve disabled arguments, persistent configuration, selected executable, `-j/-x/-s`, cookies and format/container options.
- Loopback-only authenticated RPC; no proxy, redirect, token persistence or token logging.
- Treat RPC completion as transfer completion; yt-dlp exit remains final operation authority.
- Handle repeated aria2c sessions and native fallback in one yt-dlp invocation.
- Preserve unrelated working-tree changes. Read root-to-leaf documentation before edits and update file records afterwards.
- Use an automatic available port instead of literal 16800.
- Use failing tests before implementation. Commit only task-owned changes after review; do not stage unrelated edits in shared files.

## Task 1: RPC transport and truthful progress projection

**Files:** New `src-tauri/src/aria2c_rpc.rs`; module declaration in `src-tauri/src/lib.rs`; randomness dependency in `src-tauri/Cargo.toml`/lockfile if needed; nearest directory records.

**Interfaces:** Plan `RpcConfig` for private port/token/client settings; `RpcClient` for authenticated method calls; `RpcSnapshot` for validated global and per-GID values; `SessionTracker` for per-session progress/history; `QueueCounts { active: u64, waiting: u64, stopped_total: u64 }` with `ready_to_shutdown(&self) -> bool`; `byte_percent(completed: u64, total: Option<u64>) -> Option<f64>` for pure percentage calculation. Keep all production visibility `pub(crate)` or narrower.

- [x] Write focused failing tests for known/unknown byte totals, zero speed, numeric strings, malformed/error/oversized RPC replies, GID deduplication, incomplete fragment totals and stopped-history eviction.

```rust
#[test]
fn rpc_percentage_requires_known_total() {
    assert_eq!(byte_percent(25, Some(100)), Some(25.0));
    assert_eq!(byte_percent(25, None), None);
    assert_eq!(byte_percent(0, Some(0)), None);
}

#[test]
fn rpc_shutdown_waits_for_entire_queue() {
    assert!(!QueueCounts { active: 0, waiting: 0, stopped_total: 0 }.ready_to_shutdown());
    assert!(!QueueCounts { active: 0, waiting: 1, stopped_total: 1 }.ready_to_shutdown());
    assert!(QueueCounts { active: 0, waiting: 0, stopped_total: 1 }.ready_to_shutdown());
}
```

- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml aria2c_rpc`; confirm failures name the missing RPC behavior.
- [x] Implement token generation and private endpoint configuration; client requests use `.no_proxy()`, redirect policy `none`, a one-second timeout, explicit JSON content type, serde serialization and response-size limits. Add a local fake-server test asserting authentication and that injected proxy settings are bypassed.
- [x] Implement typed RPC parsing, checked byte aggregation and projection. For known nonzero totals, use `((completed as f64 / total as f64) * 100.0).clamp(0.0, 100.0)`; return `None` for unknown/zero totals. Missing batch history invalidates percentage/ETA. Map speed/ETA to current display strings and never publish raw RPC records.
- [x] Re-run focused tests; review token handling and malformed-input behavior. Update directory records.

## Task 2: Process/session lifecycle and argument injection

**Files:** `src-tauri/src/aria2c_rpc.rs`, `src-tauri/src/aria2c.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/test_support.rs`; new `src-tauri/tests/fixtures/aria2c_rpc_probe.rs`; nearest directory records.

**Interfaces:** Extend `aria2c_downloader_args` with optional runtime `RpcConfig` input. Add `RpcMonitor` with a stop signal and joined worker result. The worker consumes `RpcClient`/`SessionTracker`, emits current `DownloadProgress`, and returns a typed failure to the download owner. The download owner retains child/process-tree ownership; the worker does not mutate the user's cancellation flag.

- [x] Write failing fixture tests for startup delay, native-only fallback, fast completion before initial polling, active-plus-waiting queues, repeated sessions on one port, RPC loss after authentication, port collision/wrong token, stalled shutdown and cancellation at each lifecycle phase.
- [x] Extend argument tests to assert enabled-only injection, fresh token per application download, exact existing parallelism/path preservation and byte-for-byte disabled argument preservation.
- [x] Run focused argument/lifecycle tests before implementation and confirm failures demonstrate missing behavior.
- [x] Implement enabled-only runtime injection plus `--stop-with-process=<app-pid>`. Start the monitor within the download lifetime and drive it using the design's awaiting/connected/shutdown states. Use session ID checks to reject stale generations.
- [x] Implement shutdown after observed terminal work and empty active/waiting queues, with an immediate eligibility recheck. Keep normal empty startup distinct from terminal completion. Graceful shutdown must preserve child error codes and yt-dlp retry policy.
- [x] Integrate stop/join and process-tree cleanup on every success, failure, cancellation and spawn-error path. Join the monitor before `DownloadGuard::finish`. Never hold the active-process mutex across network I/O or joins.
- [x] Re-run fixture tests; verify shutdown failures fail visibly, cancellation kills descendants, and monitor results cannot reach a subsequent download. Update file records.

## Task 3: Progress routing and regression evidence

**Files:** `src-tauri/src/lib.rs`, `src-tauri/src/aria2c_rpc.rs`, fixture/tests from Task 2, `docs/aria2c.md`, nearest directory records. No frontend change unless an integration test demonstrates that the existing event contract cannot render the planned values.

**Interfaces:** Reuse `parse_progress_line`, `emit_progress`, `DownloadProgress` and `OUTPUT_PATH_PREFIX`. Session ownership gates only transfer events. yt-dlp output-path capture, final success/failure and future postprocessing extension points remain available.

- [x] Add failing source-routing tests: RPC active transfer wins over duplicate transfer output; native events still work when aria2c is enabled but unused; each new aria2c session resets percentage; RPC completion never emits whole-operation success; output-path markers are always captured.
- [x] Run those tests to establish failure, then wire the monitor's progress callback to the existing event emitter with session-scoped ownership and no stale updates after cancellation or completion.
- [x] Re-run routing, argument and lifecycle tests.
- [x] Run real local-media integration checks with installed yt-dlp/aria2c/FFmpeg: native-only; aria2c HTTP with live numeric progress; separate video/audio followed by merge; a fragmented queue; fast/cached completion; unknown size; HTTP failure; fragmented retry/skip behavior; user cancellation. Every scenario must exit or cancel without orphan processes, an open RPC listener or stale events. Confirm output files with ffprobe where media is produced.
- [ ] Repeat process-lifecycle checks on macOS and Windows. macOS complete; native Windows unavailable. Report untested platforms explicitly rather than treating a macOS smoke test as cross-platform proof.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `bun run test`, `bun run build`, and `git diff --check`. Separate pre-existing failures from change regressions; do not rewrite unrelated code to make checks pass.
- [x] Update user docs and directory records with actual guarantees and test evidence. Complete code-review/code-quality gates. Present changes for review; postprocessing/merge reporting remains a separate task.

## Acceptance

- [ ] Direct native UI observation of a rate-limited aria2c transfer. Backend event generation is verified against the real tools; event schema and frontend handler remain unchanged.
- [x] Unknown progress stays indeterminate; no division-by-zero, fabricated ETA or active-only batch percentage.
- [x] Video/audio subprocess transitions and fragment queues finish without premature shutdown or an RPC-induced hang.
- [x] Native progress and final output handling behave as before.
- [x] Cancellation, RPC failure and downloader failure clean up the complete operation and cannot be reported as success.
- [x] No RPC secrets or sensitive response fields enter logs, UI events or persisted settings.

## Execution record

- Baseline: 109 Rust library tests passed before changes.
- Final checks: 131 standard Rust tests and 208 frontend tests passed; the separately invoked real-tool test passed all 11 scenarios. Clippy with warnings denied, frontend production build and diff whitespace check passed. Authenticated RPC also passed with proxy environment variables pointing at an unavailable proxy.
- Implemented on `codex/aria2c-rpc-progress`, preserving pre-existing working-tree edits. User subsequently requested local integration into `main`, with the format-selector changes committed first and RPC changes committed separately.
- Simplification: monitor runs inside existing download worker; no additional monitor thread. RPC-client methods remain inside RpcMonitor. Real Rust HTTP fixtures replace planned compiled fake RPC executable.
- Independent review found and verified fixes for: uncertain-shutdown watchdog bypass, unbounded pagination, stale initial-session retry timer, and descendants retaining pipes after failed parent exit. Each has a regression test.
- Real-tool macOS scenarios passed: native, HTTP RPC, separate video/audio with verified merged streams, HLS, native fallback with aria2c enabled, unknown size, HTTP failure, broken HLS, recovering fragment retry, cancellation and fast completion. No leftover RPC listeners/process groups.
- No native Windows execution or native UI observation was available.
- Full formatting check reports pre-existing formatting in the output-container test in lib.rs; unrelated code is preserved. New/modified RPC code is formatted.
