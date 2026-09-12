import assert from "node:assert/strict";
import test from "node:test";

import { releaseNotesForVersion, shouldShowReleaseNotes, stripTerminalSentencePunctuation } from "../src/release-notes.ts";

const changelog = `
# Changelog

## Unreleased

## 0.2.0 - 2026-06-24

- Download completed.
- Keeps ellipsis...

## 0.1.9 - 2026-06-24

- Previous version.
`;

test("shouldShowReleaseNotes only opens after a stored version changes", () => {
  assert.equal(shouldShowReleaseNotes(null, "0.2.0"), false);
  assert.equal(shouldShowReleaseNotes("", "0.2.0"), false);
  assert.equal(shouldShowReleaseNotes("v0.2.0", "0.2.0"), false);
  assert.equal(shouldShowReleaseNotes("0.1.9", "0.2.0"), true);
});

test("releaseNotesForVersion extracts plain English bullets for the current version", () => {
  assert.deepEqual(releaseNotesForVersion(changelog, "v0.2.0"), {
    version: "0.2.0",
    items: ["Download completed", "Keeps ellipsis..."],
  });
});

test("stripTerminalSentencePunctuation removes only sentence-ending full stops", () => {
  assert.equal(stripTerminalSentencePunctuation("Paste, choose, download."), "Paste, choose, download");
  assert.equal(stripTerminalSentencePunctuation("Reading metadata..."), "Reading metadata...");
  assert.equal(stripTerminalSentencePunctuation("Already clean"), "Already clean");
});

test("releaseNotesForVersion reads bullets across category headings", () => {
  assert.deepEqual(releaseNotesForVersion("## 0.2.0\n### Added\n- Parallel downloads.\n### Fixed\n- Settings.\n## 0.1.9\n- Older notes.", "0.2.0"), {
    version: "0.2.0",
    items: ["Parallel downloads", "Settings"],
  });
  assert.equal(releaseNotesForVersion(changelog, "9.0.0"), null);
});
