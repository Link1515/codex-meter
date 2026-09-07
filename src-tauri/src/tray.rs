use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::commands::window::{request_usage_refresh, toggle_widget_window};

const TOGGLE_WIDGET_ID: &str = "toggle-widget";
const REFRESH_USAGE_ID: &str = "refresh-usage";
const QUIT_ID: &str = "quit";

pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_widget = MenuItem::with_id(
        app,
        TOGGLE_WIDGET_ID,
        "Show / Hide Codex Meter",
        true,
        None::<&str>,
    )?;
    let refresh_usage =
        MenuItem::with_id(app, REFRESH_USAGE_ID, "Refresh usage", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_widget, &refresh_usage, &quit])?;

    let mut tray = TrayIconBuilder::with_id("codex-meter-tray")
        .menu(&menu)
        .tooltip("Codex Meter")
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.on_menu_event(|app, event| match tray_action(event.id().as_ref()) {
        Some(TrayAction::ToggleWidget) => {
            let _ = toggle_widget_window(app);
        }
        Some(TrayAction::RefreshUsage) => {
            let _ = request_usage_refresh(app);
        }
        Some(TrayAction::Quit) => app.exit(0),
        None => {}
    })
    .on_tray_icon_event(|tray, event| {
        if matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
        ) {
            let _ = toggle_widget_window(&tray.app_handle());
        }
    })
    .build(app)?;

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrayAction {
    ToggleWidget,
    RefreshUsage,
    Quit,
}

fn tray_action(id: &str) -> Option<TrayAction> {
    match id {
        TOGGLE_WIDGET_ID => Some(TrayAction::ToggleWidget),
        REFRESH_USAGE_ID => Some(TrayAction::RefreshUsage),
        QUIT_ID => Some(TrayAction::Quit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{tray_action, TrayAction};

    #[test]
    fn maps_tray_menu_ids_to_actions() {
        assert_eq!(tray_action("toggle-widget"), Some(TrayAction::ToggleWidget));
        assert_eq!(tray_action("refresh-usage"), Some(TrayAction::RefreshUsage));
        assert_eq!(tray_action("quit"), Some(TrayAction::Quit));
        assert_eq!(tray_action("unknown"), None);
    }
}
