use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, Window};

use crate::codex::errors::AppError;
use crate::window::{
    placement::{ensure_visible, VisibleBounds},
    types::WindowPlacementState,
};

pub const USAGE_REFRESH_REQUESTED_EVENT: &str = "codex-meter://usage-refresh-requested";
pub const WINDOW_VISIBILITY_CHANGED_EVENT: &str = "codex-meter://window-visibility-changed";
const MAIN_WINDOW_LABEL: &str = "main";

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

pub fn reveal_widget_window(window: &WebviewWindow) -> Result<(), AppError> {
    window.unminimize().map_err(|error| {
        AppError::window_control_failed(format!("Unable to restore widget window: {}", error))
    })?;
    window.show().map_err(|error| {
        AppError::window_control_failed(format!("Unable to show widget window: {}", error))
    })?;
    let _ = window.set_focus();
    window
        .emit(WINDOW_VISIBILITY_CHANGED_EVENT, true)
        .map_err(|error| {
            AppError::window_control_failed(format!(
                "Unable to notify widget visibility: {}",
                error
            ))
        })
}

#[tauri::command]
pub fn reveal_widget(window: Window) -> Result<(), AppError> {
    let label = window.label().to_string();
    let app = window.app_handle();
    let target = app
        .get_webview_window(&label)
        .ok_or_else(|| AppError::window_control_failed("Widget window handle was not found"))?;

    reveal_widget_window(&target)
}

pub fn conceal_widget_window(window: &WebviewWindow) -> Result<(), AppError> {
    window.hide().map_err(|error| {
        AppError::window_control_failed(format!("Unable to hide widget window: {}", error))
    })?;
    window
        .emit(WINDOW_VISIBILITY_CHANGED_EVENT, false)
        .map_err(|error| {
            AppError::window_control_failed(format!(
                "Unable to notify widget visibility: {}",
                error
            ))
        })
}

pub fn toggle_widget_window(app: &AppHandle) -> Result<(), AppError> {
    let window = main_widget_window(app)?;
    let is_visible = window.is_visible().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read widget visibility: {}", error))
    })?;
    let is_minimized = window.is_minimized().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read widget minimized state: {}", error))
    })?;

    if is_visible && !is_minimized {
        conceal_widget_window(&window)
    } else {
        reveal_widget_window(&window)
    }
}

pub fn conceal_main_widget_window(app: &AppHandle) -> Result<(), AppError> {
    let window = main_widget_window(app)?;
    conceal_widget_window(&window)
}

pub fn request_usage_refresh(app: &AppHandle) -> Result<(), AppError> {
    main_widget_window(app)?
        .emit(USAGE_REFRESH_REQUESTED_EVENT, ())
        .map_err(|error| {
            AppError::window_control_failed(format!("Unable to request usage refresh: {}", error))
        })
}

#[tauri::command]
pub fn is_window_polling_allowed(window: Window) -> Result<bool, AppError> {
    let is_visible = window.is_visible().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read widget visibility: {}", error))
    })?;
    let is_minimized = window.is_minimized().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read widget minimized state: {}", error))
    })?;

    Ok(is_visible && !is_minimized)
}

#[tauri::command]
pub fn set_window_size(
    window: Window,
    width: f64,
    height: f64,
    viewport_width: f64,
    viewport_height: f64,
) -> Result<(), AppError> {
    if !is_positive_finite(width)
        || !is_positive_finite(height)
        || !is_positive_finite(viewport_width)
        || !is_positive_finite(viewport_height)
    {
        return Err(AppError::invalid_config(
            "Window size must be positive finite values",
        ));
    }

    let current_inner_size = window.inner_size().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read current window size: {}", error))
    })?;
    let physical_width =
        physical_pixels_for_css_extent(width, viewport_width, current_inner_size.width)
            .ok_or_else(|| {
                AppError::invalid_config("Unable to convert widget width to physical pixels")
            })?;
    let physical_height =
        physical_pixels_for_css_extent(height, viewport_height, current_inner_size.height)
            .ok_or_else(|| {
                AppError::invalid_config("Unable to convert widget height to physical pixels")
            })?;

    window.set_resizable(true).map_err(|error| {
        AppError::window_control_failed(format!("Unable to prepare window resize: {}", error))
    })?;

    let resize_result = window
        .set_size(PhysicalSize::new(physical_width, physical_height))
        .map_err(|error| {
            AppError::window_control_failed(format!("Unable to resize window: {}", error))
        });

    let restore_result = window.set_resizable(false).map_err(|error| {
        AppError::window_control_failed(format!("Unable to restore fixed window size: {}", error))
    });

    resize_result?;
    restore_result?;

    let placement = placement_from_window(&window)?;
    restore_window_placement(window, placement)?;

    Ok(())
}

