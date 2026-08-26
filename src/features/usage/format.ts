import { devMockCommandAlias } from "./defaults";
import type { CliUsageConfig, CodexUsageSnapshot, UsageLimitSnapshot, UsageStatus } from "./types";

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

export function formatResetTimestamp(value: string | undefined, now = new Date()): string {
  if (!value) {
    return "--";
  }

  const date = parseTimestamp(value);

  if (!date) {
    return value;
  }

  const time = formatTwelveHourTime(date);
  const remainingDays = formatRemainingDays(date, now);

  if (isSameLocalDay(date, now)) {
    return `${time}\n${remainingDays}`;
  }

  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())} ${time}\n${remainingDays}`;
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

function parseTimestamp(value: string): Date | undefined {
  const epochSeconds = Number(value);
  const date = Number.isFinite(epochSeconds) ? new Date(epochSeconds * 1000) : new Date(value);

  return Number.isNaN(date.getTime()) ? undefined : date;
}

function formatTwelveHourTime(date: Date): string {
  const hours = date.getHours();
  const period = hours >= 12 ? "PM" : "AM";
  const twelveHour = hours % 12 || 12;

  return `${pad2(twelveHour)}:${pad2(date.getMinutes())} ${period}`;
}

function isSameLocalDay(left: Date, right: Date): boolean {
  return (
    left.getFullYear() === right.getFullYear() &&
    left.getMonth() === right.getMonth() &&
    left.getDate() === right.getDate()
  );
}

function formatRemainingDays(resetAt: Date, now: Date): string {
  const millisecondsPerDay = 24 * 60 * 60 * 1000;
  const remainingMilliseconds = Math.max(0, resetAt.getTime() - now.getTime());
  const days = Math.ceil(remainingMilliseconds / millisecondsPerDay);

  return `${days} day${days === 1 ? "" : "s"} remaining`;
}

function pad2(value: number): string {
  return value.toString().padStart(2, "0");
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

export function resolveWeeklyLimit(snapshot: CodexUsageSnapshot): UsageLimitSnapshot {
  return {
    usagePercent: snapshot.weeklyUsageLimit?.usagePercent,
    remainingPercent: snapshot.weeklyUsageLimit?.remainingPercent,
    resetAt: snapshot.weeklyUsageLimit?.resetAt
  };
}

export function resolveFiveHourLimit(snapshot: CodexUsageSnapshot): UsageLimitSnapshot {
  return {
    usagePercent: snapshot.fiveHourUsageLimit?.usagePercent,
    remainingPercent: snapshot.fiveHourUsageLimit?.remainingPercent,
    resetAt: snapshot.fiveHourUsageLimit?.resetAt
  };
}
