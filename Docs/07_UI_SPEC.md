# 07 — 介面規格（對標 Codex / Antigravity 2.0 / Claude 桌面版）

> 技術：eframe/egui 0.31 原生渲染。零 Chromium。`ui/` 內的 HTML 原型僅作版面參考，最終以 egui 實作。

## 版面結構

```
┌──────────────────────────────────────────────────────────────┐
│ 標題列：Agnes AI ▾ │ 模式徽章 │ Token 計量表 │ ⚙ 設定         │
├──────────┬───────────────────────────────┬───────────────────┤
│ 左側欄    │ 中央對話/工作區                │ 右側面板           │
│           │                               │                   │
│ +新對話   │  訊息流（使用者/代理/審查記錄）  │ ConfirmationGate  │
│ 對話歷史  │                               │  PendingActions   │
│ 排程任務  │  ┌─────────────────────────┐  │  [Approve][Reject]│
│           │  │ 輸入框                    │  │                   │
│ Projects  │  │ + │模型選擇▾│風險徽章│🎤│  │ 代理人狀態樹       │
│  ├ 專案A  │  │ 📁 專案▾ │模式▾ │分支▾ │  │  (22人/休眠/激活) │
│  └ 專案B  │  └─────────────────────────┘  │                   │
│ 設定      │                               │ 審計日誌(可回放)   │
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

- PendingAction 卡片：描述、目標路徑、風險徽章（Low 灰/Medium 黃/High 橘/Critical 紅）、產生它的代理人
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
