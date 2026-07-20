use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::database::double_option;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskColor {
    Blue,
    Indigo,
    Violet,
    Pink,
    Red,
    Orange,
    Green,
    Teal,
}

impl TaskColor {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskColor::Blue => "blue",
            TaskColor::Indigo => "indigo",
            TaskColor::Violet => "violet",
            TaskColor::Pink => "pink",
            TaskColor::Red => "red",
            TaskColor::Orange => "orange",
            TaskColor::Green => "green",
            TaskColor::Teal => "teal",
        }
    }

    fn from_db(value: &str) -> Self {
        match value {
            "indigo" => TaskColor::Indigo,
            "violet" => TaskColor::Violet,
            "pink" => TaskColor::Pink,
            "red" => TaskColor::Red,
            "orange" => TaskColor::Orange,
            "green" => TaskColor::Green,
            "teal" => TaskColor::Teal,
            _ => TaskColor::Blue,
        }
    }
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskPriority::High => "high",
            TaskPriority::Medium => "medium",
            TaskPriority::Low => "low",
        }
    }

    fn from_db(value: &str) -> Self {
        match value {
            "high" => TaskPriority::High,
            "low" => TaskPriority::Low,
            _ => TaskPriority::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    OnHold,
    Completed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::OnHold => "on_hold",
            TaskStatus::Completed => "completed",
            TaskStatus::Cancelled => "cancelled",
        }
    }

    fn from_db(value: &str) -> Self {
        match value {
            "in_progress" => TaskStatus::InProgress,
            "on_hold" => TaskStatus::OnHold,
            "completed" => TaskStatus::Completed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Todo,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "dueAtUtc")]
    pub due_at: Option<String>,
    pub timezone: String,
    pub priority: TaskPriority,
    pub color: TaskColor,
    pub status: TaskStatus,
    pub assignee: Option<String>,
    pub project_name: Option<String>,
    pub meeting_id: Option<String>,
    pub linked_file_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

fn default_timezone() -> String {
    "Asia/Tokyo".to_string()
}

fn default_priority() -> TaskPriority {
    TaskPriority::Medium
}

fn default_color() -> TaskColor {
    TaskColor::Blue
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub due_at_utc: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_priority")]
    pub priority: TaskPriority,
    #[serde(default = "default_color")]
    pub color: TaskColor,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub meeting_id: Option<String>,
    #[serde(default)]
    pub linked_file_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TaskPatch {
    pub title: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub due_at_utc: Option<Option<String>>,
    pub timezone: Option<String>,
    pub priority: Option<TaskPriority>,
    pub color: Option<TaskColor>,
    pub status: Option<TaskStatus>,
    #[serde(deserialize_with = "double_option")]
    pub assignee: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub project_name: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub meeting_id: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub linked_file_path: Option<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskPreset {
    All,
    Open,
    InProgress,
    Completed,
    Today,
    ThisWeek,
    Overdue,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TaskFilter {
    pub preset: Option<TaskPreset>,
    pub priority: Option<TaskPriority>,
    pub project_name: Option<String>,
    pub assignee: Option<String>,
}

const SELECT_COLUMNS: &str = "id, title, description, due_at, timezone, priority, color, status, assignee, project_name, meeting_id, linked_file_path, created_at, updated_at, completed_at";

const DEFAULT_ORDER: &str = "ORDER BY
  CASE WHEN status = 'completed' THEN 1 ELSE 0 END,
  CASE WHEN due_at IS NULL THEN 1 ELSE 0 END,
  due_at ASC,
  CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
  created_at ASC";

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let priority: String = row.get(5)?;
    let color: String = row.get(6)?;
    let status: String = row.get(7)?;
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        due_at: row.get(3)?,
        timezone: row.get(4)?,
        priority: TaskPriority::from_db(&priority),
        color: TaskColor::from_db(&color),
        status: TaskStatus::from_db(&status),
        assignee: row.get(8)?,
        project_name: row.get(9)?,
        meeting_id: row.get(10)?,
        linked_file_path: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        completed_at: row.get(14)?,
    })
}

fn validation_error(message: impl Into<String>) -> AppError {
    AppError::new("VALIDATION_ERROR", message, false)
}

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(validation_error("タイトルを入力してください"));
    }
    Ok(())
}

fn parse_utc(value: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| validation_error(format!("日時はRFC3339形式で指定してください: {value}")))
}

