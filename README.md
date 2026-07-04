# Inquivora

> **Inquivora — すべてを取り込み、ひとつにつなぐ。**
> Windows向け・ローカルファースト統合ワークスペース

ローカルファイル編集、会議音声の文字起こし、AI議事録、タスク、カレンダー、Windows通知をひとつのWindowsアプリへ統合します。

- 対象OS: Windows 11 x64
- データ方針: ローカルファースト（ファイル・タスク・予定・議事録・録音はPC内で処理・保存）
- AI方針: BYOK（ユーザー所有APIキー）。APIキーはWindows Credential Managerへ保存し、SQLite・ログ・エクスポートへは含めません

## 技術スタック

| 分野 | 技術 |
|---|---|
| デスクトップ | Tauri 2 |
| フロントエンド | React + TypeScript + Vite |
| エディタ | Monaco Editor |
| カレンダー | FullCalendar |
| バックエンド | Rust + SQLite (FTS5) |
| 音声・通知 | .NET 8 Sidecar (NAudio / Windows App SDK) |

## 開発

```bash
npm install
npm run tauri:dev
```

### 検証コマンド

```bash
npm run lint
npm run typecheck
npm test
npm run test:rust
```

## リリース

正式配布用EXEはGitHub Actionsのみで生成します。ローカルの`npm run build:local`は開発確認専用の未検証ローカルビルドであり、配布しません。

```bash
npm run release:windows -- --version 0.1.0
```

このコマンドはpreflight検査（clean tree・テスト・秘密情報スキャン）→ GitHubへpush → remote SHAとlocal HEADの一致検証 → `v*`タグpushを行い、GitHub ActionsがセットアップEXEを生成します。

## ドキュメント

仕様は`docs/Inquivora_実装仕様書.md`を唯一の仕様とします。

## 実装状況

- [x] Phase 1: Windowsアプリ基盤（アプリシェル・SQLite・トレイ常駐・リリースパイプライン）— v0.1.0-alpha.1でGitHub ActionsからのEXE生成・インストール・起動を検証済み。updaterプラグインの登録は自動更新を構成するPhase 7で行う
- [ ] Phase 2: ワークスペース（ファイルツリー・Monaco Editor・自動保存）
- [ ] Phase 3: タスク・カレンダー
- [ ] Phase 4: 通知
- [ ] Phase 5: API設定・録音・文字起こし
- [ ] Phase 6: AI議事録
- [ ] Phase 7: 検索・品質
