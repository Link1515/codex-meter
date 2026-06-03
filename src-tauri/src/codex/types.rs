use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliUsageConfig {
    pub codex_command: String,
    pub usage_args: Vec<String>,
    pub timeout_seconds: u64,
    pub parser_mode: ParserMode,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParserMode {
    #[serde(alias = "Json")]
    Json,
    #[serde(alias = "Text")]
    Text,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageStatus {
    Ok,
    Unknown,
    CliNotFound,
    NotAuthenticated,
    Timeout,
    ParseError,
    CommandError,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexUsageSnapshot {
    pub source: String,
    pub fetched_at: String,
    pub raw_output: Option<String>,
    pub five_hour_usage_limit: Option<UsageLimitSnapshot>,
    pub weekly_usage_limit: Option<UsageLimitSnapshot>,
    pub usage_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub used_tokens: Option<u64>,
    pub remaining_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub window_reset_at: Option<String>,
    pub weekly_reset_at: Option<String>,
    pub account_plan: Option<String>,
    pub model: Option<String>,
    pub status: UsageStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLimitSnapshot {
    pub usage_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub reset_at: Option<String>,
}

impl UsageLimitSnapshot {
    pub fn empty() -> Self {
        Self {
            usage_percent: None,
            remaining_percent: None,
            reset_at: None,
        }
    }

    pub fn from_usage_percent(usage_percent: f64, reset_at: Option<String>) -> Self {
        let usage_percent = usage_percent.clamp(0.0, 100.0);

        Self {
            usage_percent: Some(usage_percent),
            remaining_percent: Some((100.0 - usage_percent).max(0.0)),
            reset_at,
        }
    }

    pub fn complete_percentages(&mut self) {
        self.usage_percent = self.usage_percent.map(clamp_percent);
        self.remaining_percent = self.remaining_percent.map(clamp_percent);

        if self.remaining_percent.is_none() {
            self.remaining_percent = self.usage_percent.map(|used| (100.0 - used).max(0.0));
        }

        if self.usage_percent.is_none() {
            self.usage_percent = self
                .remaining_percent
                .map(|remaining| (100.0 - remaining).max(0.0));
        }
    }

    pub fn has_percent(&self) -> bool {
        self.usage_percent.is_some() || self.remaining_percent.is_some()
    }
}

pub fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

impl CodexUsageSnapshot {
    pub fn with_status(status: UsageStatus, message: Option<String>) -> Self {
        Self {
            source: "codex-cli".to_string(),
            fetched_at: current_timestamp(),
            raw_output: None,
            five_hour_usage_limit: None,
            weekly_usage_limit: None,
            usage_percent: None,
            remaining_percent: None,
            used_tokens: None,
            remaining_tokens: None,
            total_tokens: None,
            window_reset_at: None,
            weekly_reset_at: None,
            account_plan: None,
            model: None,
            status,
            error_message: message,
        }
    }
}

pub fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}", duration.as_secs()),
        Err(_) => "0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::ParserMode;

    #[test]
    fn deserializes_frontend_parser_mode_names() {
        assert!(matches!(
            serde_json::from_str::<ParserMode>(r#""Json""#).unwrap(),
            ParserMode::Json
        ));
        assert!(matches!(
            serde_json::from_str::<ParserMode>(r#""Text""#).unwrap(),
            ParserMode::Text
        ));
    }
}
