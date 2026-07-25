use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension};
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
    /// 周期通知の間隔（分）。Noneなら単発。
    pub repeat_interval_minutes: Option<i64>,
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
    /// 周期通知の間隔（分）。未指定・0以下なら単発として扱う。
    #[serde(default)]
    pub repeat_interval_minutes: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReminderPatch {
    pub notify_at_utc: Option<String>,
}

const SELECT_COLUMNS: &str = "id, task_id, event_id, notify_at, timezone, status, sent_at, repeat_interval_minutes, created_at, updated_at";

fn row_to_reminder(row: &rusqlite::Row<'_>) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: row.get(0)?,
        task_id: row.get(1)?,
        event_id: row.get(2)?,
        notify_at: row.get(3)?,
        timezone: row.get(4)?,
        status: row.get(5)?,
        sent_at: row.get(6)?,
        repeat_interval_minutes: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

/// 0以下や極端に短い間隔は誤設定として弾き、正の分数だけを周期として受理する。
fn normalize_repeat(minutes: Option<i64>) -> Option<i64> {
    minutes.filter(|value| *value > 0)
}

fn validation_error(message: impl Into<String>) -> AppError {
    AppError::new("VALIDATION_ERROR", message, false)
}

fn not_found(id: &str) -> AppError {
    AppError::new(
        "REMINDER_NOT_FOUND",
        format!("リマインダーが存在しません: {id}"),
        false,
    )
}

/// RFC3339をUTCのZ表記へ正規化する。
fn normalize_utc(value: &str) -> Result<String, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Secs, true))
        .map_err(|_| validation_error(format!("通知時刻はRFC3339形式で指定してください: {value}")))
}

fn validate_target(task_id: Option<&str>, event_id: Option<&str>) -> Result<(), AppError> {
    match (task_id, event_id) {
        (Some(_), None) | (None, Some(_)) => Ok(()),
        _ => Err(validation_error(
            "タスクまたは予定のどちらか一方を指定してください",
        )),
    }
}

fn is_unique_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(e, _)
            if e.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

fn insert_reminder(conn: &Connection, input: &ReminderInput) -> Result<Option<Reminder>, AppError> {
    validate_target(input.task_id.as_deref(), input.event_id.as_deref())?;
    let notify_at = normalize_utc(&input.notify_at_utc)?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let id = uuid::Uuid::new_v4().to_string();
    let repeat = normalize_repeat(input.repeat_interval_minutes);
    let result = conn.execute(
        "INSERT INTO reminders (id, task_id, event_id, notify_at, timezone, status, sent_at, repeat_interval_minutes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'scheduled', NULL, ?6, ?7, ?7)",
        rusqlite::params![id, input.task_id, input.event_id, notify_at, input.timezone, repeat, now],
    );
    match result {
        Ok(_) => Ok(Some(get_reminder(conn, &id)?)),
        Err(err) if is_unique_violation(&err) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub fn create_reminder(conn: &Connection, input: &ReminderInput) -> Result<Reminder, AppError> {
    insert_reminder(conn, input)?
        .ok_or_else(|| validation_error("同じ対象・同じ時刻の通知が既に存在します"))
}

/// 同一対象・同一時刻の重複はエラーにせずNoneを返す（自動作成用、§14.6）。
pub fn create_reminder_if_absent(
    conn: &Connection,
    input: &ReminderInput,
) -> Result<Option<Reminder>, AppError> {
    insert_reminder(conn, input)
}

pub fn get_reminder(conn: &Connection, id: &str) -> Result<Reminder, AppError> {
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM reminders WHERE id = ?1"),
        [id],
        row_to_reminder,
    )
    .optional()?
    .ok_or_else(|| not_found(id))
}

pub fn update_reminder(
    conn: &Connection,
    id: &str,
    patch: &ReminderPatch,
) -> Result<Reminder, AppError> {
    let current = get_reminder(conn, id)?;
    let notify_at = match &patch.notify_at_utc {
        Some(value) => normalize_utc(value)?,
        None => current.notify_at,
    };
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    conn.execute(
        "UPDATE reminders SET notify_at = ?2, status = 'scheduled', sent_at = NULL, updated_at = ?3
         WHERE id = ?1",
        rusqlite::params![id, notify_at, now],
    )?;
    get_reminder(conn, id)
}

pub fn delete_reminder(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn.execute("DELETE FROM reminders WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(not_found(id));
    }
    Ok(())
}

pub fn list_upcoming(conn: &Connection, limit: usize) -> Result<Vec<Reminder>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM reminders WHERE status = 'scheduled'
         ORDER BY datetime(notify_at) ASC LIMIT ?1"
    ))?;
    let list = stmt
        .query_map([limit as i64], row_to_reminder)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(list)
}

