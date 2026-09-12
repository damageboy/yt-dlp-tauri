# English-only application

The app has one English message map. Language selection, browser-locale detection and the saved-language setting no longer control rendered text. Existing saved language values are ignored. The app sets HTML language and date/time formatting to English; downloaded metadata and filenames retain Unicode support.

`src/release-notes.ts` extracts bullets from the requested version without a language argument. `.github/scripts/extract-release-notes.mjs` accepts nonempty version sections without bilingual headings. All historical `CHANGELOG.md` sections keep their English notes and version dates.

Root-file contracts:

| File | Purpose |
| --- | --- |
| `CHANGELOG.md` | Stores English release sections without language headings; See change: english-only-app. |
| `README.md` | Describes English app; links current screenshot; removes language-switch feature/link; See change: english-only-app. |
| `index.html` | Omits language controls; preserves shared controls and English fallback labels; See change: english-only-app. |

Removed `README_zh.md` and `docs/assets/readme-zh.png`. Refreshed `docs/assets/readme-en.png` so the README image no longer shows the old language switch. Automated candidate/repair commit messages use English.

Verification: release-note regression tests first failed on plain English sections; updated parsers pass. Browser preview uses a stubbed Tauri backend, `zh-CN` locale and legacy saved `zh` preference to check English startup, Settings and update notes. Native downloader behavior is unchanged.
