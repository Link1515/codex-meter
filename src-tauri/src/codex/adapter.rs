use std::{
    env, fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, OnceLock},
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::codex::{
    parser::parser_for,
    types::{
        current_timestamp, CliUsageConfig, CodexUsageSnapshot, ExecutionMode, UsageLimitSnapshot,
        UsageStatus,
    },
};

pub const DEV_MOCK_COMMAND_ALIAS: &str = "__codex_meter_mock__";
const MAX_COMMAND_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_RPC_LINE_BYTES: usize = 64 * 1024;
const MAX_QUEUED_RPC_LINES: usize = 16;

static OAUTH_USAGE_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

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

    let deadline = Instant::now() + Duration::from_secs(config.timeout_seconds.clamp(1, 120));
    let result = if should_try_oauth_usage(config) {
        fetch_oauth_usage(deadline).or_else(|_| {
            remaining_timeout(deadline)?;
            fetch_app_server_rpc_usage(config, deadline)
        })
    } else if is_app_server_rpc_config(config) {
        fetch_app_server_rpc_usage(config, deadline)
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
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            CodexUsageSnapshot::with_status(
                UsageStatus::NotAuthenticated,
                Some("Codex CLI is not authenticated".to_string()),
            )
        }
        Err(_) => CodexUsageSnapshot::with_status(
            UsageStatus::CommandError,
            Some("Codex CLI command failed".to_string()),
        ),
    }
}

fn should_try_oauth_usage(config: &CliUsageConfig) -> bool {
    config.execution_mode == ExecutionMode::Native
        && is_app_server_rpc_config(config)
        && config.codex_command.trim() != DEV_MOCK_COMMAND_ALIAS
}

fn is_authentication_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("authentication required")
        || lower.contains("not authenticated")
        || lower.contains("not logged in")
        || lower.contains("please login")
        || lower.contains("please log in")
}

fn fetch_oauth_usage(deadline: Instant) -> io::Result<CodexUsageSnapshot> {
    let access_token = read_codex_access_token()?;
    let client = oauth_usage_client()?;
    let response = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .timeout(remaining_timeout(deadline)?)
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

    remaining_timeout(deadline)?;
    snapshot_from_oauth_usage(value)
}

fn oauth_usage_client() -> io::Result<&'static reqwest::blocking::Client> {
    if let Some(client) = OAUTH_USAGE_CLIENT.get() {
        return Ok(client);
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("codex-meter/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
    let _ = OAUTH_USAGE_CLIENT.set(client);

    OAUTH_USAGE_CLIENT.get().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Codex OAuth usage client was unavailable after initialization",
        )
    })
}

fn remaining_timeout(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Codex usage request timed out"))
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

fn fetch_app_server_rpc_usage(
    config: &CliUsageConfig,
    deadline: Instant,
) -> io::Result<CodexUsageSnapshot> {
    remaining_timeout(deadline)?;
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

    let (line_tx, line_rx) = mpsc::sync_channel::<io::Result<String>>(MAX_QUEUED_RPC_LINES);
    thread::spawn(move || forward_rpc_lines(stdout, line_tx));

    let stderr_reader = thread::spawn(move || read_process_output(stderr));

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
        let _ = read_rpc_response(&line_rx, 1, deadline)?;

        send_rpc_notification(&mut stdin, "initialized")?;

        send_rpc_request(&mut stdin, 2, "account/rateLimits/read", None)?;
        let rate_limits = read_rpc_response(&line_rx, 2, deadline)?;

        send_rpc_request(&mut stdin, 3, "account/read", None)?;
        let account = read_rpc_response(&line_rx, 3, deadline)
            .ok()
            .and_then(|value| serde_json::from_value::<RpcAccountResult>(value).ok());

        snapshot_from_rate_limits(rate_limits, account)
    })();

    let _ = child.kill();
    let _ = child.wait();
    let stderr_result = join_process_output_within(
        stderr_reader,
        remaining_timeout(deadline)
            .unwrap_or_default()
            .min(Duration::from_millis(100)),
    );

    match rpc_result {
        Ok(snapshot) => {
            stderr_result?;
            Ok(snapshot)
        }
        Err(error) => Err(app_server_error_with_stderr(
            error,
            stderr_result
                .unwrap_or_default()
                .as_deref()
                .unwrap_or_default(),
        )),
    }
}

