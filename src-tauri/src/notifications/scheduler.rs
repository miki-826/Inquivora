use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use tauri::{AppHandle, Manager};

use crate::database::tasks::TaskStatus;
use crate::database::{events, reminders, settings, tasks};
use crate::error::AppError;
use crate::notifications::schedule::{self, NotificationPayload};
use crate::notifications::sender;
use crate::DbState;

const TICK: Duration = Duration::from_secs(15);

struct DueNotification {
    reminder_id: String,
    payload: NotificationPayload,
}

fn now_utc() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// 常駐通知スケジューラー（§14.2・§14.5）。約15秒間隔で期限到来リマインダーを配信し、
/// tick間隔の大幅な遅延をスリープ復帰とみなして再計算する。
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let _ = reconcile(&app);
        let mut last = Instant::now();
        loop {
            tokio::time::sleep(TICK).await;
            let elapsed = last.elapsed().as_secs();
            last = Instant::now();
            if schedule::is_resume_gap(elapsed) {
                eprintln!("スリープ復帰を検知（{elapsed}秒停止）。通知を再計算します");
                if let Err(err) = reconcile(&app) {
                    eprintln!("通知の再計算に失敗: {err}");
                }
            }
            process_due(&app).await;
        }
    });
}

/// 期限切れリマインダーの整理（§14.5 アプリ起動・スリープ復帰・設定変更時）。
pub fn reconcile(app: &AppHandle) -> Result<usize, AppError> {
    let state = app.state::<DbState>();
    let conn = state
        .0
        .lock()
        .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
    reminders::expire_stale(&conn, &now_utc())
}

fn collect_due(app: &AppHandle) -> Result<(Vec<DueNotification>, bool), AppError> {
    let state = app.state::<DbState>();
    let conn = state
        .0
        .lock()
        .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
    let config = schedule::parse_settings(settings::get_setting(&conn, "notifications")?);
    if !config.enabled {
        return Ok((Vec::new(), true));
    }
    let now = now_utc();
    reminders::expire_stale(&conn, &now)?;
    let mut due = Vec::new();
    for reminder in reminders::due_reminders(&conn, &now)? {
        let payload = if let Some(task_id) = reminder.task_id.as_deref() {
            tasks::get_task(&conn, task_id).ok().and_then(|task| {
                (!matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled)).then(|| {
                    schedule::task_notification(
                        &reminder.id,
                        &task.id,
                        &task.title,
                        task.due_at.as_deref(),
                    )
                })
            })
        } else if let Some(event_id) = reminder.event_id.as_deref() {
            events::get_event(&conn, event_id).ok().map(|event| {
                schedule::event_notification(
                    &reminder.id,
                    &event.id,
                    &event.title,
                    &event.start_at,
                    event.all_day,
                    Utc::now(),
                )
            })
        } else {
            None
        };
        match payload {
            Some(payload) => due.push(DueNotification {
                reminder_id: reminder.id,
                payload,
            }),
            None => {
                conn.execute(
                    "UPDATE reminders SET status = 'cancelled', updated_at = ?2 WHERE id = ?1",
                    rusqlite::params![reminder.id, now],
                )?;
            }
        }
    }
    Ok((due, !config.sound))
}

/// 配信後の後処理。周期通知は次回へ再スケジュール、単発は送信済みにする。
fn complete_delivery(app: &AppHandle, reminder_id: &str) -> Result<(), AppError> {
    let state = app.state::<DbState>();
    let conn = state
        .0
        .lock()
        .map_err(|e| AppError::database(format!("DB接続ロックに失敗: {e}")))?;
    reminders::advance_or_complete(&conn, reminder_id, &now_utc()).map(|_| ())
}

async fn process_due(app: &AppHandle) {
    let (due, silent) = match collect_due(app) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("通知対象の取得に失敗: {err}");
            return;
        }
    };
    for item in due {
        match sender::send_notification(app, &item.payload, silent).await {
            Ok(()) => {
                if let Err(err) = complete_delivery(app, &item.reminder_id) {
                    eprintln!("配信後処理に失敗: {err}");
                }
            }
            Err(err) => eprintln!("通知送信に失敗（次tickで再試行）: {err}"),
        }
    }
}
