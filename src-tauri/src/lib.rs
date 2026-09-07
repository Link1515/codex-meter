mod codex;
mod commands;
mod tray;
mod window;

use tauri::Manager;

use commands::usage::fetch_usage;
use commands::window::{
    get_always_on_top, get_window_placement, is_window_polling_allowed, restore_window_placement,
    reveal_widget, set_always_on_top, set_window_size, start_dragging,
};

pub fn run() {
    tauri::Builder::default()
        .setup(tray::setup)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = commands::window::conceal_main_widget_window(&window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            fetch_usage,
            set_always_on_top,
            get_always_on_top,
            start_dragging,
            set_window_size,
            reveal_widget,
            is_window_polling_allowed,
            get_window_placement,
            restore_window_placement
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Meter");
}
