# Rust application source

| File | Purpose |
| --- | --- |
| `aria2c.rs` | Owns required executable validation, persistent optional usage, resolution and arguments; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `lib.rs` | Requires aria2c for readiness and runtime preflight; distinguishes configuration remediation; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `test_support.rs` | Creates isolated native test directories and compiled executable fixtures; See change: aria2c-external-downloader. |
