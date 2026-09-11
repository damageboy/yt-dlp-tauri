# Frontend source

| File | Purpose |
| --- | --- |
| `aria2c-settings.ts` | Owns aria2c wire types, draft validation, path-only persistence and save ordering; See change: aria2c-external-downloader. |
| `main.ts` | Consolidates aria2c Toolchain controls; refreshes readiness after Save; sends selected output_format; locks download selectors while busy; See change: aria2c-external-downloader. See change: aria2c-required-tool. See change: download-format-selector. |
| `styles.css` | Styles Toolchain warnings; aligns compact Format/Quality selectors with download buttons; stacks labels above controls; See change: aria2c-external-downloader. See change: aria2c-required-tool. See change: download-format-selector. |
| `toolchain.ts` | Summarizes tool readiness; configuration_required blocks package remediation and remote update promotion; See change: aria2c-required-tool. |
