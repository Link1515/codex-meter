use std::{
    env, fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::codex::{
    parser::parser_for,
    types::{
        current_timestamp, CliUsageConfig, CodexUsageSnapshot, UsageLimitSnapshot, UsageStatus,
    },
};

pub const DEV_MOCK_COMMAND_ALIAS: &str = "__codex_meter_mock__";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

    let result = if should_try_oauth_usage(config) {
        fetch_oauth_usage().or_else(|_| fetch_app_server_rpc_usage(config))
    } else if is_app_server_rpc_config(config) {
        fetch_app_server_rpc_usage(config)
    } else {
        run_command(config).map(|result| snapshot_from_command_result(config, result))
    };

    match result {
        Ok(snapshot) => snapshot,
        Err(error) if error.kind() == io::ErrorKind::NotFound => CodexUsageSnapshot::with_status(
            UsageStatus::CliNotFound,
            Some("Codex CLI binary was not found".to_string()),
        ),
        Err(error) if error.kind() == io::ErrorKind::TimedOut => CodexUsageSnapshot::with_status(
            UsageStatus::Timeout,
            Some("Codex CLI command timed out".to_string()),
        ),
        Err(error) if is_authentication_error(&error.to_string()) => {
            CodexUsageSnapshot::with_status(
                UsageStatus::NotAuthenticated,
                Some("Codex CLI is not authenticated".to_string()),
            )
        }
        Err(error) => CodexUsageSnapshot::with_status(
            UsageStatus::CommandError,
            Some(format!(
                "Codex CLI command failed: {}",
                sanitize_message(&error.to_string())
            )),
        ),
    }
}

fn should_try_oauth_usage(config: &CliUsageConfig) -> bool {
    is_app_server_rpc_config(config) && config.codex_command.trim() != DEV_MOCK_COMMAND_ALIAS
}

fn is_authentication_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("authentication required")
        || lower.contains("not authenticated")
        || lower.contains("not logged in")
        || lower.contains("please login")
        || lower.contains("please log in")
}

fn fetch_oauth_usage() -> io::Result<CodexUsageSnapshot> {
    let access_token = read_codex_access_token()?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(format!("codex-meter/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    let response = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .bearer_auth(access_token)
        .send()
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    let status = response.status();

    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Codex OAuth token is missing or expired",
        ));
    }

    if !status.is_success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Codex OAuth usage API returned HTTP {}", status.as_u16()),
        ));
    }

    let value = response
        .json::<serde_json::Value>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;

    snapshot_from_oauth_usage(value)
}

fn read_codex_access_token() -> io::Result<String> {
    let auth_path = codex_auth_path()?;
    let raw = fs::read_to_string(auth_path)?;
    let auth = serde_json::from_str::<CodexAuthFile>(&raw)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;

    auth.tokens
        .and_then(|tokens| tokens.access_token)
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Codex OAuth access token missing"))
}

fn codex_auth_path() -> io::Result<PathBuf> {
    if let Ok(codex_home) = env::var("CODEX_HOME") {
        let trimmed = codex_home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed).join("auth.json"));
        }
    }

    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "User home directory not found"))?;

    Ok(PathBuf::from(home).join(".codex").join("auth.json"))
}

#[derive(Debug, serde::Deserialize)]
struct CodexAuthFile {
    tokens: Option<CodexAuthTokens>,
}

#[derive(Debug, serde::Deserialize)]
struct CodexAuthTokens {
    access_token: Option<String>,
}

pub(crate) fn is_app_server_rpc_config(config: &CliUsageConfig) -> bool {
    config
        .usage_args
        .iter()
        .any(|arg| arg.trim().eq_ignore_ascii_case("app-server"))
}

