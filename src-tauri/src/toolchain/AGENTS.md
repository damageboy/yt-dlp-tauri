# Toolchain source

| File | Purpose |
| --- | --- |
| `activation.rs` | Atomically replaces platform state files and activates verified toolchains; See change: aria2c-external-downloader. |
| `homebrew.rs` | Installs, verifies, updates and reinstalls required aria2 formula alongside other tools; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `mod.rs` | Exports toolchain APIs and crate-private shared probe/atomic replacement; See change: aria2c-external-downloader. |
| `probe.rs` | Verifies executables with bounded probes and stdout/stderr version summaries; See change: aria2c-external-downloader. |
