# Optional aria2c external downloader

> Installation and disabled-operation requirements are superseded by `aria2c-required-tool` following the user correction on 2026-09-11.

## Why

The app can now run its required tools on macOS, but Settings cannot select aria2c or control the external downloader's parallelism. Downloads always use yt-dlp's existing downloader selection.

## What changes

- Add an optional, default-off aria2c setting with automatic discovery or an explicit executable path.
- Add one integer Parallelism setting, 1–16, default 16, mapped to `-j N -x N -s N`.
- Persist validated settings independently of the required toolchain.
- Discover PATH installations and macOS Homebrew installations, including a nonstandard resolved Homebrew prefix.
- Revalidate before enabled downloads and cancel the yt-dlp process group/tree, including aria2c.
- Preserve metadata extraction, disabled download arguments, and the existing indeterminate-progress UI.

aria2c installation remains user-managed. This change does not add an automatic installer, raw downloader arguments, or a queue for concurrent videos. N is neither a total connection cap nor a promise that every protocol uses aria2c.

## Discipline Skills

`security-hardening`, `observability-instrumentation`, `doubt-driven-review`; conditionally `systematic-debugging` if implementation exposes a bug and `code-simplification` if the passing implementation becomes unnecessarily heavy. The project currently reports the eng-disciplines package unavailable; record this accurately during execution and apply the concrete validation and review requirements in tasks.md. Do not silently install an unrelated plugin.

## Impact and status

Planning complete; implementation not started. No new production dependency is planned. Tauri settings commands and AppState gain aria2c fields; existing saved settings remain compatible.

Canonical implementation checklist: [tasks.md](tasks.md). Behavioral requirements: [spec.md](specs/aria2c-downloader/spec.md). Detailed rationale: [design](../../../docs/superpowers/specs/2026-09-10-aria2c-external-downloader-design.md).
