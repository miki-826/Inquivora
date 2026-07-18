#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::open_database;
    use rusqlite::Connection;

    fn open_temp_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().expect("一時ディレクトリを作成できない");
        let conn = open_database(&dir.path().join("test.db")).expect("DBを開けない");
        (dir, conn)
    }

    fn sample_input(name: &str) -> ProviderInput {
        ProviderInput {
            display_name: name.to_string(),
            provider_type: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            auth_type: "bearer".to_string(),
            organization_id: None,
            project_id: None,
            default_headers: Default::default(),
            timeout_ms: 60000,
            capabilities: vec![
                "transcription.batch".to_string(),
                "models.list".to_string(),
            ],
        }
    }

    #[test]
    fn base_urlの末尾スラッシュは正規化される() {
        assert_eq!(
            normalize_base_url("https://api.openai.com/v1/").unwrap(),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            normalize_base_url("  https://api.openai.com/v1  ").unwrap(),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn httpはローカルホストのみ許可される() {
        assert!(normalize_base_url("http://localhost:1234/v1").is_ok());
        assert!(normalize_base_url("http://127.0.0.1:8080/v1").is_ok());
        assert!(normalize_base_url("http://[::1]:8080/v1").is_ok());
        let err = normalize_base_url("http://example.com/v1").unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn 非httpスキームは拒否される() {
        assert!(normalize_base_url("file:///c:/x").is_err());
        assert!(normalize_base_url("javascript:alert(1)").is_err());
        assert!(normalize_base_url("data:text/plain,x").is_err());
        assert!(normalize_base_url("api.openai.com/v1").is_err());
    }

    #[test]
    fn providerを作成して取得できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("OpenAI Personal")).unwrap();
        assert_eq!(created.display_name, "OpenAI Personal");
        assert_eq!(created.base_url, "https://api.openai.com/v1");
        assert!(created.enabled);
        let fetched = get_provider(&conn, &created.id).unwrap();
        assert_eq!(fetched.capabilities, created.capabilities);
    }

    #[test]
    fn 表示名の重複はvalidation_errorになる() {
        let (_dir, conn) = open_temp_db();
        create_provider(&conn, sample_input("A")).unwrap();
        let err = create_provider(&conn, sample_input("A")).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn カスタムヘッダーへ認証系ヘッダーは保存できない() {
        let (_dir, conn) = open_temp_db();
        let mut input = sample_input("X");
        input
            .default_headers
            .insert("Authorization".to_string(), "Bearer sk-abc".to_string());
        let err = create_provider(&conn, input).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
        let mut input2 = sample_input("Y");
        input2
            .default_headers
            .insert("x-api-key".to_string(), "abc".to_string());
        assert!(create_provider(&conn, input2).is_err());
    }

    #[test]
    fn provider一覧は表示名順で返る() {
        let (_dir, conn) = open_temp_db();
        create_provider(&conn, sample_input("B社ゲートウェイ")).unwrap();
        create_provider(&conn, sample_input("A社ゲートウェイ")).unwrap();
        let list = list_providers(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].display_name, "A社ゲートウェイ");
    }

    #[test]
    fn providerをパッチ更新できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        let patch: ProviderPatch = serde_json::from_str(
            r#"{"displayName":"改名","baseUrl":"http://localhost:1234/v1/","timeoutMs":30000,"organizationId":"org-1"}"#,
        )
        .unwrap();
        let updated = update_provider(&conn, &created.id, patch).unwrap();
        assert_eq!(updated.display_name, "改名");
        assert_eq!(updated.base_url, "http://localhost:1234/v1");
        assert_eq!(updated.timeout_ms, 30000);
        assert_eq!(updated.organization_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn パッチでorganization_idをnullへ戻せる() {
        let (_dir, conn) = open_temp_db();
        let mut input = sample_input("A");
        input.organization_id = Some("org-1".to_string());
        let created = create_provider(&conn, input).unwrap();
        let patch: ProviderPatch = serde_json::from_str(r#"{"organizationId":null}"#).unwrap();
        let updated = update_provider(&conn, &created.id, patch).unwrap();
        assert!(updated.organization_id.is_none());
    }

    #[test]
    fn 存在しないproviderはnot_foundになる() {
        let (_dir, conn) = open_temp_db();
        let err = get_provider(&conn, "missing").unwrap_err();
        assert_eq!(err.code, "API_PROVIDER_NOT_FOUND");
    }

    #[test]
    fn providerを削除できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        delete_provider(&conn, &created.id).unwrap();
        assert!(get_provider(&conn, &created.id).is_err());
    }

    #[test]
    fn 有効無効を切り替えられる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        set_provider_enabled(&conn, &created.id, false).unwrap();
        assert!(!get_provider(&conn, &created.id).unwrap().enabled);
    }

    #[test]
    fn 接続テスト結果を記録できる() {
        let (_dir, conn) = open_temp_db();
        let created = create_provider(&conn, sample_input("A")).unwrap();
        record_test_result(&conn, &created.id, "success").unwrap();
        let fetched = get_provider(&conn, &created.id).unwrap();
        assert_eq!(fetched.last_test_status.as_deref(), Some("success"));
        assert!(fetched.last_tested_at.is_some());
    }

    #[test]
    fn feature_bindingを設定して取得できる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        let input = BindingInput {
            provider_profile_id: Some(provider.id.clone()),
            model_id: Some("whisper-1".to_string()),
            fallback_provider_profile_id: None,
            fallback_model_id: None,
        };
        set_binding(&conn, "transcription.batch", input).unwrap();
        let binding = get_binding(&conn, "transcription.batch").unwrap().unwrap();
        assert_eq!(binding.provider_profile_id.as_deref(), Some(provider.id.as_str()));
        assert_eq!(binding.model_id.as_deref(), Some("whisper-1"));
    }

    #[test]
    fn feature_bindingは上書きできる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        let make = |model: &str| BindingInput {
            provider_profile_id: Some(provider.id.clone()),
            model_id: Some(model.to_string()),
            fallback_provider_profile_id: None,
            fallback_model_id: None,
        };
        set_binding(&conn, "meeting.summary", make("gpt-a")).unwrap();
        set_binding(&conn, "meeting.summary", make("gpt-b")).unwrap();
        let binding = get_binding(&conn, "meeting.summary").unwrap().unwrap();
        assert_eq!(binding.model_id.as_deref(), Some("gpt-b"));
    }

    #[test]
    fn 不正なfeature_keyは拒否される() {
        let (_dir, conn) = open_temp_db();
        let input = BindingInput {
            provider_profile_id: None,
            model_id: None,
            fallback_provider_profile_id: None,
            fallback_model_id: None,
        };
        let err = set_binding(&conn, "unknown.feature", input).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn provider削除でbindingのproviderはnullになる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        set_binding(
            &conn,
            "transcription.batch",
            BindingInput {
                provider_profile_id: Some(provider.id.clone()),
                model_id: Some("whisper-1".to_string()),
                fallback_provider_profile_id: None,
                fallback_model_id: None,
            },
        )
        .unwrap();
        delete_provider(&conn, &provider.id).unwrap();
        let binding = get_binding(&conn, "transcription.batch").unwrap().unwrap();
        assert!(binding.provider_profile_id.is_none());
    }

    #[test]
    fn 使用量ログを記録して新しい順に取得できる() {
        let (_dir, conn) = open_temp_db();
        let provider = create_provider(&conn, sample_input("A")).unwrap();
        for (i, status) in ["success", "error"].iter().enumerate() {
            insert_usage(
                &conn,
                UsageInput {
                    provider_profile_id: provider.id.clone(),
                    feature_key: "transcription.batch".to_string(),
                    model_id: "whisper-1".to_string(),
                    entity_id: Some(format!("m-{i}")),
                    input_units: None,
                    output_units: None,
                    audio_duration_ms: Some(20000),
                    latency_ms: Some(800),
                    status: status.to_string(),
                    error_code: if *status == "error" {
                        Some("API_TIMEOUT".to_string())
                    } else {
                        None
                    },
                },
            )
            .unwrap();
        }
        let logs = list_usage(&conn, Some(&provider.id), 10).unwrap();
        assert_eq!(logs.len(), 2);
        let errors: Vec<_> = logs.iter().filter(|l| l.status == "error").collect();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_code.as_deref(), Some("API_TIMEOUT"));
    }
}