fn fetch_app_server_rpc_usage(config: &CliUsageConfig) -> io::Result<CodexUsageSnapshot> {
    let timeout = Duration::from_secs(config.timeout_seconds.max(1));
    let started_at = Instant::now();
    let command_spec = command_spec(config)?;
    let mut command = command_from_spec(&command_spec);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Codex app-server stdin unavailable")
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Codex app-server stdout unavailable")
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Codex app-server stderr unavailable")
    })?;

    let (line_tx, line_rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = line_tx.send(line);
        }
    });

    let (stderr_tx, stderr_rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let _ = stderr_tx.send(line);
        }
    });

    let rpc_result = (|| -> io::Result<CodexUsageSnapshot> {
        send_rpc_request(
            &mut stdin,
            1,
            "initialize",
            Some(serde_json::json!({
                "clientInfo": {
                    "name": "codex-meter",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        )?;
        let _ = read_rpc_response(&line_rx, 1, started_at, timeout)?;

        send_rpc_notification(&mut stdin, "initialized")?;

        send_rpc_request(&mut stdin, 2, "account/rateLimits/read", None)?;
        let rate_limits = read_rpc_response(&line_rx, 2, started_at, timeout)?;

        send_rpc_request(&mut stdin, 3, "account/read", None)?;
        let account = read_rpc_response(&line_rx, 3, started_at, timeout)
            .ok()
            .and_then(|value| serde_json::from_value::<RpcAccountResult>(value).ok());

        snapshot_from_rate_limits(rate_limits, account)
    })();

    let _ = child.kill();
    let _ = child.wait();

    match rpc_result {
        Ok(snapshot) => Ok(snapshot),
        Err(error) => {
            if error.kind() == io::ErrorKind::Other {
                if let Ok(stderr_line) = stderr_rx.try_recv() {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        sanitize_message(&stderr_line),
                    ));
                }
            }
            Err(error)
        }
    }
}

fn send_rpc_request(
    stdin: &mut impl Write,
    id: u64,
    method: &str,
    params: Option<serde_json::Value>,
) -> io::Result<()> {
    let payload = serde_json::json!({
        "id": id,
        "method": method,
        "params": params.unwrap_or_else(|| serde_json::json!({}))
    });

    writeln!(stdin, "{payload}")?;
    stdin.flush()
}

fn send_rpc_notification(stdin: &mut impl Write, method: &str) -> io::Result<()> {
    let payload = serde_json::json!({
        "method": method,
        "params": {}
    });

    writeln!(stdin, "{payload}")?;
    stdin.flush()
}

fn read_rpc_response(
    line_rx: &mpsc::Receiver<String>,
    id: u64,
    started_at: Instant,
    timeout: Duration,
) -> io::Result<serde_json::Value> {
    loop {
        let remaining = timeout.checked_sub(started_at.elapsed()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::TimedOut, "Codex app-server RPC timed out")
        })?;

        let line = line_rx
            .recv_timeout(remaining)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => {
                    io::Error::new(io::ErrorKind::TimedOut, "Codex app-server RPC timed out")
                }
                mpsc::RecvTimeoutError::Disconnected => io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Codex app-server closed stdout",
                ),
            })?;

        let value = match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if value.get("id").and_then(serde_json::Value::as_u64) != Some(id) {
            continue;
        }

        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Codex app-server RPC request failed");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                sanitize_message(message),
            ));
        }

        return value.get("result").cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Codex app-server RPC response did not include a result",
            )
        });
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcAccountResult {
    account: Option<RpcAccount>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcAccount {
    plan_type: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcRateLimitsResult {
    #[serde(alias = "rateLimits")]
    rate_limits: RpcRateLimitSnapshot,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcRateLimitSnapshot {
    primary: Option<RpcRateLimitWindow>,
    secondary: Option<RpcRateLimitWindow>,
    plan_type: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RpcRateLimitWindow {
    used_percent: f64,
    resets_at: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthUsageResponse {
    #[serde(alias = "plan_type")]
    plan_type: Option<String>,
    #[serde(alias = "rate_limit")]
    rate_limit: Option<OAuthRateLimitDetails>,
    #[serde(alias = "rateLimits")]
    rate_limits: Option<RpcRateLimitSnapshot>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthRateLimitDetails {
    #[serde(alias = "primary_window")]
    primary_window: Option<OAuthRateLimitWindow>,
    #[serde(alias = "secondary_window")]
    secondary_window: Option<OAuthRateLimitWindow>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuthRateLimitWindow {
    #[serde(alias = "used_percent")]
    used_percent: f64,
    #[serde(alias = "reset_at", alias = "resetsAt", alias = "resets_at")]
    resets_at: Option<u64>,
}

fn snapshot_from_oauth_usage(value: serde_json::Value) -> io::Result<CodexUsageSnapshot> {
    let response = serde_json::from_value::<OAuthUsageResponse>(value).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Codex OAuth usage response could not be parsed: {error}"),
        )
    })?;

    let mut snapshot = CodexUsageSnapshot::with_status(UsageStatus::Ok, None);
    snapshot.fetched_at = current_timestamp();
    snapshot.account_plan = response.plan_type;

    if let Some(rate_limit) = response.rate_limit {
        if let Some(primary) = rate_limit.primary_window {
            apply_oauth_primary_window(&mut snapshot, primary);
        }

        if let Some(secondary) = rate_limit.secondary_window {
            let weekly = limit_from_oauth_window(secondary);
            snapshot.weekly_reset_at = weekly.reset_at.clone();
            snapshot.weekly_usage_limit = Some(weekly);
        }
    }

    if let Some(rate_limits) = response.rate_limits {
        if snapshot.five_hour_usage_limit.is_none() {
            if let Some(primary) = rate_limits.primary {
                apply_primary_usage_limit(&mut snapshot, limit_from_rpc_window(primary));
            }
        }

        if snapshot.weekly_usage_limit.is_none() {
            if let Some(secondary) = rate_limits.secondary {
                let weekly = limit_from_rpc_window(secondary);
                snapshot.weekly_reset_at = weekly.reset_at.clone();
                snapshot.weekly_usage_limit = Some(weekly);
            }
        }

        snapshot.account_plan = snapshot.account_plan.or(rate_limits.plan_type);
    }

    if snapshot.five_hour_usage_limit.is_none()
        && snapshot.weekly_usage_limit.is_none()
        && snapshot.usage_percent.is_none()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Codex OAuth usage response contained no rate limit windows",
        ));
    }

    Ok(snapshot)
}

fn apply_oauth_primary_window(snapshot: &mut CodexUsageSnapshot, window: OAuthRateLimitWindow) {
    apply_primary_usage_limit(snapshot, limit_from_oauth_window(window));
}

fn limit_from_oauth_window(window: OAuthRateLimitWindow) -> UsageLimitSnapshot {
    UsageLimitSnapshot::from_usage_percent(
        window.used_percent,
        window.resets_at.map(|value| value.to_string()),
    )
}

fn snapshot_from_rate_limits(
    value: serde_json::Value,
    account: Option<RpcAccountResult>,
) -> io::Result<CodexUsageSnapshot> {
    let response = deserialize_rate_limits_result(value).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Codex app-server rate limits response could not be parsed: {error}"),
        )
    })?;

    let primary = response.rate_limits.primary.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Codex app-server returned no primary usage window",
        )
    })?;

    let mut snapshot = CodexUsageSnapshot::with_status(UsageStatus::Ok, None);
    snapshot.fetched_at = current_timestamp();
    apply_primary_usage_limit(&mut snapshot, limit_from_rpc_window(primary));
    let secondary = response.rate_limits.secondary.map(limit_from_rpc_window);
    snapshot.weekly_reset_at = secondary
        .as_ref()
        .and_then(|window| window.reset_at.clone());
    snapshot.weekly_usage_limit = secondary;
    snapshot.account_plan = response.rate_limits.plan_type.or_else(|| {
        account
            .and_then(|value| value.account)
            .and_then(|account| account.plan_type)
    });

    Ok(snapshot)
}

