# Changelog

All notable changes to this project will be documented in this file.

## 0.1.115 - 2026-09-12

- Make the app English-only; remove the language switch and Chinese translations.
- Keep English UI and release notes regardless of browser locale or a previously saved language preference.
- Remove the Chinese README and screenshot, refresh the English screenshot, and keep changelog history in English.
- Generate English release notes without requiring bilingual sections.

## 0.1.14 - 2026-09-12

- Always pass `--concurrent-fragments N` using saved parallelism (default 16), including native HLS/DASH fallback while aria2c is enabled.
- Keep aria2c parallelism and authenticated RPC progress when enabled, with updated Settings help explaining both download paths.
- Use the self-contained, project-hosted `20260912.1` Windows toolchain baseline with yt-dlp, FFmpeg, FFprobe, Deno, and aria2c.

- Added Homebrew-managed tools on macOS, with install, update, and reinstall limited to validated `yt-dlp`, `ffmpeg`, `deno`, and `aria2` formulas when Homebrew already exists; the app never installs Homebrew silently.
- Added extensionless absolute executable paths in macOS Custom mode and Finder-safe discovery through the inherited `PATH`, `/opt/homebrew/bin`, and `/usr/local/bin`.
- Switched download, application state, log, and folder-opening behavior to native macOS paths while preserving existing Windows paths.
- Publish Windows x64 installers and macOS Apple Silicon / Intel disk images with CI verification. Installers are unsigned and macOS builds are not notarized.

## 0.1.13 - 2026-07-14

- Added a Settings switch between the app-managed and local toolchains, with `PATH` auto-detection and absolute-path selection for yt-dlp, the FFmpeg directory, and Deno.
- Required FFmpeg and FFprobe to share a directory and verified the complete local toolchain through version probes and a deterministic media test, while leaving local versions, hashes, and updates under user control.
- Displayed a concise `Not Found` status when a local tool cannot be detected.

## 0.1.12 - 2026-07-12

- Moved tool updates to a project-controlled immutable archive and independent stable channel so validated yt-dlp, Deno, FFmpeg, and FFprobe revisions can ship without an application release.
- Staged complete toolchain revisions before atomically activating them after source, archive, executable hash, and compatibility checks; failed updates preserve the current working revision.
- Displayed the active toolchain revision in Settings and preserved migration compatibility with v0.1.11 flat tool directories and its release manifest, switching layouts after the first successful activation.
- Added unified discovery, freshness, native compatibility validation, archive publication, Canary, and rollback workflows while keeping tool updates behind maintainer-reviewed pull requests.
- Updated the app-managed `yt-dlp` to `2026.07.04` and authenticated GitHub API requests to reduce failures from stale extractors and scheduled rate limits.
- Focused application packaging and the managed toolchain on Windows 10/11 x64 with an NSIS installer.

## 0.1.11 - 2026-06-24

- Moved tool update manifest downloads to the Rust backend so GitHub release assets without CORS headers no longer surface as `Failed to fetch` in the WebView.

## 0.1.10 - 2026-06-24

- Added a post-update release notes dialog for the current version, with a manual Release notes entry in Settings.
- UI translation copy now removes sentence-ending full stops so prompts such as "Download completed." render without the final period.
- Replaced the inline top status panel with top-right toast notifications; success and warning toasts auto-dismiss while errors stay until dismissed.

## 0.1.9 - 2026-06-24

- Reworked Settings toolchain actions around verify, check tool updates, install, update, and reinstall, removing the ambiguous repair primary action.
- Tool update checks now read only the `tools-manifest.json` attached to the latest `yt-dlp-tauri` GitHub Release and continue installing managed tools with SHA-256 verification.
- The Release workflow now uploads `tools-manifest.json` as a release asset for in-app tool update checks.

## 0.1.8 - 2026-06-24

- Tool checks now verify local tools against the `tools-manifest.json` SHA-256 pins so runnable but stale tools are detected.
- The Settings toolchain primary action now switches between installing, updating, and repairing tools based on local state.
- Tool download progress now reports the current file's real download percentage, fixing Windows repair flows that started at 99%.
- Upgraded GitHub Actions official actions and CI build Node.js to Node 24.

## 0.1.7 - 2026-06-23

- Replaced tool installation downloads with in-app Rust HTTP streaming and real progress from `Content-Length` plus downloaded bytes.
- Updated Windows `ffmpeg`/`ffprobe` to a downloadable pinned upstream build, fixing install failures from the old release asset returning 404.
- Added a tool manifest source URL health check so scheduled checks catch unavailable upstream assets.
- Tool downloads now retry transient network errors and temporary HTTP failures to reduce install failures during large downloads.

## 0.1.6 - 2026-06-23

- Updated the app-managed `yt-dlp` to `2026.06.09` to reduce metadata failures from stale site extractors.
- Preferred `yt-dlp` `ERROR:` lines when metadata parsing fails so version-age warnings do not hide the real error.
- Added a scheduled GitHub Actions workflow that checks the pinned manifest against the latest upstream `yt-dlp` release.

## 0.1.5 - 2026-05-27

- Cookie files now support standard Netscape `cookies.txt` plus one-line browser Cookie headers such as `Cookie: a=b; c=d` or `a=b; c=d`.

## 0.1.4 - 2026-05-27

- Added a home-screen Cookie file picker for switching `cookies.txt` across platforms or accounts.
- Passed the selected Cookie file to `yt-dlp` for both metadata parsing and downloads.
- Cleared parsed metadata after changing or clearing the Cookie file so downloads use the current account state.

## 0.1.3 - 2026-05-27

- Added GitHub Actions release packaging for Windows NSIS, macOS Intel DMG, and macOS Apple Silicon DMG artifacts from `v*` tags.
- Added macOS tool manifest targets and platform-specific backend tool path resolution.
- Documented the `yt-dlp` supported-sites list in the README.

## 0.1.2 - 2026-05-26

- Added GitHub release notes generation from the matching `CHANGELOG.md` version section.
- Finished the video metadata progress text after successful or failed parsing.
- Improved GitHub update check errors for API rate limits with reset-time guidance.
- Fixed Bilibili thumbnail previews by suppressing referrers on remote cover images.

## 0.1.1 - 2026-05-26

- Hid background Windows command windows during tool checks, installs, metadata parsing, downloads, extraction, and cancellation.
- Improved tool install, archive extraction, and process failure messages with concrete paths, exit codes, and stderr details.
- Aligned the Settings footer version row with update and project controls.
- Fixed thumbnail previews by normalizing HTTP thumbnail URLs to HTTPS and retrying fallback thumbnail candidates.

## 0.1.0 - 2026-05-26

- Renamed the project to `yt-dlp-tauri`.
- Added automatic current-target toolchain installation.
- Added a simplified desktop UI.
- Added fixed-version tool manifest entries with SHA-256 verification.
- Added manual GitHub Release update checks.
- Added Direct / gh-proxy routing for update checks and release links.
- Added project metadata and a project home link in Settings.
- Added GitHub-ready README documentation.
- Added CI checks for frontend tests, frontend builds, Rust tests, and Rust checks.
