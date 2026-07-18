use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::Serialize;

use crate::api::client::{
    auth_headers, classify_status, merge_headers, parse_models_response,
    parse_transcription_response, TranscriptionResult,
};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct ProviderRuntime {
    pub base_url: String,
    pub auth_type: String,
    pub secret: Option<String>,
    pub default_headers: HashMap<String, String>,
    pub timeout_ms: u64,
    pub capabilities: Vec<String>,
}

fn http_client(timeout_ms: u64) -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        // §10.3 リダイレクト先へ認証ヘッダーを引き継がないため、リダイレクト自体を追わない
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::new("API_REQUEST_FAILED", format!("HTTPクライアント初期化に失敗: {e}"), false))
}

fn request_headers(profile: &ProviderRuntime) -> HashMap<String, String> {
    let auth = profile
        .secret
        .as_deref()
        .map(|secret| auth_headers(&profile.auth_type, secret))
        .unwrap_or_default();
    merge_headers(&profile.default_headers, &auth)
}

fn map_request_error(err: reqwest::Error) -> AppError {
    if err.is_timeout() {
        AppError::new("API_TIMEOUT", "APIの応答がタイムアウトしました", true)
    } else if err.is_connect() {
        AppError::new(
            "API_CONNECTION_FAILED",
            "APIサーバーへ接続できません。Base URLとネットワークを確認してください",
            true,
        )
    } else {
        AppError::new("API_REQUEST_FAILED", "APIリクエストに失敗しました", false)
    }
}

async fn send_request(
    profile: &ProviderRuntime,
    builder: reqwest::RequestBuilder,
) -> Result<(u16, String), AppError> {
    let mut builder = builder;
    for (name, value) in request_headers(profile) {
        builder = builder.header(name, value);
    }
    let response = builder.send().await.map_err(map_request_error)?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(map_request_error)?;
    Ok((status, body))
}

fn status_error(status: u16, context: &str) -> AppError {
    let (code, retryable) = classify_status(status);
    AppError::new(code, format!("{context} (HTTP {status})"), retryable)
}

