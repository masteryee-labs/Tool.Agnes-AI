# Agnes AI — 开源 Rust 桌面端 AI 编程代理

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#安装与构建)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-compatible-skills)

---

## Agnes AI 是什么？

**Agnes AI 是一个开源的桌面端 AI 编程代理，使用纯 Rust 编写，配备原生 egui GUI（零 Chromium / WebView2）。** 它运行一条 **22 代理零信任验证管线**，因此语言模型提出的每一个动作在触及你的文件系统、Shell 或网络之前，都会被确定性安全闸门交叉校验。它还内置了 **自主目标驱动循环**（Discover → Plan → Execute → Verify → Iterate），采用 **子代理架构**（Planner / Generator / Evaluator）和 **Git worktree 隔离**，以实现安全的并行执行。

Agnes AI 是 **Claude Code、Cursor、Aider 和 Continue.dev 的免费、本地优先替代方案** —— 你的 API 密钥和代码永远不会离开你的机器，二进制文件极小，启动瞬间完成，UI 是极简的暗色原生应用（无内嵌浏览器，无 Electron）。

> **一句话概括：** 一个高防御、高速、原生 Rust 的桌面端 AI 代理，从不信任模型的口头"成功了" —— 只有 Exit Code 0 和空的 stderr 才算成功。

---

## 为什么选择 Agnes AI？（vs Claude Code / Cursor / Aider / Continue.dev）

| 特性 | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **运行时** | 原生 Rust GUI（egui） | 终端 | 基于 Electron 的 IDE | 终端 | VS Code/JetBrains 插件 |
| **二进制体积** | 极小（~MB） | 中等 | 庞大（~100 MB+） | 极小 | 取决于宿主 IDE |
| **内嵌浏览器** | 无（零 WebView2） | 无 | Chromium | 无 | 宿主 IDE 的 |
| **安全模型** | 22 代理零信任管线，一票否决 | 有限 | 有限 | 最小 | 最小 |
| **自主循环** | 是（5 阶段，目标驱动） | 是（agent 模式） | 否 | 否 | 否 |
| **子代理架构** | 是（Planner/Generator/Evaluator） | 是 | 否 | 否 | 否 |
| **Git worktree 隔离** | 是（并行子代理） | 否 | 否 | 否 | 否 |
| **MCP 支持** | 是（Claude `.mcp.json` 格式） | 是 | 部分 | 否 | 否 |
| **Claude Skills** | 是（`.claude/skills/`） | 是 | 否 | 否 | 否 |
| **本地 RAG 记忆** | 是（FTS5 + 3 阶段漏斗） | 有限 | 有限 | 否 | 有限 |
| **跨会话记忆** | 是（教训/陷阱/循环状态） | 否 | 否 | 否 | 否 |
| **WASM / Docker 沙盒** | 是 | 否 | 否 | 否 | 否 |
| **移动端绑定** | 是（UniFFI，iOS/Android） | 否 | 否 | 否 | 否 |
| **多模态（图像/视频）** | 是 | 是 | 是 | 否 | 否 |
| **多 API 密钥轮换** | 是（免费层友好） | 否 | 否 | 否 | 否 |
| **开源** | 是（MIT） | 否 | 否 | 是 | 是 |
| **价格** | 免费（自带密钥） | 付费 | 付费 | 免费（自带密钥） | 免费/付费 |

**Agnes AI 最适合以下开发者：**
- 想要 **本地优先、尊重隐私** 的 AI 编程代理（代码不经云端中转）
- **强安全保证**（零信任验证、沙盒化、密钥泄露一票否决）
- **原生、轻量的桌面应用**，而非 Electron 或终端
- **自主目标驱动执行**，附带可验证的成功标准
- 通过多密钥轮换和速率限制保护实现 **免费层可持续性**

---

## 核心特性

### 核心体验
- **原生 Rust GUI** —— eframe/egui + wgpu，无内嵌浏览器，启动瞬间，占用极小
- **极简暗色 UI** —— 纯黑 + 白色调色板，灵感来自 Claude Code / Codex / Devin / Antigravity 2.0；无干扰性品牌色
- **静默执行** —— 所有子进程（Shell 命令、编译器、git、MCP 服务器）在 Windows 上以 `CREATE_NO_WINDOW` 运行；桌面上不会弹出 CMD/PowerShell 窗口

### 工作区
- **项目 / 全局双模式** —— 侧边栏标签页切换：
  - **项目**：从任意文件夹创建项目；每个聊天会话嵌套在其项目下；对话持久化于 SQLite，并在你离开处精确恢复
  - **全局**：专用于全机操作的标签页，其中每个操作都需要逐项确认

