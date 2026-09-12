# Implementation and verification

- [x] Add failing regression tests for native fragment parallelism and both command paths.
- [x] Always add native fragment concurrency; retain optional aria2c/RPC settings.
- [x] Update English/Chinese UI help and downloader documentation.
- [x] Verify 134 Rust tests, 215 frontend tests, frontend build and Clippy; review final diff.
- [x] Run local real-tool lifecycle test: all 11 scenarios pass, including HLS, aria2c HTTP, native fallback, retries and cancellation.
- [x] Format changed Rust blocks and pass git diff --check. Full cargo fmt --check reports only pre-existing formatting in unchanged output-container test at src-tauri/src/lib.rs:2512.

Verification ran on macOS; native Windows execution was not repeated.
