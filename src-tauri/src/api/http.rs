use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::Serialize;

use crate::api::client::{
    auth_headers, classify_status, merge_headers, parse_anthropic_content,
    parse_chat_completion_content, parse_gemini_content, parse_meeting_ai_output,
    parse_models_response, parse_ollama_chat_content, parse_transcription_response, MeetingAiOutput,
    TranscriptionResult,
};
use crate::error::AppError;

const MAX_API_RESPONSE_BYTES: usize = 10 * 1024 * 1024;
const SUMMARY_CHUNK_CHARS: usize = 48_000;
const MAX_SUMMARY_CHUNKS: usize = 12;

pub fn runtime_from_profile(
    profile: &crate::database::providers::ApiProviderProfile,
    secret: Option<String>,
) -> ProviderRuntime {
    ProviderRuntime {
        provider_type: profile.provider_type.clone(),
        base_url: profile.base_url.clone(),
        auth_type: profile.auth_type.clone(),
        secret,
        default_headers: profile.default_headers.clone(),
        organization_id: profile.organization_id.clone(),
        project_id: profile.project_id.clone(),
        timeout_ms: profile.timeout_ms.max(1000) as u64,
        capabilities: profile.capabilities.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct ProviderRuntime {
    pub provider_type: String,
    pub base_url: String,
    pub auth_type: String,
    pub secret: Option<String>,
    pub default_headers: HashMap<String, String>,
    pub organization_id: Option<String>,
    pub project_id: Option<String>,
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
    let mut headers = profile.default_headers.clone();
    if profile.provider_type == "openai" {
        if let Some(organization_id) = profile.organization_id.as_deref().filter(|v| !v.trim().is_empty()) {
            headers.insert("OpenAI-Organization".to_string(), organization_id.trim().to_string());
        }
        if let Some(project_id) = profile.project_id.as_deref().filter(|v| !v.trim().is_empty()) {
            headers.insert("OpenAI-Project".to_string(), project_id.trim().to_string());
        }
    }
    if profile.provider_type == "anthropic" {
        headers
            .entry("anthropic-version".to_string())
            .or_insert_with(|| "2023-06-01".to_string());
    }
    merge_headers(&headers, &auth)
}

fn require_capability(profile: &ProviderRuntime, capability: &str) -> Result<(), AppError> {
    if profile.capabilities.iter().any(|value| value == capability) {
        Ok(())
    } else {
        Err(AppError::new(
            "API_CAPABILITY_MISSING",
            format!("Providerが必要な機能（{capability}）に対応していません"),
            false,
        ))
    }
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
    if response
        .content_length()
        .is_some_and(|size| size > MAX_API_RESPONSE_BYTES as u64)
    {
        return Err(AppError::new(
            "API_RESPONSE_TOO_LARGE",
            "APIの応答が上限（10MB）を超えています",
            false,
        ));
    }
    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_request_error)? {
        if bytes.len().saturating_add(chunk.len()) > MAX_API_RESPONSE_BYTES {
            return Err(AppError::new(
                "API_RESPONSE_TOO_LARGE",
                "APIの応答が上限（10MB）を超えています",
                false,
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let body = String::from_utf8(bytes).map_err(|_| {
        AppError::new(
            "API_INVALID_RESPONSE",
            "APIからUTF-8ではない応答が返されました",
            false,
        )
    })?;
    Ok((status, body))
}

fn status_error(status: u16, context: &str) -> AppError {
    let (code, retryable) = classify_status(status);
    AppError::new(code, format!("{context} (HTTP {status})"), retryable)
}

pub async fn list_models(profile: &ProviderRuntime) -> Result<Vec<String>, AppError> {
    require_capability(profile, "models.list")?;
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
    let probe = match profile.provider_type.as_str() {
        "anthropic" => client.get(format!("{}/v1/models", profile.base_url)),
        "ollama" => client.get(format!("{}/api/tags", profile.base_url)),
        "gemini" => client
            .get(format!("{}/models", profile.base_url))
            .query(&[("key", profile.secret.as_deref().unwrap_or(""))]),
        _ => client.get(format!("{}/models", profile.base_url)),
    };
    let started = Instant::now();
    match send_request(profile, probe).await {
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
    require_capability(profile, "transcription.batch")?;
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

#[derive(Debug, Clone)]
pub struct SummaryRequest {
    pub model: String,
    pub system_prompt: String,
    pub user_content: String,
}

fn user_content_with_repair(request: &SummaryRequest, repair_note: Option<&str>) -> String {
    match repair_note {
        Some(note) => format!("{}\n\n# 修復指示\n{}", request.user_content, note),
        None => request.user_content.clone(),
    }
}

fn chat_body(request: &SummaryRequest, repair_note: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "model": request.model,
        "temperature": 0.2,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": request.system_prompt },
            { "role": "user", "content": user_content_with_repair(request, repair_note) },
        ],
    })
}

fn anthropic_body(request: &SummaryRequest, repair_note: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "model": request.model,
        "max_tokens": 4096,
        "temperature": 0.2,
        "system": request.system_prompt,
        "messages": [
            { "role": "user", "content": user_content_with_repair(request, repair_note) },
        ],
    })
}

fn gemini_body(request: &SummaryRequest, repair_note: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "systemInstruction": { "parts": [{ "text": request.system_prompt }] },
        "contents": [
            { "role": "user", "parts": [{ "text": user_content_with_repair(request, repair_note) }] },
        ],
        "generationConfig": { "responseMimeType": "application/json", "temperature": 0.2 },
    })
}

