use chrono::Utc;
use rusqlite::Connection;
use serde::Serialize;

use crate::error::AppError;

/// 検索インデックスへ登録する1件のドキュメント。
pub struct SearchDocInput {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub body: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub snippet: String,
    pub path: Option<String>,
}

/// search_documents と FTS を同期して1件を登録・更新する（§16 アプリ側同期）。
pub fn upsert_document(conn: &Connection, doc: &SearchDocInput) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM search_documents_fts WHERE entity_type = ?1 AND entity_id = ?2",
        rusqlite::params![doc.entity_type, doc.entity_id],
    )?;
    conn.execute(
        "INSERT INTO search_documents (entity_type, entity_id, title, body, path, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(entity_type, entity_id) DO UPDATE SET
            title = excluded.title, body = excluded.body,
            path = excluded.path, updated_at = excluded.updated_at",
        rusqlite::params![
            doc.entity_type,
            doc.entity_id,
            doc.title,
            doc.body,
            doc.path,
            Utc::now().to_rfc3339(),
        ],
    )?;
    conn.execute(
        "INSERT INTO search_documents_fts (title, body, path, entity_type, entity_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![doc.title, doc.body, doc.path, doc.entity_type, doc.entity_id],
    )?;
    Ok(())
}

/// 指定エンティティを索引から削除する。
pub fn delete_document(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM search_documents_fts WHERE entity_type = ?1 AND entity_id = ?2",
        rusqlite::params![entity_type, entity_id],
    )?;
    conn.execute(
        "DELETE FROM search_documents WHERE entity_type = ?1 AND entity_id = ?2",
        rusqlite::params![entity_type, entity_id],
    )?;
    Ok(())
}

/// 指定種別の索引をまとめて削除する（再インデックス前のクリア用）。
pub fn delete_by_type(conn: &Connection, entity_type: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM search_documents_fts WHERE entity_type = ?1",
        [entity_type],
    )?;
    conn.execute(
        "DELETE FROM search_documents WHERE entity_type = ?1",
        [entity_type],
    )?;
    Ok(())
}

fn type_filter_clause(entity_types: &[String], column: &str) -> (String, Vec<String>) {
    if entity_types.is_empty() {
        return (String::new(), Vec::new());
    }
    let placeholders = entity_types
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    (
        format!(" AND {column} IN ({placeholders})"),
        entity_types.to_vec(),
    )
}

/// FTS5のMATCH式へ渡すためクエリをフレーズとしてエスケープする。
fn to_match_phrase(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

/// §15 全文検索。3文字以上はFTS5(trigram)、2文字以下はLIKEフォールバック。
pub fn search(
    conn: &Connection,
    query: &str,
    entity_types: &[String],
    limit: i64,
    offset: i64,
) -> Result<Vec<SearchResult>, AppError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.chars().count() <= 2 {
        return search_like(conn, trimmed, entity_types, limit, offset);
    }
    let (filter, filter_params) = type_filter_clause(entity_types, "entity_type");
    let sql = format!(
        "SELECT entity_type, entity_id, title,
                snippet(search_documents_fts, 1, '【', '】', '…', 12) AS snip, path
         FROM search_documents_fts
         WHERE search_documents_fts MATCH ?1{filter}
         ORDER BY rank
         LIMIT ?{limit_idx} OFFSET ?{offset_idx}",
        limit_idx = filter_params.len() + 2,
        offset_idx = filter_params.len() + 3,
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(to_match_phrase(trimmed))];
    for t in &filter_params {
        params.push(Box::new(t.clone()));
    }
    params.push(Box::new(limit));
    params.push(Box::new(offset));
    run_query(conn, &sql, params)
}

