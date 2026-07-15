# Agnes AI — オープンソース Rust デスクトップ AI コーディングエージェント

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#インストールとビルド)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-compatible-skills)

---

## Agnes AI とは？

**Agnes AI は、純粋な Rust で記述され、ネイティブ egui GUI（Chromium / WebView2 ゼロ）を備えたオープンソースのデスクトップ AI コーディングエージェントです。** 言語モデルが提案するすべてアクションは、ファイルシステム・シェル・ネットワークに触れる前に、決定論的なセキュリティゲートによってクロスチェックされる **22 エージェントのゼロトラスト検証パイプライン** を実行します。また、**サブエージェントアーキテクチャ**（Planner / Generator / Evaluator）と、安全な並列実行のための **Git worktree 分離** を備えた **自律的なゴール駆動ループ**（Discover → Plan → Execute → Verify → Iterate）も同梱しています。

Agnes AI は、**Claude Code、Cursor、Aider、Continue.dev の無料・ローカルファーストな代替品** です。API キーとコードはあなたのマシンから一切外部に送信されず、バイナリは小さく、起動は瞬時で、UI はミニマリストなダークネイティブアプリ（埋め込みブラウザなし、Electron なし）です。

> **一言で言えば:** モデルの口頭の「うまくいった」を決して信用しない、高防御・高速・ネイティブ Rust のデスクトップ AI エージェント。Exit Code 0 と空の stderr のみが成功と見なされます。

---

## なぜ Agnes AI なのか？（vs Claude Code / Cursor / Aider / Continue.dev）

| 機能 | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **ランタイム** | ネイティブ Rust GUI（egui） | ターミナル | Electron ベース IDE | ターミナル | VS Code/JetBrains プラグイン |
| **バイナリサイズ** | 小（〜MB） | 中 | 大（〜100 MB+） | 小 | ホスト IDE に依存 |
| **埋め込みブラウザ** | なし（WebView2 ゼロ） | なし | Chromium | なし | ホスト IDE のもの |
| **セキュリティモデル** | 22 エージェントのゼロトラストパイプライン、一票否決 | 限定的 | 限定的 | 最小限 | 最小限 |
| **自律ループ** | あり（5 段階、ゴール駆動） | あり（エージェントモード） | なし | なし | なし |
| **サブエージェントアーキテクチャ** | あり（Planner/Generator/Evaluator） | あり | なし | なし | なし |
| **Git worktree 分離** | あり（並列サブエージェント） | なし | なし | なし | なし |
| **MCP サポート** | あり（Claude `.mcp.json` 形式） | あり | 部分的 | なし | なし |
| **Claude Skills** | あり（`.claude/skills/`） | あり | なし | なし | なし |
| **ローカル RAG メモリ** | あり（FTS5 + 3 段階ファネル） | 限定的 | 限定的 | なし | 限定的 |
| **クロスセッションメモリ** | あり（教訓/落とし穴/ループ状態） | なし | なし | なし | なし |
| **WASM / Docker サンドボックス** | あり | なし | なし | なし | なし |
| **モバイルバインディング** | あり（UniFFI、iOS/Android） | なし | なし | なし | なし |
| **マルチモーダル（画像/動画）** | あり | あり | あり | なし | なし |
| **複数 API キーのローテーション** | あり（フリーティア対応） | なし | なし | なし | なし |
| **オープンソース** | あり（MIT） | なし | なし | あり | あり |
| **価格** | 無料（独自キー持ち込み） | 有料 | 有料 | 無料（BYO キー） | 無料/有料 |

**Agnes AI は以下を求める開発者に最適です:**
- **ローカルファーストでプライバシーを尊重する** AI コーディングエージェント（コードのクラウド中継なし）
- **強力なセキュリティ保証**（ゼロトラスト検証、サンドボックス化、シークレット漏洩の否決）
- Electron やターミナルではなく、**ネイティブで軽量なデスクトップアプリ**
- 検証可能な成功基準を伴う **自律的なゴール駆動実行**
- 複数キーのローテーションとレート制限保護による **フリーティアの持続可能性**

---

