export type ToolStatus = {
  name: string;
  relative_path: string;
  full_path: string;
  availability: "available" | "missing" | "cannot_execute" | "outdated" | "provider_missing" | "configuration_required";
  version?: string;
  expected_version?: string;
  error?: string;
};

export type ToolAction = "install" | "update" | "reinstall";
export type ToolSummaryMode = "managed" | "local" | "remote" | "homebrew";

export type RemoteToolManifest = {
  status: "available" | "no_release" | "no_manifest";
  manifestJson: string | null;
  revision: string | null;
  source: "archive" | "legacy" | null;
};

export type ManagedToolUpdateResult = {
  status: "available" | "no_release" | "no_manifest";
  source: "archive" | "legacy" | "homebrew" | null;
  tools: ToolStatus[];
  manifestJson: string | null;
  remoteRevision: string | null;
};

export type ManagedToolUpdateOutcome =
  | { kind: "no_release" }
  | { kind: "no_manifest" }
  | { kind: "invalid" }
  | {
      kind: "available";
      mode: Extract<ToolSummaryMode, "managed" | "homebrew" | "remote">;
      manifestJson: string | null;
      remoteRevision: string | null;
    };

export type ToolSummary = {
  ready: boolean;
  action: ToolAction | null;
  settingsKey:
    | "aria2c.configurationRequired"
    | "settings.toolsAvailable"
    | "settings.toolsMissing"
    | "settings.toolsDamaged"
    | "settings.localToolsAvailable"
    | "settings.localToolsMissing"
    | "settings.localToolsDamaged"
    | "settings.toolUpdatesAvailable"
    | "settings.homebrewMissing";
  noticeKey:
    | "aria2c.configurationRequired"
    | "notice.toolchainReady"
    | "notice.toolsMissing"
    | "notice.toolsDamaged"
    | "notice.toolsOutdated"
    | "notice.localToolchainReady"
    | "notice.localToolsMissing"
    | "notice.localToolsDamaged"
    | "notice.homebrewMissing";
  eventKey:
    | "aria2c.configurationRequired"
    | "event.toolsAvailable"
    | "event.toolsMissing"
    | "event.toolsDamaged"
    | "event.toolUpdatesAvailable"
    | "event.localToolsAvailable"
    | "event.localToolsMissing"
    | "event.localToolsDamaged"
    | "event.homebrewMissing";
  tone: "success" | "warning";
};

export function managedToolUpdateOutcome(
  result: ManagedToolUpdateResult,
  managedMode: Extract<ToolSummaryMode, "managed" | "homebrew">,
): ManagedToolUpdateOutcome {
  if (result.status === "no_release" || result.status === "no_manifest") {
    return { kind: result.status };
  }

  if (result.source === "archive") {
    return result.manifestJson && result.remoteRevision
      ? {
          kind: "available",
          mode: "remote",
          manifestJson: result.manifestJson,
          remoteRevision: result.remoteRevision,
        }
      : { kind: "invalid" };
  }

  if (result.source === "legacy") {
    return result.manifestJson
      ? {
          kind: "available",
          mode: "remote",
          manifestJson: result.manifestJson,
          remoteRevision: result.remoteRevision,
        }
      : { kind: "invalid" };
  }

  if (result.source === "homebrew") {
    return {
      kind: "available",
      mode: managedMode,
      manifestJson: result.manifestJson,
      remoteRevision: result.remoteRevision,
    };
  }

  return { kind: "invalid" };
}

