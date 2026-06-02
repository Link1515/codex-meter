import { defaultUsageConfig, devMockUsageConfig, emptySnapshot, legacyStatusUsageConfig } from "./defaults";
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

    if (isLegacyUsageConfig(config) || (import.meta.env.DEV && isDevMockUsageConfig(config))) {
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

function isDevMockUsageConfig(config: CliUsageConfig): boolean {
  return sameCommandConfig(config, devMockUsageConfig);
}

function isLegacyUsageConfig(config: CliUsageConfig): boolean {
  return sameCommandConfig(config, legacyStatusUsageConfig);
}

function sameCommandConfig(config: CliUsageConfig, expected: CliUsageConfig): boolean {
  return (
    config.codexCommand === expected.codexCommand &&
    config.timeoutSeconds === expected.timeoutSeconds &&
    config.parserMode === expected.parserMode &&
    config.usageArgs.length === expected.usageArgs.length &&
    config.usageArgs.every((arg, index) => arg === expected.usageArgs[index])
  );
}
