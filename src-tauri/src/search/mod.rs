use std::path::Path;

use rusqlite::Connection;

use crate::database::events::EventRecord;
use crate::database::meetings::{self, Meeting, TranscriptSegment};
use crate::database::search::{self, SearchDocInput};
use crate::database::tasks::{self, Task, TaskFilter};
use crate::error::AppError;
use crate::workspace::filetype::{self, FileCategory};
use crate::workspace::ops;

pub const TYPE_FILE: &str = "file";
pub const TYPE_MEETING: &str = "meeting";
pub const TYPE_TASK: &str = "task";
pub const TYPE_EVENT: &str = "event";

fn join_nonempty(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn task_doc(task: &Task) -> SearchDocInput {
    SearchDocInput {
        entity_type: TYPE_TASK.to_string(),
        entity_id: task.id.clone(),
        title: task.title.clone(),
        body: join_nonempty(&[
            task.description.as_deref().unwrap_or(""),
            task.assignee.as_deref().unwrap_or(""),
            task.project_name.as_deref().unwrap_or(""),
        ]),
        path: task.linked_file_path.clone(),
    }
}

pub fn event_doc(event: &EventRecord) -> SearchDocInput {
    SearchDocInput {
        entity_type: TYPE_EVENT.to_string(),
        entity_id: event.id.clone(),
        title: event.title.clone(),
        body: join_nonempty(&[
            event.description.as_deref().unwrap_or(""),
            event.location.as_deref().unwrap_or(""),
        ]),
        path: None,
    }
}

pub fn meeting_doc(meeting: &Meeting, segments: &[TranscriptSegment]) -> SearchDocInput {
    let transcript = segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    SearchDocInput {
        entity_type: TYPE_MEETING.to_string(),
        entity_id: meeting.id.clone(),
        title: meeting.title.clone(),
        body: join_nonempty(&[meeting.summary.as_deref().unwrap_or(""), &transcript]),
        path: Some(meeting.target_file_path.clone()),
    }
}

pub fn file_doc(abs_path: &str, name: &str, content: &str) -> SearchDocInput {
    SearchDocInput {
        entity_type: TYPE_FILE.to_string(),
        entity_id: abs_path.to_string(),
        title: name.to_string(),
        body: content.to_string(),
        path: Some(abs_path.to_string()),
    }
}

pub fn index_task(conn: &Connection, task: &Task) -> Result<(), AppError> {
    search::upsert_document(conn, &task_doc(task))
}

pub fn index_event(conn: &Connection, event: &EventRecord) -> Result<(), AppError> {
    search::upsert_document(conn, &event_doc(event))
}

pub fn index_meeting(
    conn: &Connection,
    meeting: &Meeting,
    segments: &[TranscriptSegment],
) -> Result<(), AppError> {
    search::upsert_document(conn, &meeting_doc(meeting, segments))
}

/// テキストファイルを索引へ登録する。Editカテゴリ以外は索引しない（trueで登録実施）。
pub fn index_file(
    conn: &Connection,
    abs_path: &str,
    name: &str,
    content: &str,
) -> Result<bool, AppError> {
    let is_text = Path::new(name)
        .extension()
        .map(|e| filetype::category_for_extension(&e.to_string_lossy()) == FileCategory::Edit)
        .unwrap_or(false);
    if !is_text {
        return Ok(false);
    }
    search::upsert_document(conn, &file_doc(abs_path, name, content))?;
    Ok(true)
}

pub fn remove_entity(conn: &Connection, entity_type: &str, entity_id: &str) -> Result<(), AppError> {
    search::delete_document(conn, entity_type, entity_id)
}

/// 単一パスの索引を最新化する。テキストファイルなら登録、消えていれば索引から除去する。
/// ウォッチャからの外部変更追従に使う（軽量・高速）。
pub fn sync_path(conn: &Connection, abs_path: &str) -> Result<(), AppError> {
    let path = Path::new(abs_path);
    if path.is_file() {
        if should_index_file(path) {
            if let Ok(file) = ops::read_text_file(path) {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                search::upsert_document(conn, &file_doc(abs_path, &name, &file.content))?;
            }
        }
    } else {
        search::delete_document(conn, TYPE_FILE, abs_path)?;
    }
    Ok(())
}

/// 索引全体をクリアし、DB由来（タスク/予定/会議）を1トランザクションで再登録する（§15.3）。
/// ファイルは別途 write_file_docs でバッチ登録する（省メモリのため一度に全読みしない）。
pub fn reindex_entities(conn: &mut Connection) -> Result<usize, AppError> {
    let mut docs: Vec<SearchDocInput> = Vec::new();
    for task in tasks::list_tasks(conn, &TaskFilter::default(), chrono::Utc::now())? {
        docs.push(task_doc(&task));
    }
    for event in all_events(conn)? {
        docs.push(event_doc(&event));
    }
    for meeting in meetings::list_meetings(conn, 10_000)? {
        let segments = meetings::list_segments(conn, &meeting.id)?;
        docs.push(meeting_doc(&meeting, &segments));
    }
    let tx = conn.transaction()?;
    for entity_type in [TYPE_FILE, TYPE_MEETING, TYPE_TASK, TYPE_EVENT] {
        search::delete_by_type(&tx, entity_type)?;
    }
    for doc in &docs {
        search::upsert_document(&tx, doc)?;
    }
    tx.commit()?;
    Ok(docs.len())
}

/// ファイル索引ドキュメントのバッチを1トランザクションで登録する。
pub fn write_file_docs(conn: &mut Connection, docs: &[SearchDocInput]) -> Result<(), AppError> {
    if docs.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for doc in docs {
        search::upsert_document(&tx, doc)?;
    }
    tx.commit()?;
    Ok(())
}

const IGNORED_DIRS: &[&str] = &["node_modules", ".git", "target", "dist", ".venv", "__pycache__"];
const MAX_DEPTH: usize = 12;
const MAX_INDEX_BYTES: u64 = 2 * 1024 * 1024;
/// 一度に読み込む索引対象ファイル数。省メモリのためバッチ単位で処理する。
pub const INDEX_BATCH: usize = 128;

fn all_events(conn: &Connection) -> Result<Vec<EventRecord>, AppError> {
    use crate::database::events;
    // 全期間を対象にするため十分に広い範囲で取得する。
    events::list_events_in_range(conn, "0000-01-01T00:00:00Z", "9999-12-31T23:59:59Z")
}

/// DBロックを取らずにworkspace_root配下の索引対象ファイルのパスだけを収集する（内容は読まない）。
pub fn collect_workspace_file_paths(root: &Path) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    collect_paths_under(root, 0, &mut paths);
    paths
}