pub async fn list_models(profile: &ProviderRuntime) -> Result<Vec<String>, AppError> {
    let client = http_client(profile.timeout_ms)?;
    let url = format!("{}/models", profile.base_url);
    let (status, body) = send_request(profile, client.get(url)).await?;
    if status != 200 {
        return Err(status_error(status, "モデル一覧の取得に失敗しました"));
    }
    parse_models_response(&body)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionTestResult {
    pub success: bool,
    pub checked_at: String,
    pub latency_ms: Option<i64>,
    pub authenticated: bool,
    pub models_endpoint_available: Option<bool>,
    pub capabilities: Vec<String>,
    pub error_code: Option<String>,
    pub user_message: Option<String>,
}

/// §10.6 接続テスト。応答本文は返さず、正規化した結果だけを返す。
pub async fn test_connection(profile: &ProviderRuntime) -> ProviderConnectionTestResult {
    let checked_at = Utc::now().to_rfc3339();
    let mut result = ProviderConnectionTestResult {
        success: false,
        checked_at,
        latency_ms: None,
        authenticated: false,
        models_endpoint_available: None,
        capabilities: profile.capabilities.clone(),
        error_code: None,
        user_message: None,
    };
    let client = match http_client(profile.timeout_ms) {
        Ok(client) => client,
        Err(err) => {
            result.error_code = Some(err.code);
            result.user_message = Some(err.message);
            return result;
        }
    };
    let url = format!("{}/models", profile.base_url);
    let started = Instant::now();
    match send_request(profile, client.get(url)).await {
        Ok((status, _body)) => {
            result.latency_ms = Some(started.elapsed().as_millis() as i64);
            match status {
                200 => {
                    result.success = true;
                    result.authenticated = true;
                    result.models_endpoint_available = Some(true);
                    result.user_message = Some("接続に成功しました".to_string());
                }
                401 | 403 => {
                    result.models_endpoint_available = Some(true);
                    result.error_code = Some("API_AUTH_FAILED".to_string());
                    result.user_message =
                        Some("認証に失敗しました。APIキーを確認してください".to_string());
                }
                404 => {
                    result.success = true;
                    result.authenticated = true;
                    result.models_endpoint_available = Some(false);
                    result.user_message = Some(
                        "接続できましたがモデル一覧APIがありません。モデルIDは手入力してください"
                            .to_string(),
                    );
                }
                _ => {
                    let err = status_error(status, "接続テストに失敗しました");
                    result.error_code = Some(err.code);
                    result.user_message = Some(err.message);
                }
            }
        }
        Err(err) => {
            result.error_code = Some(err.code);
            result.user_message = Some(err.message);
        }
    }
    result
}

#[derive(Debug, Clone)]
pub struct TranscribeRequest {
    pub audio_path: String,
    pub model: String,
    pub language: String,
    pub prompt: Option<String>,
}

pub async fn transcribe(
    profile: &ProviderRuntime,
    request: TranscribeRequest,
) -> Result<TranscriptionResult, AppError> {
    let bytes = tokio::fs::read(&request.audio_path).await.map_err(|e| {
        AppError::new(
            "TRANSCRIPTION_FAILED",
            format!("音声チャンクを読み込めません: {e}"),
            false,
        )
    })?;
    let file_name = Path::new(&request.audio_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "chunk.wav".to_string());
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("audio/wav")
        .map_err(|_| AppError::new("TRANSCRIPTION_FAILED", "音声形式の指定に失敗しました", false))?;
    let mut form = reqwest::multipart::Form::new()
        .text("model", request.model)
        .text("language", request.language)
        .text("response_format", "json")
        .part("file", part);
    if let Some(prompt) = request.prompt {
        form = form.text("prompt", prompt);
    }
    let client = http_client(profile.timeout_ms)?;
    let url = format!("{}/audio/transcriptions", profile.base_url);
    let (status, body) = send_request(profile, client.post(url).multipart(form)).await?;
    if status != 200 {
        return Err(status_error(status, "文字起こしAPIが失敗しました"));
    }
    parse_transcription_response(&body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    struct MockRequest {
        head: String,
        body: Vec<u8>,
    }

    fn spawn_mock_server(status_line: &'static str, body: &'static str) -> (String, mpsc::Receiver<MockRequest>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("モックサーバーを起動できない");
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let mut header_end = None;
                loop {
                    let n = stream.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    if header_end.is_none() {
                        header_end = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4);
                    }
                    if let Some(end) = header_end {
                        let head = String::from_utf8_lossy(&buf[..end]).to_string();
                        let content_length = head
                            .lines()
                            .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                            .and_then(|l| l.split(':').nth(1))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if buf.len() >= end + content_length {
                            let body_bytes = buf[end..end + content_length].to_vec();
                            let _ = tx.send(MockRequest {
                                head,
                                body: body_bytes,
                            });
                            break;
                        }
                    }
                }
                let response = format!(
                    "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (format!("http://127.0.0.1:{port}/v1"), rx)
    }

    fn runtime_profile(base_url: &str, secret: Option<&str>) -> ProviderRuntime {
        ProviderRuntime {
            base_url: base_url.to_string(),
            auth_type: "bearer".to_string(),
            secret: secret.map(String::from),
            default_headers: HashMap::new(),
            timeout_ms: 5000,
            capabilities: vec![
                "transcription.batch".to_string(),
                "models.list".to_string(),
            ],
        }
    }

    #[tokio::test]
    async fn モデル一覧を取得し認証ヘッダーが送信される() {
        let (base, rx) = spawn_mock_server(
            "HTTP/1.1 200 OK",
            r#"{"object":"list","data":[{"id":"whisper-1"},{"id":"gpt-4o"}]}"#,
        );
        let models = list_models(&runtime_profile(&base, Some("sk-test")))
            .await
            .unwrap();
        assert_eq!(models, vec!["gpt-4o", "whisper-1"]);
        let req = rx.recv().unwrap();
        assert!(req.head.starts_with("GET /v1/models"));
        assert!(req.head.contains("Bearer sk-test"));
    }

    #[tokio::test]
    async fn 認証失敗はapi_auth_failedになる() {
        let (base, _rx) = spawn_mock_server("HTTP/1.1 401 Unauthorized", r#"{"error":"bad key"}"#);
        let err = list_models(&runtime_profile(&base, Some("sk-bad")))
            .await
            .unwrap_err();
        assert_eq!(err.code, "API_AUTH_FAILED");
        assert!(!err.retryable);
    }

    #[tokio::test]
    async fn 接続テスト成功で応答時間とcapabilityが返る() {
        let (base, _rx) = spawn_mock_server(
            "HTTP/1.1 200 OK",
            r#"{"object":"list","data":[{"id":"whisper-1"}]}"#,
        );
        let result = test_connection(&runtime_profile(&base, Some("sk-test"))).await;
        assert!(result.success);
        assert!(result.authenticated);
        assert_eq!(result.models_endpoint_available, Some(true));
        assert!(result.latency_ms.is_some());
        assert!(result.error_code.is_none());
    }

    #[tokio::test]
    async fn 接続テストは認証失敗を報告する() {
        let (base, _rx) = spawn_mock_server("HTTP/1.1 401 Unauthorized", r#"{}"#);
        let result = test_connection(&runtime_profile(&base, Some("sk-bad"))).await;
        assert!(!result.success);
        assert!(!result.authenticated);
        assert_eq!(result.error_code.as_deref(), Some("API_AUTH_FAILED"));
        assert!(result.user_message.is_some());
    }

    #[tokio::test]
    async fn 接続不能なホストは接続エラーになる() {
        let result = test_connection(&runtime_profile("http://127.0.0.1:1/v1", Some("sk"))).await;
        assert!(!result.success);
        assert_eq!(result.error_code.as_deref(), Some("API_CONNECTION_FAILED"));
    }

    #[tokio::test]
    async fn 文字起こしはmultipartで音声を送信する() {
        let (base, rx) = spawn_mock_server(
            "HTTP/1.1 200 OK",
            r#"{"text":"こんにちは。","language":"ja"}"#,
        );
        let dir = tempfile::tempdir().unwrap();
        let wav_path = dir.path().join("chunk0.wav");
        std::fs::write(&wav_path, b"RIFFfakewav").unwrap();
        let result = transcribe(
            &runtime_profile(&base, Some("sk-test")),
            TranscribeRequest {
                audio_path: wav_path.to_string_lossy().to_string(),
                model: "whisper-1".to_string(),
                language: "ja".to_string(),
                prompt: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(result.text, "こんにちは。");
        let req = rx.recv().unwrap();
        assert!(req.head.starts_with("POST /v1/audio/transcriptions"));
        assert!(req.head.contains("multipart/form-data"));
        let body = String::from_utf8_lossy(&req.body);
        assert!(body.contains("whisper-1"));
        assert!(body.contains("chunk0.wav"));
        assert!(body.contains("RIFFfakewav"));
    }
}
