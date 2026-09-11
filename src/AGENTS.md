# Frontend source

| File | Purpose |
| --- | --- |
| `aria2c-settings.ts` | Owns aria2c wire types, draft validation, path-only persistence and save ordering; See change: aria2c-external-downloader. |
| `main.ts` | Consolidates aria2c usage and custom paths into Toolchain; refreshes readiness after Save; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `styles.css` | Styles consolidated Toolchain usage controls and configuration-required warnings; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `toolchain.ts` | Summarizes tool readiness; configuration_required blocks package remediation and remote update promotion; See change: aria2c-required-tool. |