/// 期日がAsia/Tokyoの0時ちょうど（=日付のみのタスク）かを判定する（§13.4）。
pub fn is_date_only_due(due_at_utc: &str) -> bool {
    use chrono::Timelike;
    match DateTime::parse_from_rfc3339(due_at_utc) {
        Ok(dt) => {
            let tokyo = dt.with_timezone(&chrono_tz::Asia::Tokyo);
            tokyo.hour() == 0 && tokyo.minute() == 0 && tokyo.second() == 0
        }
        Err(_) => false,
    }
}

pub fn create_task(conn: &Connection, input: &TaskInput) -> Result<Task, AppError> {
    validate_title(&input.title)?;
    if let Some(due) = &input.due_at_utc {
        parse_utc(due)?;
    }
    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    let status = input.status.unwrap_or(TaskStatus::Todo);
    let completed_at = (status == TaskStatus::Completed).then(|| now.clone());
    conn.execute(
        "INSERT INTO tasks (id, title, description, due_at, timezone, priority, color, status, assignee, project_name, meeting_id, linked_file_path, created_at, updated_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?14)",
        rusqlite::params![
            id,
            input.title.trim(),
            input.description,
            input.due_at_utc,
            input.timezone,
            input.priority.as_str(),
            input.color.as_str(),
            status.as_str(),
            input.assignee,
            input.project_name,
            input.meeting_id,
            input.linked_file_path,
            now,
            completed_at,
        ],
    )?;
    get_task(conn, &id)
}

pub fn get_task(conn: &Connection, id: &str) -> Result<Task, AppError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM tasks WHERE id = ?1"),
        [id],
        row_to_task,
    )
    .optional()?
    .ok_or_else(|| AppError::new("TASK_NOT_FOUND", format!("タスクが存在しません: {id}"), false))
}

pub fn update_task(conn: &Connection, id: &str, patch: &TaskPatch) -> Result<Task, AppError> {
    let current = get_task(conn, id)?;
    let title = patch.title.clone().unwrap_or(current.title);
    validate_title(&title)?;
    let due_at = patch.due_at_utc.clone().unwrap_or(current.due_at);
    if let Some(due) = &due_at {
        parse_utc(due)?;
    }
    let status = patch.status.unwrap_or(current.status);
    let now = Utc::now().to_rfc3339();
    let completed_at = if status == TaskStatus::Completed {
        current.completed_at.or(Some(now.clone()))
    } else {
        None
    };
    conn.execute(
        "UPDATE tasks SET title = ?2, description = ?3, due_at = ?4, timezone = ?5, priority = ?6, color = ?7, status = ?8,
           assignee = ?9, project_name = ?10, meeting_id = ?11, linked_file_path = ?12, updated_at = ?13, completed_at = ?14
         WHERE id = ?1",
        rusqlite::params![
            id,
            title.trim(),
            patch.description.clone().unwrap_or(current.description),
            due_at,
            patch.timezone.clone().unwrap_or(current.timezone),
            patch.priority.unwrap_or(current.priority).as_str(),
            patch.color.unwrap_or(current.color).as_str(),
            status.as_str(),
            patch.assignee.clone().unwrap_or(current.assignee),
            patch.project_name.clone().unwrap_or(current.project_name),
            patch.meeting_id.clone().unwrap_or(current.meeting_id),
            patch.linked_file_path.clone().unwrap_or(current.linked_file_path),
            now,
            completed_at,
        ],
    )?;
    get_task(conn, id)
}

pub fn delete_task(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(AppError::new(
            "TASK_NOT_FOUND",
            format!("タスクが存在しません: {id}"),
            false,
        ));
    }
    Ok(())
}

pub fn complete_task(conn: &Connection, id: &str) -> Result<Task, AppError> {
    get_task(conn, id)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET status = 'completed', completed_at = ?2, updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now],
    )?;
    get_task(conn, id)
}

pub fn reopen_task(conn: &Connection, id: &str) -> Result<Task, AppError> {
    get_task(conn, id)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET status = 'todo', completed_at = NULL, updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now],
    )?;
    get_task(conn, id)
}

fn tokyo_date(due_at_utc: &str) -> Option<chrono::NaiveDate> {
    DateTime::parse_from_rfc3339(due_at_utc)
        .ok()
        .map(|dt| dt.with_timezone(&chrono_tz::Asia::Tokyo).date_naive())
}

