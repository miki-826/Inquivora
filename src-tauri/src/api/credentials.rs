#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_targetはprofile_idから決まる() {
        assert_eq!(credential_target("abc-123"), "Inquivora/API/abc-123");
    }

    #[test]
    fn setコマンドのjsonを構築できる() {
        let json = build_set_command("p1", "openai", "sk-secret");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["command"], "set");
        assert_eq!(value["target"], "Inquivora/API/p1");
        assert_eq!(value["userName"], "openai");
        assert_eq!(value["secret"], "sk-secret");
        assert!(!json.contains('\n'), "NDJSONは1行で構築する");
    }

    #[test]
    fn get_has_deleteコマンドを構築できる() {
        for (builder, command) in [
            (build_get_command as fn(&str) -> String, "get"),
            (build_has_command, "has"),
            (build_delete_command, "delete"),
        ] {
            let json = builder("p1");
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["command"], command);
            assert_eq!(value["target"], "Inquivora/API/p1");
        }
    }

    #[test]
    fn 応答行からsecretを取り出せる() {
        let parsed =
            parse_credential_response(r#"{"type":"credential.secret","secret":"sk-abc"}"#).unwrap();
        assert_eq!(parsed, CredentialResponse::Secret("sk-abc".to_string()));
    }

    #[test]
    fn 応答行のok_notfound_errorを判別できる() {
        assert_eq!(
            parse_credential_response(r#"{"type":"credential.ok"}"#).unwrap(),
            CredentialResponse::Ok
        );
        assert_eq!(
            parse_credential_response(r#"{"type":"credential.notFound"}"#).unwrap(),
            CredentialResponse::NotFound
        );
        match parse_credential_response(r#"{"type":"credential.error","code":"CRED_WRITE_FAILED","message":"x"}"#)
            .unwrap()
        {
            CredentialResponse::Error { code, .. } => assert_eq!(code, "CRED_WRITE_FAILED"),
            other => panic!("errorとして解釈されるべき: {other:?}"),
        }
    }

    #[test]
    fn 不正な応答行はエラーになる() {
        assert!(parse_credential_response("not json").is_err());
        assert!(parse_credential_response(r#"{"type":"unknown.event"}"#).is_err());
    }
}
