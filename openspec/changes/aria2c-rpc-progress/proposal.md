# aria2c RPC download progress

## Why

yt-dlp passes aria2c console progress through without translating live updates into the application's progress template. The existing parser therefore receives only the final transfer event. A local experiment with yt-dlp 2026.08.19 and aria2c 1.37.0 confirmed zero structured downloading events for aria2c and working live byte/speed reports through aria2c JSON-RPC.

## What changes

- Inject authenticated, loopback-only RPC options through the existing aria2c downloader arguments when aria2c usage is enabled.
- Read aria2c transfer progress in the Rust backend and translate it into the existing `DownloadProgress` event/UI.
- Manage RPC-enabled aria2c shutdown so yt-dlp can continue through subsequent transfers and postprocessing.
- Preserve yt-dlp progress parsing for native downloads and protocols that yt-dlp does not delegate to aria2c.
- Preserve current output-path detection, final process-success handling and cancellation semantics.

No postprocessing/merge progress redesign, yt-dlp JSON-template migration, settings UI, persistent RPC server or new download queue is included.

## Discipline Skills

`security-hardening`, `observability-instrumentation`, `doubt-driven-review`; `systematic-debugging` when failures surface. The repository reports eng-disciplines unavailable. Do not silently install it; apply the explicit session-authentication, bounded-I/O, cleanup and regression requirements in this change.

## Status

Approved by user; implemented with an available port per application download. Independent review findings fixed. Native Windows runtime verification remains outstanding. See tasks.md for execution evidence.

See [design.md](design.md) and [tasks.md](tasks.md).