export function summarizeTools(tools: ToolStatus[], mode: ToolSummaryMode): ToolSummary {
  const hasMissing = tools.some((tool) => tool.availability === "missing");
  const hasOutdated = tools.some((tool) => tool.availability === "outdated");
  const hasAttention = tools.some((tool) => tool.availability === "outdated" || tool.availability === "cannot_execute");
  const ready = tools.length > 0 && tools.every((tool) => tool.availability === "available");

  if (mode === "homebrew" && tools.some((tool) => tool.availability === "provider_missing")) {
    return {
      ready: false,
      action: null,
      settingsKey: "settings.homebrewMissing",
      noticeKey: "notice.homebrewMissing",
      eventKey: "event.homebrewMissing",
      tone: "warning",
    };
  }

  if (tools.some((tool) => tool.availability === "configuration_required")) {
    return {
      ready: false,
      action: null,
      settingsKey: "aria2c.configurationRequired",
      noticeKey: "aria2c.configurationRequired",
      eventKey: "aria2c.configurationRequired",
      tone: "warning",
    };
  }

  if (ready) {
    if (mode === "local") {
      return {
        ready: true,
        action: null,
        settingsKey: "settings.localToolsAvailable",
        noticeKey: "notice.localToolchainReady",
        eventKey: "event.localToolsAvailable",
        tone: "success",
      };
    }
    return {
      ready: true,
      action: null,
      settingsKey: "settings.toolsAvailable",
      noticeKey: "notice.toolchainReady",
      eventKey: "event.toolsAvailable",
      tone: "success",
    };
  }

  if (mode === "local") {
    return hasMissing
      ? {
          ready: false,
          action: null,
          settingsKey: "settings.localToolsMissing",
          noticeKey: "notice.localToolsMissing",
          eventKey: "event.localToolsMissing",
          tone: "warning",
        }
      : {
          ready: false,
          action: null,
          settingsKey: "settings.localToolsDamaged",
          noticeKey: "notice.localToolsDamaged",
          eventKey: "event.localToolsDamaged",
          tone: "warning",
        };
  }

  if (hasMissing) {
    return {
      ready: false,
      action: "install",
      settingsKey: "settings.toolsMissing",
      noticeKey: "notice.toolsMissing",
      eventKey: "event.toolsMissing",
      tone: "warning",
    };
  }

  if ((hasAttention && mode === "remote") || (hasOutdated && mode === "homebrew")) {
    return {
      ready: false,
      action: "update",
      settingsKey: "settings.toolUpdatesAvailable",
      noticeKey: "notice.toolsOutdated",
      eventKey: "event.toolUpdatesAvailable",
      tone: "warning",
    };
  }

  if (hasAttention) {
    return {
      ready: false,
      action: "reinstall",
      settingsKey: "settings.toolsDamaged",
      noticeKey: "notice.toolsDamaged",
      eventKey: "event.toolsDamaged",
      tone: "warning",
    };
  }

  return {
    ready: false,
    action: "install",
    settingsKey: "settings.toolsMissing",
    noticeKey: "notice.toolsMissing",
    eventKey: "event.toolsMissing",
    tone: "warning",
  };
}

export function summarizeRemoteTools(
  tools: ToolStatus[],
  localRevision: string | null,
  remoteRevision: string | null,
): ToolSummary {
  if (remoteRevision) {
    compareToolchainRevisions(remoteRevision, remoteRevision);
  }
  if (localRevision) {
    compareToolchainRevisions(localRevision, localRevision);
  }
  const summary = summarizeTools(tools, "remote");
  if (!summary.ready || !remoteRevision) {
    return summary;
  }
  const newer = localRevision === null || compareToolchainRevisions(remoteRevision, localRevision) > 0;
  if (!newer) {
    return summary;
  }

  return {
    ready: false,
    action: "update",
    settingsKey: "settings.toolUpdatesAvailable",
    noticeKey: "notice.toolsOutdated",
    eventKey: "event.toolUpdatesAvailable",
    tone: "warning",
  };
}

export function compareToolchainRevisions(left: string, right: string): -1 | 0 | 1 {
  const leftParts = parseToolchainRevision(left);
  const rightParts = parseToolchainRevision(right);
  if (leftParts.date !== rightParts.date) {
    return leftParts.date < rightParts.date ? -1 : 1;
  }
  if (leftParts.sequence === rightParts.sequence) {
    return 0;
  }
  return leftParts.sequence < rightParts.sequence ? -1 : 1;
}

function parseToolchainRevision(value: string): { date: string; sequence: bigint } {
  const match = /^(\d{4})(\d{2})(\d{2})\.([1-9]\d*)$/u.exec(value);
  if (!match) {
    throw new Error(`Invalid toolchain revision: ${value}`);
  }
  const [, yearText, monthText, dayText, sequenceText] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysInMonth = [31, leapYear ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (year === 0 || month < 1 || month > 12 || day < 1 || day > daysInMonth[month - 1]) {
    throw new Error(`Invalid toolchain revision: ${value}`);
  }

  return {
    date: `${yearText}${monthText}${dayText}`,
    sequence: parseRevisionSequence(sequenceText, value),
  };
}

function parseRevisionSequence(sequenceText: string, revision: string): bigint {
  const sequence = BigInt(sequenceText);
  if (sequence > 4_294_967_295n) {
    throw new Error(`Invalid toolchain revision: ${revision}`);
  }
  return sequence;
}