### 自主循环（Phase 5）
- **目标驱动循环** —— 给它一个目标和一个退出条件；它会自主运行 Discover → Plan → Execute → Verify → Iterate，直到条件满足或达到迭代上限
- **子代理架构** —— 三个独立角色，各自拥有独立的提示词和对话状态：
  - **Planner** —— 将目标拆解为原子子任务
  - **Generator** —— 每次运行实现一个子任务，调用 `write_file` / `run_command` 工具
  - **Evaluator** —— 独立验证 Generator 的输出；拒绝纯口头的"成功"声明
- **Git worktree 隔离** —— 每个 Generator 子代理在隔离的 git worktree + 分支中工作；并行子代理互不踩踏文件；完成的工作合并回主分支
- **跨会话记忆** —— 教训、陷阱和循环状态持久化到 `.agent/memory/`，使代理能跨会话从上次中断处继续

### 安全与验证
- **22 代理验证管线** —— 模型的每个工具调用都由确定性闸门交叉校验（路径限制、Shell 注入检测、密钥泄露扫描、AI 垃圾审计等），一票否决
- **沙盒对齐** —— 写入的 `.rs` 文件会立即编译（并执行其测试）；"声称成功但无法编译"会被当场拒绝
- **WASM 沙盒** —— 不受信任的代码通过 `wasmi` 纯 Rust 解释器运行，使用空链接器（无 host imports → 无 I/O/系统调用/网络）和燃料计量
- **Docker 沙盒** —— 编译级任务在容器中运行，带 `--network=none`、`--rm`、工作区挂载到 `/work`；使用向量化参数（无 Shell）
- **不信任口头报告** —— Exit Code == 0 且 stderr 为空是成功的唯一定义；模型的口头"成功了"从不被信任

### 兼容性
- **Claude 兼容 Skills** —— 在工作区的 `.claude/skills/<name>/` 下放置 `SKILL.md` 文件；在聊天中输入 `/name` 即可调用。`CLAUDE.md` 项目规则会自动加载
- **Claude 兼容 MCP** —— 在工作区根目录放置标准 `.mcp.json`，或在 Settings → MCP Servers 中添加服务器；已连接的工具列表会自动暴露给模型

### 性能
- **分层记忆** —— 滑动窗口分块 + 基于 FTS5 索引的 3 阶段漏斗 RAG，带有蒸馏水印以避免重复消耗 token
- **速率限制与 20 RPM 保护** —— 一个全局共享的令牌桶限流器为每个 API 调用（包括蒸馏和检索）把关；`acquire()` 等待补充而非拒绝，因此突发请求永远不会突破 20 请求/分钟的免费层上限。遇到 429 时，客户端应用基于乘数的指数退避。每个参数都由配置驱动（`max_rpm`、重试退避设置）—— 无魔法数字
- **多 API 密钥轮换** —— 在多个账户密钥间轮换（基于计数 + HTTP 420/429 时强制切换），以保持完全免费而不触及任何单一账户的速率限制
- **Token 经济** —— 每会话 token 预算带硬锁，标题栏有实时预算表。请求数按设计削减：Stage 0 执行本地 FTS5 记忆查找，命中时完全跳过检索 API 调用（0 次 API 调用），漏斗 RAG 的 Stage 1+2 合并为单次调用（2 次调用 → 1 次）

---

## 常见问题

### Agnes AI 免费吗？
是的。Agnes AI 是开源的（MIT）且免费。你自带 API 密钥（例如 Agnes / OpenAI 兼容密钥）。多密钥轮换功能可让你组合多个免费层账户，完全避免速率限制。

### Agnes AI 会把我的代码发送到云端吗？
Agnes AI 本身 100% 在本地运行。你的代码永远不会经任何 Agnes AI 服务器中转。唯一的网络流量是你配置的、发往 LLM 提供商的直接 API 调用（这对任何基于 LLM 的代理都是必要的）。你的 API 密钥保存在 `config.local.toml`（已 git 忽略）中，永远不会进入版本控制或模型的上下文。

