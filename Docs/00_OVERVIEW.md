# Agnes AI — 專案總覽與差距分析

> 形態：高防禦、極速桌面端軟體（Rust + eframe/egui 原生 GUI，零 Chromium/WebView2，預留 UniFFI 行動端擴充）
> 目標：無限上下文分層記憶、0 虛假回報、0 遺忘、0 語意殘渣、零信任防禦的自主代碼與任務執行智慧體

## 文件索引

| 檔案 | 內容 |
|---|---|
| [01_ARCHITECTURE.md](01_ARCHITECTURE.md) | 模組架構與資料流（對齊現有 `src-tauri/src/` 程式碼） |
| [02_MEMORY_SYSTEM.md](02_MEMORY_SYSTEM.md) | 滑動視窗分塊、三階段漏斗 RAG、memory_tags 蒸餾管線 |
| [03_QA_AUTOPILOT.md](03_QA_AUTOPILOT.md) | 全自動 QA：API 回傳驗證閘門與提示詞自修正迴圈 |
| [04_TOKEN_ECONOMY.md](04_TOKEN_ECONOMY.md) | Token 極致節約策略與「邏輯優先於 LLM」演算法 |
| [05_SECURITY_MODEL.md](05_SECURITY_MODEL.md) | 零信任模型：沙盒對齊、確認閘門、金鑰隔離、全域模式 |
| [06_AGENT_TEAM.md](06_AGENT_TEAM.md) | 22 人專家團隊與寫檔前 22 道交叉驗證管線 |
| [07_UI_SPEC.md](07_UI_SPEC.md) | 介面規格：Projects 多資料夾、模式切換、確認面板 |
| [08_ROADMAP.md](08_ROADMAP.md) | 接續開發路線圖（Phase 0–4） |
| [09_QA_REPORT.md](09_QA_REPORT.md) | 真實 QA 報告：API 大中小任務實測 + 自我截圖驗證 |

## 現況（2026-06）

已完成（`src-tauri/`，約 4,000 行 Rust）：

- `orchestrator.rs` — Orchestrator、SubAgent 調度、ConfirmationGate、PendingAction 風險分級、自愈執行 `execute_task_with_healing`、多資料夾 `execute_multi_folder`、全域模式 `global_execute`
- `agent.rs` — AgentLoop（run_step / parse_tool_calls / execute_tool）、AuditResult 一票否決機制
- `sandbox.rs` — 引數向量化分離、程式白名單/黑名單、路徑越界檢查、間接 shell 注入檢查、`run_in_sandbox` Exit Code + Stderr 硬性擷取
- `db.rs` — SQLite 確定性狀態機：tasks / execution_logs / audit_logs / projects / conversations
- `config.rs` — `config.local.toml` 金鑰隔離、`ensure_gitignore` 自動屏蔽、全組態結構體（零 Magic Number）
- `locale.rs` — Windows `chcp 65001` / Unix `LANG=zh_TW.UTF-8` 語系校準
- `mcp.rs` — MCP 伺服器管理
- `ui/` — egui 前端原型（17 代理人側欄、Projects、工作區）
- `.agent/rules/` — core.agents.toon（17 代理人）、security_policies.toon、confirmation_gate.toon
- `tests_integration.rs` — 636 行整合測試

## 差距分析（本輪規劃要補的洞）

| # | 缺口 | 對應文件 | 優先級 |
|---|---|---|---|
| 1 | 第一組「記憶蒸餾與防幻覺組」5 名代理人未定義、未實作（17/22） | 02、06 | P0 |
| 2 | 無限上下文：滑動視窗分塊 + 三階段漏斗 RAG 尚未實作（無 `memory_tags/` 模組） | 02 | P0 |
| 3 | 全自動 QA：API 回傳指令的驗證閘門已有確定性部分（sandbox.rs），但缺「提示詞自修正」與「回歸測試語料庫」 | 03 | P0 |
| 4 | 寫檔前 22 道交叉驗證管線未形式化（目前僅 run_audits 局部審查） | 06 | P1 |
| 5 | Token 經濟：尚無 token 預算器、非對稱模型路由、提示快取對齊 | 04 | P1 |
| 6 | 多子代理人「並行」執行（目前為順序調度） | 01、08 | P1 |
| 7 | UI 距離 Codex / Antigravity / Claude 桌面版規格仍有差距（分支選擇、Artifact 審查策略、安全預設下拉） | 07 | P2 |
| 8 | UniFFI 行動端綁定未動工 | 08 | P3 |

## 鋼鐵戒律（全專案不可違反）

1. 說明文件一律進 `Docs/`；代理規則一律 `.toon` 格式進 `.agent/rules/`
2. 金鑰只存在 `config.local.toml`（已在 .gitignore），任何 `.rs` 出現 `sk-` 開頭字串 = 一票否決
3. 不信任模型口頭報告：Exit Code == 0 且 stderr 為空才算成功
4. 進度狀態只信 SQLite（`agnes_state.db`），不信對話上下文
5. Shell 執行前必先語系校準（`locale.rs`）
6. 零 Magic Number：所有數值進 `Config` 結構體（`config.rs`）
7. 修正階段只輸出 Git Diff 增量，未變更代碼不重複輸出
8. 寫檔前必過 22 道交叉驗證（見 06 與 `.agent/rules/cross_validation.toon`）
