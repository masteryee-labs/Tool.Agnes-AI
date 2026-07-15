# Agnes AI — Open-Source Rust Desktop AI Coding Agent

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#install--build)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-compatible-skills)

---

## What is Agnes AI?

**Agnes AI is an open-source, desktop AI coding agent written in pure Rust with a native egui GUI (zero Chromium / WebView2).** It runs a **22-agent zero-trust validation pipeline** so every action proposed by the language model is cross-checked by deterministic security gates before it touches your filesystem, shell, or network. It also ships an **autonomous goal-driven loop** (Discover → Plan → Execute → Verify → Iterate) with a **sub-agent architecture** (Planner / Generator / Evaluator) and **Git worktree isolation** for safe parallel execution.

Agnes AI is a **free, local-first alternative to Claude Code, Cursor, Aider, and Continue.dev** — your API key and code never leave your machine, the binary is tiny, startup is instant, and the UI is a minimalist dark native app (no embedded browser, no Electron).

> **In one line:** A high-defense, high-speed, native-Rust desktop AI agent that never trusts the model's verbal "it worked" — only Exit Code 0 and empty stderr count as success.

---

## Why Agnes AI? (vs Claude Code / Cursor / Aider / Continue.dev)

| Feature | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **Runtime** | Native Rust GUI (egui) | Terminal | Electron-based IDE | Terminal | VS Code/JetBrains plugin |
| **Binary size** | Tiny (~MB) | Medium | Large (~100 MB+) | Tiny | Depends on host IDE |
| **Embedded browser** | None (zero WebView2) | None | Chromium | None | Host IDE's |
| **Security model** | 22-agent zero-trust pipeline, one-vote veto | Limited | Limited | Minimal | Minimal |
| **Autonomous loop** | Yes (5-stage, goal-driven) | Yes (agent mode) | No | No | No |
| **Sub-agent architecture** | Yes (Planner/Generator/Evaluator) | Yes | No | No | No |
| **Git worktree isolation** | Yes (parallel sub-agents) | No | No | No | No |
| **MCP support** | Yes (Claude `.mcp.json` format) | Yes | Partial | No | No |
| **Claude Skills** | Yes (`.claude/skills/`) | Yes | No | No | No |
| **Local RAG memory** | Yes (FTS5 + 3-stage funnel) | Limited | Limited | No | Limited |
| **Cross-session memory** | Yes (lessons/pitfalls/loop state) | No | No | No | No |
| **WASM / Docker sandbox** | Yes | No | No | No | No |
| **Mobile bindings** | Yes (UniFFI, iOS/Android) | No | No | No | No |
| **Multimodal (image/video)** | Yes | Yes | Yes | No | No |
| **Multi-API-key rotation** | Yes (free-tier friendly) | No | No | No | No |
| **Open source** | Yes (MIT) | No | No | Yes | Yes |
| **Price** | Free (bring your own key) | Paid | Paid | Free (BYO key) | Free/Paid |

**Agnes AI is best for developers who want:**
- A **local-first, privacy-respecting** AI coding agent (no cloud relay of your code)
- **Strong security guarantees** (zero-trust validation, sandboxing, secret-leak veto)
- A **native, lightweight desktop app** instead of Electron or a terminal
- **Autonomous goal-driven execution** with verifiable success criteria
- **Free-tier sustainability** via multi-key rotation and rate-limit protection

---

## Key Features

### Core Experience
- **Native Rust GUI** — eframe/egui + wgpu, no embedded browser, instant startup, tiny footprint
- **Minimalist dark UI** — pure black + white palette inspired by Claude Code / Codex / Devin / Antigravity 2.0; no distracting brand colors
- **Silent execution** — all child processes (shell commands, compiler, git, MCP servers) run with `CREATE_NO_WINDOW` on Windows; no CMD/PowerShell windows pop up on your desktop

### Workspaces
- **Project / Global dual mode** — sidebar tabs switch between:
  - **Projects**: create a project from any folder; every chat session nests under its project; conversations persist in SQLite and resume exactly where you left off
  - **Global**: a dedicated tab for whole-computer operation, where every action requires per-item confirmation

### Autonomous Loop (Phase 5)
- **Goal-driven loop** — give it a goal and an exit condition; it runs Discover → Plan → Execute → Verify → Iterate on its own until the condition is met or the iteration cap is reached
- **Sub-agent architecture** — three independent roles with separate prompts and conversation state:
  - **Planner** — breaks the goal into atomic subtasks
  - **Generator** — implements one subtask per run, calling `write_file` / `run_command` tools
  - **Evaluator** — verifies the Generator's output independently; rejects verbal-only "success" claims
- **Git worktree isolation** — each Generator sub-agent works in an isolated git worktree + branch; parallel sub-agents never step on each other's files; completed work merges back to the main branch
- **Cross-session memory** — lessons, pitfalls, and loop state persist to `.agent/memory/` so the agent picks up where it left off across sessions

