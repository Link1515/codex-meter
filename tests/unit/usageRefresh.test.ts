import { describe, expect, it } from "vitest";
import {
  configurationRetryDelayMs,
  minimumAutomaticRefreshDelayMs,
  nextAutomaticRefreshDelayMs,
  retryBackoffMaxMs,
  canStartManualRefresh,
  mergeUsageRefreshResult
} from "../../src/features/usage/refresh";
import type { CodexUsageSnapshot } from "../../src/features/usage/types";

describe("usage refresh controls", () => {
  it("allows a manual refresh outside the debounce window", () => {
    expect(canStartManualRefresh(2000, 900, 1000)).toBe(true);
  });

  it("blocks a manual refresh inside the debounce window", () => {
    expect(canStartManualRefresh(1500, 900, 1000)).toBe(false);
  });

  it("keeps the previous usage values when an automatic refresh returns an error snapshot", () => {
    const previous: CodexUsageSnapshot = {
      source: "codex-cli",
      fetchedAt: "100",
      status: "ok",
      usagePercent: 28,
      remainingPercent: 72,
      fiveHourUsageLimit: { usagePercent: 28, remainingPercent: 72, resetAt: "200" },
      weeklyUsageLimit: { usagePercent: 45, remainingPercent: 55, resetAt: "300" }
    };
    const next: CodexUsageSnapshot = {
      source: "codex-cli",
      fetchedAt: "101",
      status: "timeout",
      errorMessage: "Codex CLI command timed out"
    };

    expect(mergeUsageRefreshResult(previous, next)).toEqual({
      ...previous,
      fetchedAt: "101",
      rawOutput: undefined,
      status: "timeout",
      errorMessage: "Codex CLI command timed out"
    });
  });

  it("uses the error snapshot as-is when there is no previous usage to show", () => {
    const previous: CodexUsageSnapshot = {
      source: "codex-cli",
      fetchedAt: "",
      status: "unknown",
      errorMessage: "No usage data available yet"
    };
    const next: CodexUsageSnapshot = {
      source: "codex-cli",
      fetchedAt: "101",
      status: "command_error",
      errorMessage: "Codex CLI command failed"
    };

    expect(mergeUsageRefreshResult(previous, next)).toBe(next);
  });

  it("uses the configured automatic refresh interval after a successful refresh", () => {
    expect(nextAutomaticRefreshDelayMs(snapshotWithStatus("ok"), 90, 0)).toBe(90_000);
  });

  it("enforces the minimum automatic refresh interval", () => {
    expect(nextAutomaticRefreshDelayMs(snapshotWithStatus("ok"), 30, 0)).toBe(minimumAutomaticRefreshDelayMs);
  });

  it("backs off retryable automatic refresh failures", () => {
    expect(nextAutomaticRefreshDelayMs(snapshotWithStatus("command_error"), 60, 1)).toBe(90_000);
    expect(nextAutomaticRefreshDelayMs(snapshotWithStatus("timeout"), 60, 2)).toBe(180_000);
    expect(nextAutomaticRefreshDelayMs(snapshotWithStatus("command_error"), 60, 10)).toBe(retryBackoffMaxMs);
  });

  it("slows down automatic refresh for configuration and authentication states", () => {
    expect(nextAutomaticRefreshDelayMs(snapshotWithStatus("not_authenticated"), 60, 1)).toBe(configurationRetryDelayMs);
  });
});

function snapshotWithStatus(status: CodexUsageSnapshot["status"]): CodexUsageSnapshot {
  return {
    source: "codex-cli",
    fetchedAt: "100",
    status
  };
}
