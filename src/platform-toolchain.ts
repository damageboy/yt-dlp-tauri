import type { ToolAction, ToolSummaryMode } from "./toolchain";

export type ManagedProviderKind = "archive-manifest" | "homebrew";
export type ProviderCapabilities = {
  install: boolean;
  update: boolean;
  reinstall: boolean;
};
export type PlatformPresentation = {
  target: string;
  managedProvider: ManagedProviderKind;
  sourceLabels: { managed: string; local: string };
  defaultSource: "managed" | "local";
  executableExtension: string | null;
  capabilities: ProviderCapabilities;
};

export type ExecutablePickerFilter = {
  name: string;
  extensions: string[];
};

export type HomebrewConfirmationKey =
  | "settings.homebrewInstallConfirm"
  | "settings.homebrewUpdateConfirm"
  | "settings.homebrewReinstallConfirm";

export function executablePickerFilters(
  platform: PlatformPresentation,
): ExecutablePickerFilter[] {
  return platform.executableExtension
    ? [{ name: "Executable", extensions: [platform.executableExtension] }]
    : [];
}

export function managedSummaryMode(
  platform: PlatformPresentation,
): Extract<ToolSummaryMode, "managed" | "homebrew"> {
  return platform.managedProvider === "homebrew" ? "homebrew" : "managed";
}

export function managedActionConfirmationKey(
  platform: PlatformPresentation,
  action: ToolAction,
): HomebrewConfirmationKey | null {
  if (platform.managedProvider !== "homebrew") {
    return null;
  }

  if (action === "install") {
    return "settings.homebrewInstallConfirm";
  }
  if (action === "update") {
    return "settings.homebrewUpdateConfirm";
  }
  return "settings.homebrewReinstallConfirm";
}

export function canReinstallManagedTools(
  platform: PlatformPresentation,
  source: "managed" | "local",
  managedProviderMissing: boolean,
): boolean {
  return (
    source === "managed" &&
    platform.capabilities.reinstall &&
    !managedProviderMissing
  );
}

export function showsRevision(platform: PlatformPresentation): boolean {
  return platform.managedProvider === "archive-manifest";
}