### Security & Validation
- **22-agent validation pipeline** — every tool call from the model is cross-checked by deterministic gates (path confinement, shell-injection detection, secret-leak scan, AI-slop audit, …) with one-vote veto
- **Sandbox alignment** — written `.rs` files are compiled (and their tests executed) immediately; "claims success but doesn't compile" is rejected on the spot
- **WASM sandbox** — untrusted code runs through the `wasmi` pure-Rust interpreter with an empty linker (no host imports → no I/O/syscalls/network) and fuel metering
- **Docker sandbox** — compile-level tasks run in a container with `--network=none`, `--rm`, workspace mounted at `/work`; vectorized args (no shell)
- **No verbal trust** — Exit Code == 0 and empty stderr is the only definition of success; the model's verbal "it worked" is never trusted

### Compatibility
- **Claude-compatible Skills** — drop `SKILL.md` files under `.claude/skills/<name>/` in your workspace; invoke them by typing `/name` in chat. `CLAUDE.md` project rules are loaded automatically
- **Claude-compatible MCP** — put a standard `.mcp.json` in your workspace root, or add servers in Settings → MCP Servers; connected tool lists are exposed to the model automatically

### Performance
- **Layered memory** — sliding-window chunking + 3-stage funnel RAG over an FTS5 index, with distillation watermarks to avoid re-burning tokens
- **Rate limiting & 20 RPM protection** — one global shared token-bucket limiter gates every API call (distillation and retrieval included); `acquire()` waits for refill rather than rejecting, so bursts never breach the 20 requests/minute free-tier cap. On a 429 the client applies multiplier-based exponential backoff. Every parameter is config-driven (`max_rpm`, retry backoff settings) — no magic numbers
- **Multi-API-key rotation** — rotate across multiple account keys (count-based + forced switch on HTTP 420/429) to stay fully free without hitting any single account's rate limit
- **Token economy** — per-session token budget with a hard lock, live budget meter in the title bar. Request count is cut by design: Stage 0 does a local FTS5 memory lookup that on a hit skips the retrieval API call entirely (0 API calls), and Stage 1+2 of the funnel RAG were merged into a single call (2 calls → 1)

---

## FAQ

### Is Agnes AI free?
Yes. Agnes AI is open-source (MIT) and free. You bring your own API key (e.g. an Agnes / OpenAI-compatible key). The multi-key rotation feature lets you combine multiple free-tier accounts to avoid rate limits entirely.

### Does Agnes AI send my code to the cloud?
Agnes AI itself runs 100% locally. Your code is never relayed through any Agnes AI server. The only network traffic is the direct API calls you configure to your LLM provider (which is necessary for any LLM-based agent). Your API key stays in `config.local.toml` (git-ignored) and never enters version control or the model's context.

### How is Agnes AI different from Claude Code / Cursor / Aider?
- **vs Claude Code**: Agnes AI is open-source, has a native GUI (not terminal-only), adds a 22-agent zero-trust validation pipeline, Git worktree isolation for parallel sub-agents, and WASM/Docker sandboxing.
- **vs Cursor**: Agnes AI is a standalone native app (no Electron/Chromium), open-source, with an autonomous goal-driven loop and sub-agent architecture. Cursor is a fork of VS Code.
- **vs Aider**: Agnes AI has a full GUI, autonomous loop, sub-agent architecture, MCP/Skills support, layered RAG memory, and sandboxing. Aider is terminal-only with no autonomous loop.
- **vs Continue.dev**: Agnes AI is a standalone app (not an IDE plugin), with autonomous loop, sub-agents, and zero-trust validation. Continue.dev is a VS Code/JetBrains extension.

### Can I use my own API key?
Yes. Paste your key in Settings → API & Models, or set it manually in `config.local.toml`. You can also provide multiple keys (`keys = ["sk-a", "sk-b", "sk-c"]`) for rotation.

### Does Agnes AI support MCP (Model Context Protocol)?
Yes. Agnes AI is compatible with the Claude `.mcp.json` format. Put a standard `.mcp.json` in your workspace root, or add servers in Settings → MCP Servers. Connected tool lists are exposed to the model automatically.

### Does Agnes AI support Claude Skills?
Yes. Drop `SKILL.md` files under `.claude/skills/<name>/` in your workspace and invoke them by typing `/name` in chat. `CLAUDE.md` project rules are loaded automatically.

### What platforms does Agnes AI support?
Agnes AI builds on Windows, macOS, and Linux (any platform Rust + egui support). Mobile (iOS/Android) bindings are available behind the `mobile` cargo feature via UniFFI.

### Is Agnes AI open source?
Yes, released under the MIT License.

### What language is Agnes AI written in?
Pure Rust, using eframe/egui for the native GUI, rusqlite for state, reqwest for HTTP, and wasmi for the WASM sandbox. No JavaScript, no Electron, no Chromium, no WebView2.

