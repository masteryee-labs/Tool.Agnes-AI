# 06 — 22 人專家團隊與寫檔前 22 道交叉驗證管線

> 規則檔：`.agent/rules/agents.toon`（第 2–6 組，17 人）+ `.agent/rules/memory.toon`（第 1 組 5 人）+ `.agent/rules/verification.toon`（22 道管線定義）。
>
> **兩層代理架構**：
> - **驗證層**（本檔下半部）：22 道驗證 gate = Sensor（Verify 階段啟動，0-token 為主）
> - **執行層**（本檔上半部新增）：Planner/Generator/Evaluator = 真子代理（Execute 階段，獨立 AgentLoop）

## 真子代理：Planner / Generator / Evaluator（Phase 5，evaluator-optimizer 模式）

> 對齊 Anthropic 三代理 Harness 架構 + Loop Engineering 子代理組件。
> 這是獨立的 `SubAgentInstance`（`sub_agent.rs`），不是 22 道驗證 gate。

### 三角色職責

| 角色 | system prompt 核心 | 輸入 | 輸出 | 驗證者 |
|---|---|---|---|---|
| **Planner** | 「你是規劃者。把目標分解為可執行的原子子任務列表，每項標明目標檔與風險級。」 | Goal + SQLite 現狀 + lessons + pitfalls | 子任務列表 → SQLite `tasks` 表 | Evaluator |
| **Generator** | 「你是生成者。一次實作一個子任務，增量開發，Delta-only。」 | 子任務 + worktree 隔離環境 | 程式碼變更 + 變更摘要 | Evaluator |
| **Evaluator** | 「你是評估者。獨立驗證生成結果是否符合子任務規格。禁止對自己的程式碼寬容。」 | Generator 產出 + 子任務規格 | PASS 或 REJECT+修復指示 | — |

### evaluator-optimizer 迴圈

```
Planner 拆解 → Generator 實作 → Evaluator 驗證
                    ↑                  │
                    │    REJECT        │
                    └── Delta 回饋 ────┘
                          最多 3 輪
                    3 輪同失敗 → 升級 premium → 再失敗 → FAILED
```

- **Evaluator 絕不是 Generator 的同一實例**——獨立 AgentLoop + 不同 system prompt + 不同 conversation_id
- Evaluator REJECT 訊息強制結構化：`[REJECT: 子任務ID | 檔案:行號 | 原因 | 修復指示]`
- Generator 收到 REJECT → Delta-only 修正（只改被指出的問題，禁止重寫整檔）
- 通過 Evaluator → 進入 22 道驗證 gate（Sensor 層）→ 通過 → merge worktree → 寫入磁碟

### 與 22 道驗證 gate 的關係

```
真子代理（Execute 階段）        驗證 gate（Verify 階段）
─────────────────────          ─────────────────────
Planner 拆解                   （不參與）
Generator 實作         →       22 道驗證 gate（Sensor）
Evaluator 獨立驗證     →       （可選：Evaluator 結果作為 G17 防幻覺的輸入）
```

- 真子代理是**執行層**（做事情的人）
- 22 道驗證 gate 是**感測層**（檢查的人）——對齊 Harness Engineering 的 Sensor
- 兩層獨立：Evaluator 通過 ≠ 22 gate 通過；22 gate 通過 ≠ Evaluator 通過
- 最終寫入磁碟前兩層都必須通過

## 名冊（22 人）

