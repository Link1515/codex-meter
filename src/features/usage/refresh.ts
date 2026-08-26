import type { CodexUsageSnapshot } from "./types";

export const manualRefreshDebounceMs = 1000;
export const minimumAutomaticRefreshDelayMs = 60_000;
export const retryBackoffBaseMs = 90_000;
export const retryBackoffMaxMs = 300_000;
export const configurationRetryDelayMs = 300_000;

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

export function nextAutomaticRefreshDelayMs(
  snapshot: CodexUsageSnapshot,
  pollIntervalSeconds: number,
  consecutiveFailureCount: number
): number {
  const configuredDelayMs = Math.max(pollIntervalSeconds * 1000, minimumAutomaticRefreshDelayMs);

  if (snapshot.status === "ok") {
    return configuredDelayMs;
  }

  if (isRetryableRefreshStatus(snapshot)) {
    const retryAttempt = Math.max(0, consecutiveFailureCount - 1);
    const retryDelayMs = retryBackoffBaseMs * 2 ** retryAttempt;

    return Math.min(Math.max(configuredDelayMs, retryDelayMs), retryBackoffMaxMs);
  }

  return Math.max(configuredDelayMs, configurationRetryDelayMs);
}

export function isRetryableRefreshStatus(snapshot: CodexUsageSnapshot): boolean {
  return snapshot.status === "timeout" || snapshot.status === "command_error";
}

function hasDisplayableUsage(snapshot: CodexUsageSnapshot): boolean {
  return (
    snapshot.fiveHourUsageLimit?.usagePercent !== undefined ||
    snapshot.fiveHourUsageLimit?.remainingPercent !== undefined ||
    snapshot.weeklyUsageLimit?.usagePercent !== undefined ||
    snapshot.weeklyUsageLimit?.remainingPercent !== undefined
  );
}