fn search_like(
    conn: &Connection,
    query: &str,
    entity_types: &[String],
    limit: i64,
    offset: i64,
) -> Result<Vec<SearchResult>, AppError> {
    let (filter, filter_params) = type_filter_clause(entity_types, "entity_type");
    let sql = format!(
        "SELECT entity_type, entity_id, title, substr(body, 1, 120) AS snip, path
         FROM search_documents
         WHERE (title LIKE ?1 OR body LIKE ?1){filter}
         ORDER BY updated_at DESC
         LIMIT ?{limit_idx} OFFSET ?{offset_idx}",
        limit_idx = filter_params.len() + 2,
        offset_idx = filter_params.len() + 3,
    );
    let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(like)];
    for t in &filter_params {
        params.push(Box::new(t.clone()));
    }
    params.push(Box::new(limit));
    params.push(Box::new(offset));
    run_query(conn, &sql, params)
}

fn run_query(
    conn: &Connection,
    sql: &str,
    params: Vec<Box<dyn rusqlite::ToSql>>,
) -> Result<Vec<SearchResult>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(SearchResult {
            entity_type: row.get(0)?,
            entity_id: row.get(1)?,
            title: row.get(2)?,
            snippet: row.get(3)?,
            path: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
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

    fn doc(entity_type: &str, id: &str, title: &str, body: &str) -> SearchDocInput {
        SearchDocInput {
            entity_type: entity_type.to_string(),
            entity_id: id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            path: Some(format!("C:/notes/{id}.md")),
        }
    }

    #[test]
    fn 日本語の部分一致で検索できる() {
        let (_dir, conn) = temp_conn();
        upsert_document(&conn, &doc("file", "1", "会議メモ", "議事録の要約をまとめた")).unwrap();
        let results = search(&conn, "議事録", &[], 20, 0).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, "1");
        assert!(results[0].snippet.contains("議事録"));
    }

    #[test]
    fn 更新すると内容が置き換わり重複しない() {
        let (_dir, conn) = temp_conn();
        upsert_document(&conn, &doc("file", "1", "旧タイトル", "古い本文の内容")).unwrap();
        upsert_document(&conn, &doc("file", "1", "新タイトル", "新しい本文の内容")).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM search_documents", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert!(search(&conn, "古い本文", &[], 20, 0).unwrap().is_empty());
        assert_eq!(search(&conn, "新しい本文", &[], 20, 0).unwrap().len(), 1);
    }

    #[test]
    fn 削除すると検索に出ない() {
        let (_dir, conn) = temp_conn();
        upsert_document(&conn, &doc("task", "t1", "買い物", "牛乳を買う予定")).unwrap();
        delete_document(&conn, "task", "t1").unwrap();
        assert!(search(&conn, "牛乳", &[], 20, 0).unwrap().is_empty());
    }

    #[test]
    fn 種別で絞り込める() {
        let (_dir, conn) = temp_conn();
        upsert_document(&conn, &doc("file", "1", "計画資料", "来週の計画をまとめる")).unwrap();
        upsert_document(&conn, &doc("task", "t1", "計画作成", "来週の計画を作成する")).unwrap();
        let only_tasks = search(&conn, "来週の計画", &["task".to_string()], 20, 0).unwrap();
        assert_eq!(only_tasks.len(), 1);
        assert_eq!(only_tasks[0].entity_type, "task");
    }

    #[test]
    fn 二文字以下はlikeフォールバックで検索する() {
        let (_dir, conn) = temp_conn();
        upsert_document(&conn, &doc("file", "1", "AB", "短い語ABを含む本文")).unwrap();
        let results = search(&conn, "AB", &[], 20, 0).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn 空クエリは結果なし() {
        let (_dir, conn) = temp_conn();
        upsert_document(&conn, &doc("file", "1", "資料", "本文")).unwrap();
        assert!(search(&conn, "   ", &[], 20, 0).unwrap().is_empty());
    }

    #[test]
    fn 種別ごとに索引をクリアできる() {
        let (_dir, conn) = temp_conn();
        upsert_document(&conn, &doc("file", "1", "資料一", "検索対象の本文")).unwrap();
        upsert_document(&conn, &doc("file", "2", "資料二", "検索対象の本文")).unwrap();
        upsert_document(&conn, &doc("task", "t1", "作業", "検索対象の本文")).unwrap();
        delete_by_type(&conn, "file").unwrap();
        let results = search(&conn, "検索対象", &[], 20, 0).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "task");
    }
}
