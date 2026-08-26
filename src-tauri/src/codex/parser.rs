use crate::codex::types::{
    clamp_percent, current_timestamp, CodexUsageSnapshot, ParserMode, UsageLimitSnapshot,
    UsageStatus,
};

pub trait UsageParser {
    fn parse(&self, input: &str) -> CodexUsageSnapshot;
}

pub struct TextUsageParser;
pub struct JsonUsageParser;

impl UsageParser for TextUsageParser {
    fn parse(&self, input: &str) -> CodexUsageSnapshot {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return CodexUsageSnapshot::with_status(
                UsageStatus::ParseError,
                Some("CLI output was empty".to_string()),
            );
        }

        let lower = trimmed.to_lowercase();
        if lower.contains("not logged in")
            || lower.contains("not authenticated")
            || lower.contains("please login")
            || lower.contains("please log in")
        {
            return CodexUsageSnapshot::with_status(
                UsageStatus::NotAuthenticated,
                Some("Codex CLI is not authenticated".to_string()),
            );
        }

        let mut snapshot = CodexUsageSnapshot::with_status(UsageStatus::Ok, None);
        let mut five_hour_limit = UsageLimitSnapshot::empty();
        let mut weekly_limit = UsageLimitSnapshot::empty();

        for line in trimmed.lines() {
            let normalized = line.trim();
            let lower_line = normalized.to_lowercase();

            if lower_line.starts_with("model:") {
                snapshot.model = value_after_colon(normalized);
            } else if lower_line.starts_with("plan:") || lower_line.starts_with("account plan:") {
                snapshot.account_plan = value_after_colon(normalized);
            } else if lower_line.starts_with("usage:") || lower_line.starts_with("used:") {
                five_hour_limit.usage_percent = percent_after_colon(normalized);
            } else if lower_line.starts_with("remaining:") {
                five_hour_limit.remaining_percent = percent_after_colon(normalized);
            } else if is_five_hour_usage_line(&lower_line) {
                five_hour_limit.usage_percent = percent_after_colon(normalized);
            } else if is_five_hour_remaining_line(&lower_line) {
                five_hour_limit.remaining_percent = percent_after_colon(normalized);
            } else if is_weekly_usage_line(&lower_line) {
                weekly_limit.usage_percent = percent_after_colon(normalized);
            } else if is_weekly_remaining_line(&lower_line) {
                weekly_limit.remaining_percent = percent_after_colon(normalized);
            } else if lower_line.starts_with("window resets at:") {
                five_hour_limit.reset_at = value_after_colon(normalized);
            } else if is_five_hour_reset_line(&lower_line) {
                five_hour_limit.reset_at = value_after_colon(normalized);
            } else if lower_line.starts_with("weekly resets at:") {
                weekly_limit.reset_at = value_after_colon(normalized);
            }
        }

        five_hour_limit.complete_percentages();
        weekly_limit.complete_percentages();

        if five_hour_limit.has_percent() || five_hour_limit.reset_at.is_some() {
            snapshot.five_hour_usage_limit = Some(five_hour_limit);
        }

        if weekly_limit.has_percent() || weekly_limit.reset_at.is_some() {
            snapshot.weekly_usage_limit = Some(weekly_limit);
        }

        if !snapshot_has_usage_limit(&snapshot) {
            return CodexUsageSnapshot::with_status(
                UsageStatus::ParseError,
                Some("No usage or remaining percentage found".to_string()),
            );
        }

        snapshot.fetched_at = current_timestamp();
        snapshot
    }
}

impl UsageParser for JsonUsageParser {
    fn parse(&self, input: &str) -> CodexUsageSnapshot {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return CodexUsageSnapshot::with_status(
                UsageStatus::ParseError,
                Some("CLI output was empty".to_string()),
            );
        }

        let parsed = serde_json::from_str::<serde_json::Value>(trimmed);
        let value = match parsed {
            Ok(value) => value,
            Err(_) => {
                return CodexUsageSnapshot::with_status(
                    UsageStatus::ParseError,
                    Some("CLI JSON output could not be parsed".to_string()),
                );
            }
        };

        let mut snapshot = CodexUsageSnapshot::with_status(UsageStatus::Ok, None);
        snapshot.five_hour_usage_limit = limit_field(
            &value,
            &[
                "fiveHourUsageLimit",
                "five_hour_usage_limit",
                "fiveHour",
                "primary",
            ],
        );
        snapshot.weekly_usage_limit = limit_field(
            &value,
            &[
                "weeklyUsageLimit",
                "weekly_usage_limit",
                "weekly",
                "secondary",
            ],
        );
        if snapshot.five_hour_usage_limit.is_none() && snapshot.weekly_usage_limit.is_none() {
            let mut five_hour_limit = UsageLimitSnapshot {
                usage_percent: percent_field(&value, &["usagePercent", "usage_percent", "usage"]),
                remaining_percent: percent_field(
                    &value,
                    &["remainingPercent", "remaining_percent", "remaining"],
                ),
                reset_at: string_field(
                    &value,
                    &["windowResetAt", "window_reset_at", "resetAt", "reset_at"],
                ),
            };
            five_hour_limit.complete_percentages();

            if five_hour_limit.has_percent() || five_hour_limit.reset_at.is_some() {
                snapshot.five_hour_usage_limit = Some(five_hour_limit);
            }
        }
        snapshot.account_plan = string_field(&value, &["accountPlan", "account_plan", "plan"]);
        snapshot.model = string_field(&value, &["model"]);
        snapshot.fetched_at = current_timestamp();