### Agnes AI 与 Claude Code / Cursor / Aider 有何不同？
- **vs Claude Code**：Agnes AI 是开源的，拥有原生 GUI（非仅终端），增加了 22 代理零信任验证管线、用于并行子代理的 Git worktree 隔离，以及 WASM/Docker 沙盒。
- **vs Cursor**：Agnes AI 是独立的原生应用（无 Electron/Chromium），开源，带有自主目标驱动循环和子代理架构。Cursor 是 VS Code 的分支。
- **vs Aider**：Agnes AI 拥有完整 GUI、自主循环、子代理架构、MCP/Skills 支持、分层 RAG 记忆和沙盒。Aider 仅限终端，无自主循环。
- **vs Continue.dev**：Agnes AI 是独立应用（非 IDE 插件），带有自主循环、子代理和零信任验证。Continue.dev 是 VS Code/JetBrains 扩展。

### 我可以使用自己的 API 密钥吗？
可以。在 Settings → API & Models 中粘贴你的密钥，或在 `config.local.toml` 中手动设置。你也可以提供多个密钥（`keys = ["sk-a", "sk-b", "sk-c"]`）进行轮换。

### Agnes AI 支持 MCP（Model Context Protocol）吗？
支持。Agnes AI 兼容 Claude 的 `.mcp.json` 格式。在工作区根目录放置标准 `.mcp.json`，或在 Settings → MCP Servers 中添加服务器。已连接的工具列表会自动暴露给模型。

### Agnes AI 支持 Claude Skills 吗？
支持。在工作区的 `.claude/skills/<name>/` 下放置 `SKILL.md` 文件，在聊天中输入 `/name` 即可调用。`CLAUDE.md` 项目规则会自动加载。

### Agnes AI 支持哪些平台？
Agnes AI 可在 Windows、macOS 和 Linux 上构建（任何 Rust + egui 支持的平台）。移动端（iOS/Android）绑定通过 `mobile` cargo feature 经由 UniFFI 提供。

### Agnes AI 是开源的吗？
是的，以 MIT 许可证发布。

### Agnes AI 用什么语言编写？
纯 Rust，使用 eframe/egui 作为原生 GUI，rusqlite 作为状态存储，reqwest 作为 HTTP 客户端，wasmi 作为 WASM 沙盒。无 JavaScript，无 Electron，无 Chromium，无 WebView2。

### Agnes AI 有自主模式吗？
有。切换到 **Goal 模式**（💬 Chat → 🎯 Goal），描述一个目标和一个退出条件，按 Start。循环自主运行：Planner 拆解目标，Generator 实现每个子任务，Evaluator 验证每一个。可随时停止。

---

## 安装与构建

