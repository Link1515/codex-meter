import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CliUsageConfig, CodexUsageSnapshot } from "./types";

type BackendConfig = Omit<CliUsageConfig, "pollIntervalSeconds">;

export async function fetchUsage(config: CliUsageConfig): Promise<CodexUsageSnapshot> {
  if (!isTauri()) {
    throw new Error("Codex usage requires the Tauri desktop runtime. Start dev with `pnpm tauri dev`.");
  }

  const backendConfig: BackendConfig = {
    codexCommand: config.codexCommand,
    usageArgs: config.usageArgs,
    timeoutSeconds: config.timeoutSeconds,
    parserMode: config.parserMode
  };

  try {
    return await invoke<CodexUsageSnapshot>("fetch_usage", { config: backendConfig });
  } catch (error) {
    throw new Error(messageFromInvokeError(error));
  }
}

function messageFromInvokeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) {
      return message;
    }
  }

  return "Unable to fetch Codex usage";
}
