use tauri::{AppHandle, State};

use crate::database::reminders::{self, Reminder, ReminderInput, ReminderPatch};
use crate::database::settings;
use crate::error::AppError;
use crate::notifications::{schedule, scheduler, sender};
use crate::notifications::TestNotificationGuard;
use crate::DbState;

fn with_conn<T>(
    state: &State<'_, DbState>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let conn = state
        .0
        .lock()
        .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
    f(&conn)
}

#[tauri::command]
pub fn reminder_create(
    state: State<'_, DbState>,
    input: ReminderInput,
) -> Result<Reminder, AppError> {
    with_conn(&state, |conn| reminders::create_reminder(conn, &input))
}

#[tauri::command]
pub fn reminder_update(
    state: State<'_, DbState>,
    id: String,
    patch: ReminderPatch,
) -> Result<Reminder, AppError> {
    with_conn(&state, |conn| reminders::update_reminder(conn, &id, &patch))
}

#[tauri::command]
pub fn reminder_delete(state: State<'_, DbState>, id: String) -> Result<(), AppError> {
    with_conn(&state, |conn| reminders::delete_reminder(conn, &id))
}

#[tauri::command]
pub fn reminder_list_upcoming(state: State<'_, DbState>) -> Result<Vec<Reminder>, AppError> {
    with_conn(&state, |conn| reminders::list_upcoming(conn, 200))
}

#[tauri::command]
pub fn reminder_list_for_target(
    state: State<'_, DbState>,
    task_id: Option<String>,
    event_id: Option<String>,
) -> Result<Vec<Reminder>, AppError> {
    with_conn(&state, |conn| {
        reminders::list_for_target(conn, task_id.as_deref(), event_id.as_deref())
    })
}

/// 期限切れリマインダーを整理する（§14.5 通知設定変更時などに呼ぶ）。
#[tauri::command]
pub fn notification_reconcile(app: AppHandle) -> Result<usize, AppError> {
    scheduler::reconcile(&app)
}

/// テスト通知を即時送信する（設定画面用）。
#[tauri::command]
pub async fn notification_test(
    app: AppHandle,
    state: State<'_, DbState>,
    guard: State<'_, TestNotificationGuard>,
) -> Result<(), AppError> {
    let Some(_lease) = guard.try_acquire() else {
        return Ok(());
    };
    let silent = {
        let conn = state
            .0
            .lock()
            .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
        !schedule::parse_settings(settings::get_setting(&conn, "notifications")?).sound
    };
    let payload = schedule::NotificationPayload {
        notification_id: format!("test-{}", uuid::Uuid::new_v4()),
        title: "Inquivora".to_string(),
        body: "通知のテストです。この通知が見えていれば設定は正常です。".to_string(),
        launch_uri: "inquivora://open?type=test&id=test".to_string(),
    };
    sender::send_notification(&app, &payload, silent).await
}
