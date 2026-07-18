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
