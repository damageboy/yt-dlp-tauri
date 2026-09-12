# Fork releases

Repository: [damageboy/yt-dlp-tauri](https://github.com/damageboy/yt-dlp-tauri). Default branch: `master`.

- Every push to `master` builds Windows x64 NSIS (`.exe`), macOS Apple Silicon (`.dmg`), and macOS Intel (`.dmg`). Successful builds update the [rolling master prerelease](https://github.com/damageboy/yt-dlp-tauri/releases/tag/master-build).
- Pushing a `v*` tag publishes a versioned GitHub Release after all three builds and checks pass. The tag must equal `v` plus the version in `package.json`; keep `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and both lockfiles in sync. The matching `CHANGELOG.md` section contains English release notes without language-specific headings.
- Each release includes `tools-manifest.json` and `SHA256SUMS`. Workflow artifacts retain installers for 14 days. Windows builds verify silent installation, application launch, and uninstallation. macOS builds verify the disk image.
- Installers are unsigned and macOS builds are not notarized. macOS requires Homebrew with `yt-dlp`, `ffmpeg`, `deno`, and `aria2`, as described in the README.
- The Release workflow supports manual preflight builds: leave `publish` false and select a `ref` (default `master`). To retry a tagged publication, set `publish` true and supply the existing version `tag`.
- Windows installs all five tools from owned release assets. Every Windows app release performs a clean toolchain install, version/compatibility probes and the real aria2 RPC download lifecycle before packaging. Toolchain releases share this repository, use `toolchain-*` tags, and are prereleases excluded from Latest. See [ownership and provenance](toolchain-ownership.md).
- When a push also updates the toolchain, the Windows release job waits up to 25 minutes for that exact revision and matching manifest digest before installing it. Missing or mismatched publication blocks the app release.

## v0.1.14

Application version is synchronized in root `package.json`, root `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`. Root `CHANGELOG.md` contains English release notes. This release adds shared native fragment parallelism and uses toolchain baseline `20260912.1`. See change: native-fragment-parallelism.

## v0.1.115

English-only application release. All five version files listed above use `0.1.115`, matching the requested `v0.1.115` tag; `CHANGELOG.md` supplies English release notes. See change: english-only-app.
