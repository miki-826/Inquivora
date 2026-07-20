# Inquivora

> **Inquivora — すべてを取り込み、ひとつにつなぐ。**
> Windows向け・ローカルファースト統合ワークスペース

ローカルファイル編集、会議音声の文字起こし、AI議事録、タスク、カレンダー、Windows通知をひとつのWindowsアプリへ統合します。

- 対象OS: Windows 11 x64
- データ方針: ローカルファースト（ファイル・タスク・予定・議事録・録音はPC内で処理・保存）
- AI方針: BYOK（ユーザー所有APIキー）。APIキーはWindows Credential Managerへ保存し、SQLite・ログ・エクスポートへは含めません

## インストール

1. [Releases](https://github.com/miki-826/Inquivora/releases) から `Inquivora_x.x.x_x64-setup.exe` をダウンロードします。
2. 実行すると Windows SmartScreen が「**Windows によって PC が保護されました**」「発行元不明のアプリ」と表示することがあります。これはアプリにコードサイニング証明書を付与していないためで、危険という意味ではありません。**［詳細情報］→［実行］** で続行できます（証明書の導入は今後対応予定）。
3. インストーラーは管理者権限を必要とせず、ユーザー領域（`%LOCALAPPDATA%\Inquivora`）へインストールします。

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
npm audit --omit=dev --audit-level=moderate
NODE_OPTIONS=--max-old-space-size=4096 npm run build
npm run test:rust
cd src-tauri && cargo clippy --all-targets -- -D warnings
dotnet test native/Inquivora.Native.sln --configuration Release
```

## データ保護と脅威モデル

- ワークスペースのファイル、SQLiteデータベース、文字起こし、録音WAV、保存失敗時の復旧ドラフトはPC内へ保存しますが、現時点ではアプリ独自の暗号化を行いません。Windowsのアカウント保護、BitLocker、端末ロックを利用してください。
- APIキーだけはWindows Credential Managerへ保存し、SQLite、ログ、復旧ドラフトへは保存しません。
- AI機能で外部Providerを使う場合は、実行前の画面にProvider、モデル、送信対象を表示します。内蔵Whisperを選んだ文字起こしはPC内で処理します。
- アプリのデータと録音をバックアップへ含めるか、保持期間をどうするかは利用者の運用に依存します。機密性の高い会議では、不要になった会議と録音をアプリから削除し、ごみ箱も確認してください。
- 配布物はSHA-256一覧を公開します。Authenticode署名と署名付き自動更新は未導入のため、正式公開前の対応項目です。

## リリース

正式配布用EXEはGitHub Actionsのみで生成します。ローカルの`npm run build:local`は開発確認専用の未検証ローカルビルドであり、配布しません。

```bash
npm run release:windows -- --version 0.1.0
```

このコマンドはpreflight検査（clean tree・テスト・秘密情報スキャン）→ GitHubへpush → remote SHAとlocal HEADの一致検証 → `v*`タグpushを行い、GitHub ActionsがセットアップEXEを生成します。

## ドキュメント

仕様は`docs/Inquivora_実装仕様書.md`を唯一の仕様とします。

## 実装状況

- ナビゲーション・検索・カレンダー操作改善 — v0.1.0-alpha.19: 選択中ナビゲーションを控えめな中立色へ変更し、ツール一覧の左側／上部配置を設定可能に変更。ファイルツリーやAIを含む左右パネルの開閉、Windows検索インデックスを利用したPC全体検索、カレンダー上で選択したタスクの状態・優先度・期日・時刻・色の左ペイン編集を追加
- 軽量化・UI再調整 — v0.1.0-alpha.18: ボタンデザインを全画面で統一し、タスク8色の選択状態を明瞭化。タスク色のカレンダー表示、メモ選択からのタスク作成、検索の閉じるボタン、エディタのスクロールバー非表示を実機確認しやすい形へ改善。画面遷移はホバー/フォーカス時の必要チャンクだけを先読みし、タスク・予定は楽観的更新、検索増分更新はファイルI/OをDBロック外へ移動。同じワークスペースの起動時に全再索引を繰り返さず、大量生成物の監視も除外してボタン応答を改善
- [x] Phase 1: Windowsアプリ基盤（アプリシェル・SQLite・トレイ常駐・リリースパイプライン）— v0.1.0-alpha.1でGitHub ActionsからのEXE生成・インストール・起動を検証済み。updaterプラグインの登録は自動更新を構成するPhase 7で行う
- [x] Phase 2: ワークスペース（フォルダ選択・仮想化ファイルツリー・Monaco Editorタブ・800ms自動保存＋アトミック保存・文字コード維持・Markdownプレビュー・外部変更検知と競合ダイアログ・パストラバーサル拒否）— v0.1.0-alpha.2
- [x] Phase 3: タスク・カレンダー（タスクCRUD・§12.1の期日順一覧と期限グループ表示・状態/優先度/プロジェクト/担当フィルター・FullCalendar月/週/日・予定CRUDとドラッグ日時変更・時刻付き/日付のみタスクのカレンダー表示・UTC⇔Asia/Tokyo変換）— v0.1.0-alpha.3
- [x] Phase 4: 通知（C# Sidecar notifyモード＝NDJSON §14.4・Windowsトースト・リマインダーCRUDとタスク/予定連動の既定リマインド・15秒tickスケジューラーとスリープ復帰再計算・`inquivora://`ディープリンクで通知クリック遷移・通知設定と自動起動・テスト通知）— v0.1.0-alpha.4。通知はWindows SDK標準のWindows.UI.Notificationsを使用（Windows App SDKの自己完結配置は単一ファイル発行と両立しないため）
- [x] Phase 5: API設定・録音・文字起こし（Provider Profile CRUD・APIキーはWindows Credential Managerのみ§10.4・接続テスト§10.6・モデル一覧取得＋手入力・用途別Feature Binding§10.5・C# Sidecar audioモード＝WASAPIマイク/ループバック録音を16kHzモノラルWAVで20秒チャンク＋1秒オーバーラップ保存§9.6・OpenAI互換Transcription API・リトライ付きジョブキュー§10.12・会議マーカー付きMarkdownへの継続追記＝開いているファイルはMonaco経由/閉じているファイルはRust経由§9.5）— v0.1.0-alpha.5。v0.1.0-alpha.6で内蔵Whisper（Whisper.net/whisper.cpp）によるローカル文字起こしを追加し、API未設定でも文字起こし可能（API Binding設定時はAPI優先。モデルはtiny/base/smallを設定画面からダウンロード、既定small）。v0.1.0-alpha.7でSidecar stdioのUTF-8固定（文字化け解消）・会議画面のEmbed風セグメントカード表示を追加。v0.1.0-alpha.8で録音デバイス選択（会議開始ダイアログでマイク/PC音声の入力デバイスを指定・選択は保存され次回復元）と文字起こし結果のU+FFFD除去を追加（Discord Webhook連携はalpha.7で追加後、不要のため削除）。v0.1.0-alpha.9で通知テスト連打によるクラッシュ対策（Sidecar多重起動の抑止）、フォルダツリーのドラッグ&ドロップ修正（`dragDropEnabled:false`でWindowsのHTML5 D&Dを有効化）、エクスプローラーからのファイル取込、ドラッグ中に現れるゴミ箱ゾーンへのドロップ削除を追加。なおツリーのファイルを外部アプリへドラッグ書き出しする機能はWebView2の制約で提供せず、右クリックの「エクスプローラーで表示」「パスをコピー」で代替する
- [x] Phase 6: AI議事録（会議の文字起こしから要約・決定事項・タスク候補・未確認事項を構造化生成§10.10・OpenAI互換chat/completionsでJSON出力を要求しスキーマ不一致時は1回だけ修復リクエスト・§11.4テンプレートで対象Markdownへ議事録ブロックを追記＝再生成時は置換・タスク候補は自動登録せず「タスク登録」で承認して正式タスク化§10.11・使用量記録）— v0.1.0-alpha.10。**議事録生成はAPI設定時のみ有効**（`meeting.summary`にProviderとモデルを割り当てた場合のみ。内蔵Whisperのみの環境では生成不可でローカルへフォールバックしない）
- [x] Phase 7: 検索・品質（全文検索＝SQLite FTS5 trigramで日本語部分一致・2文字以下はLIKEフォールバック§15.2／ファイル・議事録・タスク・予定を横断検索し種別で絞り込み・snippet表示・結果クリックで対象へ遷移／`search_global`・`search_reindex_workspace`§17.7／タスク・予定・会議・ファイルの変更で索引を即時更新§15.3／クラッシュ復旧＝起動時に中断された処理中ジョブをpendingへ戻す）— v0.1.0-alpha.11。**残: 自動更新（updaterプラグイン）は署名鍵とendpoint設定がユーザー準備待ちのためPhase 7時点では未組込み、E2Eは単体テストでカバー**
- セキュリティ・メモ連携 — v0.1.0-alpha.17: CSP、URL/ワークスペース境界、依存関係、Whisperモデルhash、API応答上限を強化。メモ選択からのタスク作成、8色タスク、カレンダーへのドラッグ、検索UI終了、エディタのスクロールバー非表示、PDF内蔵表示、AI Fallback・送信確認・長文分割要約・復旧ドラフトを追加
- UX改善 — v0.1.0-alpha.12: 全文検索を自動索引化（ワークスペースを開くと背景で索引・ファイル変更を監視して増分更新し、手動「再構築」ボタンを撤去）。索引処理はDBロック外でのファイル収集＋1トランザクション一括書込＋別スレッド実行に最適化しUIが固まらないよう改善。初心者向けにWhisperモデルの説明・推奨バッジ表示、会議開始時の文字起こし未準備ガイド、AI設定の文言平易化、縦ナビのラベル表示を追加
- 機能拡充 — v0.1.0-alpha.13: 会議の録音保存・書き出し（音源ごとにチャンクを重複除去して1つのWAVへ結合・録音フォルダを開く・会議削除時に録音も削除）、Whisperモデル管理（削除・ダウンロード済み合計容量）、エディタタブのドラッグ&ドロップ並び替え、インストール手順（SmartScreen警告の案内）を追加
- ダークモード・応答性 — v0.1.0-alpha.16: ダークモードを追加（設定＞外観でライト/ダーク/OS追従を選択・保存/復元、Monacoエディタもテーマ追従）、入力欄・選択欄・チェックボックスのデザインを統一（細枠・角丸・フォーカスリング）、起動後アイドル時に重い画面（Monaco等）を先読みして初回ナビの待ちを解消
- 応答性改善・デザイン — v0.1.0-alpha.15: 文字起こし中に固まる問題を解消——ローカルWhisper Sidecarを常駐化してモデルの毎回再読込を廃止し、推論スレッドをCPUの半分に制限＋プロセス優先度を下げてUIを優先、アイドル約30秒でSidecarを終了してメモリを解放。主要ボタン（会議開始・議事録生成など）と設定ボタンのデザインを刷新
- 機能拡充・軽量化 — v0.1.0-alpha.14: 録音をアプリ内で再生（全体を1トラックで通し再生・各発言を▶で個別再生）・削除できるように（asset protocol）、ワークスペース右ペインに会議パネルを実装（メモを取りながら文字起こしをライブ表示・開始/一時停止/終了）、エディタでCtrl+スクロールの文字拡大縮小。低スペック（8GB/Core i5第5世代）向けにピークメモリを削減——検索の全再構築をバッチ処理化、録音合成をi16省メモリ化、画面のコード分割（Monaco/FullCalendarを開いた画面だけ読込）