fn forward_rpc_lines(reader: impl Read, sender: mpsc::SyncSender<io::Result<String>>) {
    let mut reader = BufReader::new(reader);
    loop {
        let mut line = Vec::new();
        let result = (&mut reader)
            .take((MAX_RPC_LINE_BYTES + 1) as u64)
            .read_until(b'\n', &mut line);
        let next_line = match result {
            Ok(0) => break,
            Ok(count) if count > MAX_RPC_LINE_BYTES => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Codex app-server output line exceeded the size limit",
            )),
            Ok(_) => String::from_utf8(line).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Codex app-server output was not UTF-8",
                )
            }),
            Err(error) => Err(error),
        };
        let failed = next_line.is_err();
        if sender.send(next_line).is_err() || failed {
            break;
        }
    }
}

fn app_server_error_with_stderr(error: io::Error, stderr: &str) -> io::Error {
    if error.kind() == io::ErrorKind::PermissionDenied || is_authentication_error(stderr) {
        return io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Codex CLI is not authenticated",
        );
    }

    error
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
    line_rx: &mpsc::Receiver<io::Result<String>>,
    id: u64,
    deadline: Instant,
) -> io::Result<serde_json::Value> {
    loop {
        let remaining = remaining_timeout(deadline)?;

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
            })??;

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
            let kind = if is_authentication_error(message) {
                io::ErrorKind::PermissionDenied
            } else {
                io::ErrorKind::Other
            };
            return Err(io::Error::new(kind, "Codex app-server RPC request failed"));
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
        snapshot.five_hour_usage_limit = rate_limit.primary_window.map(limit_from_oauth_window);
        snapshot.weekly_usage_limit = rate_limit.secondary_window.map(limit_from_oauth_window);
    }

    if let Some(rate_limits) = response.rate_limits {
        snapshot.five_hour_usage_limit = snapshot
            .five_hour_usage_limit
            .or_else(|| rate_limits.primary.map(limit_from_rpc_window));
        snapshot.weekly_usage_limit = snapshot
            .weekly_usage_limit
            .or_else(|| rate_limits.secondary.map(limit_from_rpc_window));

        snapshot.account_plan = snapshot.account_plan.or(rate_limits.plan_type);
    }

    if snapshot.five_hour_usage_limit.is_none() && snapshot.weekly_usage_limit.is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Codex OAuth usage response contained no rate limit windows",
        ));
    }

    Ok(snapshot)
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

    let five_hour_window = response.rate_limits.primary.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Codex app-server returned no 5-hour usage window",
        )
    })?;

    let mut snapshot = CodexUsageSnapshot::with_status(UsageStatus::Ok, None);
    snapshot.fetched_at = current_timestamp();
    snapshot.five_hour_usage_limit = Some(limit_from_rpc_window(five_hour_window));
    snapshot.weekly_usage_limit = response.rate_limits.secondary.map(limit_from_rpc_window);
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

    wait_for_command_result(child, stdout_reader, stderr_reader, timeout)
}

