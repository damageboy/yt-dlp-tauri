# Rust application source

| File | Purpose |
| --- | --- |
| `aria2c.rs` | Owns required executable validation, persistent optional usage, provider-aware save validation and arguments; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `lib.rs` | Requires aria2c for readiness/preflight; resolves managed Homebrew aria2c; DownloadRequest accepts OutputFormat mkv/mp4, defaults mp4; video_download_command merges/remuxes selected container; tests container arguments and rejection; See change: aria2c-external-downloader. See change: aria2c-required-tool. See change: download-format-selector. |
| `test_support.rs` | Creates isolated native test directories and compiled executable fixtures; See change: aria2c-external-downloader. |
