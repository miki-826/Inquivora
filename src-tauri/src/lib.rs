pub mod commands;
pub mod database;
pub mod error;
pub mod tray;
pub mod workspace;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::{Manager, WindowEvent};

pub struct DbState(pub Mutex<Connection>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // updaterプラグインはplugins.updater設定（署名公開鍵・endpoints）が必須のため、
    // 自動更新を構成するPhase 7で登録する
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let conn = database::open_database(&data_dir.join("inquivora.db"))?;
            app.manage(DbState(Mutex::new(conn)));
            app.manage(commands::workspace::WorkspaceState::default());
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::workspace::workspace_open,
            commands::workspace::workspace_create,
            commands::workspace::workspace_list_recent,
            commands::workspace::workspace_close,
            commands::files::file_list_children,
            commands::files::file_read,
            commands::files::file_write_atomic,
            commands::files::file_create,
            commands::files::file_rename,
            commands::files::file_delete,
            commands::files::file_copy,
            commands::files::file_move,
            commands::files::file_reveal,
            commands::files::file_open_external,
            commands::files::file_detect_type,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
