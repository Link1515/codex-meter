import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

export const devMockCommandAlias = "__codex_meter_mock__";

export const defaultUsageConfig: CliUsageConfig = {
  codexCommand: "codex",
  usageArgs: ["-s", "read-only", "-a", "never", "app-server"],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Json"
};

export const legacyAppServerUsageConfig: CliUsageConfig = {
  codexCommand: "codex",
  usageArgs: ["-s", "read-only", "-a", "untrusted", "app-server"],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Json"
};

export const legacyStatusUsageConfig: CliUsageConfig = {
  codexCommand: "codex",
  usageArgs: ["status"],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Text"
};

export const devMockUsageConfig: CliUsageConfig = {
  codexCommand: devMockCommandAlias,
  usageArgs: [],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Text"
};

export const emptySnapshot: CodexUsageSnapshot = {
  source: "codex-cli",
  fetchedAt: "",
  status: "unknown",
  errorMessage: "No usage data available yet"
};
