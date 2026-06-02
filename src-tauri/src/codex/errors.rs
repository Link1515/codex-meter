use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorKind {
    InvalidConfig,
    WindowControlFailed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: AppErrorKind,
    pub message: String,
}

impl AppError {
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::InvalidConfig,
            message: message.into(),
        }
    }

    pub fn window_control_failed(message: impl Into<String>) -> Self {
        Self {
            kind: AppErrorKind::WindowControlFailed,
            message: message.into(),
        }
    }
}
