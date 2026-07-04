use std::path::Path;

use rusqlite::Connection;

use crate::error::AppError;

pub const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../../migrations/001_init.sql"))];

pub fn open_database(_path: &Path) -> Result<Connection, AppError> {
    Err(AppError::new("NOT_IMPLEMENTED", "未実装", false))
}

pub fn run_migrations(_conn: &mut Connection) -> Result<(), AppError> {
    Err(AppError::new("NOT_IMPLEMENTED", "未実装", false))
}

pub fn current_schema_version(_conn: &Connection) -> Result<i64, AppError> {
    Err(AppError::new("NOT_IMPLEMENTED", "未実装", false))
}

fn strip_pragma_statements(sql: &str) -> String {
    let _ = sql;
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_temp_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("一時ディレクトリを作成できない");
        let conn = open_database(&dir.path().join("test.db")).expect("DBを開けない");
        (dir, conn)
    }

    #[test]
    fn マイグレーション適用でスキーマバージョンが1になる() {
        let (_dir, conn) = open_temp_db();
        assert_eq!(current_schema_version(&conn).unwrap(), 1);
    }

    #[test]
    fn マイグレーション適用で主要テーブルが作成される() {
        let (_dir, conn) = open_temp_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table', 'index') AND name IN (
                    'workspaces', 'files', 'meetings', 'transcript_segments', 'meeting_decisions',
                    'tasks', 'task_candidates', 'events', 'reminders', 'api_provider_profiles',
                    'ai_feature_bindings', 'api_usage_logs', 'api_jobs', 'app_settings',
                    'recent_tabs', 'search_documents', 'search_documents_fts'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 17);
    }

    #[test]
    fn マイグレーションは再適用しても冪等である() {
        let (_dir, mut conn) = open_temp_db();
        run_migrations(&mut conn).expect("再適用に失敗");
        assert_eq!(current_schema_version(&conn).unwrap(), 1);
    }

    #[test]
    fn 外部キー制約が有効である() {
        let (_dir, conn) = open_temp_db();
        let result = conn.execute(
            "INSERT INTO transcript_segments
                (id, meeting_id, source, speaker_label, start_ms, end_ms, text, status, created_at)
             VALUES ('seg1', 'missing-meeting', 'mic', '自分', 0, 1000, 'テスト', 'final', '2026-07-04T00:00:00Z')",
            [],
        );
        assert!(result.is_err(), "存在しないmeeting_idへの挿入は失敗すべき");
    }

    #[test]
    fn journal_modeがwalである() {
        let (_dir, conn) = open_temp_db();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn strip_pragma_statementsはpragma行だけを除去する() {
        let sql = "PRAGMA foreign_keys = ON;\nCREATE TABLE t (id TEXT);\nPRAGMA journal_mode = WAL;\n";
        let stripped = strip_pragma_statements(sql);
        assert!(!stripped.contains("PRAGMA"));
        assert!(stripped.contains("CREATE TABLE t (id TEXT);"));
    }

    #[test]
    fn fts5のtrigram検索が機能する() {
        let (_dir, conn) = open_temp_db();
        conn.execute(
            "INSERT INTO search_documents_fts (title, body, path, entity_type, entity_id)
             VALUES ('議事録', '8月から試験導入を開始します', NULL, 'meeting', 'm1')",
            [],
        )
        .unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM search_documents_fts WHERE search_documents_fts MATCH '試験導入'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