fn matches_date_preset(task: &Task, preset: TaskPreset, now: DateTime<Utc>) -> bool {
    use chrono::Datelike;
    let Some(due) = task.due_at.as_deref() else {
        return false;
    };
    let Some(due_date) = tokyo_date(due) else {
        return false;
    };
    let today = now.with_timezone(&chrono_tz::Asia::Tokyo).date_naive();
    match preset {
        TaskPreset::Today => due_date == today,
        TaskPreset::ThisWeek => {
            let week_start = today - chrono::Days::new(today.weekday().num_days_from_monday() as u64);
            let week_end = week_start + chrono::Days::new(7);
            due_date >= week_start && due_date < week_end
        }
        TaskPreset::Overdue => {
            if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
                return false;
            }
            if is_date_only_due(due) {
                due_date < today
            } else {
                parse_utc(due).map(|dt| dt < now).unwrap_or(false)
            }
        }
        _ => true,
    }
}

/// §12.1の既定順で一覧を返し、日付系プリセットはAsia/Tokyo基準で絞り込む。
pub fn list_tasks(
    conn: &Connection,
    filter: &TaskFilter,
    now: DateTime<Utc>,
) -> Result<Vec<Task>, AppError> {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();
    match filter.preset {
        Some(TaskPreset::Open) => {
            conditions.push("status NOT IN ('completed', 'cancelled')".to_string());
        }
        Some(TaskPreset::InProgress) => conditions.push("status = 'in_progress'".to_string()),
        Some(TaskPreset::Completed) => conditions.push("status = 'completed'".to_string()),
        _ => {}
    }
    if let Some(priority) = filter.priority {
        params.push(priority.as_str().to_string());
        conditions.push(format!("priority = ?{}", params.len()));
    }
    if let Some(project) = &filter.project_name {
        params.push(project.clone());
        conditions.push(format!("project_name = ?{}", params.len()));
    }
    if let Some(assignee) = &filter.assignee {
        params.push(assignee.clone());
        conditions.push(format!("assignee = ?{}", params.len()));
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM tasks {where_clause} {DEFAULT_ORDER}"
    ))?;
    let tasks = stmt
        .query_map(rusqlite::params_from_iter(params.iter()), row_to_task)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let tasks = match filter.preset {
        Some(preset @ (TaskPreset::Today | TaskPreset::ThisWeek | TaskPreset::Overdue)) => tasks
            .into_iter()
            .filter(|t| matches_date_preset(t, preset, now))
            .collect(),
        _ => tasks,
    };
    Ok(tasks)
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

    fn input(title: &str) -> TaskInput {
        serde_json::from_value(serde_json::json!({ "title": title })).unwrap()
    }

    fn input_json(value: serde_json::Value) -> TaskInput {
        serde_json::from_value(value).unwrap()
    }

    fn patch_json(value: serde_json::Value) -> TaskPatch {
        serde_json::from_value(value).unwrap()
    }

    fn now() -> DateTime<Utc> {
        // Asia/Tokyoで2026-07-17（金）12:00
        "2026-07-17T03:00:00Z".parse().unwrap()
    }

    #[test]
    fn タスクを作成すると既定値が入る() {
        let (_dir, conn) = temp_conn();
        let task = create_task(&conn, &input("資料作成")).unwrap();
        assert_eq!(task.title, "資料作成");
        assert_eq!(task.priority, TaskPriority::Medium);
        assert_eq!(task.color, TaskColor::Blue);
        assert_eq!(task.status, TaskStatus::Todo);
        assert_eq!(task.timezone, "Asia/Tokyo");
        assert!(task.due_at.is_none());
        assert!(task.completed_at.is_none());
        let fetched = get_task(&conn, &task.id).unwrap();
        assert_eq!(fetched.id, task.id);
    }

    #[test]
    fn 空タイトルは拒否する() {
        let (_dir, conn) = temp_conn();
        let err = create_task(&conn, &input("  ")).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 不正な期日文字列は拒否する() {
        let (_dir, conn) = temp_conn();
        let err = create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "t", "dueAtUtc": "2026/07/17" })),
        )
        .unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 一覧は仕様の既定順で返る() {
        let (_dir, conn) = temp_conn();
        let done = create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "完了済み", "dueAtUtc": "2026-07-16T15:00:00Z" })),
        )
        .unwrap();
        complete_task(&conn, &done.id).unwrap();
        create_task(&conn, &input("期日なし")).unwrap();
        create_task(
            &conn,
            &input_json(serde_json::json!({
                "title": "明日・低", "dueAtUtc": "2026-07-18T01:00:00Z", "priority": "low"
            })),
        )
        .unwrap();
        create_task(
            &conn,
            &input_json(serde_json::json!({
                "title": "明日・高", "dueAtUtc": "2026-07-18T01:00:00Z", "priority": "high"
            })),
        )
        .unwrap();
        create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "今日", "dueAtUtc": "2026-07-17T01:00:00Z" })),
        )
        .unwrap();
        let titles: Vec<String> = list_tasks(&conn, &TaskFilter::default(), now())
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect();
        assert_eq!(titles, vec!["今日", "明日・高", "明日・低", "期日なし", "完了済み"]);
    }

    #[test]
    fn 同一期日同一優先度は作成順() {
        let (_dir, conn) = temp_conn();
        let first = create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "先", "dueAtUtc": "2026-07-18T01:00:00Z" })),
        )
        .unwrap();
        let second = create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "後", "dueAtUtc": "2026-07-18T01:00:00Z" })),
        )
        .unwrap();
        conn.execute(
            "UPDATE tasks SET created_at = '2026-07-01T00:00:00Z' WHERE id = ?1",
            [&first.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE tasks SET created_at = '2026-07-02T00:00:00Z' WHERE id = ?1",
            [&second.id],
        )
        .unwrap();
        let titles: Vec<String> = list_tasks(&conn, &TaskFilter::default(), now())
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect();
        assert_eq!(titles, vec!["先", "後"]);
    }

    #[test]
    fn パッチで指定フィールドだけ更新できる() {
        let (_dir, conn) = temp_conn();
        let task = create_task(
            &conn,
            &input_json(serde_json::json!({
                "title": "元", "description": "説明", "dueAtUtc": "2026-07-17T01:00:00Z"
            })),
        )
        .unwrap();
        let updated = update_task(
            &conn,
            &task.id,
            &patch_json(serde_json::json!({ "title": "新", "description": null })),
        )
        .unwrap();
        assert_eq!(updated.title, "新");
        assert!(updated.description.is_none());
        assert_eq!(updated.due_at.as_deref(), Some("2026-07-17T01:00:00Z"));
    }

    #[test]
    fn ステータスパッチでcompleted_atが同期する() {
        let (_dir, conn) = temp_conn();
        let task = create_task(&conn, &input("t")).unwrap();
        let done = update_task(
            &conn,
            &task.id,
            &patch_json(serde_json::json!({ "status": "completed" })),
        )
        .unwrap();
        assert_eq!(done.status, TaskStatus::Completed);
        assert!(done.completed_at.is_some());
        let reopened = update_task(
            &conn,
            &task.id,
            &patch_json(serde_json::json!({ "status": "in_progress" })),
        )
        .unwrap();
        assert_eq!(reopened.status, TaskStatus::InProgress);
        assert!(reopened.completed_at.is_none());
    }

    #[test]
    fn 完了と再開が往復できる() {
        let (_dir, conn) = temp_conn();
        let task = create_task(&conn, &input("t")).unwrap();
        let done = complete_task(&conn, &task.id).unwrap();
        assert_eq!(done.status, TaskStatus::Completed);
        assert!(done.completed_at.is_some());
        let reopened = reopen_task(&conn, &task.id).unwrap();
        assert_eq!(reopened.status, TaskStatus::Todo);
        assert!(reopened.completed_at.is_none());
    }

    #[test]
    fn 削除すると取得できない() {
        let (_dir, conn) = temp_conn();
        let task = create_task(&conn, &input("t")).unwrap();
        delete_task(&conn, &task.id).unwrap();
        let err = get_task(&conn, &task.id).unwrap_err();
        assert_eq!(err.code, "TASK_NOT_FOUND");
    }

    #[test]
    fn 存在しないidの操作はtask_not_found() {
        let (_dir, conn) = temp_conn();
        assert_eq!(get_task(&conn, "missing").unwrap_err().code, "TASK_NOT_FOUND");
        assert_eq!(delete_task(&conn, "missing").unwrap_err().code, "TASK_NOT_FOUND");
        assert_eq!(complete_task(&conn, "missing").unwrap_err().code, "TASK_NOT_FOUND");
    }

    #[test]
    fn ステータス系プリセットで絞り込める() {
        let (_dir, conn) = temp_conn();
        let a = create_task(&conn, &input("未着手")).unwrap();
        let b = create_task(&conn, &input("進行中")).unwrap();
        update_task(&conn, &b.id, &patch_json(serde_json::json!({ "status": "in_progress" }))).unwrap();
        let c = create_task(&conn, &input("完了")).unwrap();
        complete_task(&conn, &c.id).unwrap();
        let filter = |preset: &str| -> Vec<String> {
            let f: TaskFilter =
                serde_json::from_value(serde_json::json!({ "preset": preset })).unwrap();
            list_tasks(&conn, &f, now()).unwrap().into_iter().map(|t| t.title).collect()
        };
        assert_eq!(filter("open"), vec!["未着手", "進行中"]);
        assert_eq!(filter("inProgress"), vec!["進行中"]);
        assert_eq!(filter("completed"), vec!["完了"]);
        assert_eq!(filter("all").len(), 3);
        let _ = a;
    }

    #[test]
    fn 日付系プリセットはtokyo基準で絞り込める() {
        let (_dir, conn) = temp_conn();
        // Tokyo 7/17 の日付のみ（0時） = 今日
        create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "今日・日付のみ", "dueAtUtc": "2026-07-16T15:00:00Z" })),
        )
        .unwrap();
        // Tokyo 7/17 10:00 は now(12:00) より前 = 今日かつ期限切れ
        create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "今日・時刻超過", "dueAtUtc": "2026-07-17T01:00:00Z" })),
        )
        .unwrap();
        // Tokyo 7/16 の日付のみ = 期限切れ
        create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "昨日", "dueAtUtc": "2026-07-15T15:00:00Z" })),
        )
        .unwrap();
        // Tokyo 7/19（日） = 今週
        create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "日曜", "dueAtUtc": "2026-07-19T01:00:00Z" })),
        )
        .unwrap();
        // Tokyo 7/20（月） = 来週
        create_task(
            &conn,
            &input_json(serde_json::json!({ "title": "来週月曜", "dueAtUtc": "2026-07-20T01:00:00Z" })),
        )
        .unwrap();
        let filter = |preset: &str| -> Vec<String> {
            let f: TaskFilter =
                serde_json::from_value(serde_json::json!({ "preset": preset })).unwrap();
            list_tasks(&conn, &f, now()).unwrap().into_iter().map(|t| t.title).collect()
        };
        assert_eq!(filter("today"), vec!["今日・日付のみ", "今日・時刻超過"]);
        assert_eq!(filter("overdue"), vec!["昨日", "今日・時刻超過"]);
        assert_eq!(
            filter("thisWeek"),
            vec!["昨日", "今日・日付のみ", "今日・時刻超過", "日曜"]
        );
    }

    #[test]
    fn 属性フィルターで絞り込める() {
        let (_dir, conn) = temp_conn();
        create_task(
            &conn,
            &input_json(serde_json::json!({
                "title": "A高", "priority": "high", "projectName": "P1", "assignee": "山田"
            })),
        )
        .unwrap();
        create_task(
            &conn,
            &input_json(serde_json::json!({
                "title": "B中", "projectName": "P2", "assignee": "佐藤"
            })),
        )
        .unwrap();
        let by_priority: TaskFilter =
            serde_json::from_value(serde_json::json!({ "priority": "high" })).unwrap();
        let by_project: TaskFilter =
            serde_json::from_value(serde_json::json!({ "projectName": "P2" })).unwrap();
        let by_assignee: TaskFilter =
            serde_json::from_value(serde_json::json!({ "assignee": "山田" })).unwrap();
        let titles = |f: &TaskFilter| -> Vec<String> {
            list_tasks(&conn, f, now()).unwrap().into_iter().map(|t| t.title).collect()
        };
        assert_eq!(titles(&by_priority), vec!["A高"]);
        assert_eq!(titles(&by_project), vec!["B中"]);
        assert_eq!(titles(&by_assignee), vec!["A高"]);
    }

    #[test]
    fn 日付のみ判定はtokyoの0時で行う() {
        assert!(is_date_only_due("2026-07-16T15:00:00Z"));
        assert!(!is_date_only_due("2026-07-17T01:00:00Z"));
        assert!(!is_date_only_due("2026-07-16T15:30:00Z"));
    }
}