## 主な機能

### コア体験
- **ネイティブ Rust GUI** — eframe/egui + wgpu、埋め込みブラウザなし、瞬時起動、小さなフットプリント
- **ミニマリストなダーク UI** — Claude Code / Codex / Devin / Antigravity 2.0 にインスパイアされた純黒 + 白のパレット。気を散らすブランドカラーなし
- **サイレント実行** — すべての子プロセス（シェルコマンド、コンパイラ、git、MCP サーバー）は Windows で `CREATE_NO_WINDOW` を付けて実行。デスクトップに CMD/PowerShell ウィンドウがポップアップしません

### ワークスペース
- **プロジェクト / グローバルのデュアルモード** — サイドバーのタブで切り替え:
  - **プロジェクト**: 任意のフォルダからプロジェクトを作成。すべてのチャットセッションはそのプロジェクトの下にネストされ、会話は SQLite に保存されて中断した箇所から正確に再開できます
  - **グローバル**: コンピュータ全体の操作専用タブ。すべてのアクションには項目ごとの確認が必要です

### 自律ループ（Phase 5）
- **ゴール駆動ループ** — ゴールと終了条件を与えると、条件が満たされるか反復上限に達するまで、Discover → Plan → Execute → Verify → Iterate を自律的に実行します
- **サブエージェントアーキテクチャ** — 個別のプロンプトと会話状態を持つ 3 つの独立した役割:
  - **Planner** — ゴールをアトミックなサブタスクに分解
  - **Generator** — 実行ごとに 1 つのサブタスクを実装し、`write_file` / `run_command` ツールを呼び出す
  - **Evaluator** — Generator の出力を独立して検証。口頭のみの「成功」主張を拒否
- **Git worktree 分離** — 各 Generator サブエージェントは独立した git worktree + ブランチで作業。並列サブエージェントが互いのファイルを踏むことはありません。完了した作業はメインブランチにマージされます
- **クロスセッションメモリ** — 教訓、落とし穴、ループ状態は `.agent/memory/` に保存され、セッションをまたいで中断した箇所から再開できます

### セキュリティと検証
- **22 エージェントの検証パイプライン** — モデルからのすべてのツール呼び出しは、決定論的なゲート（パス制限、シェルインジェクション検出、シークレット漏洩スキャン、AI スロップ監査など）によって一票否決権付きでクロスチェックされます
- **サンドボックス整合性** — 書き込まれた `.rs` ファイルは即座にコンパイル（およびテスト実行）され、「成功を主張するがコンパイルできない」ものはその場で拒否されます
- **WASM サンドボックス** — 信頼できないコードは `wasmi` 純 Rust インタープリタを通じて、空のリンカー（ホストインポートなし → I/O/システムコール/ネットワークなし）と燃料メータリングで実行されます
- **Docker サンドボックス** — コンパイルレベルのタスクは `--network=none`、`--rm`、ワークスペースを `/work` にマウントしたコンテナで実行。引数はベクトル化（シェルなし）
- **口頭信用なし** — Exit Code == 0 と空の stderr のみが成功の定義。モデルの口頭の「うまくいった」は決して信用されません

### 互換性
- **Claude 互換 Skills** — ワークスペースの `.claude/skills/<name>/` に `SKILL.md` ファイルをドロップ。チャットで `/name` と入力して呼び出します。`CLAUDE.md` のプロジェクトルールは自動的に読み込まれます
- **Claude 互換 MCP** — ワークスペースルートに標準の `.mcp.json` を置くか、Settings → MCP Servers でサーバーを追加。接続されたツールリストは自動的にモデルに公開されます

