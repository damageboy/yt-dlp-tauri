import assert from "node:assert/strict";
import test from "node:test";
import {
  executablePickerFilters,
  managedActionConfirmationKey,
  managedSummaryMode,
  showsRevision,
  type PlatformPresentation,
} from "../src/platform-toolchain.ts";

const macos: PlatformPresentation = {
  target: "macos-arm64",
  managedProvider: "homebrew",
  sourceLabels: { managed: "Homebrew", local: "Custom" },
  defaultSource: "managed",
  executableExtension: null,
  capabilities: { install: true, update: true, reinstall: true },
};

const windows: PlatformPresentation = {
  target: "win-x64",
  managedProvider: "archive-manifest",
  sourceLabels: { managed: "Managed", local: "Local" },
  defaultSource: "managed",
  executableExtension: "exe",
  capabilities: { install: true, update: true, reinstall: true },
};

test("macOS uses extensionless pickers and Homebrew confirmations", () => {
  assert.deepEqual(executablePickerFilters(macos), []);
  assert.equal(managedSummaryMode(macos), "homebrew");
  assert.equal(showsRevision(macos), false);
  assert.equal(
    managedActionConfirmationKey(macos, "install"),
    "settings.homebrewInstallConfirm",
  );
});

test("Windows preserves executable pickers and archive behavior", () => {
  assert.deepEqual(executablePickerFilters(windows), [
    { name: "Executable", extensions: ["exe"] },
  ]);
  assert.equal(managedSummaryMode(windows), "managed");
  assert.equal(showsRevision(windows), true);
  assert.equal(managedActionConfirmationKey(windows, "install"), null);
});