fn is_positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn physical_pixels_for_css_extent(
    desired_css_extent: f64,
    viewport_css_extent: f64,
    current_physical_extent: u32,
) -> Option<u32> {
    if !is_positive_finite(desired_css_extent)
        || !is_positive_finite(viewport_css_extent)
        || current_physical_extent == 0
    {
        return None;
    }

    let physical_extent =
        (desired_css_extent * current_physical_extent as f64 / viewport_css_extent).ceil();

    if physical_extent.is_finite() && physical_extent > 0.0 && physical_extent <= u32::MAX as f64 {
        Some(physical_extent as u32)
    } else {
        None
    }
}

fn main_widget_window(app: &AppHandle) -> Result<WebviewWindow, AppError> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| AppError::window_control_failed("Widget window handle was not found"))
}

#[tauri::command]
pub fn get_window_placement(window: Window) -> Result<WindowPlacementState, AppError> {
    placement_from_window(&window)
}

#[tauri::command]
pub fn restore_window_placement(
    window: Window,
    placement: WindowPlacementState,
) -> Result<WindowPlacementState, AppError> {
    let current_size = window.outer_size().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read window size: {}", error))
    })?;
    let bounds = visible_bounds_for_placement(&window, placement.display_id.as_deref())?;
    let corrected = ensure_visible(
        &WindowPlacementState {
            width: current_size.width,
            height: current_size.height,
            ..placement
        },
        bounds,
    );

    if corrected.x != placement.x || corrected.y != placement.y {
        window
            .set_position(PhysicalPosition::new(corrected.x, corrected.y))
            .map_err(|error| {
                AppError::window_control_failed(format!(
                    "Unable to restore window position: {}",
                    error
                ))
            })?;
    }

    Ok(corrected)
}

fn placement_from_window(window: &Window) -> Result<WindowPlacementState, AppError> {
    let position = window.outer_position().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read window position: {}", error))
    })?;
    let size = window.outer_size().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read window size: {}", error))
    })?;
    let display_id = window
        .current_monitor()
        .map_err(|error| {
            AppError::window_control_failed(format!("Unable to read current display: {}", error))
        })?
        .and_then(|monitor| monitor.name().cloned());

    Ok(WindowPlacementState {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        display_id,
        updated_at: crate::codex::types::current_timestamp(),
    })
}

fn visible_bounds_for_placement(
    window: &Window,
    display_id: Option<&str>,
) -> Result<VisibleBounds, AppError> {
    let monitors = window.available_monitors().map_err(|error| {
        AppError::window_control_failed(format!("Unable to read displays: {}", error))
    })?;
    let selected = display_id
        .and_then(|id| {
            monitors
                .iter()
                .find(|monitor| monitor.name().is_some_and(|name| name == id))
                .cloned()
        })
        .or_else(|| {
            window
                .current_monitor()
                .ok()
                .flatten()
                .or_else(|| monitors.first().cloned())
        })
        .ok_or_else(|| AppError::window_control_failed("No display was available"))?;
    let position = selected.position();
    let size = selected.size();

    Ok(VisibleBounds {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

#[cfg(test)]
mod tests {
    use super::physical_pixels_for_css_extent;

    #[test]
    fn rounds_up_css_content_to_native_pixels() {
        assert_eq!(physical_pixels_for_css_extent(205.0, 190.0, 262), Some(283));
    }

    #[test]
    fn rejects_invalid_viewport_measurements() {
        assert_eq!(physical_pixels_for_css_extent(205.0, 0.0, 262), None);
        assert_eq!(physical_pixels_for_css_extent(f64::NAN, 190.0, 262), None);
    }
}
