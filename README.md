# Agnes AI — Multi-Agent Security Engine

> 繁體中文說明請見 [README.zh-TW.md](README.zh-TW.md)

Agnes AI is a high-defense, high-speed desktop AI agent built with **pure Rust + egui native GUI** (zero Chromium / WebView). It runs a 22-agent orchestration pipeline with a zero-trust security model: every model-proposed action passes deterministic validation gates before it touches your system.

## Features

### Core

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
- **No verbal trust** — Exit Code == 0 and empty stderr is the only definition of success; the model's verbal "it worked" is never trusted

### Compatibility

- **Claude-compatible Skills** — drop `SKILL.md` files under `.claude/skills/<name>/` in your workspace; invoke them by typing `/name` in chat. `CLAUDE.md` project rules are loaded automatically
- **Claude-compatible MCP** — put a standard `.mcp.json` in your workspace root, or add servers in Settings → MCP Servers; connected tool lists are exposed to the model automatically

### Performance

- **Layered memory** — sliding-window chunking + 3-stage funnel RAG over an FTS5 index, with distillation watermarks to avoid re-burning tokens
- **Rate limiting & 20 RPM protection** — one global shared token-bucket limiter gates every Agnes API call (distillation and retrieval included); `acquire()` waits for refill rather than rejecting, so bursts never breach the 20 requests/minute free-tier cap. On a 429 the client applies multiplier-based exponential backoff. Every parameter is config-driven (`max_rpm`, retry backoff settings) — no magic numbers
- **Token economy** — per-session token budget with a hard lock, live budget meter in the title bar. Request count is cut by design: Stage 0 does a local FTS5 memory lookup that on a hit skips the retrieval API call entirely (0 API calls), and Stage 1+2 of the funnel RAG were merged into a single call (2 calls → 1)

## Install & Build

Prerequisites: [Rust toolchain](https://rust-lang.org/) (stable, 2021 edition).

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

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

## Security Model

- API keys live only in `config.local.toml` (git-ignored); any `sk-` string in source is an automatic veto
- Commands are executed as argument vectors — no shell string concatenation
- Path confinement: file operations outside the selected workspace are rejected (project mode)
- Exit codes and stderr are captured raw; the model's verbal "success" is never trusted
- The global rate limiter plus 429 exponential backoff protect the key and account from rate-limit lockout; no single subsystem (memory archival included) can bypass the shared 20 RPM cap

## Development docs

Full architecture / memory / QA / security specs are under [`Docs/`](Docs/00_OVERVIEW.md). Agent orchestration rules live in `.agent/rules/*.toon`.

## License

MIT
