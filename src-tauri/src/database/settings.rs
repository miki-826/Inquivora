use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

use crate::error::AppError;

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<Value>, AppError> {
    let json: Option<String> = conn
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    match json {
        Some(raw) => {
            let value = serde_json::from_str(&raw).map_err(|e| {
                AppError::database(format!("設定値のJSONを解釈できません ({key}): {e}"))
            })?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &Value) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO app_settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        (key, value.to_string(), chrono::Utc::now().to_rfc3339()),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;
    use serde_json::json;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    #[test]
    fn 未設定のキーはnoneを返す() {
        let (_dir, conn) = temp_conn();
        assert_eq!(get_setting(&conn, "ui.layout").unwrap(), None);
    }

    #[test]
    fn 設定値を保存して取得できる() {
        let (_dir, conn) = temp_conn();
        let value = json!({ "leftSidebarWidth": 320, "lastScreen": "/tasks" });
        set_setting(&conn, "ui.layout", &value).unwrap();
        assert_eq!(get_setting(&conn, "ui.layout").unwrap(), Some(value));
    }

    #[test]
    fn 同じキーへの保存は上書きされる() {
        let (_dir, conn) = temp_conn();
        set_setting(&conn, "ui.layout", &json!({ "v": 1 })).unwrap();
        set_setting(&conn, "ui.layout", &json!({ "v": 2 })).unwrap();
        assert_eq!(get_setting(&conn, "ui.layout").unwrap(), Some(json!({ "v": 2 })));
    }
}
