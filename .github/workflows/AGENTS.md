# Workflow records

| File | Purpose |
| --- | --- |
| `ci.yml` | Tests frontend and Rust on pull requests and master pushes; verifies native macOS app; See change: fork-release-builds. |
| `release.yml` | Builds Windows x64 and macOS arm64/x64 installers; publishes v* tags and rolling master-build after platform verification; supports manual preflight; See change: fork-release-builds. |
