# 08 — 接續開發路線圖

> 每個 Phase 結束條件 = 驗收測試全綠（cargo test，含 qa_replay 語料重放）+ 22 道交叉驗證管線通過。
> 進度狀態以 SQLite tasks 表為準，本文件僅描述計畫，不記錄進度。

## Phase 0 — 地基修整（0.5 週）

- [ ] 刪除 `nul` 殘留檔（根目錄與 src-tauri/，用 `\\.\nul` 語法）、`run_error.log`、`main.rs.bak`
- [ ] 移除 Tauri 殘留：`tauri.conf.json`、`capabilities/`、`gen/`（確認無引用後）
- [ ] 首次 git commit（目前倉庫零提交；確認 .gitignore 先行生效，`agnes_state.db` 不入庫）
- [ ] `cargo clippy -D warnings` 清零，作為 G9 的基線

## Phase 1 — 記憶系統 + QA 自動化（核心，2–3 週）

- [ ] `memory.rs`：estimate_tokens、sliding_window_chunk、memory_tags 讀寫、md_token_cap 分裂（02）
- [ ] SQLite 新表：`memory_index`（FTS5）、`token_ledger`
- [ ] 三階段漏斗 RAG + 階段 0 本地預過濾（02）
- [ ] 第一組 5 代理人接入 `dispatch_subagents`（規則已在 memory.toon）
- [ ] `validation.rs`：22-gate `trait Gate` 執行器，接 `AgentEngine::run_validation`（06）
- [ ] QA 自修正迴圈：失敗碼 → repair_table → Delta 回饋（03）
- [ ] 回歸語料庫 `tests/fixtures/qa_corpus/` + `cargo test qa_replay`（03）
- 驗收：02 與 03 文末的驗收測試全部進 tests_integration.rs 並通過

## Phase 2 — 並行調度 + Token 經濟（2 週）

- [x] `dispatch_subagents` 改 DAG 分層 + tokio JoinSet 同層並行原語（`parallel.rs`）；`execute_multi_folder_parallel` 多資料夾並行（驗證 gate 並行沿用 `thread::scope`）（01）
- [ ] TokenBudgeter + 非對稱模型路由表 + 提示前綴快取對齊（04）
- [ ] Delta-only：unified diff 強制 + `apply_patch()` 驗證
- [ ] UI：Token 計量表、代理人狀態樹、ConfirmationGate 面板補完（07）
- [ ] CJK 字型內嵌驗證
- 驗收：同一基準任務集，token 消耗相對 Phase 1 下降 ≥ 50%（token_ledger 數字為準）

## Phase 3 — 沙盒強化 + 全域模式完備（2 週）

- [x] WASM 沙盒執行不可信代碼片段（改用 `wasmi` 直譯器：空 host import + fuel 計量；較 wasmtime 更輕、無 JIT 攻擊面）
- [x] Docker 沙盒（`--network=none` 預設）跑編譯級任務（偵測缺 docker 自動降級）
- [ ] 全域模式：Critical 二次確認、AllowedPaths 管理 UI、審計回放視圖（05）
- [x] 安全紅隊測試：`tests/red_team.rs` 對 D2–D8 + WASM 隔離投射路徑逃逸/注入/禁止程式/間接 shell/金鑰/破壞性命令
- 驗收：✅ 達成——`cargo test --test red_team` 17/17 通過，0 穿透；惡意命令於沙盒入口即被攔（Exit Code 對齊判否）

## Phase 4 — 行動端與多模態（後置）

- [x] UniFFI 綁定（`agnes.udl` + proc-macro，`--features mobile`）；iOS/Android 殼層待接已產生的綁定
- [x] MultimodalMediaSpecialist 接 Agnes Image 2.1 Flash / Agnes-Video-V2.0（`multimodal.rs`，動態激活、共用限流器）
- [x] 行動端沙盒降級策略（無 Docker → `run_in_docker_sandbox` 自動回退；WASM/行程隔離可選）

## Phase 5 — 自主代理迴圈 + 真子代理（核心升級，3–4 週）

> 目標：從「單次反應式工具」升級為「自主迴圈代理」——能自己尋找工作、派發子代理、驗證結果、跨 Session 記憶。
> 對齊 Loop Engineering 六組件 + Harness Engineering Guides×Sensors 框架。

### 5A — 自主迴圈引擎（Automations，迴圈心跳）

- [x] `loop_engine.rs`：目標驅動自主迴圈
  - `AutonomousLoop` struct：持有一個 `Goal`（目標條件）+ `max_iterations` + `exit_condition`
  - 每輪走 Discover→Plan→Execute→Verify→Iterate 五階段（對齊 AGENTS.md）
  - Discover：讀 SQLite 真實狀態 + `memory.rs` RAG 檢索 + `.agent/memory/lessons.md` + `pitfalls.md`
  - 退出條件：目標達成 OR 達迭代上限 OR 3 輪同失敗碼升級 premium 再失敗 → FAILED 停止
  - 與現有 `AgentLoop.run_step` 的關係：`AutonomousLoop` 是外層迴圈，每輪呼叫 `AgentLoop.run_step` 作為 Execute 階段
- [x] `db.rs` 新表：`goals`（id, description, exit_condition, status, created_at, completed_at）
- [x] UI：目標輸入框 + 迴圈狀態視覺化（當前階段/迭代數/剩餘預算）
- 驗收：給定目標「tests/e2e_tests.rs 全綠」→ 自主迴圈跑到通過或 3 輪失敗停止

