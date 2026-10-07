import { afterEach, describe, expect, it, vi } from "vitest";
import { defaultUsageConfig } from "../../src/features/usage/defaults";
import { validateUsageConfig } from "../../src/features/usage/config";
import { loadUsageConfig, saveUsageConfig } from "../../src/features/usage/storage";

afterEach(() => vi.unstubAllGlobals());

describe("usage settings", () => {
  it("round trips WSL settings without changing command arguments", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value)
    });
    const config = { ...defaultUsageConfig, executionMode: "wsl" as const, wslDistribution: "Ubuntu", wslUser: "terry", usageArgs: ["app-server", "a b", "$(touch /tmp/test)"] };
    saveUsageConfig(config);
    expect(loadUsageConfig()).toEqual(config);
  });

  it("loads older native settings with defaults for new fields", () => {
    vi.stubGlobal("localStorage", { getItem: () => JSON.stringify({ codexCommand: "custom-codex", timeoutSeconds: 20 }) });
    expect(loadUsageConfig()).toEqual({ ...defaultUsageConfig, codexCommand: "custom-codex", timeoutSeconds: 20 });
  });

  it.each(["{", "null", '{"pollIntervalSeconds":1}', '{"usageArgs":[1]}', '{"executionMode":"invalid"}'])("falls back from invalid stored settings: %s", (raw) => {
    vi.stubGlobal("localStorage", { getItem: () => raw });
    expect(loadUsageConfig()).toEqual(defaultUsageConfig);
  });

  it.each([
    { pollIntervalSeconds: 59 }, { pollIntervalSeconds: 60.5 },
    { timeoutSeconds: 0 }, { timeoutSeconds: 121 },
    { codexCommand: " " }, { usageArgs: ["\0"] },
    { wslDistribution: "--exec" }, { wslUser: "-root" }
  ])("rejects invalid settings: %j", (changes) => {
    expect(() => validateUsageConfig({ ...defaultUsageConfig, ...changes })).toThrow();
  });
});
