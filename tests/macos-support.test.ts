import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const macConfig = JSON.parse(readFileSync("src-tauri/tauri.macos.conf.json", "utf8"));
const baseConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const ci = readFileSync(".github/workflows/ci.yml", "utf8");

test("macOS config emits app and dmg bundles with the icns icon", () => {
  assert.deepEqual(macConfig.bundle.targets, ["app", "dmg"]);
  assert.ok(macConfig.bundle.icon.includes("icons/icon.icns"));
  assert.ok(baseConfig.bundle.resources.includes("platform-toolchains.json"));
});

test("CI builds and verifies a native macOS app", () => {
  assert.match(ci, /verify-macos:[\s\S]*?runs-on: macos-15/u);
  assert.match(ci, /npm run tauri build -- --debug --bundles app/u);
  assert.match(ci, /target\/debug\/bundle\/macos\/yt-dlp-tauri\.app/u);
});
