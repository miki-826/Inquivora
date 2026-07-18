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