### 5B — 真子代理架構（Sub-agents，evaluator-optimizer 模式）

- [x] `sub_agent.rs`：獨立子代理（不是驗證 gate）
  - `SubAgentInstance` struct：持有獨立 `AgentLoop` + 角色特定 system prompt + 獨立 conversation_id
  - 三角色模式（對齊 Anthropic Planner/Generator/Evaluator）：
    - **Planner**：分解目標為原子子任務列表，寫入 SQLite `tasks` 表
    - **Generator**：一次實作一個子任務，呼叫 `AgentLoop.run_step` 執行
    - **Evaluator**：獨立驗證 Generator 產出（不同 system prompt，防自我寬容）
  - Evaluator REJECT → 回饋 Generator 修正（Delta-only），最多 3 輪
  - 子代理間只透過 SQLite + `tokio::sync::mpsc` 溝通，禁止共享可變記憶體
- [x] `orchestrator.rs` 擴充：`dispatch_real_subagents`（與現有 `dispatch_subagents` 驗證 gate 並存）
  - 現有 `dispatch_subagents`（22 道驗證）保留為 Verify 階段的 Sensor
  - 新增 `dispatch_real_subagents` 為 Execute 階段的子代理派發
- [x] `db.rs` 新表：`sub_agent_runs`（id, goal_id, role, conversation_id, status, result_summary）
- 驗收：給定目標 → Planner 拆解 → Generator 實作 → Evaluator 驗證 → 通過即交付

### 5C — Git Worktree 隔離（多子代理平行不打架）

- [x] `worktree.rs`：git worktree 管理
  - `WorktreeManager::create(agent_id) -> PathBuf`：為子代理建立隔離工作目錄 + 分支
  - `WorktreeManager::merge(agent_id) -> Result`：子代理完成後 merge 回主分支
  - `WorktreeManager::cleanup(agent_id)`：清理工作目錄
  - 共用同一份專案歷史，子代理動不到彼此檔案
- [x] `sub_agent.rs` 整合：每個 Generator 子代理在獨立 worktree 中工作
- 驗收：兩個 Generator 子代理平行修改同一檔案 → 不衝突 → 各自 merge

### 5D — 跨 Session 記憶整合（Memory，代理會忘倉庫不會忘）

- [x] `memory.rs` 擴充：讀寫 `.agent/memory/` 三檔
  - `read_loop_state() -> String`：讀 `loop_state.md`（當前任務進度）
  - `append_loop_state(lines: &str)`：追加 ≤3 行（每子任務蒸餾）
  - `distill_loop_state()`：達 40 行中段蒸餾；任務完成清空 + 晉升 lessons
  - `read_lessons() -> Vec<String>`：讀 `lessons.md`（跨 Session 教訓）
  - `read_pitfalls() -> Vec<String>`：讀 `pitfalls.md`（跨 Session 雷庫）
  - `append_lesson(line: &str)`：FIFO 30 條上限
  - `append_pitfall(domain: &str, line: &str)`：去重 + 每領域 ≤5 條
- [x] `loop_engine.rs` 整合：Discover 階段必讀 lessons + pitfalls；每子任務完成蒸餾 loop_state
- 驗收：Session A 中斷 → Session B 開頭讀 loop_state 接續；同類錯誤跨 Session 不重複

### 5E — 極簡黑+白暗色模式 + 無視窗執行（UI/UX 現代化）

- [x] 配色全面改版：Claude 橘色 → 極簡黑+白暗色模式（對標 Claude Code / Codex / Devin / Antigravity 2.0）
  - `ui_theme.rs`：`ACCENT_ORANGE` 由品牌橘 `(217,119,87)` → 純白 `(235,235,235)`
  - 新增 `TEXT_ON_ACCENT = (18,18,18)`：白色按鈕上的深色文字
  - 背景純黑→深灰漸層、文字白→灰階、邊框低對比灰線
- [x] `no_window.rs`：Windows 無視窗執行 helper（`CREATE_NO_WINDOW` flag）
  - `silent_command()` + `NoWindowExt` trait + `NoWindowExtTokio` trait
  - 套用至所有子進程：sandbox、agent、loop_engine、worktree、validation、mcp、qa_runner
  - 解決 CMD/PowerShell 視窗不斷彈出干擾使用者桌面的問題
- [x] Generator prompt 強化：明確要求使用 `<write_file>` 工具標籤
- [x] Evaluator 強化：附上工具執行清單，禁止只看文字口頭報告
- [x] exit_condition 支援「檔案存在」語意（`file:path exists`）
- [x] API 連接錯誤重試（5 次指數退避）
- [x] 迴圈狀態即時更新 UI（500ms poll）
- 驗收：實機 QA — 目標「建立 Docs/test.txt」→ 1 輪 SUCCESS、檔案實際建立、零彈窗

## 持續性紀律（每個 Phase 通用）

1. 寫檔前 22 道交叉驗證（verification.toon），無例外
2. 每次 API 回傳過 QA 閘門；新 REJECT 樣本即刻進語料庫
3. 文件改動同步：行為變更 → 對應 Docs/*.md 同 commit 更新
4. 每 Phase 結束跑一次 token_ledger 趨勢檢視，「每 token 完成任務數」必須單調上升
