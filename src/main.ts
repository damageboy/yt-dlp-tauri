import { Aria2cSettingsDraft, parseParallelConnections, type Aria2cConfig, type Aria2cSettings } from "./aria2c-settings";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import changelogMarkdown from "../CHANGELOG.md?raw";
import packageInfo from "../package.json";
import { releaseNotesForVersion, shouldShowReleaseNotes, stripTerminalSentencePunctuation } from "./release-notes";
import { thumbnailUrlCandidates } from "./thumbnail";
import {
  canReinstallManagedTools,
  executablePickerFilters,
  managedActionConfirmationKey,
  managedSummaryMode,
  showsRevision,
  type PlatformPresentation,
} from "./platform-toolchain";
import {
  managedToolUpdateOutcome,
  summarizeRemoteTools,
  summarizeTools,
  type ManagedToolUpdateResult,
  type ToolAction,
  type ToolStatus,
  type ToolSummaryMode,
} from "./toolchain";
import { type GithubAccessMode, getUpdateStatus, parseGithubHttpError, parseLatestRelease, resolveGithubUrl } from "./update-check";

type VideoFormatOption = {
  label: string;
  format_selector: string;
  height?: number;
  extension: string;
  is_best: boolean;
};

type VideoMetadata = {
  title: string;
  id?: string;
  webpage_url: string;
  thumbnail_url?: string;
  thumbnail_urls?: string[];
  duration_seconds?: number;
  description?: string;
  format_options: VideoFormatOption[];
};

type AppState = {
  aria2c: Aria2cSettings;
  download_directory: string;
  tools_root: string;
  toolchain_revision?: string | null;
  toolchain_source: ToolchainSource;
  platform: PlatformPresentation;
  local_toolchain: LocalToolchainConfig;
  local_toolchain_paths: LocalToolchainPaths;
  cookies_file?: string | null;
};

type ToolchainSource = "managed" | "local";

type LocalToolchainConfig = {
  schemaVersion: number;
  ytDlpPath?: string | null;
  ffmpegDirectory?: string | null;
  denoPath?: string | null;
};

type LocalToolchainPaths = Omit<LocalToolchainConfig, "schemaVersion">;

type DownloadProgress = {
  percent?: number;
  status: string;
  speed?: string;
  eta?: string;
  raw?: string;
};

type ToolInstallProgress = {
  percent?: number;
  status: string;
  tool?: string;
};

const APP_VERSION = packageInfo.version;
const PROJECT_REPOSITORY_URL = "https://github.com/damageboy/yt-dlp-tauri";
const PROJECT_RELEASES_URL = `${PROJECT_REPOSITORY_URL}/releases`;
const LATEST_RELEASE_API_URL = "https://api.github.com/repos/damageboy/yt-dlp-tauri/releases/latest";
const GITHUB_ACCESS_STORAGE_KEY = "yt-dlp-tauri-github-access-mode";
const RELEASE_NOTES_SEEN_VERSION_STORAGE_KEY = "yt-dlp-tauri-release-notes-seen-version";
const HOMEBREW_URL = "https://brew.sh/";
const WINDOWS_PLATFORM: PlatformPresentation = {
  target: "win-x64",
  managedProvider: "archive-manifest",
  sourceLabels: { managed: "Managed", local: "Local" },
  defaultSource: "managed",
  executableExtension: "exe",
  capabilities: { install: true, update: true, reinstall: true },
};
const MAX_TOASTS = 4;
const TOAST_AUTO_DISMISS_MS: Record<NoticeTone, number> = {
  success: 6000,
  warning: 8000,
  error: 0,
};

