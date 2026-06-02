#!/usr/bin/env node

const mode = process.argv.includes("--json") ? "json" : "text";

if (process.argv.includes("--fail")) {
  console.error("mock codex command failed");
  process.exit(2);
}

if (process.argv.includes("--auth-error")) {
  console.error("not authenticated; please log in");
  process.exit(1);
}

if (mode === "json") {
  console.log(JSON.stringify({
    model: "gpt-5-codex",
    usagePercent: 28,
    remainingPercent: 72,
    windowResetAt: "2026-01-01T18:00:00+08:00",
    weeklyResetAt: "2026-01-04T00:00:00+08:00"
  }));
  process.exit(0);
}

console.log(`Codex usage
Model: gpt-5-codex
Usage: 28%
Remaining: 72%
Window resets at: 2026-01-01T18:00:00+08:00
Weekly resets at: 2026-01-04T00:00:00+08:00`);
