# Agnes AI — 專案總覽與差距分析

> 形態：高防禦、極速桌面端軟體（Rust + eframe/egui 原生 GUI，零 Chromium/WebView2，預留 UniFFI 行動端擴充）
> 目標：無限上下文分層記憶、0 虛假回報、0 遺忘、0 語意殘渣、零信任防禦的**自主代理**（含子代理）
> 架構對齊：Loop Engineering（5 階段迴圈）+ Harness Engineering（Guides×Sensors 四象限）
> 當前階段：Phase 0–5 已完成（自主代理 + 真子代理 + 極簡黑+白暗色 UI + 無視窗執行）

## 文件索引

| 檔案 | 內容 |
|---|---|
| [01_ARCHITECTURE.md](01_ARCHITECTURE.md) | 模組架構與資料流（對齊現有 `src-tauri/src/` 程式碼） |
| [02_MEMORY_SYSTEM.md](02_MEMORY_SYSTEM.md) | 滑動視窗分塊、三階段漏斗 RAG、memory_tags 蒸餾管線 |
| [03_QA_AUTOPILOT.md](03_QA_AUTOPILOT.md) | 全自動 QA：API 回傳驗證閘門與提示詞自修正迴圈 |
| [04_TOKEN_ECONOMY.md](04_TOKEN_ECONOMY.md) | Token 極致節約策略與「邏輯優先於 LLM」演算法 |
| [05_SECURITY_MODEL.md](05_SECURITY_MODEL.md) | 零信任模型：沙盒對齊、確認閘門、金鑰隔離、全域模式 |
| [06_AGENT_TEAM.md](06_AGENT_TEAM.md) | 22 人專家團隊與多代理編排協議 |
| [07_UI_SPEC.md](07_UI_SPEC.md) | 介面規格：極簡黑+白暗色模式、無視窗執行、Projects、模式切換、確認面板 |
| [08_ROADMAP.md](08_ROADMAP.md) | 接續開發路線圖（Phase 0–5） |
| [09_QA_REPORT.md](09_QA_REPORT.md) | 真實 QA 報告：API 大中小任務實測 + 自我截圖驗證 |
| [10_TEST_INFRA.md](10_TEST_INFRA.md) | 測試基礎設施：整合測試 + e2e + QA 回歸語料庫 |
| [11_TEST_READY.md](11_TEST_READY.md) | 測試就緒檢查清單 |

## 現況（2026-06）

> **架構定位**：目前是「反應式工具」（使用者 prompt → 執行一次）。
> Phase 5 升級目標：成為「自主代理」——能自己尋找工作、派發子代理、驗證結果、跨 Session 記憶。
> 對齊 Loop Engineering 六組件：Automations（5A）、Worktree（5C）、Skills（已有）、Connectors（已有）、Sub-agents（5B）、Memory（5D）。

已完成（`src-tauri/`，約 4,000 行 Rust）：

- `orchestrator.rs` — Orchestrator、SubAgent 調度、ConfirmationGate、PendingAction 風險分級、自愈執行 `execute_task_with_healing`、多資料夾 `execute_multi_folder`、全域模式 `global_execute`
- `agent.rs` — AgentLoop（run_step / parse_tool_calls / execute_tool）、AuditResult 一票否決機制
- `sandbox.rs` — 引數向量化分離、程式白名單/黑名單、路徑越界檢查、間接 shell 注入檢查、`run_in_sandbox` Exit Code + Stderr 硬性擷取；WASM 沙盒 `run_wasm_func`（wasmi 直譯器：空 host import + fuel 計量隔離不可信代碼）、Docker 沙盒 `run_in_docker_sandbox`（`--network=none` 斷網、偵測缺失自動降級）
- `db.rs` — SQLite 確定性狀態機：tasks / execution_logs / audit_logs / projects / conversations
- `config.rs` — `config.local.toml` 金鑰隔離、`ensure_gitignore` 自動屏蔽、全組態結構體（零 Magic Number）
- `locale.rs` — Windows `chcp 65001` / Unix `LANG=zh_TW.UTF-8` 語系校準
- `mcp.rs` — MCP 伺服器管理（含 env 注入；GUI 啟動時自動拉起啟用的伺服器）
- `skills.rs` — Claude 互通層：`.claude/skills/*/SKILL.md` 技能、`CLAUDE.md` 規則、`.mcp.json` MCP 設定的確定性解析與系統提示注入
- `memory.rs` — 滑動視窗分塊 + 三階段漏斗 RAG + FTS5 索引；Stage 0 本機記憶查詢命中即跳過檢索 API，Stage 1+2 已合併為單次呼叫
- `rate_limiter.rs` — 全域共享令牌桶限流器：把關每一個 Agnes API 呼叫，20 RPM 上限、`acquire()` 等待補充而非拒絕、429 倍率式指數退避；`max_rpm = 0` 可停用上限（測試用）
- `parallel.rs` — DAG 分層並行原語：`compute_dag_layers`（Kahn 拓樸、偵測環）+ `run_layers_parallel`（同層 tokio JoinSet 並行、確定性還原）；`dispatch_subagents` 改用分層、`execute_multi_folder_parallel` 多資料夾並行建構
- `multimodal.rs` — MultimodalMediaSpecialist（動態激活）：Agnes Image 2.1 Flash / Video V2.0 客戶端，`is_visual_intent` 確定性意圖偵測，媒體呼叫共用 rate_limiter
- `mobile.rs` + `agnes.udl` — UniFFI 行動端綁定（`--features mobile`）：版本、組態摘要、視覺意圖、token 估算等確定性 API 匯出供 iOS/Android 殼層
- `ui/` — egui 前端原型（22 代理人側欄、Projects、工作區、標題列即時預算計）
- `.agent/rules/` — agents.toon（22 代理人）、security.toon、verification.toon、memory.toon、harness.toon（條件式載入，見 AGENTS.md 路由表）
- `tests_integration.rs` — 整合測試；`tests/e2e_tests.rs` — 端到端測試；`tests/fixtures/qa_corpus/` — QA 回歸語料庫（`cargo test qa_replay`）
- TokenBudgeter — Session 級 token 預算硬鎖：預算耗盡後僅允許確定性（非 API）操作繼續

