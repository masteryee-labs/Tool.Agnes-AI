# 06 — 22 人專家團隊與寫檔前 22 道交叉驗證管線

> 規則檔：`.agent/rules/core.agents.toon`（第 2–6 組，17 人）+ `.agent/rules/memory_distillation.toon`（第 1 組 5 人，本輪新增）+ `.agent/rules/cross_validation.toon`（22 道管線定義）。

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

- `AgentEngine::run_validation`（agent.rs:414）擴充為 22-gate 執行器
- `AgentLoop::run_audits`（agent.rs:483）保留為階段 C 的 LLM 呼叫載體
- `Orchestrator.dispatch_subagents`（orchestrator.rs:232）按 `.toon` 的 `prerequisites` 建 DAG，同層並行
