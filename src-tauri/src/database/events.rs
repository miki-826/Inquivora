use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::database::double_option;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRecord {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "startAtUtc")]
    pub start_at: String,
    #[serde(rename = "endAtUtc")]
    pub end_at: Option<String>,
    pub timezone: String,
    pub all_day: bool,
    pub event_type: String,
    pub recurrence_rule: Option<String>,
    pub meeting_id: Option<String>,
    pub task_id: Option<String>,
    pub location: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_timezone() -> String {
    "Asia/Tokyo".to_string()
}

fn default_event_type() -> String {
    "event".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInput {
    pub title: String,
    pub start_at_utc: String,
    #[serde(default)]
    pub end_at_utc: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub all_day: bool,
    #[serde(default = "default_event_type")]
    pub event_type: String,
    #[serde(default)]
    pub recurrence_rule: Option<String>,
    #[serde(default)]
    pub meeting_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EventPatch {
    pub title: Option<String>,
    pub start_at_utc: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub end_at_utc: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    pub timezone: Option<String>,
    pub all_day: Option<bool>,
    pub event_type: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub recurrence_rule: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub location: Option<Option<String>>,
}

pub fn create_event(_conn: &Connection, _input: &EventInput) -> Result<EventRecord, AppError> {
    todo!()
}

pub fn get_event(_conn: &Connection, _id: &str) -> Result<EventRecord, AppError> {
    todo!()
}

pub fn update_event(
    _conn: &Connection,
    _id: &str,
    _patch: &EventPatch,
) -> Result<EventRecord, AppError> {
    todo!()
}

pub fn delete_event(_conn: &Connection, _id: &str) -> Result<(), AppError> {
    todo!()
}

/// [startUtc, endUtc) と重なる予定を開始日時の昇順で返す（§17.5）。
pub fn list_events_in_range(
    _conn: &Connection,
    _start_utc: &str,
    _end_utc: &str,
) -> Result<Vec<EventRecord>, AppError> {
    todo!()
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

    fn input_json(value: serde_json::Value) -> EventInput {
        serde_json::from_value(value).unwrap()
    }

    fn patch_json(value: serde_json::Value) -> EventPatch {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn 予定を作成すると既定値が入る() {
        let (_dir, conn) = temp_conn();
        let event = create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "打合せ",
                "startAtUtc": "2026-07-17T01:00:00Z",
                "endAtUtc": "2026-07-17T02:00:00Z"
            })),
        )
        .unwrap();
        assert_eq!(event.title, "打合せ");
        assert!(!event.all_day);
        assert_eq!(event.event_type, "event");
        assert_eq!(event.timezone, "Asia/Tokyo");
        let fetched = get_event(&conn, &event.id).unwrap();
        assert_eq!(fetched.id, event.id);
    }

    #[test]
    fn 空タイトルは拒否する() {
        let (_dir, conn) = temp_conn();
        let err = create_event(
            &conn,
            &input_json(serde_json::json!({ "title": " ", "startAtUtc": "2026-07-17T01:00:00Z" })),
        )
        .unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 終了が開始より前なら拒否する() {
        let (_dir, conn) = temp_conn();
        let err = create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "逆転",
                "startAtUtc": "2026-07-17T02:00:00Z",
                "endAtUtc": "2026-07-17T01:00:00Z"
            })),
        )
        .unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 期間内に重なる予定だけ開始順で返る() {
        let (_dir, conn) = temp_conn();
        create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "前日", "startAtUtc": "2026-07-16T01:00:00Z", "endAtUtc": "2026-07-16T02:00:00Z"
            })),
        )
        .unwrap();
        create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "またぎ", "startAtUtc": "2026-07-16T10:00:00Z", "endAtUtc": "2026-07-18T10:00:00Z"
            })),
        )
        .unwrap();
        create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "当日", "startAtUtc": "2026-07-17T01:00:00Z", "endAtUtc": "2026-07-17T02:00:00Z"
            })),
        )
        .unwrap();
        create_event(
            &conn,
            &input_json(serde_json::json!({ "title": "終了なし", "startAtUtc": "2026-07-17T05:00:00Z" })),
        )
        .unwrap();
        create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "翌日", "startAtUtc": "2026-07-18T01:00:00Z", "endAtUtc": "2026-07-18T02:00:00Z"
            })),
        )
        .unwrap();
        let titles: Vec<String> =
            list_events_in_range(&conn, "2026-07-17T00:00:00Z", "2026-07-18T00:00:00Z")
                .unwrap()
                .into_iter()
                .map(|e| e.title)
                .collect();
        assert_eq!(titles, vec!["またぎ", "当日", "終了なし"]);
    }

    #[test]
    fn パッチで部分更新できる() {
        let (_dir, conn) = temp_conn();
        let event = create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "元",
                "startAtUtc": "2026-07-17T01:00:00Z",
                "endAtUtc": "2026-07-17T02:00:00Z",
                "location": "会議室A"
            })),
        )
        .unwrap();
        let updated = update_event(
            &conn,
            &event.id,
            &patch_json(serde_json::json!({
                "startAtUtc": "2026-07-17T03:00:00Z",
                "endAtUtc": "2026-07-17T04:00:00Z",
                "location": null
            })),
        )
        .unwrap();
        assert_eq!(updated.title, "元");
        assert_eq!(updated.start_at, "2026-07-17T03:00:00Z");
        assert_eq!(updated.end_at.as_deref(), Some("2026-07-17T04:00:00Z"));
        assert!(updated.location.is_none());
    }

    #[test]
    fn 更新後に終了が開始より前になるなら拒否する() {
        let (_dir, conn) = temp_conn();
        let event = create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "t",
                "startAtUtc": "2026-07-17T01:00:00Z",
                "endAtUtc": "2026-07-17T02:00:00Z"
            })),
        )
        .unwrap();
        let err = update_event(
            &conn,
            &event.id,
            &patch_json(serde_json::json!({ "startAtUtc": "2026-07-17T05:00:00Z" })),
        )
        .unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 削除すると取得できない() {
        let (_dir, conn) = temp_conn();
        let event = create_event(
            &conn,
            &input_json(serde_json::json!({ "title": "t", "startAtUtc": "2026-07-17T01:00:00Z" })),
        )
        .unwrap();
        delete_event(&conn, &event.id).unwrap();
        assert_eq!(get_event(&conn, &event.id).unwrap_err().code, "EVENT_NOT_FOUND");
        assert_eq!(delete_event(&conn, &event.id).unwrap_err().code, "EVENT_NOT_FOUND");
    }

    #[test]
    fn タスク削除で連動する予定も消える() {
        let (_dir, conn) = temp_conn();
        let task = crate::database::tasks::create_task(
            &conn,
            &serde_json::from_value(serde_json::json!({ "title": "連動元" })).unwrap(),
        )
        .unwrap();
        let event = create_event(
            &conn,
            &input_json(serde_json::json!({
                "title": "期限イベント",
                "startAtUtc": "2026-07-17T01:00:00Z",
                "eventType": "task",
                "taskId": task.id
            })),
        )
        .unwrap();
        crate::database::tasks::delete_task(&conn, &task.id).unwrap();
        assert_eq!(get_event(&conn, &event.id).unwrap_err().code, "EVENT_NOT_FOUND");
    }
}