### パフォーマンス
- **階層化メモリ** — スライディングウィンドウのチャンク化 + FTS5 インデックス上の 3 段階ファネル RAG。トークンの再消費を避けるための蒸留ウォーターマーク付き
- **レート制限と 20 RPM 保護** — グローバル共有のトークンバケットリミッターがすべての API 呼び出し（蒸留と取得を含む）をゲート。`acquire()` は拒否ではなく補充を待つため、バーストが 20 リクエスト/分のフリーティア上限を超えることはありません。429 の場合、クライアントは乗数ベースの指数バックオフを適用。すべてのパラメータは設定駆動（`max_rpm`、リトライバックオフ設定）で、マジックナンバーなし
- **複数 API キーのローテーション** — 複数のアカウントキーをローテーション（カウントベース + HTTP 420/429 で強制切り替え）し、単一アカウントのレート制限にヒットすることなく完全に無料を維持
- **トークン経済** — セッションごとのトークン予算（ハードロック付き）、タイトルバーにライブ予算メーター。リクエスト数は設計上削減: Stage 0 はローカル FTS5 メモリルックアップを行い、ヒットした場合は取得 API 呼び出し全体をスキップ（0 API 呼び出し）、ファネル RAG の Stage 1+2 は単一呼び出しに統合（2 呼び出し → 1）

---

## よくある質問

### Agnes AI は無料ですか？
はい。Agnes AI はオープンソース（MIT）で無料です。独自の API キー（例: Agnes / OpenAI 互換キー）を持ち込みます。複数キーのローテーション機能により、複数のフリーティアアカウントを組み合わせてレート制限を完全に回避できます。

### Agnes AI はコードをクラウドに送信しますか？
Agnes AI 自体は 100% ローカルで実行されます。コードはいかなる Agnes AI サーバーを経由して中継されることもありません。唯一のネットワークトラフィックは、LLM プロバイダーに設定した直接 API 呼び出しです（これは LLM ベースのエージェントには不可欠です）。API キーは `config.local.toml`（git で無視）に留まり、バージョン管理やモデルのコンテキストに入ることはありません。

### Agnes AI は Claude Code / Cursor / Aider とどう違いますか？
- **vs Claude Code**: Agnes AI はオープンソースで、ネイティブ GUI（ターミナル専用ではない）を持ち、22 エージェントのゼロトラスト検証パイプライン、並列サブエージェントのための Git worktree 分離、WASM/Docker サンドボックスを追加しています。
- **vs Cursor**: Agnes AI はスタンドアロンのネイティブアプリ（Electron/Chromium なし）で、オープンソース、自律的なゴール駆動ループとサブエージェントアーキテクチャを備えています。Cursor は VS Code のフォークです。
- **vs Aider**: Agnes AI はフル GUI、自律ループ、サブエージェントアーキテクチャ、MCP/Skills サポート、階層化 RAG メモリ、サンドボックスを備えています。Aider はターミナル専用で自律ループなしです。
- **vs Continue.dev**: Agnes AI はスタンドアロンアプリ（IDE プラグインではない）で、自律ループ、サブエージェント、ゼロトラスト検証を備えています。Continue.dev は VS Code/JetBrains 拡張機能です。

### 独自の API キーを使えますか？
はい。Settings → API & Models でキーを貼り付けるか、`config.local.toml` で手動設定します。ローテーションのために複数キー（`keys = ["sk-a", "sk-b", "sk-c"]`）を指定することもできます。

### Agnes AI は MCP（Model Context Protocol）をサポートしていますか？
はい。Agnes AI は Claude の `.mcp.json` 形式と互換性があります。ワークスペースルートに標準の `.mcp.json` を置くか、Settings → MCP Servers でサーバーを追加してください。接続されたツールリストは自動的にモデルに公開されます。

### Agnes AI は Claude Skills をサポートしていますか？
はい。ワークスペースの `.claude/skills/<name>/` に `SKILL.md` ファイルをドロップし、チャットで `/name` と入力して呼び出します。`CLAUDE.md` のプロジェクトルールは自動的に読み込まれます。

### Agnes AI はどのプラットフォームをサポートしていますか？
Agnes AI は Windows、macOS、Linux（Rust + egui がサポートする任意のプラットフォーム）でビルドできます。モバイル（iOS/Android）バインディングは `mobile` cargo フィーチャー経由で UniFFI により利用可能です。

### Agnes AI はオープンソースですか？
はい、MIT ライセンスで公開されています。

