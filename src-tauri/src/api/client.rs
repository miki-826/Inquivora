use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

const PROTECTED_HEADERS: &[&str] = &["authorization", "x-api-key", "proxy-authorization", "cookie"];

pub fn auth_headers(auth_type: &str, secret: &str) -> Vec<(String, String)> {
    match auth_type {
        "bearer" => vec![("Authorization".to_string(), format!("Bearer {secret}"))],
        "x-api-key" => vec![("x-api-key".to_string(), secret.to_string())],
        _ => Vec::new(),
    }
}

/// カスタムヘッダーと認証ヘッダーを結合する。認証系ヘッダーは常に正規の値が優先される。
pub fn merge_headers(
    defaults: &HashMap<String, String>,
    auth: &[(String, String)],
) -> HashMap<String, String> {
    let mut merged: HashMap<String, String> = defaults
        .iter()
        .filter(|(name, _)| !PROTECTED_HEADERS.contains(&name.to_ascii_lowercase().as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    for (name, value) in auth {
        merged.insert(name.clone(), value.clone());
    }
    merged
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

pub fn parse_models_response(body: &str) -> Result<Vec<String>, AppError> {
    let parsed: ModelsResponse = serde_json::from_str(body).map_err(|_| {
        AppError::new(
            "API_RESPONSE_INVALID",
            "モデル一覧の応答を解釈できません",
            false,
        )
    })?;
    let mut models: Vec<String> = parsed.data.into_iter().map(|entry| entry.id).collect();
    models.sort();
    Ok(models)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RawTranscription {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
}

pub fn parse_transcription_response(body: &str) -> Result<TranscriptionResult, AppError> {
    let raw: RawTranscription = serde_json::from_str(body).map_err(|_| {
        AppError::new(
            "API_RESPONSE_INVALID",
            "文字起こしの応答を解釈できません",
            false,
        )
    })?;
    Ok(TranscriptionResult {
        text: raw.text,
        language: raw.language,
        duration_ms: raw.duration.map(|d| (d * 1000.0).round() as i64),
    })
}

pub fn classify_status(status: u16) -> (&'static str, bool) {
    match status {
        401 | 403 => ("API_AUTH_FAILED", false),
        404 => ("API_NOT_FOUND", false),
        429 => ("API_RATE_LIMITED", true),
        500..=599 => ("API_SERVER_ERROR", true),
        _ => ("API_REQUEST_FAILED", false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn bearer認証はauthorizationヘッダーになる() {
        let headers = auth_headers("bearer", "sk-test");
        assert_eq!(
            headers,
            vec![("Authorization".to_string(), "Bearer sk-test".to_string())]
        );
    }

    #[test]
    fn x_api_key認証は専用ヘッダーになる() {
        let headers = auth_headers("x-api-key", "key-1");
        assert_eq!(headers, vec![("x-api-key".to_string(), "key-1".to_string())]);
    }

    #[test]
    fn 認証なしはヘッダーを付けない() {
        assert!(auth_headers("none", "ignored").is_empty());
    }

    #[test]
    fn カスタムヘッダーは認証ヘッダーを上書きできない() {
        let mut defaults = HashMap::new();
        defaults.insert("X-Custom".to_string(), "1".to_string());
        defaults.insert("Authorization".to_string(), "Bearer stolen".to_string());
        let merged = merge_headers(&defaults, &auth_headers("bearer", "sk-real"));
        assert_eq!(merged.get("X-Custom").map(String::as_str), Some("1"));
        assert_eq!(
            merged.get("Authorization").map(String::as_str),
            Some("Bearer sk-real")
        );
    }

    #[test]
    fn モデル一覧レスポンスをソート済みで取り出せる() {
        let body = r#"{"object":"list","data":[{"id":"whisper-1"},{"id":"gpt-4o-mini"},{"id":"gpt-4o"}]}"#;
        let models = parse_models_response(body).unwrap();
        assert_eq!(models, vec!["gpt-4o", "gpt-4o-mini", "whisper-1"]);
    }

    #[test]
    fn 不正なモデル一覧レスポンスはエラーになる() {
        assert!(parse_models_response("not json").is_err());
        assert!(parse_models_response(r#"{"items":[]}"#).is_err());
    }

    #[test]
    fn 文字起こしレスポンスを取り出せる() {
        let body = r#"{"text":"こんにちは。","language":"ja","duration":19.5}"#;
        let result = parse_transcription_response(body).unwrap();
        assert_eq!(result.text, "こんにちは。");
        assert_eq!(result.language.as_deref(), Some("ja"));
        assert_eq!(result.duration_ms, Some(19500));
    }

    #[test]
    fn textだけの文字起こしレスポンスも受理する() {
        let result = parse_transcription_response(r#"{"text":"はい"}"#).unwrap();
        assert_eq!(result.text, "はい");
        assert!(result.language.is_none());
        assert!(result.duration_ms.is_none());
    }

    #[test]
    fn httpステータスをエラーコードへ正規化する() {
        assert_eq!(classify_status(401), ("API_AUTH_FAILED", false));
        assert_eq!(classify_status(403), ("API_AUTH_FAILED", false));
        assert_eq!(classify_status(404), ("API_NOT_FOUND", false));
        assert_eq!(classify_status(429), ("API_RATE_LIMITED", true));
        assert_eq!(classify_status(500), ("API_SERVER_ERROR", true));
        assert_eq!(classify_status(503), ("API_SERVER_ERROR", true));
        assert_eq!(classify_status(400), ("API_REQUEST_FAILED", false));
    }
}