fn ollama_body(request: &SummaryRequest, repair_note: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "model": request.model,
        "stream": false,
        "format": "json",
        "options": { "temperature": 0.2 },
        "messages": [
            { "role": "system", "content": request.system_prompt },
            { "role": "user", "content": user_content_with_repair(request, repair_note) },
        ],
    })
}

type ContentParser = fn(&str) -> Result<String, AppError>;

/// プロバイダー種別ごとにチャット系エンドポイントへ送り、本文テキストを返す。
async fn post_summary(
    profile: &ProviderRuntime,
    client: &reqwest::Client,
    request: &SummaryRequest,
    repair_note: Option<&str>,
) -> Result<String, AppError> {
    let (builder, parse): (reqwest::RequestBuilder, ContentParser) =
        match profile.provider_type.as_str() {
            "anthropic" => (
                client
                    .post(format!("{}/v1/messages", profile.base_url))
                    .json(&anthropic_body(request, repair_note)),
                parse_anthropic_content,
            ),
            "gemini" => (
                client
                    .post(format!(
                        "{}/models/{}:generateContent",
                        profile.base_url, request.model
                    ))
                    .query(&[("key", profile.secret.as_deref().unwrap_or(""))])
                    .json(&gemini_body(request, repair_note)),
                parse_gemini_content,
            ),
            "ollama" => (
                client
                    .post(format!("{}/api/chat", profile.base_url))
                    .json(&ollama_body(request, repair_note)),
                parse_ollama_chat_content,
            ),
            _ => (
                client
                    .post(format!("{}/chat/completions", profile.base_url))
                    .json(&chat_body(request, repair_note)),
                parse_chat_completion_content,
            ),
        };
    let (status, response_body) = send_request(profile, builder).await?;
    if status != 200 {
        return Err(status_error(status, "議事録生成APIが失敗しました"));
    }
    parse(&response_body)
}

/// §10.10 議事録生成。JSONスキーマ不一致の場合は1回だけ修復リクエストを行う。
async fn generate_summary_once(
    profile: &ProviderRuntime,
    request: SummaryRequest,
) -> Result<MeetingAiOutput, AppError> {
    let client = http_client(profile.timeout_ms)?;
    let content = post_summary(profile, &client, &request, None).await?;
    match parse_meeting_ai_output(&content) {
        Ok(output) => Ok(output),
        Err(_) => {
            let repair = "前回の応答が指定スキーマの有効なJSONではありませんでした。説明やコードフェンスを付けず、スキーマに一致するJSONオブジェクトのみを返してください。";
            let retried = post_summary(profile, &client, &request, Some(repair)).await?;
            parse_meeting_ai_output(&retried)
        }
    }
}

