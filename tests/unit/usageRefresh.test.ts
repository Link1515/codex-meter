import { describe, expect, it } from "vitest";
import { canStartManualRefresh } from "../../src/features/usage/refresh";

describe("usage refresh controls", () => {
  it("allows a manual refresh outside the debounce window", () => {
    expect(canStartManualRefresh(2000, 900, 1000)).toBe(true);
  });

  it("blocks a manual refresh inside the debounce window", () => {
    expect(canStartManualRefresh(1500, 900, 1000)).toBe(false);
  });
});
