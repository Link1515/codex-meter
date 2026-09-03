import { describe, expect, it, vi } from "vitest";
import { saveCachedSnapshot } from "../../src/features/usage/storage";

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
});
