# 04 — Token 極致節約與「邏輯優先於 LLM」演算法

> 原則：能用 Rust 確定性邏輯解決的，一個 token 都不花。LLM 只負責「語意判斷」這一件機器邏輯做不到的事。
> 規則檔：`.agent/rules/harness.toon`。

## 誠實前提

「以最小 token 超越任何旗艦模型」的正確解讀不是讓小模型變聰明，而是**把智慧花在刀口上**：
旗艦模型 80% 的 token 消耗在重讀上下文、重述未變更代碼、口頭確認。Agnes 用系統工程把這 80% 砍掉，
讓同樣的模型呼叫次數產出數倍的有效工作量。比較基準是「每美元完成的已驗證任務數」，不是單輪聰明度。

## 七大策略（依節約幅度排序）

### 1. 邏輯優先（Logic-before-LLM）— 最大宗

任何判斷先問：「這能寫成 Rust 函式嗎？」能 → 0 token。

| 工作 | 旗艦模型做法 | Agnes 做法 | 成本 |
|---|---|---|---|
| 指令安全檢查 | 叫模型審 | sandbox.rs 規則引擎 D1–D8 | 0 |
| 成功判定 | 信模型口頭報告 | Exit Code + Stderr | 0 |
| Token 計數 | 叫 API 數 | 本地 estimate_tokens() | 0 |
| 記憶檔超標分裂 | 叫模型摘要切 | 行級切分 + 計數器 | 0 |
| 進度回憶 | 重讀對話 | SQLite SELECT | 0 |
| 22 道交叉驗證 | 22 次 LLM 呼叫 | 16 道 Rust + 6 道低檔 LLM（見 06） | ~27% |
| 記憶檢索階段 0 | 直接問模型 | SQLite FTS5 倒排索引預過濾 | 0 |

### 2. 三階段漏斗 RAG（見 02）

上下文注入量從「全部歷史」降為「標籤名 → 檔名 → 精選內容」，注入上限 = `Config.memory.context_budget`。
階段 0 本地 FTS5 命中時連階段 1 的 LLM 呼叫都省掉。

### 3. 非對稱模型路由（Asymmetric Routing）

| 任務類型 | 檔位 |
|---|---|
| 標籤/檔名分類、語意審查、蒸餾壓縮 | 最低檔（flash 級） |
| 代碼生成、複雜推理 | 主力檔 |
| 自修正升級（同錯誤 3 次後） | 高檔，僅一次 |

路由表進 `Config.api.model_routing`，由 WorkflowTopology 規則強制：「Token 消耗超過閾值必須路由至精簡模型」。

### 4. Delta-only Patching

修正階段只輸出 Git Diff 增量。實作面：system prompt 強制 unified diff 格式 + Rust 端 `apply_patch()` 驗證可套用，套不上即 `E_SCHEMA` REJECT。未變更代碼重複輸出 = SlopPathPurgeSpecialist 違規。

### 5. 提示前綴快取對齊（Prompt Cache Alignment）

API request 結構固定為「靜態前綴 + 動態尾部」：

```
[system: 角色+規則（每 session 不變）]      ← 可被供應商快取
[system: SQLite 狀態注入（變動慢）]
[user: 漏斗檢索結果 + 本輪 prompt（變動快）] ← 永遠放最後
```

規則：靜態段內容**位元組級穩定**（不嵌時間戳、不嵌隨機序），讓 provider-side prompt caching 命中率最大化。

### 6. 休眠代理（Lazy Activation）

22 代理人中未被 Orchestrator 路由者不產生任何 API 呼叫、不出現在 prompt 中。代理「存在」只是 `.toon` 規則檔的一行宣告，激活才有成本。標稱降低 70% 消耗的來源即此。

### 7. 失敗回饋最小化

REJECT 回饋只含：失敗碼 + 出錯行 + 修補指令（見 03）。禁止重送原始回傳全文。Stderr 截斷至 `Config.qa.stderr_max_lines`。

## Token 預算器（TokenBudgeter，待實作 P1）

```rust
pub struct TokenBudgeter {
    session_budget: u64,      // Config.api.session_budget
    spent_prompt: u64, spent_completion: u64,
}
```

- 每次 API 呼叫前：`estimate_tokens(request)` 超出單輪上限 → 強制先走蒸餾/漏斗縮減，不送出
- 從 API response usage 欄位記帳，寫入 SQLite `token_ledger` 表 → UI 顯示每任務真實成本
- 水位 80% 警告、100% 鎖定（僅確定性操作可繼續）
- 每週可從 `token_ledger` 產出「每 token 完成任務數」趨勢，驗證優化是否真的生效——拒絕氛圍感、只看數字

## 反模式（審查時直接 REJECT）

1. 為「確認一下」呼叫 API（確認用 Exit Code 與 SQLite）
2. 把整個檔案塞進 prompt 只為改一行（用 Delta + 行號範圍）
3. 讓模型複述任務狀態（狀態注入是後端的事）
4. 語意審查用主力檔模型（一律 flash 級）
5. 重試時重送相同 prompt 期待不同結果（必須先過 repair_table 修補）

## Phase 5 Token 經濟：自主迴圈 + 子代理

> 自主迴圈引入新的 token 消耗來源，需額外的節約策略。

### 子代理 token 預算分配

| 子代理角色 | 模型檔位 | 預算佔比 | 節約機制 |
|---|---|---|---|
| Planner | flash 級 | ≤10% | 拆解結果寫 SQLite，不重複呼叫 |
| Generator | 主力檔 | ≤60% | Delta-only + worktree 隔離避免重做 |
| Evaluator | flash 級 | ≤20% | 獨立驗證，REJECT 訊息結構化最小化 |
| 22 gate（Sensor） | 16 道 0 token + flash | ≤10% | 休眠路由，未激活零成本 |

### 自主迴圈 token 控制

- **每輪預算上限**：`AutonomousLoop` 每輪從 `TokenBudgeter` 領取預算，耗盡即停止該輪
- **迭代上限**：最多 3 輪同失敗碼 → 升級 premium 一次 → 再失敗 FAILED（禁止無限循環浪費）
- **跨 Session 記憶省 token**：Discover 階段讀 `lessons.md` + `pitfalls.md`，避免重複踩雷的重試成本
- **子代理間不重送上下文**：子代理間只透過 SQLite + mpsc 溝通，禁止把整個對話歷史在子代理間傳遞

### 跨 Session 記憶的 token 投資回報

| 機制 | 一次性成本 | 長期節約 |
|---|---|---|
| `loop_state.md` 蒸餾 | 每子任務 ≤3 行寫入 | 跨 Session 接續，省去重新探索 |
| `lessons.md` 晉升 | 任務完成 1 條 | 避免重複已學教訓的重試 |
| `pitfalls.md` 去重 | 發現時 1 條 | 避免跨工具重複踩雷的 N 輪重試 |

> 投資回報率：每條 lesson/pitfall 的寫入成本（≤2 行）< 避免一次重試的成本（1 輪 API 呼叫）。
