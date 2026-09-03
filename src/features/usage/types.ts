export type ParserMode = "Text" | "Json";

export type UsageStatus =
  | "ok"
  | "unknown"
  | "cli_not_found"
  | "not_authenticated"
  | "timeout"
  | "parse_error"
  | "command_error";

export type CliUsageConfig = {
  codexCommand: string;
  usageArgs: string[];
  pollIntervalSeconds: number;
  timeoutSeconds: number;
  parserMode: ParserMode;
};

export type CodexUsageSnapshot = {
  source: "codex-cli";
  fetchedAt: string;
  fiveHourUsageLimit?: UsageLimitSnapshot;
  weeklyUsageLimit?: UsageLimitSnapshot;
  accountPlan?: string;
  model?: string;
  status: UsageStatus;
  errorMessage?: string;
};

export type UsageLimitSnapshot = {
  usagePercent?: number;
  remainingPercent?: number;
  resetAt?: string;
};

export type UsageViewState =
  | { kind: "idle"; snapshot: CodexUsageSnapshot }
  | { kind: "loading"; snapshot: CodexUsageSnapshot }
  | { kind: "ready"; snapshot: CodexUsageSnapshot }
  | { kind: "failed"; snapshot: CodexUsageSnapshot };
