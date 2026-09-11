# Toolchain ownership and provenance

Windows runtime downloads and the `toolchain-stable` channel belong to `damageboy/yt-dlp-tauri`. The app never falls back to the previous maintainer's release service. Cached manifests from that service cannot supply installation downloads. Install or reinstall tools after upgrading an older app to activate the owned five-tool revision.

## Binary origins

| Tool | Selected upstream artifact | Transformation |
| --- | --- | --- |
| yt-dlp | `yt-dlp/yt-dlp`, `2026.08.19`, `yt-dlp.exe` | Mirrored executable, unchanged |
| Deno | `denoland/deno`, `v2.9.5`, `deno-x86_64-pc-windows-msvc.zip` | Mirrored ZIP, unchanged; installer extracts `deno.exe` |
| FFmpeg / ffprobe | `yt-dlp/FFmpeg-Builds`, `autobuild-2026-07-31-16-16`, `ffmpeg-N-125875-g5d4d3bdc61-win64-gpl.zip` | Mirrored ZIP, unchanged; installer extracts both executables |
| aria2c | `aria2/aria2`, `release-1.37.0`, `aria2-1.37.0-win-64bit-build1.zip` | Official x64 ZIP, unchanged; installer extracts `aria2-1.37.0-win-64bit-build1/aria2c.exe` |

Mirroring freezes the selected bytes under our control. It does not establish that inherited binaries were independently audited or reproducibly built. `toolchain-lock.json` retains upstream identities and hashes, while `src-tauri/tools-manifest.json` contains only owned runtime URLs. Original author attribution, notices and historical provenance remain intact.

aria2 ZIP SHA-256: `67d015301eef0b612191212d564c5bb0a14b5b9c4796b76454276a4d28d9b288`.

aria2 executable SHA-256: `be2099c214f63a3cb4954b09a0becd6e2e34660b886d4c898d260febfe9d70c2`.

The old aria2 release has no GitHub asset digest. Its explicitly reviewed archive hash is committed in policy as `expectedSha256`; the resolver refuses different bytes. It is pinned to 1.37.0 rather than automatically following the latest release.

## Historical migration

All 42 assets from four original toolchain revisions were downloaded, checked against their published SHA-256 and sizes, uploaded to matching tags in this repository, then downloaded again and checked. [The inventory](toolchain-migration-inventory.json) records every filename, size and hash.

Historical manifests and reports remain byte-identical archival evidence, including their original provenance URLs. They are not active channel manifests and cannot be promoted by the owned runtime. Revision `20260911.1` changes runtime custody and adds aria2c while reusing the copied Deno, FFmpeg and yt-dlp archives.

## Publication contract

- `toolchain-*` releases share the application repository. They are public prereleases with `make_latest: false`; drafts cannot serve unauthenticated desktop downloads.
- Repository-wide immutable releases are not enabled because the app's rolling `master-build` must be updated. Toolchain publishing refuses to replace existing revision assets and verifies their exact size, digest and final download bytes before channel promotion.
- Discovery and freshness workflows use scoped `GITHUB_TOKEN` permissions and open pull requests against `master`. Enable **Allow GitHub Actions to create and approve pull requests** in repository Actions settings for automated PR creation. No separate repository or original maintainer's GitHub App credentials are needed.
- New revisions require native Windows validation. Publication binds the reviewed PR commit, candidate digest, lock, manifest and validation report, then revalidates the same bytes before promoting the stable channel.
- The app release workflow independently performs a clean five-tool Windows installation, compatibility checks and real aria2 RPC download lifecycle test before packaging. macOS continues to use local Homebrew formulas.

## Files maintained together

| File | Contract |
| --- | --- |
| `toolchain-policy.json` | Reviewed source selection, owned archive destination and pinned aria2 hash |
| `toolchain-lock.json` | Exact selected upstream and mirror identities; archive/member hashes |
| `src-tauri/tools-manifest.json` | Generated owned Windows runtime URLs and all five required tools |
| `TOOLCHAIN_CHANGELOG.md` | Tool revision and custody changes independent of app version |
| `THIRD-PARTY-NOTICES.md` | Binary origins, applicable licenses and corresponding-source references |
| `README.md`, `README_zh.md` | Installation, owned hosting and maintenance instructions |

Generate the manifest and changelog through `scripts/toolchain/generate-manifest.mjs`; use `scripts/update-toolchain.mjs` for future source resolution. The installer validates both downloaded archives and extracted executables before activation. Missing redistribution evidence stops publication; there is no runtime upstream fallback.
