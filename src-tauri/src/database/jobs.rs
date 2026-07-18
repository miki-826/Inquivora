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
