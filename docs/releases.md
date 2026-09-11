# Fork releases

Repository: [damageboy/yt-dlp-tauri](https://github.com/damageboy/yt-dlp-tauri). Default branch: `master`.

- Every push to `master` builds Windows x64 NSIS (`.exe`), macOS Apple Silicon (`.dmg`), and macOS Intel (`.dmg`). Successful builds update the [rolling master prerelease](https://github.com/damageboy/yt-dlp-tauri/releases/tag/master-build).
- Pushing a `v*` tag publishes a versioned GitHub Release after all three builds and checks pass. The tag must equal `v` plus the version in `package.json`; keep `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and both lockfiles in sync. The matching `CHANGELOG.md` section must include English and Chinese release notes.
- Each release includes `tools-manifest.json` and `SHA256SUMS`. Workflow artifacts retain installers for 14 days. Windows builds verify silent installation, application launch, and uninstallation. macOS builds verify the disk image.
- Installers are unsigned and macOS builds are not notarized. macOS requires Homebrew with `yt-dlp`, `ffmpeg`, `deno`, and `aria2`, as described in the README.
- The Release workflow supports manual preflight builds: leave `publish` false and select a `ref` (default `master`). To retry a tagged publication, set `publish` true and supply the existing version `tag`.
- The inherited Windows toolchain still uses upstream toolchain archives. This fork's app release workflow needs only the automatic `GITHUB_TOKEN`; it does not require upstream archive publishing credentials.
