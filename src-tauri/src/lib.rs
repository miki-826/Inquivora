pub mod api;
pub mod commands;
pub mod database;
pub mod error;
pub mod meeting;
pub mod whisper;
pub mod notifications;
pub mod tray;
pub mod workspace;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::{Manager, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;

pub struct DbState(pub Mutex<Connection>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // updaterプラグインはplugins.updater設定（署名公開鍵・endpoints）が必須のため、
    // 自動更新を構成するPhase 7で登録する
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            #[cfg(debug_assertions)]
            app.deep_link().register_all()?;
            // §14.3 通知クリック時の前面化。画面遷移はフロント側のonOpenUrlが担う
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |_event| {
                tray::show_main_window(&handle);
            });
            notifications::scheduler::spawn(app.handle().clone());
            app.manage(meeting::session::AudioSessions::default());
            app.manage(notifications::TestNotificationGuard::default());
            meeting::worker::spawn(app.handle().clone());
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
            commands::files::file_read_base64,
            commands::files::file_import,
            commands::files::tabs_save,
            commands::files::tabs_load,
            commands::tasks::task_create,
            commands::tasks::task_update,
            commands::tasks::task_delete,
            commands::tasks::task_get,
            commands::tasks::task_list,
            commands::tasks::task_complete,
            commands::tasks::task_reopen,
            commands::events::event_create,
            commands::events::event_update,
            commands::events::event_delete,
            commands::events::event_get,
            commands::events::event_get_range,
            commands::reminders::reminder_create,
            commands::reminders::reminder_update,
            commands::reminders::reminder_delete,
            commands::reminders::reminder_list_upcoming,
            commands::reminders::reminder_list_for_target,
            commands::reminders::notification_test,
            commands::reminders::notification_reconcile,
            commands::providers::api_provider_list,
            commands::providers::api_provider_get,
            commands::providers::api_provider_create,
            commands::providers::api_provider_update,
            commands::providers::api_provider_delete,
            commands::providers::api_provider_enable,
            commands::providers::api_provider_set_secret,
            commands::providers::api_provider_delete_secret,
            commands::providers::api_provider_has_secret,
            commands::providers::api_provider_test,
            commands::providers::api_provider_list_models,
            commands::providers::ai_feature_binding_get,
            commands::providers::ai_feature_binding_set,
            commands::providers::api_usage_list,
            commands::meetings::meeting_start,
            commands::meetings::meeting_pause,
            commands::meetings::meeting_resume,
            commands::meetings::meeting_stop,
            commands::meetings::meeting_get,
            commands::meetings::meeting_list,
            commands::meetings::meeting_delete,
            commands::meetings::meeting_list_segments,
            commands::meetings::meeting_append_segment,
            commands::meetings::meeting_list_devices,
            commands::whisper::whisper_model_status,
            commands::whisper::whisper_model_select,
            commands::whisper::whisper_model_download,
            commands::whisper::whisper_model_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
