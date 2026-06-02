import { invoke } from "@tauri-apps/api/core";
import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

type BackendConfig = Omit<CliUsageConfig, "pollIntervalSeconds">;

export async function fetchUsage(config: CliUsageConfig): Promise<CodexUsageSnapshot> {
  const backendConfig: BackendConfig = {
    codexCommand: config.codexCommand,
    usageArgs: config.usageArgs,
    timeoutSeconds: config.timeoutSeconds,
    parserMode: config.parserMode
  };

  return invoke<CodexUsageSnapshot>("fetch_usage", { config: backendConfig });
}
