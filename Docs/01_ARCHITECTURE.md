# 01 — 系統架構

## 技術底座

- 語言：Rust 2021 Edition
- GUI：eframe/egui 0.31（原生渲染，**零 Chromium、零 WebView2**）
- 非同步：tokio（full features）
- 狀態：rusqlite（bundled SQLite）
- HTTP：reqwest + rustls-tls（不依賴系統 OpenSSL）
- 組態：toml（`config.local.toml` 本機隔離）
- 行動端（預留）：crate-type 已含 `staticlib`/`cdylib`，供 UniFFI 綁定

註：`src-tauri/tauri.conf.json` 為早期 Tauri 殘留；目前實際進入點是 `[[bin]] agnes-ai`（eframe）。Phase 2 應清除 Tauri 設定檔以免誤導（見 08_ROADMAP）。

## 模組地圖（src-tauri/src/）

```
main.rs          eframe 進入點，視窗初始化
lib.rs           模組匯出與共用入口
config.rs        Config{api, sandbox, security, general, mcp} 全組態 + 金鑰讀寫 + ensure_gitignore
locale.rs        OS 語系探針：Windows chcp 65001 / Unix LANG=zh_TW.UTF-8
db.rs            SQLite 狀態機：tasks / execution_logs / audit_logs / projects / conversations
sandbox.rs       零信任執行：引數消毒、白黑名單、路徑圈禁、run_in_sandbox(ExitCode+Stderr)
agent.rs         AgentLoop：run_step → parse_tool_calls → run_audits → execute_tool
orchestrator.rs  Orchestrator：dispatch_subagents、ConfirmationGate、自愈循環、多資料夾/全域模式
mcp.rs           McpManager：外部 MCP 伺服器整合
tests_integration.rs  整合測試
```

## 資料流（單次任務生命週期）

```
使用者 Prompt（egui UI）
  │
  ▼
[1] locale.rs 語系校準（一次性探針）
[2] db.rs 讀取 SQLite 真實狀態 → 硬編碼注入 System Context（防遺忘）
[3] 漏斗 RAG 三階段檢索 memory_tags/（見 02，待實作 memory.rs）
  │
  ▼
[4] Orchestrator.dispatch_subagents — 解析任務 → 選定代理子集（其餘休眠）
[5] AgentLoop.run_step — 呼叫 Agnes API（reqwest）
  │
  ▼
[6] parse_tool_calls — 結構化抽取工具呼叫
[7] ★ QA 驗證閘門（見 03）：
      確定性層（0 token）：sandbox::validate_sandbox_input / 路徑圈禁 / 注入檢查
      語意層（按風險分級才呼叫 LLM）：run_audits 一票否決
  │  REJECT → 提示詞自修正迴圈（Delta-only 回饋）
  ▼
[8] ConfirmationGate — PendingAction 風險分級，使用者逐項 Approve/Reject
[9] sandbox::run_in_sandbox — 真實 Exit Code + Stderr 擷取
  │  ExitCode != 0 → execute_task_with_healing 自愈重寫（Git Diff 增量）
  ▼
[10] db.rs 寫入 execution_logs + audit_logs；task_status → SUCCESS
[11] Token 增量觸及閾值 → 喚醒蒸餾組，封裝寫入 memory_tags/（見 02）
```

## 多子代理人並行模型（待實作，P1）

目前 `dispatch_subagents` 為順序調度。目標架構：

- **tokio JoinSet 並行**：互不依賴的代理（例：Distiller Alpha 與 Beta、各審查員）以 `tokio::task::JoinSet` 並行執行，依 `core.agents.toon` 的 `prerequisites` 欄位建 DAG，拓撲排序後同層並行。
- **休眠即零成本**：未被路由的代理不建立 task、不佔 API 呼叫——「激活」的唯一語意是進入 JoinSet。
- **執行緒池上限**：worker 數 = CPU 核心數，進 `Config.general`，禁止 Magic Number。
- **共享狀態**：代理間只透過 SQLite 與訊息通道（`tokio::sync::mpsc`）溝通，禁止共享可變記憶體（避免 `Arc<Mutex>` 死鎖，呼應 MemoryEfficiencyReviewer 規則）。

## 三種工作模式

| 模式 | 範圍 | 安全等級 |
|---|---|---|
| Project 模式 | 使用者選定的一或多個資料夾（`Orchestrator.set_workspaces`） | 路徑圈禁於資料夾內 |
| 多資料夾模式 | `execute_multi_folder` 跨數個已授權資料夾 | 每資料夾獨立圈禁 |
| 全域模式（Hermes 式） | `global_execute` 全電腦 | 強制 ConfirmationGate 逐項確認 + AllowedPaths 白名單 + 封鎖 C:\Windows 等（見 confirmation_gate.toon） |

## 行動端擴充（Phase 4 預留）

- 核心邏輯全部留在 `lib.rs` 可重用層（已是 rlib/cdylib/staticlib）
- UniFFI 定義檔置於 `src-tauri/src/agnes.udl`（未建）
- GUI 層替換：egui → 各平台原生（Swift/Kotlin 透過 UniFFI 呼叫）
- 沙盒層差異：行動端無 Docker，僅 WASM/行程隔離
