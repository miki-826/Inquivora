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

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

/// OpenAI互換のchat/completions応答から最初のメッセージ本文を取り出す。
pub fn parse_chat_completion_content(body: &str) -> Result<String, AppError> {
    let parsed: ChatCompletionResponse = serde_json::from_str(body).map_err(|_| {
        AppError::new("API_RESPONSE_INVALID", "AI応答を解釈できません", false)
    })?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| AppError::new("API_RESPONSE_INVALID", "AI応答に本文がありません", false))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAiOutput {
    pub title: String,
    pub summary: String,
    pub decisions: Vec<AiDecision>,
    pub task_candidates: Vec<AiTaskCandidate>,
    pub open_questions: Vec<AiOpenQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiDecision {
    pub text: String,
    #[serde(default)]
    pub source_start_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTaskCandidate {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    pub priority: String,
    #[serde(default)]
    pub source_start_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiOpenQuestion {
    pub text: String,
    #[serde(default)]
    pub source_start_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMeetingAi {
    #[serde(default)]
    title: String,
    summary: String,
    #[serde(default)]
    decisions: Vec<AiDecision>,
    #[serde(default)]
    task_candidates: Vec<AiTaskCandidate>,
    #[serde(default)]
    open_questions: Vec<AiOpenQuestion>,
}

fn invalid_ai_output() -> AppError {
    AppError::new(
        "API_RESPONSE_INVALID",
        "AIの議事録出力が仕様の形式ではありません",
        false,
    )
}

/// コードフェンスや前後の説明文が混じっていても最初のJSONオブジェクトを抜き出す。
fn extract_json_object(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end > start {
        Some(&content[start..=end])
    } else {
        None
    }
}

/// §10.10 議事録AI出力をZod相当で検証して取り出す。
pub fn parse_meeting_ai_output(content: &str) -> Result<MeetingAiOutput, AppError> {
    let json = extract_json_object(content).ok_or_else(invalid_ai_output)?;
    let raw: RawMeetingAi = serde_json::from_str(json).map_err(|_| invalid_ai_output())?;
    if raw.summary.trim().is_empty() {
        return Err(invalid_ai_output());
    }
    for candidate in &raw.task_candidates {
        if !matches!(candidate.priority.as_str(), "high" | "medium" | "low") {
            return Err(invalid_ai_output());
        }
        if candidate.title.trim().is_empty() {
            return Err(invalid_ai_output());
        }
    }
    Ok(MeetingAiOutput {
        title: raw.title,
        summary: raw.summary,
        decisions: raw.decisions,
        task_candidates: raw.task_candidates,
        open_questions: raw.open_questions,
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
    fn チャット応答からcontentを取り出せる() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"{\"ok\":true}"}}]}"#;
        assert_eq!(parse_chat_completion_content(body).unwrap(), r#"{"ok":true}"#);
    }

    #[test]
    fn choicesが空のチャット応答はエラー() {
        assert!(parse_chat_completion_content(r#"{"choices":[]}"#).is_err());
        assert!(parse_chat_completion_content("not json").is_err());
    }

    #[test]
    fn 議事録ai出力を検証して取り出せる() {
        let content = r#"{
            "title": "定例会議",
            "summary": "新機能の導入方針を確認した。",
            "decisions": [{"text": "8月から試験導入する", "sourceStartMs": 12000}],
            "taskCandidates": [
                {"title": "見積り作成", "priority": "high", "assignee": "田中", "sourceStartMs": 30000},
                {"title": "資料共有", "priority": "low"}
            ],
            "openQuestions": [{"text": "予算上限は?"}]
        }"#;
        let output = parse_meeting_ai_output(content).unwrap();
        assert_eq!(output.summary, "新機能の導入方針を確認した。");
        assert_eq!(output.decisions.len(), 1);
        assert_eq!(output.decisions[0].source_start_ms, Some(12000));
        assert_eq!(output.task_candidates.len(), 2);
        assert_eq!(output.task_candidates[0].priority, "high");
        assert_eq!(output.task_candidates[0].assignee.as_deref(), Some("田中"));
        assert_eq!(output.open_questions.len(), 1);
    }

    #[test]
    fn コードフェンス付きの議事録aiも解釈する() {
        let content = "```json\n{\"title\":\"会議\",\"summary\":\"要約\",\"decisions\":[],\"taskCandidates\":[],\"openQuestions\":[]}\n```";
        let output = parse_meeting_ai_output(content).unwrap();
        assert_eq!(output.summary, "要約");
        assert!(output.decisions.is_empty());
    }

    #[test]
    fn 不正なpriorityの議事録aiはエラー() {
        let content = r#"{"title":"会議","summary":"要約","decisions":[],"taskCandidates":[{"title":"x","priority":"urgent"}],"openQuestions":[]}"#;
        assert!(parse_meeting_ai_output(content).is_err());
    }

    #[test]
    fn 要約が空の議事録aiはエラー() {
        let content = r#"{"title":"会議","summary":"   ","decisions":[],"taskCandidates":[],"openQuestions":[]}"#;
        assert!(parse_meeting_ai_output(content).is_err());
    }

    #[test]
    fn jsonでない議事録aiはエラー() {
        assert!(parse_meeting_ai_output("これはJSONではありません").is_err());
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
