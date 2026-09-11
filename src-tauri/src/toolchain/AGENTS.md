# Toolchain source

| File | Purpose |
| --- | --- |
| `activation.rs` | Atomically replaces platform state files and activates verified toolchains; See change: aria2c-external-downloader. |
| `channel.rs` | Validates owned published revision releases including prereleases; enforces manifest digest and owned archive URLs; See change: owned-toolchain. |
| `homebrew.rs` | Limits Unix absolute-path discovery tests to Unix hosts; See change: fork-release-builds. Installs, verifies, updates and reinstalls required aria2 formula alongside other tools; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `install.rs` | Installs verified owned archive bytes; retains explicit candidate asset support; See change: owned-toolchain. |
| `mod.rs` | Exposes owned channel contract to runtime validation; See change: owned-toolchain. Exports toolchain APIs and crate-private shared probe/atomic replacement; See change: aria2c-external-downloader. |
| `probe.rs` | Verifies executables with bounded probes and stdout/stderr version summaries; See change: aria2c-external-downloader. |
