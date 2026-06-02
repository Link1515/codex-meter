import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

export const devMockCommandAlias = "__codex_meter_mock__";

export const legacyDefaultUsageConfig: CliUsageConfig = {
  codexCommand: "codex",
  usageArgs: ["status"],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Text"
};

const devMockUsageConfig: CliUsageConfig = {
  codexCommand: devMockCommandAlias,
  usageArgs: [],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Text"
};

export const defaultUsageConfig: CliUsageConfig = {
  ...(import.meta.env.DEV ? devMockUsageConfig : legacyDefaultUsageConfig)
};

export const emptySnapshot: CodexUsageSnapshot = {
  source: "codex-cli",
  fetchedAt: "",
  status: "unknown",
  errorMessage: "No usage data available yet"
};
