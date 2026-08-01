# Third-Party Notices

Inquivora の Windows 配布物に含まれる第三者ソフトウェアと、アプリから取得できる外部コンポーネントの通知です。各コンポーネントには、それぞれのライセンスが適用されます。

依存関係のライセンス全文・著作権表示は、次の自動生成レポートに収録し、Windows インストーラーにも同梱しています。

- `THIRD_PARTY_LICENSES_NODE.txt` — フロントエンドの本番依存関係
- `THIRD_PARTY_LICENSES_RUST.txt` — Windows 向け Rust 実行時依存関係
- `THIRD_PARTY_LICENSES_DOTNET.txt` — .NET sidecar、.NET Runtime、Whisper.net／whisper.cpp、NSIS

レポートは `npm run licenses:generate` で lock file とローカルの解決済み依存関係から再生成できます。2026-08-01 時点の監査では、GPL／AGPL／LGPL の依存関係は検出されていません。MPL-2.0、Apache-2.0、MIT、BSD、ISC、Unicode-3.0、Zlib、CPL-1.0（NSIS の LZMA モジュール）等の通知は各レポートに含まれます。

## 主なコンポーネント

### Monaco Editor 0.55.1

Source: https://github.com/microsoft/monaco-editor

Monaco Editor is licensed under the MIT License. The complete license and copyright notice are included in `THIRD_PARTY_LICENSES_NODE.txt`.

### Whisper.net / whisper.cpp

Source: https://github.com/sandrohanea/whisper.net

The packaged local transcription sidecar uses Whisper.net and native whisper.cpp libraries. Their MIT license texts and copyright notices are included in `THIRD_PARTY_LICENSES_DOTNET.txt`.

### Whisper models downloaded by the user

Source: https://huggingface.co/ggerganov/whisper.cpp

Whisper model files are not embedded in the installer. When the user chooses to download a local model, Inquivora obtains a converted OpenAI Whisper model from the whisper.cpp model repository. OpenAI publishes Whisper code and model weights under the MIT License: https://github.com/openai/whisper/blob/main/LICENSE

### Microsoft Edge WebView2 Runtime

Source: https://developer.microsoft.com/microsoft-edge/webview2/

WebView2 Runtime is not embedded as a fixed runtime. The installer uses Microsoft's Evergreen Bootstrapper to obtain it when required. WebView2 is distributed under Microsoft's applicable terms.

## Inquivora 本体

このリポジトリには、Inquivora 本体に対するオープンソースライセンスを設定していません。第三者ライセンスで許諾された部分を除き、リポジトリの公開だけを理由とする複製・改変・再配布の許諾はありません。

This repository does not currently grant an open-source license for Inquivora itself. Except for rights granted by the applicable third-party licenses, no permission to copy, modify, or redistribute Inquivora is granted merely because the source repository is publicly accessible.
