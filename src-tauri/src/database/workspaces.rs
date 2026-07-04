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

/// ワークスペースを開いた記録を保存する。同一root_pathは最終使用日時のみ更新する。
pub fn open_workspace(_conn: &Connection, _root_path: &str, _name: &str) -> Result<WorkspaceRecord, AppError> {
    todo!()
}

/// 最近開いたワークスペースを新しい順に最大10件返す（§7.1）。
pub fn list_recent_workspaces(_conn: &Connection) -> Result<Vec<WorkspaceRecord>, AppError> {
    todo!()
}

pub fn get_workspace(_conn: &Connection, _id: &str) -> Result<WorkspaceRecord, AppError> {
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
