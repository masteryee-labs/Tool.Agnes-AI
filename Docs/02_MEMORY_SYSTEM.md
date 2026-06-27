# 02 — 無限上下文分層記憶系統

> 目標：突破單次 1M Token 物理限制，0 遺忘、0 幻覺殘留。
> 模組：`src-tauri/src/memory.rs`（已實作：滑動視窗 + 漏斗 RAG + FTS5）。規則檔：`.agent/rules/memory.toon`。
> Phase 5 擴充：`.agent/memory/` 三檔跨 Session 記憶（loop_state/lessons/pitfalls）。

## 儲存佈局

```
memory_tags/
├── rust_engine/            ← 標籤資料夾（領域）
│   ├── 3f2a…e1.md          ← 單一記憶檔，UUID 命名
│   └── 8c01…77.md
├── qa_pipeline/
└── ui_spec/
```

硬性限制（全部進 `Config.memory`，零 Magic Number）：

| 參數 | 預設值 | 說明 |
|---|---|---|
| `md_token_cap` | 2000 | 單一 .md 上限，超量自動分裂為新 UUID 檔 |
| `context_high_watermark` | 800_000 | 觸發分塊蒸餾的上下文水位 |
| `chunk_size` | 100_000 | 單塊大小 |
| `overlap_lines` | 50 | 相鄰塊上下各保留的重疊行數 |
| `distill_trigger_delta` | 50_000 | 對話 Token 增量觸發蒸餾歸檔的閾值 |

## 機制一：重疊滑動視窗分塊（Overlapping Sliding Window Chunking）

超過 `context_high_watermark` 時**絕對禁止直接截斷**：

1. 以行為單位切分為 N 個 chunk（每塊 ≤ `chunk_size` token）
2. 相鄰 chunk 強制重疊：chunk[i] 尾部 `overlap_lines` 行 = chunk[i+1] 頭部 `overlap_lines` 行
3. Token 計數用本地估算器（`estimate_tokens()`：CJK 字元 ≈ 1 token/字、ASCII ≈ 4 字元/token），**不呼叫 API 計數**——這是 0 token 成本的純 Rust 邏輯

```rust
pub struct Chunk { pub index: usize, pub text: String,
                   pub overlap_head: String, pub overlap_tail: String }
pub fn sliding_window_chunk(text: &str, cfg: &MemoryConfig) -> Vec<Chunk>
```

## 機制二：三階段漏斗式動態記憶檢索（3-Stage Funnel RAG）

每階段只送「最小必要文本」，這是 Token 經濟的核心（見 04）：

| 階段 | 送入模型的內容 | 回傳 | 模型檔位 |
|---|---|---|---|
| 1 找領域 | 使用者 Prompt + **標籤資料夾名稱清單**（僅名稱） | 關聯標籤清單 | 最小檔（flash 級） |
| 2 找檔案 | Prompt + 關聯標籤內**所有 .md 檔名**（僅檔名，檔名須含語意摘要前綴） | 精準檔案清單 | 最小檔 |
| 3 注內容 | Prompt + 篩選後 .md **內容** | 最終任務 Context | 任務主模型 |

優化（邏輯優先，見 04）：

- **階段 0（0 token 預過濾）**：先用 Rust 端關鍵詞/標籤倒排索引（SQLite FTS5 表 `memory_index`）粗篩。若命中分數高於 `Config.memory.local_hit_threshold`，**跳過階段 1 的 LLM 呼叫**，直接進階段 2。
- 檔名規範：`[UUID前8碼]_[kebab-case語意摘要].md`，讓階段 2 僅憑檔名即可判斷，不必開檔。
- 三階段全部記入 `audit_logs`，可重放驗證檢索精準度。

## 機制三：蒸餾管線（第一組 5 名代理人，現缺，P0）

任務完成且 Token 增量 ≥ `distill_trigger_delta` 時觸發：

```
原始上下文
  ├─[並行]→ ContextDistillerAlpha（前半段 chunks 高密度壓縮）
  └─[並行]→ ContextDistillerBeta （後半段 chunks 並行壓縮）
            ↓
       DistillationIntegrator（總和重組、靠 overlap 區消弭斷層 → 極簡 .md）
            ↓
       FactHallucinationAuditor（第一道審查：與原文交叉比對，
         無中生有/關鍵參數遺失 → [REJECT] 退回重啟蒸餾）
            ↓
       TokenOverlapAuditor（第二道審查：0 token 純 Rust 邏輯——
         estimate_tokens(.md) ≤ md_token_cap？標籤資料夾分類正確？
         overlap 區是否被完整消化？未達標一票否決）
            ↓
       寫入 memory_tags/[標籤]/[UUID]_[摘要].md + 更新 SQLite memory_index
```

