use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::meeting::markdown;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeetingStatus {
    Recording,
    Paused,
    Completed,
}

impl MeetingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MeetingStatus::Recording => "recording",
            MeetingStatus::Paused => "paused",
            MeetingStatus::Completed => "completed",
        }
    }

    fn from_db(value: &str) -> Self {
        match value {
            "paused" => MeetingStatus::Paused,
            "completed" => MeetingStatus::Completed,
            _ => MeetingStatus::Recording,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
    pub workspace_id: Option<String>,
    pub title: String,
    #[serde(rename = "startedAtUtc")]
    pub started_at: String,
    #[serde(rename = "endedAtUtc")]
    pub ended_at: Option<String>,
    pub timezone: String,
    pub target_file_path: String,
    pub start_marker: String,
    pub end_marker: String,
    pub summary: Option<String>,
    pub status: MeetingStatus,
    pub created_at: String,
    pub updated_at: String,
}

fn default_timezone() -> String {
    "Asia/Tokyo".to_string()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingInput {
    pub title: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    pub target_file_path: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn row_to_meeting(row: &Row) -> rusqlite::Result<Meeting> {
    let status: String = row.get("status")?;
    Ok(Meeting {
        id: row.get("id")?,
        workspace_id: row.get("workspace_id")?,
        title: row.get("title")?,
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
        timezone: row.get("timezone")?,
        target_file_path: row.get("target_file_path")?,
        start_marker: row.get("start_marker")?,
        end_marker: row.get("end_marker")?,
        summary: row.get("summary")?,
        status: MeetingStatus::from_db(&status),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

const MEETING_COLUMNS: &str = "id, workspace_id, title, started_at, ended_at, timezone, \
     target_file_path, start_marker, end_marker, summary, status, created_at, updated_at";

pub fn create_meeting(conn: &Connection, input: MeetingInput) -> Result<Meeting, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "会議タイトルを入力してください",
            false,
        ));
    }
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO meetings
            (id, workspace_id, title, started_at, timezone, target_file_path,
             start_marker, end_marker, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'recording', ?4, ?4)",
        rusqlite::params![
            id,
            input.workspace_id,
            input.title.trim(),
            now,
            input.timezone,
            input.target_file_path,
            markdown::start_marker(&id),
            markdown::end_marker(&id),
        ],
    )?;
    get_meeting(conn, &id)
}

pub fn get_meeting(conn: &Connection, id: &str) -> Result<Meeting, AppError> {
    conn.query_row(
        &format!("SELECT {MEETING_COLUMNS} FROM meetings WHERE id = ?1"),
        [id],
        row_to_meeting,
    )
    .optional()?
    .ok_or_else(|| AppError::new("MEETING_NOT_FOUND", "会議が見つかりません", false))
}

pub fn list_meetings(conn: &Connection, limit: i64) -> Result<Vec<Meeting>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {MEETING_COLUMNS} FROM meetings ORDER BY started_at DESC LIMIT ?1"
    ))?;
    let rows = stmt.query_map([limit], row_to_meeting)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn rename_meeting(conn: &Connection, id: &str, title: &str) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        return Ok(());
    }
    let affected = conn.execute(
        "UPDATE meetings SET title = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, title, Utc::now().to_rfc3339()],
    )?;
    if affected == 0 {
        return Err(AppError::new("MEETING_NOT_FOUND", "会議が見つかりません", false));
    }
    Ok(())
}

pub fn set_meeting_status(
    conn: &Connection,
    id: &str,
    status: MeetingStatus,
) -> Result<(), AppError> {
    let affected = conn.execute(
        "UPDATE meetings SET status = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, status.as_str(), Utc::now().to_rfc3339()],
    )?;
    if affected == 0 {
        return Err(AppError::new("MEETING_NOT_FOUND", "会議が見つかりません", false));
    }
    Ok(())
}

pub fn end_meeting(conn: &Connection, id: &str) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE meetings SET status = 'completed', ended_at = ?2, updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now],
    )?;
    if affected == 0 {
        return Err(AppError::new("MEETING_NOT_FOUND", "会議が見つかりません", false));
    }
    Ok(())
}

/// 新しいアプリプロセスには前回の録音セッションを引き継げないため、
/// DBに残った録音中・一時停止中の会議を終了済みにする。
pub fn complete_interrupted_meetings(conn: &Connection) -> Result<usize, AppError> {
    let now = Utc::now().to_rfc3339();
    Ok(conn.execute(
        "UPDATE meetings
         SET status = 'completed', ended_at = ?1, updated_at = ?1
         WHERE status IN ('recording', 'paused')",
        [&now],
    )?)
}

