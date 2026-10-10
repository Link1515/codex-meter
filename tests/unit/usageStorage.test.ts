import { describe, expect, it, vi } from "vitest";
import { loadCachedSnapshot, saveCachedSnapshot } from "../../src/features/usage/storage";

describe("usage snapshot storage", () => {
  it("persists cached snapshots", () => {
    const storage = {
      getItem: vi.fn(() => null),
      setItem: vi.fn()
    };
    vi.stubGlobal("localStorage", storage);

    saveCachedSnapshot({
      source: "codex-cli",
      fetchedAt: "0",
      status: "ok"
    });

    const [, serialized] = storage.setItem.mock.calls[0];
    expect(JSON.parse(serialized)).toEqual({
      source: "codex-cli",
      fetchedAt: "0",
      status: "ok"
    });

    vi.unstubAllGlobals();
  });

  it("does not persist CLI error details", () => {
    const storage = { setItem: vi.fn() };
    vi.stubGlobal("localStorage", storage);

    saveCachedSnapshot({
      source: "codex-cli",
      fetchedAt: "100",
      status: "command_error",
      weeklyUsageLimit: { remainingPercent: 55 },
      errorMessage: "sk-proj-secret-value"
    });

    const [, serialized] = storage.setItem.mock.calls[0];
    expect(serialized).not.toContain("sk-proj-secret-value");
    expect(JSON.parse(serialized)).toEqual({
      source: "codex-cli",
      fetchedAt: "100",
      status: "command_error",
      weeklyUsageLimit: { remainingPercent: 55 }
    });
    vi.unstubAllGlobals();
  });

  it("clears legacy cached error details while preserving usage", () => {
    const storage = {
      getItem: vi.fn(() => JSON.stringify({
        source: "codex-cli",
        fetchedAt: "100",
        status: "command_error",
        weeklyUsageLimit: { remainingPercent: 55 },
        errorMessage: "sk-proj-secret-value"
      })),
      setItem: vi.fn()
    };
    vi.stubGlobal("localStorage", storage);

    const snapshot = loadCachedSnapshot();

    expect(snapshot.weeklyUsageLimit?.remainingPercent).toBe(55);
    expect(snapshot.errorMessage).toBeUndefined();
    expect(storage.setItem.mock.calls[0][1]).not.toContain("sk-proj-secret-value");
    vi.unstubAllGlobals();
  });
});
