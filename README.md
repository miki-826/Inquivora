# Inquivora

> **Inquivora — すべてを取り込み、ひとつにつなぐ。**
> Windows 向け・ローカルファーストの統合ワークスペース

会議の録音・文字起こし、AI議事録、メモ編集、タスク、カレンダー、通知を **1 つの Windows アプリ**にまとめました。データは基本的に PC 内で処理・保存されます。

- **対象 OS**: Windows 11 x64
- **データ方針**: ローカルファースト（ファイル・タスク・予定・議事録・録音はすべて PC 内で保存）
- **AI 方針**: BYOK（自分の API キーを使う方式）。文字起こしは API キーなしでも内蔵 Whisper でローカル実行できます

---

## 主な機能

### 📝 メモ・ワークスペース
- フォルダを開いてファイルを直接編集（Monaco エディタ／800ms 自動保存・文字コード保持）
- Markdown プレビュー、メモの左右 2 分割表示、タブ並び替え
- 外部変更の検知と競合ダイアログ

### 🎙 会議の録音・文字起こし
- マイクと PC 音声（スピーカー出力）を同時録音。入力デバイスは選択可能
- **内蔵 Whisper でローカル文字起こし**（API キー不要）。tiny / base / small モデルを設定画面からダウンロード
- 録音はアプリ内で再生・WAV 保存でき、文字起こしはそのまま Markdown へ追記

### 🤖 AI 議事録
- 文字起こしから **要約・決定事項・タスク候補・未確認事項**を自動生成
- 対応 AI: **ChatGPT（OpenAI） / Gemini（Google）**
- タスク候補は「タスク登録」を押したものだけ正式タスク化
- 文字起こし・要約は Markdown で保存可能
- ※議事録生成は AI（接続先）を設定したときのみ有効

### ✅ タスク・カレンダー
- タスクの作成・期日順一覧・状態／優先度／プロジェクト／担当フィルター・8 色分け
- FullCalendar の月／週／日表示、ドラッグで日時変更（Asia/Tokyo 基準）

### 🔔 通知
- タスク・予定のリマインダー（Windows トースト通知）
- **周期通知**（1 時間ごと／毎日／毎週 など）に対応
- 通知クリックで対象画面へ移動、自動起動・テスト通知にも対応

### 🔎 検索
- ファイル・議事録・タスク・予定を横断検索（SQLite FTS5・日本語部分一致）
- 種別で絞り込み、結果を選ぶと対象へ移動（ファイルはそのままメモで開く）

### 🎨 その他
- ダークモード（ライト／ダーク／OS 追従）
- サイドバー・パネルの開閉幅調整、低スペック PC 向けの軽量化

---

## インストール

1. [Releases](https://github.com/miki-826/Inquivora/releases) から `Inquivora_x.x.x_x64-setup.exe` をダウンロード。
2. 起動時に SmartScreen が「**Windows によって PC が保護されました**」と表示することがあります。コードサイニング証明書を付けていないためで、危険という意味ではありません。**［詳細情報］→［実行］** で続行できます。
3. 管理者権限は不要で、ユーザー領域（`%LOCALAPPDATA%\Inquivora`）にインストールされます。

## はじめかた（かんたんな流れ）

1. **ワークスペースを開く**（メモや会議メモを保存するフォルダを選択）
2. 文字起こしを使う → 設定 ＞ AI 設定で **Whisper モデルをダウンロード**（ローカル・無料）
3. AI 議事録を使う → 設定 ＞ AI 設定で **AI（ChatGPT / Gemini）と API キー・モデルを登録**
4. 「新しい会議を開始」で録音 → 文字起こし → 「議事録を生成」

---

## 技術スタック

| 分野 | 技術 |
|---|---|
| デスクトップ | Tauri 2 |
| フロントエンド | React + TypeScript + Vite |
| エディタ | Monaco Editor |
| カレンダー | FullCalendar |
| バックエンド | Rust + SQLite (FTS5) |
| 音声・通知 | .NET Sidecar（NAudio / Windows.UI.Notifications / Whisper.net） |

## 開発

```bash
npm install
npm run sidecar:build   # 初回と native/ 変更時に .NET Sidecar を発行
npm run tauri:dev
```

### 検証コマンド

```bash
npm run lint
npm run typecheck
npm test                # フロントエンド (vitest)
npm run test:rust       # Rust (cargo test)
dotnet test native/Inquivora.Native.sln
```

## データ保護と脅威モデル

- ファイル・SQLite・文字起こし・録音 WAV・保存失敗時の復旧ドラフトは PC 内に保存しますが、アプリ独自の暗号化は行いません。Windows アカウント保護・BitLocker・端末ロックを併用してください。
- **API キーは Windows Credential Manager にのみ保存**し、SQLite・ログ・エクスポートには含めません。
- 外部 AI を使うときは、実行前の画面に Provider・モデル・送信対象を表示します。内蔵 Whisper を選んだ文字起こしは PC 内で処理します。
- 機密性の高い会議は、不要になった会議・録音をアプリから削除し、ごみ箱も確認してください。
- 配布物は SHA-256 一覧を公開します。Authenticode 署名と署名付き自動更新は今後対応予定です。

## リリース

正式配布用 EXE は GitHub Actions のみで生成します（`npm run build:local` は開発確認専用で配布しません）。

```bash
npm run release:windows -- --version 0.1.0-alpha.X
```

preflight 検査（clean tree・テスト・秘密情報スキャン）→ GitHub へ push → remote SHA と local HEAD の一致検証 → `v*` タグ push を行い、GitHub Actions がセットアップ EXE を生成します。

## ドキュメント

- 仕様: `docs/Inquivora_実装仕様書.md`（唯一の仕様）
- 変更履歴: [Releases](https://github.com/miki-826/Inquivora/releases) を参照

## 第三者ライセンス

Inquivora は Monaco Editor を使用しています。Monaco Editor は Microsoft Corporation により MIT License で提供されています。著作権表示とライセンス全文は [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を参照してください。このファイルは Windows 配布パッケージにも同梱されます。

### 最近の更新（抜粋）

- **v0.1.0-alpha.28**: カレンダーの予定・タスクを大きく見やすく。Gemini の認証・出力設定を見直し（API エラーの理由もそのまま表示）。対応 AI を ChatGPT / Gemini の 2 種へ整理（Claude・Ollama を廃止）。メモの分割表示はタブのドラッグ＆ドロップで左右を入れ替え可能に。Ctrl+F の検索バーでツールチップが邪魔してボタンを押せない問題を修正。
- **v0.1.0-alpha.26**: タスクの周期通知に対応。検索したファイルをそのままメモで開くよう変更し、PC 全体検索を撤去・結果を左寄せに。Ollama のタイムアウトを延長し（ローカル推論で要約が失敗する問題を修正）、Gemini の対応モデルを現行版（gemini-2.5 系）へ更新。
- **v0.1.0-alpha.23**: AI 設定を Claude / ChatGPT / Gemini / Ollama の 4 種から選ぶだけに簡素化。メモの分割幅ドラッグ、アイコン刷新。
- **v0.1.0-alpha.16**: ダークモードを追加、入力欄デザインを統一、初回表示の先読みで待ち時間を短縮。
- **v0.1.0-alpha.14**: 録音のアプリ内再生／削除、ワークスペースの会議パネル、低スペック PC 向けの軽量化。
- **v0.1.0-alpha.6**: 内蔵 Whisper によるローカル文字起こしを追加（API キーなしでも利用可能）。

> Phase 1〜7（アプリ基盤・ワークスペース・タスク／カレンダー・通知・録音／文字起こし・AI 議事録・検索）は実装済みです。
