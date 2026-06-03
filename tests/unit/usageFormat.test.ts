import { describe, expect, it } from "vitest";
import {
  commandPreview,
  formatPercent,
  formatResetTimestamp,
  resolveFiveHourLimit,
  resolveWeeklyLimit,
  statusLabel
} from "../../src/features/usage/format";
import type { CodexUsageSnapshot } from "../../src/features/usage/types";

describe("usage formatters", () => {
  it("formats missing percentages without leaking undefined", () => {
    expect(formatPercent(undefined)).toBe("--");
  });

  it("formats rounded percentages", () => {
    expect(formatPercent(72.4)).toBe("72%");
    expect(formatPercent(72.5)).toBe("73%");
  });

  it("formats same-day reset timestamps with 12-hour time", () => {
    const resetAt = new Date(2026, 5, 2, 15, 30).toISOString();
    const now = new Date(2026, 5, 2, 9, 0);

    expect(formatResetTimestamp(resetAt, now)).toBe("03:30 PM");
  });

  it("formats other-day reset timestamps with date and 12-hour time", () => {
    const resetAt = new Date(2026, 5, 3, 8, 5).toISOString();
    const now = new Date(2026, 5, 2, 9, 0);

    expect(formatResetTimestamp(resetAt, now)).toBe("2026-06-03 08:05 AM");
  });

  it("maps stable app statuses to labels", () => {
    expect(statusLabel("cli_not_found")).toBe("CLI missing");
    expect(statusLabel("parse_error")).toBe("Parse failed");
  });

  it("formats the dev mock command alias for display", () => {
    expect(commandPreview({
      codexCommand: "__codex_meter_mock__",
      usageArgs: [],
      pollIntervalSeconds: 60,
      timeoutSeconds: 10,
      parserMode: "Text"
    })).toBe("mock codex usage");
  });

  it("resolves limit windows from new snapshot fields", () => {
    const snapshot: CodexUsageSnapshot = {
      source: "codex-cli",
      fetchedAt: "0",
      fiveHourUsageLimit: { usagePercent: 28, remainingPercent: 72, resetAt: "five-hour-reset" },
      weeklyUsageLimit: { usagePercent: 45, remainingPercent: 55, resetAt: "weekly-reset" },
      status: "ok"
    };

    expect(resolveFiveHourLimit(snapshot)).toEqual({
      usagePercent: 28,
      remainingPercent: 72,
      resetAt: "five-hour-reset"
    });
    expect(resolveWeeklyLimit(snapshot)).toEqual({
      usagePercent: 45,
      remainingPercent: 55,
      resetAt: "weekly-reset"
    });
  });

  it("keeps legacy single-window snapshots compatible", () => {
    const snapshot: CodexUsageSnapshot = {
      source: "codex-cli",
      fetchedAt: "0",
      usagePercent: 28,
      remainingPercent: 72,
      windowResetAt: "window-reset",
      weeklyResetAt: "weekly-reset",
      status: "ok"
    };

    expect(resolveFiveHourLimit(snapshot)).toEqual({
      usagePercent: 28,
      remainingPercent: 72,
      resetAt: "window-reset"
    });
    expect(resolveWeeklyLimit(snapshot)).toEqual({
      usagePercent: undefined,
      remainingPercent: undefined,
      resetAt: "weekly-reset"
    });
  });
});
