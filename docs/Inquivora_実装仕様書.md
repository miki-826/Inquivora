# Inquivora 実装仕様書

> **Inquivora — すべてを取り込み、ひとつにつなぐ。**  
> Windows向け・ローカルファースト統合ワークスペース

---

## 0. 文書情報

| 項目 | 内容 |
|---|---|
| 文書名 | Inquivora 実装仕様書 |
| 対象バージョン | MVP 0.1.0 |
| 仕様書改訂 | v1.3（セキュリティ強化・メモ連携タスク・色分けカレンダー対応） |
| 対象OS | Windows 11 x64 |
| 配布形式 | NSISセットアップEXE |
| データ方針 | ローカルファースト |
| AI方針 | BYOK（ユーザー所有APIキー）・必要な処理だけ外部APIへ送信 |
| UI言語 | 日本語 |
| タイムゾーン | Asia/Tokyo |

本書は、Claude Code、Codex、GitHub Copilot、または開発者へ渡し、そのまま実装へ着手できる粒度を目標とする。

---

# 1. プロダクト定義

## 1.1 コンセプト

Inquivoraは、次の情報を一つのWindowsアプリへ取り込んで関連付ける。

- ローカルファイル
- Markdown・テキスト・ソースコード
- 会議音声
- PC再生音声
- 文字起こし
- 議事録
- タスク
- 予定
- Windows通知
- AI要約・抽出結果

## 1.2 中核価値

```text
ローカルファイルを開く
        ↓
開いているファイルへ文字起こしを追記
        ↓
議事録・決定事項・タスク候補を生成
        ↓
タスクを期日順で管理
        ↓
カレンダーへ表示
        ↓
Windows通知でリマインド
```

## 1.3 ローカルファーストの定義

以下はPC内だけで処理・保存する。

- ファイル編集
- ファイルツリー
- タスク
- カレンダー
- 予定
- 議事録一覧
- 録音データ
- SQLiteデータ
- 全文検索インデックス
- 通知スケジュール
- ユーザー設定

外部APIへ送信してよいものは、ユーザーが許可した次のデータのみとする。

- 文字起こし対象の音声
- 議事録生成対象の文字起こし
- ユーザーが明示的にAI処理を実行した選択テキスト

## 1.4 MVPの非対象

- VS Code拡張機能互換
- Git GUI
- ターミナル・デバッガー
- Word・Excel・PowerPointの編集
- 複数端末同期
- 共同編集
- Web版
- スマートフォン版
- 組織アカウント管理
- 完全な話者識別
- Google Calendar・Outlook同期
- ローカルLLM

---

# 2. 採用アーキテクチャ

## 2.1 全体構成

```text
┌──────────────────────────────────────────────┐
│ Inquivora.exe                                │
│ Tauri 2                                      │
│                                              │
│ React / TypeScript                           │
│ ├─ ワークスペース                            │
│ ├─ Monaco Editor                             │
│ ├─ 議事録一覧                                │
│ ├─ タスク一覧                                │
│ ├─ FullCalendar                              │
│ └─ AI・会議パネル                            │
│                                              │
│ Rust                                         │
│ ├─ ファイル操作                              │
│ ├─ SQLite                                    │
│ ├─ ファイル監視                              │
│ ├─ 検索インデックス                          │
│ ├─ 通知スケジューラー                        │
│ ├─ APIジョブ管理                             │
│ └─ Sidecar制御                               │
└───────────────────┬──────────────────────────┘
                    │ stdin/stdout: NDJSON
┌───────────────────▼──────────────────────────┐
│ inquivora-native.exe                         │
│ .NET 8 / C#                                  │
│ ├─ マイク録音                                │
│ ├─ WASAPIループバック録音                    │
│ ├─ Windowsネイティブ通知                     │
│ ├─ Credential Manager                        │
│ └─ オーディオデバイス監視                    │
└──────────────────────────────────────────────┘
                    │ HTTPS / WebSocket
┌───────────────────▼──────────────────────────┐
│ AI Provider Registry                         │
│ ├─ ユーザーが設定したProvider Profile         │
│ ├─ 音声文字起こしAPI                         │
│ ├─ 議事録・タスク抽出API                     │
│ └─ OpenAI互換・ローカルAPI Endpoint           │
└──────────────────────────────────────────────┘
```

## 2.2 採用技術

| 分野 | 技術 |
|---|---|
| デスクトップ | Tauri 2 |
| フロントエンド | React + TypeScript + Vite |
| ルーティング | React Router |
| 状態管理 | Zustand |
| 入力検証 | Zod |
| エディタ | Monaco Editor |
| Markdown | react-markdown + remark-gfm |
| カレンダー | FullCalendar React |
| 仮想リスト | TanStack Virtual |
| バックエンド | Rust |
| DB | SQLite + rusqlite |
| 全文検索 | SQLite FTS5 trigram |
| ファイル監視 | notify crate |
| 音声 | .NET 8 + NAudio |
| Windows通知 | Windows App SDK App Notifications |
| API通信 | reqwest / OpenAI公式SDK相当 |
| ログ | tracing + tracing-appender |
| ID | UUID v7またはUUID v4 |
| 日時 | UTC保存 + IANA timezone |
| インストーラー | Tauri NSIS |
| ソース管理 | GitHub（指定リポジトリ） |
| CI/CD | GitHub Actions |

## 2.3 採用理由

- TauriはWindows向けEXEとセットアップEXEを作成できる。
- Monaco EditorでVS Codeに近い編集体験を構築できる。
- FullCalendarで月・週・日表示を統一できる。
- SQLiteによりサーバーなしでタスク・予定・議事録を保存できる。
- NAudioによりマイクとWASAPIループバックを実装しやすい。
- Windows App SDKでWindows通知と通知操作を扱う。

---

# 3. 開発環境と初期構築

## 3.1 必須環境

- Windows 11 x64
- Node.js LTS
- npm
- Rust stable
- Visual Studio 2022 Build Tools
  - Desktop development with C++
  - Windows 11 SDK
- .NET 8 SDK
- Git
- WebView2 Runtime

## 3.2 プロジェクト作成

```bash
npm create tauri-app@latest inquivora
cd inquivora
npm install
```

テンプレート選択：

```text
TypeScript
React
npm
```

## 3.3 フロントエンド依存関係

```bash
npm install \
  react-router-dom \
  zustand \
  zod \
  @monaco-editor/react \
  react-markdown \
  remark-gfm \
  @fullcalendar/core \
  @fullcalendar/react \
  @fullcalendar/daygrid \
  @fullcalendar/timegrid \
  @fullcalendar/interaction \
  @tanstack/react-virtual \
  date-fns \
  date-fns-tz \
  lucide-react
```

## 3.4 Tauriプラグイン

```bash
npm run tauri add dialog
npm run tauri add fs
npm run tauri add opener
npm run tauri add shell
npm run tauri add store
npm run tauri add autostart
npm run tauri add single-instance
npm run tauri add global-shortcut
npm run tauri add updater
```

## 3.5 Rust依存関係