pub fn delete_meeting(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn.execute("DELETE FROM meetings WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(AppError::new("MEETING_NOT_FOUND", "会議が見つかりません", false));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub meeting_id: String,
    pub source: String,
    pub speaker_label: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub status: String,
    pub audio_chunk_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentInput {
    pub meeting_id: String,
    pub source: String,
    pub speaker_label: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    #[serde(default)]
    pub audio_chunk_path: Option<String>,
}

fn row_to_segment(row: &Row) -> rusqlite::Result<TranscriptSegment> {
    Ok(TranscriptSegment {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        source: row.get(2)?,
        speaker_label: row.get(3)?,
        start_ms: row.get(4)?,
        end_ms: row.get(5)?,
        text: row.get(6)?,
        status: row.get(7)?,
        audio_chunk_path: row.get(8)?,
        created_at: row.get(9)?,
    })
}

const SEGMENT_COLUMNS: &str = "id, meeting_id, source, speaker_label, start_ms, end_ms, text, \
     status, audio_chunk_path, created_at";

pub fn insert_segment(conn: &Connection, input: SegmentInput) -> Result<TranscriptSegment, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO transcript_segments
            (id, meeting_id, source, speaker_label, start_ms, end_ms, text, status,
             audio_chunk_path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'confirmed', ?8, ?9)",
        rusqlite::params![
            id,
            input.meeting_id,
            input.source,
            input.speaker_label,
            input.start_ms,
            input.end_ms,
            input.text,
            input.audio_chunk_path,
            now,
        ],
    )?;
    conn.query_row(
        &format!("SELECT {SEGMENT_COLUMNS} FROM transcript_segments WHERE id = ?1"),
        [&id],
        row_to_segment,
    )
    .map_err(Into::into)
}