| # | 代理人 | 組別 | 寫檔前負責的驗證道 | 型態 | 狀態 |
|---|---|---|---|---|---|
| 1 | ContextDistillerAlpha 脈絡蒸餾專家 Alpha | 1 記憶蒸餾 | G19 變更前半段蒸餾（大 diff 才激活） | LLM-flash | **新增** |
| 2 | ContextDistillerBeta 脈絡蒸餾專家 Beta | 1 記憶蒸餾 | G20 變更後半段蒸餾（與 G19 並行） | LLM-flash | **新增** |
| 3 | DistillationIntegrator 蒸餾邏輯整合官 | 1 記憶蒸餾 | G21 蒸餾整合 → 記憶 .md | LLM-flash | **新增** |
| 4 | FactHallucinationAuditor 事實防幻覺審查員 | 1 記憶蒸餾 | G17 變更摘要與實際 diff 交叉比對 | LLM-flash | **新增** |
| 5 | TokenOverlapAuditor Token 額度與重疊區審查員 | 1 記憶蒸餾 | G18 記憶檔 token 上限/標籤/重疊區檢查 | Rust 0-token | **新增** |
| 6 | WorkflowTopology 工作流拓撲架構師 | 2 工作流 | G1 步驟數與 token 預算檢查 | Rust 0-token | 已有 |
| 7 | WorkflowRuntimeEvaluator 工作流運行評估員 | 2 工作流 | G2 死循環計數器（3 輪重複/50 輪上限） | Rust 0-token | 已有 |
| 8 | SlopVibeAuditor AI 語意與氛圍稽核員 | 2 工作流 | G3 banned_words 正則掃描 | Rust 0-token | 已有 |
| 9 | SlopPathPurgeSpecialist 殘渣與路徑清理專員 | 2 工作流 | G4 產物分流（.md→Docs/，.toon→.agent/rules/）+ Magic Number 檢查 | Rust 0-token | 已有 |
| 10 | OrchestratorAgent 主編排調度官 | 3 指揮 | G22 終端整合簽核（最後一道） | LLM-主力 | 已有 |
| 11 | LocaleCalibrationSpecialist 環境語系校準專家 | 3 指揮 | G5 語系環境變數已注入檢查 | Rust 0-token | 已有 |
| 12 | LeadSystemArchitect 跨平台系統首席架構師 | 3 指揮 | G6 依賴檢查（Cargo.toml 禁 Chromium/WebView 系） | Rust 0-token | 已有 |
| 13 | PerformanceArchitectureEngineer 極致效能架構師 | 4 效能 | G7 Delta-only 格式檢查（diff 可套用性） | Rust 0-token | 已有 |
| 14 | ResourceAnalyticsEngineer 資源動態分析師 | 4 效能 | G8 阻塞呼叫掃描（主執行緒禁同步 I/O） | Rust 0-token (clippy) | 已有 |
| 15 | MemoryEfficiencyReviewer 低耗能與記憶體審查員 | 4 效能 | G9 `cargo clippy -D warnings` 所有權/生命週期 | Rust 0-token | 已有 |
| 16 | SecurityArchitectureDesigner 安全架構設計師 | 5 安全 | G10 沙盒邊界組態檢查 | Rust 0-token | 已有 |
| 17 | DefensiveCodingSpecialist 防禦性編程實作專家 | 5 安全 | G11 引數向量化/拼接掃描 + .gitignore 完整性 | Rust 0-token | 已有 |
| 18 | SecurityComplianceAuditor 合規與韌性審查員 | 5 安全 | G12 金鑰 `sk-` 掃描 + 升權掃描（一票否決） | Rust 0-token | 已有 |
| 19 | CoreEngineCoder 核心引擎開發工程師 | 6 工程 | G13 完整性掃描（禁 TODO/unimplemented!/省略標記） | Rust 0-token | 已有 |
| 20 | IntegrationEngineer 跨平台服務整合工程師 | 6 工程 | G14 HTTP 客戶端規範（禁 blocking、必含重試組態） | Rust 0-token | 已有 |
| 21 | MultimodalMediaSpecialist 多模態媒體生成專家 | 6 工程 | G15 媒體產物路徑檢查（僅媒體任務激活，否則休眠跳過記 SKIP） | Rust 0-token | 已有 |
| 22 | SandboxRuntimeTester 自動化虛擬沙盒測試員 | 6 工程 | G16 `cargo check` + `cargo test` 真實 Exit Code | Rust 0-token | 已有 |

成本結構：22 道中 **17 道為純 Rust 確定性檢查（0 token）**，4 道 flash 級 LLM（且 G19/G20 並行、僅大 diff 激活），1 道主力模型簽核。一次寫檔的典型 LLM 成本 ≈ 1–2 次 flash 呼叫 + 1 次主力簽核；小變更（diff < 閾值）時蒸餾組整組休眠，僅 G17 一次 flash 呼叫。

## 分工路由演算法（validation.rs `route_dormant_agents`，v0.5.0 已實作）

每輪審查前先以一次任務特徵掃描（0 token）決定激活集合，未激活代理直接記 `DORMANT`、不執行：

