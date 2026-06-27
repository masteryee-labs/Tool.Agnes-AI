# 07 — 介面規格（對標 Codex / Antigravity 2.0 / Claude 桌面版）

> 技術：eframe/egui 0.31 原生渲染。零 Chromium。`ui/` 內的 HTML 原型僅作版面參考，最終以 egui 實作。

## 已實作現況（2026-06-27 第三輪：極簡黑+白暗色模式 + 無視窗執行 v0.9.0）

- **配色全面改版**：從 Claude 橘色風格改為極簡黑+白暗色模式（對標 Claude Code / Codex / Devin / Antigravity 2.0 終端美學）
  - `ACCENT_ORANGE` 由品牌橘 `(217,119,87)` 改為純白 `(235,235,235)`——按鈕、活躍狀態、強調元素
  - 新增 `TEXT_ON_ACCENT = (18,18,18)`——白色按鈕上的深色文字，確保對比度
  - 背景層次純黑→深灰漸層：`BG_SIDEBAR=(12,12,12)` → `BG_PRIMARY=(18,18,18)` → `BG_CARD=(28,28,28)` → `BG_TERTIARY=(36,36,36)` → `BG_HOVER=(44,44,44)`
  - 文字白→灰階：`TEXT_PRIMARY=(235,235,235)` / `TEXT_SECONDARY=(165,165,165)` / `TEXT_MUTED=(100,100,100)`
  - 狀態色保留但柔和化：綠/紅/黃僅用於狀態徽章，不干擾主視覺
  - 邊框低對比灰線 `BORDER=(48,48,48)`，讓內容聚焦
- **無視窗執行（CREATE_NO_WINDOW）**：新增 `no_window.rs` 模組，所有子進程在 Windows 上靜默執行，不再彈出 CMD/PowerShell 視窗
  - `silent_command()` 函數：建構已注入 `CREATE_NO_WINDOW` flag 的 `std::process::Command`
  - `NoWindowExt` trait：為既有 `std::process::Command` 注入 flag
  - `NoWindowExtTokio` trait：為 `tokio::process::Command` 注入 flag（MCP Server 用）
  - 套用範圍：sandbox.rs（run_command 工具）、agent.rs（rustc 編譯檢查）、loop_engine.rs（cargo test/check）、worktree.rs（git 操作）、validation.rs（clippy/check 閘門）、mcp.rs（MCP Server spawn）、qa_runner.rs（QA 測試）
- **Phase 5 自主迴圈 UI**：目標模式膠囊切換（💬 對話 / 🎯 目標）、目標輸入卡、迴圈狀態即時更新（500ms poll）、啟動/停止按鈕

## 上一輪現況（2026-06-11 第二輪：Claude Desktop 式改版 v0.6.0）

- **模組拆分**：GUI 由單檔 main.rs 拆出三個 bin 模組——`ui_theme.rs`（全部色彩/圓角/間距/字級具名常數 + `apply_theme`）、`ui_chat.rs`（訊息流渲染）、`ui_panels.rs`（右側三 Tab 面板）；main.rs 淨縮約 1,000 行
- **頂列麵包屑**：「📁 專案名｜🌍 全域 / Session 標題或新對話」；右叢集：檔案/檢視選單、語言、▤ 右面板開關、Token 預算條、⚙
- **左側欄**：白色「＋ 新增對話」整寬鈕 → 膠囊 segmented「專案｜全域」（切 Tab 同步 work_mode 與 config）→「最近」目前範疇最新 8 筆 Session（相對時間、hover 🗑）→ 專案摺疊樹/全域列表 → 底部 ⚙ + 模式徽章
- **訊息流（Claude Desktop 式）**：
  - user＝整寬淡色卡；assistant＝無氣泡純文字
  - **Think 收闔**：工具標籤前的自由文字 >3 行渲染為「✱ 思考過程（N 行）›」弱色斜體列，預設收闔；`<think>` 標籤同樣處理；最終結論永遠直出
  - **活動卡片**：每個 `<read_file>/<write_file>/<run_command>/<run_mcp>` 標籤＝可收闔活動列（「📖 讀取: path ›」等），展開顯示參數＋依序配對的 tool 結果；連續多列聚成「⚙ 執行了 N 個動作 ›」群組卡；配對掉的 tool 訊息不再重複渲染
  - **變更 chips**：對話尾端列出「✎ 檔名 +a −r」（diff stats 按 change_id 快取於 UiState），點擊開右面板「變更」Tab
