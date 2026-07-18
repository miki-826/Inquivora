use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::api::credentials;
use crate::api::http::{self, ProviderConnectionTestResult};
use crate::database::providers::{
    self, ApiProviderProfile, BindingInput, FeatureBinding, ProviderInput, ProviderPatch, UsageLog,
};
use crate::error::AppError;
use crate::DbState;

fn lock_error(e: impl std::fmt::Display) -> AppError {
    AppError::database(format!("DB接続ロックに失敗: {e}"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    #[serde(flatten)]
    pub profile: ApiProviderProfile,
    pub has_secret: bool,
}

#[tauri::command]
pub async fn api_provider_list(app: AppHandle) -> Result<Vec<ProviderView>, AppError> {
    let profiles = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        providers::list_providers(&conn)?
    };
    let mut views = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let has_secret = credentials::has_secret(&app, &profile.id).await.unwrap_or(false);
        views.push(ProviderView { profile, has_secret });
    }
    Ok(views)
}

#[tauri::command]
pub async fn api_provider_get(app: AppHandle, id: String) -> Result<ProviderView, AppError> {
    let profile = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        providers::get_provider(&conn, &id)?
    };
    let has_secret = credentials::has_secret(&app, &profile.id).await.unwrap_or(false);
    Ok(ProviderView { profile, has_secret })
}

#[tauri::command]
pub fn api_provider_create(
    state: State<'_, DbState>,
    input: ProviderInput,
) -> Result<ApiProviderProfile, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    providers::create_provider(&conn, input)
}

#[tauri::command]
pub fn api_provider_update(
    state: State<'_, DbState>,
    id: String,
    patch: ProviderPatch,
) -> Result<ApiProviderProfile, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    providers::update_provider(&conn, &id, patch)
}

#[tauri::command]
pub async fn api_provider_delete(app: AppHandle, id: String) -> Result<(), AppError> {
    let _ = credentials::delete_secret(&app, &id).await;
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(lock_error)?;
    providers::delete_provider(&conn, &id)
}

#[tauri::command]
pub fn api_provider_enable(
    state: State<'_, DbState>,
    id: String,
    enabled: bool,
) -> Result<(), AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    providers::set_provider_enabled(&conn, &id, enabled)
}

#[tauri::command]
pub async fn api_provider_set_secret(
    app: AppHandle,
    id: String,
    secret: String,
) -> Result<(), AppError> {
    if secret.trim().is_empty() {
        return Err(AppError::new("VALIDATION_ERROR", "APIキーを入力してください", false));
    }
    let provider_type = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        providers::get_provider(&conn, &id)?.provider_type
    };
    credentials::set_secret(&app, &id, &provider_type, secret.trim()).await
}

#[tauri::command]
pub async fn api_provider_delete_secret(app: AppHandle, id: String) -> Result<(), AppError> {
    credentials::delete_secret(&app, &id).await
}

#[tauri::command]
pub async fn api_provider_has_secret(app: AppHandle, id: String) -> Result<bool, AppError> {
    credentials::has_secret(&app, &id).await
}

#[tauri::command]
pub async fn api_provider_test(
    app: AppHandle,
    id: String,
) -> Result<ProviderConnectionTestResult, AppError> {
    let profile = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        providers::get_provider(&conn, &id)?
    };
    let secret = credentials::get_secret(&app, &id).await?;
    let runtime = http::runtime_from_profile(&profile, secret);
    let result = http::test_connection(&runtime).await;
    {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        providers::record_test_result(&conn, &id, if result.success { "success" } else { "failed" })?;
    }
    Ok(result)
}

#[tauri::command]
pub async fn api_provider_list_models(app: AppHandle, id: String) -> Result<Vec<String>, AppError> {
    let profile = {
        let state = app.state::<DbState>();
        let conn = state.0.lock().map_err(lock_error)?;
        providers::get_provider(&conn, &id)?
    };
    let secret = credentials::get_secret(&app, &id).await?;
    let runtime = http::runtime_from_profile(&profile, secret);
    http::list_models(&runtime).await
}

#[tauri::command]
pub fn ai_feature_binding_get(
    state: State<'_, DbState>,
    feature_key: String,
) -> Result<Option<FeatureBinding>, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    providers::get_binding(&conn, &feature_key)
}

#[tauri::command]
pub fn ai_feature_binding_set(
    state: State<'_, DbState>,
    feature_key: String,
    input: BindingInput,
) -> Result<(), AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    providers::set_binding(&conn, &feature_key, input)
}

#[tauri::command]
pub fn api_usage_list(
    state: State<'_, DbState>,
    provider_profile_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<UsageLog>, AppError> {
    let conn = state.0.lock().map_err(lock_error)?;
    providers::list_usage(&conn, provider_profile_id.as_deref(), limit.unwrap_or(100))
}