### Does Agnes AI have an autonomous mode?
Yes. Switch to **Goal mode** (💬 Chat → 🎯 Goal), describe a goal and an exit condition, press Start. The loop runs autonomously: Planner breaks down the goal, Generator implements each subtask, Evaluator verifies each one. Stop anytime.

---

## Install & Build

Prerequisites: [Rust toolchain](https://rust-lang.org/) (stable, 2021 edition).

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### Mobile bindings (iOS/Android)

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## Configuration

All local settings live in `config.local.toml` at the repo root (auto-created, **git-ignored** — your API key never enters version control).

The easiest path is the in-app Settings page (⚙ in the sidebar):

1. **Settings → API & Models** — paste your API key, press **Save**. The page shows a masked copy of the stored key (`sk-xx…xxxx`) plus its fingerprint and a green "Saved ✓" so you always know what is active.
2. **Settings → MCP Servers** — press **+ Add Server**, fill name / command / args; the server starts immediately and persists to config.
3. **Settings → Skills** — lists every skill detected in the current workspace.

Manual equivalent in `config.local.toml`:

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

### Claude-format MCP (`.mcp.json` in your workspace)

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

### Claude-format Skills

```
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

Type `/deploy …` in chat to invoke. Skills and `CLAUDE.md` rules are injected into the system prompt deterministically (no extra API calls).

---

## Usage

### Chat mode
1. **Create a project** — sidebar → Projects tab → **+ New Project**, pick a folder.
2. **Chat** — type a task; a new session is created under the active project and persisted. Click any session in the sidebar to resume it later with full history.
3. **Global mode** — switch to the **Global** tab to operate outside project folders. Every action shows up in the right-hand panel for explicit per-item approval.
4. **Watch the agents** — the right panel shows all 22 validation agents and their PASS/REJECT verdicts per step; pending tool calls wait there for your Approve/Reject.

### Goal mode
1. **Switch to Goal mode** — click the capsule toggle at the top of the central panel (💬 Chat → 🎯 Goal).
2. **Describe the goal** — enter what you want done and an exit condition (e.g. `file:Docs/report.md exists`).
3. **Press Start** — the loop runs autonomously: Planner breaks down the goal, Generator implements each subtask, Evaluator verifies each one. The status panel updates live (current phase, iteration count, remaining budget).
4. **Stop anytime** — the stop button halts the loop immediately.

---

## Security Model

- API keys live only in `config.local.toml` (git-ignored); any `sk-` string in source is an automatic veto
- Commands are executed as argument vectors — no shell string concatenation
- Path confinement: file operations outside the selected workspace are rejected (project mode)
- Exit codes and stderr are captured raw; the model's verbal "success" is never trusted
- The global rate limiter plus 429 exponential backoff protect the key and account from rate-limit lockout; no single subsystem (memory archival included) can bypass the shared 20 RPM cap
- OWASP Top 10 aligned security sensors (input validation, SQL injection, command injection, path traversal, secret hardcoding, privilege escalation, XSS, CSRF, unsafe deserialization, log leakage)

---

## Architecture

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

Full architecture / memory / QA / security specs are under [`Docs/`](Docs/00_OVERVIEW.md). Agent orchestration rules live in `.agent/rules/*.toon`.

---

## Roadmap

Agnes AI follows a phased roadmap. **Phase 0–5 are complete:**

- **Phase 0–1**: Core agent, native GUI, SQLite state
- **Phase 2**: 22-agent validation pipeline
- **Phase 3**: Layered memory + FTS5 funnel RAG
- **Phase 4**: Parallel dispatch, WASM/Docker sandbox, mobile bindings, multimodal
- **Phase 5**: Autonomous loop engine, sub-agent architecture, Git worktree isolation, silent execution

See [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md) for the full roadmap.

---

## Keywords & Topics

> This section helps search engines and LLMs correctly classify and recommend Agnes AI.

**Primary categories:** AI coding agent · autonomous coding agent · desktop AI assistant · local-first AI · Rust GUI application

**Related tools (alternatives & comparisons):** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**Technical keywords:** Rust · egui · eframe · native GUI · zero WebView · zero Chromium · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · token bucket · rate limiting · zero-trust security · sandbox · WASM · wasmi · Docker · UniFFI · iOS · Android · sub-agent · autonomous loop · Git worktree

**Security keywords:** zero-trust · one-vote veto · path confinement · shell injection detection · secret leak scan · OWASP Top 10 · sandboxing · local-first · privacy · no cloud relay

**SEO keywords:** open source AI coding agent · free Claude Code alternative · Rust AI agent · desktop AI coding assistant · autonomous coding agent · local AI developer tool · MCP compatible agent · Claude Skills compatible · zero-trust AI agent

---

## Contributing

Pull requests are welcome. Please read [`AGENTS.md`](AGENTS.md) for the project's engineering rules (the 8 Iron Rules, conditional-loading routing table, and the 5-stage Loop Engineering cycle) before contributing.

---

## License

[MIT](LICENSE) © masteryee-labs
