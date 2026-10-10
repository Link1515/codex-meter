import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

type BackendConfig = Omit<CliUsageConfig, "pollIntervalSeconds">;

export async function isWslSupported(): Promise<boolean> {
  if (!isTauri()) return false;
  return invoke<boolean>("is_wsl_supported");
}

export async function fetchUsage(config: CliUsageConfig): Promise<CodexUsageSnapshot> {
  if (!isTauri()) {
    throw new Error("Codex usage requires the Tauri desktop runtime. Start dev with `pnpm tauri dev`.");
  }

  const backendConfig: BackendConfig = {
    executionMode: config.executionMode,
    wslDistribution: config.wslDistribution,
    wslUser: config.wslUser,
    codexCommand: config.codexCommand,
    usageArgs: config.usageArgs,
    timeoutSeconds: config.timeoutSeconds,
    parserMode: config.parserMode
  };

  try {
    return await invoke<CodexUsageSnapshot>("fetch_usage", { config: backendConfig });
  } catch {
    throw new Error("Unable to fetch Codex usage");
  }
}
