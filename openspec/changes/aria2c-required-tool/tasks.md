# Implementation and verification

- [x] Add Homebrew aria2 formula to both macOS targets and all management operations.
- [x] Require aria2c during readiness checks and metadata/download preflight independent of enabled usage.
- [x] Keep off-by-default usage and existing downloader arguments; refresh readiness after saving configuration.
- [x] Update English/Chinese UI and setup documentation.
- [x] Native missing-executable/recovery checks and independent review passed. 208 frontend tests, 109 Rust tests, production build, formatting and Clippy passed.
- [ ] Resolve Windows managed-distribution choice.
