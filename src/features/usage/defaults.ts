import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

export const defaultUsageConfig: CliUsageConfig = {
  codexCommand: "codex",
  usageArgs: ["-s", "read-only", "-a", "never", "app-server"],
  pollIntervalSeconds: 60,
  timeoutSeconds: 10,
  parserMode: "Json"
};

export const emptySnapshot: CodexUsageSnapshot = {
  source: "codex-cli",
  fetchedAt: "",
  status: "unknown",
  errorMessage: "No usage data available yet"
};
