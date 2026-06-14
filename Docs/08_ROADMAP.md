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
- [ ] 第一組 5 代理人接入 `dispatch_subagents`（規則已在 memory_distillation.toon）
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
- [ ] 安全紅隊測試：以 qa_corpus 攻擊樣本 + 新增路徑逃逸/注入用例打 D1–D8
- 驗收：紅隊樣本 0 穿透；虛假回報攔截率 100%（合成測試）

## Phase 4 — 行動端與多模態（後置）

- [x] UniFFI 綁定（`agnes.udl` + proc-macro，`--features mobile`）；iOS/Android 殼層待接已產生的綁定
- [x] MultimodalMediaSpecialist 接 Agnes Image 2.1 Flash / Agnes-Video-V2.0（`multimodal.rs`，動態激活、共用限流器）
- [x] 行動端沙盒降級策略（無 Docker → `run_in_docker_sandbox` 自動回退；WASM/行程隔離可選）

## 持續性紀律（每個 Phase 通用）

1. 寫檔前 22 道交叉驗證（cross_validation.toon），無例外
2. 每次 API 回傳過 QA 閘門；新 REJECT 樣本即刻進語料庫
3. 文件改動同步：行為變更 → 對應 Docs/*.md 同 commit 更新
4. 每 Phase 結束跑一次 token_ledger 趨勢檢視，「每 token 完成任務數」必須單調上升
