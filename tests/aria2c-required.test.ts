import assert from "node:assert/strict";
import test from "node:test";
import { summarizeTools, summarizeRemoteTools, type ToolStatus } from "../src/toolchain.ts";

function aria2c(availability: ToolStatus["availability"]): ToolStatus {
  return { name: "aria2c", full_path: "", relative_path: "aria2c", availability };
}

test("unavailable configured aria2c blocks readiness without reinstalling managed packages", () => {
  for (const mode of ["managed", "homebrew", "local", "remote"] as const) {
    const summary = summarizeTools([aria2c("configuration_required")], mode);
    assert.equal(summary.ready, false);
    assert.equal(summary.action, null);
    assert.equal(summary.settingsKey, "aria2c.configurationRequired");
  }
});

test("a newer archive cannot replace missing aria2c configuration with an update action", () => {
  const summary = summarizeRemoteTools([aria2c("configuration_required")], "20260910.1", "20260911.1");
  assert.equal(summary.ready, false);
  assert.equal(summary.action, null);
});

test("missing managed Homebrew aria2 still requests package installation", () => {
  assert.equal(summarizeTools([aria2c("missing")], "homebrew").action, "install");
  assert.equal(summarizeTools([aria2c("outdated")], "homebrew").action, "update");
  assert.equal(summarizeTools([aria2c("available")], "homebrew").ready, true);
});
