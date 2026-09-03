import { describe, expect, it, vi } from "vitest";
import { saveCachedSnapshot } from "../../src/features/usage/storage";

describe("usage snapshot storage", () => {
  it("does not persist raw CLI output in cached snapshots", () => {
    const storage = {
      getItem: vi.fn(() => null),
      setItem: vi.fn()
    };
    vi.stubGlobal("localStorage", storage);

    saveCachedSnapshot({
      source: "codex-cli",
      fetchedAt: "0",
      status: "ok",
      rawOutput: "token: secret"
    });

    const [, serialized] = storage.setItem.mock.calls[0];
    expect(JSON.parse(serialized)).not.toHaveProperty("rawOutput");

    vi.unstubAllGlobals();
  });
});
