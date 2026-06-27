# Agnes AI — 多代理安全引擎

> English version: [README.md](README.md)

Agnes AI 是一款高防禦、極速的桌面 AI 代理，以 **純 Rust + egui 原生 GUI** 打造（零 Chromium / WebView）。內建 22 代理編排管線與零信任安全模型：模型提出的每一個動作，都必須先通過確定性驗證閘門，才會碰到你的系統。

## 特色

### 核心體驗

- **原生 Rust GUI** — eframe/egui + wgpu，沒有內嵌瀏覽器，啟動即開、體積極小
- **極簡暗色介面** — 純黑加白色的配色，對標 Claude Code / Codex / Devin / Antigravity 2.0 的終端美學，沒有干擾視線的品牌色
- **靜默執行** — 所有子程序（shell 指令、編譯器、git、MCP 伺服器）在 Windows 上都以 `CREATE_NO_WINDOW` 方式執行，不會在你的桌面彈出 CMD 或 PowerShell 視窗

### 工作區

- **專案／全域雙模式** — 側邊欄分頁一鍵切換：
  - **專案模式**：任選資料夾就能建立專案；每個對話都掛在所屬專案底下；對話記錄存進 SQLite，隨時點開都能從上次中斷的地方繼續
  - **全域模式**：操作整台電腦的專屬分頁，每一個動作都要你逐項確認後才會執行

### 自主迴圈（Phase 5）

- **目標驅動迴圈** — 給它一個目標和退出條件，它就自己跑 Discover → Plan → Execute → Verify → Iterate，直到條件達成或迭代次數用完
- **子代理架構** — 三個獨立角色，各有獨立的 prompt 和對話狀態：
  - **Planner（規劃者）** — 把目標拆成原子子任務
  - **Generator（生成者）** — 一次實作一個子任務，呼叫 `write_file` / `run_command` 工具
  - **Evaluator（評估者）** — 獨立驗證生成者的產出；只說「我做完了」但沒有實際產出，直接退回
- **Git Worktree 隔離** — 每個生成者子代理都在獨立的 git worktree 和分支裡工作，多個子代理並行也不會互相踩檔案，完成後合併回主分支
- **跨對話記憶** — 教訓、雷庫、迴圈進度都存在 `.agent/memory/` 裡，換了對話也能接著上次的進度繼續

### 安全與驗證

- **22 代理驗證管線** — 模型的每個工具呼叫都會經過確定性閘門交叉驗證（路徑圈禁、Shell 注入偵測、金鑰外洩掃描、AI 廢話審計等），任何一關不過就否決
- **沙盒對齊** — 寫入的 `.rs` 檔案會立刻編譯（還會實際跑測試）；「嘴上說成功但根本編譯不過」當場退回
- **不信口頭報告** — Exit Code == 0 且 stderr 為空才算成功；模型說「成功了」從來不被信任

### 相容性

- **Claude 相容 Skills** — 在工作區放 `.claude/skills/<名稱>/SKILL.md`，對話輸入 `/名稱` 就能呼叫；`CLAUDE.md` 專案規則自動載入
- **Claude 相容 MCP** — 工作區根目錄放標準 `.mcp.json`，或在「設定 → MCP 伺服器」裡新增；連上線的伺服器工具清單會自動暴露給模型

### 效能

- **分層記憶** — 滑動視窗分塊 + 三階段漏斗 RAG（FTS5 索引），蒸餾水位記號避免重複燒 token
- **速率限制與 20 RPM 防護** — 全域共享的令牌桶限流器把關每一次 API 呼叫（包含蒸餾和檢索）；令牌不夠時會等補充而不是直接拒絕，突發流量也不會突破免費方案每分鐘 20 次的上限。遇到 429 就用倍率式指數退避重試。所有參數都從設定檔讀取（`max_rpm`、退避參數），沒有寫死的 Magic Number
- **Token 經濟** — 每個對話有 token 預算硬上限，標題列即時顯示用量。請求次數從設計面壓低：Stage 0 先在本機做 FTS5 記憶查詢，命中就完全跳過檢索 API（0 次 API 呼叫），漏斗 RAG 的 Stage 1+2 也合併成一次呼叫（2 次 → 1 次）

## 安裝與建置

前置需求：[Rust 工具鏈](https://rust-lang.org/)（stable，2021 edition）。

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# 啟動 GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

## 設定

所有本機設定都在倉庫根目錄的 `config.local.toml`（自動建立，**已加入 .gitignore**，你的 API 金鑰永遠不會進版本控制）。

最簡單的方式是直接用程式內的設定頁（側邊欄的 ⚙）：

1. **設定 → API 與模型** — 貼上 API 金鑰，按**儲存**。頁面會顯示已存金鑰的遮罩版（`sk-xx…xxxx`）、指紋，以及綠色的「已儲存 ✓」，讓你隨時知道目前生效的是哪把金鑰。
2. **設定 → MCP 伺服器** — 按「＋ 新增伺服器」，填入名稱、指令、引數；伺服器會立刻啟動並寫進設定檔。
3. **設定 → 技能 Skills** — 列出目前工作區偵測到的所有技能。

`config.local.toml` 手動設定範例：

```toml
[api]
key = "{{API_KEY}}"
model = "agnes-2.0-flash"
session_budget = 500000

[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"]
```

### Claude 格式 MCP（工作區的 `.mcp.json`）

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"],
      "env": { "LOG_LEVEL": "info" }
    }
  }
}
```

### Claude 格式 Skills

```
你的專案/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter（name + description）+ 指示內文
```

對話輸入 `/deploy …` 就能呼叫。技能和 `CLAUDE.md` 規則會以確定性方式注入系統提示，不額外消耗 API 呼叫。

## 使用方式

### 對話模式

1. **建立專案** — 側邊欄 → 專案分頁 → **＋ 新增專案**，選一個資料夾。
2. **開始對話** — 輸入任務，新對話會自動掛在目前專案底下並持久化。之後點側邊欄裡任一對話就能載入完整歷史、繼續工作。
3. **全域模式** — 切到**全域**分頁，就能在專案資料夾以外操作。每個動作都會出現在右側面板，讓你逐項批准或拒絕。
4. **觀察代理** — 右側面板顯示 22 個驗證代理每一步的 PASS / REJECT 結果；待確認的工具呼叫也在這裡等你核准。

### 目標模式

1. **切換到目標模式** — 點中央面板上方的膠囊切換鈕（💬 對話 → 🎯 目標）。
2. **描述目標** — 輸入你想完成的事，以及退出條件（例如 `file:Docs/report.md exists`）。
3. **按下開始** — 迴圈會自主運行：規劃者拆解目標、生成者實作每個子任務、評估者逐一驗證。狀態面板即時更新（目前階段、迭代次數、剩餘預算）。
4. **隨時停止** — 按停止鈕就會立刻中斷迴圈。

## 安全模型

- API 金鑰只存在 `config.local.toml`（已 git 忽略）；原始碼裡出現 `sk-` 字串＝一票否決
- 指令以引數向量執行，不做 shell 字串拼接
- 路徑圈禁：專案模式下，工作區以外的檔案操作一律拒絕
- 原始擷取 Exit Code 與 stderr；絕不信任模型口頭宣稱的「成功」
- 全域速率限制器加上 429 指數退避，保護金鑰和帳號不被速率限制鎖定；任何單一子系統（包含記憶歸檔）都無法繞過共享的 20 RPM 上限

## 開發文件

完整的架構、記憶系統、QA、安全規格都在 [`Docs/`](Docs/00_OVERVIEW.md)；代理編排規則在 `.agent/rules/*.toon`。

## 授權

MIT
