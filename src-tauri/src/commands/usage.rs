use crate::codex::{
    adapter::fetch_codex_usage,
    errors::AppError,
    types::{CliUsageConfig, CodexUsageSnapshot},
};

#[tauri::command]
pub fn fetch_usage(config: CliUsageConfig) -> Result<CodexUsageSnapshot, AppError> {
    validate_config(&config)?;
    Ok(fetch_codex_usage(&config))
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
