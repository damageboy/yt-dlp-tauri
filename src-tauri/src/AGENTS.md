# Rust application source

| File | Purpose |
| --- | --- |
| `aria2c.rs` | Owns required executable validation, persistent optional usage, provider-aware save validation and optional runtime RPC arguments; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `aria2c_rpc.rs` | Separates poll-result policy from transport for platform-independent tests; See change: fork-release-builds. Owns local authenticated aria2c RPC lifecycle, declared/streamed response limits and progress projection; See change: aria2c-rpc-progress. |
| `aria2c_rpc_smoke_tests.rs` | Exercises actual yt-dlp, aria2c and FFmpeg against disposable local media; See change: aria2c-rpc-progress. |
| `aria2c_rpc_tests.rs` | Uses blocking accepted sockets; tolerates client timeout disconnects and OS-independent refusal/timeout policy assertions; See change: fork-release-builds. Verifies RPC progress, queue shutdown, transport and session lifecycle; See change: aria2c-rpc-progress. |
| `lib.rs` | Restricts runtime manifests and channel to damageboy/yt-dlp-tauri; rejects foreign/incomplete active revisions for both paths and manifests; resolves required managed Windows aria2c from manifest; See change: owned-toolchain. Requires aria2c for readiness/preflight; resolves managed Homebrew aria2c; DownloadRequest accepts OutputFormat mkv/mp4, defaults mp4; video_download_command merges/remuxes selected container; tests container arguments and rejection; See change: aria2c-external-downloader. See change: aria2c-required-tool. See change: download-format-selector. Routes RPC/native progress; owns RPC polling and pre-reader process cleanup; See change: aria2c-rpc-progress. |
| `test_support.rs` | Creates isolated native test directories and compiled executable fixtures; See change: aria2c-external-downloader. |
| `windows_process_tree.rs` | Finds owned Windows descendants after parent exit for RPC failure cleanup; See change: aria2c-rpc-progress. |
