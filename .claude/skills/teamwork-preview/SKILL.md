---
name: teamwork-preview
description: Agnes AI 22 代理人多代理編排開發模式（本專案強制預設）。在 Agnes-AI 倉庫進行任何代碼修改、新功能、重構、文件撰寫時使用。強制執行：寫檔前 22 道交叉驗證、API 回傳 QA 閘門與提示詞自修正、Delta-only 增量更新、產物路徑分流（.md→Docs/、.toon→.agent/rules/）、金鑰零硬編碼、Token 極致節約。
---

# /teamwork-preview — Agnes AI 多代理人編排開發模式

本 skill 是 Agnes-AI 專案的強制開發協議。啟動後依序執行以下四個階段，不得跳過。

## 階段 0：載入規則與真實狀態（每次會話開頭）

1. 讀取 `.agent/rules/` 全部 `.toon` 規則：core.agents、memory_distillation、cross_validation、qa_validation、token_economy、security_policies、confirmation_gate
2. 讀取 SQLite 真實狀態，不信對話記憶：
   ```powershell
   # 進度狀態只信資料庫（db.rs schema：tasks/execution_logs/audit_logs/projects）
   sqlite3 agnes_state.db "SELECT id,description,status FROM tasks ORDER BY created_at DESC LIMIT 20"
   ```
   無 sqlite3 CLI 時改讀 `Docs/08_ROADMAP.md` 確認當前 Phase，並以 `git log --oneline -10` 對照
3. 文件索引在 `Docs/00_OVERVIEW.md`；按任務領域只讀相關的 1–2 份（漏斗原則，省上下文）

## 階段 1：任務拆解與代理路由（Orchestrator 角色）

1. 將使用者任務拆為原子子任務，標明每項的目標檔案與風險級（Low/Medium/High/Critical）
2. 按 `core.agents.toon` + `memory_distillation.toon` 的 22 人名冊路由：只激活必要代理，其餘休眠（未激活 = 不思考、不輸出，零成本）
3. 涉及媒體生成才喚醒 MultimodalMediaSpecialist；diff 預估很小則蒸餾組（G19–G21）整組 SKIP

## 階段 2：實作（鋼鐵戒律）

- **產物分流**：說明文件只寫進 `Docs/`；代理規則只寫 `.toon` 進 `.agent/rules/`；代碼進 `src-tauri/src/`
- **金鑰**：任何檔案不得出現 `sk-` 開頭實際金鑰；範例一律 `{{API_KEY}}`；金鑰只活在 `config.local.toml`（已在 .gitignore）
- **零 Magic Number**：新數值一律進 `config.rs` 的對應 Config 結構體
- **Delta-only**：修正既有檔案用 Edit 精準替換，禁止重寫整檔；回報時不重貼未變更代碼
- **完整性**：交付的 Rust 代碼禁止 `TODO` / `unimplemented!()` / 「省略」標記
- **語系**：產生的任何 shell 執行邏輯，Windows 必含 `chcp 65001` 前置、Unix 必含 `LANG=zh_TW.UTF-8`
- **零 AI 腔**：輸出禁 delve/testament/underscore/crucial/furthermore 等贅字；審查結論只有 `[PASS]` 或 `[REJECT: 檔案:行號 | 原因]`，禁止互相恭維

## 階段 3：寫檔前 22 道交叉驗證（cross_validation.toon）

每次準備寫入/修改檔案前，依序跑四個 Stage。任一 veto gate REJECT → 退回階段 2 修正，禁止帶病寫入。

**Stage A — 靜態確定性（用 Grep/Read 工具完成，0 token API）**
- G3 banned_words 掃描提議內容
- G4 路徑分流正確（.md→Docs/、.toon→.agent/rules/）+ 無 Magic Number
- G6 `Cargo.toml` 無 Chromium/WebView 系依賴
- G11 shell 命令為引數向量，無字串拼接；.gitignore 含 config.local.toml 與 .env
- G12 金鑰掃描：`Grep pattern:"sk-[A-Za-z0-9]{20,}"` 全倉庫 → 命中即 REJECT（一票否決）
- G13 無 TODO/unimplemented!/省略標記
- G14 reqwest 無 blocking feature、重試參數在 Config

**Stage B — 編譯期確定性（真實 Exit Code，不信口頭報告）**
```powershell
chcp 65001; cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings   # G9 一票否決
cargo test --manifest-path src-tauri/Cargo.toml                    # G16 一票否決
```
Exit Code != 0 → 視為失敗，取 stderr 前 30 行做 Delta-only 修正，禁止聲稱成功

**Stage C — 語意審查**
- G17 防幻覺自查：逐條比對「我聲稱的變更」與 `git diff` 實際內容，不符即退回
- G18 若產出記憶/文件檔：估算 token（CJK≈1字/token）不得超過 2000/檔

**Stage D — 簽核**
- G22 終端整合審查：變更是否完整覆蓋原始需求、是否引入範圍外修改 → `[PASS]` 才算完成

## 階段 4：QA 自動化與歸檔（qa_validation.toon）

1. **API 回傳驗證**（開發 Agnes API 整合時）：任何模型回傳的指令先過 D1–D8 確定性閘門（sandbox.rs 函式）再放行；REJECT 時按 RepairTable 修補提示詞，同失敗碼最多 3 次，禁止重送相同 prompt
2. **回歸語料**：每個新的壞回傳樣本存入 `src-tauri/tests/fixtures/qa_corpus/[失敗碼]/`，並確認 `cargo test qa_replay` 涵蓋
3. **狀態落地**：完成的任務更新 SQLite tasks 表（或在回報中明列「需寫入 DB 的狀態變更」）；行為變更同步更新對應 `Docs/*.md`
4. **誠實回報**：測試失敗就說失敗並附 stderr；跳過的步驟明說跳過；嚴禁虛假回報

## 快速檢查清單（每次交付前）

- [ ] 22 道 gate 全部 PASS 或合理 SKIP（附 reason）
- [ ] cargo check + clippy + test Exit Code == 0
- [ ] 無 sk- 硬編碼、無 Magic Number、無 TODO
- [ ] .md 在 Docs/、.toon 在 .agent/rules/
- [ ] 回報為 Delta-only，無重貼未變更代碼