- **右側面板三 Tab**：🤖 代理人（22 人 G1–G6 + 待審批，功能沿用）｜✎ 變更（file_changes 列表 + diff 視圖：行號、綠底/紅底、diff/全文切換）｜📄 檔案（唯讀檢視器，>file_viewer_max_bytes 顯示過大提示）
- **檔案變更追蹤後端**：`file_changes` 表（db.rs）+ `diffview.rs` 行級 LCS diff（`line_diff`，DP 保險絲退化全刪全增）；agent.rs write_file 在 strip_secrets 後快照前後內容，`AgentLoop::set_conversation_id` 掛載於送出與 Approve 兩路徑
- **新組態**（GeneralConfig，皆 serde default，舊 config 可照常載入）：`right_panel_open_default=true`、`diff_view_max_lines=800`、`file_viewer_max_bytes=512000`
- **file_changes 保留策略**（`[file_changes]` 組態節，FileChangesConfig，皆 serde default）：單筆 before/after 超過 `content_max_bytes=512000` 截斷至 UTF-8 邊界並附截斷標記；每對話只留最新 `keep_per_conversation=200` 筆（插入時刪最舊）；刪除對話時級聯清空該對話全部快照——三道閘防止全文快照讓 DB 無界成長
- **QA 鉤子**：AGNES_QA_VIEW=global 現同步切 work_mode（與真實點擊一致）；展示對話種子腳本 `scratch/qa_seed_demo_conv.py`；v0.6 實證截圖 `qa_screenshots/v060_*.png`

## 上一輪現況（2026-06-11 第一輪）

- **側邊欄 Tab 化**：頂部「📁 專案｜🌍 全域」雙 Tab，切 Tab 即切工作模式（同步寫回 `config.general.project_mode`）
  - 專案 Tab：「＋ 新增專案」直接挑資料夾建專案；每個專案為摺疊節點，底下巢狀該專案的對話 Session（點擊載入續聊、🗑 刪除）與資料夾管理子摺疊
  - 全域 Tab：灰色說明列 + 全域範疇 Session 清單（`conversations.project_id = 'global'` 哨兵）
- **Session 持久化**：`conversations` 表新增 `project_id` 欄（含舊庫 ALTER 遷移與孤兒補綁）；新 Session 自動掛目前範疇
- **API 金鑰 UX**：儲存後顯示遮罩金鑰（頭5尾4）+ SHA-256 指紋 + 常駐綠色「已儲存 ✓」
- **全域字級**：egui text_styles 拉高（Body 16 / Button 15.5 / Mono 14.5 / Small 13），右側代理人面板與各小字同步放大
- **MCP 設定**：「＋ 新增伺服器」為真表單（名稱/指令/引數 → 寫入 config 並立即啟動）；伺服器 toggle 即時 start/stop；🗑 刪除；工作區 `.mcp.json`（Claude 格式）唯讀列出；App 啟動時自動啟動 config + .mcp.json 全部啟用伺服器
- **技能設定區**：列出工作區 `.claude/skills/*/SKILL.md`（Claude 格式），對話以 `/名稱` 呼叫
- **介面縮放**（v0.5.0）：`config.general.ui_scale`（預設 1.25，範圍 1.0–1.75），設定→一般→介面縮放下拉即時生效；先前強制 `pixels_per_point=1.0` 是高解析螢幕字太小的根因
- **22 代理人面板跟隨 Session**（v0.5.0）：副標顯示目前範疇（📂 專案 / 💬 Session 或「範疇：全域」）；每輪審查以取代式寫入 `conversation_audits` 表（不可用 audit_logs——其 task_id 外鍵在 rusqlite bundled 預設啟用 FK 下會拒絕對話 id），點擊 Session 即還原該輪狀態；圖例：✓ 通過、✗ 否決、~ 跳過、· 休眠（灰名稱）；hover 列顯示該代理裁決原因

## 版面結構

