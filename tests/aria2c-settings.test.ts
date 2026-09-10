import assert from "node:assert/strict";
import test from "node:test";
import { Aria2cSettingsDraft, defaultAria2cSettings, parseParallelConnections, type Aria2cStatus } from "../src/aria2c-settings.ts";

test("parallelism rejects non-integral, empty and out-of-range input", () => {
  assert.equal(parseParallelConnections("1"), 1);
  assert.equal(parseParallelConnections("16"), 16);
  for (const value of ["", "0", "17", "1.5", "1e1", "NaN", "-1"]) {
    assert.equal(parseParallelConnections(value), null, value);
  }
});

test("failed save retains draft and does not overwrite saved settings", async () => {
  const draft = new Aria2cSettingsDraft();
  draft.edit({ enabled: true, executablePath: "/tools/aria2c" });
  await assert.rejects(draft.save(async () => { throw new Error("missing"); }));
  assert.equal(draft.dirty, true);
  assert.equal(draft.config.enabled, true);
  assert.equal(draft.saved.config.enabled, false);
  draft.edit({ enabled: false, executablePath: null }, "8");
  const result = await draft.save(async config => ({ ...defaultAria2cSettings(), config }));
  assert.equal(result.config.parallelConnections, 8);
  assert.equal(draft.dirty, false);
  assert.equal(draft.saved.config.executablePath, null);
});

test("late inspection cannot overwrite a newer draft or its status", async () => {
  const draft = new Aria2cSettingsDraft();
  let resolveOld!: (status: Aria2cStatus) => void;
  const pending = draft.inspect(() => new Promise(resolve => { resolveOld = resolve; }));
  draft.edit({ executablePath: "/new/aria2c" });
  await draft.inspect(async () => ({ ...defaultAria2cSettings().status, errorCode: "not-found", error: "new" }));
  resolveOld({ ...defaultAria2cSettings().status, available: true, version: "old" });
  await pending;
  assert.equal(draft.status.error, "new");
  assert.equal(draft.status.available, false);
});

test("unrelated AppState refresh preserves dirty settings and invalid numeric input", () => {
  const draft = new Aria2cSettingsDraft();
  draft.edit({ enabled: true }, "");
  draft.applySaved(defaultAria2cSettings());
  assert.equal(draft.config.enabled, true);
  assert.equal(draft.parallelInput, "");
  assert.throws(() => draft.validatedConfig());
});