const messages = {
  "aria2c.configurationRequired": "aria2c is required. In Toolchain, choose a working aria2c executable or Use PATH.",
  "aria2c.enabled": "Use aria2c",
  "aria2c.parallel": "Parallelism",
  "aria2c.parallelHint": "Sets simultaneous HLS/DASH segment downloads. When aria2c is used, also sets its concurrent items, connections per server per item, and splits. This is not the number of simultaneous videos.",
  "aria2c.save": "Save",
  "aria2c.saved": "aria2c settings saved",
  "aria2c.saveFailed": "Could not save aria2c settings: {message}",
  "aria2c.invalid": "Enter a whole number from 1 to 16",
  "aria2c.loadFailed": "Could not load saved settings; aria2c defaults to disabled",
  "aria2c.choose": "Choose aria2c executable",
  "app.title": "yt-dlp-tauri",
  "app.eyebrow": "Desktop downloader",
  "app.heading": "Paste, choose, download.",
  "notifications.label": "Notifications",
  "action.settings": "Settings",
  "action.close": "Close",
  "action.done": "Done",
  "action.dismissNotification": "Dismiss notification",
  "action.parse": "Parse",
  "action.download": "Download",
  "action.cancel": "Cancel",
  "action.openFolder": "Open folder",
  "action.browse": "Browse",
  "action.save": "Save",
  "action.reset": "Reset",
  "action.chooseCookies": "Choose Cookie file",
  "action.clearCookies": "Clear",
  "action.verifyTools": "Verify tools",
  "action.checkToolUpdates": "Check tool updates",
  "action.installTools": "Install tools",
  "action.updateTools": "Update tools",
  "action.reinstallTools": "Reinstall tools",
  "action.choosePath": "Choose",
  "action.chooseYtDlp": "Choose yt-dlp",
  "action.chooseFfmpegDirectory": "Choose FFmpeg directory",
  "action.chooseDeno": "Choose Deno",
  "action.usePath": "Use PATH",
  "action.checkUpdates": "Check updates",
  "action.openRelease": "Open release",
  "action.releaseNotes": "Release notes",
  "action.projectHome": "Project home",
  "action.openHomebrew": "Open Homebrew website",
  "github.accessLabel": "GitHub access mode",
  "github.direct": "Direct",
  "github.proxy": "gh-proxy",
  "url.label": "Video URL",
  "url.placeholder": "https://www.youtube.com/watch?v=...",
  "cookies.label": "Cookie file",
  "cookies.none": "No cookies",
  "cookies.chooseFile": "Choose Cookie file",
  "preview.thumbnailAlt": "video thumbnail",
  "preview.emptyImage": "Preview",
  "preview.label": "Preview",
  "preview.noVideo": "No video parsed",
  "preview.emptyStart": "Paste a video URL to inspect title, cover, duration, and qualities.",
  "preview.emptyChanged": "Paste a URL and parse it before downloading.",
  "preview.cookiesChanged": "Cookie file changed. Parse again before downloading.",
  "preview.toolsChanged": "Tool source changed. Parse again before downloading.",
  "preview.readingMetadata": "Reading metadata from yt-dlp...",
  "preview.parseFailed": "Metadata parsing failed. Check the URL and tools.",
  "preview.noDescription": "No description returned by yt-dlp.",
  "download.format": "Format",
  "download.quality": "Quality",
  "progress.idle": "Idle",
  "progress.parsing": "Parsing video metadata...",
  "progress.metadataReady": "Metadata parsed. Choose a quality, then download.",
  "progress.metadataFailed": "Metadata parsing failed.",
  "progress.startingDownload": "Starting {quality} download...",
  "progress.savedTo": "Saved to {path}",
  "progress.completedOpenFolder": "Download completed. Open the folder to view the file.",
  "progress.downloadCancelled": "Download cancelled.",
  "progress.downloadFailed": "Download failed.",
  "progress.cancelling": "Cancelling download...",
  "progress.eta": "ETA",
  "notice.checkingTools": "Checking tools...",
  "notice.toolchainReady": "Toolchain ready.",
  "notice.toolsMissing": "Some tools are missing.",
  "notice.toolsOutdated": "Toolchain update available.",
  "notice.toolsDamaged": "Toolchain needs reinstall.",
  "notice.localToolchainReady": "Local toolchain ready.",
  "notice.localToolsMissing": "Some local tools are missing.",
  "notice.localToolsDamaged": "Local toolchain verification failed.",
  "notice.homebrewMissing": "Homebrew is not installed. Install it from brew.sh, then verify tools again.",
  "notice.toolCheckFailed": "Tool check failed.",
  "notice.toolsInstalled": "Toolchain installed.",
  "notice.toolInstallNeedsAttention": "Tool install needs attention.",
  "notice.toolInstallFailed": "Tool install failed.",
  "notice.metadataParsed": "Metadata parsed.",
  "notice.downloadCompleted": "Download completed.",
  "notice.downloadCancelled": "Download cancelled.",
  "notice.folderUpdated": "Download folder updated.",
  "notice.folderReset": "Download folder reset.",
  "notice.cookiesUpdated": "Cookie file updated.",
  "notice.cookiesCleared": "Cookie file cleared.",
  "updates.checking": "Checking GitHub releases...",
  "updates.available": "New version available: {version}",
  "updates.current": "You are up to date.",
  "updates.noRelease": "No GitHub release found yet.",
  "updates.invalidRelease": "GitHub returned an unreadable release.",
  "updates.failed": "Could not check updates: {message}",
  "updates.rateLimited": "GitHub API rate limit reached. Try again after {time}, or switch GitHub access mode.",
  "updates.later": "later",
  "releaseNotes.kicker": "Updated",
  "releaseNotes.title": "What's new",
  "releaseNotes.version": "Version {version}",
  "releaseNotes.empty": "No release notes found for this version.",
  "settings.kicker": "Preferences",
  "settings.title": "Settings",
  "settings.outputFolder": "Output folder",
  "settings.resolvingFolder": "Resolving download folder...",
  "settings.toolchain": "Toolchain",
  "settings.toolchainHint": "Per-target tools are verified with SHA-256.",
  "settings.localToolchainHint": "Local executables are verified by behavior and remain user-managed.",
  "settings.homebrewToolchainHint": "Required tools are managed with Homebrew.",
  "settings.homebrewGuidance": "Homebrew manages the yt-dlp, ffmpeg, deno, and aria2 formulas.",
  "settings.homebrewMissing": "Homebrew is not installed. Install it from brew.sh, then verify tools again.",
  "settings.homebrewPrefix": "Homebrew prefix: {path}",
  "settings.homebrewPrefixPending": "Homebrew prefix not found",
  "settings.toolSource": "Tool source",
  "settings.managedTools": "Managed",
  "settings.localTools": "Local",
  "settings.activeRevision": "Active revision",
  "settings.noActiveRevision": "None",
  "settings.resolvingTools": "Resolving tools path...",
  "settings.installMissing": "Install missing tools automatically.",
  "settings.installingTools": "Installing missing tools...",
  "settings.updatingTools": "Updating tools to pinned versions...",
  "settings.reinstallingTools": "Reinstalling managed tools...",
  "settings.toolsPathPending": "Tools path not resolved yet",
  "settings.toolsChecking": "Checking tools...",
  "settings.toolsAvailable": "All required tools are available.",
  "settings.toolsMissing": "Missing tools can be installed automatically.",
  "settings.toolsDamaged": "Some tools are missing, damaged, or do not match the active manifest.",
  "settings.localPathNotDetected": "Not detected",
  "settings.detectingLocalTools": "Detecting local tools from PATH...",
  "settings.usePathHint": "Clear selected paths and resolve all local tools from the current PATH.",
  "settings.localToolsAvailable": "Local yt-dlp, FFmpeg, FFprobe, Deno and aria2c passed verification.",
  "settings.localToolsMissing": "Choose missing local paths or use the current PATH.",
  "settings.localToolsDamaged": "One or more local tools failed version or compatibility checks.",
  "settings.toolSourceFailed": "Could not change tool source: {message}",
  "settings.localToolSaveFailed": "Could not save local tool paths: {message}",
  "settings.localToolDetectFailed": "Could not detect local tools: {message}",
  "settings.toolUpdatesChecking": "Checking the latest released tool manifest...",
  "settings.toolUpdatesAvailable": "A released toolchain update is available.",
  "settings.toolUpdatesCurrent": "Tools match the latest released manifest.",
  "settings.toolUpdatesNoManifest": "The latest release does not include a tool manifest yet.",
  "settings.toolUpdatesInvalidManifest": "The released tool manifest could not be read.",
  "settings.toolUpdatesFailed": "Tool update check failed: {message}",
  "settings.reinstallConfirm": "Download and verify a fresh toolchain at {path}? The current revision stays active until the replacement passes every check",
  "settings.homebrewInstallConfirm": "Install Homebrew formulas yt-dlp, ffmpeg, deno, and aria2?",
  "settings.homebrewUpdateConfirm": "Update Homebrew formulas yt-dlp, ffmpeg, deno, and aria2?",
  "settings.homebrewReinstallConfirm": "Reinstall Homebrew formulas yt-dlp, ffmpeg, deno, and aria2?",
  "settings.toolCheckFailed": "Tool check failed.",
  "settings.toolsInstalled": "Toolchain installed.",
  "settings.toolsInstallPartial": "Install finished, but some tools still need attention.",
  "settings.toolInstallFailed": "Tool install failed.",
  "settings.activity": "Activity",
  "settings.activityHint": "Recent local events.",
  "settings.version": "Version",
  "settings.githubSite": "GitHub site",
  "settings.chooseFolder": "Choose download folder",
  "tool.currentUnknown": "unknown",
  "event.booted": "App booted.",
  "event.toolsAvailable": "yt-dlp, ffmpeg, ffprobe, deno and aria2c are available.",
  "event.toolsMissing": "Tool check found missing tools.",
  "event.toolsDamaged": "Tool check found tools that need reinstall.",
  "event.localToolsAvailable": "Local toolchain passed verification.",
  "event.localToolsMissing": "Local toolchain has missing paths.",
  "event.localToolsDamaged": "Local toolchain failed verification.",
  "event.homebrewMissing": "Homebrew was not found.",
  "event.localToolsSelected": "Local tool source selected.",
  "event.managedToolsSelected": "Managed tool source selected.",
  "event.toolUpdatesAvailable": "Released toolchain update found.",
  "event.toolUpdatesCurrent": "Tools match the latest released manifest.",
  "event.toolsInstalled": "Toolchain installed.",
  "event.toolsPartial": "Tool install completed with missing tools.",
  "event.toolInstallFailed": "Tool install failed.",
  "event.parsed": "Parsed {title}",
  "event.metadataFailed": "Metadata parsing failed.",
  "event.saved": "Saved {path}",
  "event.downloadCompleted": "Download completed.",
  "event.downloadCancelled": "Download cancelled.",
  "event.downloadFailed": "Download failed.",
  "event.cancelRequested": "Cancel requested.",
  "event.cookiesUpdated": "Cookie file selected: {file}",
  "event.cookiesCleared": "Cookie file cleared.",
} as const;

type MessageKey = keyof typeof messages;
type NoticeTone = "success" | "warning" | "error";
type UpdateTone = "neutral" | "success" | "warning" | "error";

const state = {
  aria2c: new Aria2cSettingsDraft(),
  aria2cResolvedPath: null as string | null,
  aria2cMessage: null as { key: MessageKey; detail?: string } | null,
  metadata: null as VideoMetadata | null,
  selectedFormat: null as VideoFormatOption | null,
  busy: false,
  activeOperation: null as "metadata" | "download" | "tools" | null,
  cancelRequested: false,
  lastUrl: "",
  toolsReady: false,
  toolAction: null as ToolAction | null,
  toolchainRevision: null as string | null,
  toolchainSource: "managed" as ToolchainSource,
  toolsRoot: "",
  platform: WINDOWS_PLATFORM,
  managedProviderMissing: false,
  localToolchain: {
    schemaVersion: 1,
    ytDlpPath: null,
    ffmpegDirectory: null,
    denoPath: null,
  } as LocalToolchainConfig,
  localToolchainPaths: {
    ytDlpPath: null,
    ffmpegDirectory: null,
    denoPath: null,
  } as LocalToolchainPaths,
  pendingToolManifestJson: null as string | null,
  updateChecking: false,
  latestReleaseUrl: "",
  updateStatus: null as { key: MessageKey; values: Record<string, string | number>; tone: UpdateTone } | null,
  githubAccessMode: resolveInitialGithubAccessMode(),
  cookiesFile: null as string | null,
  releaseNotesOpen: false,
  thumbnailCandidates: [] as string[],
  thumbnailCandidateIndex: 0,
};

let releaseNotesReturnFocus: HTMLElement | null = null;
const toastTimers = new Map<HTMLElement, number>();

