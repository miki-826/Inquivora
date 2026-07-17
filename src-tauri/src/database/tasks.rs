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

/// 期日がAsia/Tokyoの0時ちょうど（=日付のみのタスク）かを判定する（§13.4）。
pub fn is_date_only_due(_due_at_utc: &str) -> bool {
    todo!()
}

pub fn create_task(_conn: &Connection, _input: &TaskInput) -> Result<Task, AppError> {
    todo!()
}

pub fn get_task(_conn: &Connection, _id: &str) -> Result<Task, AppError> {
    todo!()
}

pub fn update_task(_conn: &Connection, _id: &str, _patch: &TaskPatch) -> Result<Task, AppError> {
    todo!()
}

pub fn delete_task(_conn: &Connection, _id: &str) -> Result<(), AppError> {
    todo!()
}

pub fn complete_task(_conn: &Connection, _id: &str) -> Result<Task, AppError> {
    todo!()
}

pub fn reopen_task(_conn: &Connection, _id: &str) -> Result<Task, AppError> {
    todo!()
}

/// §12.1の既定順で一覧を返し、日付系プリセットはAsia/Tokyo基準で絞り込む。
pub fn list_tasks(
    _conn: &Connection,
    _filter: &TaskFilter,
    _now: DateTime<Utc>,
) -> Result<Vec<Task>, AppError> {
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
