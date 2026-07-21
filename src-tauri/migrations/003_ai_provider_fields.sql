-- AI設定簡素化: プロバイダーごとに既定モデルと独自プロンプトを保持する。
ALTER TABLE api_provider_profiles ADD COLUMN model_id TEXT;
ALTER TABLE api_provider_profiles ADD COLUMN custom_prompt TEXT;
