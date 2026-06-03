import type { CodexUsageSnapshot } from "./types";

export const manualRefreshDebounceMs = 1000;

export function canStartManualRefresh(
  nowMs: number,
  lastRefreshMs: number,
  debounceMs = manualRefreshDebounceMs
): boolean {
  return nowMs - lastRefreshMs >= debounceMs;
}

export function mergeUsageRefreshResult(
  previousSnapshot: CodexUsageSnapshot,
  nextSnapshot: CodexUsageSnapshot
): CodexUsageSnapshot {
  if (nextSnapshot.status === "ok" || !hasDisplayableUsage(previousSnapshot)) {
    return nextSnapshot;
  }

  return {
    ...previousSnapshot,
    source: nextSnapshot.source,
    fetchedAt: nextSnapshot.fetchedAt,
    rawOutput: nextSnapshot.rawOutput,
    status: nextSnapshot.status,
    errorMessage: nextSnapshot.errorMessage
  };
}

function hasDisplayableUsage(snapshot: CodexUsageSnapshot): boolean {
  return (
    snapshot.usagePercent !== undefined ||
    snapshot.remainingPercent !== undefined ||
    snapshot.fiveHourUsageLimit?.usagePercent !== undefined ||
    snapshot.fiveHourUsageLimit?.remainingPercent !== undefined ||
    snapshot.weeklyUsageLimit?.usagePercent !== undefined ||
    snapshot.weeklyUsageLimit?.remainingPercent !== undefined
  );
}
