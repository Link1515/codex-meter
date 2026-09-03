mod codex;
mod commands;
mod window;

use commands::usage::fetch_usage;
use commands::window::{
    get_always_on_top, get_window_placement, restore_window_placement, set_always_on_top,
    set_window_size, show_window, start_dragging,
};

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            fetch_usage,
            set_always_on_top,
            get_always_on_top,
            start_dragging,
            set_window_size,
            show_window,
            get_window_placement,
            restore_window_placement
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Meter");
}
