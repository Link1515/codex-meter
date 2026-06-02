use std::{
    env,
    io,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::codex::{
    parser::parser_for,
    types::{CliUsageConfig, CodexUsageSnapshot, UsageStatus},
};

pub const DEV_MOCK_COMMAND_ALIAS: &str = "__codex_meter_mock__";

#[derive(Debug)]
pub struct CommandRunResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn fetch_codex_usage(config: &CliUsageConfig) -> CodexUsageSnapshot {
    if config.codex_command.trim().is_empty() {
        return CodexUsageSnapshot::with_status(
            UsageStatus::CommandError,
            Some("Codex CLI command is empty".to_string()),
        );
    }

    match run_command(config) {
        Ok(result) => snapshot_from_command_result(config, result),
        Err(error) if error.kind() == io::ErrorKind::NotFound => CodexUsageSnapshot::with_status(
            UsageStatus::CliNotFound,
            Some("Codex CLI binary was not found".to_string()),
        ),
        Err(error) if error.kind() == io::ErrorKind::TimedOut => CodexUsageSnapshot::with_status(
            UsageStatus::Timeout,
            Some("Codex CLI command timed out".to_string()),
        ),
        Err(error) => CodexUsageSnapshot::with_status(
            UsageStatus::CommandError,
            Some(format!(
                "Codex CLI command failed: {}",
                sanitize_message(&error.to_string())
            )),
        ),
    }
}

pub fn run_command(config: &CliUsageConfig) -> io::Result<CommandRunResult> {
    let timeout = Duration::from_secs(config.timeout_seconds.max(1));
    let command_spec = command_spec(config)?;
    let mut child = Command::new(&command_spec.program)
        .args(command_spec.args.iter().map(String::as_str))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let started_at = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            return Ok(CommandRunResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(io::ErrorKind::TimedOut, "command timed out"));
        }

        thread::sleep(Duration::from_millis(25));
    }
}

struct CommandSpec {
    program: String,
    args: Vec<String>,
}

fn command_spec(config: &CliUsageConfig) -> io::Result<CommandSpec> {
    let command = config.codex_command.trim();

    if command == DEV_MOCK_COMMAND_ALIAS {
        return dev_mock_command_spec(&config.usage_args);
    }

    Ok(CommandSpec {
        program: command.to_string(),
        args: config.usage_args.clone(),
    })
}

fn dev_mock_command_spec(extra_args: &[String]) -> io::Result<CommandSpec> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = manifest_dir.join("..").join("dev").join("mock-codex-cli").join("index.js");

    if !script_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Codex Meter mock CLI was not found",
        ));
    }

    let mut args = vec![script_path.to_string_lossy().to_string()];
    args.extend(extra_args.iter().cloned());

    Ok(CommandSpec {
        program: "node".to_string(),
        args,
    })
}

fn snapshot_from_command_result(
    config: &CliUsageConfig,
    result: CommandRunResult,
) -> CodexUsageSnapshot {
    if result.exit_code.unwrap_or(1) != 0 {
        let combined = format!("{} {}", result.stdout, result.stderr).to_lowercase();
        let status = if combined.contains("not authenticated")
            || combined.contains("not logged in")
            || combined.contains("please login")
            || combined.contains("please log in")
        {
            UsageStatus::NotAuthenticated
        } else {
            UsageStatus::CommandError
        };

        return CodexUsageSnapshot::with_status(
            status,
            Some(sanitize_message(&summary_text(
                &result.stderr,
                &result.stdout,
            ))),
        );
    }

    let parser = parser_for(&config.parser_mode);
    parser.parse(&result.stdout)
}

pub fn summary_text(primary: &str, fallback: &str) -> String {
    let source = if primary.trim().is_empty() {
        fallback
    } else {
        primary
    };
    let trimmed = source.trim();

    if trimmed.len() > 240 {
        format!("{}...", &trimmed[..240])
    } else if trimmed.is_empty() {
        "Codex CLI returned no output".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn sanitize_message(message: &str) -> String {
    let sensitive_terms = [
        "api_key",
        "apikey",
        "authorization",
        "access_token",
        "refresh_token",
        "session",
        "cookie",
        "token",
    ];

    let lower = message.to_lowercase();
    if sensitive_terms.iter().any(|term| lower.contains(term)) {
        "Command failed. Sensitive details were redacted.".to_string()
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{fetch_codex_usage, DEV_MOCK_COMMAND_ALIAS};
    use crate::codex::types::{CliUsageConfig, ParserMode, UsageStatus};

    #[test]
    fn fetches_usage_from_dev_mock_alias() {
        let snapshot = fetch_codex_usage(&CliUsageConfig {
            codex_command: DEV_MOCK_COMMAND_ALIAS.to_string(),
            usage_args: Vec::new(),
            timeout_seconds: 10,
            parser_mode: ParserMode::Text,
        });

        assert!(matches!(snapshot.status, UsageStatus::Ok));
        assert_eq!(snapshot.usage_percent, Some(28.0));
        assert_eq!(snapshot.remaining_percent, Some(72.0));
    }
}
