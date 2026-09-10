export type Aria2cConfig = {
  schemaVersion: 1;
  enabled: boolean;
  executablePath: string | null;
  parallelConnections: number;
};

export type Aria2cStatus = {
  source: "configured" | "path" | "homebrew-prefix" | "homebrew-apple-silicon" | "homebrew-intel" | null;
  executablePath: string | null;
  available: boolean;
  version: string | null;
  errorCode: string | null;
  error: string | null;
};

export type Aria2cSettings = {
  config: Aria2cConfig;
  status: Aria2cStatus;
  loadError: string | null;
};

export function defaultAria2cSettings(): Aria2cSettings {
  return {
    config: { schemaVersion: 1, enabled: false, executablePath: null, parallelConnections: 16 },
    status: { source: null, executablePath: null, available: false, version: null, errorCode: null, error: null },
    loadError: null,
  };
}

export function parseParallelConnections(value: string): number | null {
  return /^(?:[1-9]|1[0-6])$/u.test(value.trim()) ? Number(value) : null;
}

export class Aria2cSettingsDraft {
  saved = defaultAria2cSettings();
  config = { ...this.saved.config };
  parallelInput = "16";
  status = { ...this.saved.status };
  dirty = false;
  inspecting = false;
  private generation = 0;

  applySaved(settings: Aria2cSettings) {
    this.saved = structuredClone(settings);
    if (this.dirty) return;
    this.generation++;
    this.inspecting = false;
    this.config = { ...settings.config };
    this.parallelInput = String(settings.config.parallelConnections);
    this.status = { ...settings.status };
  }

  edit(patch: Partial<Aria2cConfig>, parallelInput = this.parallelInput) {
    this.config = { ...this.config, ...patch };
    this.parallelInput = parallelInput;
    this.dirty = true;
    this.generation++;
    this.inspecting = false;
    this.status = defaultAria2cSettings().status;
  }

  validatedConfig(): Aria2cConfig {
    const parallelConnections = parseParallelConnections(this.parallelInput);
    if (parallelConnections === null) throw new Error("Enter a whole number from 1 to 16.");
    return { ...this.config, parallelConnections };
  }

  async inspect(run: (config: Aria2cConfig) => Promise<Aria2cStatus>) {
    const config = this.validatedConfig();
    const generation = ++this.generation;
    this.inspecting = true;
    try {
      const status = await run(config);
      if (generation === this.generation) this.status = status;
    } catch (error) {
      if (generation === this.generation) {
        this.status = { ...defaultAria2cSettings().status, errorCode: "probe-failed", error: String(error) };
      }
    } finally {
      if (generation === this.generation) this.inspecting = false;
    }
  }

  async save(run: (config: Aria2cConfig) => Promise<Aria2cSettings>) {
    const config = this.validatedConfig();
    const generation = ++this.generation;
    this.inspecting = false;
    const settings = await run(config);
    this.saved = structuredClone(settings);
    if (generation === this.generation) {
      this.dirty = false;
      this.applySaved(settings);
    }
    return settings;
  }
}
