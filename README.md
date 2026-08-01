# Inquivora

> すべてを取り込み、ひとつにつなぐ。

会議の録音・文字起こし・AI 議事録、メモ、タスク、カレンダー、通知を 1 つにまとめた、Windows 向けのローカルファースト・ワークスペースです。

[![CI](https://github.com/miki-826/Inquivora/actions/workflows/ci.yml/badge.svg)](https://github.com/miki-826/Inquivora/actions/workflows/ci.yml)
[![Windows Release](https://github.com/miki-826/Inquivora/actions/workflows/release-windows.yml/badge.svg)](https://github.com/miki-826/Inquivora/actions/workflows/release-windows.yml)

[Windows 版をダウンロード](https://github.com/miki-826/Inquivora/releases/tag/v0.1.0-alpha.42) · [リリース一覧](https://github.com/miki-826/Inquivora/releases) · [実装仕様](docs/Inquivora_実装仕様書.md)

## 名前に込めた意味

**Inquivora（インキボラ）**は、「すべてを取り込み、ひとつにつなぐ」というコンセプトから生まれた造語です。

- **Inqui-**: Inquiry / Inquire（問い、情報を探る）
- **-vora**: ラテン語由来の「食べる・取り込むもの」

会議音声、文字起こし、メモ、タスクなどを取り込み、1 つの情報基盤へ統合するアプリという意味を込めています。

## 特長

- **ローカルファースト** — メモ、タスク、予定、議事録、録音は PC 内に保存します。
- **会議を一気通貫で処理** — 録音、文字起こし、要約、決定事項・タスク候補の抽出までを同じ画面で扱えます。
- **API キーなしでも文字起こし** — 内蔵 Whisper の tiny / base / small モデルを使ってローカルで実行できます。
- **必要なときだけ外部 AI** — ChatGPT（OpenAI）または Gemini（Google）を BYOK 方式で利用できます。
- **日常業務を一元管理** — Monaco Editor を使ったメモ、タスク、カレンダー、通知、横断検索を統合しています。

## 主な機能

| 分野 | できること |
| --- | --- |
| メモ | フォルダ内のファイルを直接編集、Markdown プレビュー、左右分割、タブ並び替え、文字コード保持 |
| 保存 | 既定は入力後約 0.8 秒の自動保存。設定から手動保存へ切り替え、保存ボタンまたは `Ctrl+S` を利用可能 |
| 会議 | マイクと PC 音声の同時録音、入力デバイス選択、アプリ内再生、WAV 保存 |
| 文字起こし | 内蔵 Whisper によるローカル処理、または設定した AI 接続先による処理 |
| AI 議事録 | 要約、決定事項、タスク候補、未確認事項を生成。モデルと要約プロンプトは「議事録要約」で一元設定 |
| タスク | 状態・優先度・プロジェクト・担当による絞り込み、期日管理、8 色の色分け |
| カレンダー | 月・週・日表示、ドラッグによる日時変更、予定／タスクの表示切り替え（初期状態は両方表示） |
| 通知 | Windows トースト、周期通知、通知から対象画面への移動、自動起動、テスト通知 |
| 検索 | ファイル・議事録・タスク・予定の横断検索、種別による絞り込み |
| 表示 | ライト／ダーク／OS 追従、サイドバーとパネル幅の調整、低スペック PC 向け設定 |

## 動作環境とインストール

- Windows 11 x64
- 管理者権限は不要
- インストール先: `%LOCALAPPDATA%\Inquivora`

1. [v0.1.0-alpha.42 の Release](https://github.com/miki-826/Inquivora/releases/tag/v0.1.0-alpha.42) から `Inquivora_0.1.0-alpha.42_x64-setup.exe` をダウンロードします。
2. 必要に応じて、同じページの `SHA256SUMS.txt` でファイルを検証します。
3. インストーラーを実行します。コードサイニング証明書を付けていないため SmartScreen が表示される場合は、内容を確認して［詳細情報］→［実行］を選びます。

> 現在はアルファ版です。重要なデータは別途バックアップしてください。

## 最短の確認手順

1. ワークスペースとして使うフォルダを開き、メモを作成・編集します。
2. 設定でメモ帳の保存方法を「自動保存」または「手動保存」から選びます。既定値は自動保存です。
3. タスクと予定を作成し、カレンダー上部の「予定」「タスク」で表示を絞り込みます。
4. ローカル文字起こしを使う場合は、設定 ＞ AI 設定から Whisper モデルをダウンロードします。
5. AI 議事録を使う場合は ChatGPT または Gemini の接続先を登録し、「議事録要約」でモデルとプロンプトを設定します。
6. 「新しい会議を開始」から録音し、文字起こしと議事録生成を確認します。

## データとセキュリティ

- ファイル、SQLite データ、文字起こし、録音 WAV、復旧ドラフトは PC 内に保存します。アプリ独自の暗号化は行わないため、Windows アカウント保護、BitLocker、端末ロックを併用してください。
- API キーは Windows Credential Manager にのみ保存し、SQLite、ログ、エクスポートには含めません。
- 外部 AI の実行前には接続先、モデル、送信対象を表示します。内蔵 Whisper の文字起こしは PC 内で処理します。
- 機密性の高い会議は、不要になった会議・録音をアプリから削除し、ごみ箱も確認してください。
- 配布物には `SHA256SUMS.txt` を添付しています。Authenticode 署名と署名付き自動更新は今後の対応予定です。

## 技術スタック

| 分野 | 技術 |
| --- | --- |
| デスクトップ | Tauri 2 |
| フロントエンド | React、TypeScript、Vite |
| エディタ | Monaco Editor |
| カレンダー | FullCalendar |
| バックエンド | Rust、SQLite（FTS5） |
| 音声・通知 | .NET Sidecar、NAudio、Windows.UI.Notifications、Whisper.net |

## 開発

前提環境は Node.js、Rust、.NET SDK です。

```bash
npm install
npm run sidecar:build
npm run tauri:dev
```

### 検証

```bash
npm run lint
npm run typecheck
npm test
npm run test:rust
npm run test:dotnet
npm run build
```

正式配布用 EXE は GitHub Actions のみで生成します。

```bash
npm run release:windows -- --version 0.1.0-alpha.X
```

リリーススクリプトは、作業ツリー、バージョン整合、テスト、秘密情報を検査し、GitHub 上のコミットとローカルの一致を確認してからタグを作成します。

## ドキュメントとライセンス

- [実装仕様書](docs/Inquivora_実装仕様書.md) — 本プロジェクトの唯一の仕様書
- [リリース履歴](https://github.com/miki-826/Inquivora/releases)
- [第三者ライセンス概要](THIRD_PARTY_NOTICES.md) — 配布物・外部取得コンポーネントの適用範囲
- [Node.js 依存関係](THIRD_PARTY_LICENSES_NODE.txt) / [Rust 依存関係](THIRD_PARTY_LICENSES_RUST.txt) / [.NET・Whisper・NSIS](THIRD_PARTY_LICENSES_DOTNET.txt) — ライセンス全文と著作権表示（Windows 配布パッケージにも同梱）

プロジェクト本体にはオープンソースライセンスを設定していません。第三者ライセンスで許諾された部分を除き、リポジトリの公開だけを理由とする複製・改変・再配布の許諾はありません。

依存関係を更新した場合は、次のコマンドで配布用ライセンス一覧を再生成します。

```powershell
cargo install cargo-about --version 0.9.1 --locked --features cli
npm run licenses:generate
```