fn deserialize_rate_limits_result(
    value: serde_json::Value,
) -> Result<RpcRateLimitsResult, serde_json::Error> {
    if value.get("primary").is_some() || value.get("secondary").is_some() {
        serde_json::from_value(serde_json::json!({ "rateLimits": value }))
    } else {
        serde_json::from_value(value)
    }
}

fn limit_from_rpc_window(window: RpcRateLimitWindow) -> UsageLimitSnapshot {
    UsageLimitSnapshot::from_usage_percent(
        window.used_percent,
        window.resets_at.map(|value| value.to_string()),
    )
}

fn apply_primary_usage_limit(snapshot: &mut CodexUsageSnapshot, limit: UsageLimitSnapshot) {
    if let Some(usage_percent) = limit.usage_percent {
        snapshot.usage_percent = Some(usage_percent);
    }

    if let Some(remaining_percent) = limit.remaining_percent {
        snapshot.remaining_percent = Some(remaining_percent);
    }

    snapshot.window_reset_at = limit.reset_at.clone();
    snapshot.five_hour_usage_limit = Some(limit);
}

pub fn run_command(config: &CliUsageConfig) -> io::Result<CommandRunResult> {
    let timeout = Duration::from_secs(config.timeout_seconds.max(1));
    let command_spec = command_spec(config)?;
    let mut command = command_from_spec(&command_spec);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "command stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "command stderr unavailable"))?;
    let stdout_reader = thread::spawn(move || read_process_output(stdout));
    let stderr_reader = thread::spawn(move || read_process_output(stderr));

    let started_at = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let _ = child.wait();
            return Ok(CommandRunResult {
                exit_code: status.code(),
                stdout: join_process_output(stdout_reader)?,
                stderr: join_process_output(stderr_reader)?,
            });
        }

        if started_at.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = join_process_output(stdout_reader);
            let _ = join_process_output(stderr_reader);
            return Err(io::Error::new(io::ErrorKind::TimedOut, "command timed out"));
        }

        thread::sleep(Duration::from_millis(25));
    }
}

