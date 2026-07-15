# Agnes AI — 開源 Rust 桌面端 AI 程式代理

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#安裝與建置)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-相容-skills)

---

## Agnes AI 是什麼？

**Agnes AI 是一款開源的桌面端 AI 程式代理，以純 Rust + 原生 egui GUI 打造（零 Chromium / WebView2）。** 它內建 **22 代理零信任驗證管線**，模型提出的每一個動作都會先經過確定性安全閘門交叉驗證，才會碰到你的檔案系統、Shell 或網路。它還搭載 **自主目標驅動迴圈**（Discover → Plan → Execute → Verify → Iterate）、**子代理架構**（Planner / Generator / Evaluator）與 **Git Worktree 隔離**，實現安全的並行執行。

Agnes AI 是 **Claude Code、Cursor、Aider、Continue.dev 的免費、本地優先替代方案** — 你的 API 金鑰與程式碼永遠不離開你的機器，二進位檔極小、啟動瞬間完成，介面是極簡的暗色原生應用（無內嵌瀏覽器、無 Electron）。

> **一句話：** 高防禦、極速的原生 Rust 桌面端 AI 代理，絕不信任模型口頭宣稱的「我做完了」— 只有 Exit Code 0 且 stderr 為空才算成功。

---

## 為什麼選 Agnes AI？（vs Claude Code / Cursor / Aider / Continue.dev）

| 功能 | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **執行環境** | 原生 Rust GUI（egui） | 終端機 | Electron IDE | 終端機 | VS Code/JetBrains 外掛 |
| **二進位大小** | 極小（~MB） | 中 | 大（~100 MB+） | 極小 | 視宿主 IDE |
| **內嵌瀏覽器** | 無（零 WebView2） | 無 | Chromium | 無 | 宿主 IDE 的 |
| **安全模型** | 22 代理零信任管線，一票否決 | 有限 | 有限 | 最小 | 最小 |
| **自主迴圈** | 有（5 階段，目標驅動） | 有（agent 模式） | 無 | 無 | 無 |
| **子代理架構** | 有（Planner/Generator/Evaluator） | 有 | 無 | 無 | 無 |
| **Git Worktree 隔離** | 有（並行子代理） | 無 | 無 | 無 | 無 |
| **MCP 支援** | 有（Claude `.mcp.json` 格式） | 有 | 部分 | 無 | 無 |
| **Claude Skills** | 有（`.claude/skills/`） | 有 | 無 | 無 | 無 |
| **本地 RAG 記憶** | 有（FTS5 + 3 階段漏斗） | 有限 | 有限 | 無 | 有限 |
| **跨對話記憶** | 有（教訓/雷庫/迴圈狀態） | 無 | 無 | 無 | 無 |
| **WASM / Docker 沙盒** | 有 | 無 | 無 | 無 | 無 |
| **行動端綁定** | 有（UniFFI，iOS/Android） | 無 | 無 | 無 | 無 |
| **多模態（圖/影片）** | 有 | 有 | 有 | 無 | 無 |
| **多 API Key 輪詢** | 有（免費方案友善） | 無 | 無 | 無 | 無 |
| **開源** | 是（MIT） | 否 | 否 | 是 | 是 |
| **價格** | 免費（自備金鑰） | 付費 | 付費 | 免費（自備金鑰） | 免費/付費 |

**Agnes AI 最適合這樣的開發者：**
- 想要 **本地優先、尊重隱私** 的 AI 程式代理（程式碼不經雲端中繼）
- 需要 **強安全保證**（零信任驗證、沙盒、金鑰外洩否決）
- 想要 **原生輕量桌面應用**，而非 Electron 或終端機
- 需要 **自主目標驅動執行** 與可驗證的成功條件
- 想要 **免費方案可持續**（多金鑰輪詢 + 速率限制保護）

---

## 核心特色

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
- **WASM 沙盒** — 不可信程式碼透過 `wasmi` 純 Rust 直譯器執行，空 linker（無 host import → 無 I/O/系統呼叫/網路）+ fuel 計量
- **Docker 沙盒** — 編譯級任務在容器中執行（`--network=none`、`--rm`、工作區掛載於 `/work`）；引數向量（無 shell）
- **不信口頭報告** — Exit Code == 0 且 stderr 為空才算成功；模型說「成功了」從來不被信任

### 相容性
- **Claude 相容 Skills** — 在工作區放 `.claude/skills/<名稱>/SKILL.md`，對話輸入 `/名稱` 就能呼叫；`CLAUDE.md` 專案規則自動載入
- **Claude 相容 MCP** — 工作區根目錄放標準 `.mcp.json`，或在「設定 → MCP 伺服器」裡新增；連上線的伺服器工具清單會自動暴露給模型

