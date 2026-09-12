# Frontend source

| File | Purpose |
| --- | --- |
| `aria2c-settings.ts` | Owns aria2c wire types, draft validation, path-only persistence and save ordering; See change: aria2c-external-downloader. |
| `main.ts` | Points app updates to damageboy/yt-dlp-tauri; See change: fork-release-builds. Consolidates aria2c Toolchain controls; refreshes readiness after Save; sends selected output_format; locks download selectors while busy; See change: aria2c-external-downloader. See change: aria2c-required-tool. See change: download-format-selector. English help describes shared native fragment and aria2c parallelism; See change: native-fragment-parallelism. Uses English messages; removes language state, locale detection and selector handlers; See change: english-only-app. |
| `release-notes.ts` | Parses version-scoped English bullets without locale argument; preserves version gating and English punctuation handling; See change: english-only-app. |
| `styles.css` | Styles Toolchain warnings; aligns compact Format/Quality selectors with download buttons; stacks labels above controls; See change: aria2c-external-downloader. See change: aria2c-required-tool. See change: download-format-selector. Removes language-toggle styles; retains shared segment controls; See change: english-only-app. |
| `toolchain.ts` | Summarizes tool readiness; configuration_required blocks package remediation and remote update promotion; See change: aria2c-required-tool. |
