use serde::Serialize;

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{code}: {message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            retryable,
        }
    }

    pub fn database_migration_failed(message: impl Into<String>) -> Self {
        Self::new("DATABASE_MIGRATION_FAILED", message, false)
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::new("DATABASE_ERROR", message, false)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        Self::database(err.to_string())
    }
}
