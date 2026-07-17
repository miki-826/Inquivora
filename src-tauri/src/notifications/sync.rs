use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::database::events::EventRecord;
use crate::database::reminders::{self, ReminderInput};
use crate::database::tasks::{Task, TaskStatus};
use crate::error::AppError;
use crate::notifications::schedule::{
    anchor_delta_seconds, event_default_notify_at, task_default_notify_at, NotificationSettings,
};

fn is_future(notify_at_utc: &str, now: DateTime<Utc>) -> bool {
    DateTime::parse_from_rfc3339(notify_at_utc)
        .map(|dt| dt.with_timezone(&Utc) > now)
        .unwrap_or(false)
}

fn create_default(
    conn: &Connection,
    task_id: Option<&str>,
    event_id: Option<&str>,
    notify_at: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let Some(notify_at) = notify_at else {
        return Ok(());
    };
    if !is_future(&notify_at, now) {
        return Ok(());
    }
    reminders::create_reminder_if_absent(
        conn,
        &ReminderInput {
            task_id: task_id.map(str::to_string),
            event_id: event_id.map(str::to_string),
            notify_at_utc: notify_at,
            timezone: "Asia/Tokyo".to_string(),
        },
    )?;
    Ok(())
}

fn sync_anchor_change(
    conn: &Connection,
    task_id: Option<&str>,
    event_id: Option<&str>,
    old_anchor: Option<&str>,
    new_anchor: Option<&str>,
    default_notify_at: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    match (old_anchor, new_anchor) {
        (_, None) => {
            if old_anchor.is_some() {
                reminders::cancel_scheduled(conn, task_id, event_id)?;
            }
        }
        (None, Some(_)) => {
            if reminders::count_for_target(conn, task_id, event_id)? == 0 {
                create_default(conn, task_id, event_id, default_notify_at, now)?;
            }
        }
        (Some(old), Some(new)) if old != new => {
            if reminders::count_for_target(conn, task_id, event_id)? == 0 {
                create_default(conn, task_id, event_id, default_notify_at, now)?;
            } else if let Some(delta) = anchor_delta_seconds(old, new) {
                reminders::shift_scheduled(conn, task_id, event_id, delta)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// タスク保存後のリマインダー同期（§14.5 タスク更新時の再計算）。
/// - 完了・中止: scheduledをcancelled
/// - 期日変更: scheduledを差分移動（リマインダーが1件もなければ既定を作成）
/// - 期日削除: scheduledをcancelled
/// - 新規または期日追加: 既定リマインダーを作成（過去時刻なら作らない）
pub fn sync_after_task_saved(
    conn: &Connection,
    old: Option<&Task>,
    task: &Task,
    settings: &NotificationSettings,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
        reminders::cancel_scheduled(conn, Some(&task.id), None)?;
        return Ok(());
    }
    if !settings.enabled {
        return Ok(());
    }
    let default_notify_at = task
        .due_at
        .as_deref()
        .and_then(|due| task_default_notify_at(due, settings));
    match old {
        None => {
            if task.due_at.is_some() {
                create_default(conn, Some(&task.id), None, default_notify_at, now)?;
            }
        }
        Some(old) => sync_anchor_change(
            conn,
            Some(&task.id),
            None,
            old.due_at.as_deref(),
            task.due_at.as_deref(),
            default_notify_at,
            now,
        )?,
    }
    Ok(())
}

/// 予定保存後のリマインダー同期（§14.5 予定更新時の再計算）。
pub fn sync_after_event_saved(
    conn: &Connection,
    old: Option<&EventRecord>,
    event: &EventRecord,
    settings: &NotificationSettings,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if !settings.enabled {
        return Ok(());
    }
    let default_notify_at = event_default_notify_at(&event.start_at, event.all_day, settings);
    match old {
        None => create_default(conn, None, Some(&event.id), default_notify_at, now)?,
        Some(old) => sync_anchor_change(
            conn,
            None,
            Some(&event.id),
            Some(old.start_at.as_str()),
            Some(event.start_at.as_str()),
            default_notify_at,
            now,
        )?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{events, open_database, reminders, tasks};
    use chrono::TimeZone;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 17, 3, 0, 0).unwrap()
    }

    fn settings() -> NotificationSettings {
        NotificationSettings::default()
    }

    fn create_task(conn: &Connection, value: serde_json::Value) -> tasks::Task {
        tasks::create_task(conn, &serde_json::from_value(value).unwrap()).unwrap()
    }

    fn saved_task(conn: &Connection, value: serde_json::Value) -> tasks::Task {
        let task = create_task(conn, value);
        sync_after_task_saved(conn, None, &task, &settings(), fixed_now()).unwrap();
        task
    }

    fn task_reminders(conn: &Connection, task_id: &str) -> Vec<reminders::Reminder> {
        reminders::list_for_target(conn, Some(task_id), None).unwrap()
    }

    #[test]
    fn 期日付きタスク作成で既定リマインダーが入る() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(
            &conn,
            serde_json::json!({ "title": "資料作成", "dueAtUtc": "2026-07-18T01:00:00Z" }),
        );
        let list = task_reminders(&conn, &task.id);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notify_at, "2026-07-18T00:30:00Z");
        assert_eq!(list[0].status, "scheduled");
    }

    #[test]
    fn 日付のみ期日は既定通知時刻でリマインダーが入る() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(
            &conn,
            serde_json::json!({ "title": "提出", "dueAtUtc": "2026-07-17T15:00:00Z" }),
        );
        let list = task_reminders(&conn, &task.id);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notify_at, "2026-07-18T00:00:00Z");
    }

    #[test]
    fn 期日なしタスクはリマインダーを作らない() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(&conn, serde_json::json!({ "title": "いつか" }));
        assert!(task_reminders(&conn, &task.id).is_empty());
    }

    #[test]
    fn 通知無効設定ではリマインダーを作らない() {
        let (_dir, conn) = temp_conn();
        let task = create_task(
            &conn,
            serde_json::json!({ "title": "資料作成", "dueAtUtc": "2026-07-18T01:00:00Z" }),
        );
        let disabled = NotificationSettings { enabled: false, ..settings() };
        sync_after_task_saved(&conn, None, &task, &disabled, fixed_now()).unwrap();
        assert!(task_reminders(&conn, &task.id).is_empty());
    }

    #[test]
    fn 通知時刻が過去になる場合は作らない() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(
            &conn,
            serde_json::json!({ "title": "直前", "dueAtUtc": "2026-07-17T03:10:00Z" }),
        );
        assert!(task_reminders(&conn, &task.id).is_empty());
    }

    #[test]
    fn 期日変更でscheduledリマインダーが同じ差分だけ移動する() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(
            &conn,
            serde_json::json!({ "title": "資料作成", "dueAtUtc": "2026-07-18T01:00:00Z" }),
        );
        let updated = tasks::update_task(
            &conn,
            &task.id,
            &serde_json::from_value(serde_json::json!({ "dueAtUtc": "2026-07-19T01:00:00Z" }))
                .unwrap(),
        )
        .unwrap();
        sync_after_task_saved(&conn, Some(&task), &updated, &settings(), fixed_now()).unwrap();
        let list = task_reminders(&conn, &task.id);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notify_at, "2026-07-19T00:30:00Z");
    }

    #[test]
    fn 期日削除でscheduledリマインダーがcancelledになる() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(
            &conn,
            serde_json::json!({ "title": "資料作成", "dueAtUtc": "2026-07-18T01:00:00Z" }),
        );
        let updated = tasks::update_task(
            &conn,
            &task.id,
            &serde_json::from_value(serde_json::json!({ "dueAtUtc": null })).unwrap(),
        )
        .unwrap();
        sync_after_task_saved(&conn, Some(&task), &updated, &settings(), fixed_now()).unwrap();
        let list = task_reminders(&conn, &task.id);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, "cancelled");
    }

    #[test]
    fn 完了でscheduledリマインダーがcancelledになる() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(
            &conn,
            serde_json::json!({ "title": "資料作成", "dueAtUtc": "2026-07-18T01:00:00Z" }),
        );
        let completed = tasks::complete_task(&conn, &task.id).unwrap();
        sync_after_task_saved(&conn, Some(&task), &completed, &settings(), fixed_now()).unwrap();
        let list = task_reminders(&conn, &task.id);
        assert_eq!(list[0].status, "cancelled");
    }

    #[test]
    fn リマインダーが全て消えた後の期日変更では既定を作り直す() {
        let (_dir, conn) = temp_conn();
        let task = saved_task(
            &conn,
            serde_json::json!({ "title": "資料作成", "dueAtUtc": "2026-07-18T01:00:00Z" }),
        );
        for reminder in task_reminders(&conn, &task.id) {
            reminders::delete_reminder(&conn, &reminder.id).unwrap();
        }
        let updated = tasks::update_task(
            &conn,
            &task.id,
            &serde_json::from_value(serde_json::json!({ "dueAtUtc": "2026-07-19T01:00:00Z" }))
                .unwrap(),
        )
        .unwrap();
        sync_after_task_saved(&conn, Some(&task), &updated, &settings(), fixed_now()).unwrap();
        let list = task_reminders(&conn, &task.id);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notify_at, "2026-07-19T00:30:00Z");
    }

    #[test]
    fn 予定作成で既定リマインダーが入る() {
        let (_dir, conn) = temp_conn();
        let event = events::create_event(
            &conn,
            &serde_json::from_value(serde_json::json!({
                "title": "定例会",
                "startAtUtc": "2026-07-18T01:00:00Z"
            }))
            .unwrap(),
        )
        .unwrap();
        sync_after_event_saved(&conn, None, &event, &settings(), fixed_now()).unwrap();
        let list = reminders::list_for_target(&conn, None, Some(&event.id)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notify_at, "2026-07-18T00:50:00Z");
    }

    #[test]
    fn 終日予定は初日の既定通知時刻でリマインダーが入る() {
        let (_dir, conn) = temp_conn();
        let event = events::create_event(
            &conn,
            &serde_json::from_value(serde_json::json!({
                "title": "全社休暇",
                "startAtUtc": "2026-07-17T15:00:00Z",
                "allDay": true
            }))
            .unwrap(),
        )
        .unwrap();
        sync_after_event_saved(&conn, None, &event, &settings(), fixed_now()).unwrap();
        let list = reminders::list_for_target(&conn, None, Some(&event.id)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notify_at, "2026-07-18T00:00:00Z");
    }

    #[test]
    fn 予定の開始変更でリマインダーが移動する() {
        let (_dir, conn) = temp_conn();
        let event = events::create_event(
            &conn,
            &serde_json::from_value(serde_json::json!({
                "title": "定例会",
                "startAtUtc": "2026-07-18T01:00:00Z"
            }))
            .unwrap(),
        )
        .unwrap();
        sync_after_event_saved(&conn, None, &event, &settings(), fixed_now()).unwrap();
        let updated = events::update_event(
            &conn,
            &event.id,
            &serde_json::from_value(serde_json::json!({ "startAtUtc": "2026-07-18T05:00:00Z" }))
                .unwrap(),
        )
        .unwrap();
        sync_after_event_saved(&conn, Some(&event), &updated, &settings(), fixed_now()).unwrap();
        let list = reminders::list_for_target(&conn, None, Some(&event.id)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].notify_at, "2026-07-18T04:50:00Z");
    }
}
