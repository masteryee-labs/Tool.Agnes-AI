# 09 — 真實 QA 報告（2026-06-11）

## 第二輪：穩定化掃蕩（同日）

代碼審視找出 5 個真實缺陷，全部修復並以證據驗證：

| # | 缺陷 | 等級 | 修復 |
|---|---|---|---|
| 1 | 對話標題用位元組切片 `&prompt[..20]`——中文輸入超過 ~6 字必 panic | 崩潰 | `truncate_chars` 字元級截斷 + 2 個單元測試 |
| 2 | 工具執行結果不入庫、不顯示——使用者看不到 AI 實際做了什麼 | 功能 | execution_results 持久化為 `tool` 訊息並渲染（chat_view.png 可見 🛠 執行結果卡） |
| 3 | 蒸餾無水位記號——超過閾值後每輪重複蒸餾燒 token | 成本 | `distill_markers` 表記錄上次水位，增量達閾值才再蒸餾 |
| 4 | 多資料夾勾選時工作區取 HashSet 第一個（隨機） | 不確定行為 | 依專案資料夾宣告順序取第一個被勾選者 |
| 5 | `run_command` 空白切割——含空白的引號路徑被切碎 | 功能 | `split_command_line` 引號感知切割 + 2 個測試 |

附帶：中止鈕現在會丟棄遲到結果；標題列「檔案/檢視」改為真實選單（新增對話/設定/結束/語言/模式）；Token 預算列新增 ↻ 重設鈕；`handle_send` 的 DB unwrap 改為優雅失敗。

第二輪驗證：67/67 測試、clippy 0 警告、四視圖自截圖（含新增 chat 視圖）、qa_runner all **3/3 PASS**（9,337 tokens）。

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
