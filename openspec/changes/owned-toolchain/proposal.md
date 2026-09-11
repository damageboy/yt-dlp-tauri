# Own the Windows toolchain

The fork still installs tools controlled by the original app maintainer. Move every active download and channel into `damageboy/yt-dlp-tauri`, preserve all historical release assets byte for byte, and add official Windows x64 aria2 1.37.0. Keep legal attribution and original provenance truthful.

## Discipline Skills

Security-hardening and doubt-driven-review apply to manifest trust and release promotion; their plugin is unavailable. Use focused trust-boundary tests and independent review. Use subagent-driven-development for implementation and verification-before-completion for closeout.

## Acceptance

- Managed Windows installs yt-dlp, ffmpeg, ffprobe, deno, and aria2c from owned release URLs with archive and executable SHA-256 verification.
- Original-owner channels, fallbacks, cached download URLs, publishing credentials and Git remote are removed from active use.
- Same-repository `toolchain-*` releases are public prereleases, never latest; publishing refuses asset replacement.
- Master pushes and version tags retain Windows and both macOS installer builds. Native Windows validates the five-tool installation and aria2 RPC lifecycle before app publication.
- User's existing uncommitted UI edits remain intact.
