# Implementation and verification

- [x] Add failing plain-English release-note tests, then update both parsers.
- [x] Remove Chinese UI, language controls/state and locale-based formatting.
- [x] Remove Chinese README, screenshot, changelog sections and automation messages.
- [x] Verify browser startup with Chinese locale/legacy preference, Settings and update notes; refresh README screenshot.
- [x] Clean and verify published v0.1.14 description.
- [x] Run frontend tests/build, release-note extraction and final content/diff checks.

Result: 216 frontend tests and production build pass; all historical version notes parse. Browser check confirms English with zh-CN browser locale and legacy zh preference. Published v0.1.14 description matches cleaned text.
