# AGENTS.md — Agnes AI 跨工具單一真相來源

> 本檔是 Antigravity / Gemini / Claude Code / Cursor / Codex / Devin 共用的唯一前饋 Guide。
> 規則細節採**條件式載入**：只讀任務對應的 `.toon` 與 `Docs/`，禁止一次載入全部。
> 跨工具鏡像：`CLAUDE.md` 是本檔的同步鏡像——修改本檔後必須立即執行 `copy AGENTS.md CLAUDE.md`（Windows）。

## 企劃核心（5 行，讓你不讀其他 Docs 也能做正確決策）

- **技術棧**：Rust + eframe/egui 原生 GUI，零 Chromium/WebView2，預留 UniFFI 行動端
- **目標**：無限上下文分層記憶、0 虛假回報、0 遺忘、零信任防禦的自主代理
- **程式碼位置**：`src-tauri/src/`（目錄名保留 tauri 慣例但**不使用 Tauri 框架**）；狀態：`agnes_state.db`（SQLite）；金鑰：`config.local.toml`（支援多 Key 組 `keys` + `key_rotation_every` 輪詢，見 `key_rotation.rs`）
- **當前 Phase**：Phase 0–5 已全數完成（22 代理、真子代理、egui UI、無視窗執行、Loop Engine）
- **架構定位**：Harness Engineering（Guides×Sensors）+ Loop Engineering（5 階段迴圈）已實作

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

**永遠載入**：本檔。
**依任務複雜度追加**（只讀該列對應檔，其餘休眠）：

| 任務領域關鍵字 | 載入規則 `.toon` | 追加讀 Docs（僅跨 Session 或複雜任務） |
|---|---|---|
| 記憶 / RAG / 蒸餾 / 上下文 | `memory.toon` | `02_MEMORY_SYSTEM.md` |
| 安全 / 沙盒 / 金鑰 / 執行 / 確認閘 | `security.toon` | `05_SECURITY_MODEL.md` |
| 驗證 / 測試 / QA / 寫檔前 | `verification.toon` | `03_QA_AUTOPILOT.md` |
| Token / 預算 / 模型路由 / 效能 | `harness.toon` | `04_TOKEN_ECONOMY.md` |
| 改多代理編排系統本身 | `agents.toon` | `06_AGENT_TEAM.md` |
| UI / 介面 / 配色 / 終端嵌入 | （無額外規則） | `07_UI_SPEC.md` |
| 架構 / 模組 / 資料流 | `harness.toon` | `01_ARCHITECTURE.md` |
| 路線圖 / 規劃 | （無額外規則） | `08_ROADMAP.md` |

> **載入紀律**：讀完 AGENTS.md 後，先判斷任務領域，只 `read` 路由表指定的 ≤2 份規則。
> **Docs 讀取時機**：簡單任務（≤2 子任務）跳過 Docs；複雜或跨 Session 任務才讀路由指定 Docs。
> **禁止預防性掃描**：一次載入全部規則或全部 Docs = Token 浪費 = Session 裁切 = AI 遺忘。

## Loop Engineering 迴圈（5 階段，禁止跳階）

```
Discover → Plan → Execute → Verify → Iterate
```

1. **Discover** — 掃 `.agent/memory/lessons.md` + `pitfalls.md`（防踩雷）；複雜/跨 Session 任務才讀 SQLite 狀態與路由指定 Docs
2. **Plan** — 拆原子子任務，標目標檔與風險級（Low/Med/High/Critical）；複雜任務先寫 `.agent/memory/loop_state.md`
3. **Execute** — Delta-only 實作，遵守鋼鐵戒律；每完成一個子任務蒸餾 ≤3 行至 `loop_state.md`
4. **Verify** — 跑 `verification.toon` 分層感測器（計算型優先，推理性其次）
5. **Iterate** — 未過 → Delta-only 修，最多 3 輪；同失敗碼第 3 輪升 premium 重試一次；再不過標 FAILED 停止；通過後蒸餾教訓寫入 `lessons.md`

## Harness：Guides × Sensors（前饋與回饋分離）

- **Guides（前饋）**：本檔 + `.toon` 規則 + 路由指定 Docs（按需）
- **Sensors（回饋）**：`cargo check` / `clippy -D warnings` / `cargo test`（計算型，0 token）；防幻覺自查（推理性，flash 級）
- **四象限**：計算型 Guide（型別/LSP）+ 推理型 Guide（本檔）+ 計算型 Sensor（cargo）+ 推理型 Sensor（防幻覺）
- **Sensors 平時休眠**，只在 Verify 階段啟動

## 跨記憶機制（代理會忘，倉庫不會忘）

| 層 | 檔案 | 用途 | 上限 |
|---|---|---|---|
| 短期 | `.agent/memory/loop_state.md` | 當前任務進度 | ≤50 行；達 40 行中段蒸餾 |
| 長期教訓 | `.agent/memory/lessons.md` | 蒸餾後教訓 | ≤30 條，每條 ≤2 行，FIFO |
| 雷庫 | `.agent/memory/pitfalls.md` | 重複踩過的雷 | ≤40 條，每領域 ≤5 |

**任務完成後**：全檔蒸餾 → 1 條 lesson 寫入 `lessons.md` → 清空 `loop_state.md`。
**SQLite** 是進度真相來源，僅複雜/跨 Session 任務才查詢。

## CLAUDE.md 同步協議（跨工具零浪費）

- `AGENTS.md` 是**唯一編輯源**（Antigravity / Gemini / Codex / Devin 直接讀本檔）
- `CLAUDE.md` 是**同步鏡像**（Claude Code 讀本檔）
- **同步觸發**：任何對 `AGENTS.md` 的寫入完成後，**下一步必須立即執行** `copy AGENTS.md CLAUDE.md`
- **禁止手動編輯 CLAUDE.md**（只由 copy 產生）
- **禁止修改 AGENTS.md 後不執行 copy**（會導致 Claude Code 讀到過期內容）

## 安全審查（OWASP Top 10 對齊，寫完 code 第一道防線）

`verification.toon` 的安全感測器涵蓋：輸入驗證、SQL 注入、命令注入、路徑穿越、金鑰硬編碼、特權提升、XSS、CSRF、不安全反序列化、日誌洩漏。發現即 REJECT 附修復指示。

## 反模式（審查時直接 REJECT）

- 為確認而呼叫 API（用 Exit Code 與 SQLite）
- 整檔塞 prompt 只為改一行
- 語意審查用主力模型（應用 flash）
- 重試時重送相同 prompt 未過 repair table
- 一次載入全部 `.toon` 規則或全部 Docs
- 修改 AGENTS.md 後不執行 `copy AGENTS.md CLAUDE.md`
- `loop_state.md` 超過 50 行未蒸餾
- 跨 Session 重複踩同類錯誤未寫入 `pitfalls.md`

## 快速檢查清單（交付前）

- [ ] 路由表規則（≤2 份 .toon）已讀，Docs 按複雜度決定是否讀
- [ ] Discover 已掃 `lessons.md` + `pitfalls.md`
- [ ] cargo check + clippy -D warnings + test Exit Code == 0
- [ ] 無 sk- 硬編碼、無 Magic Number、無 TODO/unimplemented!
- [ ] Delta-only 回報，無重貼未變更代碼
- [ ] 子任務完成已蒸餾至 `loop_state.md`（≤3 行）
- [ ] 修改 AGENTS.md 後已執行 `copy AGENTS.md CLAUDE.md`
