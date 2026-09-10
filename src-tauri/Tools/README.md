# Tool cache

Large Windows tool binaries are not committed to Git because GitHub rejects files over 100 MB.

`Tools/win-x64` is only a cache for project-controlled Windows archives. The app can install that Windows toolchain from the Toolchain panel. For development or offline packaging, restore `win-x64` from the Tauri project root with:

```powershell
.\scripts\download-tools.ps1
```

The expected Windows target, layout, source URLs, and hashes are documented in `src-tauri/tools-manifest.json`.

macOS managed tools are external Homebrew installations. They are never downloaded or copied into this directory, the application bundle, or the repository.