fn wait_for_command_result(
    mut child: Child,
    stdout_reader: thread::JoinHandle<io::Result<String>>,
    stderr_reader: thread::JoinHandle<io::Result<String>>,
    timeout: Duration,
) -> io::Result<CommandRunResult> {
    let started_at = Instant::now();
    let mut exit_status = None;
    loop {
        if exit_status.is_none() {
            exit_status = child.try_wait()?;
        }

        if let Some(status) = exit_status.as_ref() {
            if stdout_reader.is_finished() && stderr_reader.is_finished() {
                let _ = child.wait();
                return Ok(CommandRunResult {
                    exit_code: status.code(),
                    stdout: join_process_output(stdout_reader)?,
                    stderr: join_process_output(stderr_reader)?,
                });
            }
        }

        if started_at.elapsed() >= timeout {
            if exit_status.is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
            // Joining a reader here can block indefinitely if a descendant
            // inherited stdout or stderr. Dropping the handles detaches them.
            return Err(io::Error::new(io::ErrorKind::TimedOut, "command timed out"));
        }

        thread::sleep(Duration::from_millis(25));
    }
}

fn read_process_output(reader: impl Read) -> io::Result<String> {
    let mut output = Vec::new();
    reader
        .take((MAX_COMMAND_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut output)?;
    if output.len() > MAX_COMMAND_OUTPUT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Codex CLI output exceeded the size limit",
        ));
    }
    Ok(String::from_utf8_lossy(&output).to_string())
}

fn join_process_output(reader: thread::JoinHandle<io::Result<String>>) -> io::Result<String> {
    reader
        .join()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "command output reader panicked"))?
}

fn join_process_output_within(
    reader: thread::JoinHandle<io::Result<String>>,
    wait: Duration,
) -> io::Result<Option<String>> {
    let started_at = Instant::now();
    while !reader.is_finished() {
        if started_at.elapsed() >= wait {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(10));
    }
    join_process_output(reader).map(Some)
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

    if config.execution_mode == ExecutionMode::Wsl {
        if !cfg!(windows) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "WSL mode requires Windows",
            ));
        }
        return Ok(wsl_command_spec(config));
    }

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

