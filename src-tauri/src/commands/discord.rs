use tauri::AppHandle;

use crate::api::credentials;
use crate::discord;
use crate::error::AppError;

#[tauri::command]
pub async fn discord_webhook_set(app: AppHandle, url: String) -> Result<(), AppError> {
    let validated = discord::validate_webhook_url(&url)?;
    credentials::set_secret(&app, discord::WEBHOOK_CREDENTIAL_ID, "discord", &validated).await
}

#[tauri::command]
pub async fn discord_webhook_has(app: AppHandle) -> Result<bool, AppError> {
    credentials::has_secret(&app, discord::WEBHOOK_CREDENTIAL_ID).await
}

#[tauri::command]
pub async fn discord_webhook_delete(app: AppHandle) -> Result<(), AppError> {
    credentials::delete_secret(&app, discord::WEBHOOK_CREDENTIAL_ID).await
}

#[tauri::command]
pub async fn discord_webhook_test(app: AppHandle) -> Result<(), AppError> {
    let url = credentials::get_secret(&app, discord::WEBHOOK_CREDENTIAL_ID)
        .await?
        .ok_or_else(|| {
            AppError::new(
                "DISCORD_NOT_CONFIGURED",
                "Webhook URLが未設定です。設定画面で登録してください",
                false,
            )
        })?;
    let embed = serde_json::json!({
        "title": "Inquivora テスト投稿",
        "description": "Discord連携が有効になりました。会議の文字起こしがこのチャンネルへ投稿されます。",
        "color": discord::EMBED_COLOR,
    });
    discord::post_webhook(&url, &[embed]).await
}
