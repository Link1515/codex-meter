import { emptySnapshot } from "./defaults";
import type { CodexUsageSnapshot } from "./types";

const snapshotKey = "codex-meter:last-snapshot";

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
  const { rawOutput: _rawOutput, ...cacheableSnapshot } = snapshot;
  localStorage.setItem(snapshotKey, JSON.stringify(cacheableSnapshot));
}
