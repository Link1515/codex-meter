use crate::codex::{
    adapter::{fetch_codex_usage, run_command, sanitize_message, summary_text},
    errors::AppError,
    parser::parser_for,
    types::{CliUsageConfig, CodexUsageSnapshot},
};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliTestResult {
    pub exit_code: Option<i32>,
    pub stdout_summary: String,
    pub stderr_summary: String,
    pub parser_result: CodexUsageSnapshot,
}

#[tauri::command]
pub fn fetch_usage(config: CliUsageConfig) -> Result<CodexUsageSnapshot, AppError> {
    validate_config(&config)?;
    Ok(fetch_codex_usage(&config))
}

#[tauri::command]
pub fn test_cli_command(config: CliUsageConfig) -> Result<CliTestResult, AppError> {
    validate_config(&config)?;

    match run_command(&config) {
        Ok(result) => {
            let parser = parser_for(&config.parser_mode);
            let parser_result = parser.parse(&result.stdout);

            Ok(CliTestResult {
                exit_code: result.exit_code,
                stdout_summary: sanitize_message(&summary_text(&result.stdout, "")),
                stderr_summary: sanitize_message(&summary_text(&result.stderr, "")),
                parser_result,
            })
        }
        Err(error) => Err(AppError::invalid_config(format!(
            "Unable to run CLI command: {}",
            sanitize_message(&error.to_string())
        ))),
    }
}

fn validate_config(config: &CliUsageConfig) -> Result<(), AppError> {
    if config.codex_command.trim().is_empty() {
        return Err(AppError::invalid_config(
            "Codex CLI command cannot be empty",
        ));
    }

    if config.timeout_seconds == 0 || config.timeout_seconds > 120 {
        return Err(AppError::invalid_config(
            "Timeout must be between 1 and 120 seconds",
        ));
    }

    Ok(())
}