成本設計：5 道流程中只有 Alpha/Beta/Integrator/FactAuditor 需 LLM（且 Alpha+Beta 並行、用低檔模型）；TokenOverlapAuditor 為純 Rust 確定性檢查，0 token。

## 機制四：防遺忘——SQLite 優先於上下文

蒸餾解決「長期知識」，SQLite 解決「進度狀態」，兩者分工不可混用：

- 任務樹、狀態（PENDING/SUCCESS/FAILED）、檔案路徑 → 只進 `agnes_state.db`（已實作 `db.rs`）
- 每輪新對話：Rust 後端讀 SQLite → 組成不可篡改 System 區塊強行注入 API Request
- 蒸餾 .md 內**禁止**記錄任務狀態（會過期），只記錄結論、決策、參數

## 機制五：跨 Session 記憶（Phase 5，代理會忘倉庫不會忘）

> 對齊 Loop Engineering Memory 組件 + AGENTS.md 跨記憶機制。
> 解決跨 AI 工具 / 跨 Session 重複踩雷的 Token 浪費問題。

### 三層記憶架構

| 層 | 檔案 | 用途 | 硬性上限 | 觸發 |
|---|---|---|---|---|
| 短期進度 | `.agent/memory/loop_state.md` | 當前任務進度 | ≤50 行/≤2KB | 每子任務 ≤3 行；達 40 行中段蒸餾；任務完成清空 |
| 長期教訓 | `.agent/memory/lessons.md` | 蒸餾後的教訓 | ≤30 條/每條 ≤2 行 | 任務完成時晉升；FIFO 刪最舊 |
| 雷庫 | `.agent/memory/pitfalls.md` | 重複踩過的雷 | ≤40 條/每領域 ≤5 | 發現同類錯誤重複時；去重更新 |

### 蒸餾協議（防止 loop_state 膨脹 + 跨 Session 防踩雷）

```
每完成一個子任務
  │
  ▼
loop_state.md 追加 ≤3 行（做了什麼/改了哪些檔/下一步）
  │
  ├─ 達 40 行 → 中段蒸餾：已完成子任務壓成 1 行摘要，只留未完成詳細
  │
  └─ 任務全部完成
       │
       ├─ 全檔蒸餾為 1 條 lesson → 寫入 lessons.md（FIFO 30 條）
       ├─ 清空 loop_state.md
       └─ 發現同類錯誤跨 Session/跨工具重複 → 追加 pitfalls.md（先掃描去重）
```

### Discover 階段強制讀取（防跨 Session 重複踩雷）

`loop_engine.rs` 在 Discover 階段必讀：
1. `loop_state.md` — 接續上次中斷的進度
2. `lessons.md` — 避免重複已學過的教訓
3. `pitfalls.md` — 避免重複踩同類的雷

### 與既有記憶系統的分工

| 記憶層 | 儲存位置 | 生命週期 | 用途 |
|---|---|---|---|
| SQLite 狀態 | `agnes_state.db` | 單任務 | 任務樹/狀態/路徑（確定性真相） |
| 跨 Session 記憶 | `.agent/memory/*.md` | 跨 Session | 進度/教訓/雷庫（AI 可讀可寫） |
| 長期知識 | `memory_tags/[標籤]/` | 永久 | 蒸餾結論/決策/參數（RAG 檢索） |

- 三層不重疊：SQLite 記狀態、`.agent/memory/` 記教訓、`memory_tags/` 記知識
- `.agent/memory/` 是 AI 跨 Session 的「短期+中期記憶」，`memory_tags/` 是「長期記憶」

## 驗收測試（進 tests_integration.rs）

1. 1.2M token 合成文本 → chunk 數正確、所有相鄰塊 overlap 完全一致
2. `.md` 超過 `md_token_cap` → 自動分裂，無檔案超標
3. 漏斗檢索：植入已知記憶 → 三階段後命中率 100%、注入內容 token ≤ 預算
4. 蒸餾後關鍵參數保留率：原文植入 20 個關鍵值 → FactAuditor 比對 20/20