const elements = {
  aria2cEnabled: must<HTMLInputElement>("#aria2c-enabled"),
  aria2cToolPaths: must<HTMLElement>("#aria2c-tool-paths"),
  localAria2cPath: must<HTMLElement>("#local-aria2c-path"),
  aria2cParallel: must<HTMLInputElement>("#aria2c-parallel"),
  aria2cStatus: must<HTMLElement>("#aria2c-status"),
  chooseAria2c: must<HTMLButtonElement>("#choose-local-aria2c"),
  saveAria2c: must<HTMLButtonElement>("#save-aria2c"),
  url: must<HTMLInputElement>("#url"),
  parse: must<HTMLButtonElement>("#parse"),
  download: must<HTMLButtonElement>("#download"),
  cancel: must<HTMLButtonElement>("#cancel"),
  openFolder: must<HTMLButtonElement>("#open-folder"),
  chooseCookies: must<HTMLButtonElement>("#choose-cookies"),
  clearCookies: must<HTMLButtonElement>("#clear-cookies"),
  settingsToggle: must<HTMLButtonElement>("#settings-toggle"),
  settingsClose: must<HTMLButtonElement>("#settings-close"),
  settingsBackdrop: must<HTMLElement>("#settings-backdrop"),
  settingsDrawer: must<HTMLElement>("#settings-drawer"),
  verifyTools: must<HTMLButtonElement>("#verify-tools"),
  toolSourceManaged: must<HTMLButtonElement>("#tool-source-managed"),
  toolSourceLocal: must<HTMLButtonElement>("#tool-source-local"),
  managedToolchainDetails: must<HTMLElement>("#managed-toolchain-details"),
  managedProviderGuidance: must<HTMLElement>("#managed-provider-guidance"),
  homebrewHelp: must<HTMLButtonElement>("#homebrew-help"),
  toolchainRevisionRow: must<HTMLElement>("#toolchain-revision-row"),
  localToolchainPaths: must<HTMLElement>("#local-toolchain-paths"),
  localYtDlpPath: must<HTMLElement>("#local-yt-dlp-path"),
  localFfmpegPath: must<HTMLElement>("#local-ffmpeg-path"),
  localDenoPath: must<HTMLElement>("#local-deno-path"),
  chooseLocalYtDlp: must<HTMLButtonElement>("#choose-local-yt-dlp"),
  chooseLocalFfmpeg: must<HTMLButtonElement>("#choose-local-ffmpeg"),
  chooseLocalDeno: must<HTMLButtonElement>("#choose-local-deno"),
  autoDetectLocalTools: must<HTMLButtonElement>("#auto-detect-local-tools"),
  checkToolUpdates: must<HTMLButtonElement>("#check-tool-updates"),
  installTools: must<HTMLButtonElement>("#install-tools"),
  reinstallTools: must<HTMLButtonElement>("#reinstall-tools"),
  browseFolder: must<HTMLButtonElement>("#browse-folder"),
  resetFolder: must<HTMLButtonElement>("#reset-folder"),
  saveFolder: must<HTMLButtonElement>("#save-folder"),
  checkUpdates: must<HTMLButtonElement>("#check-updates"),
  releaseLink: must<HTMLButtonElement>("#release-link"),
  releaseNotesButton: must<HTMLButtonElement>("#release-notes-button"),
  githubLink: must<HTMLButtonElement>("#github-link"),
  githubDirect: must<HTMLButtonElement>("#github-direct"),
  githubProxy: must<HTMLButtonElement>("#github-proxy"),
  releaseNotesBackdrop: must<HTMLElement>("#release-notes-backdrop"),
  releaseNotesDialog: must<HTMLElement>("#release-notes-dialog"),
  releaseNotesClose: must<HTMLButtonElement>("#release-notes-close"),
  releaseNotesDone: must<HTMLButtonElement>("#release-notes-done"),
  releaseNotesVersion: must<HTMLElement>("#release-notes-version"),
  releaseNotesList: must<HTMLElement>("#release-notes-list"),
  appVersion: must<HTMLElement>("#app-version"),
  updateStatus: must<HTMLElement>("#update-status"),
  folderInput: must<HTMLInputElement>("#folder-input"),
  folderText: must<HTMLElement>("#folder-text"),
  cookiesFile: must<HTMLElement>("#cookies-file"),
  toolRoot: must<HTMLElement>("#tool-root"),
  toolchainHint: must<HTMLElement>("#toolchain-hint"),
  toolchainRevision: must<HTMLElement>("#toolchain-revision"),
  toolList: must<HTMLElement>("#tool-list"),
  toolInstallStatus: must<HTMLElement>("#tool-install-status"),
  title: must<HTMLElement>("#video-title"),
  details: must<HTMLElement>("#video-details"),
  description: must<HTMLElement>("#video-description"),
  thumbnail: must<HTMLImageElement>("#thumbnail"),
  thumbnailEmpty: must<HTMLElement>("#thumbnail-empty"),
  outputFormat: must<HTMLSelectElement>("#output-format"),
  quality: must<HTMLSelectElement>("#quality"),
  progress: must<HTMLProgressElement>("#progress"),
  progressText: must<HTMLElement>("#progress-text"),
  events: must<HTMLElement>("#events"),
  toastRegion: must<HTMLElement>("#toast-region"),
};

window.addEventListener("DOMContentLoaded", () => {
  bindEvents();
  applyMessages();
  listen<DownloadProgress>("download-progress", (event) => updateDownloadProgress(event.payload));
  listen<ToolInstallProgress>("tool-install-progress", (event) => updateToolInstallProgress(event.payload));
  void bootstrap();
});

function must<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) {
    throw new Error(`Missing element: ${selector}`);
  }
  return element;
}

function resolveInitialGithubAccessMode(): GithubAccessMode {
  return localStorage.getItem(GITHUB_ACCESS_STORAGE_KEY) === "gh-proxy" ? "gh-proxy" : "direct";
}

function t(key: MessageKey, values: Record<string, string | number> = {}) {
  let text: string = messages[key] || key;
  for (const [name, value] of Object.entries(values)) {
    text = text.split(`{${name}}`).join(String(value));
  }
  return stripTerminalSentencePunctuation(text);
}

function applyMessages() {
  renderAria2c();
  document.documentElement.lang = "en";
  document.title = t("app.title");

  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((element) => {
    const key = element.dataset.i18n as MessageKey | undefined;
    if (key) {
      element.textContent = t(key);
    }
  });

  document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("[data-i18n-placeholder]").forEach((element) => {
    const key = element.dataset.i18nPlaceholder as MessageKey | undefined;
    if (key) {
      element.placeholder = t(key);
    }
  });

  document.querySelectorAll<HTMLElement>("[data-i18n-aria-label]").forEach((element) => {
    const key = element.dataset.i18nAriaLabel as MessageKey | undefined;
    if (key) {
      element.setAttribute("aria-label", t(key));
    }
  });

  document.querySelectorAll<HTMLImageElement>("[data-i18n-alt]").forEach((element) => {
    const key = element.dataset.i18nAlt as MessageKey | undefined;
    if (key) {
      element.alt = t(key);
    }
  });

  elements.appVersion.textContent = APP_VERSION;
  if (state.updateStatus) {
    renderUpdateStatus(t(state.updateStatus.key, state.updateStatus.values), state.updateStatus.tone);
  }
  renderCookiesFile(state.cookiesFile);
  renderToolchainRevision();
  renderToolchainSource();
  renderLocalToolchainPaths();
  updateGithubAccessButtons();
  updateToolActionButton();
  if (state.releaseNotesOpen) {
    renderReleaseNotes();
  }
}

function setGithubAccessMode(accessMode: GithubAccessMode) {
  state.githubAccessMode = accessMode;
  localStorage.setItem(GITHUB_ACCESS_STORAGE_KEY, accessMode);
  clearUpdateStatus();
  updateGithubAccessButtons();
  updateButtons();
}

function setSettingsOpen(isOpen: boolean) {
  elements.settingsDrawer.hidden = !isOpen;
  elements.settingsBackdrop.hidden = !isOpen;
  elements.settingsDrawer.setAttribute("aria-hidden", String(!isOpen));
  document.body.classList.toggle("settings-open", isOpen);

  if (isOpen) {
    elements.settingsClose.focus();
  } else {
    elements.settingsToggle.focus();
  }
}

function maybeShowReleaseNotesAfterUpdate() {
  const seenVersion = localStorage.getItem(RELEASE_NOTES_SEEN_VERSION_STORAGE_KEY);
  if (!seenVersion) {
    localStorage.setItem(RELEASE_NOTES_SEEN_VERSION_STORAGE_KEY, APP_VERSION);
    return;
  }

  if (shouldShowReleaseNotes(seenVersion, APP_VERSION)) {
    showReleaseNotes();
  }
}

function showReleaseNotes() {
  releaseNotesReturnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  renderReleaseNotes();
  setReleaseNotesOpen(true);
}

