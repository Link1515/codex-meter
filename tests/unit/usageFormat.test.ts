import { describe, expect, it } from "vitest";
import { commandPreview, formatPercent, statusLabel } from "../../src/features/usage/format";

describe("usage formatters", () => {
  it("formats missing percentages without leaking undefined", () => {
    expect(formatPercent(undefined)).toBe("--");
  });

  it("formats rounded percentages", () => {
    expect(formatPercent(72.4)).toBe("72%");
    expect(formatPercent(72.5)).toBe("73%");
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
});
