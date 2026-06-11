# 09 — 真實 QA 報告（2026-06-11）

> 原則：不信模型口頭回報。所有驗收以硬性證據判定——檔案系統實際內容、`cargo test` 真實 Exit Code、應用程式自我截圖。

## 一、真實 API 任務測試（qa_runner）

工具：`cargo run --release --bin qa_runner [small|medium|large|all]`
（[qa_runner.rs](../src-tauri/src/bin/qa_runner.rs)：真實呼叫 Agnes API，工具經 22-gate 驗證後執行，多步迴圈回饋工具結果直到任務完成）

| 級別 | 任務 | 驗收方式（硬性） | 結果 | 步數 | Tokens |
|---|---|---|---|---|---|
| small | 建立 hello.txt 含「你好 Agnes」 | 檔案存在 + 內容關鍵詞 | **PASS** | 2 | 1,412 |
| medium | 既有 crate 實作迭代版 fib + 單元測試 | `cargo test` Exit Code == 0 | **PASS** | 2 | 2,055 |
| large | 三檔函式庫（math/strings/lib，含 CJK 計數測試） | 3/3 檔案存在 + `cargo test` Exit Code == 0 | **PASS** | 2 | 5,556 |

**QA 抓到並修復的真實缺陷**：首輪 small 任務 6 步全敗（`hello.txt` 不存在）。根因：Windows temp 目錄以 **8.3 短檔名**（`MASTER~1`）傳入工作區，與 `canonicalize` 後的長路徑前綴比對不相等，路徑圈禁誤判「越權」、阻擋一切寫入。修復：`agent.rs::normalize_workspace`（AgentLoop 建構時統一展開長路徑），GUI 與 qa_runner 同步受惠。修復後三級任務全過——**這正是 03_QA_AUTOPILOT「先驗證再放行」設計的實證**。

## 二、應用程式自我截圖驗證（不觸碰使用者螢幕）

機制：`AGNES_QA_SHOT=<png> [AGNES_QA_VIEW=settings|history] agnes-ai.exe`
影像來自 egui 自身渲染管線（`ViewportCommand::Screenshot`），只含應用程式視窗內容，零螢幕擷取、零輸入控制。

| 視圖 | 檔案 | 驗證點 | 結果 |
|---|---|---|---|
| 主畫面 | qa_screenshots/main_view.png | CJK 全部正常渲染（亂碼根治）；Codex 風格置中提問+輸入卡；22 代理人面板 | PASS |
| 設定頁 | qa_screenshots/settings_view.png | 全頁式版面：返回鈕、搜尋框、個人/整合分組導航、工作模式雙選卡、卡片式設定列 | PASS |
| 歷史頁 | qa_screenshots/history_view.png | 側欄歷史分頁渲染 | PASS |

## 三、迴歸基線

- `cargo test`：63 個整合測試（含 qa_replay 語料重放、蒸餾審查、沙盒/路徑/注入防護）
- `cargo clippy --all-targets`：0 警告
- 後續每次提交前重跑上述兩項 + 必要時 qa_runner small（最便宜的端到端煙霧測試，約 1.4K tokens）

## 重跑方式

```powershell
# 端到端 API QA（需 config.local.toml 有金鑰）
cargo run --release --bin qa_runner all

# 介面自截圖
$env:AGNES_QA_SHOT="$PWD\qa_screenshots\main_view.png"; .\src-tauri\target\release\agnes-ai.exe
```