fn split_summary_content(content: &str) -> Result<Vec<String>, AppError> {
    let chars = content.chars().count();
    if chars <= SUMMARY_CHUNK_CHARS {
        return Ok(vec![content.to_string()]);
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in content.split_inclusive('\n') {
        let mut rest = line;
        while !rest.is_empty() {
            let room = SUMMARY_CHUNK_CHARS.saturating_sub(current.chars().count());
            if room == 0 {
                chunks.push(std::mem::take(&mut current));
                continue;
            }
            let split_byte = rest
                .char_indices()
                .nth(room)
                .map(|(index, _)| index)
                .unwrap_or(rest.len());
            current.push_str(&rest[..split_byte]);
            rest = &rest[split_byte..];
            if current.chars().count() >= SUMMARY_CHUNK_CHARS {
                chunks.push(std::mem::take(&mut current));
            }
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.len() > MAX_SUMMARY_CHUNKS {
        return Err(AppError::new(
            "MEETING_TOO_LONG",
            "会議内容が長すぎます。要約対象を分けてください（1回につき約58万文字まで）",
            false,
        ));
    }
    Ok(chunks)
}

/// 長い会議は安全な文字数で分割して要約し、重複を除きながら結果を統合する。
pub async fn generate_summary(
    profile: &ProviderRuntime,
    request: SummaryRequest,
) -> Result<MeetingAiOutput, AppError> {
    require_capability(profile, "text.structured_output")?;
    let chunks = split_summary_content(&request.user_content)?;
    let total = chunks.len();
    let mut outputs = Vec::with_capacity(total);
    for (index, chunk) in chunks.into_iter().enumerate() {
        let user_content = if total == 1 {
            chunk
        } else {
            format!(
                "# 分割要約\n全{total}分割のうち{}番目です。この部分だけから根拠のある項目を抽出してください。\n\n{chunk}",
                index + 1
            )
        };
        outputs.push(
            generate_summary_once(
                profile,
                SummaryRequest {
                    model: request.model.clone(),
                    system_prompt: request.system_prompt.clone(),
                    user_content,
                },
            )
            .await?,
        );
    }
    if outputs.len() == 1 {
        return Ok(outputs.remove(0));
    }

    let mut merged = MeetingAiOutput {
        title: outputs.iter().find_map(|item| (!item.title.trim().is_empty()).then(|| item.title.clone())).unwrap_or_default(),
        summary: outputs
            .iter()
            .enumerate()
            .map(|(index, item)| format!("【{}】{}", index + 1, item.summary.trim()))
            .collect::<Vec<_>>()
            .join("\n\n"),
        decisions: Vec::new(),
        task_candidates: Vec::new(),
        open_questions: Vec::new(),
    };
    let mut decisions = HashSet::new();
    let mut tasks = HashSet::new();
    let mut questions = HashSet::new();
    for output in outputs {
        merged.decisions.extend(output.decisions.into_iter().filter(|item| {
            decisions.insert(item.text.trim().to_lowercase())
        }));
        merged.task_candidates.extend(output.task_candidates.into_iter().filter(|item| {
            tasks.insert(item.title.trim().to_lowercase())
        }));
        merged.open_questions.extend(output.open_questions.into_iter().filter(|item| {
            questions.insert(item.text.trim().to_lowercase())
        }));
    }
    Ok(merged)
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
        let (root, rx) = spawn_mock_server_root(status_line, body);
        (format!("{root}/v1"), rx)
    }

    fn spawn_mock_server_root(status_line: &'static str, body: &'static str) -> (String, mpsc::Receiver<MockRequest>) {
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
        (format!("http://127.0.0.1:{port}"), rx)
    }

    fn provider_runtime(provider_type: &str, base_url: &str, secret: Option<&str>) -> ProviderRuntime {
        let mut profile = runtime_profile(base_url, secret);
        provider_type.clone_into(&mut profile.provider_type);
        profile.auth_type = match provider_type {
            "anthropic" => "x-api-key",
            "ollama" => "none",
            "gemini" => "query",
            _ => "bearer",
        }
        .to_string();
        profile
    }

    fn runtime_profile(base_url: &str, secret: Option<&str>) -> ProviderRuntime {
        ProviderRuntime {
            provider_type: "openai".to_string(),
            base_url: base_url.to_string(),
            auth_type: "bearer".to_string(),
            secret: secret.map(String::from),
            default_headers: HashMap::new(),
            organization_id: None,
            project_id: None,
            timeout_ms: 5000,
            capabilities: vec![
                "transcription.batch".to_string(),
                "models.list".to_string(),
                "text.generate".to_string(),
                "text.structured_output".to_string(),
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
    async fn 議事録生成はchat_completionsへjsonを送り構造化出力を返す() {
        let (base, rx) = spawn_mock_server(
            "HTTP/1.1 200 OK",
            r#"{"choices":[{"message":{"role":"assistant","content":"{\"title\":\"定例\",\"summary\":\"要約本文\",\"decisions\":[{\"text\":\"試験導入する\",\"sourceStartMs\":12000}],\"taskCandidates\":[{\"title\":\"見積り\",\"priority\":\"high\"}],\"openQuestions\":[]}"}}]}"#,
        );
        let output = generate_summary(
            &runtime_profile(&base, Some("sk-test")),
            SummaryRequest {
                model: "gpt-4o".to_string(),
                system_prompt: "system".to_string(),
                user_content: "[10:03|12000] 自分: 導入します".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.summary, "要約本文");
        assert_eq!(output.decisions.len(), 1);
        assert_eq!(output.task_candidates[0].priority, "high");
        let req = rx.recv().unwrap();
        assert!(req.head.starts_with("POST /v1/chat/completions"));
        let body = String::from_utf8_lossy(&req.body);
        assert!(body.contains("\"model\":\"gpt-4o\""));
        assert!(body.contains("json_object"));
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

    #[test]
    fn 長い日本語入力は文字境界を壊さず分割する() {
        let source = "会議メモです。".repeat(10_000);
        let chunks = split_summary_content(&source).unwrap();
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.chars().count() <= SUMMARY_CHUNK_CHARS));
        assert_eq!(chunks.concat(), source);
    }

    #[test]
    fn openai組織とprojectは専用headerへ反映する() {
        let mut profile = runtime_profile("https://api.openai.com/v1", Some("sk"));
        profile.organization_id = Some("org-test".to_string());
        profile.project_id = Some("proj-test".to_string());
        let headers = request_headers(&profile);
        assert_eq!(headers.get("OpenAI-Organization").map(String::as_str), Some("org-test"));
        assert_eq!(headers.get("OpenAI-Project").map(String::as_str), Some("proj-test"));
    }

    #[tokio::test]
    async fn capability不足なら通信前に拒否する() {
        let mut profile = runtime_profile("http://127.0.0.1:1/v1", Some("sk"));
        profile.capabilities.clear();
        let err = list_models(&profile).await.unwrap_err();
        assert_eq!(err.code, "API_CAPABILITY_MISSING");
    }

    const SUMMARY_JSON: &str =
        r#"{\"title\":\"定例\",\"summary\":\"要約本文\",\"decisions\":[],\"taskCandidates\":[],\"openQuestions\":[]}"#;

    fn summary_request() -> SummaryRequest {
        SummaryRequest {
            model: "test-model".to_string(),
            system_prompt: "system".to_string(),
            user_content: "[10:03|12000] 自分: 導入します".to_string(),
        }
    }

    #[tokio::test]
    async fn anthropic議事録はv1_messagesへ送りx_api_keyを付ける() {
        let body = format!(r#"{{"content":[{{"type":"text","text":"{SUMMARY_JSON}"}}]}}"#);
        let leaked: &'static str = Box::leak(body.into_boxed_str());
        let (base, rx) = spawn_mock_server_root("HTTP/1.1 200 OK", leaked);
        let output = generate_summary(&provider_runtime("anthropic", &base, Some("sk-ant")), summary_request())
            .await
            .unwrap();
        assert_eq!(output.summary, "要約本文");
        let req = rx.recv().unwrap();
        assert!(req.head.starts_with("POST /v1/messages"));
        assert!(req.head.to_lowercase().contains("x-api-key: sk-ant"));
        assert!(req.head.to_lowercase().contains("anthropic-version: 2023-06-01"));
        let sent = String::from_utf8_lossy(&req.body);
        assert!(sent.contains("\"system\":\"system\""));
        assert!(sent.contains("\"max_tokens\""));
    }

    #[tokio::test]
    async fn gemini議事録はgenerate_contentへkey付きで送る() {
        let body = format!(r#"{{"candidates":[{{"content":{{"parts":[{{"text":"{SUMMARY_JSON}"}}]}}}}]}}"#);
        let leaked: &'static str = Box::leak(body.into_boxed_str());
        let (base, rx) = spawn_mock_server_root("HTTP/1.1 200 OK", leaked);
        let output = generate_summary(&provider_runtime("gemini", &base, Some("g-key")), summary_request())
            .await
            .unwrap();
        assert_eq!(output.summary, "要約本文");
        let req = rx.recv().unwrap();
        assert!(req.head.starts_with("POST /models/test-model:generateContent"));
        assert!(req.head.contains("key=g-key"));
        let sent = String::from_utf8_lossy(&req.body);
        assert!(sent.contains("systemInstruction"));
        assert!(sent.contains("responseMimeType"));
    }

    #[tokio::test]
    async fn ollama議事録はapi_chatへ認証なしで送る() {
        let body = format!(r#"{{"message":{{"role":"assistant","content":"{SUMMARY_JSON}"}},"done":true}}"#);
        let leaked: &'static str = Box::leak(body.into_boxed_str());
        let (base, rx) = spawn_mock_server_root("HTTP/1.1 200 OK", leaked);
        let output = generate_summary(&provider_runtime("ollama", &base, None), summary_request())
            .await
            .unwrap();
        assert_eq!(output.summary, "要約本文");
        let req = rx.recv().unwrap();
        assert!(req.head.starts_with("POST /api/chat"));
        assert!(!req.head.to_lowercase().contains("authorization"));
        let sent = String::from_utf8_lossy(&req.body);
        assert!(sent.contains("\"format\":\"json\""));
        assert!(sent.contains("\"stream\":false"));
    }
}
