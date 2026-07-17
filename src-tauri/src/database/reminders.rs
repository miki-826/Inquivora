use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub task_id: Option<String>,
    pub event_id: Option<String>,
    #[serde(rename = "notifyAtUtc")]
    pub notify_at: String,
    pub timezone: String,
    pub status: String,
    #[serde(rename = "sentAtUtc")]
    pub sent_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_timezone() -> String {
    "Asia/Tokyo".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    pub notify_at_utc: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReminderPatch {
    pub notify_at_utc: Option<String>,
}

pub fn create_reminder(_conn: &Connection, _input: &ReminderInput) -> Result<Reminder, AppError> {
    unimplemented!()
}

/// 同一対象・同一時刻の重複はエラーにせずNoneを返す（自動作成用、§14.6）。
pub fn create_reminder_if_absent(
    _conn: &Connection,
    _input: &ReminderInput,
) -> Result<Option<Reminder>, AppError> {
    unimplemented!()
}

pub fn get_reminder(_conn: &Connection, _id: &str) -> Result<Reminder, AppError> {
    unimplemented!()
}

pub fn update_reminder(
    _conn: &Connection,
    _id: &str,
    _patch: &ReminderPatch,
) -> Result<Reminder, AppError> {
    unimplemented!()
}

pub fn delete_reminder(_conn: &Connection, _id: &str) -> Result<(), AppError> {
    unimplemented!()
}

pub fn list_upcoming(_conn: &Connection, _limit: usize) -> Result<Vec<Reminder>, AppError> {
    unimplemented!()
}

pub fn list_for_target(
    _conn: &Connection,
    _task_id: Option<&str>,
    _event_id: Option<&str>,
) -> Result<Vec<Reminder>, AppError> {
    unimplemented!()
}

pub fn due_reminders(_conn: &Connection, _now_utc: &str) -> Result<Vec<Reminder>, AppError> {
    unimplemented!()
}

pub fn mark_sent(_conn: &Connection, _id: &str, _now_utc: &str) -> Result<(), AppError> {
    unimplemented!()
}

pub fn cancel_scheduled(
    _conn: &Connection,
    _task_id: Option<&str>,
    _event_id: Option<&str>,
) -> Result<usize, AppError> {
    unimplemented!()
}

pub fn shift_scheduled(
    _conn: &Connection,
    _task_id: Option<&str>,
    _event_id: Option<&str>,
    _delta_seconds: i64,
) -> Result<usize, AppError> {
    unimplemented!()
}

pub fn count_for_target(
    _conn: &Connection,
    _task_id: Option<&str>,
    _event_id: Option<&str>,
) -> Result<i64, AppError> {
    unimplemented!()
}

/// 通知予定時刻から24時間以上経過したscheduledをexpiredにする。
pub fn expire_stale(_conn: &Connection, _now_utc: &str) -> Result<usize, AppError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    fn make_task(conn: &Connection) -> String {
        crate::database::tasks::create_task(
            conn,
            &serde_json::from_value(serde_json::json!({ "title": "対象タスク" })).unwrap(),
        )
        .unwrap()
        .id
    }

    fn make_event(conn: &Connection) -> String {
        crate::database::events::create_event(
            conn,
            &serde_json::from_value(serde_json::json!({
                "title": "対象予定",
                "startAtUtc": "2026-07-18T01:00:00Z"
            }))
            .unwrap(),
        )
        .unwrap()
        .id
    }

    fn input(value: serde_json::Value) -> ReminderInput {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn リマインダーを作成すると既定値が入る() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        assert_eq!(reminder.task_id.as_deref(), Some(task_id.as_str()));
        assert_eq!(reminder.status, "scheduled");
        assert_eq!(reminder.timezone, "Asia/Tokyo");
        assert!(reminder.sent_at.is_none());
        let fetched = get_reminder(&conn, &reminder.id).unwrap();
        assert_eq!(fetched.notify_at, "2026-07-18T00:30:00Z");
    }

    #[test]
    fn 通知時刻はutcのz表記へ正規化される() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(
                serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T09:30:00+09:00" }),
            ),
        )
        .unwrap();
        assert_eq!(reminder.notify_at, "2026-07-18T00:30:00Z");
    }

    #[test]
    fn 対象未指定と両方指定は拒否する() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let event_id = make_event(&conn);
        let err = create_reminder(
            &conn,
            &input(serde_json::json!({ "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
        let err = create_reminder(
            &conn,
            &input(serde_json::json!({
                "taskId": task_id, "eventId": event_id, "notifyAtUtc": "2026-07-18T00:30:00Z"
            })),
        )
        .unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 不正な日時は拒否する() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let err = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026/07/18 09:00" })),
        )
        .unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 同一対象同一時刻の重複は拒否する() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let payload =
            serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" });
        create_reminder(&conn, &input(payload.clone())).unwrap();
        let err = create_reminder(&conn, &input(payload.clone())).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert!(create_reminder_if_absent(&conn, &input(payload)).unwrap().is_none());
    }

    #[test]
    fn 送信済みを時刻変更するとscheduledへ戻る() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        mark_sent(&conn, &reminder.id, "2026-07-18T00:30:05Z").unwrap();
        let sent = get_reminder(&conn, &reminder.id).unwrap();
        assert_eq!(sent.status, "sent");
        assert_eq!(sent.sent_at.as_deref(), Some("2026-07-18T00:30:05Z"));
        let updated = update_reminder(
            &conn,
            &reminder.id,
            &serde_json::from_value(serde_json::json!({ "notifyAtUtc": "2026-07-19T00:30:00Z" }))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(updated.status, "scheduled");
        assert!(updated.sent_at.is_none());
        assert_eq!(updated.notify_at, "2026-07-19T00:30:00Z");
    }

    #[test]
    fn 削除すると取得できない() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        delete_reminder(&conn, &reminder.id).unwrap();
        assert_eq!(get_reminder(&conn, &reminder.id).unwrap_err().code, "REMINDER_NOT_FOUND");
        assert_eq!(delete_reminder(&conn, &reminder.id).unwrap_err().code, "REMINDER_NOT_FOUND");
    }

    #[test]
    fn list_upcomingはscheduledのみ時刻昇順で返す() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let later = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-19T00:00:00Z" })),
        )
        .unwrap();
        let earlier = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:00:00Z" })),
        )
        .unwrap();
        let sent = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-17T00:00:00Z" })),
        )
        .unwrap();
        mark_sent(&conn, &sent.id, "2026-07-17T00:00:10Z").unwrap();
        let ids: Vec<String> = list_upcoming(&conn, 10).unwrap().into_iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![earlier.id, later.id]);
    }

    #[test]
    fn due_remindersは現在時刻以前のscheduledだけ返す() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let due = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-17T02:59:00Z" })),
        )
        .unwrap();
        create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-17T03:01:00Z" })),
        )
        .unwrap();
        let ids: Vec<String> = due_reminders(&conn, "2026-07-17T03:00:00Z")
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, vec![due.id]);
    }

    #[test]
    fn タスク削除でリマインダーも消える() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        crate::database::tasks::delete_task(&conn, &task_id).unwrap();
        assert_eq!(get_reminder(&conn, &reminder.id).unwrap_err().code, "REMINDER_NOT_FOUND");
    }

    #[test]
    fn cancel_scheduledはscheduledだけをcancelledにする() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let scheduled = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        let sent = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-17T00:30:00Z" })),
        )
        .unwrap();
        mark_sent(&conn, &sent.id, "2026-07-17T00:30:05Z").unwrap();
        let affected = cancel_scheduled(&conn, Some(&task_id), None).unwrap();
        assert_eq!(affected, 1);
        assert_eq!(get_reminder(&conn, &scheduled.id).unwrap().status, "cancelled");
        assert_eq!(get_reminder(&conn, &sent.id).unwrap().status, "sent");
    }

    #[test]
    fn shift_scheduledは対象のscheduledだけ移動する() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let other_task = make_task(&conn);
        let target = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        let other = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": other_task, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        let affected = shift_scheduled(&conn, Some(&task_id), None, 24 * 3600).unwrap();
        assert_eq!(affected, 1);
        assert_eq!(get_reminder(&conn, &target.id).unwrap().notify_at, "2026-07-19T00:30:00Z");
        assert_eq!(get_reminder(&conn, &other.id).unwrap().notify_at, "2026-07-18T00:30:00Z");
    }

    #[test]
    fn count_for_targetは対象の件数を返す() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        assert_eq!(count_for_target(&conn, Some(&task_id), None).unwrap(), 0);
        create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:30:00Z" })),
        )
        .unwrap();
        assert_eq!(count_for_target(&conn, Some(&task_id), None).unwrap(), 1);
    }

    #[test]
    fn expire_staleは24時間以上前のscheduledをexpiredにする() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let stale = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-16T02:00:00Z" })),
        )
        .unwrap();
        let recent = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-17T01:00:00Z" })),
        )
        .unwrap();
        let affected = expire_stale(&conn, "2026-07-17T03:00:00Z").unwrap();
        assert_eq!(affected, 1);
        assert_eq!(get_reminder(&conn, &stale.id).unwrap().status, "expired");
        assert_eq!(get_reminder(&conn, &recent.id).unwrap().status, "scheduled");
    }

    #[test]
    fn イベント向けリマインダーも作成できる() {
        let (_dir, conn) = temp_conn();
        let event_id = make_event(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({ "eventId": event_id, "notifyAtUtc": "2026-07-18T00:50:00Z" })),
        )
        .unwrap();
        assert_eq!(reminder.event_id.as_deref(), Some(event_id.as_str()));
        let listed = list_for_target(&conn, None, Some(&event_id)).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, reminder.id);
    }
}