fn collect_paths_under(dir: &Path, depth: usize, out: &mut Vec<std::path::PathBuf>) {
    if depth > MAX_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || IGNORED_DIRS.contains(&name.as_str()) {
            continue;
        }
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            collect_paths_under(&path, depth + 1, out);
        } else if file_type.is_file() && should_index_file(&path) {
            out.push(path);
        }
    }
}

/// DBロックを取らずに、指定パス群のテキストを読み索引ドキュメントへ変換する（バッチ単位で呼ぶ）。
pub fn read_file_docs(paths: &[std::path::PathBuf]) -> Vec<SearchDocInput> {
    let mut docs = Vec::with_capacity(paths.len());
    for path in paths {
        if let Ok(file) = ops::read_text_file(path) {
            let abs = path.to_string_lossy().into_owned();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            docs.push(file_doc(&abs, &name, &file.content));
        }
    }
    docs
}

fn should_index_file(path: &Path) -> bool {
    let category = path
        .extension()
        .map(|e| filetype::category_for_extension(&e.to_string_lossy()))
        .unwrap_or(FileCategory::Unknown);
    if category != FileCategory::Edit {
        return false;
    }
    std::fs::metadata(path)
        .map(|m| m.len() <= MAX_INDEX_BYTES)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::meetings::{create_meeting, MeetingInput, MeetingStatus};
    use crate::database::open_database;
    use crate::database::search::search;
    use crate::database::tasks::{create_task, TaskInput, TaskPriority, TaskStatus};

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    fn sample_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            title: "設計レビュー".to_string(),
            description: Some("認証まわりの設計を確認する".to_string()),
            due_at: None,
            timezone: "Asia/Tokyo".to_string(),
            priority: TaskPriority::Medium,
            status: TaskStatus::Todo,
            assignee: Some("田中".to_string()),
            project_name: Some("基盤刷新".to_string()),
            meeting_id: None,
            linked_file_path: Some("C:/notes/task.md".to_string()),
            created_at: "2026-07-19T00:00:00Z".to_string(),
            updated_at: "2026-07-19T00:00:00Z".to_string(),
            completed_at: None,
        }
    }

    #[test]
    fn タスクdocは説明と担当と案件を本文に含む() {
        let doc = task_doc(&sample_task("t1"));
        assert_eq!(doc.entity_type, "task");
        assert_eq!(doc.title, "設計レビュー");
        assert!(doc.body.contains("認証まわり"));
        assert!(doc.body.contains("田中"));
        assert!(doc.body.contains("基盤刷新"));
    }

    #[test]
    fn 会議docは要約と文字起こしを本文に含む() {
        let meeting = Meeting {
            id: "m1".to_string(),
            workspace_id: None,
            title: "定例".to_string(),
            started_at: "2026-07-19T00:00:00Z".to_string(),
            ended_at: None,
            timezone: "Asia/Tokyo".to_string(),
            target_file_path: "C:/notes/m.md".to_string(),
            start_marker: "s".to_string(),
            end_marker: "e".to_string(),
            summary: Some("要約テキスト".to_string()),
            status: MeetingStatus::Completed,
            created_at: "t".to_string(),
            updated_at: "t".to_string(),
        };
        let seg = TranscriptSegment {
            id: "s1".to_string(),
            meeting_id: "m1".to_string(),
            source: "mic".to_string(),
            speaker_label: "自分".to_string(),
            start_ms: 0,
            end_ms: 1000,
            text: "重要な発言内容".to_string(),
            status: "confirmed".to_string(),
            audio_chunk_path: None,
            created_at: "t".to_string(),
        };
        let doc = meeting_doc(&meeting, &[seg]);
        assert!(doc.body.contains("要約テキスト"));
        assert!(doc.body.contains("重要な発言内容"));
    }

    #[test]
    fn 再インデックスでタスクと会議を検索できる() {
        let (_dir, mut conn) = temp_conn();
        create_task(
            &conn,
            &TaskInput {
                title: "在庫確認".to_string(),
                description: Some("倉庫の在庫を数える".to_string()),
                due_at_utc: None,
                timezone: "Asia/Tokyo".to_string(),
                priority: TaskPriority::Medium,
                status: None,
                assignee: None,
                project_name: None,
                meeting_id: None,
                linked_file_path: None,
            },
        )
        .unwrap();
        create_meeting(
            &conn,
            MeetingInput {
                title: "棚卸し会議".to_string(),
                workspace_id: None,
                target_file_path: "C:/notes/inv.md".to_string(),
                timezone: "Asia/Tokyo".to_string(),
            },
        )
        .unwrap();
        let count = reindex_entities(&mut conn).unwrap();
        assert!(count >= 2, "少なくともタスクと会議が索引される: {count}");
        assert_eq!(search(&conn, "在庫を数える", &[], 20, 0).unwrap().len(), 1);
        assert_eq!(search(&conn, "棚卸し会議", &[], 20, 0).unwrap()[0].entity_type, "meeting");
    }

    #[test]
    fn ファイルパス収集は無視ディレクトリと非テキストを除外する() {
        let ws = tempfile::tempdir().unwrap();
        std::fs::write(ws.path().join("メモ.md"), "# 見出し\n横断的な検索対象の本文\n").unwrap();
        std::fs::write(ws.path().join("画像.png"), [0u8, 1, 2, 3]).unwrap();
        std::fs::create_dir(ws.path().join("node_modules")).unwrap();
        std::fs::write(ws.path().join("node_modules").join("ignored.md"), "無視される本文").unwrap();
        let paths = collect_workspace_file_paths(ws.path());
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("メモ.md"));
    }

    #[test]
    fn バッチ読込と登録でファイルを検索できる() {
        let (_dir, mut conn) = temp_conn();
        reindex_entities(&mut conn).unwrap();
        let ws = tempfile::tempdir().unwrap();
        std::fs::write(ws.path().join("メモ.md"), "横断的な検索対象の本文\n").unwrap();
        let paths = collect_workspace_file_paths(ws.path());
        let docs = read_file_docs(&paths);
        write_file_docs(&mut conn, &docs).unwrap();
        assert_eq!(search(&conn, "横断的な検索対象", &[], 20, 0).unwrap().len(), 1);
    }

    #[test]
    fn sync_pathは追加で出現し削除で消える() {
        let (_dir, conn) = temp_conn();
        let ws = tempfile::tempdir().unwrap();
        let file = ws.path().join("追記メモ.md");
        std::fs::write(&file, "後から追加した検索本文\n").unwrap();
        let abs = file.to_string_lossy().into_owned();
        sync_path(&conn, &abs).unwrap();
        assert_eq!(search(&conn, "後から追加した検索本文", &[], 20, 0).unwrap().len(), 1);
        std::fs::remove_file(&file).unwrap();
        sync_path(&conn, &abs).unwrap();
        assert!(search(&conn, "後から追加した検索本文", &[], 20, 0).unwrap().is_empty());
    }
}
