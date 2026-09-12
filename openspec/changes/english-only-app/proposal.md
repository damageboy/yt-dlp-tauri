# Remove Chinese localization

## Why

The user wants English-only application UI, README, changelog and release notes.

## What changes

Remove the Chinese message dictionary, language controls and locale selection. Ignore saved language preferences and use English date/time rendering. Keep one English message map. Remove the Chinese README/screenshot and refresh the English screenshot. Preserve English changelog history while removing language headings. Make app and release parsers accept plain version-scoped English notes. Remove Chinese automation commit text and clean the published v0.1.14 release description.

This supersedes earlier bilingual UI/release-note requirements. Toolchain behavior, Unicode metadata/filenames and original license/attribution remain unchanged.
