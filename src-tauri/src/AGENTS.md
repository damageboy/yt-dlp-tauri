# Rust application source

| File | Purpose |
| --- | --- |
| `aria2c.rs` | Owns optional aria2c configuration, resolution, atomic settings, concurrent saves and arguments; See change: aria2c-external-downloader. |
| `lib.rs` | Orchestrates Tauri commands, toolchain, metadata and download lifecycle; See change: aria2c-external-downloader. |
| `test_support.rs` | Creates isolated native test directories and compiled executable fixtures; See change: aria2c-external-downloader. |
