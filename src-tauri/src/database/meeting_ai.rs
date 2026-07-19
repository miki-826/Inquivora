use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

use crate::error::AppError;

/// meetings.summary を更新する。
pub fn set_summary(conn: &Connection, meeting_id: &str, summary: &str) -> Result<(), AppError> {
    let affected = conn.execute(
        "UPDATE meetings SET summary = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![meeting_id, summary, Utc::now().to_rfc3339()],
    )?;
    if affected == 0 {
        return Err(AppError::new("MEETING_NOT_FOUND", "会議が見つかりません", false));
    }
    Ok(())
}

pub struct DecisionInput {
    pub text: String,
    pub source_start_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingDecision {
    pub id: String,
    pub meeting_id: String,
    pub text: String,
    pub source_start_ms: Option<i64>,
    pub created_at: String,
}

fn row_to_decision(row: &Row) -> rusqlite::Result<MeetingDecision> {
    Ok(MeetingDecision {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        text: row.get(2)?,
        source_start_ms: row.get(3)?,
        created_at: row.get(4)?,
    })
}

/// 会議の決定事項を丸ごと置き換える（再生成時に重複しないよう既存を削除する）。
pub fn replace_decisions(
    conn: &Connection,
    meeting_id: &str,
    decisions: &[DecisionInput],
) -> Result<(), AppError> {
    conn.execute("DELETE FROM meeting_decisions WHERE meeting_id = ?1", [meeting_id])?;
    let now = Utc::now().to_rfc3339();
    for decision in decisions {
        conn.execute(
            "INSERT INTO meeting_decisions (id, meeting_id, text, source_start_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                meeting_id,
                decision.text,
                decision.source_start_ms,
                now,
            ],
        )?;
    }
    Ok(())
}

pub fn list_decisions(conn: &Connection, meeting_id: &str) -> Result<Vec<MeetingDecision>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, meeting_id, text, source_start_ms, created_at
         FROM meeting_decisions WHERE meeting_id = ?1 ORDER BY created_at, id",
    )?;
    let rows = stmt.query_map([meeting_id], row_to_decision)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub struct CandidateInput {
    pub title: String,
    pub description: Option<String>,
    pub due_at: Option<String>,
    pub priority: String,
    pub assignee: Option<String>,
    pub source_start_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCandidate {
    pub id: String,
    pub meeting_id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "dueAtUtc")]
    pub due_at: Option<String>,
    pub priority: String,
    pub assignee: Option<String>,
    pub source_start_ms: Option<i64>,
    pub status: String,
    pub created_at: String,
}

const CANDIDATE_COLUMNS: &str = "id, meeting_id, title, description, due_at, priority, assignee, \
     source_start_ms, status, created_at";

fn row_to_candidate(row: &Row) -> rusqlite::Result<TaskCandidate> {
    Ok(TaskCandidate {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        due_at: row.get(4)?,
        priority: row.get(5)?,
        assignee: row.get(6)?,
        source_start_ms: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
    })
}

/// 会議のタスク候補を丸ごと置き換える。承認済み候補（status='accepted'）は保持する。
pub fn replace_candidates(
    conn: &Connection,
    meeting_id: &str,
    candidates: &[CandidateInput],
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM task_candidates WHERE meeting_id = ?1 AND status != 'accepted'",
        [meeting_id],
    )?;
    let now = Utc::now().to_rfc3339();
    for candidate in candidates {
        conn.execute(
            "INSERT INTO task_candidates
                (id, meeting_id, title, description, due_at, priority, assignee,
                 source_start_ms, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                meeting_id,
                candidate.title,
                candidate.description,
                candidate.due_at,
                candidate.priority,
                candidate.assignee,
                candidate.source_start_ms,
                now,
            ],
        )?;
    }
    Ok(())
}

pub fn list_candidates(conn: &Connection, meeting_id: &str) -> Result<Vec<TaskCandidate>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {CANDIDATE_COLUMNS} FROM task_candidates
         WHERE meeting_id = ?1 ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map([meeting_id], row_to_candidate)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_candidate(conn: &Connection, id: &str) -> Result<TaskCandidate, AppError> {
    conn.query_row(
        &format!("SELECT {CANDIDATE_COLUMNS} FROM task_candidates WHERE id = ?1"),
        [id],
        row_to_candidate,
    )
    .optional()?
    .ok_or_else(|| AppError::new("CANDIDATE_NOT_FOUND", "タスク候補が見つかりません", false))
}