## 差距分析（實作進度與剩餘缺口）

| # | 缺口 | 對應文件 | 優先級 | 狀態 |
|---|---|---|---|---|
| 1 | 第一組「記憶蒸餾與防幻覺組」代理人——22 代理人全數定義並路由（非 17/22） | 02、06 | P0 | 實作完成 |
| 2 | 無限上下文：滑動視窗分塊 + 三階段漏斗 RAG + FTS5（`memory.rs`） | 02 | P0 | 實作完成 |
| 3 | 全自動 QA：回歸測試語料庫（`tests/fixtures/qa_corpus/` + `cargo test qa_replay`）、e2e 測試（`tests/e2e_tests.rs`）、提示詞自修正 | 03 | P0 | 實作完成 |
| 4 | 寫檔前 22 道交叉驗證管線形式化 | 06 | P1 | 實作完成 |
| 5 | Token 經濟：TokenBudgeter 預算器、全域令牌桶限流器（`rate_limiter.rs`，20 RPM + 429 退避）、Stage 0 FTS5 旁路、Stage 1+2 合併 | 04 | P1 | 實作完成 |
| 6 | 多子代理人並行執行：`parallel.rs` 提供 Kahn 分層 + 同層 tokio JoinSet 並行引擎；`dispatch_subagents` 改 DAG 分層（去除 O(n²) 重跑）、`execute_multi_folder_parallel` 多資料夾並行 | 01、08 | P1 | 實作完成 |
| 7 | WASM 沙盒（`run_wasm_func`，wasmi 直譯器 + 空 host import + fuel）與 Docker 沙盒（`run_in_docker_sandbox`，`--network=none`）| 05、08 | P2 | 實作完成 |
| 8 | UniFFI 行動端綁定（`mobile.rs` + `agnes.udl`，`--features mobile`）與多模態媒體（`multimodal.rs`，動態激活）| 08 | P3 | 實作完成 |
| 9 | **自主迴圈引擎**（`loop_engine.rs`）：目標驅動 Discover→Plan→Execute→Verify→Iterate，退出條件明確 | 01、08 | P0-Phase5 | 實作完成 |
| 10 | **真子代理架構**（`sub_agent.rs`）：Planner/Generator/Evaluator 獨立 AgentLoop，evaluator-optimizer 模式 | 01、06 | P0-Phase5 | 實作完成 |
| 11 | **Git Worktree 隔離**（`worktree.rs`）：多 Generator 子代理平行不打架 | 01、08 | P1-Phase5 | 實作完成 |
| 12 | **跨 Session 記憶整合**：`memory.rs` 讀寫 `.agent/memory/` 三檔（loop_state/lessons/pitfalls） | 02、08 | P1-Phase5 | 實作完成 |

> 工程備註：WASM 沙盒選用 `wasmi`（純 Rust 直譯器）而非 `wasmtime`——對「執行不可信片段」而言，直譯器無 JIT 攻擊面、無系統依賴、編譯極輕，且空 Linker + fuel 即達完全隔離，較重量級 JIT 更契合本專案的高防禦與極速定位。UniFFI 採官方現行 proc-macro 法，`agnes.udl` 作為等價介面定義文件並存。多資料夾並行與多模態已於 v0.8.1 接入 GUI 即時流程（`handle_send`）：App 級共享令牌桶（`AppState.rate_limiter`）讓並行的多資料夾代理與多模態呼叫共用同一 20 RPM 桶；視覺意圖（`is_visual_intent`）觸發圖片生成並回貼聊天——端點 `POST /v1/images/generations` 已實測回真實圖片 URL（~40–50s/張，故 `multimodal.timeout_seconds` 預設 180s；影片端點為單數 `/v1/video/generations`）。安全紅隊測試見 `tests/red_team.rs`（D2–D8 + WASM 隔離，17/17，0 穿透）。

## 鋼鐵戒律（全專案不可違反）

1. 說明文件一律進 `Docs/`；代理規則一律 `.toon` 格式進 `.agent/rules/`
2. 金鑰只存在 `config.local.toml`（已在 .gitignore），任何 `.rs` 出現 `sk-` 開頭字串 = 一票否決
3. 不信任模型口頭報告：Exit Code == 0 且 stderr 為空才算成功
4. 進度狀態只信 SQLite（`agnes_state.db`），不信對話上下文
5. Shell 執行前必先語系校準（`locale.rs`）
6. 零 Magic Number：所有數值進 `Config` 結構體（`config.rs`）
7. 修正階段只輸出 Git Diff 增量，未變更代碼不重複輸出
8. 寫檔前必過驗證管線（見 03 與 `.agent/rules/verification.toon`）
