# AGENTS.md — Agnes AI 跨工具單一真相來源

> 本檔是 Claude Code / Cursor / Codex / Devin / Agnes 共用的唯一前饋 Guide。
> 規則細節採**條件式載入**：只讀任務領域對應的 `.toon` 與 `Docs/`，禁止一次載入全部。
> 跨工具鏡像：`CLAUDE.md` 是本檔的**同步鏡像**——修改本檔後必須執行 `copy AGENTS.md CLAUDE.md`（Windows）同步，零 token 成本。

## 專案形態

Rust + eframe/egui 原生 GUI 桌面端，零 Chromium/WebView2，預留 UniFFI 行動端。
目標：無限上下文分層記憶、0 虛假回報、0 遺忘、零信任防禦的自主代碼智慧體。
程式碼在 `src-tauri/src/`，狀態在 `agnes_state.db`（SQLite），金鑰在 `config.local.toml`。

## 鋼鐵戒律（全專案不可違反，8 條）

1. 說明文件進 `Docs/`；代理規則 `.toon` 進 `.agent/rules/`；代碼進 `src-tauri/src/`
2. 金鑰只存在 `config.local.toml`（已 .gitignore）；任何 `.rs` 出現 `sk-` 開頭實鑰 = 一票否決
3. 不信模型口頭報告：Exit Code == 0 且 stderr 為空才算成功
4. 進度狀態只信 SQLite（`agnes_state.db`），不信對話上下文
5. Shell 執行前必先語系校準（Windows `chcp 65001` / Unix `LANG=zh_TW.UTF-8`）
6. 零 Magic Number：新數值進 `config.rs` 的 Config 結構體
7. 修正階段 Delta-only（Edit 精準替換 / unified diff），禁止重寫整檔、禁止重貼未變更代碼
8. 寫檔前過驗證管線（見 `.agent/rules/verification.toon`），帶病寫入禁止

## 條件式載入路由表（省 Token 核心）

**永遠載入**：本檔 + `Docs/00_OVERVIEW.md`（專案全貌索引）。
**依任務領域追加**（只讀該列對應檔，其餘休眠零成本）：

| 任務領域關鍵字 | 載入規則 `.toon` | 追加讀 Docs |
|---|---|---|
| 記憶 / RAG / 蒸餾 / 上下文 | `memory.toon` | `02_MEMORY_SYSTEM.md` |
| 安全 / 沙盒 / 金鑰 / 執行 / 確認閘 | `security.toon` | `05_SECURITY_MODEL.md` |
| 驗證 / 測試 / QA / 寫檔前 | `verification.toon` | `03_QA_AUTOPILOT.md` |
| Token / 預算 / 模型路由 / 效能 | `harness.toon` | `04_TOKEN_ECONOMY.md` |
| 改多代理編排系統本身 | `agents.toon` | `06_AGENT_TEAM.md` |
| UI / 介面 | （無額外規則） | `07_UI_SPEC.md` |
| 架構 / 模組 / 資料流 | `harness.toon` | `01_ARCHITECTURE.md` |
| 路線圖 / 規劃 | （無額外規則） | `08_ROADMAP.md` |

> 載入紀律：讀完 AGENTS.md 後，先判斷任務領域，只 `read` 路由表指定的 1–2 份規則與 Docs。
> 禁止「預防性」掃描載入全部規則——那是 Token 浪費與 Session 裁切的元兇。

## Loop Engineering 迴圈（每次任務走 5 階段，禁止跳階）

```
Discover → Plan → Execute → Verify → Iterate
```

1. **Discover**（強制）— 必讀三件：① SQLite 真實狀態（`agnes_state.db`）確認當前 Phase；② 路由表指定 Docs（**禁止跳過，跳過會導致產出不符企劃**）；③ 掃 `.agent/memory/lessons.md` + `pitfalls.md` 對照已知雷。三件缺一即視為 Discover 未完成，禁止進入 Plan。
2. **Plan**：拆原子子任務，標目標檔與風險級（Low/Med/High/Critical）；複雜任務先寫 `.agent/memory/loop_state.md`
3. **Execute**：Delta-only 實作，遵守鋼鐵戒律；**每完成一個子任務即蒸餾**（見下方跨記憶協議）
4. **Verify**：跑 `verification.toon` 分層感測器（計算型優先，推理性其次）
5. **Iterate**：未過 → Delta-only 修，**最多 3 輪**；同失敗碼第 3 輪升級 premium 模型重試一次；再不過標 FAILED 停止（禁止無限循環）；**通過後蒸餾**教訓寫入 `lessons.md`

> 通過驗證即交付，未通過再跑一次。系統是回饋迴圈，不是你。

## Harness：Guides × Sensors（前饋與回饋分離）