pub fn list_for_target(
    conn: &Connection,
    task_id: Option<&str>,
    event_id: Option<&str>,
) -> Result<Vec<Reminder>, AppError> {
    validate_target(task_id, event_id)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM reminders
         WHERE (?1 IS NOT NULL AND task_id = ?1) OR (?2 IS NOT NULL AND event_id = ?2)
         ORDER BY datetime(notify_at) ASC"
    ))?;
    let list = stmt
        .query_map(rusqlite::params![task_id, event_id], row_to_reminder)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(list)
}

pub fn due_reminders(conn: &Connection, now_utc: &str) -> Result<Vec<Reminder>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM reminders
         WHERE status = 'scheduled' AND datetime(notify_at) <= datetime(?1)
         ORDER BY datetime(notify_at) ASC"
    ))?;
    let list = stmt
        .query_map([now_utc], row_to_reminder)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(list)
}

pub fn mark_sent(conn: &Connection, id: &str, now_utc: &str) -> Result<(), AppError> {
    let affected = conn.execute(
        "UPDATE reminders SET status = 'sent', sent_at = ?2, updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now_utc],
    )?;
    if affected == 0 {
        return Err(not_found(id));
    }
    Ok(())
}

/// 現在時刻より後になるまで通知時刻へ間隔を足し続けた次回時刻を返す。
fn next_occurrence(notify_at: &str, now_utc: &str, interval_minutes: i64) -> Option<String> {
    let base = DateTime::parse_from_rfc3339(notify_at).ok()?.with_timezone(&Utc);
    let now = DateTime::parse_from_rfc3339(now_utc).ok()?.with_timezone(&Utc);
    let step = chrono::Duration::minutes(interval_minutes.max(1));
    let mut next = base + step;
    // スリープなどで長く止まっていても、未来の1回にまとめて追いつく。
    while next <= now {
        next += step;
    }
    Some(next.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// 通知配信後の後処理。周期通知なら次回時刻へ再スケジュールし、単発なら送信済みにする。
/// 戻り値は次回がまだ残っているか（true=周期継続, false=完了）。
pub fn advance_or_complete(conn: &Connection, id: &str, now_utc: &str) -> Result<bool, AppError> {
    let reminder = get_reminder(conn, id)?;
    match normalize_repeat(reminder.repeat_interval_minutes) {
        Some(interval) => {
            let next = next_occurrence(&reminder.notify_at, now_utc, interval)
                .unwrap_or_else(|| now_utc.to_string());
            conn.execute(
                "UPDATE reminders SET notify_at = ?2, status = 'scheduled', sent_at = NULL, updated_at = ?3
                 WHERE id = ?1",
                rusqlite::params![id, next, now_utc],
            )?;
            Ok(true)
        }
        None => {
            mark_sent(conn, id, now_utc)?;
            Ok(false)
        }
    }
}

fn set_status_scheduled(
    conn: &Connection,
    task_id: Option<&str>,
    event_id: Option<&str>,
    status: &str,
) -> Result<usize, AppError> {
    validate_target(task_id, event_id)?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let affected = conn.execute(
        "UPDATE reminders SET status = ?3, updated_at = ?4
         WHERE status = 'scheduled'
           AND ((?1 IS NOT NULL AND task_id = ?1) OR (?2 IS NOT NULL AND event_id = ?2))",
        rusqlite::params![task_id, event_id, status, now],
    )?;
    Ok(affected)
}

pub fn cancel_scheduled(
    conn: &Connection,
    task_id: Option<&str>,
    event_id: Option<&str>,
) -> Result<usize, AppError> {
    set_status_scheduled(conn, task_id, event_id, "cancelled")
}

pub fn shift_scheduled(
    conn: &Connection,
    task_id: Option<&str>,
    event_id: Option<&str>,
    delta_seconds: i64,
) -> Result<usize, AppError> {
    let targets: Vec<Reminder> = list_for_target(conn, task_id, event_id)?
        .into_iter()
        .filter(|r| r.status == "scheduled")
        .collect();
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut affected = 0;
    for reminder in targets {
        let Ok(current) = DateTime::parse_from_rfc3339(&reminder.notify_at) else {
            continue;
        };
        let shifted = (current.with_timezone(&Utc) + chrono::Duration::seconds(delta_seconds))
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        affected += conn.execute(
            "UPDATE reminders SET notify_at = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![reminder.id, shifted, now],
        )?;
    }
    Ok(affected)
}

pub fn count_for_target(
    conn: &Connection,
    task_id: Option<&str>,
    event_id: Option<&str>,
) -> Result<i64, AppError> {
    validate_target(task_id, event_id)?;
    let count = conn.query_row(
        "SELECT COUNT(*) FROM reminders
         WHERE (?1 IS NOT NULL AND task_id = ?1) OR (?2 IS NOT NULL AND event_id = ?2)",
        rusqlite::params![task_id, event_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// 通知予定時刻から24時間以上経過したscheduledをexpiredにする。
pub fn expire_stale(conn: &Connection, now_utc: &str) -> Result<usize, AppError> {
    let affected = conn.execute(
        "UPDATE reminders SET status = 'expired', updated_at = ?1
         WHERE status = 'scheduled' AND repeat_interval_minutes IS NULL
           AND datetime(notify_at) <= datetime(?1, '-24 hours')",
        [now_utc],
    )?;
    Ok(affected)
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
    fn 周期通知は配信後に次回時刻へ再スケジュールされる() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({
                "taskId": task_id,
                "notifyAtUtc": "2026-07-18T00:00:00Z",
                "repeatIntervalMinutes": 60
            })),
        )
        .unwrap();
        assert_eq!(reminder.repeat_interval_minutes, Some(60));
        // 2回分遅れて配信された場合でも、現在時刻より後の1回にまとめて追いつく。
        let repeated = advance_or_complete(&conn, &reminder.id, "2026-07-18T01:30:00Z").unwrap();
        assert!(repeated);
        let next = get_reminder(&conn, &reminder.id).unwrap();
        assert_eq!(next.status, "scheduled");
        assert!(next.sent_at.is_none());
        assert_eq!(next.notify_at, "2026-07-18T02:00:00Z");
    }

    #[test]
    fn 単発通知はadvanceで送信済みになる() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({ "taskId": task_id, "notifyAtUtc": "2026-07-18T00:00:00Z" })),
        )
        .unwrap();
        let repeated = advance_or_complete(&conn, &reminder.id, "2026-07-18T00:00:05Z").unwrap();
        assert!(!repeated);
        assert_eq!(get_reminder(&conn, &reminder.id).unwrap().status, "sent");
    }

    #[test]
    fn 周期通知は期限切れ整理の対象にならない() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let repeating = create_reminder(
            &conn,
            &input(serde_json::json!({
                "taskId": task_id,
                "notifyAtUtc": "2026-07-15T00:00:00Z",
                "repeatIntervalMinutes": 1440
            })),
        )
        .unwrap();
        let affected = expire_stale(&conn, "2026-07-18T00:00:00Z").unwrap();
        assert_eq!(affected, 0);
        assert_eq!(get_reminder(&conn, &repeating.id).unwrap().status, "scheduled");
    }

    #[test]
    fn 非正の繰り返し間隔は単発として保存される() {
        let (_dir, conn) = temp_conn();
        let task_id = make_task(&conn);
        let reminder = create_reminder(
            &conn,
            &input(serde_json::json!({
                "taskId": task_id,
                "notifyAtUtc": "2026-07-18T00:00:00Z",
                "repeatIntervalMinutes": 0
            })),
        )
        .unwrap();
        assert!(reminder.repeat_interval_minutes.is_none());
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
