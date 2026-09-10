# Toolchain source

| File | Purpose |
| --- | --- |
| `activation.rs` | Atomically replaces platform state files and activates verified toolchains; See change: aria2c-external-downloader. |
| `homebrew.rs` | Resolves Homebrew with bounded prefix queries; manages required formulas; See change: aria2c-external-downloader. |
| `mod.rs` | Exports toolchain APIs and crate-private shared probe/atomic replacement; See change: aria2c-external-downloader. |
| `probe.rs` | Verifies executables with bounded probes and stdout/stderr version summaries; See change: aria2c-external-downloader. |
