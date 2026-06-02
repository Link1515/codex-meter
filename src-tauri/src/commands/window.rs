use tauri::{Manager, Window};

use crate::codex::errors::AppError;

#[tauri::command]
pub fn set_always_on_top(window: Window, enabled: bool) -> Result<bool, AppError> {
    window.set_always_on_top(enabled).map_err(|error| {
        AppError::window_control_failed(format!("Unable to update always-on-top: {}", error))
    })?;

    Ok(enabled)
}

#[tauri::command]
pub fn get_always_on_top(window: Window) -> Result<bool, AppError> {
    let label = window.label().to_string();
    let app = window.app_handle();
    let target = app
        .get_webview_window(&label)
        .ok_or_else(|| AppError::window_control_failed("Window handle was not found"))?;

    target.is_always_on_top().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read always-on-top: {}", error))
    })
}

#[tauri::command]
pub fn start_dragging(window: Window) -> Result<(), AppError> {
    window.start_dragging().map_err(|error| {
        AppError::window_control_failed(format!("Unable to start drag: {}", error))
    })
}
