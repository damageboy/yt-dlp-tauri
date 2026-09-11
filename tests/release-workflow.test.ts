import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/release.yml", "utf8");
const installerCheck = readFileSync("scripts/verify-windows-installer.ps1", "utf8");
const cargoManifest = readFileSync("src-tauri/Cargo.toml", "utf8");

test("release workflow builds master pushes, version tags, and manual preflights", () => {
  assert.match(workflow, /push:\s+branches: \[master\]\s+tags: \["v\*"\]/u);
  assert.match(workflow, /workflow_dispatch:[\s\S]*?ref:[\s\S]*?publish:[\s\S]*?tag:/u);
  assert.match(workflow, /default: master/u);
  assert.match(workflow, /persist-credentials: false/u);
  for (const target of ["windows-x64", "macos-aarch64", "macos-x64"]) {
    assert.ok(workflow.includes(`name: ${target}`));
  }
  assert.match(workflow, /--target aarch64-apple-darwin --bundles dmg/u);
  assert.match(workflow, /--target x86_64-apple-darwin --bundles dmg/u);
});

test("release publication waits for every installer and distinguishes rolling builds", () => {
  assert.match(workflow, /publish-release:[\s\S]*?needs: build-release/u);
  assert.match(workflow, /if:.*github.event_name == 'push' \|\| inputs.publish/u);
  assert.match(workflow, /gh release create.*--draft/u);
  assert.match(workflow, /gh release edit.*--draft=false/u);
  assert.match(workflow, /master-build/u);
  assert.match(workflow, /--prerelease.*--latest=false/u);
  assert.match(workflow, /sha256sum.*SHA256SUMS/u);
  assert.match(workflow, /if-no-files-found: error/u);
});

test("every Windows release installs, launches, and removes the package", () => {
  assert.match(workflow, /if: runner.os == 'Windows'[\s\S]*?verify-windows-installer\.ps1/u);
  assert.match(workflow, /actions\/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a/u);
  assert.match(installerCheck, /ArgumentList "\/S"/u);
  assert.match(installerCheck, /DisplayVersion/u);
  assert.match(installerCheck, /Start-Process.*installedExecutable/u);
  assert.match(installerCheck, /uninstall\.exe/u);
  assert.match(installerCheck, /ConvertTo-Json/u);
});

test("release builds pin the Tauri application binary", () => {
  const packageSection = cargoManifest.match(/^\[package\]\n([\s\S]*?)(?=^\[)/mu)?.[1] ?? "";

  assert.match(packageSection, /^default-run = "yt-dlp-tauri"$/mu);
  assert.doesNotMatch(packageSection, /^default-run = "toolchain-smoke"$/mu);
});