fn read_process_output(mut reader: impl Read) -> io::Result<String> {
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok(String::from_utf8_lossy(&output).to_string())
}

fn join_process_output(reader: thread::JoinHandle<io::Result<String>>) -> io::Result<String> {
    reader
        .join()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "command output reader panicked"))?
}

struct CommandSpec {
    program: String,
    args: Vec<String>,
}

fn command_from_spec(command_spec: &CommandSpec) -> Command {
    let mut command = Command::new(&command_spec.program);
    command.args(command_spec.args.iter().map(String::as_str));

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command
}

fn command_spec(config: &CliUsageConfig) -> io::Result<CommandSpec> {
    let command = config.codex_command.trim();

    if command == DEV_MOCK_COMMAND_ALIAS {
        return dev_mock_command_spec(&config.usage_args);
    }

    let mut args = config.usage_args.clone();

    #[cfg(windows)]
    if let Some((program, mut prefix_args)) = resolve_windows_node_shim(command) {
        prefix_args.append(&mut args);
        return Ok(CommandSpec {
            program,
            args: prefix_args,
        });
    }

    Ok(CommandSpec {
        program: resolve_program(command),
        args,
    })
}

fn resolve_program(command: &str) -> String {
    #[cfg(windows)]
    {
        resolve_windows_program(command).unwrap_or_else(|| command.to_string())
    }

    #[cfg(not(windows))]
    {
        command.to_string()
    }
}

