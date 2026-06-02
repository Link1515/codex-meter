use crate::codex::types::{current_timestamp, CodexUsageSnapshot, ParserMode, UsageStatus};

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
        snapshot.raw_output = Some(trimmed.to_string());

        for line in trimmed.lines() {
            let normalized = line.trim();
            let lower_line = normalized.to_lowercase();

            if lower_line.starts_with("model:") {
                snapshot.model = value_after_colon(normalized);
            } else if lower_line.starts_with("plan:") || lower_line.starts_with("account plan:") {
                snapshot.account_plan = value_after_colon(normalized);
            } else if lower_line.starts_with("usage:") || lower_line.starts_with("used:") {
                snapshot.usage_percent = percent_after_colon(normalized);
            } else if lower_line.starts_with("remaining:") {
                snapshot.remaining_percent = percent_after_colon(normalized);
            } else if lower_line.starts_with("window resets at:") {
                snapshot.window_reset_at = value_after_colon(normalized);
            } else if lower_line.starts_with("weekly resets at:") {
                snapshot.weekly_reset_at = value_after_colon(normalized);
            }
        }

        if snapshot.usage_percent.is_none() && snapshot.remaining_percent.is_none() {
            return CodexUsageSnapshot::with_status(
                UsageStatus::ParseError,
                Some("No usage or remaining percentage found".to_string()),
            );
        }

        if snapshot.remaining_percent.is_none() {
            snapshot.remaining_percent = snapshot.usage_percent.map(|used| (100.0 - used).max(0.0));
        }

        if snapshot.usage_percent.is_none() {
            snapshot.usage_percent = snapshot
                .remaining_percent
                .map(|remaining| (100.0 - remaining).max(0.0));
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
        snapshot.raw_output = Some(trimmed.to_string());
        snapshot.usage_percent = number_field(&value, &["usagePercent", "usage_percent", "usage"]);
        snapshot.remaining_percent = number_field(
            &value,
            &["remainingPercent", "remaining_percent", "remaining"],
        );
        snapshot.used_tokens = integer_field(&value, &["usedTokens", "used_tokens"]);
        snapshot.remaining_tokens = integer_field(&value, &["remainingTokens", "remaining_tokens"]);
        snapshot.total_tokens = integer_field(&value, &["totalTokens", "total_tokens"]);
        snapshot.window_reset_at = string_field(&value, &["windowResetAt", "window_reset_at"]);
        snapshot.weekly_reset_at = string_field(&value, &["weeklyResetAt", "weekly_reset_at"]);
        snapshot.account_plan = string_field(&value, &["accountPlan", "account_plan", "plan"]);
        snapshot.model = string_field(&value, &["model"]);
        snapshot.fetched_at = current_timestamp();

        if snapshot.usage_percent.is_none() && snapshot.remaining_percent.is_none() {
            return CodexUsageSnapshot::with_status(
                UsageStatus::ParseError,
                Some("No usage or remaining percentage found in JSON output".to_string()),
            );
        }

        if snapshot.remaining_percent.is_none() {
            snapshot.remaining_percent = snapshot.usage_percent.map(|used| (100.0 - used).max(0.0));
        }

        if snapshot.usage_percent.is_none() {
            snapshot.usage_percent = snapshot
                .remaining_percent
                .map(|remaining| (100.0 - remaining).max(0.0));
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

fn number_field(value: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|field| field.as_f64()))
}

fn integer_field(value: &serde_json::Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|field| field.as_u64()))
}

fn string_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|field| field.as_str())
            .map(|field| field.to_string())
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
        assert_eq!(snapshot.usage_percent, Some(28.0));
        assert_eq!(snapshot.remaining_percent, Some(72.0));
        assert_eq!(snapshot.model, Some("gpt-5-codex".to_string()));
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
        assert_eq!(snapshot.usage_percent, Some(28.0));
        assert_eq!(snapshot.remaining_percent, Some(72.0));
    }
}