        if !snapshot_has_usage_limit(&snapshot) {
            return CodexUsageSnapshot::with_status(
                UsageStatus::ParseError,
                Some("No usage or remaining percentage found in JSON output".to_string()),
            );
        }

        snapshot
    }
}

pub fn parser_for(mode: &ParserMode) -> Box<dyn UsageParser> {
    match mode {
        ParserMode::Json => Box::new(JsonUsageParser),
        ParserMode::Text => Box::new(TextUsageParser),
    }
}

fn value_after_colon(line: &str) -> Option<String> {
    line.split_once(':')
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn percent_after_colon(line: &str) -> Option<f64> {
    value_after_colon(line).and_then(|value| {
        value
            .trim_end_matches('%')
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|percent| *percent >= 0.0 && *percent <= 100.0)
    })
}

fn is_weekly_usage_line(line: &str) -> bool {
    line.starts_with("weekly") && (line.contains("usage") || line.contains("used"))
}

fn is_weekly_remaining_line(line: &str) -> bool {
    line.starts_with("weekly") && (line.contains("remaining") || line.contains("left"))
}

fn is_five_hour_usage_line(line: &str) -> bool {
    has_five_hour_prefix(line) && (line.contains("usage") || line.contains("used"))
}

fn is_five_hour_remaining_line(line: &str) -> bool {
    has_five_hour_prefix(line) && (line.contains("remaining") || line.contains("left"))
}

fn is_five_hour_reset_line(line: &str) -> bool {
    has_five_hour_prefix(line) && line.contains("resets at:")
}

fn has_five_hour_prefix(line: &str) -> bool {
    line.starts_with("5 hour") || line.starts_with("5-hour") || line.starts_with("five hour")
}

fn snapshot_has_usage_limit(snapshot: &CodexUsageSnapshot) -> bool {
    snapshot
        .five_hour_usage_limit
        .as_ref()
        .is_some_and(UsageLimitSnapshot::has_percent)
        || snapshot
            .weekly_usage_limit
            .as_ref()
            .is_some_and(UsageLimitSnapshot::has_percent)
}

fn number_field(value: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|field| field.as_f64()))
}

fn percent_field(value: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    number_field(value, keys).map(clamp_percent)
}

fn string_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|field| field.as_str())
            .map(|field| field.to_string())
    })
}

fn limit_field(value: &serde_json::Value, keys: &[&str]) -> Option<UsageLimitSnapshot> {
    keys.iter().find_map(|key| {
        let field = value.get(*key)?;
        let mut limit = UsageLimitSnapshot {
            usage_percent: percent_field(
                field,
                &[
                    "usagePercent",
                    "usage_percent",
                    "usage",
                    "usedPercent",
                    "used_percent",
                ],
            ),
            remaining_percent: percent_field(
                field,
                &["remainingPercent", "remaining_percent", "remaining"],
            ),
            reset_at: string_field(field, &["resetAt", "reset_at", "resetsAt", "resets_at"]),
        };
        limit.complete_percentages();

        if limit.has_percent() || limit.reset_at.is_some() {
            Some(limit)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{JsonUsageParser, TextUsageParser, UsageParser};
    use crate::codex::types::UsageStatus;

    #[test]
    fn parses_text_usage() {
        let output = "Codex usage\nModel: gpt-5-codex\nUsage: 28%\nRemaining: 72%\nWindow resets at: 2026-01-01T18:00:00+08:00";
        let snapshot = TextUsageParser.parse(output);

        assert!(matches!(snapshot.status, UsageStatus::Ok));
        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.usage_percent),
            Some(28.0)
        );
        assert_eq!(snapshot.model, Some("gpt-5-codex".to_string()));
    }

    #[test]
    fn parses_text_weekly_limit() {
        let output =
            "Codex usage\n5 hour usage limit: 28%\n5 hour resets at: 2026-01-01T18:00:00+08:00\nWeekly usage limit: 45%\nWeekly resets at: 2026-01-04T00:00:00+08:00";
        let snapshot = TextUsageParser.parse(output);

        assert!(matches!(snapshot.status, UsageStatus::Ok));
        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(72.0)
        );
        assert_eq!(
            snapshot
                .weekly_usage_limit
                .as_ref()
                .and_then(|limit| limit.usage_percent),
            Some(45.0)
        );
    }

    #[test]
    fn rejects_empty_text() {
        let snapshot = TextUsageParser.parse("");

        assert!(matches!(snapshot.status, UsageStatus::ParseError));
    }

    #[test]
    fn parses_json_usage() {
        let snapshot = JsonUsageParser.parse(r#"{"usagePercent":28,"remainingPercent":72}"#);

        assert!(matches!(snapshot.status, UsageStatus::Ok));
        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(72.0)
        );
    }

    #[test]
    fn clamps_json_percentages() {
        let snapshot = JsonUsageParser.parse(r#"{"usagePercent":130}"#);

        assert!(matches!(snapshot.status, UsageStatus::Ok));
        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(0.0)
        );
    }

    #[test]
    fn parses_json_limit_windows() {
        let snapshot = JsonUsageParser
            .parse(r#"{"primary":{"usagePercent":28,"resetAt":"2026-01-01T18:00:00+08:00"},"secondary":{"usagePercent":45,"resetAt":"2026-01-04T00:00:00+08:00"}}"#);

        assert!(matches!(snapshot.status, UsageStatus::Ok));
        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(72.0)
        );
        assert_eq!(
            snapshot
                .weekly_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(55.0)
        );
    }
}