function closeReleaseNotes() {
  localStorage.setItem(RELEASE_NOTES_SEEN_VERSION_STORAGE_KEY, APP_VERSION);
  setReleaseNotesOpen(false);
}

function setReleaseNotesOpen(isOpen: boolean) {
  state.releaseNotesOpen = isOpen;
  elements.releaseNotesDialog.hidden = !isOpen;
  elements.releaseNotesBackdrop.hidden = !isOpen;
  elements.releaseNotesDialog.setAttribute("aria-hidden", String(!isOpen));
  document.body.classList.toggle("modal-open", isOpen);

  if (isOpen) {
    elements.releaseNotesClose.focus();
    return;
  }

  releaseNotesReturnFocus?.focus();
  releaseNotesReturnFocus = null;
}

function renderReleaseNotes() {
  const notes = releaseNotesForVersion(changelogMarkdown, APP_VERSION);
  const items = notes?.items.length ? notes.items : [t("releaseNotes.empty")];

  elements.releaseNotesVersion.textContent = t("releaseNotes.version", { version: `v${APP_VERSION}` });
  elements.releaseNotesList.replaceChildren(
    ...items.map((item) => {
      const row = document.createElement("li");
      row.textContent = stripTerminalSentencePunctuation(item);
      return row;
    }),
  );
}

function bindEvents() {
  elements.aria2cEnabled.addEventListener("change", () => editAria2c({ enabled: elements.aria2cEnabled.checked }));
  elements.aria2cParallel.addEventListener("input", () => editAria2c({}));
  elements.chooseAria2c.addEventListener("click", () => void chooseAria2c());
  elements.saveAria2c.addEventListener("click", () => void saveAria2c());
  elements.parse.addEventListener("click", () => void parseCurrentUrl());
  elements.download.addEventListener("click", () => void downloadCurrentVideo());
  elements.cancel.addEventListener("click", () => void cancelCurrentDownload());
  elements.chooseCookies.addEventListener("click", () => void chooseCookiesFile());
  elements.clearCookies.addEventListener("click", () => void clearCookiesFile());
  elements.settingsToggle.addEventListener("click", () => setSettingsOpen(true));
  elements.settingsClose.addEventListener("click", () => setSettingsOpen(false));
  elements.settingsBackdrop.addEventListener("click", () => setSettingsOpen(false));
  elements.toolSourceManaged.addEventListener("click", () => void setToolchainSource("managed"));
  elements.toolSourceLocal.addEventListener("click", () => void setToolchainSource("local"));
  elements.chooseLocalYtDlp.addEventListener("click", () => void chooseLocalTool("yt-dlp"));
  elements.chooseLocalFfmpeg.addEventListener("click", () => void chooseLocalTool("ffmpeg"));
  elements.chooseLocalDeno.addEventListener("click", () => void chooseLocalTool("deno"));
  elements.autoDetectLocalTools.addEventListener("click", () => void autoDetectLocalTools());
  elements.verifyTools.addEventListener("click", () => void verifyTools());
  elements.checkToolUpdates.addEventListener("click", () => void checkToolUpdates());
  elements.installTools.addEventListener("click", () => void installTools());
  elements.reinstallTools.addEventListener("click", () => void reinstallTools());
  elements.openFolder.addEventListener("click", () => void openDownloadFolder());
  elements.browseFolder.addEventListener("click", () => void browseDownloadFolder());
  elements.saveFolder.addEventListener("click", () => void saveDownloadFolder());
  elements.resetFolder.addEventListener("click", () => void resetDownloadFolder());
  elements.checkUpdates.addEventListener("click", () => void checkForUpdates());
  elements.releaseLink.addEventListener("click", () => void openLatestRelease());
  elements.releaseNotesButton.addEventListener("click", () => showReleaseNotes());
  elements.githubLink.addEventListener("click", () => void openProjectRepository());
  elements.homebrewHelp.addEventListener("click", () => void openHomebrewWebsite());
  elements.githubDirect.addEventListener("click", () => setGithubAccessMode("direct"));
  elements.githubProxy.addEventListener("click", () => setGithubAccessMode("gh-proxy"));
  elements.thumbnail.addEventListener("load", () => showLoadedThumbnail());
  elements.thumbnail.addEventListener("error", () => loadNextThumbnailCandidate());
  elements.releaseNotesClose.addEventListener("click", () => closeReleaseNotes());
  elements.releaseNotesDone.addEventListener("click", () => closeReleaseNotes());
  elements.releaseNotesBackdrop.addEventListener("click", () => closeReleaseNotes());
  elements.quality.addEventListener("change", () => {
    state.selectedFormat = state.metadata?.format_options[elements.quality.selectedIndex] ?? null;
    updateButtons();
  });
  elements.url.addEventListener("input", () => {
    if (elements.url.value.trim() !== state.lastUrl) {
      invalidateParsedVideo(t("preview.emptyChanged"));
    }
    updateButtons();
  });
  elements.url.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      void parseCurrentUrl();
    }
  });
  window.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") {
      return;
    }

    if (state.releaseNotesOpen) {
      closeReleaseNotes();
      return;
    }

    if (!elements.settingsDrawer.hidden) {
      setSettingsOpen(false);
      return;
    }

    const latestToast = elements.toastRegion.firstElementChild;
    if (latestToast instanceof HTMLElement) {
      dismissToast(latestToast);
    }
  });
}

async function bootstrap() {
  elements.progressText.textContent = t("progress.idle");
  renderEmptyPreview(t("preview.emptyStart"));
  logEvent(t("event.booted"));
  await loadAppState();
  maybeShowReleaseNotesAfterUpdate();
  await verifyTools({ quietReady: true });
}

function editAria2c(patch: Partial<Aria2cConfig>) {
  if (state.busy) return;
  state.aria2c.edit(patch, elements.aria2cParallel.value);
  state.aria2cMessage = null;
  renderAria2c();
}

async function chooseAria2c() {
  if (state.busy) return;
  try {
    const filters = executablePickerFilters(state.platform);
    const selected = await open({ title: t("aria2c.choose"), directory: false, multiple: false, ...(filters.length ? { filters } : {}) });
    if (typeof selected !== "string") return;
    state.aria2cMessage = null;
    setBusy(true);
    try {
      await state.aria2c.saveExecutablePath(selected, config => invoke<Aria2cSettings>("save_aria2c_config", { config }));
    } finally {
      setBusy(false);
    }
    await verifyTools({ quietReady: true });
  } catch (error) {
    state.aria2cMessage = { key: "aria2c.saveFailed", detail: String(error) };
    renderAria2c();
  }
}

async function saveAria2c() {
  if (state.busy || parseParallelConnections(state.aria2c.parallelInput) === null) return;
  state.aria2cMessage = null;
  let saved = false;
  setBusy(true);
  try {
    await state.aria2c.save(config => invoke<Aria2cSettings>("save_aria2c_config", { config }));
    state.aria2cMessage = { key: "aria2c.saved" };
    saved = true;
  } catch (error) {
    state.aria2cMessage = { key: "aria2c.saveFailed", detail: String(error) };
  } finally {
    setBusy(false);
  }
  if (saved) await verifyTools({ quietReady: true });
}

function renderAria2c() {
  const draft = state.aria2c;
  const invalid = parseParallelConnections(draft.parallelInput) === null;
  elements.aria2cEnabled.checked = draft.config.enabled;
  if (elements.aria2cParallel.value !== draft.parallelInput) elements.aria2cParallel.value = draft.parallelInput;
  elements.aria2cParallel.setAttribute("aria-invalid", String(invalid));
  renderLocalToolPath(elements.localAria2cPath, draft.config.executablePath ?? state.aria2cResolvedPath);
  let message = invalid ? t("aria2c.invalid") : state.aria2cMessage
    ? t(state.aria2cMessage.key, { message: state.aria2cMessage.detail ?? "" }) : "";
  if (draft.saved.loadError) message = t("aria2c.loadFailed") + " · " + draft.saved.loadError + " · " + message;
  elements.aria2cStatus.textContent = message;
  elements.aria2cStatus.hidden = !message;
  elements.aria2cStatus.classList.toggle("is-error", invalid || !!draft.saved.loadError || state.aria2cMessage?.key === "aria2c.saveFailed");
  for (const control of [elements.aria2cEnabled, elements.aria2cParallel, elements.chooseAria2c, elements.saveAria2c]) control.disabled = state.busy;
  elements.saveAria2c.disabled ||= invalid;
}

async function loadAppState() {
  const appState = await invoke<AppState>("get_app_state");
  applyAppState(appState);
}

