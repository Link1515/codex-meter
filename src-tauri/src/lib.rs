mod codex;
mod commands;
mod window;

use commands::usage::{fetch_usage, test_cli_command};
use commands::window::{get_always_on_top, set_always_on_top, start_dragging};

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            fetch_usage,
            test_cli_command,
            set_always_on_top,
            get_always_on_top,
            start_dragging
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Meter");
}
