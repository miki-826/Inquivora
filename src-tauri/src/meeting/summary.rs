use crate::database::meetings::{Meeting, TranscriptSegment};
use crate::meeting::markdown;

pub const MEETING_SUMMARY_FEATURE: &str = "meeting.summary";

/// AIへ渡す発話ログを「[HH:MM|開始ms] 話者: 本文」形式へ整形する。
pub fn build_transcript_text(meeting: &Meeting, segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(|segment| {
            let time = markdown::segment_time_label(&meeting.started_at, segment.start_ms);
            format!(
                "[{time}|{ms}] {speaker}: {text}",
                ms = segment.start_ms,
                speaker = segment.speaker_label,
                text = segment.text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 議事録生成のシステムプロンプト。JSONのみを返すよう指示する（§10.10）。
pub fn system_prompt() -> String {
    "あなたは日本語の会議アシスタントです。与えられた文字起こしから議事録を作成します。\
     出力は必ず次のスキーマに一致する有効なJSONオブジェクトのみとし、前後に説明文やコードフェンスを付けないでください。\
     {\"title\": string, \"summary\": string, \"decisions\": [{\"text\": string, \"sourceStartMs\"?: number}], \
     \"taskCandidates\": [{\"title\": string, \"description\"?: string, \"assignee\"?: string, \"dueAt\"?: string, \
     \"priority\": \"high\"|\"medium\"|\"low\", \"sourceStartMs\"?: number}], \
     \"openQuestions\": [{\"text\": string, \"sourceStartMs\"?: number}]}。\
     sourceStartMsには根拠となる発話の開始ミリ秒（角括弧内の|の後ろの数値）を入れてください。\
     決定事項・タスク・未確認事項が無い場合は空配列にします。summaryは日本語で簡潔にまとめます。"
        .to_string()
}

/// 文字起こしとユーザーメモからユーザーメッセージ本文を組み立てる。
pub fn build_user_content(
    meeting: &Meeting,
    transcript_text: &str,
    user_notes: &str,
) -> String {
    let notes = user_notes.trim();
    let notes_block = if notes.is_empty() {
        String::new()
    } else {
        format!("\n\n# ユーザーメモ\n{notes}")
    };
    format!(
        "# 会議情報\nタイトル: {title}\nタイムゾーン: Asia/Tokyo\n\n# 文字起こし\n{transcript_text}{notes_block}",
        title = meeting.title
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::meetings::MeetingStatus;

    fn meeting() -> Meeting {
        Meeting {
            id: "m-1".to_string(),
            workspace_id: None,
            title: "定例会議".to_string(),
            started_at: "2026-07-17T01:00:00Z".to_string(),
            ended_at: Some("2026-07-17T02:00:00Z".to_string()),
            timezone: "Asia/Tokyo".to_string(),
            target_file_path: "C:/notes/m.md".to_string(),
            start_marker: "s".to_string(),
            end_marker: "e".to_string(),
            summary: None,
            status: MeetingStatus::Completed,
            created_at: "2026-07-17T01:00:00Z".to_string(),
            updated_at: "2026-07-17T01:00:00Z".to_string(),
        }
    }

    fn segment(start_ms: i64, speaker: &str, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: "seg".to_string(),
            meeting_id: "m-1".to_string(),
            source: "mic".to_string(),
            speaker_label: speaker.to_string(),
            start_ms,
            end_ms: start_ms + 1000,
            text: text.to_string(),
            status: "confirmed".to_string(),
            audio_chunk_path: None,
            created_at: "2026-07-17T01:00:00Z".to_string(),
        }
    }

    #[test]
    fn 発話ログは時刻と開始msと話者付きで整形される() {
        let text = build_transcript_text(
            &meeting(),
            &[segment(180_000, "自分", "導入を開始します"), segment(240_000, "PC音声", "了解です")],
        );
        assert_eq!(
            text,
            "[10:03|180000] 自分: 導入を開始します\n[10:04|240000] PC音声: 了解です"
        );
    }

    #[test]
    fn システムプロンプトはjsonスキーマを含む() {
        let prompt = system_prompt();
        assert!(prompt.contains("taskCandidates"));
        assert!(prompt.contains("openQuestions"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn ユーザーメモがあれば本文へ含める() {
        let content = build_user_content(&meeting(), "[10:03|180000] 自分: あ", "重要な補足");
        assert!(content.contains("# 文字起こし"));
        assert!(content.contains("定例会議"));
        assert!(content.contains("# ユーザーメモ"));
        assert!(content.contains("重要な補足"));
    }

    #[test]
    fn ユーザーメモが空なら見出しを付けない() {
        let content = build_user_content(&meeting(), "本文", "   ");
        assert!(!content.contains("ユーザーメモ"));
    }

    use crate::database::open_database;
    use crate::database::providers::{self, BindingInput, ProviderInput};
    use rusqlite::Connection;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    fn provider(conn: &Connection) -> String {
        providers::create_provider(
            conn,
            ProviderInput {
                display_name: "OpenAI".to_string(),
                provider_type: "openai".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                auth_type: "bearer".to_string(),
                organization_id: None,
                project_id: None,
                default_headers: Default::default(),
                timeout_ms: 60000,
                capabilities: vec!["meeting.summary".to_string()],
            },
        )
        .unwrap()
        .id
    }

    fn bind_summary(conn: &Connection, provider_id: &str, model: Option<&str>) {
        providers::set_binding(
            conn,
            MEETING_SUMMARY_FEATURE,
            BindingInput {
                provider_profile_id: Some(provider_id.to_string()),
                model_id: model.map(String::from),
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn binding未設定なら設定不足エラーでローカルへ落ちない() {
        let (_dir, conn) = temp_conn();
        let err = resolve_summary_provider(&conn).unwrap_err();
        assert_eq!(err.code, "SUMMARY_NOT_CONFIGURED");
        assert!(!summary_available(&conn).unwrap());
    }

    #[test]
    fn binding設定済みで有効ならprovider_and_modelを返す() {
        let (_dir, conn) = temp_conn();
        let id = provider(&conn);
        bind_summary(&conn, &id, Some("gpt-4o"));
        let (profile, model) = resolve_summary_provider(&conn).unwrap();
        assert_eq!(profile.id, id);
        assert_eq!(model, "gpt-4o");
        assert!(summary_available(&conn).unwrap());
    }

    #[test]
    fn モデル未指定は設定不足として扱う() {
        let (_dir, conn) = temp_conn();
        let id = provider(&conn);
        bind_summary(&conn, &id, None);
        assert_eq!(
            resolve_summary_provider(&conn).unwrap_err().code,
            "SUMMARY_NOT_CONFIGURED"
        );
    }

    #[test]
    fn provider無効なら無効エラーでローカルへ落ちない() {
        let (_dir, conn) = temp_conn();
        let id = provider(&conn);
        bind_summary(&conn, &id, Some("gpt-4o"));
        providers::set_provider_enabled(&conn, &id, false).unwrap();
        let err = resolve_summary_provider(&conn).unwrap_err();
        assert_eq!(err.code, "API_PROVIDER_DISABLED");
        assert!(!summary_available(&conn).unwrap());
    }
}
