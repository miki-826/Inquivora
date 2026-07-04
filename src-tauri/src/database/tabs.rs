use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 開いているタブの永続化（§18・recent_tabs）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecentTab {
    pub path: String,
    pub is_pinned: bool,
    pub cursor_line: i64,
    pub cursor_column: i64,
}

/// ワークスペースのタブ一覧を丸ごと置き換えて保存する。
pub fn save_tabs(_conn: &Connection, _workspace_id: &str, _tabs: &[RecentTab]) -> Result<(), AppError> {
    todo!()
}

/// タブ一覧を保存時の順序で返す。
pub fn load_tabs(_conn: &Connection, _workspace_id: &str) -> Result<Vec<RecentTab>, AppError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;
    use crate::database::workspaces::open_workspace;

    fn setup() -> (tempfile::TempDir, Connection, String) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        let ws = open_workspace(&conn, r"C:\ws", "ws").unwrap();
        (dir, conn, ws.id)
    }

    fn tab(path: &str, line: i64) -> RecentTab {
        RecentTab {
            path: path.to_string(),
            is_pinned: false,
            cursor_line: line,
            cursor_column: 1,
        }
    }

    #[test]
    fn タブを保存して順序どおり読み込める() {
        let (_dir, conn, ws_id) = setup();
        let tabs = vec![tab(r"C:\ws\b.md", 10), tab(r"C:\ws\a.md", 2)];
        save_tabs(&conn, &ws_id, &tabs).unwrap();
        assert_eq!(load_tabs(&conn, &ws_id).unwrap(), tabs);
    }

    #[test]
    fn 再保存で置き換えられる() {
        let (_dir, conn, ws_id) = setup();
        save_tabs(&conn, &ws_id, &[tab(r"C:\ws\old.md", 1)]).unwrap();
        save_tabs(&conn, &ws_id, &[tab(r"C:\ws\new.md", 5)]).unwrap();
        let loaded = load_tabs(&conn, &ws_id).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].path, r"C:\ws\new.md");
    }

    #[test]
    fn 未保存のワークスペースは空を返す() {
        let (_dir, conn, _ws_id) = setup();
        let other = open_workspace(&conn, r"C:\other", "other").unwrap();
        assert!(load_tabs(&conn, &other.id).unwrap().is_empty());
    }
}