#[cfg(windows)]
fn resolve_windows_program(command: &str) -> Option<String> {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 || command_path.extension().is_some() {
        return Some(command.to_string());
    }

    let path = env::var_os("PATH")?;
    let extensions = env::var_os("PATHEXT")
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());
    let extensions = extensions
        .split(';')
        .filter(|extension| !extension.trim().is_empty())
        .map(|extension| extension.trim().to_string())
        .collect::<Vec<_>>();

    for directory in env::split_paths(&path) {
        for extension in &extensions {
            let candidate = directory.join(format!("{command}{extension}"));
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    None
}

#[cfg(windows)]
fn resolve_windows_node_shim(command: &str) -> Option<(String, Vec<String>)> {
    let command_path = Path::new(command);
    let shim_path = if command_path.components().count() > 1 {
        command_path.to_path_buf()
    } else {
        find_windows_path_candidate(command, &[".CMD"])?
    };

    if shim_path.extension()?.to_string_lossy().to_lowercase() != "cmd" {
        return None;
    }

    let shim_dir = shim_path.parent()?;
    let script_path = shim_dir
        .join("node_modules")
        .join("@openai")
        .join("codex")
        .join("bin")
        .join("codex.js");

    if !script_path.exists() {
        return None;
    }

    let node_path = shim_dir.join("node.exe");
    let program = if node_path.exists() {
        node_path
    } else {
        PathBuf::from(resolve_windows_program("node").unwrap_or_else(|| "node".to_string()))
    };

    Some((
        program.to_string_lossy().to_string(),
        vec![script_path.to_string_lossy().to_string()],
    ))
}

#[cfg(windows)]
fn find_windows_path_candidate(command: &str, extensions: &[&str]) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;

    for directory in env::split_paths(&path) {
        for extension in extensions {
            let candidate = directory.join(format!("{command}{extension}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn dev_mock_command_spec(extra_args: &[String]) -> io::Result<CommandSpec> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = manifest_dir
        .join("..")
        .join("dev")
        .join("mock-codex-cli")
        .join("index.js");

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

    if trimmed.chars().count() > 240 {
        format!("{}...", trimmed.chars().take(240).collect::<String>())
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
    use super::{
        fetch_codex_usage, is_app_server_rpc_config, snapshot_from_oauth_usage, summary_text,
        DEV_MOCK_COMMAND_ALIAS,
    };
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

    #[test]
    fn truncates_unicode_summary_on_char_boundary() {
        let message = "錯".repeat(241);
        let summary = summary_text(&message, "");

        assert!(summary.ends_with("..."));
        assert_eq!(summary.trim_end_matches("...").chars().count(), 240);
    }

    #[test]
    fn detects_app_server_rpc_config() {
        let config = CliUsageConfig {
            codex_command: "codex".to_string(),
            usage_args: vec![
                "-s".to_string(),
                "read-only".to_string(),
                "app-server".to_string(),
            ],
            timeout_seconds: 10,
            parser_mode: ParserMode::Json,
        };

        assert!(is_app_server_rpc_config(&config));
    }

    #[test]
    fn parses_oauth_usage_response() {
        let snapshot = snapshot_from_oauth_usage(serde_json::json!({
            "plan_type": "plus",
            "rate_limit": {
                "primary_window": {
                    "used_percent": 28,
                    "reset_at": 1777969707
                },
                "secondary_window": {
                    "used_percent": 45,
                    "reset_at": 1778574507
                }
            }
        }))
        .expect("OAuth usage response should parse");

        assert!(matches!(snapshot.status, UsageStatus::Ok));
        assert_eq!(snapshot.account_plan, Some("plus".to_string()));
        assert_eq!(snapshot.usage_percent, Some(28.0));
        assert_eq!(snapshot.remaining_percent, Some(72.0));
        assert_eq!(snapshot.window_reset_at, Some("1777969707".to_string()));
        assert_eq!(
            snapshot
                .weekly_usage_limit
                .as_ref()
                .and_then(|limit| limit.usage_percent),
            Some(45.0)
        );
    }

    #[test]
    fn parses_oauth_rate_limits_shape() {
        let snapshot = snapshot_from_oauth_usage(serde_json::json!({
            "rateLimits": {
                "planType": "team",
                "primary": {
                    "usedPercent": 12,
                    "resetsAt": 1777969707
                },
                "secondary": {
                    "usedPercent": 20,
                    "resetsAt": 1778574507
                }
            }
        }))
        .expect("OAuth rateLimits response should parse");

        assert_eq!(snapshot.account_plan, Some("team".to_string()));
        assert_eq!(snapshot.usage_percent, Some(12.0));
        assert_eq!(
            snapshot
                .weekly_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(80.0)
        );
    }
}
