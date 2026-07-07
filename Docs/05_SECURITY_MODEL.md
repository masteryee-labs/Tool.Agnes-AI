# 05 — 零信任安全模型

> 規則檔：`.agent/rules/security.toon`（含安全策略 + 確認閘門）。
> 核心立場：模型輸出 = 不可信外部輸入。所有執行過五道防線。

## 五道防線

```
模型輸出
 → [1] 確定性驗證閘門（03 的 D1–D8，sandbox.rs）
 → [2] 風險分級（ActionRiskLevel: Low/Medium/High/Critical，orchestrator.rs:58）
 → [3] ConfirmationGate 使用者確認（orchestrator.rs:153）
 → [4] 沙盒執行（run_in_sandbox：引數向量化、無 shell 字串拼接）
 → [5] Exit Code + Stderr 硬性對齊（is_false_positive 攔虛假回報）
```

## 金鑰隔離（已實作，config.rs）

- 金鑰僅存 `config.local.toml`；GUI 輸入 → `write_api_key` 落地 → 重啟 `read_api_key` 自動加載
- `ensure_gitignore` 啟動時自動追加 `config.local.toml` 與 `.env`（.gitignore 已含）
- 任何 `.rs`、任何模型輸出、任何日誌出現 `sk-` 前綴實際金鑰 → SecurityComplianceAuditor 一票否決（D6）
- 日誌與 UI 僅顯示 `hash_key` 後的指紋
- prompt 中金鑰一律 `{{API_KEY}}` 佔位符，送出前由 Rust 端替換——金鑰永不進入模型上下文

## 多 API Key 輪詢（已實作，key_rotation.rs）

Agnes AI 免費方案每帳號有獨立速率上限（20 RPM）。使用者可註冊多個帳號、各別取得 API Key，在這些 Key 之間輪詢以盡量不觸及任一帳號上限：

- **組態**：`[api] keys = ["sk-a", "sk-b", "sk-c"]`（金鑰組，非空時優先於 `key`）+ `key_rotation_every = 15`（每把 Key 連續使用 N 次後輪替，0=預設 15）
- **計數輪詢**：`KeyRotator::next_key()` 每次回傳目前 Key 並累加計數，達閾值自動切下一把——流量平均分散到所有帳號
- **429 強制換 Key**：`send_api_request` 收到 420/429 時呼叫 `mark_rate_limited()` 立即跳下一把 Key 重試，不必乾等退避（多帳號方案核心收益）
- **單 Key 退化**：只有一把 Key 時不輪詢，行為等同舊版，向後相容
- **共享範圍**：`AppState.key_rotator` 為 App 級單一共享輪詢器，所有 Agent / 多資料夾並行 / 子代理（Planner/Generator/Evaluator）/ 多模態共用同一計數，流量在所有帳號間均勻分散
- **安全不變**：多把金鑰同樣只存 `config.local.toml`（gitignore），UI/日誌只顯示各別指紋，永不進入模型上下文

## 沙盒規格

| 層級 | 機制 | 狀態 |
|---|---|---|
| L1 行程隔離 | std::process + 引數向量分離 + 工作目錄圈禁 + 逾時殺除 + CREATE_NO_WINDOW | 已實作（sandbox.rs + no_window.rs） |
| L2 WASM | wasmtime 執行不可信代碼片段 | 規劃（Phase 3） |
| L3 Docker | 編譯/測試級任務的完整隔離（`--network=none` 預設） | 規劃（Phase 3，桌面端限定） |

語系前置（locale.rs）：任何沙盒執行前 Windows 注入 `chcp 65001`、Unix 注入 `LANG=zh_TW.UTF-8` + `LC_ALL=zh_TW.UTF-8`，確保 Stderr 解碼 100% 正確——亂碼的 Stderr 會讓自愈迴圈誤判，這是安全問題不只是顯示問題。

無視窗執行（no_window.rs）：Windows 上所有子進程（sandbox、rustc、cargo、git、MCP Server）統一注入 `CREATE_NO_WINDOW` flag，在背景靜默執行，不彈出 CMD/PowerShell 視窗干擾使用者桌面。對標 Claude Code / Codex / Devin / Antigravity 2.0 的嵌入式終端體驗。

## 全域模式（Hermes 式全電腦代理）的「互相確認」協議

全域模式（`global_execute`）權力最大，因此約束最多：

1. **雙向確認**：每個 PendingAction 必須 (a) 通過機器側 Gate 1–4（security.toon）且 (b) 使用者在 UI 逐項 Approve。任一否決即丟棄。Critical 級另要求使用者輸入動作摘要關鍵詞二次確認（防誤點）。
2. **路徑白名單**：僅 AllowedPaths 列出的根目錄；`C:\Windows`、`C:\System32` 永久封鎖，白名單修改本身屬 Critical 級。
3. **無靜默升權**：全域模式下不存在 auto_approve（四個風險級全部 `auto_approve: false`）。
4. **完整審計**：每筆 Approve/Reject/執行結果進 `audit_logs` 表，UI 可回放。
5. **會話範圍**：全域模式授權單次會話有效，重啟歸零。

## 威脅模型對照

| 威脅 | 防線 |
|---|---|
| 模型生成惡意/錯誤指令 | D1–D8 確定性閘門 + 風險分級 + 使用者確認 |
| 提示注入（讀入的檔案內容夾帶指令） | 工具結果以資料層注入，標記不可信來源；高風險動作仍需閘門，注入無法繞過 Rust 端驗證 |
| 虛假回報 | Exit Code 對齊，`is_false_positive` |
| 金鑰外洩 | 本機隔離 + gitignore + 佔位符 + 掃描否決 |
| 路徑逃逸 | `has_path_traversal_component` + `is_path_in_workspace` |
| 引數注入 | `sanitize_all_args` + 向量化分離 + `check_indirect_shell_injection` |
| 死循環燒錢 | WorkflowRuntimeEvaluator 3 輪重置 / 50 輪強制終止 + TokenBudgeter 鎖定 |
| 中文亂碼導致誤判 | locale.rs 前置校準 |

## 已知待修（P1）

- 根目錄與 `src-tauri/` 下有名為 `nul` 的 Windows 裝置名殘留檔，會干擾部分 Git 工具，需以 `\\.\nul` 語法刪除
- `tauri.conf.json` CSP 為 null——Tauri 路徑棄用後整組移除（見 01）
- run_error.log 為空檔殘留，可刪