- **Guides（前饋，行動前）**：本檔、`.toon` 規則、`Docs/`
- **Sensors（回饋，行動後）**：`cargo check` / `clippy -D warnings` / `cargo test`（計算型，0 token）；防幻覺自查（推理性，1 次 flash）
- **四象限覆蓋**：計算型 Guide（型別/LSP）+ 推理型 Guide（本檔/Skills）+ 計算型 Sensor（cargo）+ 推理型 Sensor（防幻覺審查）
- **Sensors 平時休眠**，只在 Verify 階段啟動；未路由的代理零成本不思考

## 跨記憶機制（代理會忘，倉庫不會忘）

三層記憶，硬性上限防止膨脹：

| 層 | 檔案 | 用途 | 上限 | 觸發 |
|---|---|---|---|---|
| 短期 | `.agent/memory/loop_state.md` | 當前任務進度 | ≤50 行/≤2KB | 每子任務 ≤3 行；達 40 行中段蒸餾；任務完成清空 |
| 長期教訓 | `.agent/memory/lessons.md` | 蒸餾後的教訓 | ≤30 條/每條 ≤2 行 | 任務完成時；FIFO 刪最舊 |
| 雷庫 | `.agent/memory/pitfalls.md` | 重複踩過的雷 | ≤40 條/每領域 ≤5 | 發現同類錯誤重複時；去重更新 |

**蒸餾協議（防止 loop_state 膨脹 + 跨 Session 防踩雷）**：
1. 每完成一個子任務 → `loop_state.md` 追加 ≤3 行（做了什麼/改了哪些檔/下一步）
2. `loop_state.md` 達 40 行 → 中段蒸餾：已完成子任務壓成 1 行摘要，只留未完成詳細
3. 任務全部完成 → 全檔蒸餾為 1 條 lesson 寫入 `lessons.md`，清空 `loop_state.md`
4. 發現同類錯誤跨 Session/跨工具重複 → 追加 `pitfalls.md`（先掃描去重）
5. 任務開頭必掃 `lessons.md` + `pitfalls.md`（在 Discover 階段）

**SQLite 仍是進度真相來源**：`tasks` 表（PENDING/SUCCESS），每輪從 DB 讀真實狀態。
**長期知識** → `memory_tags/[標籤]/[UUID]_[摘要].md`，單檔 ≤2000 token。
**檢索** → 三階段漏斗 RAG（Stage 0 FTS5 本機 0-token 預過濾命中即跳過 API）。
**文件** → `Docs/` 為系統記錄，AI 必讀路由表指定份，禁止跳過。

## CLAUDE.md 同步協議（跨工具零浪費）

- `AGENTS.md` 是**唯一編輯源**（Antigravity / Gemini / Codex / Devin 直接讀本檔）
- `CLAUDE.md` 是**同步鏡像**（Claude Code 直接讀到完整內容，省去二次搜尋 token）
- **同步觸發**：任何對 `AGENTS.md` 的 `edit`/`write` 操作完成後，下一步必須是 `copy AGENTS.md CLAUDE.md`（Windows）或 `cp AGENTS.md CLAUDE.md`（Unix）
- **禁止手動編輯 CLAUDE.md**（只由 copy 產生，避免知識碎片化）
- **禁止修改 AGENTS.md 後不執行 copy 同步**（會導致 Claude Code 讀到過期內容）

## 安全審查清單（對齊 OWASP Top 10，寫完 code 第一道防線）

寫檔前 `verification.toon` 的安全感測器涵蓋：輸入驗證、SQL 注入、命令注入、路徑穿越、
金鑰硬編碼、特權提升、XSS、CSRF、不安全反序列化、日誌洩漏。發現即 REJECT 附修復指示。
（灵感：UnitOneAI/SecuritySkills OWASP/NIST 對齊 + Pold911/vibe-code-security-audit 20 漏洞掃描）

## 反模式（審查時直接 REJECT）

- 為確認而呼叫 API（確認用 Exit Code 與 SQLite）
- 整檔塞 prompt 只為改一行
- 讓模型複述任務狀態
- 語意審查用主力模型（應用 flash）
- 重試時重送相同 prompt 未過 repair table
- 一次載入全部 `.toon` 規則
- 跳過路由表指定的 Docs（會導致產出不符企劃）
- 修改 AGENTS.md 後不執行 `copy AGENTS.md CLAUDE.md` 同步
- `loop_state.md` 超過 50 行未蒸餾
- 跨 Session 重複踩同類錯誤未寫入 `pitfalls.md`

## 快速檢查清單（交付前）

- [ ] 路由表指定的規則與 Docs 已讀
- [ ] Discover 已掃 `lessons.md` + `pitfalls.md`
- [ ] cargo check + clippy -D warnings + test Exit Code == 0
- [ ] 無 sk- 硬編碼、無 Magic Number、無 TODO/unimplemented!
- [ ] .md 在 Docs/、.toon 在 .agent/rules/
- [ ] Delta-only 回報，無重貼未變更代碼
- [ ] SQLite 狀態已更新或已列明待寫入
- [ ] 子任務完成已蒸餾至 `loop_state.md`（≤3 行）
- [ ] 修改 AGENTS.md 後已執行 `copy AGENTS.md CLAUDE.md`
