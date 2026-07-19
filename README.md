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
npm run sidecar:build
npm run tauri:dev
```

`sidecar:build`は.NET Sidecarを発行して`src-tauri/binaries/`へ配置します（初回と`native/`変更時に実行）。

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
- [x] Phase 2: ワークスペース（フォルダ選択・仮想化ファイルツリー・Monaco Editorタブ・800ms自動保存＋アトミック保存・文字コード維持・Markdownプレビュー・外部変更検知と競合ダイアログ・パストラバーサル拒否）— v0.1.0-alpha.2
- [x] Phase 3: タスク・カレンダー（タスクCRUD・§12.1の期日順一覧と期限グループ表示・状態/優先度/プロジェクト/担当フィルター・FullCalendar月/週/日・予定CRUDとドラッグ日時変更・時刻付き/日付のみタスクのカレンダー表示・UTC⇔Asia/Tokyo変換）— v0.1.0-alpha.3
- [x] Phase 4: 通知（C# Sidecar notifyモード＝NDJSON §14.4・Windowsトースト・リマインダーCRUDとタスク/予定連動の既定リマインド・15秒tickスケジューラーとスリープ復帰再計算・`inquivora://`ディープリンクで通知クリック遷移・通知設定と自動起動・テスト通知）— v0.1.0-alpha.4。通知はWindows SDK標準のWindows.UI.Notificationsを使用（Windows App SDKの自己完結配置は単一ファイル発行と両立しないため）
- [x] Phase 5: API設定・録音・文字起こし（Provider Profile CRUD・APIキーはWindows Credential Managerのみ§10.4・接続テスト§10.6・モデル一覧取得＋手入力・用途別Feature Binding§10.5・C# Sidecar audioモード＝WASAPIマイク/ループバック録音を16kHzモノラルWAVで20秒チャンク＋1秒オーバーラップ保存§9.6・OpenAI互換Transcription API・リトライ付きジョブキュー§10.12・会議マーカー付きMarkdownへの継続追記＝開いているファイルはMonaco経由/閉じているファイルはRust経由§9.5）— v0.1.0-alpha.5。v0.1.0-alpha.6で内蔵Whisper（Whisper.net/whisper.cpp）によるローカル文字起こしを追加し、API未設定でも文字起こし可能（API Binding設定時はAPI優先。モデルはtiny/base/smallを設定画面からダウンロード、既定small）。v0.1.0-alpha.7でSidecar stdioのUTF-8固定（文字化け解消）・会議画面のEmbed風セグメントカード表示を追加。v0.1.0-alpha.8で録音デバイス選択（会議開始ダイアログでマイク/PC音声の入力デバイスを指定・選択は保存され次回復元）と文字起こし結果のU+FFFD除去を追加（Discord Webhook連携はalpha.7で追加後、不要のため削除）。v0.1.0-alpha.9で通知テスト連打によるクラッシュ対策（Sidecar多重起動の抑止）、フォルダツリーのドラッグ&ドロップ修正（`dragDropEnabled:false`でWindowsのHTML5 D&Dを有効化）、エクスプローラーからのファイル取込、ドラッグ中に現れるゴミ箱ゾーンへのドロップ削除を追加。なおツリーのファイルを外部アプリへドラッグ書き出しする機能はWebView2の制約で提供せず、右クリックの「エクスプローラーで表示」「パスをコピー」で代替する
- [ ] Phase 6: AI議事録
- [ ] Phase 7: 検索・品質