pub fn list_segments(conn: &Connection, meeting_id: &str) -> Result<Vec<TranscriptSegment>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SEGMENT_COLUMNS} FROM transcript_segments
         WHERE meeting_id = ?1 ORDER BY start_ms, created_at"
    ))?;
    let rows = stmt.query_map([meeting_id], row_to_segment)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn last_segment_text(
    conn: &Connection,
    meeting_id: &str,
    source: &str,
) -> Result<Option<String>, AppError> {
    Ok(conn
        .query_row(
            "SELECT text FROM transcript_segments
             WHERE meeting_id = ?1 AND source = ?2
             ORDER BY start_ms DESC, created_at DESC LIMIT 1",
            [meeting_id, source],
            |row| row.get(0),
        )
        .optional()?)
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

    fn sample_input(title: &str) -> MeetingInput {
        MeetingInput {
            title: title.to_string(),
            workspace_id: None,
            target_file_path: "C:/notes/meeting.md".to_string(),
            timezone: "Asia/Tokyo".to_string(),
        }
    }

    #[test]
    fn 会議を作成するとマーカーとrecording状態が設定される() {
        let (_dir, conn) = open_temp_db();
        let meeting = create_meeting(&conn, sample_input("定例会議")).unwrap();
        assert_eq!(meeting.status, MeetingStatus::Recording);
        assert_eq!(
            meeting.start_marker,
            format!("<!-- inquivora:meeting:{}:start -->", meeting.id)
        );
        assert_eq!(
            meeting.end_marker,
            format!("<!-- inquivora:meeting:{}:end -->", meeting.id)
        );
        assert!(meeting.ended_at.is_none());
    }

    #[test]
    fn 会議を取得できないときはnot_foundになる() {
        let (_dir, conn) = open_temp_db();
        let err = get_meeting(&conn, "missing").unwrap_err();
        assert_eq!(err.code, "MEETING_NOT_FOUND");
    }

    #[test]
    fn 会議一覧は開始日時の新しい順で返る() {
        let (_dir, conn) = open_temp_db();
        let first = create_meeting(&conn, sample_input("先の会議")).unwrap();
        conn.execute(
            "UPDATE meetings SET started_at = '2026-07-01T00:00:00Z' WHERE id = ?1",
            [&first.id],
        )
        .unwrap();
        create_meeting(&conn, sample_input("後の会議")).unwrap();
        let list = list_meetings(&conn, 50).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].title, "後の会議");
    }

    #[test]
    fn 会議の状態を遷移できる() {
        let (_dir, conn) = open_temp_db();
        let meeting = create_meeting(&conn, sample_input("会議")).unwrap();
        set_meeting_status(&conn, &meeting.id, MeetingStatus::Paused).unwrap();
        assert_eq!(
            get_meeting(&conn, &meeting.id).unwrap().status,
            MeetingStatus::Paused
        );
        set_meeting_status(&conn, &meeting.id, MeetingStatus::Recording).unwrap();
        assert_eq!(
            get_meeting(&conn, &meeting.id).unwrap().status,
            MeetingStatus::Recording
        );
    }

    #[test]
    fn 会議を終了するとended_atとcompletedが設定される() {
        let (_dir, conn) = open_temp_db();
        let meeting = create_meeting(&conn, sample_input("会議")).unwrap();
        end_meeting(&conn, &meeting.id).unwrap();
        let fetched = get_meeting(&conn, &meeting.id).unwrap();
        assert_eq!(fetched.status, MeetingStatus::Completed);
        assert!(fetched.ended_at.is_some());
    }

    #[test]
    fn 再起動時は中断された会議を終了済みにする() {
        let (_dir, conn) = open_temp_db();
        let recording = create_meeting(&conn, sample_input("録音中")).unwrap();
        let paused = create_meeting(&conn, sample_input("一時停止中")).unwrap();
        set_meeting_status(&conn, &paused.id, MeetingStatus::Paused).unwrap();
        let completed = create_meeting(&conn, sample_input("終了済み")).unwrap();
        end_meeting(&conn, &completed.id).unwrap();

        assert_eq!(complete_interrupted_meetings(&conn).unwrap(), 2);
        for id in [&recording.id, &paused.id, &completed.id] {
            let meeting = get_meeting(&conn, id).unwrap();
            assert_eq!(meeting.status, MeetingStatus::Completed);
            assert!(meeting.ended_at.is_some());
        }
    }

    #[test]
    fn セグメントを追加して時系列で取得できる() {
        let (_dir, conn) = open_temp_db();
        let meeting = create_meeting(&conn, sample_input("会議")).unwrap();
        insert_segment(
            &conn,
            SegmentInput {
                meeting_id: meeting.id.clone(),
                source: "loopback".to_string(),
                speaker_label: "PC音声".to_string(),
                start_ms: 20000,
                end_ms: 40000,
                text: "後の発言".to_string(),
                audio_chunk_path: None,
            },
        )
        .unwrap();
        insert_segment(
            &conn,
            SegmentInput {
                meeting_id: meeting.id.clone(),
                source: "mic".to_string(),
                speaker_label: "自分".to_string(),
                start_ms: 0,
                end_ms: 20000,
                text: "先の発言".to_string(),
                audio_chunk_path: Some("C:/audio/chunk0.wav".to_string()),
            },
        )
        .unwrap();
        let segments = list_segments(&conn, &meeting.id).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "先の発言");
        assert_eq!(segments[1].speaker_label, "PC音声");
    }

    #[test]
    fn 音源ごとの最終セグメント本文を取得できる() {
        let (_dir, conn) = open_temp_db();
        let meeting = create_meeting(&conn, sample_input("会議")).unwrap();
        for (start, text) in [(0, "最初"), (19000, "次の発言")] {
            insert_segment(
                &conn,
                SegmentInput {
                    meeting_id: meeting.id.clone(),
                    source: "mic".to_string(),
                    speaker_label: "自分".to_string(),
                    start_ms: start,
                    end_ms: start + 20000,
                    text: text.to_string(),
                    audio_chunk_path: None,
                },
            )
            .unwrap();
        }
        let last = last_segment_text(&conn, &meeting.id, "mic").unwrap();
        assert_eq!(last.as_deref(), Some("次の発言"));
        assert!(last_segment_text(&conn, &meeting.id, "loopback")
            .unwrap()
            .is_none());
    }

    #[test]
    fn 会議を削除するとセグメントも削除される() {
        let (_dir, conn) = open_temp_db();
        let meeting = create_meeting(&conn, sample_input("会議")).unwrap();
        insert_segment(
            &conn,
            SegmentInput {
                meeting_id: meeting.id.clone(),
                source: "mic".to_string(),
                speaker_label: "自分".to_string(),
                start_ms: 0,
                end_ms: 1000,
                text: "x".to_string(),
                audio_chunk_path: None,
            },
        )
        .unwrap();
        delete_meeting(&conn, &meeting.id).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transcript_segments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
