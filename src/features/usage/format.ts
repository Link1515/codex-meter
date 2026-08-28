import type { CodexUsageSnapshot, UsageStatus } from "./types";

export function formatPercent(value: number | undefined): string {
  return typeof value === "number" && Number.isFinite(value) ? `${Math.round(value)}%` : "--";
}

export function formatCompactResetTimestamp(value: string | undefined, now = new Date()): string {
  if (!value) {
    return "--";
  }

  const date = parseTimestamp(value);

  if (!date) {
    return value;
  }

  if (isSameLocalDay(date, now)) {
    return formatTwelveHourTime(date);
  }

  return `${shortWeekday(date)} ${formatTwelveHourTime(date)}`;
}

export function formatDatedResetTimestamp(value: string | undefined): string {
  if (!value) {
    return "--";
  }

  const date = parseTimestamp(value);

  if (!date) {
    return value;
  }

  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())} · ${shortWeekday(date)}\n${formatTwelveHourTime(date)}`;
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

function shortWeekday(date: Date): string {
  return ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][date.getDay()];
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