| 任務特徵 | 激活的代理組 |
|---|---|
| 恆常 | G1 拓撲、G2 循環、G3 殘渣詞、G5 語系、G17 防幻覺、G18 token 上限、G22 簽核 |
| 有檔案寫入 | G4 路徑分流、G6 依賴、G8 阻塞 I/O、G12 金鑰、G13 完整性、G14 HTTP |
| 有指令執行（或寫檔） | G7 循環/等待、G10 沙盒參數、G11 Shell 注入 |
| 寫入 `.rs` | G9 clippy、G16 cargo check（重量級，純聊天絕不觸發） |
| 媒體關鍵詞 | G15 多模態 |
| 對話 tokens ≥ distill_trigger_delta | G19/G20/G21 蒸餾組（整組同進退） |

G17 防幻覺已具確定性實作：助理宣稱「已建立/寫入檔案」但無對應寫檔工具呼叫 → 一票否決。
G18：提議寫入的 `.md` 超過 `md_token_cap` → 一票否決。
G22 為真簽核：彙整前 21 道裁決，任一 REJECT 即整案 REJECT 並列出否決者名單。
每輪 22 筆裁決以取代式寫入 `conversation_audits` 表，GUI 右側面板按 Session 還原。

## 22 道交叉驗證管線（寫檔前強制執行順序）

```
提議的檔案變更（統一 diff 格式）
  │
  ├─ 階段 A：靜態確定性（並行，全部 0 token）
  │   G1 預算 → G2 循環 → G3 殘渣詞 → G4 路徑分流 → G5 語系
  │   G6 依賴 → G10 沙盒組態 → G11 防禦編碼 → G12 金鑰(否決)
  │   G13 完整性 → G14 HTTP 規範 → G15 媒體路徑(或 SKIP)
  │
  ├─ 階段 B：編譯期確定性（順序，0 token）
  │   G7 diff 可套用 → 套用至暫存工作樹 → G9 clippy → G8 阻塞掃描
  │   → G16 cargo check + test（真實 Exit Code）
  │
  ├─ 階段 C：語意審查（flash 級）
  │   G17 變更摘要 vs 實際 diff 交叉比對（防幻覺：聲稱做了 A 實際改了 B → REJECT）
  │   G18 若產出記憶檔：token 上限/標籤/重疊區（0 token）
  │   G19+G20 大 diff 時並行蒸餾 → G21 整合為記憶 .md
  │
  └─ 階段 D：簽核
      G22 Orchestrator 終端整合審查 → [PASS] 才真正寫入磁碟
                                      → 任一 REJECT：帶失敗碼回 03 自修正迴圈
```

實作要點（`src-tauri/src/validation.rs`，待實作 P1）：

- 每道 gate 是 `trait Gate { fn check(&self, diff: &ProposedChange) -> GateResult }`，回傳 `Pass / Reject{code, line, chain} / Skip{reason}`
- 22 道結果全部寫入 `audit_logs`（gate 編號、結果、耗時、token 成本）——「交叉驗證」的證據可回放
- 階段 A 以 `rayon`/`JoinSet` 並行，總延遲 < 1 秒
- REJECT 訊息強制結構化 `[REJECT: G12 | src/api.rs:42 | 偵測硬編碼金鑰前綴]`，禁止感性語句（SlopVibeAuditor 自我約束）

## 與既有代碼的接點

### 驗證層（22 道 gate，已實作）

- `AgentEngine::run_validation`（agent.rs）為 22-gate 執行器
- `AgentLoop::run_audits`（agent.rs）保留為階段 C 的 LLM 呼叫載體
- `Orchestrator.dispatch_subagents`（orchestrator.rs）按 `.toon` 的 `prerequisites` 建 DAG，同層並行
- `validation.rs`：`route_dormant_agents` 依任務特徵激活子集，未激活記 DORMANT

### 執行層（真子代理，Phase 5 待實作）

- `loop_engine.rs`（新）：`AutonomousLoop` 外層迴圈，每輪 Discover→Plan→Execute→Verify→Iterate
- `sub_agent.rs`（新）：`SubAgentInstance` 持有獨立 `AgentLoop` + 角色特定 system prompt
- `worktree.rs`（新）：`WorktreeManager` git worktree 隔離，多 Generator 平行
- `orchestrator.rs` 擴充：`dispatch_real_subagents`（與 `dispatch_subagents` 驗證 gate 並存）
- `memory.rs` 擴充：讀寫 `.agent/memory/` 三檔（loop_state/lessons/pitfalls）
- `db.rs` 新表：`goals`（目標驅動）+ `sub_agent_runs`（子代理執行記錄）
