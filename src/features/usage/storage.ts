import { defaultUsageConfig, emptySnapshot, legacyDefaultUsageConfig } from "./defaults";
import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

const configKey = "codex-meter:usage-config";
const snapshotKey = "codex-meter:last-snapshot";

export function loadUsageConfig(): CliUsageConfig {
  const raw = localStorage.getItem(configKey);
  if (!raw) {
    return defaultUsageConfig;
  }

  try {
    const parsed = JSON.parse(raw) as Partial<CliUsageConfig>;
    const config = {
      ...defaultUsageConfig,
      ...parsed,
      usageArgs: Array.isArray(parsed.usageArgs) ? parsed.usageArgs : defaultUsageConfig.usageArgs
    };

    if (import.meta.env.DEV && isLegacyDefaultUsageConfig(config)) {
      return defaultUsageConfig;
    }

    return config;
  } catch {
    return defaultUsageConfig;
  }
}

export function saveUsageConfig(config: CliUsageConfig): void {
  localStorage.setItem(configKey, JSON.stringify(config));
}

export function loadCachedSnapshot(): CodexUsageSnapshot {
  const raw = localStorage.getItem(snapshotKey);
  if (!raw) {
    return emptySnapshot;
  }

  try {
    return JSON.parse(raw) as CodexUsageSnapshot;
  } catch {
    return emptySnapshot;
  }
}

export function saveCachedSnapshot(snapshot: CodexUsageSnapshot): void {
  localStorage.setItem(snapshotKey, JSON.stringify(snapshot));
}

function isLegacyDefaultUsageConfig(config: CliUsageConfig): boolean {
  return (
    config.codexCommand === legacyDefaultUsageConfig.codexCommand &&
    config.timeoutSeconds === legacyDefaultUsageConfig.timeoutSeconds &&
    config.parserMode === legacyDefaultUsageConfig.parserMode &&
    config.usageArgs.length === legacyDefaultUsageConfig.usageArgs.length &&
    config.usageArgs.every((arg, index) => arg === legacyDefaultUsageConfig.usageArgs[index])
  );
}