fn wsl_command_spec(config: &CliUsageConfig) -> CommandSpec {
    let mut args = Vec::new();
    for (flag, value) in [
        ("--distribution", &config.wsl_distribution),
        ("--user", &config.wsl_user),
    ] {
        if !value.trim().is_empty() {
            args.extend([flag.to_string(), value.trim().to_string()]);
        }
    }
    // A fixed shell program loads the same PATH as an interactive Codex session.
    // All user input stays in positional arguments, never in shell source.
    args.extend(
        [
            "--cd",
            "~",
            "--exec",
            "bash",
            "-lic",
            "exec \"$@\"",
            "codex-meter",
        ]
        .map(str::to_string),
    );
    args.push(config.codex_command.trim().to_string());
    args.extend(config.usage_args.clone());
    CommandSpec {
        program: "wsl.exe".to_string(),
        args,
    }
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

        let message = if status == UsageStatus::NotAuthenticated {
            "Codex CLI is not authenticated"
        } else {
            "Codex CLI command failed"
        };
        return CodexUsageSnapshot::with_status(status, Some(message.to_string()));
    }

    let parser = parser_for(&config.parser_mode);
    parser.parse(&result.stdout)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Cursor},
        sync::mpsc,
        time::{Duration, Instant},
    };

    use super::{
        app_server_error_with_stderr, fetch_codex_usage, is_app_server_rpc_config,
        oauth_usage_client, remaining_timeout, snapshot_from_oauth_usage,
        snapshot_from_rate_limits, CommandRunResult, DEV_MOCK_COMMAND_ALIAS,
    };
    use crate::codex::types::{CliUsageConfig, ParserMode, UsageStatus};

    fn wsl_config() -> CliUsageConfig {
        CliUsageConfig {
            execution_mode: super::ExecutionMode::Wsl,
            wsl_distribution: String::new(),
            wsl_user: String::new(),
            codex_command: "codex".into(),
            usage_args: vec!["app-server".into()],
            timeout_seconds: 20,
            parser_mode: ParserMode::Json,
        }
    }

    #[test]
    fn wsl_uses_positional_arguments_and_the_selected_account() {
        let mut config = wsl_config();
        config.wsl_distribution = " Ubuntu ".into();
        config.wsl_user = "terry".into();
        config.codex_command = "/home/terry/a b/codex".into();
        config
            .usage_args
            .push("$(touch /tmp/should-not-exist)".into());
        let spec = super::wsl_command_spec(&config);
        assert_eq!(spec.program, "wsl.exe");
        assert_eq!(
            spec.args,
            [
                "--distribution",
                "Ubuntu",
                "--user",
                "terry",
                "--cd",
                "~",
                "--exec",
                "bash",
                "-lic",
                "exec \"$@\"",
                "codex-meter",
                "/home/terry/a b/codex",
                "app-server",
                "$(touch /tmp/should-not-exist)"
            ]
        );
        assert!(!super::should_try_oauth_usage(&config));
        config.execution_mode = super::ExecutionMode::Native;
        assert!(super::should_try_oauth_usage(&config));
    }

    #[test]
    fn wsl_defaults_to_the_default_distribution_and_user() {
        let spec = super::wsl_command_spec(&wsl_config());
        assert_eq!(&spec.args[..2], ["--cd", "~"]);
        assert!(!spec
            .args
            .iter()
            .any(|arg| arg == "--distribution" || arg == "--user"));
    }

    #[test]
    fn fetches_usage_from_dev_mock_alias() {
        let snapshot = fetch_codex_usage(&CliUsageConfig {
            execution_mode: Default::default(),
            wsl_distribution: String::new(),
            wsl_user: String::new(),
            codex_command: DEV_MOCK_COMMAND_ALIAS.to_string(),
            usage_args: Vec::new(),
            timeout_seconds: 10,
            parser_mode: ParserMode::Text,
        });

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

    #[test]
    fn command_timeout_does_not_wait_for_descendants_holding_output_pipes() {
        for keep_parent_running in [false, true] {
            let script = if keep_parent_running {
                "setTimeout(() => {}, 4000)"
            } else {
                "process.exit(0)"
            };
            let child = std::process::Command::new("node")
                .args(["-e", script])
                .spawn()
                .expect("Node should start for the command timeout test");
            let (release_reader, reader_gate) = mpsc::channel::<()>();
            let blocked_reader = std::thread::spawn(move || {
                let _ = reader_gate.recv();
                Ok(String::new())
            });
            let finished_reader = std::thread::spawn(|| Ok(String::new()));

            let started_at = Instant::now();
            let result = super::wait_for_command_result(
                child,
                blocked_reader,
                finished_reader,
                Duration::from_secs(1),
            );
            drop(release_reader);
            let error = result.expect_err("open output pipes should time out");
            assert_eq!(error.kind(), io::ErrorKind::TimedOut);
            assert!(started_at.elapsed() < Duration::from_secs(3));
        }
    }

    #[test]
    fn app_server_cleanup_does_not_wait_forever_for_stderr() {
        let (release_reader, reader_gate) = mpsc::channel::<()>();
        let blocked_reader = std::thread::spawn(move || {
            let _ = reader_gate.recv();
            Ok(String::new())
        });

        let started_at = Instant::now();
        let output = super::join_process_output_within(blocked_reader, Duration::from_millis(50))
            .expect("waiting for stderr should not fail");
        drop(release_reader);

        assert!(output.is_none());
        assert!(started_at.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn reuses_the_oauth_usage_client() {
        let first = oauth_usage_client().expect("OAuth client should initialize");
        let second = oauth_usage_client().expect("OAuth client should remain available");

        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn reports_when_the_shared_usage_deadline_expires() {
        let deadline = Instant::now() + Duration::from_secs(45);
        let remaining = remaining_timeout(deadline).expect("future deadline should have time left");
        assert!(remaining <= Duration::from_secs(45));
        assert!(remaining > Duration::from_secs(44));
        assert_eq!(
            remaining_timeout(Instant::now() - Duration::from_millis(1))
                .expect_err("expired deadline should time out")
                .kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn app_server_uses_only_the_remaining_usage_time() {
        let config = CliUsageConfig {
            execution_mode: Default::default(),
            wsl_distribution: String::new(),
            wsl_user: String::new(),
            codex_command: "node".to_string(),
            usage_args: vec![
                "-e".to_string(),
                "setTimeout(() => {}, 4000)".to_string(),
                "app-server".to_string(),
            ],
            timeout_seconds: 10,
            parser_mode: ParserMode::Json,
        };
        let started_at = Instant::now();
        let deadline = started_at + Duration::from_millis(500);
        let error = super::fetch_app_server_rpc_usage(&config, deadline)
            .expect_err("unresponsive app-server should time out at the shared deadline");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started_at.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn hides_cli_error_output_from_snapshots() {
        let snapshot = super::snapshot_from_command_result(
            &wsl_config(),
            CommandRunResult {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "sk-proj-secret-value".to_string(),
            },
        );

        assert_eq!(snapshot.status, UsageStatus::CommandError);
        assert_eq!(
            snapshot.error_message.as_deref(),
            Some("Codex CLI command failed")
        );
    }

    #[test]
    fn limits_command_output_without_truncating_valid_output() {
        let at_limit = vec![b'a'; super::MAX_COMMAND_OUTPUT_BYTES];
        assert_eq!(
            super::read_process_output(Cursor::new(at_limit))
                .expect("Output at the limit should be accepted")
                .len(),
            super::MAX_COMMAND_OUTPUT_BYTES
        );

        let over_limit = vec![b'a'; super::MAX_COMMAND_OUTPUT_BYTES + 1];
        let error = super::read_process_output(Cursor::new(over_limit))
            .expect_err("Oversized output should be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_oversized_app_server_lines() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let over_limit = vec![b'a'; super::MAX_RPC_LINE_BYTES + 1];
        super::forward_rpc_lines(Cursor::new(over_limit), sender);

        let error = receiver
            .recv()
            .expect("Oversized line should be reported")
            .expect_err("Oversized line should be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn detects_app_server_rpc_config() {
        let config = CliUsageConfig {
            execution_mode: Default::default(),
            wsl_distribution: String::new(),
            wsl_user: String::new(),
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
    fn hides_app_server_stderr_when_stdout_closes() {
        let error = app_server_error_with_stderr(
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Codex app-server closed stdout",
            ),
            "sk-proj-secret-value",
        );

        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(error.to_string(), "Codex app-server closed stdout");
    }

    #[test]
    fn classifies_app_server_authentication_errors_without_exposing_stderr() {
        let error = app_server_error_with_stderr(
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Codex app-server closed stdout",
            ),
            "not authenticated: sk-proj-secret-value",
        );

        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "Codex CLI is not authenticated");
    }

    #[test]
    fn hides_rpc_error_messages() {
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Ok(
                r#"{"id":1,"error":{"message":"sk-proj-secret-value"}}"#.to_string()
            ))
            .expect("RPC response should be queued");

        let error = super::read_rpc_response(&receiver, 1, Instant::now() + Duration::from_secs(1))
            .expect_err("RPC error should fail the request");

        assert_eq!(error.to_string(), "Codex app-server RPC request failed");
    }

    #[test]
    fn maps_a_single_app_server_window_to_the_five_hour_limit() {
        let snapshot = snapshot_from_rate_limits(
            serde_json::json!({
                "primary": {
                    "usedPercent": 20,
                    "resetsAt": 1778574507
                }
            }),
            None,
        )
        .expect("A single rate-limit window should parse");

        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(80.0)
        );
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
        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.usage_percent),
            Some(28.0)
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
        assert_eq!(
            snapshot
                .five_hour_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(88.0)
        );
        assert_eq!(
            snapshot
                .weekly_usage_limit
                .as_ref()
                .and_then(|limit| limit.remaining_percent),
            Some(80.0)
        );
    }
}