```
┌──────────────────────────────────────────────────────────────┐
│ 📁 專案｜🌍 全域 / Session │ 選單 │ EN │ ▤ │ Token 計量 │ ⚙   │
├──────────┬───────────────────────────────┬───────────────────┤
│ 左側欄    │ 中央訊息流                     │ 右側面板（三 Tab） │
│           │  user 淡色卡                  │ 🤖 代理人          │
│ +新對話   │  ✱ 思考過程（N 行）›（收闔）    │   22人 G1–G6 樹    │
│ 專案｜全域 │  📖 讀取: path ›（活動列）     │   待審批 Gate      │
│ 最近      │  ⚙ 執行了 N 個動作 ›（群組）   │ ✎ 變更            │
│  Session… │  結論文字（永遠直出）           │   檔案列表+diff    │
│ 專案樹    │  ✎ 檔名 +a −r（變更 chips）    │ 📄 檔案            │
│  ├ 專案A  │  ┌─────────────────────────┐  │   唯讀檢視器       │
│  └ 專案B  │  │ 輸入卡（圓角 14）         │  │                   │
│ ⚙ 設定   │  │ + │📁▾│🌍│   模型名│(↑) │  │                   │
│  +模式徽章│  └─────────────────────────┘  │                   │
└──────────┴───────────────────────────────┴───────────────────┘
```

## Projects（對標 Antigravity 專案管理）

- 專案 = 名稱 + **一或多個資料夾**（已實作：db.rs `projects` 表、`update_project_folders`）
- 專案設定頁：資料夾清單（可增刪）、分支選擇、安全預設（Security Preset）下拉、Artifact 審查策略
- 新對話預設綁定當前專案 → `Orchestrator.set_workspaces(folders)`，路徑圈禁即時生效
- 側欄專案樹：專案 → 對話列表（含相對時間），對標 Antigravity 左欄

## 模式切換（輸入框下拉，對標 Codex「本機作業」選單）

| 模式 | UI 標示 | 行為 |
|---|---|---|
| Project | 📁 專案名 | 圈禁於專案資料夾 |
| 多資料夾 | 📁 N 個資料夾 | `execute_multi_folder` |
| 全域（Hermes 式） | 🌐 紅色徽章 | `global_execute`，右側面板強制展開，逐項確認 |

全域模式進入時彈出一次性警示對話框，列出 AllowedPaths 與封鎖清單；Critical 動作要求輸入關鍵詞二次確認（見 05）。

## ConfirmationGate 右側面板

- PendingAction 卡片：描述、目標路徑、風險徽章（Low 灰/Medium 黃/High 白/Critical 紅）、產生它的代理人
- 操作：單項 Approve/Reject（Reject 附理由欄）、「全部核准 Low」批次鈕（Medium 以上禁止批次）
- 執行後卡片轉為結果態：Exit Code、Stderr 摘要（截斷至組態行數）、虛假回報攔截標記

## 代理人狀態樹

- 22 人按 6 組折疊顯示；狀態：休眠（灰）/ 激活（藍）/ 審查中（黃）/ REJECT（紅）
- 點擊代理 → 顯示其最近 gate 結果與耗用 token（讀 `audit_logs` + `token_ledger`）

## Token 計量表（標題列）

- 即時顯示本會話 prompt/completion 消耗與預算水位（TokenBudgeter，見 04）
- 80% 變黃、100% 變紅鎖定（僅確定性操作可續）

## 設定頁

- API 金鑰輸入（只顯示 hash 指紋；落地 `config.local.toml`）
- 模型路由表（flash/主力/高檔三檔位映射）
- 沙盒組態（逾時、白名單增刪——增刪屬 Critical 動作走確認閘門）
- 語言（zh-TW/en，自動偵測）、Shell 選擇（PowerShell/cmd）
- MCP 伺服器管理（mcp.rs）

## egui 實作守則

- 主執行緒只渲染：所有 I/O、API 呼叫經 `tokio` 背景任務 + `mpsc` 通道回傳，幀率守 60FPS（ResourceAnalyticsEngineer 規則）
- 長清單（日誌/對話）用 `egui::ScrollArea::show_rows` 虛擬化
- 中文字型：`default_fonts` 已啟用；需驗證 CJK fallback（Noto Sans TC 內嵌，Phase 2）
- 狀態持久化：視窗大小/面板寬度進 `config.local.toml` 的 `[ui]` 段
