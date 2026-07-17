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

const SELECT_COLUMNS: &str = "id, title, description, start_at, end_at, timezone, all_day, event_type, recurrence_rule, meeting_id, task_id, location, created_at, updated_at";

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<EventRecord> {
    let all_day: i64 = row.get(6)?;
    Ok(EventRecord {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        start_at: row.get(3)?,
        end_at: row.get(4)?,
        timezone: row.get(5)?,
        all_day: all_day != 0,
        event_type: row.get(7)?,
        recurrence_rule: row.get(8)?,
        meeting_id: row.get(9)?,
        task_id: row.get(10)?,
        location: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn validation_error(message: impl Into<String>) -> AppError {
    AppError::new("VALIDATION_ERROR", message, false)
}

fn validate(title: &str, start_at: &str, end_at: Option<&str>) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(validation_error("タイトルを入力してください"));
    }
    let start = chrono::DateTime::parse_from_rfc3339(start_at)
        .map_err(|_| validation_error(format!("開始日時はRFC3339形式で指定してください: {start_at}")))?;
    if let Some(end_at) = end_at {
        let end = chrono::DateTime::parse_from_rfc3339(end_at)
            .map_err(|_| validation_error(format!("終了日時はRFC3339形式で指定してください: {end_at}")))?;
        if end < start {
            return Err(validation_error("終了日時は開始日時より後にしてください"));
        }
    }
    Ok(())
}

pub fn create_event(conn: &Connection, input: &EventInput) -> Result<EventRecord, AppError> {
    validate(&input.title, &input.start_at_utc, input.end_at_utc.as_deref())?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO events (id, title, description, start_at, end_at, timezone, all_day, event_type, recurrence_rule, meeting_id, task_id, location, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
        rusqlite::params![
            id,
            input.title.trim(),
            input.description,
            input.start_at_utc,
            input.end_at_utc,
            input.timezone,
            input.all_day as i64,
            input.event_type,
            input.recurrence_rule,
            input.meeting_id,
            input.task_id,
            input.location,
            now,
        ],
    )?;
    get_event(conn, &id)
}

pub fn get_event(conn: &Connection, id: &str) -> Result<EventRecord, AppError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM events WHERE id = ?1"),
        [id],
        row_to_event,
    )
    .optional()?
    .ok_or_else(|| AppError::new("EVENT_NOT_FOUND", format!("予定が存在しません: {id}"), false))
}

pub fn update_event(
    conn: &Connection,
    id: &str,
    patch: &EventPatch,
) -> Result<EventRecord, AppError> {
    let current = get_event(conn, id)?;
    let title = patch.title.clone().unwrap_or(current.title);
    let start_at = patch.start_at_utc.clone().unwrap_or(current.start_at);
    let end_at = patch.end_at_utc.clone().unwrap_or(current.end_at);
    validate(&title, &start_at, end_at.as_deref())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE events SET title = ?2, description = ?3, start_at = ?4, end_at = ?5, timezone = ?6,
           all_day = ?7, event_type = ?8, recurrence_rule = ?9, location = ?10, updated_at = ?11
         WHERE id = ?1",
        rusqlite::params![
            id,
            title.trim(),
            patch.description.clone().unwrap_or(current.description),
            start_at,
            end_at,
            patch.timezone.clone().unwrap_or(current.timezone),
            patch.all_day.unwrap_or(current.all_day) as i64,
            patch.event_type.clone().unwrap_or(current.event_type),
            patch.recurrence_rule.clone().unwrap_or(current.recurrence_rule),
            patch.location.clone().unwrap_or(current.location),
            now,
        ],
    )?;
    get_event(conn, id)
}

pub fn delete_event(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn.execute("DELETE FROM events WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(AppError::new(
            "EVENT_NOT_FOUND",
            format!("予定が存在しません: {id}"),
            false,
        ));
    }
    Ok(())
}

/// [startUtc, endUtc) と重なる予定を開始日時の昇順で返す（§17.5）。
pub fn list_events_in_range(
    conn: &Connection,
    start_utc: &str,
    end_utc: &str,
) -> Result<Vec<EventRecord>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM events
         WHERE start_at < ?2 AND COALESCE(end_at, start_at) >= ?1
         ORDER BY start_at ASC"
    ))?;
    let events = stmt
        .query_map([start_utc, end_utc], row_to_event)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(events)
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
