# 01 — 系統架構

## 技術底座

- 語言：Rust 2021 Edition
- GUI：eframe/egui 0.31（原生渲染，**零 Chromium、零 WebView2**）
- 非同步：tokio（full features）
- 狀態：rusqlite（bundled SQLite）
- HTTP：reqwest + rustls-tls（不依賴系統 OpenSSL）
- 組態：toml（`config.local.toml` 本機隔離）
- 行動端（預留）：crate-type 已含 `staticlib`/`cdylib`，供 UniFFI 綁定

註：`src-tauri/tauri.conf.json` 為早期 Tauri 殘留；目前實際進入點是 `[[bin]] agnes-ai`（eframe）。Phase 0 應清除 Tauri 設定檔以免誤導（見 08_ROADMAP）。

## 模組地圖（src-tauri/src/）

```
main.rs          eframe 進入點，視窗初始化
lib.rs           模組匯出與共用入口
config.rs        Config{api, sandbox, security, general, mcp} 全組態 + 金鑰讀寫 + ensure_gitignore
locale.rs        OS 語系探針：Windows chcp 65001 / Unix LANG=zh_TW.UTF-8
db.rs            SQLite 狀態機：tasks / execution_logs / audit_logs / projects / conversations / goals / sub_agent_runs
sandbox.rs       零信任執行：引數消毒、白黑名單、路徑圈禁、run_in_sandbox(ExitCode+Stderr)
no_window.rs     Windows 無視窗執行 helper（CREATE_NO_WINDOW），所有子進程靜默不彈窗
agent.rs         AgentLoop：run_step → parse_tool_calls → run_audits → execute_tool（單輪執行單元）
orchestrator.rs  Orchestrator：dispatch_subagents（驗證 gate）、dispatch_real_subagents（真子代理）、ConfirmationGate、自愈循環
loop_engine.rs   AutonomousLoop：目標驅動自主迴圈（Discover→Plan→Execute→Verify→Iterate）— Phase 5
sub_agent.rs     SubAgentInstance：獨立 AgentLoop + 角色特定 prompt（Planner/Generator/Evaluator）— Phase 5
worktree.rs      WorktreeManager：git worktree 隔離，多子代理平行不打架 — Phase 5
mcp.rs           McpManager：外部 MCP 伺服器整合（Connectors）
memory.rs        滑動視窗分塊 + 三階段漏斗 RAG + FTS5 + .agent/memory/ 跨 Session 記憶
parallel.rs      DAG 分層並行原語（Kahn 拓樸 + tokio JoinSet）
validation.rs    22-gate 驗證管線（Sensor，Verify 階段啟動）
tests_integration.rs  整合測試
```

## 資料流（單次任務生命週期）

```
使用者 Prompt（egui UI）
  │
  ▼
[1] locale.rs 語系校準（一次性探針）
[2] db.rs 讀取 SQLite 真實狀態 → 硬編碼注入 System Context（防遺忘）
[3] 漏斗 RAG 三階段檢索 memory_tags/（見 02，已實作 memory.rs）
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

### 資料流（自主迴圈模式，Phase 5）

```
使用者設定 Goal（egui UI）
  │
  ▼
[1] AutonomousLoop 啟動（loop_engine.rs）
  │
  ▼
[2] Discover：讀 SQLite 真實狀態 + RAG 檢索 + .agent/memory/lessons.md + pitfalls.md
  │
  ▼
[3] Plan：Planner 子代理拆解目標 → 子任務列表寫入 SQLite tasks 表
  │
  ▼
[4] Execute：Generator 子代理在 git worktree 隔離中執行子任務
  │         └─ 呼叫 AgentLoop.run_step（內層單輪，同上方 [5]–[9]）
  │
  ▼
[5] Verify：Evaluator 子代理獨立驗證（evaluator-optimizer 模式）
  │         └─ REJECT → Delta 回饋 Generator，最多 3 輪
  │         └─ 通過 → 進入 22 道驗證 gate（Sensor 層）
  │
  ▼
[6] Iterate：未過 → Delta-only 修；3 輪同失敗碼 → 升級 premium → 再失敗 → FAILED 停止
  │
  ▼
[7] 通過 → merge worktree → 寫入磁碟 → 蒸餾 loop_state.md（≤3 行）
  │
  ▼
