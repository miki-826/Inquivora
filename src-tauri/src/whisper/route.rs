use rusqlite::Connection;

use crate::error::AppError;

#[derive(Debug, PartialEq)]
pub enum TranscriptionRoute {
    Api { provider_id: String, model: String },
    Local,
}

/// 文字起こしの経路解決。APIのFeature Bindingが設定済みならAPIを優先し、
/// 未設定または無効ならローカルWhisperへフォールバックする。
pub fn resolve_transcription_route(
    _conn: &Connection,
    _local_available: bool,
) -> Result<TranscriptionRoute, AppError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;
    use crate::database::providers::{self, BindingInput, ProviderInput};
    use crate::meeting::worker::TRANSCRIPTION_FEATURE;

    fn temp_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_database(&dir.path().join("test.db")).unwrap();
        (dir, conn)
    }

    fn create_provider(conn: &Connection) -> String {
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
                capabilities: vec!["transcription.batch".to_string()],
            },
        )
        .unwrap()
        .id
    }

    fn bind(conn: &Connection, provider_id: &str) {
        providers::set_binding(
            conn,
            TRANSCRIPTION_FEATURE,
            BindingInput {
                provider_profile_id: Some(provider_id.to_string()),
                model_id: Some("whisper-1".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn binding設定済みで有効ならapi経路() {
        let (_dir, conn) = temp_conn();
        let provider_id = create_provider(&conn);
        bind(&conn, &provider_id);
        let route = resolve_transcription_route(&conn, true).unwrap();
        assert_eq!(
            route,
            TranscriptionRoute::Api {
                provider_id,
                model: "whisper-1".to_string()
            }
        );
    }

    #[test]
    fn binding未設定でローカルモデルがあればローカル経路() {
        let (_dir, conn) = temp_conn();
        assert_eq!(
            resolve_transcription_route(&conn, true).unwrap(),
            TranscriptionRoute::Local
        );
    }

    #[test]
    fn binding未設定でローカルモデルもなければ準備不足エラー() {
        let (_dir, conn) = temp_conn();
        let err = resolve_transcription_route(&conn, false).unwrap_err();
        assert_eq!(err.code, "TRANSCRIPTION_NOT_READY");
    }

    #[test]
    fn provider無効ならローカルへフォールバックする() {
        let (_dir, conn) = temp_conn();
        let provider_id = create_provider(&conn);
        bind(&conn, &provider_id);
        providers::set_provider_enabled(&conn, &provider_id, false).unwrap();
        assert_eq!(
            resolve_transcription_route(&conn, true).unwrap(),
            TranscriptionRoute::Local
        );
    }

    #[test]
    fn provider無効でローカルもなければ無効エラー() {
        let (_dir, conn) = temp_conn();
        let provider_id = create_provider(&conn);
        bind(&conn, &provider_id);
        providers::set_provider_enabled(&conn, &provider_id, false).unwrap();
        let err = resolve_transcription_route(&conn, false).unwrap_err();
        assert_eq!(err.code, "API_PROVIDER_DISABLED");
    }

    #[test]
    fn モデル未指定のbindingは未設定として扱う() {
        let (_dir, conn) = temp_conn();
        let provider_id = create_provider(&conn);
        providers::set_binding(
            &conn,
            TRANSCRIPTION_FEATURE,
            BindingInput {
                provider_profile_id: Some(provider_id),
                model_id: None,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            resolve_transcription_route(&conn, true).unwrap(),
            TranscriptionRoute::Local
        );
    }
}
