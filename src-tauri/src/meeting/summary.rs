use rusqlite::Connection;

use crate::database::meetings::{Meeting, TranscriptSegment};
use crate::database::providers::{self, ApiProviderProfile};
use crate::error::AppError;
use crate::meeting::markdown;

pub const MEETING_SUMMARY_FEATURE: &str = "meeting.summary";

fn summary_not_configured() -> AppError {
    AppError::new(
        "SUMMARY_NOT_CONFIGURED",
        "議事録の生成にはAPIプロバイダーの設定が必要です。設定画面のAI設定で「議事録・タスク抽出」にProviderとモデルを割り当ててください",
        false,
    )
}

/// 議事録生成の経路解決。文字起こしと異なりローカルフォールバックは無く、
/// APIのFeature Binding（meeting.summary）が有効な場合のみ生成できる。
pub fn resolve_summary_provider(
    conn: &Connection,
) -> Result<(ApiProviderProfile, String), AppError> {
    resolve_summary_providers(conn).map(|mut providers| providers.remove(0))
}

/// Primary、Fallbackの順で実行可能な要約Providerを返す。
pub fn resolve_summary_providers(
    conn: &Connection,
) -> Result<Vec<(ApiProviderProfile, String)>, AppError> {
    let binding = providers::get_binding(conn, MEETING_SUMMARY_FEATURE)?;
    let candidates = binding
        .as_ref()
        .into_iter()
        .flat_map(|binding| {
            [
                (
                    binding.provider_profile_id.as_ref(),
                    binding.model_id.as_ref(),
                ),
                (
                    binding.fallback_provider_profile_id.as_ref(),
                    binding.fallback_model_id.as_ref(),
                ),
            ]
        })
        .filter_map(|(provider_id, model)| Some((provider_id?.clone(), model?.clone())))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(summary_not_configured());
    }
    let mut last_error = None;
    let mut resolved = Vec::new();
    for (provider_id, model) in candidates {
        match providers::get_provider(conn, &provider_id) {
            Ok(profile)
                if profile.enabled
                    && profile
                        .capabilities
                        .iter()
                        .any(|capability| capability == "text.structured_output") =>
            {
                resolved.push((profile, model));
            }
            Ok(profile) if !profile.enabled => {
                last_error = Some(AppError::new(
                    "API_PROVIDER_DISABLED",
                    "議事録生成のProviderが無効化されています",
                    false,
                ));
            }
            Ok(_) => {
                last_error = Some(AppError::new(
                    "API_CAPABILITY_MISSING",
                    "議事録生成Providerにtext.structured_output capabilityがありません",
                    false,
                ));
            }
            Err(err) => last_error = Some(err),
        }
    }
    if resolved.is_empty() {
        Err(last_error.unwrap_or_else(summary_not_configured))
    } else {
        Ok(resolved)
    }
}

/// 議事録生成が可能か（API設定済みか）を返す。UIのボタン活性判定に使う。
pub fn summary_available(conn: &Connection) -> Result<bool, AppError> {
    match resolve_summary_provider(conn) {
        Ok(_) => Ok(true),
        Err(err) if err.code == "SUMMARY_NOT_CONFIGURED" || err.code == "API_PROVIDER_DISABLED" => {
            Ok(false)
        }
        Err(err) => Err(err),
    }
}

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

/// 会議タイトルが日付だけ・空・既定値など内容を表さないものかを判定する。
/// これらの場合はAIが生成したタイトルで会議名を更新し、要約に反映しやすくする。
pub fn is_generic_title(title: &str) -> bool {
    let trimmed = title.trim();
    if trimmed.is_empty() || trimmed == "会議" || trimmed == "無題" {
        return true;
    }
    // 日付・時刻・区切り文字だけで構成されているタイトル（例: 2026-07-20）は内容を表さない。
    trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '-' | '/' | '.' | ':' | ' ' | '_'))
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
     決定事項・タスク・未確認事項が無い場合は空配列にします。\
     summaryは日付やファイル名ではなく、文字起こし本文の内容そのものを日本語で簡潔にまとめます。\
     titleは会議の議題が一目で分かる短い日本語の見出しにします（日付だけにしないでください）。"
        .to_string()
}

/// 既定のシステムプロンプトへ、ユーザーが設定した独自プロンプトを追加指示として結合する。
/// JSONスキーマ指示（前段）は常に保持し、出力形式を壊さないようにする。
pub fn build_system_prompt(custom_prompt: Option<&str>) -> String {
    let base = system_prompt();
    match custom_prompt.map(str::trim).filter(|note| !note.is_empty()) {
        Some(note) => format!(
            "{base}\n\n# 追加の指示\n以下はまとめ方に関するユーザーの希望です。JSON以外を出力しない・スキーマを変えないという上のルールは必ず守ってください。\n{note}"
        ),
        None => base,
    }
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
    fn 日付だけや既定値のタイトルは汎用と判定される() {
        assert!(is_generic_title(""));
        assert!(is_generic_title("  "));
        assert!(is_generic_title("会議"));
        assert!(is_generic_title("無題"));
        assert!(is_generic_title("2026-07-20"));
        assert!(is_generic_title("2026/07/20 14:30"));
        assert!(!is_generic_title("定例会議"));
        assert!(!is_generic_title("7月の予算レビュー"));
    }

    #[test]
    fn システムプロンプトはjsonスキーマを含む() {
        let prompt = system_prompt();
        assert!(prompt.contains("taskCandidates"));
        assert!(prompt.contains("openQuestions"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn 独自プロンプトは追加指示として結合されスキーマ指示は保持される() {
        let base = build_system_prompt(None);
        assert_eq!(base, system_prompt());
        let combined = build_system_prompt(Some("  敬体でまとめて  "));
        assert!(combined.contains("taskCandidates"));
        assert!(combined.contains("# 追加の指示"));
        assert!(combined.contains("敬体でまとめて"));
        // 空白のみの独自プロンプトは無視して既定と同じになる。
        assert_eq!(build_system_prompt(Some("   ")), system_prompt());
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
                display_name: format!("OpenAI {}", uuid::Uuid::new_v4()),
                provider_type: "openai".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                auth_type: "bearer".to_string(),
                organization_id: None,
                project_id: None,
                model_id: None,
                custom_prompt: None,
                default_headers: Default::default(),
                timeout_ms: 60000,
                capabilities: vec!["text.structured_output".to_string()],
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
    fn primary無効時はfallback_providerを使う() {
        let (_dir, conn) = temp_conn();
        let primary = provider(&conn);
        let fallback = provider(&conn);
        providers::set_provider_enabled(&conn, &primary, false).unwrap();
        providers::set_binding(
            &conn,
            MEETING_SUMMARY_FEATURE,
            BindingInput {
                provider_profile_id: Some(primary),
                model_id: Some("primary-model".to_string()),
                fallback_provider_profile_id: Some(fallback.clone()),
                fallback_model_id: Some("fallback-model".to_string()),
            },
        )
        .unwrap();
        let (resolved, model) = resolve_summary_provider(&conn).unwrap();
        assert_eq!(resolved.id, fallback);
        assert_eq!(model, "fallback-model");
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