[8] 目標達成 OR 迭代上限 → 蒸餾 lessons.md + 清空 loop_state → 迴圈結束
```

## 多子代理人並行模型（已實作，P1）

`dispatch_subagents` 已改為 DAG 分層 + tokio JoinSet 同層並行（`parallel.rs`）。

- **tokio JoinSet 並行**：互不依賴的代理（例：Distiller Alpha 與 Beta、各審查員）以 `tokio::task::JoinSet` 並行執行，依 `agents.toon` 的 `prerequisites` 欄位建 DAG，拓撲排序後同層並行。
- **休眠即零成本**：未被路由的代理不建立 task、不佔 API 呼叫——「激活」的唯一語意是進入 JoinSet。
- **執行緒池上限**：worker 數 = CPU 核心數，進 `Config.general`，禁止 Magic Number。
- **共享狀態**：代理間只透過 SQLite 與訊息通道（`tokio::sync::mpsc`）溝通，禁止共享可變記憶體（避免 `Arc<Mutex>` 死鎖，呼應 MemoryEfficiencyReviewer 規則）。

> **重要區分**：`dispatch_subagents`（22 道驗證 gate）是 **Sensor**（Verify 階段），不是真子代理。
> 真子代理（Planner/Generator/Evaluator）見下方「自主迴圈 + 真子代理架構」。

## 自主迴圈 + 真子代理架構（Phase 5，對齊 Loop Engineering）

### 兩層架構：外層迴圈 + 內層單輪

```
┌─────────────────────────────────────────────────────┐
│  AutonomousLoop（loop_engine.rs）— 外層迴圈          │
│                                                     │
│  Discover → Plan → Execute → Verify → Iterate       │
│     │         │        │         │         │        │
│     │         │        │         │         │        │
│  讀 SQLite   Planner   Generator  Evaluator  未過→修 │
│  + RAG       拆子任務   執行子任務   獨立驗證    最多3輪 │
│  + lessons   → tasks   → worktree   → REJECT?       │
│  + pitfalls  表        隔離執行     → 回饋 Generator │
│                                                     │
│  退出：目標達成 OR 迭代上限 OR 3輪同失敗→premium→FAILED│
└─────────────────────────────────────────────────────┘
         │ 每輪 Execute 呼叫
         ▼
┌─────────────────────────────────────────────────────┐
│  AgentLoop.run_step（agent.rs）— 內層單輪            │
│                                                     │
│  呼叫 LLM API → parse_tool_calls → run_audits       │
│  → execute_tool（sandbox）→ Exit Code 對齊          │
└─────────────────────────────────────────────────────┘
```

### 三角色子代理（evaluator-optimizer 模式，對齊 Anthropic）

| 角色 | system prompt 核心 | 職責 | 驗證者 |
|---|---|---|---|
| Planner | 「你是規劃者，把目標分解為可執行的原子子任務列表」 | 拆解 → 寫入 SQLite `tasks` 表 | Evaluator |
| Generator | 「你是生成者，一次實作一個子任務，增量開發」 | 呼叫 `AgentLoop.run_step` 執行 | Evaluator |
| Evaluator | 「你是評估者，驗證生成結果是否符合子任務規格，禁止寬容」 | 獨立驗證 → PASS/REJECT+修復指示 | — |

- **Evaluator 不是 Generator 的同一個模型實例**——獨立 AgentLoop + 不同 system prompt，防「自己說服自己」
- Evaluator REJECT → Delta-only 回饋 Generator 修正，最多 3 輪（對齊 Stripe Minions 退出條件）
- 3 輪同失敗碼 → 升級 premium 模型重試一次 → 再失敗標 FAILED

### Git Worktree 隔離（多 Generator 平行不打架）

```
主分支 (main)
  │
  ├── worktree/agent-gen-1/  ← Generator-1 在此工作
  ├── worktree/agent-gen-2/  ← Generator-2 在此工作（平行）
  └── worktree/agent-eval-1/ ← Evaluator 在此驗證（唯讀）
```

- 每個 Generator 子代理在獨立 git worktree 中工作，共用同一份專案歷史
- 完成後 merge 回主分支；Evaluator 在唯讀 worktree 中驗證
- `WorktreeManager`（`worktree.rs`）：create / merge / cleanup

### 跨 Session 記憶（代理會忘，倉庫不會忘）

| 層 | 檔案 | 讀取時機 | 寫入時機 |
|---|---|---|---|
| 短期進度 | `.agent/memory/loop_state.md` | Discover 階段 | 每子任務完成 ≤3 行；達 40 行中段蒸餾 |
| 長期教訓 | `.agent/memory/lessons.md` | Discover 階段 | 任務完成晉升；FIFO 30 條 |
| 雷庫 | `.agent/memory/pitfalls.md` | Discover 階段 | 發現同類錯誤重複時；去重 |

`memory.rs` 擴充讀寫這三檔，`loop_engine.rs` 在 Discover 階段強制讀取。

## 三種工作模式

| 模式 | 範圍 | 安全等級 |
|---|---|---|
| Project 模式 | 使用者選定的一或多個資料夾（`Orchestrator.set_workspaces`） | 路徑圈禁於資料夾內 |
| 多資料夾模式 | `execute_multi_folder` 跨數個已授權資料夾 | 每資料夾獨立圈禁 |
| 全域模式（Hermes 式） | `global_execute` 全電腦 | 強制 ConfirmationGate 逐項確認 + AllowedPaths 白名單 + 封鎖 C:\Windows 等（見 security.toon） |

## 行動端擴充（Phase 4 預留）

- 核心邏輯全部留在 `lib.rs` 可重用層（已是 rlib/cdylib/staticlib）
- UniFFI 定義檔置於 `src-tauri/src/agnes.udl`（已建，proc-macro 法 + UDL 並存）
- GUI 層替換：egui → 各平台原生（Swift/Kotlin 透過 UniFFI 呼叫）
- 沙盒層差異：行動端無 Docker，僅 WASM/行程隔離
