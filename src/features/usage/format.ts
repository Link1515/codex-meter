import { devMockCommandAlias } from "./defaults";
import type { CliUsageConfig, CodexUsageSnapshot, UsageStatus } from "./types";

export function formatPercent(value: number | undefined): string {
  return typeof value === "number" ? `${Math.round(value)}%` : "--";
}

export function formatTimestamp(value: string): string {
  if (!value) {
    return "Never";
  }

  const epochSeconds = Number(value);
  const date = Number.isFinite(epochSeconds) ? new Date(epochSeconds * 1000) : new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit"
  }).format(date);
}

export function statusLabel(status: UsageStatus): string {
  const labels: Record<UsageStatus, string> = {
    ok: "Ready",
    unknown: "No data",
    cli_not_found: "CLI missing",
    not_authenticated: "Sign in required",
    timeout: "Timed out",
    parse_error: "Parse failed",
    command_error: "Command failed"
  };

  return labels[status];
}

export function snapshotMessage(snapshot: CodexUsageSnapshot): string {
  if (snapshot.status === "ok") {
    return snapshot.model ? `Model ${snapshot.model}` : "Usage data is current";
  }

  return snapshot.errorMessage ?? statusLabel(snapshot.status);
}

export function commandPreview(config: CliUsageConfig): string {
  if (config.codexCommand === devMockCommandAlias) {
    return "mock codex usage";
  }

  return [config.codexCommand, ...config.usageArgs].join(" ");
}
