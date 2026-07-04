use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppError;

pub const RECENT_WORKSPACE_LIMIT: usize = 10;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub last_opened_at: String,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRecord> {
    Ok(WorkspaceRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        root_path: row.get(2)?,
        last_opened_at: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

const SELECT_COLUMNS: &str = "id, name, root_path, last_opened_at, created_at, updated_at";

/// ワークスペースを開いた記録を保存する。同一root_pathは最終使用日時のみ更新する。
pub fn open_workspace(conn: &Connection, root_path: &str, name: &str) -> Result<WorkspaceRecord, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, last_opened_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4, ?4)
         ON CONFLICT(root_path) DO UPDATE SET
           name = excluded.name,
           last_opened_at = excluded.last_opened_at,
           updated_at = excluded.updated_at",
        (uuid::Uuid::new_v4().to_string(), name, root_path, &now),
    )?;
    let record = conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM workspaces WHERE root_path = ?1"),
        [root_path],
        row_to_record,
    )?;
    Ok(record)
}

/// 最近開いたワークスペースを新しい順に最大10件返す（§7.1）。
pub fn list_recent_workspaces(conn: &Connection) -> Result<Vec<WorkspaceRecord>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM workspaces ORDER BY last_opened_at DESC LIMIT {RECENT_WORKSPACE_LIMIT}"
    ))?;
    let records = stmt
        .query_map([], row_to_record)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(records)
}

pub fn get_workspace(conn: &Connection, id: &str) -> Result<WorkspaceRecord, AppError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM workspaces WHERE id = ?1"),
        [id],
        row_to_record,
    )
    .optional()?
    .ok_or_else(|| AppError::new("WORKSPACE_NOT_FOUND", format!("ワークスペースが存在しません: {id}"), false))
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

    #[test]
    fn ワークスペースを登録して取得できる() {
        let (_dir, conn) = temp_conn();
        let ws = open_workspace(&conn, r"C:\Users\test\notes", "notes").unwrap();
        assert_eq!(ws.name, "notes");
        assert_eq!(ws.root_path, r"C:\Users\test\notes");
        let fetched = get_workspace(&conn, &ws.id).unwrap();
        assert_eq!(fetched.id, ws.id);
    }

    #[test]
    fn 同じパスを再度開いてもレコードは1件のまま() {
        let (_dir, conn) = temp_conn();
        let first = open_workspace(&conn, r"C:\ws", "ws").unwrap();
        let second = open_workspace(&conn, r"C:\ws", "ws").unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(list_recent_workspaces(&conn).unwrap().len(), 1);
    }

    #[test]
    fn 最近のワークスペースは新しい順で最大10件() {
        let (_dir, conn) = temp_conn();
        for i in 0..12 {
            let ws = open_workspace(&conn, &format!(r"C:\ws{i}"), &format!("ws{i}")).unwrap();
            conn.execute(
                "UPDATE workspaces SET last_opened_at = ?1 WHERE id = ?2",
                (format!("2026-07-04T00:00:{i:02}Z"), ws.id),
            )
            .unwrap();
        }
        let recent = list_recent_workspaces(&conn).unwrap();
        assert_eq!(recent.len(), RECENT_WORKSPACE_LIMIT);
        assert_eq!(recent[0].name, "ws11");
        assert_eq!(recent[9].name, "ws2");
    }

    #[test]
    fn 存在しないidはworkspace_not_found() {
        let (_dir, conn) = temp_conn();
        let err = get_workspace(&conn, "missing").unwrap_err();
        assert_eq!(err.code, "WORKSPACE_NOT_FOUND");
    }
}
