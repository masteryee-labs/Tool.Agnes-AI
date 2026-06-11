# Agnes AI — 多代理人安全引擎

> For the English version, see [README.md](README.md)

Agnes AI 是一套高防禦、極速的桌面 AI 代理人，以 **純 Rust + egui 原生 GUI** 打造（零 Chromium / WebView）。內建 22 代理人編排管線與零信任安全模型：模型提出的每一個動作，都先通過確定性驗證閘門才會碰到你的系統。

## 特色

- **原生 Rust GUI** — eframe/egui + wgpu，無內嵌瀏覽器，啟動即開、體積極小
- **專案／全域雙工作區** — 側邊欄 Tab 一鍵切換：
  - **專案**：任選資料夾直接建立專案；每個對話 Session 巢狀掛在所屬專案底下；對話記錄存於 SQLite，點擊即可從上次進度無縫續聊
  - **全域**：操控整台電腦的專屬 Tab，所有操作逐項確認後才執行
- **22 代理人驗證管線** — 模型的每個工具呼叫都經過確定性閘門交叉驗證（路徑圈禁、Shell 注入偵測、金鑰外洩掃描、AI 廢話審計……），一票否決
- **Claude 相容 Skills** — 在工作區放 `.claude/skills/<名稱>/SKILL.md`，對話輸入 `/名稱` 即可呼叫；`CLAUDE.md` 專案規則自動載入
- **Claude 相容 MCP** — 工作區根目錄放標準 `.mcp.json`，或在「設定 → MCP 伺服器」新增；已連線伺服器的工具清單自動曝露給模型
- **分層記憶** — 滑動視窗分塊 + 三階段漏斗 RAG（FTS5 索引），蒸餾水位記號避免重複燒 token
- **Token 經濟** — Session 級 token 預算硬鎖定，標題列即時顯示用量
- **沙盒對齊** — 寫入的 `.rs` 檔案立即編譯（含測試實際執行）；「嘴上說成功但編譯不過」當場退回

## 安裝與建置

前置需求：[Rust 工具鏈](https://rustup.rs/)（stable，2021 edition）。

```powershell
git clone https://github.com/masteryee-labs/Agnes-AI.git
cd Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# 啟動 GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

## 設定

所有本機設定都在倉庫根目錄的 `config.local.toml`（自動建立，**已加入 .gitignore**——你的 API 金鑰永遠不會進版本控制）。

最簡單的方式是用程式內設定頁（側邊欄的 ⚙）：

1. **設定 → API 與模型** — 貼上 API 金鑰按**儲存**。頁面會顯示已存金鑰的遮罩版（`sk-xx…xxxx`）、指紋，以及綠色「已儲存 ✓」——你隨時知道目前生效的是哪把金鑰。
2. **設定 → MCP 伺服器** — 按「＋ 新增伺服器」填名稱／指令／引數；伺服器立即啟動並寫入設定檔。
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

對話輸入 `/deploy …` 即可呼叫。技能與 `CLAUDE.md` 規則以確定性方式注入系統提示（不額外耗 API）。

## 使用方式

1. **建立專案** — 側邊欄 → 專案 Tab → **＋ 新增專案**，選一個資料夾。
2. **開始對話** — 輸入任務；新 Session 自動掛在目前專案底下並持久化。之後點側邊欄任一 Session 即可載入完整歷史、繼續工作。
3. **全域模式** — 切到**全域** Tab 即可在專案資料夾以外操作。每個動作都會出現在右側面板，逐項批准／拒絕。
4. **觀察代理人** — 右側面板顯示 22 個驗證代理人每一步的 PASS／REJECT 結果；待確認的工具呼叫也在這裡等你核准。

## 安全模型

- API 金鑰只存在 `config.local.toml`（git 忽略）；原始碼出現 `sk-` 字串＝一票否決
- 指令以引數向量執行——不做 shell 字串拼接
- 路徑圈禁：專案模式下，工作區以外的檔案操作一律拒絕
- 原始擷取 Exit Code 與 stderr；永不信任模型口頭宣稱的「成功」

## 開發文件

完整架構／記憶系統／QA／安全規格在 [`Docs/`](Docs/00_OVERVIEW.md)；代理人編排規則在 `.agent/rules/*.toon`。
