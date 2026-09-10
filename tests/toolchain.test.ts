import assert from "node:assert/strict";
import test from "node:test";

import {
  compareToolchainRevisions,
  managedToolUpdateOutcome,
  summarizeRemoteTools,
  summarizeTools,
  type ManagedToolUpdateResult,
  type ToolStatus,
} from "../src/toolchain.ts";

function tool(availability: ToolStatus["availability"], version = "1.0.0", expectedVersion = "1.0.0"): ToolStatus {
  return {
    name: "yt-dlp",
    relative_path: "yt-dlp/yt-dlp",
    full_path: "/tools/yt-dlp/yt-dlp",
    availability,
    version,
    expected_version: expectedVersion,
  };
}

test("summarizeTools leaves healthy managed toolchains without a primary action", () => {
  assert.deepEqual(summarizeTools([tool("available")], "managed"), {
    ready: true,
    action: null,
    settingsKey: "settings.toolsAvailable",
    noticeKey: "notice.toolchainReady",
    eventKey: "event.toolsAvailable",
    tone: "success",
  });
});

test("summarizeTools asks for reinstall when managed verification finds damaged tools", () => {
  assert.deepEqual(summarizeTools([tool("outdated", "1.0.0", "1.0.0")], "managed"), {
    ready: false,
    action: "reinstall",
    settingsKey: "settings.toolsDamaged",
    noticeKey: "notice.toolsDamaged",
    eventKey: "event.toolsDamaged",
    tone: "warning",
  });
});

test("summarizeTools never offers managed actions for local tools", () => {
  assert.deepEqual(summarizeTools([tool("missing")], "local"), {
    ready: false,
    action: null,
    settingsKey: "settings.localToolsMissing",
    noticeKey: "notice.localToolsMissing",
    eventKey: "event.localToolsMissing",
    tone: "warning",
  });
  assert.deepEqual(summarizeTools([tool("cannot_execute")], "local"), {
    ready: false,
    action: null,
    settingsKey: "settings.localToolsDamaged",
    noticeKey: "notice.localToolsDamaged",
    eventKey: "event.localToolsDamaged",
    tone: "warning",
  });
});

test("summarizeTools asks for update when release manifest verification finds newer tools", () => {
  assert.deepEqual(summarizeTools([tool("outdated", "1.0.0", "2.0.0")], "remote"), {
    ready: false,
    action: "update",
    settingsKey: "settings.toolUpdatesAvailable",
    noticeKey: "notice.toolsOutdated",
    eventKey: "event.toolUpdatesAvailable",
    tone: "warning",
  });
});

test("missing Homebrew keeps Settings usable without offering a formula action", () => {
  assert.deepEqual(summarizeTools([tool("provider_missing")], "homebrew"), {
    ready: false,
    action: null,
    settingsKey: "settings.homebrewMissing",
    noticeKey: "notice.homebrewMissing",
    eventKey: "event.homebrewMissing",
    tone: "warning",
  });
});

test("Homebrew formulas map missing and outdated tools to install and update", () => {
  assert.equal(summarizeTools([tool("missing")], "homebrew").action, "install");
  assert.equal(summarizeTools([tool("outdated")], "homebrew").action, "update");
});

function managedUpdate(
  overrides: Partial<ManagedToolUpdateResult>,
): ManagedToolUpdateResult {
  return {
    status: "available",
    source: "archive",
    tools: [tool("available")],
    manifestJson: "{}",
    remoteRevision: "20260712.1",
    ...overrides,
  };
}

test("managed update outcomes preserve archive no-release and no-manifest results", () => {
  assert.deepEqual(
    managedToolUpdateOutcome(
      managedUpdate({
        status: "no_release",
        source: null,
        manifestJson: null,
        remoteRevision: null,
      }),
      "managed",
    ),
    { kind: "no_release" },
  );
  assert.deepEqual(
    managedToolUpdateOutcome(
      managedUpdate({
        status: "no_manifest",
        source: null,
        manifestJson: null,
        remoteRevision: null,
      }),
      "managed",
    ),
    { kind: "no_manifest" },
  );
});

test("legacy manifests without revisions remain remote and installable", () => {
  const result = managedUpdate({ source: "legacy", remoteRevision: null });

  assert.deepEqual(managedToolUpdateOutcome(result, "managed"), {
    kind: "available",
    mode: "remote",
    manifestJson: "{}",
    remoteRevision: null,
  });
  assert.equal(summarizeRemoteTools([tool("outdated")], null, null).action, "update");
});

test("remote archive revision produces update only when newer", () => {
  assert.equal(compareToolchainRevisions("20260712.1", "20260711.2"), 1);
  assert.equal(compareToolchainRevisions("20260712.1", "20260712.1"), 0);
  assert.equal(compareToolchainRevisions("20260711.2", "20260712.1"), -1);
  assert.throws(() => compareToolchainRevisions("20260229.1", "20260712.1"));
  assert.throws(() => compareToolchainRevisions("20260712.4294967296", "20260712.1"));
});

test("new archive revision updates matching installed bytes", () => {
  assert.equal(
    summarizeRemoteTools([tool("available")], "20260711.2", "20260712.1").action,
    "update",
  );
  assert.equal(
    summarizeRemoteTools([tool("available")], "20260712.1", "20260712.1").action,
    null,
  );
  assert.throws(() =>
    summarizeRemoteTools([tool("missing")], null, "20261301.1"),
  );
});
