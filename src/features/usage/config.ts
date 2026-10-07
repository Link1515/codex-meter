import { defaultUsageConfig } from "./defaults";
import type { CliUsageConfig } from "./types";

export function validateUsageConfig(value: unknown): CliUsageConfig {
  if (typeof value !== "object" || value === null) throw new Error("Invalid usage settings");
  const config = { ...defaultUsageConfig, ...value };
  if (config.executionMode !== "native" && config.executionMode !== "wsl") {
    throw new Error("Invalid execution mode");
  }
  if (typeof config.codexCommand !== "string" || !config.codexCommand.trim() || config.codexCommand.includes("\0")) {
    throw new Error("Codex command cannot be empty or contain a null character");
  }
  if (!Array.isArray(config.usageArgs) || !config.usageArgs.every((arg: unknown) => typeof arg === "string" && !arg.includes("\0"))) {
    throw new Error("Arguments must be a JSON array of strings");
  }
  if (!Number.isInteger(config.pollIntervalSeconds) || config.pollIntervalSeconds < 60 || config.pollIntervalSeconds > 86400) {
    throw new Error("Poll interval must be between 60 and 86400 seconds");
  }
  if (!Number.isInteger(config.timeoutSeconds) || config.timeoutSeconds < 1 || config.timeoutSeconds > 120) {
    throw new Error("Timeout must be between 1 and 120 seconds");
  }
  if (config.parserMode !== "Text" && config.parserMode !== "Json") throw new Error("Invalid parser mode");
  for (const value of [config.wslDistribution, config.wslUser]) {
    if (typeof value !== "string" || value.includes("\0") || value.trim().startsWith("-")) {
      throw new Error("Invalid WSL distribution or user");
    }
  }
  return { ...config, codexCommand: config.codexCommand.trim(), wslDistribution: config.wslDistribution.trim(), wslUser: config.wslUser.trim() };
}