function applyAppState(appState: AppState) {
  if (appState.aria2c) state.aria2c.applySaved(appState.aria2c);
  renderAria2c();
  elements.folderText.textContent = appState.download_directory;
  elements.folderInput.value = appState.download_directory;
  state.toolchainRevision = appState.toolchain_revision ?? null;
  state.toolchainSource = appState.toolchain_source;
  state.toolsRoot = appState.tools_root;
  state.platform = appState.platform;
  state.localToolchain = appState.local_toolchain;
  state.localToolchainPaths = appState.local_toolchain_paths;
  renderToolchainRevision();
  renderToolchainSource();
  renderLocalToolchainPaths();
  renderCookiesFile(appState.cookies_file ?? null);
}

async function setToolchainSource(source: ToolchainSource) {
  if (state.busy || source === state.toolchainSource) {
    return;
  }

  const previousSource = state.toolchainSource;
  let changed = false;
  setBusy(true, undefined, "tools");
  try {
    const appState = await invoke<AppState>("set_toolchain_source", { source });
    state.toolsReady = false;
    state.toolAction = null;
    state.pendingToolManifestJson = null;
    state.managedProviderMissing = false;
    elements.toolList.replaceChildren();
    applyAppState(appState);
    invalidateParsedVideo(t("preview.toolsChanged"));
    logEvent(t(source === "local" ? "event.localToolsSelected" : "event.managedToolsSelected"));
    changed = true;
  } catch (error) {
    const message = String(error);
    state.toolchainSource = previousSource;
    renderToolchainSource();
    showNotice(t("settings.toolSourceFailed", { message }), "error");
  } finally {
    setBusy(false);
  }

  if (changed) {
    await verifyTools();
  }
}

async function chooseLocalTool(tool: "yt-dlp" | "ffmpeg" | "deno") {
  if (state.busy || state.toolchainSource !== "local") {
    return;
  }

  const filters = executablePickerFilters(state.platform);
  const selected = await open({
    multiple: false,
    directory: tool === "ffmpeg",
    ...(tool === "ffmpeg" || filters.length === 0 ? {} : { filters }),
  });
  if (typeof selected !== "string") {
    return;
  }

  const config = { ...state.localToolchain };
  if (tool === "yt-dlp") {
    config.ytDlpPath = selected;
  } else if (tool === "ffmpeg") {
    config.ffmpegDirectory = selected;
  } else {
    config.denoPath = selected;
  }
  await saveLocalToolchain(config);
}

async function saveLocalToolchain(config: LocalToolchainConfig) {
  let saved = false;
  setBusy(true, undefined, "tools");
  try {
    const appState = await invoke<AppState>("set_local_toolchain", {
      config: {
        ytDlpPath: config.ytDlpPath ?? null,
        ffmpegDirectory: config.ffmpegDirectory ?? null,
        denoPath: config.denoPath ?? null,
      },
    });
    state.toolsReady = false;
    applyAppState(appState);
    invalidateParsedVideo(t("preview.toolsChanged"));
    saved = true;
  } catch (error) {
    showNotice(t("settings.localToolSaveFailed", { message: String(error) }), "error");
  } finally {
    setBusy(false);
  }

  if (saved) {
    await verifyTools();
  }
}

async function autoDetectLocalTools() {
  if (state.busy) {
    return;
  }

  let detected = false;
  setBusy(true, undefined, "tools");
  elements.toolInstallStatus.textContent = t("settings.detectingLocalTools");
  try {
    await state.aria2c.saveExecutablePath(null, config => invoke<Aria2cSettings>("save_aria2c_config", { config }));
    const appState = await invoke<AppState>(state.toolchainSource === "local" ? "auto_detect_local_toolchain" : "get_app_state");
    state.toolsReady = false;
    applyAppState(appState);
    invalidateParsedVideo(t("preview.toolsChanged"));
    detected = true;
  } catch (error) {
    showNotice(t("settings.localToolDetectFailed", { message: String(error) }), "error");
  } finally {
    setBusy(false);
  }

  if (detected) {
    await verifyTools();
  }
}

async function verifyTools(options: { quietReady?: boolean } = {}) {
  setBusy(true, undefined, "tools");
  state.pendingToolManifestJson = null;
  elements.toolInstallStatus.textContent = t("settings.toolsChecking");
  try {
    const tools = await invoke<ToolStatus[]>("check_tools");
    applyToolSummary(
      tools,
      state.toolchainSource === "local" ? "local" : managedSummaryMode(state.platform),
      options,
    );
  } catch (error) {
    state.toolsReady = false;
    state.toolAction = state.toolchainSource === "managed" ? "install" : null;
    const message = String(error);
    elements.toolInstallStatus.textContent = message || t("settings.toolCheckFailed");
    showNotice(message || t("settings.toolCheckFailed"), "error");
    updateToolActionButton();
  } finally {
    setBusy(false);
  }
}

async function installTools() {
  if (state.busy || state.toolchainSource !== "managed" || !state.toolAction) {
    return;
  }

  if (state.toolAction === "reinstall") {
    await reinstallTools();
    return;
  }

  const confirmationKey = managedActionConfirmationKey(state.platform, state.toolAction);
  if (confirmationKey && !window.confirm(t(confirmationKey))) {
    return;
  }

  setBusy(true, undefined, "tools");
  elements.toolInstallStatus.textContent = t(toolActionStatusKey(state.toolAction));
  try {
    const tools = state.pendingToolManifestJson
      ? await invoke<ToolStatus[]>("install_tools_from_manifest", {
          manifestJson: state.pendingToolManifestJson,
          githubAccessMode: state.githubAccessMode,
        })
      : await invoke<ToolStatus[]>("install_tools", { githubAccessMode: state.githubAccessMode });
    state.pendingToolManifestJson = null;
    await loadAppState();
    applyToolSummary(tools, managedSummaryMode(state.platform));
    elements.toolInstallStatus.textContent = state.toolsReady ? t("settings.toolsInstalled") : t("settings.toolsInstallPartial");
    showNotice(state.toolsReady ? t("notice.toolsInstalled") : t("notice.toolInstallNeedsAttention"), state.toolsReady ? "success" : "warning");
    logEvent(state.toolsReady ? t("event.toolsInstalled") : t("event.toolsPartial"));
  } catch (error) {
    const message = String(error);
    elements.toolInstallStatus.textContent = message || t("settings.toolInstallFailed");
    showNotice(message || t("settings.toolInstallFailed"), "error");
    logEvent(`${t("event.toolInstallFailed")} ${message}`.trim());
  } finally {
    setBusy(false);
  }
}

