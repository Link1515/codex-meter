import { defaultUsageConfig, emptySnapshot } from "./defaults";
import { validateUsageConfig } from "./config";
import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

const snapshotKey = "codex-meter:last-snapshot";
const configKey = "codex-meter:usage-config";

export function loadUsageConfig(): CliUsageConfig {
  try {
    const raw = localStorage.getItem(configKey);
    return raw ? validateUsageConfig(JSON.parse(raw)) : defaultUsageConfig;
  } catch {
    return defaultUsageConfig;
  }
}

export function saveUsageConfig(config: CliUsageConfig): void {
  localStorage.setItem(configKey, JSON.stringify(validateUsageConfig(config)));
}

export function loadCachedSnapshot(): CodexUsageSnapshot {
  const raw = localStorage.getItem(snapshotKey);
  if (!raw) {
    return emptySnapshot;
  }

  try {
    const snapshot = JSON.parse(raw) as CodexUsageSnapshot;
    const safeSnapshot = withoutErrorMessage(snapshot);
    if (snapshot.errorMessage !== undefined) {
      localStorage.setItem(snapshotKey, JSON.stringify(safeSnapshot));
    }
    return safeSnapshot;
  } catch {
    return emptySnapshot;
  }
}

export function saveCachedSnapshot(snapshot: CodexUsageSnapshot): void {
  localStorage.setItem(snapshotKey, JSON.stringify(withoutErrorMessage(snapshot)));
}

function withoutErrorMessage(snapshot: CodexUsageSnapshot): CodexUsageSnapshot {
  const safeSnapshot = { ...snapshot };
  delete safeSnapshot.errorMessage;
  return safeSnapshot;
}
