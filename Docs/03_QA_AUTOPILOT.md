# 03 — 全自動 QA：API 回傳驗證閘門與提示詞自修正

> 問題：Agnes API 回傳的指令不保證正確或安全。開發階段必須**先驗證再放行**；驗證失敗時不是盲目重試，而是**修正提示詞**。
> 規則檔：`.agent/rules/qa_validation.toon`。

## 設計總則

1. 不信任模型輸出的任何一個字——只信結構化驗證結果與沙盒 Exit Code
2. 驗證分兩層：**確定性層（0 token，純 Rust）先跑**，全過才考慮語意層
3. 每次 REJECT 都產生兩個產物：(a) 給模型的最小修正回饋 (b) 存入回歸語料庫的測試樣本
4. 重試次數、退避時間全部進 `Config.qa`，禁止 Magic Number

## 驗證閘門（Validation Gate）

API 回傳 → `parse_tool_calls`（agent.rs:615）→ 進入閘門：

### 第一層：確定性驗證（0 token，已大半實作於 sandbox.rs）

| # | 檢查 | 實作 | 失敗碼 |
|---|---|---|---|
| D1 | JSON/工具呼叫結構可解析、欄位齊全 | parse_tool_calls 強 schema | `E_SCHEMA` |
| D2 | 程式在白名單、不在黑名單 | `is_allowed_program` / `is_forbidden_program` | `E_PROGRAM` |
| D3 | 引數消毒（注入字元、長度） | `sanitize_all_args` / `validate_cmd_length` | `E_ARGS` |
| D4 | 路徑圈禁（無 `..` 越界、在 workspace 內） | `has_path_traversal_component` / `is_path_in_workspace` | `E_PATH` |
| D5 | 間接 shell 注入（`sh -c`、`cmd /c` 包裝） | `check_indirect_shell_injection` | `E_SHELL` |
| D6 | 金鑰洩漏掃描（輸出/代碼含 `sk-` 前綴） | regex 掃描 | `E_SECRET` |
| D7 | 破壞性指令模式（rm -rf、format、Remove-Item -Recurse） | confirmation_gate.toon Gate 1 | `E_DESTRUCT` |
| D8 | Rust 代碼產物：`cargo check` 編譯通過 | 沙盒內執行，取 Exit Code | `E_COMPILE` |

### 第二層：語意審查（按風險分級才花 token）

- 風險 Low/Medium 且第一層全過 → **跳過語意層**（省 token）
- 風險 High/Critical → `run_audits`（agent.rs:483）以低檔模型做一票否決審查，輸出強制 `[PASS]` 或 `[REJECT: 行號+邏輯鏈]`
- 寫檔操作 → 另過 22 道交叉驗證管線（見 06；其中 16 道為確定性 0 token）

### 第三層：沙盒硬性對齊（執行後）

`run_in_sandbox` 真實 Exit Code + Stderr。模型聲稱成功但 ExitCode != 0 → 判定**虛假回報**，`is_false_positive()` 攔截，真實 Stderr 砸回 Orchestrator。

## 提示詞自修正迴圈（Prompt Self-Repair Loop）

REJECT 不等於重試同樣的話。失敗碼對應**確定性的提示詞修補規則**（存於 `qa_validation.toon` 的 repair_table）：

| 失敗碼 | 修補動作（注入 system prompt 的增量指令） |
|---|---|
| `E_SCHEMA` | 附上正確 schema 範例 + 「僅輸出該 JSON，無前後綴文字」 |
| `E_PROGRAM` | 附上白名單清單 + 「僅能使用下列程式」 |
| `E_ARGS` / `E_SHELL` | 「引數一律以陣列逐項給出，禁止 shell 字串拼接」 |
| `E_PATH` | 附上允許的 workspace 根路徑 + 「所有路徑必須位於其下且禁止 ..」 |
| `E_SECRET` | 「金鑰一律以 `{{API_KEY}}` 佔位符表示，由後端注入」 |
| `E_DESTRUCT` | 「該操作需改為產生 PendingAction 由使用者確認，不得直接執行」 |
| `E_COMPILE` | 只回傳 rustc 錯誤的前 N 行（`Config.qa.stderr_max_lines`）+ Delta-only 修正指令 |

迴圈規則：

- 回饋採 **Delta-only**：只送失敗碼、出錯行、修補指令，不重送整個上下文
- 同一失敗碼連續 `Config.qa.max_repairs`（預設 3）次 → 升級：換高檔模型重試一次；再失敗 → 任務標 FAILED、寫 audit_log、停止（防死循環，呼應 WorkflowRuntimeEvaluator 的 3 輪重置規則）
- 每次修補成功的「修補指令」自動寫入 memory_tags/qa_pipeline/，下次同類任務在 system prompt **預先注入**，把事後修正變成事前預防——重複錯誤率隨使用遞減

## 回歸測試語料庫（Regression Corpus）— 全自動化的核心

每個被 REJECT 的 API 回傳原文存為 fixture：

```
src-tauri/tests/fixtures/qa_corpus/
└── E_PATH/20260610_a3f2.json   ← {原始回傳, 失敗碼, 期望判定}
```

- `cargo test qa_replay`：把全部語料重放過確定性驗證層，斷言判定不變——**零 API 呼叫、零 token 的全自動回歸測試**，CI 每次跑
- 語料同時是「攻擊樣本庫」：新增驗證規則時必須先讓全語料通過
- 開發階段的「直接幫我先確認回傳的指令是否正確或安全」由此落地：所有歷史壞回傳永遠擋得住，新壞回傳第一次被擋就進語料庫

## 全自動 QA 排程

| 時機 | 動作 | Token 成本 |
|---|---|---|
| 每次 API 回傳 | 確定性驗證層 D1–D8 | 0 |
| High/Critical 風險 | 語意審查 | 低（flash 級） |
| 每次執行後 | Exit Code 對齊 | 0 |
| 每次 `cargo test` | qa_replay 全語料重放 + 既有 636 行整合測試 | 0 |
| 寫檔前 | 22 道交叉驗證（06） | 16 道 0 token + 6 道低檔 |
