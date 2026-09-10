# Frontend source

| File | Purpose |
| --- | --- |
| `aria2c-settings.ts` | Owns aria2c wire types, draft validation and stale inspection protection; See change: aria2c-external-downloader. |
| `main.ts` | Renders required aria2c setup with optional usage; refreshes readiness after Save; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `styles.css` | Styles app settings and configuration-required tool warnings; See change: aria2c-external-downloader. See change: aria2c-required-tool. |
| `toolchain.ts` | Summarizes tool readiness; configuration_required blocks package remediation and remote update promotion; See change: aria2c-required-tool. |