`src-tauri/Cargo.toml`へ追加する。

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.32", features = ["bundled"] }
notify = "8"
walkdir = "2"
uuid = { version = "1", features = ["v4", "v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = "0.10"
encoding_rs = "0.8"
chardetng = "0.1"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-appender = "0.2"
tracing-subscriber = "0.3"
reqwest = { version = "0.12", features = ["json", "multipart", "stream"] }
sha2 = "0.10"
base64 = "0.22"
```

## 3.6 C# Sidecar

```bash
dotnet new console -n Inquivora.Native -f net8.0
cd Inquivora.Native
dotnet add package NAudio
dotnet add package Microsoft.WindowsAppSDK
dotnet add package System.CommandLine
```

Sidecarは自己完結・単一ファイルとして発行する。

```bash
dotnet publish -c Release -r win-x64 \
  --self-contained true \
  -p:PublishSingleFile=true
```

---


## 3.6 GitHubリポジトリ・リリース設定

正式なWindows配布物は、開発PC上の未共有コードから直接生成してはならない。  
指定したGitHubリポジトリへソースコードをpushし、GitHub上のコミットSHAとローカルHEADが一致したことを確認してから、GitHub ActionsでEXEを生成する。

### リポジトリ設定ファイル

リポジトリ直下へ`release.config.json`を置く。秘密情報は含めない。

```json
{
  "repositoryUrl": "https://github.com/<OWNER>/<REPOSITORY>.git",
  "branch": "main",
  "releaseTagPrefix": "v",
  "buildMode": "github-actions",
  "artifactRetentionDays": 30
}
```

### 設定ルール

- `repositoryUrl`はユーザーが指定したGitHubリポジトリとする。
- `branch`は正式リリース元ブランチとし、既定値は`main`とする。
- 設定変更時は、対象URLがGitHubリポジトリとして取得可能か確認する。
- Personal Access Token、SSH秘密鍵、APIキー、署名パスワードは設定ファイルへ保存しない。
- GitHub認証はGit Credential Manager、GitHub CLI、またはSSH Agentへ委譲する。
- AI ProviderのAPIキーはGitHubへ一切送信しない。

### 必須`.gitignore`

```gitignore
# Node / frontend
node_modules/
dist/

# Rust / Tauri build
src-tauri/target/

# .NET Sidecar build
sidecars/**/bin/
sidecars/**/obj/

# Local data and secrets
.env
.env.*
!.env.example
*.db
*.db-shm
*.db-wal
*.sqlite
*.sqlite3
*.wav
*.mp3
*.m4a
*.log
logs/
recordings/
credentials/
local-data/

# Signing material
*.pfx
*.p12
*.pem
*.key
*.cer
*.crt

# IDE / OS
.vscode/settings.local.json
.idea/
.DS_Store
Thumbs.db
```

### リポジトリへ含めるもの

- React・TypeScriptソース
- Rustソース
- C# Sidecarソース
- DB Migration
- テスト
- GitHub Actions Workflow
- `release.config.json`
- `.env.example`
- README・仕様書
- アイコンなど再配布可能なアセット

### リポジトリへ含めないもの

- APIキー
- Windows Credential Managerの内容
- 実ユーザーのSQLite DB
- 録音・文字起こし実データ
- ログ
- コード署名証明書の実体
- ビルド済み`target`・`bin`・`obj`

# 4. ディレクトリ構成

```text
inquivora/
├─ src/
│  ├─ app/
│  │  ├─ App.tsx
│  │  ├─ router.tsx
│  │  └─ AppShell.tsx
│  ├─ components/
│  │  ├─ layout/
│  │  ├─ common/
│  │  ├─ dialogs/
│  │  └─ statusbar/
│  ├─ features/
│  │  ├─ workspace/
│  │  ├─ editor/
│  │  ├─ search/
│  │  ├─ meetings/
│  │  ├─ transcription/
│  │  ├─ tasks/
│  │  ├─ calendar/
│  │  ├─ notifications/
│  │  ├─ ai/
│  │  └─ settings/
│  ├─ stores/
│  ├─ services/
│  ├─ schemas/
│  ├─ types/
│  ├─ hooks/
│  ├─ styles/
│  └─ assets/
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ lib.rs
│  │  ├─ commands/
│  │  ├─ database/
│  │  ├─ files/
│  │  ├─ watcher/
│  │  ├─ search/
│  │  ├─ meetings/
│  │  ├─ tasks/
│  │  ├─ calendar/
│  │  ├─ notifications/
│  │  ├─ ai/
│  │  ├─ sidecar/
│  │  ├─ security/
│  │  └─ logging/
│  ├─ migrations/
│  │  └─ 001_init.sql
│  ├─ capabilities/
│  ├─ binaries/
│  │  └─ inquivora-native-x86_64-pc-windows-msvc.exe
│  └─ tauri.conf.json
├─ native/
│  └─ Inquivora.Native/
│     ├─ Audio/
│     ├─ Notifications/
│     ├─ Credentials/
│     ├─ Protocol/
│     └─ Program.cs
├─ tests/
│  ├─ unit/
│  ├─ integration/
│  └─ e2e/
├─ docs/
└─ .github/workflows/
```

---

# 5. アプリ画面構成

## 5.1 共通レイアウト

```text
┌──────────────────────────────────────────────────────────┐
│ タイトルバー                                             │
├──────────────────────────────────────────────────────────┤
│ メニュー・ツールバー                                     │
├───┬──────────────────────────────────────┬───────────────┤
│縦 │ 左パネル / 中央メインコンテンツ       │ 右詳細パネル  │
│ナ │                                      │               │
│ビ │                                      │               │
├───┴──────────────────────────────────────┴───────────────┤
│ ステータスバー                                           │
└──────────────────────────────────────────────────────────┘
```

## 5.2 縦ナビゲーション

| ID | 画面 | アイコン |
|---|---|---|
| workspace | ワークスペース | FolderTree |
| search | 全文検索 | Search |
| meetings | 議事録 | FileAudio |
| tasks | タスク | SquareCheckBig |
| calendar | カレンダー | CalendarDays |
| settings | 設定 | Settings |

## 5.3 画面ルート

```text
/workspace
/search
/meetings
/tasks
/calendar
/settings
```

## 5.4 ワークスペース画面

- 左：ファイルツリー
- 中央：タブ＋Monaco Editorまたはプレビュー
- 右：AI・会議パネル

## 5.5 議事録画面

- 左：取得日時の新しい順の議事録一覧
- 中央：関連ファイル
- 右：要約・決定事項・タスク候補・音声

## 5.6 タスク画面

- 左：今日・今週・期限切れ・完了・期限カレンダー
- 中央：期日昇順のタスク一覧
- 右：タスク詳細・通知・メモ

## 5.7 カレンダー画面

カレンダー画面では中央領域全体をカレンダーへ切り替える。

- 月・週・日表示
- 左：期日なしの未完了タスク（カレンダーへドラッグ可能）
- 右：選択日の予定・タスク詳細
- エディタタブは非表示
- 左ファイルツリーは非表示

---

# 6. UIデザイン仕様

## 6.1 カラートークン

```css
:root {
  --color-bg: #f6f8fc;
  --color-surface: #ffffff;
  --color-surface-subtle: #f8fafc;
  --color-border: #dce3ef;
  --color-text: #172033;
  --color-text-muted: #667085;
  --color-primary: #2563eb;
  --color-primary-hover: #1d4ed8;
  --color-primary-soft: #eaf2ff;
  --color-success: #16a34a;
  --color-warning: #f59e0b;
  --color-danger: #ef4444;
  --color-purple: #7c3aed;
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
}
```

## 6.2 サイズ

| 要素 | 初期値 |
|---|---:|
| 縦ナビ | 52px |
| 左サイドバー | 320px |
| 右サイドバー | 360px |
| ツールバー | 48px |
| ステータスバー | 28px |
| 最小ウィンドウ幅 | 1100px |
| 最小ウィンドウ高 | 700px |

すべてのサイドバーはドラッグでリサイズ可能とし、幅は設定へ保存する。

## 6.3 テーマ

MVPはライトテーマを必須とする。ダークテーマは拡張機能扱いとするが、CSSトークンはテーマ切替可能な設計にする。

---

# 7. ワークスペース・ファイル管理

## 7.1 ワークスペース

ユーザーが選択した任意のフォルダをワークスペースルートとする。

```text
例：C:\Users\miki\Documents\InquivoraWorkspace
```

最近開いたワークスペースを最大10件保存する。

## 7.2 必須操作

- ワークスペースを開く
- 新しいワークスペースを作成
- ファイル作成
- フォルダ作成
- 名前変更
- 削除
- コピー
- 移動
- ドラッグ＆ドロップ
- ファイルパスのコピー
- エクスプローラーで表示
- 既定アプリで開く
- 更新

## 7.3 ファイルツリー

- フォルダは遅延読み込みする。
- 展開状態を保存する。
- 50,000件を想定し、全件DOM描画しない。
- 無視対象を設定可能にする。

初期無視設定：

```text
.git
node_modules
target
dist
build
.next
.venv
__pycache__
```

## 7.4 ファイル種別

### 編集対象

```text
md txt log csv json jsonl yaml yml xml ini conf env
html css scss js jsx ts tsx py ps1 bat sh sql rs cs java
```

### プレビュー対象

```text
png jpg jpeg gif webp svg pdf wav mp3 m4a mp4 webm
```

PDFはアプリ内の読み取り専用ビューで表示する。

### 外部アプリ対象

```text
docx xlsx pptx dwg zip 7z exe dll
```

## 7.5 テキスト判定

未知拡張子は先頭8KBを読み、次の条件で判定する。

- NULバイトを含む：バイナリ
- UTF-8として妥当：テキスト
- Shift_JIS等として検出可能：テキスト
- 制御文字比率が高い：バイナリ

## 7.6 文字コード

対応：

- UTF-8
- UTF-8 BOM
- UTF-16 LE
- UTF-16 BE
- Shift_JIS

保存時は元文字コードと改行コードを維持する。

## 7.7 安全な保存

```text
1. 同一ディレクトリへ一時ファイル作成
2. 内容を書き込み
3. flush / fsync
4. 元ファイルをバックアップまたは置換
5. 一時ファイル削除
```

## 7.8 外部変更

ファイル監視で外部変更を検出する。

未保存変更がない場合：自動再読込。  
未保存変更がある場合：次の選択肢を表示する。

- 外部内容を読み込む
- 現在の内容で上書き
- 差分を表示
- 別名保存

---

# 8. エディタ

## 8.1 Monaco機能

- 複数タブ
- 行番号
- シンタックスハイライト
- 検索・置換
- Undo / Redo
- 自動インデント
- 折りたたみ
- ミニマップ切替
- 行・列表示
- ワードラップ
- 複数カーソル
- 未保存インジケーター
- Markdownプレビュー
- 差分表示
- Ctrl+F検索ウィジェットは×、Escのどちらでも閉じられる
- 縦横スクロールは維持し、エディタ内のスクロールバーだけを非表示にする
- 選択テキストの右クリックメニューから、確認・編集ダイアログを経てタスク化できる

## 8.2 タブモデル

```ts
export type EditorTab = {
  id: string;
  path: string;
  name: string;
  language: string;
  encoding: FileEncoding;
  lineEnding: "LF" | "CRLF";
  isDirty: boolean;
  isPinned: boolean;
  cursorLine: number;
  cursorColumn: number;
  viewType: "editor" | "markdown-preview" | "image" | "pdf" | "audio" | "video";
};
```

## 8.3 自動保存

- 既定：有効
- デバウンス：800ms
- エラー時：タブに赤い状態表示
- 失敗した内容：リカバリ領域へ保存

## 8.4 大容量ファイル

| サイズ | 動作 |
|---:|---|
| 0〜10MB | 通常編集 |
| 10〜100MB | 読み取り専用大容量モード |
| 100MB超 | 先頭・末尾プレビューのみ |

---

# 9. 会議・録音・文字起こし

## 9.1 会議開始ダイアログ

入力項目：

- 会議タイトル
- 文字起こし先ファイル
- 追記位置
- マイク録音 ON/OFF
- PC音声録音 ON/OFF
- 音声保存 ON/OFF
- 会議終了後AI議事録生成 ON/OFF

追記位置：

- カーソル位置
- ファイル末尾
- `## 文字起こし`セクション
- 新規Markdownファイル

## 9.2 文字起こし対象ファイル

推奨：`.md`または`.txt`。

非テキストファイルを開いている場合は、新規Markdownファイル作成を既定選択とする。

## 9.3 会議ファイルのマーカー

既存ファイルへ追記する場合、次のHTMLコメントを使用する。

```md
<!-- inquivora:meeting:MEETING_ID:start -->

## 文字起こし

### 10:02 自分

発言内容

<!-- inquivora:meeting:MEETING_ID:end -->
```

一覧から開く際は開始マーカーを検索してスクロールする。

## 9.4 文字起こし追記ルール

- 暫定文字列は右パネルだけに表示する。
- 確定文字列だけファイルへ書き込む。
- 1発言単位で追加する。
- 発言時刻と音源を付ける。
- 自分のマイクは`自分`、ループバックは`PC音声`と表記する。

```md
### 10:03 PC音声

8月から試験導入を開始します。
```

## 9.5 競合防止

対象ファイルが開いている場合：

```text
文字起こし確定
  ↓
フロントエンドイベント
  ↓
Monaco Modelへ挿入
  ↓
自動保存
```

対象ファイルが閉じている場合：

```text
文字起こし確定
  ↓
Rustでファイルロック
  ↓
終了マーカー直前へ挿入
  ↓
安全な保存
```

## 9.6 録音

### 音源

- マイク
- Windows既定出力のループバック

### API送信用形式

- PCM WAV
- 16kHzまたは24kHz
- mono
- 16bit

### チャンク

MVPではリアルタイムWebSocketではなく、安定したチャンク方式を採用する。

- チャンク長：20秒
- オーバーラップ：1秒
- 無音チャンクは送らない
- API失敗時はローカルキューへ保存

### 文字重複除去

オーバーラップ部分は次の条件で除去する。

- 前セグメント末尾と新セグメント先頭の文字列類似度
- 時刻の重なり
- 完全一致または一定以上の類似語列

## 9.7 Sidecar音声プロトコル

### 起動

```text
inquivora-native.exe audio --session SESSION_ID
```

### stdinコマンド

```json
{"command":"start","micDeviceId":"default","loopbackDeviceId":"default","chunkSeconds":20,"outputDir":"..."}
{"command":"pause"}
{"command":"resume"}
{"command":"stop"}
{"command":"listDevices"}
```

### stdoutイベント

```json
{"type":"audio.started","sessionId":"..."}
{"type":"audio.level","source":"mic","rms":0.42}
{"type":"audio.chunk","source":"mic","path":"...wav","startMs":0,"endMs":20000}
{"type":"audio.deviceLost","source":"loopback","deviceId":"..."}
{"type":"audio.error","code":"CAPTURE_FAILED","message":"..."}
{"type":"audio.stopped","sessionId":"..."}
```

1行1JSONのNDJSONとする。ログはstderrへ出し、stdoutへ混在させない。

## 9.8 録音障害

- デバイス切断：再接続を試行
- 出力デバイス変更：新しい既定デバイスへ切替確認
- スリープ：セッションを一時停止し、復帰後再開
- Sidecar終了：UIへエラー表示し、再起動可能にする
- API停止：録音を継続し、後から再送する

---

# 10. AI・API Provider設定

## 10.1 基本方針

InquivoraはBYOK（Bring Your Own Key）方式とし、APIキーをアプリへ埋め込まない。
ユーザーは設定画面から自分のProvider、APIキー、エンドポイント、モデルを登録する。

MVPでGUI設定を保証するProvider種別：

1. `openai`
   - OpenAI公式API用プリセット
2. `openai_compatible`
   - OpenAI互換API
   - 社内ゲートウェイ、LM Studio、互換Whisperサーバー等を想定
3. `none`
   - 当該機能を無効化

Anthropic、Gemini、Azure OpenAI等は、Provider Adapterを追加して拡張できる構造とする。
完全に任意のJSON形式をGUIだけで接続する「汎用RESTビルダー」はMVP対象外とする。

ユーザーは用途ごとに別Providerを選択できる。

```text
文字起こし（バッチ）       → Provider A / Model A
文字起こし（リアルタイム） → Provider B / Model B
議事録・タスク抽出         → Provider C / Model C
エディタAI処理             → Provider C / Model D
```

## 10.2 Provider Capability

```ts
export type ProviderCapability =
  | "transcription.batch"
  | "transcription.realtime"
  | "text.generate"
  | "text.structured_output"
  | "models.list";
```

機能実行前に、選択Providerが必要Capabilityを持つか検証する。
未対応の場合は処理を開始せず、設定画面への導線を表示する。

## 10.3 Provider Profile

秘密情報を除くProvider設定はSQLiteへ保存する。

```ts
export type ApiProviderProfile = {
  id: string;
  displayName: string;
  providerType: "openai" | "openai_compatible";
  baseUrl: string;
  authType: "bearer" | "x-api-key" | "none";
  credentialTarget: string | null;
  organizationId?: string;
  projectId?: string;
  defaultHeaders: Record<string, string>;
  timeoutMs: number;
  enabled: boolean;
  capabilities: ProviderCapability[];
  createdAt: string;
  updatedAt: string;
};
```

### 制約

- `baseUrl`の末尾スラッシュは保存時に正規化する。
- `https://`を原則とする。
- `http://localhost`、`http://127.0.0.1`、`http://[::1]`はローカルAPI用途として許可する。
- その他の平文HTTPは警告を表示し、既定では保存不可とする。
- 認証ヘッダーはProvider Profileの設定先ホストにだけ付与する。
- 別ホストへのリダイレクト時は認証ヘッダーを引き継がない。
- `file:`、`javascript:`、`data:`等の非HTTPスキームは禁止する。

## 10.4 APIキー保存

APIキーはSQLite、`app_settings`、`.env`、ログ、診断ZIPへ保存しない。

Windows版MVPでは、C# SidecarからWindows Credential ManagerのGeneric Credentialを使用する。

```text
TargetName:
Inquivora/API/{providerProfileId}

UserName:
providerType またはユーザー指定識別子

CredentialBlob:
APIキー

Persist:
LOCAL_MACHINE（同一PC・同一ユーザーの後続ログオンで利用）
```

UIでは以下だけを表示する。

```text
APIキー: ●●●●●●●●  設定済み
最終更新: 2026/07/04 14:20
```

APIキーの再表示機能は実装しない。
変更時は新しい値で上書きし、削除時はCredential Managerから完全に削除する。

### フロントエンド上の扱い

- APIキー入力値をZustand、localStorage、sessionStorageへ保存しない。
- 入力値は保存コマンドへ渡した直後にフォーム状態から破棄する。
- React DevTools等への露出を減らすため、グローバル状態へ格納しない。
- 接続処理時にキーをフロントエンドへ戻さない。
- RustまたはSidecarが資格情報を取得してHTTPリクエストを組み立てる。

## 10.5 AI設定画面

ルート：`/settings/ai`

### Provider一覧

各カードへ以下を表示する。

- 表示名
- Provider種別
- Base URL
- APIキー設定済み／未設定
- 対応Capability
- 接続状態
- 最終接続テスト日時
- 使用中の機能

操作：

- Providerを追加
- 編集
- 複製（秘密情報は複製しない）
- 有効／無効
- 接続テスト
- モデル一覧取得
- 既定に設定
- 削除

### Provider追加・編集フォーム

```text
表示名                 [ OpenAI Personal             ]
Provider種別           [ OpenAI                    ▼]
Base URL               [ https://api.openai.com/v1  ]
認証方式                [ Bearer                    ▼]
APIキー                 [ ●●●●●●●●●●●●             ]
Organization ID        [ 任意                         ]
Project ID             [ 任意                         ]
タイムアウト            [ 60 秒                       ]
カスタムヘッダー         [ 詳細設定                     ]

[接続テスト] [保存]
```

### 用途別モデル設定

```text
バッチ文字起こし
  Provider: [OpenAI Personal ▼]
  Model:    [手入力または取得一覧 ▼]

リアルタイム文字起こし
  Provider: [OpenAI Personal ▼]
  Model:    [手入力または取得一覧 ▼]

議事録・タスク抽出
  Provider: [OpenAI Personal ▼]
  Model:    [手入力または取得一覧 ▼]

エディタAI
  Provider: [Local LLM ▼]
  Model:    [local-model-name ▼]
```

モデル一覧取得に失敗しても、ユーザーはモデルIDを手入力できる。

## 10.6 接続テスト

接続テストはProviderのCapabilityに応じて段階的に行う。

```text
1. URL・スキーム検証
2. DNS／TCP／TLS接続
3. 認証確認
4. モデル一覧取得（対応時）
5. 最小リクエスト
6. 応答時間と結果を表示
```

テスト結果例：

```ts
export type ProviderConnectionTestResult = {
  success: boolean;
  checkedAt: string;
  latencyMs?: number;
  authenticated: boolean;
  modelsEndpointAvailable?: boolean;
  capabilities: ProviderCapability[];
  errorCode?: string;
  userMessage?: string;
};
```

UIへAPIレスポンス本文全体を表示しない。
認証情報、内部ヘッダー、個人データを除去したユーザー向けメッセージだけを表示する。

## 10.7 Provider抽象化

```ts
export interface TranscriptionProvider {
  readonly profileId: string;
  readonly capabilities: ProviderCapability[];

  transcribe(input: {
    audioPath: string;
    language: "ja";
    prompt?: string;
    model: string;
  }): Promise<TranscriptionResult>;
}

export interface RealtimeTranscriptionProvider {
  readonly profileId: string;

  createSession(input: {
    language: "ja";
    model: string;
  }): Promise<RealtimeTranscriptionSession>;
}

export interface MeetingAiProvider {
  readonly profileId: string;

  summarize(input: {
    model: string;
    payload: MeetingAiInput;
  }): Promise<MeetingAiOutput>;
}
```

API固有処理は次へ閉じ込める。

```text
src/services/ai/providers/
├─ openai/
├─ openai-compatible/
└─ registry.ts
```

Provider Registryは、設定されたProfileからAdapterを生成する。

## 10.8 文字起こし結果

```ts
export type TranscriptionResult = {
  text: string;
  language: string;
  durationMs?: number;
  segments?: Array<{
    startMs: number;
    endMs: number;
    text: string;
  }>;
  usage?: {
    inputUnits?: number;
    outputUnits?: number;
  };
};
```

## 10.9 議事録AI入力

```ts
export type MeetingAiInput = {
  title: string;
  startedAt: string;
  endedAt: string;
  transcript: Array<{
    source: "mic" | "system";
    speakerLabel: string;
    startMs: number;
    text: string;
  }>;
  userNotes: string;
  timezone: "Asia/Tokyo";
};
```

## 10.10 議事録AI出力

```ts
export type MeetingAiOutput = {
  title: string;
  summary: string;
  decisions: Array<{
    text: string;
    sourceStartMs?: number;
  }>;
  taskCandidates: Array<{
    title: string;
    description?: string;
    assignee?: string;
    dueAt?: string;
    priority: "high" | "medium" | "low";
    sourceStartMs?: number;
  }>;
  openQuestions: Array<{
    text: string;
    sourceStartMs?: number;
  }>;
};
```

出力はZodで検証する。
Schema不一致の場合は1回だけ修復リクエストを行い、それでも失敗した場合はジョブを失敗扱いにする。

## 10.11 タスク候補

AIが抽出したタスクは自動登録しない。

```text
AI抽出
  ↓
タスク候補一覧
  ↓
ユーザーが選択・修正
  ↓
正式タスク登録
```

## 10.12 APIジョブ

状態：

```text
pending
processing
completed
retry_wait
failed
cancelled
```

APIジョブへ必ず次を記録する。

- Provider Profile ID
- Model ID
- Capability
- 対象Entity ID
- 開始・完了日時
- リトライ回数
- 正規化したエラーコード
- Usage（取得できる場合）

リトライ：

```text
2秒 → 5秒 → 15秒 → 60秒
```

認証エラー、モデル不存在、設定不備は自動再試行しない。
レート制限・一時的な5xx・接続切断だけを再試行対象にする。

## 10.13 Fallback

用途ごとに任意でFallback Providerを1件設定できる。

Fallbackを実行する条件：

- 接続タイムアウト
- 一時的な5xx
- レート制限
- Provider停止

Fallbackしない条件：

- APIキー不正
- 入力データ不正
- ユーザーによるキャンセル
- プライバシー確認の拒否

Fallback先へデータを送信する前に、Providerが変わることをUIへ明示する設定を用意する。

## 10.14 使用量表示

ProviderがUsageを返す場合、ローカルDBへ記録して設定画面で集計できる。

- 日別リクエスト数
- 入力／出力Unit
- 音声処理時間
- エラー数

料金はProviderごとに変動するため、MVPでは自動金額計算を保証しない。
ユーザーが任意単価を設定した場合だけ参考金額を表示する。

---

# 11. 議事録管理

## 11.1 一覧表示

- 取得日時の新しい順
- 今日・昨日・年月でグループ化
- タイトル
- 開始・終了日時
- 会議時間
- 要約の冒頭
- タスク候補数
- 決定事項数
- 音声の有無
- 関連ファイル

## 11.2 議事録クリック時

1. 関連ワークスペースを開く
2. 関連ファイルをタブで開く
3. 会議マーカーへスクロール
4. AI・会議パネルへ要約を復元
5. 音声プレイヤーを復元

## 11.3 会議終了フロー

```text
終了ボタン
  ↓
録音停止
  ↓
未送信チャンクの処理
  ↓
文字起こし確定
  ↓
AI議事録生成
  ↓
プレビュー
  ↓
ファイルへ追記 / 保存
  ↓
タスク候補確認
```

## 11.4 追記テンプレート

```md
## AI要約

{summary}

## 決定事項

- {decision}

## タスク候補

- [ ] {task}

## 未確認事項

- {question}
```

---

# 12. タスク管理

## 12.1 一覧の既定順

```sql
ORDER BY
  CASE WHEN status = 'completed' THEN 1 ELSE 0 END,
  CASE WHEN due_at IS NULL THEN 1 ELSE 0 END,
  due_at ASC,
  CASE priority
    WHEN 'high' THEN 0
    WHEN 'medium' THEN 1
    ELSE 2
  END,
  created_at ASC;
```

表示順：

1. 期限切れ
2. 今日
3. 明日
4. 今週
5. 来週以降
6. 期日なし
7. 完了

## 12.2 タスク属性

```ts
export type Task = {
  id: string;
  title: string;
  description: string | null;
  dueAtUtc: string | null;
  timezone: string;
  priority: "high" | "medium" | "low";
  color: "blue" | "indigo" | "violet" | "pink" | "red" | "orange" | "green" | "teal";
  status: "todo" | "in_progress" | "on_hold" | "completed" | "cancelled";
  assignee: string | null;
  projectName: string | null;
  meetingId: string | null;
  linkedFilePath: string | null;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
};
```

## 12.3 操作

- 作成
- 編集
- 複製
- 削除
- 完了
- 未完了へ戻す
- 進行中へ変更
- 期日変更
- 優先度変更
- 通知追加
- 関連ファイルを開く
- 関連議事録を開く
- 8色から表示色を選ぶ
- メモ内の選択テキストから作成する

## 12.4 一覧フィルター

- すべて
- 未完了
- 進行中
- 完了
- 今日
- 今週
- 期限切れ
- 優先度
- プロジェクト
- 担当

## 12.5 期日なし

期日なしタスクは、未完了タスクの最後へ表示する。

---

# 13. カレンダー

## 13.1 表示

- 月：`dayGridMonth`
- 週：`timeGridWeek`
- 日：`timeGridDay`

## 13.2 表示対象

- 独立した予定
- 会議
- 時刻付きタスク
- 終日タスク期限
- 繰り返し予定

## 13.3 カレンダー操作

- 日付クリック：予定作成
- 時刻範囲ドラッグ：予定作成
- イベントクリック：右詳細を表示
- ドラッグ：日時変更
- リサイズ：終了日時変更
- 今日へ移動
- 月・週・日切替
- 会議開始
- 左の期日なしタスクを日付・時刻へドラッグして期日を設定

## 13.4 タスク表示

- 時刻あり：時刻イベント
- 日付のみ：終日イベント
- 完了：薄く表示または非表示設定
- タスクと予定はアイコンで区別
- タスクは個別に選択した色で表示

## 13.5 日時

DBはUTCで保存し、表示時に`Asia/Tokyo`へ変換する。

```text
DB: 2026-07-04T01:00:00Z
UI: 2026年7月4日 10:00
```

---

# 14. Windows通知

## 14.1 通知対象

- タスク期限
- 予定開始
- 会議開始
- 文字起こし完了
- AI議事録完了
- 録音エラー
- API処理失敗

## 14.2 MVP動作方式

通常の閉じる操作ではアプリを完全終了せず、タスクトレイへ格納する。

- トレイ常駐中：通知を送信
- 完全終了中：通知しない
- 完全終了時は警告を表示可能
- Windows起動時の自動起動を設定可能

## 14.3 通知クリック

通知の引数に対象種別とIDを含める。

```text
inquivora://open?type=task&id=TASK_ID
inquivora://open?type=event&id=EVENT_ID
inquivora://open?type=meeting&id=MEETING_ID
```

クリック時：

1. メインアプリを前面化
2. 対象画面へ遷移
3. 対象レコードを選択
4. 右詳細パネルを表示

## 14.4 通知プロトコル

```json
{
  "command": "notify",
  "notificationId": "...",
  "title": "リマインダー",
  "body": "DX導入定例会が10:00から始まります。",
  "launchUri": "inquivora://open?type=event&id=..."
}
```

## 14.5 通知再計算

次のタイミングで再計算する。

- アプリ起動
- スリープ復帰
- タスク更新
- 予定更新
- 通知設定変更
- タイムゾーン変更

## 14.6 重複防止

`reminder_id + scheduled_at`を一意キーにし、送信済み通知を再送しない。

---

# 15. 全文検索

## 15.1 対象

- ファイル名
- テキストファイル本文
- 議事録要約
- 文字起こし
- タスク
- 予定
- 自分用メモ

## 15.2 日本語検索

SQLite FTS5の`trigram` tokenizerを使用する。

```sql
CREATE VIRTUAL TABLE search_documents_fts USING fts5(
  title,
  body,
  path UNINDEXED,
  entity_type UNINDEXED,
  entity_id UNINDEXED,
  tokenize='trigram'
);
```

2文字以下の検索はLIKEによるフォールバックを行う。

## 15.3 インデックス更新

- ファイル作成・更新・削除時
- 議事録確定時
- タスク更新時
- 予定更新時

大規模変更時はバッチ処理し、UIをブロックしない。

---

# 16. SQLiteスキーマ

`src-tauri/migrations/001_init.sql`

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE workspaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  root_path TEXT NOT NULL UNIQUE,
  last_opened_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE files (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  absolute_path TEXT NOT NULL,
  name TEXT NOT NULL,
  extension TEXT,
  size_bytes INTEGER NOT NULL DEFAULT 0,
  encoding TEXT,
  line_ending TEXT,
  content_hash TEXT,
  modified_at TEXT,
  indexed_at TEXT,
  is_binary INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
  UNIQUE(workspace_id, relative_path)
);

CREATE INDEX idx_files_workspace ON files(workspace_id);
CREATE INDEX idx_files_name ON files(name);

CREATE TABLE meetings (
  id TEXT PRIMARY KEY,
  workspace_id TEXT,
  title TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  target_file_path TEXT NOT NULL,
  start_marker TEXT NOT NULL,
  end_marker TEXT NOT NULL,
  mic_audio_path TEXT,
  system_audio_path TEXT,
  summary TEXT,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE INDEX idx_meetings_started ON meetings(started_at DESC);

CREATE TABLE transcript_segments (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL,
  source TEXT NOT NULL,
  speaker_label TEXT NOT NULL,
  start_ms INTEGER NOT NULL,
  end_ms INTEGER NOT NULL,
  text TEXT NOT NULL,
  confidence REAL,
  status TEXT NOT NULL,
  audio_chunk_path TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX idx_transcript_meeting_time
  ON transcript_segments(meeting_id, start_ms);

CREATE TABLE meeting_decisions (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL,
  text TEXT NOT NULL,
  source_start_ms INTEGER,
  created_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  due_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  priority TEXT NOT NULL DEFAULT 'medium',
  status TEXT NOT NULL DEFAULT 'todo',
  assignee TEXT,
  project_name TEXT,
  meeting_id TEXT,
  linked_file_path TEXT,
  source_start_ms INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE SET NULL
);

CREATE INDEX idx_tasks_due ON tasks(due_at);
CREATE INDEX idx_tasks_status_due ON tasks(status, due_at);
CREATE INDEX idx_tasks_meeting ON tasks(meeting_id);

CREATE TABLE task_candidates (
  id TEXT PRIMARY KEY,
  meeting_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  due_at TEXT,
  priority TEXT NOT NULL DEFAULT 'medium',
  assignee TEXT,
  source_start_ms INTEGER,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE TABLE events (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  start_at TEXT NOT NULL,
  end_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  all_day INTEGER NOT NULL DEFAULT 0,
  event_type TEXT NOT NULL DEFAULT 'event',
  recurrence_rule TEXT,
  meeting_id TEXT,
  task_id TEXT,
  location TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE SET NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX idx_events_start ON events(start_at);

CREATE TABLE reminders (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  event_id TEXT,
  notify_at TEXT NOT NULL,
  timezone TEXT NOT NULL DEFAULT 'Asia/Tokyo',
  status TEXT NOT NULL DEFAULT 'scheduled',
  sent_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE,
  CHECK (task_id IS NOT NULL OR event_id IS NOT NULL)
);

CREATE UNIQUE INDEX idx_reminder_unique
  ON reminders(COALESCE(task_id, ''), COALESCE(event_id, ''), notify_at);
CREATE INDEX idx_reminders_notify ON reminders(status, notify_at);

CREATE TABLE api_provider_profiles (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  provider_type TEXT NOT NULL,
  base_url TEXT NOT NULL,
  auth_type TEXT NOT NULL DEFAULT 'bearer',
  credential_target TEXT,
  organization_id TEXT,
  project_id TEXT,
  default_headers_json TEXT NOT NULL DEFAULT '{}',
  timeout_ms INTEGER NOT NULL DEFAULT 60000,
  capabilities_json TEXT NOT NULL DEFAULT '[]',
  enabled INTEGER NOT NULL DEFAULT 1,
  last_test_status TEXT,
  last_tested_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_api_provider_name
  ON api_provider_profiles(display_name);

CREATE TABLE ai_feature_bindings (
  feature_key TEXT PRIMARY KEY,
  provider_profile_id TEXT,
  model_id TEXT,
  fallback_provider_profile_id TEXT,
  fallback_model_id TEXT,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE SET NULL,
  FOREIGN KEY (fallback_provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE SET NULL
);

-- feature_key:
-- transcription.batch
-- transcription.realtime
-- meeting.summary
-- editor.ai

CREATE TABLE api_usage_logs (
  id TEXT PRIMARY KEY,
  provider_profile_id TEXT NOT NULL,
  feature_key TEXT NOT NULL,
  model_id TEXT NOT NULL,
  entity_id TEXT,
  input_units INTEGER,
  output_units INTEGER,
  audio_duration_ms INTEGER,
  latency_ms INTEGER,
  status TEXT NOT NULL,
  error_code TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE CASCADE
);

CREATE INDEX idx_api_usage_provider_created
  ON api_usage_logs(provider_profile_id, created_at);

CREATE TABLE api_jobs (
  id TEXT PRIMARY KEY,
  job_type TEXT NOT NULL,
  provider_profile_id TEXT,
  model_id TEXT,
  capability TEXT,
  entity_id TEXT,
  request_path TEXT,
  status TEXT NOT NULL,
  retry_count INTEGER NOT NULL DEFAULT 0,
  next_retry_at TEXT,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_profile_id)
    REFERENCES api_provider_profiles(id) ON DELETE SET NULL
);

CREATE INDEX idx_api_jobs_status_retry
  ON api_jobs(status, next_retry_at);

CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE recent_tabs (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  path TEXT NOT NULL,
  tab_order INTEGER NOT NULL,
  is_pinned INTEGER NOT NULL DEFAULT 0,
  cursor_line INTEGER NOT NULL DEFAULT 1,
  cursor_column INTEGER NOT NULL DEFAULT 1,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE TABLE search_documents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  path TEXT,
  updated_at TEXT NOT NULL,
  UNIQUE(entity_type, entity_id)
);

CREATE VIRTUAL TABLE search_documents_fts USING fts5(
  title,
  body,
  path UNINDEXED,
  entity_type UNINDEXED,
  entity_id UNINDEXED,
  tokenize='trigram'
);
```

FTSテーブルとの同期はアプリケーションコードで行い、更新処理をトランザクション化する。

---

# 17. Tauriコマンド契約

すべてのコマンドは成功時にデータ、失敗時に共通エラー形式を返す。

```ts
export type AppError = {
  code: string;
  message: string;
  details?: unknown;
  retryable: boolean;
};
```

## 17.1 ワークスペース

```text
workspace_open(path)
workspace_create(path, name)
workspace_list_recent()
workspace_close(id)
workspace_scan(id)
```

## 17.2 ファイル

```text
file_list_children(workspaceId, relativePath)
file_read(path)
file_write_atomic(path, content, encoding, lineEnding)
file_create(path, type)
file_rename(oldPath, newPath)
file_delete(path, useRecycleBin)
file_copy(source, destination)
file_move(source, destination)
file_reveal(path)
file_open_external(path)
file_detect_type(path)
```

## 17.3 会議

```text
meeting_start(input)
meeting_pause(meetingId)
meeting_resume(meetingId)
meeting_stop(meetingId)
meeting_get(meetingId)
meeting_list(filter)
meeting_delete(meetingId)
meeting_append_segment(meetingId, segment)
meeting_generate_summary(meetingId)
```

## 17.4 タスク

```text
task_create(input)
task_update(id, patch)
task_delete(id)
task_get(id)
task_list(filter, sort)
task_complete(id)
task_reopen(id)
task_accept_candidate(candidateId, patch)
```

## 17.5 カレンダー

```text
event_create(input)
event_update(id, patch)
event_delete(id)
event_get_range(startUtc, endUtc)
```

## 17.6 通知

```text
reminder_create(input)
reminder_update(id, patch)
reminder_delete(id)
reminder_list_upcoming()
notification_test()
notification_reconcile()
```

## 17.7 検索

```text
search_global(query, filters, limit, offset)
search_reindex_workspace(workspaceId)
search_cancel(jobId)
```

## 17.8 API Provider設定

```text
api_provider_list()
api_provider_get(id)
api_provider_create(input)
api_provider_update(id, patch)
api_provider_delete(id)
api_provider_enable(id, enabled)
api_provider_set_secret(id, secret)
api_provider_delete_secret(id)
api_provider_has_secret(id)
api_provider_test(id)
api_provider_list_models(id)
ai_feature_binding_get(featureKey)
ai_feature_binding_set(featureKey, input)
api_usage_list(filter)
```

`api_provider_get`および`api_provider_list`はAPIキーを返してはならない。
返却可能なのは`hasSecret: boolean`と最終更新日時のみとする。

---

# 18. フロントエンド状態管理

Zustand Storeを機能単位で分ける。

```text
useAppStore
useWorkspaceStore
useEditorStore
useMeetingStore
useTaskStore
useCalendarStore
useNotificationStore
useSettingsStore
```

永続化対象：

- 最後の画面
- サイドバー幅
- 開いているタブ
- カーソル位置
- カレンダー表示モード
- タスクフィルター

録音中状態やAPIジョブ状態はSQLiteを正とし、UI Storeだけを正本にしない。

---

# 19. 設定

## 19.1 一般

- Windows起動時に開始
- 閉じる時にトレイへ格納
- 自動保存
- 自動保存間隔
- 言語
- タイムゾーン

## 19.2 エディタ

- フォントサイズ
- タブ幅
- ワードラップ
- ミニマップ
- Markdownプレビュー方式

## 19.3 会議

- 既定マイク
- 既定出力デバイス
- 音声を保存
- 文字起こし後に音声削除
- チャンク秒数
- 既定追記位置

## 19.4 AI・API

- Provider Profile一覧
- Provider追加・編集・削除
- OpenAIプリセット
- OpenAI互換Endpoint
- Base URL
- 認証方式
- APIキー設定状態
- Organization ID・Project ID（任意）
- カスタムヘッダー（秘密値は禁止）
- 接続テスト
- モデル一覧取得
- モデルID手入力
- 用途別Provider／Model割り当て
- Fallback Provider（任意）
- API送信前の確認
- Usage表示

APIキーはSQLiteや設定JSONへ保存せず、Windows Credential Managerへ保存する。
設定のエクスポートにはProvider Profileを含めてよいが、秘密情報は必ず除外する。

## 19.5 通知

- 通知有効
- サウンド
- 既定通知時刻
- 予定の既定リマインド
- タスクの既定リマインド

---

# 20. セキュリティ

## 20.1 ファイルアクセス

- 選択済みワークスペース配下だけを通常アクセス対象とする。
- パストラバーサルを拒否する。
- シンボリックリンク経由でルート外へ出る場合は確認する。
- 削除は既定でごみ箱を使用する。

## 20.2 Tauri権限

- 必要なコマンドだけcapabilityへ許可する。
- 任意Shell実行を公開しない。
- Sidecarの実行ファイル名を固定する。
- 外部URLを開く前にスキームを検証する。

## 20.3 Markdown

HTMLを無効化するか、サニタイズしてから表示する。

## 20.4 API

- APIキーをログへ出さない。
- 音声・本文をデバッグログへ出さない。
- TLS検証を無効化しない。
- タイムアウトを設定する。

---

# 21. プライバシー

録音開始時に次を明示する。

- マイクを録音するか
- PC音声を録音するか
- 外部APIへ送信するか
- ローカル音声を保存するか

音声保存設定：

- 保存しない
- 文字起こし完了後に削除
- 7日後削除
- 30日後削除
- 手動削除

音声削除ジョブはアプリ起動時と1日1回実行する。

---

# 22. ログと診断

## 22.1 ログ

```text
app.log
file.log
audio.log
api.log
notification.log
crash.log
```

## 22.2 ローテーション

- 1ファイル10MB
- 最大5世代
- 個人データは記録しない

## 22.3 診断情報

設定画面から診断ZIPを生成する。

含める：

- アプリバージョン
- OSバージョン
- 匿名化ログ
- DBスキーマバージョン
- 音声デバイス名

含めない：

- APIキー
- ファイル本文
- 文字起こし全文
- 音声

---

# 23. エラーコード

| コード | 内容 |
|---|---|
| WORKSPACE_NOT_FOUND | ワークスペースが存在しない |
| PATH_OUTSIDE_WORKSPACE | 許可範囲外のパス |
| FILE_ENCODING_UNSUPPORTED | 文字コード非対応 |
| FILE_CONFLICT | 外部変更との競合 |
| FILE_TOO_LARGE | ファイルが大きすぎる |
| AUDIO_DEVICE_NOT_FOUND | 音声デバイスなし |
| AUDIO_DEVICE_LOST | 録音中にデバイス切断 |
| AUDIO_CAPTURE_FAILED | 録音失敗 |
| API_AUTH_FAILED | API認証失敗 |
| API_RATE_LIMITED | API制限 |
| API_TIMEOUT | APIタイムアウト |
| TRANSCRIPTION_FAILED | 文字起こし失敗 |
| AI_SCHEMA_INVALID | AI出力形式不正 |
| NOTIFICATION_FAILED | 通知失敗 |
| DATABASE_MIGRATION_FAILED | DB移行失敗 |

---

# 24. パフォーマンス目標

| 項目 | 目標 |
|---|---:|
| コールド起動 | 3秒以内 |
| ワークスペース初期表示 | 2秒以内 |
| 通常ファイル表示 | 1秒以内 |
| 自動保存 | 入力停止後1秒以内 |
| タスク一覧 | 10,000件 |
| 議事録一覧 | 10,000件 |
| ファイルツリー | 50,000件 |
| 検索結果初回 | 2秒以内 |
| 通知誤差 | 30秒以内 |
| 録音UI反応 | 200ms以内 |

---

# 25. アクセシビリティ

- キーボード操作
- 明確なフォーカスリング
- アイコンにaria-label
- 文字サイズ変更
- 高DPI対応
- 色だけに依存しない状態表示
- Windows高コントラスト確認

---

# 26. ショートカット

| 操作 | キー |
|---|---|
| ファイルを開く | Ctrl+O |
| 保存 | Ctrl+S |
| 全保存 | Ctrl+Shift+S |
| ファイル内検索 | Ctrl+F |
| 選択テキストをタスク化 | 右クリック→「選択範囲からタスクを作成」 |
| 全文検索 | Ctrl+Shift+F |
| コマンドパレット | Ctrl+Shift+P |
| 新規タスク | Ctrl+Shift+T |
| 新規予定 | Ctrl+Shift+E |
| 会議開始・終了 | Ctrl+Shift+R |
| 音声入力 | 右Alt長押し |
| タスク画面 | Ctrl+Alt+T |
| カレンダー画面 | Ctrl+Alt+C |

---

# 27. テスト

## 27.1 単体テスト

- タスク期日順
- 期日なしが最後になる
- UTC・日本時間変換
- 文字コード判定
- 安全なファイル保存
- マーカー挿入
- 文字起こし重複除去
- AI出力Zod検証
- 通知時刻計算
- パストラバーサル拒否

## 27.2 結合テスト

- Monaco編集→ファイル保存
- 外部変更→競合ダイアログ
- 録音→チャンク→API→ファイル追記
- 会議終了→要約→タスク候補
- 候補承認→タスク一覧→カレンダー
- 通知クリック→対象詳細
- スリープ復帰→通知再計算

## 27.3 E2E

1. 初回起動
2. ワークスペース選択
3. Markdown作成
4. 会議開始
5. 文字起こし追記
6. 会議終了
7. AI議事録生成
8. タスク候補承認
9. 期日順表示
10. カレンダー表示
11. Windows通知
12. 再起動後復元

## 27.4 実機条件

- 内蔵マイク
- USBヘッドセット
- Bluetoothヘッドセット
- 複数出力デバイス
- PCスリープ
- オフライン
- 日本語パス
- OneDrive配下
- 125%・150%表示倍率

---

# 28. GitHub先行push・ビルド・EXE化

## 28.1 正式ビルド方針

正式配布用のEXEは、次の条件を満たしたコミットからのみ生成する。

1. 変更がすべてコミット済みである。
2. lint、型チェック、単体テストが成功している。
3. 禁止ファイル・秘密情報の検査が成功している。
4. 指定GitHubリポジトリへ対象ブランチをpushできている。
5. GitHub上の対象ブランチSHAとローカル`HEAD`が完全一致している。
6. リリースタグがGitHubへpushされている。
7. GitHub Actionsが、そのタグのコミットからWindowsインストーラーを生成する。

ローカルで実行する`npm run tauri build`は開発確認専用とし、正式配布物として扱わない。

## 28.2 npmスクリプト

`package.json`へ次を定義する。

```json
{
  "scripts": {
    "dev": "vite",
    "tauri:dev": "tauri dev",
    "lint": "eslint .",
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:rust": "cargo test --manifest-path src-tauri/Cargo.toml",
    "test:dotnet": "dotnet test sidecars/inquivora-native/Inquivora.Native.sln",
    "release:preflight": "node scripts/release-preflight.mjs",
    "release:windows": "node scripts/release-windows.mjs",
    "build:local": "tauri build"
  }
}
```

## 28.3 正式リリースコマンド

```bash
npm run release:windows -- --version 0.1.0
```

このコマンドはローカルでEXEを生成せず、次を順番に実行する。

```text
release.config.json読込
        ↓
Gitリポジトリ・ブランチ確認
        ↓
作業ツリーがcleanか確認
        ↓
lint / typecheck / frontend test
        ↓
Rust test / .NET test
        ↓
秘密情報・禁止ファイル検査
        ↓
バージョン整合性確認
        ↓
指定GitHubへブランチpush
        ↓
remote SHA == local HEADを確認
        ↓
v0.1.0タグ作成・push
        ↓
GitHub ActionsがWindowsビルド
        ↓
Artifact / Pre-releaseを生成してRelease一覧へ公開
```

## 28.4 Preflight検査

`scripts/release-preflight.mjs`は最低限、次を検査する。

- Windows環境であること
- Node.js、npm、Rust、Cargo、.NET SDK、Gitが利用可能であること
- `release.config.json`が存在すること
- `repositoryUrl`と現在のrelease remoteが一致すること
- 現在ブランチが設定された`branch`と一致すること
- `git status --porcelain`が空であること
- `package.json`、`tauri.conf.json`、`Cargo.toml`のバージョンが一致すること
- 同名タグがローカル・GitHubに存在しないこと
- `.env`、DB、録音、ログ、証明書がGit追跡対象に含まれていないこと
- ソース内に既知形式のAPIキーらしい文字列がないこと
- lint、型チェック、全テストが成功すること

1件でも失敗した場合はpush・タグ作成・ビルドを中止する。

## 28.5 GitHub remote

既存の`origin`を勝手に変更しない。正式配布先は`release` remoteとして登録する。

```bash
git remote add release https://github.com/<OWNER>/<REPOSITORY>.git
```

既に存在する場合：

```bash
git remote set-url release https://github.com/<OWNER>/<REPOSITORY>.git
```

リリーススクリプトは`release.config.json`と`git remote get-url release`の一致を検証する。

## 28.6 pushとSHA検証

```bash
git push release HEAD:main
```

push後、以下を取得する。

```bash
LOCAL_SHA=$(git rev-parse HEAD)
REMOTE_SHA=$(git ls-remote release refs/heads/main | awk '{print $1}')
```

`LOCAL_SHA`と`REMOTE_SHA`が一致しない場合、タグ作成とビルドを禁止する。

## 28.7 リリースタグ

SHA一致確認後にannotated tagを作成する。

```bash
git tag -a v0.1.0 -m "Inquivora v0.1.0"
git push release v0.1.0
```

- タグは対象バージョンと一致させる。
- 既存タグの上書きは禁止する。
- GitHub Actionsは`v*`タグpushだけを正式リリーストリガーとする。
- 通常のブランチpushではテストのみを実行し、配布用EXEを公開しない。

## 28.8 GitHub ActionsでのWindowsビルド

タグpushを受けたGitHub Actionsの`windows-latest` Runnerで次を実行する。

1. タグのコミットをcheckout
2. Node.jsセットアップ
3. Rustセットアップ
4. .NET 8セットアップ
5. npm依存関係インストール
6. Sidecar test・publish
7. lint・typecheck・test
8. Rust test
9. Tauri NSIS build
10. コード署名（設定されている場合）
11. ArtifactへセットアップEXEを保存
12. GitHub Draft ReleaseへセットアップEXEを添付
13. ビルド元のGit SHA、バージョン、SHA-256をRelease Notesへ記録
14. 全成果物の添付成功後、Pre-releaseとしてRelease一覧へ自動公開
15. 既存Draftの公開が必要な場合は`workflow_dispatch`へタグを指定し、Actionsから公開

## 28.9 GitHub Actions Workflow例

`.github/workflows/release-windows.yml`

```yaml
name: Release Windows

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build-windows:
    runs-on: windows-latest

    steps:
      - name: Checkout tagged commit
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Setup .NET
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '8.0.x'

      - name: Install frontend dependencies
        run: npm ci

      - name: Frontend checks
        run: |
          npm run lint
          npm run typecheck
          npm test

      - name: Rust tests
        run: cargo test --manifest-path src-tauri/Cargo.toml

      - name: Native sidecar tests
        run: dotnet test sidecars/inquivora-native/Inquivora.Native.sln --configuration Release

      - name: Publish native sidecar
        run: dotnet publish sidecars/inquivora-native/Inquivora.Native.csproj --configuration Release --runtime win-x64 --self-contained true

      - name: Build and create draft release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Inquivora ${{ github.ref_name }}'
          releaseDraft: true
          prerelease: false
          includeUpdaterJson: true

      - name: Upload build artifacts
        uses: actions/upload-artifact@v4
        with:
          name: Inquivora-Windows-${{ github.ref_name }}
          path: |
            src-tauri/target/release/bundle/nsis/*.exe
            src-tauri/target/release/bundle/msi/*.msi
          if-no-files-found: error
          retention-days: 30

      - name: Publish release after all assets are ready
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: gh release edit ${{ github.ref_name }} --draft=false --prerelease
```

実際のSidecarパスとTauri external binary名は実装時に一致させる。

## 28.10 GitHub Actions Secrets

GitHub Actionsへ登録してよい秘密情報：

- Windowsコード署名用証明書または署名サービス認証情報
- Tauri Updater署名秘密鍵
- 各署名パスワード

登録禁止：

- ユーザーのOpenAI等のAPIキー
- アプリ内で保存されたProvider資格情報
- 実際の録音・議事録・SQLiteデータ

署名情報を設定していない開発段階では、署名処理をスキップできるようにする。

## 28.11 生成物

正式成果物：

```text
Inquivora_0.1.0_x64-setup.exe
Inquivora_0.1.0_x64_en-US.msi     # MSIも有効化した場合
latest.json                        # Updaterを有効化した場合
SHA256SUMS.txt
```

開発確認用ローカル生成物：

```text
src-tauri/target/release/inquivora.exe
src-tauri/target/release/bundle/nsis/Inquivora_0.1.0_x64-setup.exe
```

ローカル生成物には「未検証ローカルビルド」と分かるよう、配布しない運用ルールをREADMEへ記載する。

## 28.12 配布設定

- インストール単位：ユーザー
- インストール先：LocalAppData
- スタートメニュー登録
- デスクトップショートカット：選択式
- アンインストーラー登録
- WebView2の最低バージョン確認

## 28.13 コード署名

一般配布時はWindowsコード署名を行う。署名は原則としてGitHub Actions内で実行し、証明書や秘密鍵をリポジトリへコミットしない。

## 28.14 失敗時の扱い

- push失敗：ビルドしない
- remote SHA不一致：ビルドしない
- タグpush失敗：ビルドしない
- CIテスト失敗：成果物を公開しない
- Artifact作成失敗：Releaseを公開しない
- コード署名失敗：一般公開しない
- 一部工程の再実行時も、同じタグ・同じSHAであることを確認する

---

# 29. CI/CDとブランチ運用

## 29.1 Workflow構成

```text
pull_request / branch push:
  secret scan
  lint
  typecheck
  frontend unit test
  rust test
  dotnet test
  optional Tauri compile check

v* tag push:
  上記全チェック
  Windows Sidecar publish
  Tauri NSIS/MSI build
  code signing
  artifact upload
  GitHub Draft Release
  updater metadata
```

## 29.2 mainブランチ保護

正式運用時は、`main`へ次の保護を推奨する。

- 必須Status Check
- force push禁止
- branch deletion禁止
- 可能ならPull Request経由
- 管理者による例外を最小化
- GitHub Secret ScanningとPush Protectionを有効化

個人開発の初期段階で直接pushを許可する場合も、リリーススクリプトのpreflightとSHA検証は省略しない。

## 29.3 ArtifactとRelease

- CI検証用成果物はGitHub Actions Artifactへ保存する。
- 正式候補はDraft Releaseへ添付し、成果物とSHA-256の添付完了後にActionsからPre-releaseとして自動公開する。
- 途中ステップが失敗した場合はDraftのまま保持し、Release一覧へ不完全な成果物を公開しない。
- ReleaseへGit SHAとファイルハッシュを記録する。
- 同じバージョン番号の成果物を別コミットから作り直さない。

## 29.4 再現可能性

- `package-lock.json`をコミットし、CIでは`npm ci`を使う。
- Rustの`Cargo.lock`をコミットする。
- .NETの依存バージョンを固定する。
- GitHub Actionsの主要Actionは少なくともメジャーバージョンを固定する。
- ビルド時刻に依存するアプリ挙動を避ける。
- Release NotesへRunner、コミットSHA、バージョンを記録する。

## 29.5 APIキー方針

BYOKのAPIキーはユーザー端末のWindows Credential Managerへだけ保存する。GitHub ActionsではAI APIの実通信テストを行わず、Mock ServerまたはAdapter単体テストを使用する。

---

# 30. 実装フェーズ

## Phase 1：Windowsアプリ基盤

- Tauri起動
- アプリシェル
- 縦ナビ
- 設定保存
- SQLite
- トレイ常駐
- GitHub release remote設定
- GitHub Actions基盤
- push前Preflight
- remote SHA検証
- NSISビルド

完了条件：指定GitHubへ同一コミットがpushされた場合に限り、GitHub ActionsからセットアップEXEを生成できる。

## Phase 2：ワークスペース

- フォルダ選択
- ファイルツリー
- Monaco Editor
- タブ
- 自動保存
- Markdownプレビュー
- 外部変更検知

完了条件：実ファイルを安全に編集できる。

## Phase 3：タスク・カレンダー

- タスクCRUD
- 期日順一覧
- 状態・優先度
- 月・週・日カレンダー
- 予定CRUD
- タスク連携

完了条件：タスクと予定をローカル管理できる。

## Phase 4：通知

- Windows通知
- リマインダー
- 通知クリック
- スリープ復帰
- 自動起動

完了条件：トレイ常駐中に期限通知できる。

## Phase 5：API設定・録音・文字起こし

- Provider Profile CRUD
- Windows Credential Managerへの秘密保存
- 接続テスト
- モデル一覧取得・手入力
- 用途別Provider割り当て
- Sidecar
- マイク
- ループバック
- チャンク保存
- API文字起こし
- 開いているファイルへの追記

完了条件：会議音声が対象Markdownへ継続追記される。

## Phase 6：AI議事録

- 要約
- 決定事項
- タスク候補
- 未確認事項
- 候補承認

完了条件：会議からタスク・予定までつながる。

## Phase 7：検索・品質

- FTS5
- パフォーマンス
- クラッシュ復旧
- ログ
- E2E
- 自動更新

---

# 31. GitHub Issue分解

```text
EPIC-01 App Foundation
  #1 Tauri Reactプロジェクト作成
  #2 AppShell実装
  #3 SQLite初期化・Migration
  #4 Tray・Single Instance
  #5 NSISビルド

EPIC-02 Workspace
  #10 Workspace選択
  #11 FileTree遅延読み込み
  #12 Monaco Editor
  #13 Tabs・自動保存
  #14 Markdown Preview
  #15 File Watcher

EPIC-03 Tasks
  #20 Task CRUD
  #21 期日順一覧
  #22 Task Detail
  #23 Filter・Search

EPIC-04 Calendar
  #30 FullCalendar導入
  #31 Event CRUD
  #32 Task Calendar連携
  #33 Drag & Drop

EPIC-05 Notifications
  #40 Native Notification Bridge
  #41 Reminder Scheduler
  #42 Notification Activation

EPIC-06 Meetings
  #50 Audio Sidecar
  #51 Microphone Capture
  #52 WASAPI Loopback
  #53 Meeting Session
  #54 Transcript Append

EPIC-07 AI / BYOK
  #60 Provider Registry
  #61 Provider Profile CRUD
  #62 Credential Manager Bridge
  #63 Connection Test
  #64 Model Selection
  #65 Feature Binding
  #66 Transcription Provider
  #67 API Job Queue
  #68 Meeting Summary Schema
  #69 Task Candidate Approval

EPIC-08 Search and Quality
  #70 FTS5 Index
  #71 Recovery
  #72 Logging
  #73 E2E

EPIC-09 GitHub Release Pipeline
  #80 release.config.json
  #81 release-preflight.mjs
  #82 release-windows.mjs
  #83 禁止ファイル・秘密情報検査
  #84 GitHub release remote・SHA検証
  #85 Windows GitHub Actions Workflow
  #86 Artifact・Pre-release自動公開
  #87 Code Signing・Updater Signing
```

---

# 32. MVP受入基準

## 32.1 GitHub・インストール

- [ ] `release.config.json`で指定GitHubリポジトリを設定できる
- [ ] 作業ツリーがdirtyの場合、正式リリースを開始しない
- [ ] 禁止ファイルまたは秘密情報候補がある場合、pushを中止する
- [ ] 指定GitHubへのpush後、remote SHAとlocal HEADの一致を検証できる
- [ ] remote SHAが一致しない場合、タグ作成とEXE生成を行わない
- [ ] `v*`タグからGitHub ActionsがセットアップEXEを生成する
- [ ] ArtifactとReleaseへ同じ成果物を保存し、全ステップ成功後にPre-releaseを自動公開できる
- [ ] Release NotesへGit SHAとSHA-256を記録できる
- [ ] `Inquivora_0.1.0_x64-setup.exe`からインストールできる
- [ ] スタートメニューから起動できる
- [ ] アンインストールできる

## 32.2 ファイル

- [ ] 任意フォルダをワークスペースとして開ける
- [ ] 左にファイルツリーが表示される
- [ ] Markdown・TXT・コードを編集できる
- [ ] 自動保存される
- [ ] 外部変更を検出できる

## 32.3 会議

- [ ] マイクを録音できる
- [ ] PC音声を録音できる
- [ ] 文字起こし結果を開いているMarkdownへ追記できる
- [ ] 失敗したチャンクを後から再送できる
- [ ] 議事録一覧を日時順で確認できる

## 32.4 AI・API設定

- [ ] 設定画面からProviderを追加・編集・削除できる
- [ ] APIキーを安全に保存できる
- [ ] 保存済みAPIキーを画面へ再表示しない
- [ ] 接続テストを実行できる
- [ ] モデルを一覧選択または手入力できる
- [ ] 文字起こしと要約で別Providerを指定できる
- [ ] OpenAI互換のローカルEndpointを登録できる
- [ ] 要約を生成できる
- [ ] 決定事項を抽出できる
- [ ] タスク候補を抽出できる
- [ ] 候補を確認後にタスク登録できる

## 32.5 タスク

- [ ] タスクを期日昇順で表示できる
- [ ] 今日・明日・今週・期限切れを確認できる
- [ ] 優先度・状態を変更できる
- [ ] タスクを完了できる

## 32.6 カレンダー

- [ ] カレンダーを開くと中央全体がカレンダーになる
- [ ] 月・週・日を切り替えられる
- [ ] 予定とタスク期限を表示できる
- [ ] ドラッグで日時変更できる

## 32.7 通知

- [ ] Windows通知を表示できる
- [ ] 指定時刻に通知できる
- [ ] 通知クリックで対象を開ける
- [ ] スリープ復帰後に予定を再計算できる

---

# 33. Definition of Done

MVP 0.1.0は、以下をすべて満たしたとき完成とする。

1. Windows 11 x64へセットアップEXEで導入できる。
2. ローカルフォルダをVS Code風に閲覧・編集できる。
3. 会議のマイク・PC音声を取得できる。
4. API文字起こしを現在開いているMarkdownへ追記できる。
5. 議事録を新しい順に一覧表示できる。
6. AIが要約・決定事項・タスク候補を構造化出力できる。
7. タスクを期日順で表示できる。
8. カレンダーを全体表示できる。
9. Windows通知を送信できる。
10. API障害時も録音データと未処理ジョブを失わない。
11. 通常のアプリ操作で管理者権限を要求しない。
12. APIキーと個人データを安全に扱う。
13. ユーザーが設定画面からProvider、Endpoint、APIキー、Modelを変更できる。
14. APIキーはSQLite・ログ・設定エクスポートへ含まれない。
15. 文字起こしと議事録生成へ別々のProviderを割り当てられる。
16. 正式ビルド前に指定GitHubへソースコードをpushできる。
17. GitHubのremote SHAとローカルHEADが一致しなければ正式EXEを生成しない。
18. 正式EXEはGitHub Actionsがタグ対象コミットから生成する。
19. APIキー、録音、DB、署名鍵がGitHubリポジトリへ含まれない。

---

# 34. 実装開始用プロンプト

以下をClaude CodeまたはCodexへ渡し、Phaseごとに実装する。

```text
添付の「Inquivora 実装仕様書」を唯一の仕様として、Windows 11 x64向けの
Tauri 2 + React + TypeScript + Rustアプリを実装してください。

最初はPhase 1だけを実装してください。
- 不明点は仕様書内の方針を優先して合理的に決定する
- 既存ファイルを不用意に削除しない
- 各機能に型、エラー処理、ログ、テストを付ける
- UI文言は日本語にする
- APIキーをコードやSQLiteへ保存しない
- Provider、Base URL、Modelを設定画面から変更可能にする
- APIキーはWindows Credential Managerへ保存し、UIへ再表示しない
- OpenAI公式APIとOpenAI互換EndpointをProvider Adapterで分離する
- 各Phase完了時にビルド・テストを実行する
- 正式EXE生成前にrelease.config.jsonで指定されたGitHubへコードをpushする
- push後にremote SHAとlocal HEADの一致を検証する
- SHA一致後だけリリースタグをpushし、GitHub ActionsでEXEを生成する
- APIキー、DB、録音、ログ、署名鍵をGitHubへ含めない
- 既存のoriginを勝手に変更せず、正式配布先はrelease remoteとして扱う
- 作業結果、変更ファイル、残課題をREADMEへ記録する

Phase 1の受入基準を満たした後で停止し、次のPhaseへ進む前に結果を提示してください。
```

---

# 35. 参照する公式仕様

実装時は次の公式ドキュメントの最新版を確認する。

- Tauri 2：Architecture、File System、Shell Sidecar、Windows Installer、Updater
- Microsoft：Windows App SDK App Notifications
- Microsoft：Credentials Management API、CREDENTIAL、CRED_PERSIST_LOCAL_MACHINE
- Microsoft：DPAPI（将来の秘密ストレージ代替案）
- Microsoft：Monaco Editor
- FullCalendar：React Component、DayGrid、TimeGrid、Interaction
- SQLite：FTS5、trigram tokenizer
- OpenAI：Speech-to-Text、Realtime Transcription、Structured Outputs、Responses API
- NAudio：WASAPI Capture、Loopback Capture
- GitHub Docs：Workflow Artifacts、Releases、Actions Secrets、Push Protection、Branch Protection
- Tauri 2：Distribute with GitHub、Windows Code Signing
