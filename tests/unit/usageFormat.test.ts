import { describe, expect, it } from "vitest";
import {
  formatCompactResetTimestamp,
  formatDatedResetTimestamp,
  formatPercent,
  statusLabel
} from "../../src/features/usage/format";

describe("usage formatters", () => {
  it("formats missing percentages without leaking undefined", () => {
    expect(formatPercent(undefined)).toBe("--");
  });

  it("formats rounded percentages", () => {
    expect(formatPercent(72.4)).toBe("72%");
    expect(formatPercent(72.5)).toBe("73%");
  });

  it("formats invalid percentages as unavailable", () => {
    expect(formatPercent(Number.NaN)).toBe("--");
  });

  it("formats compact reset timestamps for a two-row meter", () => {
    const now = new Date(2026, 5, 2, 9, 0);

    expect(formatCompactResetTimestamp(new Date(2026, 5, 2, 15, 30).toISOString(), now)).toBe("03:30 PM");
    expect(formatCompactResetTimestamp(new Date(2026, 5, 3, 8, 5).toISOString(), now)).toBe("Wed 08:05 AM");
  });

  it("formats dated reset timestamps for the weekly meter", () => {
    const resetAt = new Date(2026, 5, 3, 8, 5).toISOString();

    expect(formatDatedResetTimestamp(resetAt)).toBe("2026-06-03 · Wed\n08:05 AM");
  });

  it("maps stable app statuses to labels", () => {
    expect(statusLabel("cli_not_found")).toBe("CLI missing");
    expect(statusLabel("parse_error")).toBe("Parse failed");
  });

});
