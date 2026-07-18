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