前置条件：[Rust 工具链](https://rust-lang.org/)（stable，2021 edition）。

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### 移动端绑定（iOS/Android）

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## 配置

所有本地设置存放在仓库根目录的 `config.local.toml` 中（自动创建，**已 git 忽略** —— 你的 API 密钥永远不会进入版本控制）。

最简单的方式是使用应用内的 Settings 页面（侧边栏的 ⚙）：

1. **Settings → API & Models** —— 粘贴你的 API 密钥，按 **Save**。页面会显示已存储密钥的掩码副本（`sk-xx…xxxx`）及其指纹，以及绿色的"Saved ✓"，让你随时知道当前激活的是什么。
2. **Settings → MCP Servers** —— 按 **+ Add Server**，填写名称 / 命令 / 参数；服务器立即启动并持久化到配置。
3. **Settings → Skills** —— 列出当前工作区中检测到的所有 skill。

在 `config.local.toml` 中的手动等价写法：

```toml
[api]
key = "{{API_KEY}}"
# Optional: multiple keys for rotation (free-tier friendly)
keys = ["sk-a", "sk-b", "sk-c"]
key_rotation_every = 15
model = "agnes-2.0-flash"
session_budget = 500000

[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"]
```

### Claude 格式 MCP（工作区中的 `.mcp.json`）

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
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

在聊天中输入 `/deploy …` 即可调用。Skills 和 `CLAUDE.md` 规则以确定性方式注入系统提示词（无额外 API 调用）。

---

## 使用方式

### Chat 模式
1. **创建项目** —— 侧边栏 → Projects 标签页 → **+ New Project**，选择一个文件夹。
2. **聊天** —— 输入任务；会在当前项目下创建新会话并持久化。点击侧边栏中的任意会话即可稍后恢复，完整历史保留。
3. **全局模式** —— 切换到 **Global** 标签页以在项目文件夹之外操作。每个操作都会显示在右侧面板中，供你逐项明确批准。
4. **观察代理** —— 右侧面板显示全部 22 个验证代理及其每一步的 PASS/REJECT 判定；待处理的工具调用在此等待你的 Approve/Reject。

### Goal 模式
1. **切换到 Goal 模式** —— 点击中央面板顶部的胶囊切换器（💬 Chat → 🎯 Goal）。
2. **描述目标** —— 输入你想要完成的内容和一个退出条件（例如 `file:Docs/report.md exists`）。
3. **按 Start** —— 循环自主运行：Planner 拆解目标，Generator 实现每个子任务，Evaluator 验证每一个。状态面板实时更新（当前阶段、迭代次数、剩余预算）。
4. **随时停止** —— 停止按钮立即中止循环。

---

## 安全模型

- API 密钥仅存在于 `config.local.toml`（已 git 忽略）；源码中出现任何 `sk-` 字符串即自动否决
- 命令以参数向量执行 —— 无 Shell 字符串拼接
- 路径限制：所选工作区之外的文件操作被拒绝（项目模式）
- Exit code 和 stderr 以原始形式捕获；模型的口头"成功"从不被信任
- 全局速率限制器加 429 指数退避保护密钥和账户免受速率限制锁定；无任何单一子系统（包括记忆归档）可绕过共享的 20 RPM 上限
- 对齐 OWASP Top 10 的安全传感器（输入验证、SQL 注入、命令注入、路径穿越、密钥硬编码、权限提升、XSS、CSRF、不安全反序列化、日志泄露）

---

## 架构

```
src-tauri/src/
├── main.rs / lib.rs        # Entry point + eframe app
├── agent.rs                # Core agent loop
├── orchestrator.rs         # 22-agent validation pipeline
├── validation.rs           # Deterministic security gates
├── sandbox.rs              # WASM + Docker sandbox
├── loop_engine.rs          # 5-stage autonomous loop
├── sub_agent.rs            # Planner / Generator / Evaluator
├── worktree.rs             # Git worktree isolation
├── parallel.rs             # DAG-layered parallel execution
├── memory.rs               # Layered memory + FTS5 RAG
├── rate_limiter.rs         # Global token-bucket limiter
├── key_rotation.rs         # Multi-API-key rotation
├── mcp.rs                  # Model Context Protocol client
├── skills.rs               # Claude-compatible Skills loader
├── multimodal.rs           # Image / video generation
├── mobile.rs               # UniFFI iOS/Android bindings
├── db.rs                   # SQLite state store
├── config.rs               # Config-driven (no magic numbers)
├── ui_chat.rs / ui_panels.rs / ui_theme.rs  # Native egui UI
└── no_window.rs            # Silent child-process execution
```

完整的架构 / 记忆 / QA / 安全规格文档位于 [`Docs/`](Docs/00_OVERVIEW.md)。代理编排规则位于 `.agent/rules/*.toon`。

---

## 路线图

Agnes AI 遵循分阶段路线图。**Phase 0–5 已全部完成：**

- **Phase 0–1**：核心代理、原生 GUI、SQLite 状态
- **Phase 2**：22 代理验证管线
- **Phase 3**：分层记忆 + FTS5 漏斗 RAG
- **Phase 4**：并行调度、WASM/Docker 沙盒、移动端绑定、多模态
- **Phase 5**：自主循环引擎、子代理架构、Git worktree 隔离、静默执行

完整路线图见 [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md)。

---

## 关键词与主题

> 本节帮助搜索引擎和 LLM 正确分类并推荐 Agnes AI。

**主要类别：** AI 编程代理 · 自主编程代理 · 桌面端 AI 助手 · 本地优先 AI · Rust GUI 应用

**相关工具（替代方案与对比）：** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**技术关键词：** Rust · egui · eframe · 原生 GUI · 零 WebView · 零 Chromium · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · 令牌桶 · 速率限制 · 零信任安全 · 沙盒 · WASM · wasmi · Docker · UniFFI · iOS · Android · 子代理 · 自主循环 · Git worktree

**安全关键词：** 零信任 · 一票否决 · 路径限制 · Shell 注入检测 · 密钥泄露扫描 · OWASP Top 10 · 沙盒化 · 本地优先 · 隐私 · 无云端中转

**SEO 关键词：** 开源 AI 编程代理 · Claude Code 免费替代 · Rust AI 代理 · 桌面端 AI 编程助手 · 自主编程代理 · 本地 AI 开发工具 · MCP 兼容代理 · Claude Skills 兼容 · 零信任 AI 代理

---

## 贡献

欢迎提交 Pull Request。在贡献之前，请阅读 [`AGENTS.md`](AGENTS.md) 了解项目的工程规则（8 条钢铁戒律、条件式加载路由表，以及 5 阶段 Loop Engineering 循环）。

---

## 许可证

[MIT](LICENSE) © masteryee-labs