async function checkToolUpdates() {
  if (state.busy || state.toolchainSource !== "managed") {
    return;
  }

  setBusy(true, undefined, "tools");
  state.pendingToolManifestJson = null;
  if (state.toolAction === "update") {
    state.toolAction = null;
    updateToolActionButton();
  }
  elements.toolInstallStatus.textContent = t("settings.toolUpdatesChecking");
  try {
    const result = await invoke<ManagedToolUpdateResult>("check_managed_tool_updates", {
      githubAccessMode: state.githubAccessMode,
    });
    const outcome = managedToolUpdateOutcome(
      result,
      managedSummaryMode(state.platform),
    );
    if (outcome.kind === "no_release") {
      elements.toolInstallStatus.textContent = t("updates.noRelease");
      showNotice(t("updates.noRelease"), "warning");
      return;
    }
    if (outcome.kind === "no_manifest") {
      elements.toolInstallStatus.textContent = t("settings.toolUpdatesNoManifest");
      showNotice(t("settings.toolUpdatesNoManifest"), "warning");
      return;
    }
    if (outcome.kind === "invalid") {
      elements.toolInstallStatus.textContent = t("settings.toolUpdatesInvalidManifest");
      showNotice(t("settings.toolUpdatesInvalidManifest"), "warning");
      return;
    }

    const summary = applyToolSummary(result.tools, outcome.mode, {
      remoteRevision: outcome.remoteRevision,
    });
    state.pendingToolManifestJson = summary.action ? outcome.manifestJson : null;
    if (summary.ready) {
      elements.toolInstallStatus.textContent = t("settings.toolUpdatesCurrent");
      logEvent(t("event.toolUpdatesCurrent"));
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    elements.toolInstallStatus.textContent = t("settings.toolUpdatesFailed", { message });
    showNotice(t("settings.toolUpdatesFailed", { message }), "error");
  } finally {
    setBusy(false);
  }
}

async function reinstallTools() {
  if (
    state.busy ||
    !canReinstallManagedTools(
      state.platform,
      state.toolchainSource,
      state.managedProviderMissing,
    )
  ) {
    return;
  }

  const path = elements.toolRoot.textContent || t("settings.toolsPathPending");
  const confirmationKey = managedActionConfirmationKey(state.platform, "reinstall");
  const confirmation = confirmationKey
    ? t(confirmationKey)
    : t("settings.reinstallConfirm", { path });
  if (!window.confirm(confirmation)) {
    return;
  }

  setBusy(true, undefined, "tools");
  elements.toolInstallStatus.textContent = t("settings.reinstallingTools");
  try {
    const tools = await invoke<ToolStatus[]>("reinstall_tools", {
      manifestJson: null,
      githubAccessMode: state.githubAccessMode,
    });
    await loadAppState();
    applyToolSummary(tools, managedSummaryMode(state.platform));
    elements.toolInstallStatus.textContent = state.toolsReady ? t("settings.toolsInstalled") : t("settings.toolsInstallPartial");
    showNotice(state.toolsReady ? t("notice.toolsInstalled") : t("notice.toolInstallNeedsAttention"), state.toolsReady ? "success" : "warning");
    logEvent(state.toolsReady ? t("event.toolsInstalled") : t("event.toolsPartial"));
  } catch (error) {
    const message = String(error);
    elements.toolInstallStatus.textContent = message || t("settings.toolInstallFailed");
    showNotice(message || t("settings.toolInstallFailed"), "error");
    logEvent(`${t("event.toolInstallFailed")} ${message}`.trim());
  } finally {
    setBusy(false);
  }
}

async function parseCurrentUrl() {
  const url = elements.url.value.trim();
  if (!url || state.busy) {
    return;
  }

  setBusy(true, t("progress.parsing"), "metadata");
  renderEmptyPreview(t("preview.readingMetadata"));
  try {
    const metadata = await invoke<VideoMetadata>("parse_metadata", { url });
    state.metadata = metadata;
    state.lastUrl = url;
    state.selectedFormat = metadata.format_options[0] ?? null;
    renderMetadata(metadata);
    renderQualityOptions(metadata.format_options);
    elements.progressText.textContent = t("progress.metadataReady");
    showNotice(t("notice.metadataParsed"), "success");
    logEvent(t("event.parsed", { title: metadata.title }));
  } catch (error) {
    renderEmptyPreview(t("preview.parseFailed"));
    elements.progressText.textContent = t("progress.metadataFailed");
    showNotice(String(error), "error");
    logEvent(t("event.metadataFailed"));
  } finally {
    setBusy(false);
  }
}

async function downloadCurrentVideo() {
  const metadata = state.metadata;
  const selectedFormat = state.selectedFormat;
  const url = state.lastUrl || elements.url.value.trim();
  if (!metadata || !selectedFormat || !url || state.busy) {
    return;
  }

  setBusy(true, t("progress.startingDownload", { quality: selectedFormat.label }), "download");
  elements.progress.removeAttribute("value");
  try {
    const outputPath = await invoke<string | null>("download_video", {
      request: {
        url,
        format_selector: selectedFormat.format_selector,
        output_format: elements.outputFormat.value,
        label: selectedFormat.label,
      },
    });
    elements.progress.value = 100;
    elements.progressText.textContent = outputPath ? t("progress.savedTo", { path: outputPath }) : t("progress.completedOpenFolder");
    showNotice(t("notice.downloadCompleted"), "success");
    logEvent(outputPath ? t("event.saved", { path: outputPath }) : t("event.downloadCompleted"));
  } catch (error) {
    const message = String(error);
    elements.progress.value = 0;
    if (message.toLowerCase().includes("cancel")) {
      elements.progressText.textContent = t("progress.downloadCancelled");
      showNotice(t("notice.downloadCancelled"), "warning");
      logEvent(t("event.downloadCancelled"));
    } else {
      elements.progressText.textContent = t("progress.downloadFailed");
      showNotice(message, "error");
      logEvent(t("event.downloadFailed"));
    }
  } finally {
    setBusy(false);
  }
}

async function cancelCurrentDownload() {
  if (state.activeOperation !== "download" || state.cancelRequested) {
    return;
  }

  state.cancelRequested = true;
  elements.progressText.textContent = t("progress.cancelling");
  updateButtons();
  try {
    await invoke("cancel_download");
    logEvent(t("event.cancelRequested"));
  } catch (error) {
    showNotice(String(error), "error");
    state.cancelRequested = false;
    updateButtons();
  }
}

async function openDownloadFolder() {
  try {
    await invoke("open_download_directory");
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function openProjectRepository() {
  try {
    await openUrl(PROJECT_REPOSITORY_URL);
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function openHomebrewWebsite() {
  try {
    await openUrl(HOMEBREW_URL);
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function openLatestRelease() {
  try {
    await openUrl(resolveGithubUrl(state.latestReleaseUrl || PROJECT_RELEASES_URL, state.githubAccessMode));
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function checkForUpdates() {
  if (state.updateChecking) {
    return;
  }

  state.updateChecking = true;
  state.latestReleaseUrl = "";
  elements.releaseLink.hidden = true;
  setUpdateStatus("updates.checking", "neutral");
  updateButtons();

  try {
    const response = await fetch(resolveGithubUrl(LATEST_RELEASE_API_URL, state.githubAccessMode), {
      cache: "no-store",
      headers: {
        Accept: "application/vnd.github+json",
      },
    });

    if (response.status === 404) {
      setUpdateStatus("updates.noRelease", "warning");
      return;
    }

    if (!response.ok) {
      const githubError = await parseGithubHttpError(response);
      if (githubError.isRateLimited) {
        setUpdateStatus("updates.rateLimited", "error", { time: formatGithubRateLimitReset(githubError.rateLimitResetEpochSeconds) });
        return;
      }
      throw new Error(githubError.message);
    }

    const latestRelease = parseLatestRelease(await response.json());
    if (!latestRelease) {
      setUpdateStatus("updates.invalidRelease", "error");
      return;
    }

    const updateStatus = getUpdateStatus(APP_VERSION, latestRelease);
    if (updateStatus.kind === "available") {
      state.latestReleaseUrl = updateStatus.releaseUrl;
      elements.releaseLink.hidden = false;
      setUpdateStatus("updates.available", "success", { version: updateStatus.latestVersion });
    } else {
      setUpdateStatus("updates.current", "success");
    }
  } catch (error) {
    setUpdateStatus("updates.failed", "error", { message: error instanceof Error ? error.message : String(error) });
  } finally {
    state.updateChecking = false;
    updateButtons();
  }
}

async function browseDownloadFolder() {
  try {
    const selected = await open({
      title: t("settings.chooseFolder"),
      directory: true,
      multiple: false,
      defaultPath: elements.folderInput.value || undefined,
    });

    if (typeof selected === "string") {
      elements.folderInput.value = selected;
      await saveDownloadFolder();
    }
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function saveDownloadFolder() {
  try {
    const appState = await invoke<AppState>("set_download_directory", { directory: elements.folderInput.value });
    elements.folderText.textContent = appState.download_directory;
    elements.folderInput.value = appState.download_directory;
    showNotice(t("notice.folderUpdated"), "success");
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function resetDownloadFolder() {
  try {
    const appState = await invoke<AppState>("reset_download_directory");
    elements.folderText.textContent = appState.download_directory;
    elements.folderInput.value = appState.download_directory;
    showNotice(t("notice.folderReset"), "success");
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function chooseCookiesFile() {
  if (state.busy) {
    return;
  }

  try {
    const selected = await open({
      title: t("cookies.chooseFile"),
      directory: false,
      multiple: false,
      defaultPath: state.cookiesFile || undefined,
    });

    if (typeof selected === "string") {
      const appState = await invoke<AppState>("set_cookies_file", { path: selected });
      renderCookiesFile(appState.cookies_file ?? null);
      invalidateParsedVideo(t("preview.cookiesChanged"));
      showNotice(t("notice.cookiesUpdated"), "success");
      logEvent(t("event.cookiesUpdated", { file: fileNameFromPath(appState.cookies_file || selected) }));
    }
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function clearCookiesFile() {
  if (state.busy || !state.cookiesFile) {
    return;
  }

  try {
    const appState = await invoke<AppState>("clear_cookies_file");
    renderCookiesFile(appState.cookies_file ?? null);
    invalidateParsedVideo(t("preview.cookiesChanged"));
    showNotice(t("notice.cookiesCleared"), "success");
    logEvent(t("event.cookiesCleared"));
  } catch (error) {
    showNotice(String(error), "error");
  }
}

function renderMetadata(metadata: VideoMetadata) {
  elements.title.textContent = metadata.title;
  elements.details.textContent = [
    metadata.id ? `ID ${metadata.id}` : null,
    metadata.duration_seconds ? formatDuration(metadata.duration_seconds) : null,
    metadata.webpage_url,
  ]
    .filter(Boolean)
    .join(" · ");
  elements.description.textContent = metadata.description?.trim() || t("preview.noDescription");

  renderThumbnailCandidates(thumbnailUrlCandidates(metadata));
}

function renderEmptyPreview(message: string) {
  elements.title.textContent = t("preview.noVideo");
  elements.details.textContent = message;
  elements.description.textContent = "";
  clearThumbnail();
}

function invalidateParsedVideo(message: string) {
  state.metadata = null;
  state.selectedFormat = null;
  state.lastUrl = "";
  renderEmptyPreview(message);
  renderQualityOptions([]);
}

function renderThumbnailCandidates(urls: string[]) {
  state.thumbnailCandidates = urls;
  state.thumbnailCandidateIndex = 0;

  if (urls.length === 0) {
    clearThumbnail();
    return;
  }

  loadThumbnailCandidate(0);
}

function loadThumbnailCandidate(index: number) {
  const url = state.thumbnailCandidates[index];
  if (!url) {
    clearThumbnail();
    return;
  }

  state.thumbnailCandidateIndex = index;
  elements.thumbnail.dataset.thumbnailIndex = String(index);
  elements.thumbnail.hidden = true;
  elements.thumbnailEmpty.hidden = false;
  elements.thumbnail.src = url;
}

function showLoadedThumbnail() {
  const currentIndex = Number(elements.thumbnail.dataset.thumbnailIndex ?? state.thumbnailCandidateIndex);
  if (!state.thumbnailCandidates[currentIndex]) {
    return;
  }

  elements.thumbnail.hidden = false;
  elements.thumbnailEmpty.hidden = true;
}

function loadNextThumbnailCandidate() {
  const currentIndex = Number(elements.thumbnail.dataset.thumbnailIndex ?? state.thumbnailCandidateIndex);
  const nextIndex = currentIndex + 1;
  if (nextIndex < state.thumbnailCandidates.length) {
    loadThumbnailCandidate(nextIndex);
    return;
  }

  clearThumbnail();
}

function clearThumbnail() {
  state.thumbnailCandidates = [];
  state.thumbnailCandidateIndex = 0;
  delete elements.thumbnail.dataset.thumbnailIndex;
  elements.thumbnail.removeAttribute("src");
  elements.thumbnail.hidden = true;
  elements.thumbnailEmpty.hidden = false;
}

function renderQualityOptions(options: VideoFormatOption[]) {
  elements.quality.replaceChildren(
    ...options.map((option) => {
      const item = document.createElement("option");
      item.textContent = option.label;
      item.value = option.format_selector;
      return item;
    }),
  );
  elements.quality.disabled = options.length === 0;
}

function renderTools(tools: ToolStatus[]) {
  elements.toolList.replaceChildren(
    ...tools.map((tool) => {
      const row = document.createElement("li");
      row.className = `tool-row is-${tool.availability}`;
      row.innerHTML = `
        <span class="tool-dot"></span>
        <span class="tool-name"></span>
        <span class="tool-version"></span>
      `;
      row.querySelector(".tool-name")!.textContent = tool.name;
      row.querySelector(".tool-version")!.textContent = formatToolVersion(tool);
      row.title = formatToolTitle(tool);
      return row;
    }),
  );
}

function renderToolchainRevision() {
  elements.toolchainRevision.textContent =
    state.toolchainRevision ?? t("settings.noActiveRevision");
}

function renderToolchainSource() {
  const isLocal = state.toolchainSource === "local";
  const isHomebrew = state.platform.managedProvider === "homebrew";
  const managedLabel =
    isHomebrew
      ? state.platform.sourceLabels.managed
      : t("settings.managedTools");
  const localLabel =
    isHomebrew
      ? state.platform.sourceLabels.local
      : t("settings.localTools");

  elements.toolSourceManaged.textContent = managedLabel;
  elements.toolSourceLocal.textContent = localLabel;
  elements.toolSourceManaged.classList.toggle("is-active", !isLocal);
  elements.toolSourceLocal.classList.toggle("is-active", isLocal);
  elements.toolSourceManaged.setAttribute("aria-pressed", String(!isLocal));
  elements.toolSourceLocal.setAttribute("aria-pressed", String(isLocal));
  elements.managedToolchainDetails.hidden = isLocal;
  elements.localToolchainPaths.hidden = !isLocal;
  elements.aria2cToolPaths.hidden = !isLocal && isHomebrew;
  elements.autoDetectLocalTools.hidden = !isLocal && isHomebrew;
  elements.toolchainRevisionRow.hidden = !showsRevision(state.platform);
  elements.toolRoot.textContent = isHomebrew
    ? state.toolsRoot
      ? t("settings.homebrewPrefix", { path: state.toolsRoot })
      : t("settings.homebrewPrefixPending")
    : state.toolsRoot || t("settings.toolsPathPending");
  elements.toolchainHint.textContent = t(
    isLocal
      ? "settings.localToolchainHint"
      : isHomebrew
        ? "settings.homebrewToolchainHint"
        : "settings.toolchainHint",
  );
  elements.managedProviderGuidance.hidden = isLocal || !isHomebrew;
  elements.managedProviderGuidance.textContent = t(
    state.managedProviderMissing
      ? "settings.homebrewMissing"
      : "settings.homebrewGuidance",
  );
  elements.homebrewHelp.hidden = isLocal || !state.managedProviderMissing;
  elements.autoDetectLocalTools.title = t("settings.usePathHint");
  elements.checkToolUpdates.hidden = isLocal || !state.platform.capabilities.update;
  elements.reinstallTools.hidden = !canReinstallManagedTools(
    state.platform,
    state.toolchainSource,
    state.managedProviderMissing,
  );
  updateToolActionButton();
}

function renderLocalToolchainPaths() {
  renderLocalToolPath(elements.localYtDlpPath, state.localToolchainPaths.ytDlpPath);
  renderLocalToolPath(elements.localFfmpegPath, state.localToolchainPaths.ffmpegDirectory);
  renderLocalToolPath(elements.localDenoPath, state.localToolchainPaths.denoPath);
}

function renderLocalToolPath(element: HTMLElement, path?: string | null) {
  const value = path?.trim() || "";
  element.textContent = value || t("settings.localPathNotDetected");
  element.title = value || t("settings.localPathNotDetected");
}

function applyToolSummary(
  tools: ToolStatus[],
  mode: ToolSummaryMode,
  options: { quietReady?: boolean; remoteRevision?: string | null } = {},
) {
  const summary =
    mode === "remote"
      ? summarizeRemoteTools(tools, state.toolchainRevision, options.remoteRevision ?? null)
      : summarizeTools(tools, mode);
  state.aria2cResolvedPath = tools.find(tool => tool.name === "aria2c")?.full_path || null;
  state.toolsReady = summary.ready;
  state.toolAction = summary.action;
  state.managedProviderMissing = tools.some(
    (tool) => tool.availability === "provider_missing",
  );
  renderTools(tools);
  renderToolchainSource();
  elements.toolInstallStatus.textContent = t(summary.settingsKey);
  if (!(options.quietReady && summary.ready)) {
    showNotice(t(summary.noticeKey), summary.tone);
  }
  logEvent(t(summary.eventKey));
  return summary;
}

function formatToolVersion(tool: ToolStatus) {
  if (tool.availability === "outdated" && tool.expected_version) {
    return `${tool.version || t("tool.currentUnknown")} -> ${tool.expected_version}`;
  }
  return tool.version || tool.error || tool.relative_path;
}

function formatToolTitle(tool: ToolStatus) {
  return [
    tool.full_path,
    tool.expected_version ? `Expected ${tool.expected_version}` : null,
    tool.error,
  ]
    .filter(Boolean)
    .join("\n");
}

function updateDownloadProgress(progress: DownloadProgress) {
  if (typeof progress.percent === "number") {
    elements.progress.value = progress.percent;
  } else {
    elements.progress.removeAttribute("value");
  }

  elements.progressText.textContent = [
    progress.status,
    typeof progress.percent === "number" ? `${progress.percent.toFixed(1)}%` : null,
    progress.speed,
    progress.eta ? `${t("progress.eta")} ${progress.eta}` : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

function updateToolInstallProgress(progress: ToolInstallProgress) {
  elements.toolInstallStatus.textContent = [
    progress.status,
    typeof progress.percent === "number" ? `${progress.percent.toFixed(0)}%` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  if (typeof progress.percent !== "number" || progress.percent >= 100) {
    logEvent(progress.tool ? `${progress.status}: ${progress.tool}` : progress.status);
  }
}

function setBusy(isBusy: boolean, progressText?: string, operation: "metadata" | "download" | "tools" | null = null) {
  state.busy = isBusy;
  state.activeOperation = isBusy ? operation : null;
  if (!isBusy) {
    state.cancelRequested = false;
  }
  if (progressText) {
    elements.progressText.textContent = progressText;
  }
  updateButtons();
}

function updateToolActionButton() {
  elements.installTools.hidden =
    state.toolchainSource === "local" ||
    state.toolAction === null ||
    (state.toolAction !== null && !supportsManagedAction(state.toolAction));
  if (!state.toolAction) {
    return;
  }

  const labelKey =
    state.toolAction === "reinstall"
      ? "action.reinstallTools"
      : state.toolAction === "update"
        ? "action.updateTools"
        : "action.installTools";
  elements.installTools.textContent = t(labelKey);
}

function supportsManagedAction(action: ToolAction): boolean {
  return state.platform.capabilities[action];
}

function toolActionStatusKey(action: ToolAction | null): MessageKey {
  if (action === "reinstall") {
    return "settings.reinstallingTools";
  }
  if (action === "update") {
    return "settings.updatingTools";
  }
  return "settings.installingTools";
}

function renderCookiesFile(file: string | null) {
  state.cookiesFile = file?.trim() || null;
  elements.cookiesFile.textContent = state.cookiesFile ? fileNameFromPath(state.cookiesFile) : t("cookies.none");
  elements.cookiesFile.title = state.cookiesFile || t("cookies.none");
  updateButtons();
}

function updateButtons() {
  renderAria2c();
  const hasUrl = elements.url.value.trim().length > 0;
  elements.parse.disabled = state.busy || !hasUrl || !state.toolsReady;
  elements.download.disabled = state.busy || !state.metadata || !state.selectedFormat || !state.toolsReady;
  elements.outputFormat.disabled = state.busy;
  elements.quality.disabled = state.busy || !state.metadata?.format_options.length;
  elements.cancel.disabled = state.activeOperation !== "download" || state.cancelRequested;
  elements.chooseCookies.disabled = state.busy;
  elements.clearCookies.disabled = state.busy || !state.cookiesFile;
  elements.toolSourceManaged.disabled = state.busy;
  elements.toolSourceLocal.disabled = state.busy;
  elements.chooseLocalYtDlp.disabled = state.busy || state.toolchainSource !== "local";
  elements.chooseLocalFfmpeg.disabled = state.busy || state.toolchainSource !== "local";
  elements.chooseLocalDeno.disabled = state.busy || state.toolchainSource !== "local";
  elements.autoDetectLocalTools.disabled = state.busy;
  elements.verifyTools.disabled = state.busy;
  elements.checkToolUpdates.disabled =
    state.busy ||
    state.toolchainSource !== "managed" ||
    !state.platform.capabilities.update;
  elements.installTools.disabled =
    state.busy || !state.toolAction || !supportsManagedAction(state.toolAction);
  elements.reinstallTools.disabled =
    state.busy ||
    !canReinstallManagedTools(
      state.platform,
      state.toolchainSource,
      state.managedProviderMissing,
    );
  elements.browseFolder.disabled = state.busy;
  elements.saveFolder.disabled = state.busy;
  elements.resetFolder.disabled = state.busy;
  elements.checkUpdates.disabled = state.updateChecking;
  elements.githubDirect.disabled = state.updateChecking;
  elements.githubProxy.disabled = state.updateChecking;
}

function showNotice(message: string, tone: NoticeTone) {
  const text = stripTerminalSentencePunctuation(message.trim());
  if (!text) {
    return;
  }

  const toast = document.createElement("div");
  toast.className = `toast is-${tone}`;
  toast.setAttribute("role", tone === "error" ? "alert" : "status");

  const indicator = document.createElement("span");
  indicator.className = "toast-indicator";
  indicator.setAttribute("aria-hidden", "true");

  const copy = document.createElement("p");
  copy.className = "toast-copy";
  copy.textContent = text;

  const close = document.createElement("button");
  close.className = "toast-close";
  close.type = "button";
  close.textContent = "×";
  close.setAttribute("aria-label", t("action.dismissNotification"));
  close.addEventListener("click", () => dismissToast(toast));

  toast.addEventListener("pointerenter", () => clearToastTimer(toast));
  toast.addEventListener("pointerleave", () => maybeResumeToastTimer(toast, tone));
  toast.addEventListener("focusin", () => clearToastTimer(toast));
  toast.addEventListener("focusout", () => maybeResumeToastTimer(toast, tone));

  toast.append(indicator, copy, close);
  elements.toastRegion.prepend(toast);
  trimToastStack();
  scheduleToastDismiss(toast, tone);
}

function scheduleToastDismiss(toast: HTMLElement, tone: NoticeTone) {
  clearToastTimer(toast);
  const duration = TOAST_AUTO_DISMISS_MS[tone];
  if (duration <= 0) {
    return;
  }

  toastTimers.set(
    toast,
    window.setTimeout(() => dismissToast(toast), duration),
  );
}

function maybeResumeToastTimer(toast: HTMLElement, tone: NoticeTone) {
  if (toast.matches(":hover") || toast.contains(document.activeElement)) {
    return;
  }
  scheduleToastDismiss(toast, tone);
}

function clearToastTimer(toast: HTMLElement) {
  const timer = toastTimers.get(toast);
  if (timer) {
    window.clearTimeout(timer);
    toastTimers.delete(toast);
  }
}

function dismissToast(toast: HTMLElement) {
  if (!toast.isConnected || toast.classList.contains("is-leaving")) {
    return;
  }

  clearToastTimer(toast);
  toast.classList.add("is-leaving");
  window.setTimeout(() => toast.remove(), 180);
}

function trimToastStack() {
  while (elements.toastRegion.children.length > MAX_TOASTS) {
    const oldestToast = elements.toastRegion.lastElementChild;
    if (!(oldestToast instanceof HTMLElement)) {
      return;
    }
    clearToastTimer(oldestToast);
    oldestToast.remove();
  }
}

function renderUpdateStatus(message: string, tone: UpdateTone) {
  elements.updateStatus.textContent = message;
  elements.updateStatus.className = `update-status is-${tone}`;
}

function setUpdateStatus(key: MessageKey, tone: UpdateTone, values: Record<string, string | number> = {}) {
  state.updateStatus = { key, values, tone };
  renderUpdateStatus(t(key, values), tone);
}

function clearUpdateStatus() {
  state.latestReleaseUrl = "";
  state.updateStatus = null;
  elements.releaseLink.hidden = true;
  renderUpdateStatus("", "neutral");
}

function updateGithubAccessButtons() {
  elements.githubDirect.classList.toggle("is-active", state.githubAccessMode === "direct");
  elements.githubProxy.classList.toggle("is-active", state.githubAccessMode === "gh-proxy");
  elements.githubDirect.setAttribute("aria-pressed", String(state.githubAccessMode === "direct"));
  elements.githubProxy.setAttribute("aria-pressed", String(state.githubAccessMode === "gh-proxy"));
}

function logEvent(message: string) {
  const row = document.createElement("li");
  row.textContent = `${new Date().toLocaleTimeString("en")} ${message}`;
  elements.events.prepend(row);
  while (elements.events.children.length > 8) {
    elements.events.lastElementChild?.remove();
  }
}

function formatGithubRateLimitReset(epochSeconds?: number) {
  if (!epochSeconds) {
    return t("updates.later");
  }

  return new Intl.DateTimeFormat("en", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(epochSeconds * 1000));
}

function formatDuration(seconds: number) {
  const rounded = Math.max(0, Math.round(seconds));
  const hours = Math.floor(rounded / 3600);
  const minutes = Math.floor((rounded % 3600) / 60);
  const secs = rounded % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`
    : `${minutes}:${String(secs).padStart(2, "0")}`;
}

function fileNameFromPath(path: string) {
  return path.replace(/\\/g, "/").split("/").filter(Boolean).pop() || path;
}