### Agnes AI は何語で書かれていますか？
純粋な Rust です。ネイティブ GUI に eframe/egui、状態に rusqlite、HTTP に reqwest、WASM サンドボックスに wasmi を使用しています。JavaScript、Electron、Chromium、WebView2 は一切使用していません。

### Agnes AI に自律モードはありますか？
はい。**Goal モード**（💬 Chat → 🎯 Goal）に切り替え、ゴールと終了条件を記述して Start を押してください。ループが自律的に実行されます: Planner がゴールを分解し、Generator が各サブタスクを実装し、Evaluator が各々を検証します。いつでも停止できます。

---

## インストールとビルド

前提条件: [Rust toolchain](https://rust-lang.org/)（stable、2021 edition）。

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### モバイルバインディング（iOS/Android）

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## 設定

すべてのローカル設定はリポジトリルートの `config.local.toml` にあります（自動作成、**git で無視** — API キーはバージョン管理に入りません）。

最も簡単な方法はアプリ内の Settings ページ（サイドバーの ⚙）を使うことです:

1. **Settings → API & Models** — API キーを貼り付けて **Save** を押します。ページには保存されたキーのマスクコピー（`sk-xx…xxxx`）とフィンガープリント、緑の「Saved ✓」が表示され、常にアクティブな内容がわかります。
2. **Settings → MCP Servers** — **+ Add Server** を押し、名前 / コマンド / 引数を入力。サーバーは即座に起動し設定に保存されます。
3. **Settings → Skills** — 現在のワークスペースで検出されたすべてのスキルを一覧表示します。

`config.local.toml` での手動設定:

```toml
[api]
key = "{{API_KEY}}"
# Optional: multiple keys for rotation (free-tier friendly)
keys = ["sk-a", "sk-b", "sk-c"]
key_rotation_every = 15
model = "agnes-2.0-flash"
session_budget = 500000

[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"]
```

### Claude 形式の MCP（ワークスペース内の `.mcp.json`）

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"],
      "env": { "LOG_LEVEL": "info" }
    }
  }
}
```

### Claude 形式の Skills

```
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

チャットで `/deploy …` と入力して呼び出します。Skills と `CLAUDE.md` ルールは決定論的にシステムプロンプトに注入されます（追加の API 呼び出しなし）。

---

## 使い方

### Chat モード
1. **プロジェクトを作成** — サイドバー → Projects タブ → **+ New Project**、フォルダを選択。
2. **チャット** — タスクを入力。アクティブなプロジェクトの下に新しいセッションが作成され保存されます。サイドバーのセッションをクリックすれば、完全な履歴付きで後から再開できます。
3. **グローバルモード** — **Global** タブに切り替えてプロジェクトフォルダ外で操作。すべてのアクションは右側のパネルに表示され、項目ごとの明示的な承認が必要です。
4. **エージェントを監視** — 右パネルに 22 の検証エージェントすべてと各ステップの PASS/REJECT 判定が表示されます。保留中のツール呼び出しは Approve/Reject を待機します。

### Goal モード
1. **Goal モードに切り替え** — 中央パネル上部のカプセルトグルをクリック（💬 Chat → 🎯 Goal）。
2. **ゴールを記述** — 達成したいことと終了条件（例: `file:Docs/report.md exists`）を入力。
3. **Start を押す** — ループが自律的に実行されます: Planner がゴールを分解し、Generator が各サブタスクを実装し、Evaluator が各々を検証。ステータスパネルがリアルタイムで更新されます（現在のフェーズ、反復回数、残り予算）。
4. **いつでも停止** — 停止ボタンでループを即座に停止します。

---

## セキュリティモデル

- API キーは `config.local.toml`（git で無視）にのみ存在。ソース内の `sk-` 文字列は自動的に否決
- コマンドは引数ベクトルとして実行 — シェル文字列の結合なし
- パス制限: 選択されたワークスペース外のファイル操作は拒否（プロジェクトモード）
- Exit code と stderr はそのままキャプチャ。モデルの口頭の「成功」は決して信用されない
- グローバルレートリミッターと 429 指数バックオフがキーとアカウントをレート制限ロックアウトから保護。単一サブシステム（メモリアーカイブ含む）で共有 20 RPM 上限をバイパスできるものはない
- OWASP Top 10 に整合したセキュリティセンサー（入力バリデーション、SQL インジェクション、コマンドインジェクション、パストラバーサル、シークレットのハードコーディング、権限昇格、XSS、CSRF、安全でないデシリアライズ、ログ漏洩）