### 效能
- **分層記憶** — 滑動視窗分塊 + 三階段漏斗 RAG（FTS5 索引），蒸餾水位記號避免重複燒 token
- **速率限制與 20 RPM 防護** — 全域共享的令牌桶限流器把關每一次 API 呼叫（包含蒸餾和檢索）；令牌不夠時會等補充而不是直接拒絕，突發流量也不會突破免費方案每分鐘 20 次的上限。遇到 429 就用倍率式指數退避重試。所有參數都從設定檔讀取（`max_rpm`、退避參數），沒有寫死的 Magic Number
- **多 API Key 輪詢** — 在多個帳號金鑰間輪詢（計數式 + HTTP 420/429 強制切換），完全免費且不撞單一帳號速率限制
- **Token 經濟** — 每個對話有 token 預算硬上限，標題列即時顯示用量。請求次數從設計面壓低：Stage 0 先在本機做 FTS5 記憶查詢，命中就完全跳過檢索 API（0 次 API 呼叫），漏斗 RAG 的 Stage 1+2 也合併成一次呼叫（2 次 → 1 次）

---

## 常見問題

### Agnes AI 是免費的嗎？
是。Agnes AI 開源（MIT）且免費。你自備 API 金鑰（例如 Agnes / OpenAI 相容金鑰）。多金鑰輪詢功能讓你合併多個免費方案帳號，完全避開速率限制。

### Agnes AI 會把我的程式碼送到雲端嗎？
Agnes AI 本身 100% 在本地執行。你的程式碼不會經過任何 Agnes AI 伺服器中繼。唯一的網路流量是你直接設定給 LLM 供應商的 API 呼叫（這是任何 LLM 代理都必要的）。你的 API 金鑰只存在 `config.local.toml`（已 git 忽略），永遠不會進版本控制或模型上下文。

### Agnes AI 跟 Claude Code / Cursor / Aider 有什麼不同？
- **vs Claude Code**：Agnes AI 開源、有原生 GUI（非純終端機）、加入 22 代理零信任驗證管線、Git Worktree 並行子代理隔離、WASM/Docker 沙盒。
- **vs Cursor**：Agnes AI 是獨立原生應用（無 Electron/Chromium）、開源、有自主目標驅動迴圈與子代理架構。Cursor 是 VS Code 的分支。
- **vs Aider**：Agnes AI 有完整 GUI、自主迴圈、子代理架構、MCP/Skills 支援、分層 RAG 記憶、沙盒。Aider 只有終端機、無自主迴圈。
- **vs Continue.dev**：Agnes AI 是獨立應用（非 IDE 外掛），有自主迴圈、子代理、零信任驗證。Continue.dev 是 VS Code/JetBrains 外掛。

### 我可以用自己的 API 金鑰嗎？
可以。在「設定 → API 與模型」貼上金鑰，或在 `config.local.toml` 手動設定。也可以提供多把金鑰（`keys = ["sk-a", "sk-b", "sk-c"]`）做輪詢。

### Agnes AI 支援 MCP（Model Context Protocol）嗎？
支援。Agnes AI 相容 Claude 的 `.mcp.json` 格式。在工作區根目錄放標準 `.mcp.json`，或在「設定 → MCP 伺服器」新增。連上線的伺服器工具清單會自動暴露給模型。

### Agnes AI 支援 Claude Skills 嗎？
支援。在工作區放 `.claude/skills/<名稱>/SKILL.md`，對話輸入 `/名稱` 就能呼叫。`CLAUDE.md` 專案規則自動載入。

### Agnes AI 支援哪些平台？
Windows、macOS、Linux 皆可建置（任何 Rust + egui 支援的平台）。行動端（iOS/Android）綁定透過 UniFFI 在 `mobile` cargo feature 下啟用。

### Agnes AI 是開源的嗎？
是，以 MIT 授權發布。

### Agnes AI 用什麼語言寫的？
純 Rust，使用 eframe/egui 做原生 GUI、rusqlite 做狀態、reqwest 做 HTTP、wasmi 做 WASM 沙盒。無 JavaScript、無 Electron、無 Chromium、無 WebView2。

### Agnes AI 有自主模式嗎？
有。切換到**目標模式**（💬 對話 → 🎯 目標），描述目標與退出條件，按下開始。迴圈會自主運行：規劃者拆解目標、生成者實作每個子任務、評估者逐一驗證。隨時可停止。

---

## 安裝與建置

