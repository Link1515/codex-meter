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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

impl CodexUsageSnapshot {
    pub fn with_status(status: UsageStatus, message: Option<String>) -> Self {
        Self {
            source: "codex-cli".to_string(),
            fetched_at: current_timestamp(),
            raw_output: None,
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
