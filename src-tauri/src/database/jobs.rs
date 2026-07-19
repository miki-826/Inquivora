use chrono::{Duration, Utc};
use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// §10.12 リトライ間隔。上限到達でNone（failed確定）。
pub fn retry_delay_seconds(retry_count: i64) -> Option<i64> {
    match retry_count {
        0 => Some(2),
        1 => Some(5),
        2 => Some(15),
        3 => Some(60),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInput {
    pub job_type: String,
    #[serde(default)]
    pub provider_profile_id: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub request_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiJob {
    pub id: String,
    pub job_type: String,
    pub provider_profile_id: Option<String>,
    pub model_id: Option<String>,
    pub capability: Option<String>,
    pub entity_id: Option<String>,
    pub request_path: Option<String>,
    pub status: String,
    pub retry_count: i64,
    pub next_retry_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to_job(row: &Row) -> rusqlite::Result<ApiJob> {
    Ok(ApiJob {
        id: row.get(0)?,
        job_type: row.get(1)?,
        provider_profile_id: row.get(2)?,
        model_id: row.get(3)?,
        capability: row.get(4)?,
        entity_id: row.get(5)?,
        request_path: row.get(6)?,
        status: row.get(7)?,
        retry_count: row.get(8)?,
        next_retry_at: row.get(9)?,
        last_error: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

const JOB_COLUMNS: &str = "id, job_type, provider_profile_id, model_id, capability, entity_id, \
     request_path, status, retry_count, next_retry_at, last_error, created_at, updated_at";

pub fn enqueue_job(conn: &Connection, input: JobInput) -> Result<ApiJob, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO api_jobs
            (id, job_type, provider_profile_id, model_id, capability, entity_id,
             request_path, status, retry_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', 0, ?8, ?8)",
        rusqlite::params![
            id,
            input.job_type,
            input.provider_profile_id,
            input.model_id,
            input.capability,
            input.entity_id,
            input.request_path,
            now,
        ],
    )?;
    conn.query_row(
        &format!("SELECT {JOB_COLUMNS} FROM api_jobs WHERE id = ?1"),
        [&id],
        row_to_job,
    )
    .map_err(Into::into)
}

pub fn next_pending_job(conn: &Connection) -> Result<Option<ApiJob>, AppError> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {JOB_COLUMNS} FROM api_jobs
                 WHERE status = 'pending'
                    OR (status = 'retry_wait' AND datetime(next_retry_at) <= datetime(?1))
                 ORDER BY created_at LIMIT 1"
            ),
            [Utc::now().to_rfc3339()],
            row_to_job,
        )
        .optional()?)
}

pub fn mark_job_processing(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE api_jobs SET status = 'processing', updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn complete_job(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE api_jobs SET status = 'completed', updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn fail_job(
    conn: &Connection,
    id: &str,
    error_code: &str,
    retryable: bool,
) -> Result<(), AppError> {
    let retry_count: i64 = conn
        .query_row("SELECT retry_count FROM api_jobs WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?
        .ok_or_else(|| AppError::new("API_JOB_NOT_FOUND", "ジョブが見つかりません", false))?;
    let now = Utc::now();
    match retry_delay_seconds(retry_count).filter(|_| retryable) {
        Some(delay) => {
            conn.execute(
                "UPDATE api_jobs SET status = 'retry_wait', retry_count = ?2,
                    next_retry_at = ?3, last_error = ?4, updated_at = ?5
                 WHERE id = ?1",
                rusqlite::params![
                    id,
                    retry_count + 1,
                    (now + Duration::seconds(delay)).to_rfc3339(),
                    error_code,
                    now.to_rfc3339(),
                ],
            )?;
        }
        None => {
            conn.execute(
                "UPDATE api_jobs SET status = 'failed', last_error = ?2, updated_at = ?3
                 WHERE id = ?1",
                rusqlite::params![id, error_code, now.to_rfc3339()],
            )?;
        }
    }
    Ok(())
}

/// §DoD 起動時のクラッシュ復旧。前回の実行中に中断された処理中ジョブを
/// pendingへ戻し、ワーカーが再処理できるようにする。戻した件数を返す。
pub fn requeue_interrupted_jobs(conn: &Connection) -> Result<usize, AppError> {
    let affected = conn.execute(
        "UPDATE api_jobs SET status = 'pending', updated_at = ?1 WHERE status = 'processing'",
        [Utc::now().to_rfc3339()],
    )?;
    Ok(affected)
}

pub fn cancel_jobs_for_entity(conn: &Connection, entity_id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE api_jobs SET status = 'cancelled', updated_at = ?2
         WHERE entity_id = ?1 AND status IN ('pending', 'retry_wait')",
        rusqlite::params![entity_id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;
    use rusqlite::Connection;

    fn open_temp_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("一時ディレクトリを作成できない");
        let conn = open_database(&dir.path().join("test.db")).expect("DBを開けない");
        (dir, conn)
    }

    fn sample_input() -> JobInput {
        JobInput {
            job_type: "transcription".to_string(),
            provider_profile_id: None,
            model_id: Some("whisper-1".to_string()),
            capability: Some("transcription.batch".to_string()),
            entity_id: Some("meeting-1".to_string()),
            request_path: Some(r#"{"path":"C:/audio/c0.wav","source":"mic","startMs":0,"endMs":20000}"#.to_string()),
        }
    }

    #[test]
    fn リトライ間隔は2_5_15_60秒である() {
        assert_eq!(retry_delay_seconds(0), Some(2));
        assert_eq!(retry_delay_seconds(1), Some(5));
        assert_eq!(retry_delay_seconds(2), Some(15));
        assert_eq!(retry_delay_seconds(3), Some(60));
        assert_eq!(retry_delay_seconds(4), None);
    }

    #[test]
    fn ジョブを登録するとpendingで取得できる() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        assert_eq!(job.status, "pending");
        assert_eq!(job.retry_count, 0);
        let next = next_pending_job(&conn).unwrap().unwrap();
        assert_eq!(next.id, job.id);
    }

    #[test]
    fn 処理中のジョブはnext_pendingに含まれない() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        mark_job_processing(&conn, &job.id).unwrap();
        assert!(next_pending_job(&conn).unwrap().is_none());
    }

    #[test]
    fn ジョブ完了でcompletedになる() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        complete_job(&conn, &job.id).unwrap();
        let status: String = conn
            .query_row("SELECT status FROM api_jobs WHERE id = ?1", [&job.id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "completed");
    }

    #[test]
    fn 再試行可能な失敗はretry_waitへ移り待機時刻が付く() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        fail_job(&conn, &job.id, "API_TIMEOUT", true).unwrap();
        let (status, retry_count, next_retry_at): (String, i64, Option<String>) = conn
            .query_row(
                "SELECT status, retry_count, next_retry_at FROM api_jobs WHERE id = ?1",
                [&job.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "retry_wait");
        assert_eq!(retry_count, 1);
        assert!(next_retry_at.is_some());
        assert!(next_pending_job(&conn).unwrap().is_none(), "待機中は取得されない");
    }

    #[test]
    fn 待機時刻を過ぎたretry_waitジョブは取得される() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        fail_job(&conn, &job.id, "API_TIMEOUT", true).unwrap();
        conn.execute(
            "UPDATE api_jobs SET next_retry_at = '2020-01-01T00:00:00Z' WHERE id = ?1",
            [&job.id],
        )
        .unwrap();
        let next = next_pending_job(&conn).unwrap().unwrap();
        assert_eq!(next.id, job.id);
        assert_eq!(next.retry_count, 1);
    }

    #[test]
    fn リトライ上限を超えるとfailedになる() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        for _ in 0..4 {
            fail_job(&conn, &job.id, "API_RATE_LIMITED", true).unwrap();
        }
        fail_job(&conn, &job.id, "API_RATE_LIMITED", true).unwrap();
        let status: String = conn
            .query_row("SELECT status FROM api_jobs WHERE id = ?1", [&job.id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "failed");
    }

    #[test]
    fn 再試行不可の失敗は即failedになる() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        fail_job(&conn, &job.id, "API_AUTH_FAILED", false).unwrap();
        let (status, last_error): (String, Option<String>) = conn
            .query_row(
                "SELECT status, last_error FROM api_jobs WHERE id = ?1",
                [&job.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "failed");
        assert_eq!(last_error.as_deref(), Some("API_AUTH_FAILED"));
    }

    #[test]
    fn 中断された処理中ジョブは起動時にpendingへ戻る() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        mark_job_processing(&conn, &job.id).unwrap();
        assert!(next_pending_job(&conn).unwrap().is_none());
        let requeued = requeue_interrupted_jobs(&conn).unwrap();
        assert_eq!(requeued, 1);
        let next = next_pending_job(&conn).unwrap().unwrap();
        assert_eq!(next.id, job.id);
    }

    #[test]
    fn 完了ジョブは復旧対象にしない() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        complete_job(&conn, &job.id).unwrap();
        assert_eq!(requeue_interrupted_jobs(&conn).unwrap(), 0);
    }

    #[test]
    fn ジョブをキャンセルできる() {
        let (_dir, conn) = open_temp_db();
        let job = enqueue_job(&conn, sample_input()).unwrap();
        cancel_jobs_for_entity(&conn, "meeting-1").unwrap();
        let status: String = conn
            .query_row("SELECT status FROM api_jobs WHERE id = ?1", [&job.id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "cancelled");
        assert!(next_pending_job(&conn).unwrap().is_none());
    }
}