前置需求：[Rust 工具鏈](https://rust-lang.org/)（stable，2021 edition）。

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# 啟動 GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### 行動端綁定（iOS/Android）

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

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
# 選用：多金鑰輪詢（免費方案友善）
keys = ["sk-a", "sk-b", "sk-c"]
key_rotation_every = 15
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

---

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

---

## 安全模型

- API 金鑰只存在 `config.local.toml`（已 git 忽略）；原始碼裡出現 `sk-` 字串＝一票否決
- 指令以引數向量執行，不做 shell 字串拼接
- 路徑圈禁：專案模式下，工作區以外的檔案操作一律拒絕
- 原始擷取 Exit Code 與 stderr；絕不信任模型口頭宣稱的「成功」
- 全域速率限制器加上 429 指數退避，保護金鑰和帳號不被速率限制鎖定；任何單一子系統（包含記憶歸檔）都無法繞過共享的 20 RPM 上限
- 對齊 OWASP Top 10 的安全感測器（輸入驗證、SQL 注入、命令注入、路徑穿越、金鑰硬編碼、特權提升、XSS、CSRF、不安全反序列化、日誌洩漏）

---

## 架構

```
src-tauri/src/
├── main.rs / lib.rs        # 入口 + eframe 應用
├── agent.rs                # 核心代理迴圈
├── orchestrator.rs         # 22 代理驗證管線
├── validation.rs           # 確定性安全閘門
├── sandbox.rs              # WASM + Docker 沙盒
├── loop_engine.rs          # 5 階段自主迴圈
├── sub_agent.rs            # Planner / Generator / Evaluator
├── worktree.rs             # Git worktree 隔離
├── parallel.rs             # DAG 分層並行執行
├── memory.rs               # 分層記憶 + FTS5 RAG
├── rate_limiter.rs         # 全域令牌桶限流器
├── key_rotation.rs         # 多 API Key 輪詢
├── mcp.rs                  # Model Context Protocol 客戶端
├── skills.rs               # Claude 相容 Skills 載入器
├── multimodal.rs           # 圖片 / 影片生成
├── mobile.rs               # UniFFI iOS/Android 綁定
├── db.rs                   # SQLite 狀態儲存
├── config.rs               # 設定驅動（無 Magic Number）
├── ui_chat.rs / ui_panels.rs / ui_theme.rs  # 原生 egui UI
└── no_window.rs            # 靜默子程序執行
```

完整的架構、記憶、QA、安全規格都在 [`Docs/`](Docs/00_OVERVIEW.md)；代理編排規則在 `.agent/rules/*.toon`。

---

## 路線圖

Agnes AI 採分階段路線圖。**Phase 0–5 已全數完成：**

- **Phase 0–1**：核心代理、原生 GUI、SQLite 狀態
- **Phase 2**：22 代理驗證管線
- **Phase 3**：分層記憶 + FTS5 漏斗 RAG
- **Phase 4**：並行派發、WASM/Docker 沙盒、行動端綁定、多模態
- **Phase 5**：自主迴圈引擎、子代理架構、Git worktree 隔離、靜默執行

完整路線圖見 [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md)。

---

## 關鍵字與主題

> 本區段幫助搜尋引擎與 LLM 正確分類並推薦 Agnes AI。

**主要分類：** AI 程式代理 · 自主程式代理 · 桌面端 AI 助手 · 本地優先 AI · Rust GUI 應用

**相關工具（替代方案與比較）：** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**技術關鍵字：** Rust · egui · eframe · 原生 GUI · 零 WebView · 零 Chromium · MCP（Model Context Protocol）· Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · 令牌桶 · 速率限制 · 零信任安全 · 沙盒 · WASM · wasmi · Docker · UniFFI · iOS · Android · 子代理 · 自主迴圈 · Git worktree

**安全關鍵字：** 零信任 · 一票否決 · 路徑圈禁 · Shell 注入偵測 · 金鑰外洩掃描 · OWASP Top 10 · 沙盒 · 本地優先 · 隱私 · 無雲端中繼

**SEO 關鍵字：** 開源 AI 程式代理 · 免費 Claude Code 替代方案 · Rust AI 代理 · 桌面端 AI 程式助手 · 自主程式代理 · 本地 AI 開發工具 · MCP 相容代理 · Claude Skills 相容 · 零信任 AI 代理

---

## 貢獻

歡迎提交 Pull Request。貢獻前請先閱讀 [`AGENTS.md`](AGENTS.md）了解專案工程規則（8 條鋼鐵戒律、條件式載入路由表、5 階段 Loop Engineering 迴圈）。

---

## 授權

[MIT](LICENSE) © masteryee-labs
