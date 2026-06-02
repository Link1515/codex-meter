import { describe, expect, it, vi } from "vitest";
import { loadUsageConfig, saveCachedSnapshot } from "../../src/features/usage/storage";

describe("usage config storage", () => {
  it("defaults to the Codex app-server RPC command", () => {
    vi.stubGlobal("localStorage", {
      getItem: vi.fn(() => null),
      setItem: vi.fn()
    });

    expect(loadUsageConfig()).toMatchObject({
      codexCommand: "codex",
      usageArgs: ["-s", "read-only", "-a", "untrusted", "app-server"]
    });

    vi.unstubAllGlobals();
  });

  it("migrates the legacy interactive status command to app-server RPC", () => {
    vi.stubGlobal("localStorage", {
      getItem: vi.fn(() => JSON.stringify({
        codexCommand: "codex",
        usageArgs: ["status"],
        pollIntervalSeconds: 60,
        timeoutSeconds: 10,
        parserMode: "Text"
      })),
      setItem: vi.fn()
    });

    expect(loadUsageConfig()).toMatchObject({
      codexCommand: "codex",
      usageArgs: ["-s", "read-only", "-a", "untrusted", "app-server"]
    });

    vi.unstubAllGlobals();
  });

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