pub fn set_candidate_status(conn: &Connection, id: &str, status: &str) -> Result<(), AppError> {
    let affected = conn.execute(
        "UPDATE task_candidates SET status = ?2 WHERE id = ?1",
        rusqlite::params![id, status],
    )?;
    if affected == 0 {
        return Err(AppError::new(
            "CANDIDATE_NOT_FOUND",
            "タスク候補が見つかりません",
            false,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::meetings::{create_meeting, MeetingInput};
    use crate::database::open_database;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    fn meeting(conn: &Connection) -> String {
        create_meeting(
            conn,
            MeetingInput {
                title: "会議".to_string(),
                workspace_id: None,
                target_file_path: "C:/notes/m.md".to_string(),
                timezone: "Asia/Tokyo".to_string(),
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn 要約を保存できる() {
        let (_dir, conn) = temp_conn();
        let id = meeting(&conn);
        set_summary(&conn, &id, "要約本文").unwrap();
        let summary: Option<String> = conn
            .query_row("SELECT summary FROM meetings WHERE id = ?1", [&id], |r| r.get(0))
            .unwrap();
        assert_eq!(summary.as_deref(), Some("要約本文"));
    }

    #[test]
    fn 決定事項は置き換えで重複しない() {
        let (_dir, conn) = temp_conn();
        let id = meeting(&conn);
        replace_decisions(
            &conn,
            &id,
            &[DecisionInput { text: "旧決定".to_string(), source_start_ms: Some(1000) }],
        )
        .unwrap();
        replace_decisions(
            &conn,
            &id,
            &[
                DecisionInput { text: "決定A".to_string(), source_start_ms: None },
                DecisionInput { text: "決定B".to_string(), source_start_ms: Some(5000) },
            ],
        )
        .unwrap();
        let list = list_decisions(&conn, &id).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].text, "決定A");
        assert_eq!(list[1].source_start_ms, Some(5000));
    }

    #[test]
    fn 承認済み候補は再生成で残る() {
        let (_dir, conn) = temp_conn();
        let id = meeting(&conn);
        replace_candidates(
            &conn,
            &id,
            &[CandidateInput {
                title: "承認予定".to_string(),
                description: None,
                due_at: None,
                priority: "medium".to_string(),
                assignee: None,
                source_start_ms: None,
            }],
        )
        .unwrap();
        let candidate = list_candidates(&conn, &id).unwrap().remove(0);
        set_candidate_status(&conn, &candidate.id, "accepted").unwrap();
        replace_candidates(
            &conn,
            &id,
            &[CandidateInput {
                title: "新候補".to_string(),
                description: Some("説明".to_string()),
                due_at: None,
                priority: "high".to_string(),
                assignee: Some("田中".to_string()),
                source_start_ms: Some(2000),
            }],
        )
        .unwrap();
        let list = list_candidates(&conn, &id).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|c| c.title == "承認予定" && c.status == "accepted"));
        assert!(list.iter().any(|c| c.title == "新候補" && c.priority == "high"));
    }

    #[test]
    fn 候補を取得し状態を更新できる() {
        let (_dir, conn) = temp_conn();
        let id = meeting(&conn);
        replace_candidates(
            &conn,
            &id,
            &[CandidateInput {
                title: "候補".to_string(),
                description: None,
                due_at: None,
                priority: "low".to_string(),
                assignee: None,
                source_start_ms: None,
            }],
        )
        .unwrap();
        let candidate = list_candidates(&conn, &id).unwrap().remove(0);
        assert_eq!(get_candidate(&conn, &candidate.id).unwrap().status, "pending");
        set_candidate_status(&conn, &candidate.id, "accepted").unwrap();
        assert_eq!(get_candidate(&conn, &candidate.id).unwrap().status, "accepted");
    }

    #[test]
    fn 存在しない候補はnot_found() {
        let (_dir, conn) = temp_conn();
        assert_eq!(get_candidate(&conn, "missing").unwrap_err().code, "CANDIDATE_NOT_FOUND");
    }
}
