use crate::codex::{
    adapter::fetch_codex_usage,
    errors::AppError,
    types::{CliUsageConfig, CodexUsageSnapshot, ExecutionMode, UsageStatus},
};

#[tauri::command]
pub fn is_wsl_supported() -> bool {
    cfg!(windows)
}

#[tauri::command]
pub async fn fetch_usage(config: CliUsageConfig) -> Result<CodexUsageSnapshot, AppError> {
    validate_config(&config)?;

    let result = tauri::async_runtime::spawn_blocking(move || fetch_codex_usage(&config)).await;
    Ok(match result {
        Ok(snapshot) => snapshot,
        Err(_) => CodexUsageSnapshot::with_status(
            UsageStatus::CommandError,
            Some("Codex usage worker failed".to_string()),
        ),
    })
}

fn validate_config(config: &CliUsageConfig) -> Result<(), AppError> {
    if config.codex_command.trim().is_empty() || config.codex_command.contains('\0') {
        return Err(AppError::invalid_config(
            "Codex CLI command cannot be empty",
        ));
    }

    if config.usage_args.iter().any(|arg| arg.contains('\0')) {
        return Err(AppError::invalid_config(
            "Arguments cannot contain null characters",
        ));
    }
    if config.execution_mode == ExecutionMode::Wsl {
        if !cfg!(windows) {
            return Err(AppError::invalid_config("WSL mode requires Windows"));
        }
        for value in [&config.wsl_distribution, &config.wsl_user] {
            if value.contains('\0') || value.trim().starts_with('-') {
                return Err(AppError::invalid_config("Invalid WSL distribution or user"));
            }
        }
    }

    if config.timeout_seconds == 0 || config.timeout_seconds > 120 {
        return Err(AppError::invalid_config(
            "Timeout must be between 1 and 120 seconds",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> CliUsageConfig {
        CliUsageConfig {
            execution_mode: ExecutionMode::Native,
            wsl_distribution: String::new(),
            wsl_user: String::new(),
            codex_command: "codex".into(),
            usage_args: vec!["app-server".into()],
            timeout_seconds: 10,
            parser_mode: crate::codex::types::ParserMode::Json,
        }
    }

    #[test]
    fn validates_native_config_on_every_platform() {
        assert!(validate_config(&config()).is_ok());
    }

    #[test]
    fn rejects_null_arguments_and_invalid_timeouts() {
        let mut value = config();
        value.usage_args.push("\0".into());
        assert!(validate_config(&value).is_err());
        value.usage_args.clear();
        value.timeout_seconds = 121;
        assert!(validate_config(&value).is_err());
    }

    #[test]
    fn validates_wsl_platform_and_option_values() {
        let mut value = config();
        value.execution_mode = ExecutionMode::Wsl;
        assert_eq!(validate_config(&value).is_ok(), cfg!(windows));
        value.wsl_user = "--exec".into();
        assert!(validate_config(&value).is_err());
    }
}