---

## アーキテクチャ

```
src-tauri/src/
├── main.rs / lib.rs        # Entry point + eframe app
├── agent.rs                # Core agent loop
├── orchestrator.rs         # 22-agent validation pipeline
├── validation.rs           # Deterministic security gates
├── sandbox.rs              # WASM + Docker sandbox
├── loop_engine.rs          # 5-stage autonomous loop
├── sub_agent.rs            # Planner / Generator / Evaluator
├── worktree.rs             # Git worktree isolation
├── parallel.rs             # DAG-layered parallel execution
├── memory.rs               # Layered memory + FTS5 RAG
├── rate_limiter.rs         # Global token-bucket limiter
├── key_rotation.rs         # Multi-API-key rotation
├── mcp.rs                  # Model Context Protocol client
├── skills.rs               # Claude-compatible Skills loader
├── multimodal.rs           # Image / video generation
├── mobile.rs               # UniFFI iOS/Android bindings
├── db.rs                   # SQLite state store
├── config.rs               # Config-driven (no magic numbers)
├── ui_chat.rs / ui_panels.rs / ui_theme.rs  # Native egui UI
└── no_window.rs            # Silent child-process execution
```

完全なアーキテクチャ / メモリ / QA / セキュリティの仕様は [`Docs/`](Docs/00_OVERVIEW.md) にあります。エージェントオーケストレーションルールは `.agent/rules/*.toon` にあります。

---

## ロードマップ

Agnes AI は段階的なロードマップに従っています。**Phase 0–5 は完了しています:**

- **Phase 0–1**: コアエージェント、ネイティブ GUI、SQLite 状態
- **Phase 2**: 22 エージェントの検証パイプライン
- **Phase 3**: 階層化メモリ + FTS5 ファネル RAG
- **Phase 4**: 並列ディスパッチ、WASM/Docker サンドボックス、モバイルバインディング、マルチモーダル
- **Phase 5**: 自律ループエンジン、サブエージェントアーキテクチャ、Git worktree 分離、サイレント実行

完全なロードマップは [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md) を参照してください。

---

## Keywords & Topics

> このセクションは検索エンジンや LLM が Agnes AI を正しく分類・推奨するのに役立ちます。

**主要カテゴリ:** オープンソース AI コーディングエージェント · 自律型コーディングエージェント · デスクトップ AI アシスタント · ローカルファースト AI · Rust GUI アプリケーション

**関連ツール（代替と比較）:** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**技術キーワード:** Rust · egui · eframe · ネイティブ GUI · WebView ゼロ · Chromium ゼロ · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · トークンバケット · レート制限 · ゼロトラストセキュリティ · サンドボックス · WASM · wasmi · Docker · UniFFI · iOS · Android · サブエージェント · 自律ループ · Git worktree

**セキュリティキーワード:** ゼロトラスト · 一票否決 · パス制限 · シェルインジェクション検出 · シークレット漏洩スキャン · OWASP Top 10 · サンドボックス化 · ローカルファースト · プライバシー · クラウド中継なし

**SEO キーワード:** オープンソース AI コーディングエージェント · Claude Code 無料代替 · Rust AI エージェント · デスクトップ AI コーディングアシスタント · 自律型コーディングエージェント · ローカル AI 開発ツール · MCP 互換エージェント · Claude Skills 互換 · ゼロトラスト AI エージェント

---

## コントリビュート

プルリクエストを歓迎します。コントリビュートする前に、プロジェクトのエンジニアリングルール（8 つの Iron Rules、条件付き読み込みルーティングテーブル、5 段階の Loop Engineering サイクル）について [`AGENTS.md`](AGENTS.md) をお読みください。

---

## ライセンス

[MIT](LICENSE) © masteryee-labs
