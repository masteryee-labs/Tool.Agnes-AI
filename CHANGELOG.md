# Changelog

All notable changes to Agnes AI are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.4] - 2026-07-07

Theme: multi-account API key rotation to keep Agnes AI fully free without hitting any single account's 20 RPM rate limit.

### Added
- **Multi-API-Key rotation** (`src-tauri/src/key_rotation.rs`): `KeyRotator` rotates across multiple account keys — count-based rotation (after `key_rotation_every` consecutive calls, defaults to 15) plus forced key switch on HTTP 420/429 (jumps to the next key immediately instead of waiting out the backoff). Single-key mode degrades to the legacy behavior (backward compatible).
- **Config**: `[api] keys = ["sk-a", "sk-b", "sk-c"]` (key group, takes precedence over `key` when non-empty) + `key_rotation_every` (rotation threshold, 0 = default 15). `ApiConfig::all_keys()` / `has_key()` / `build_rotator()` helpers.
- **App-global shared rotator**: `AppState.key_rotator` is a single shared rotator for all agents, multi-folder parallel loops, sub-agents (Planner/Generator/Evaluator), and multimodal — load is spread evenly across all accounts.
- **GUI**: the API key input now accepts one key (`sk-…`) or multiple separated by comma/newline (`sk-a, sk-b, sk-c`) → auto-built into the `keys` group; multi-key state shows "金鑰組：N 組（指紋 a, b, c）"; new "金鑰輪詢間隔" DragValue.
- **Wiring**: `AgentLoop::with_rate_limiter_and_rotator`, `MultimodalManager::new(cfg, rotator)`, `MemoryManager::{llm_call, stage12_merged, distill_text}` all take the shared rotator; `send_api_request` calls `mark_rate_limited()` on 429/420.

### Security
- Multiple keys are still stored only in `config.local.toml` (gitignored); UI/logs show per-key SHA-256 fingerprints, never raw keys; keys never enter model context. Verified: `config.local.toml` is untracked, not staged, and matched by `.gitignore:7`.

### Gates
- `cargo check --all-targets` clean; `cargo clippy -D warnings` clean; `cargo test --lib` green (147 tests, including 7 new KeyRotator tests).

## [0.8.1] - 2026-06-14

Theme: wire the parallel + multimodal capabilities into the live GUI, and add a red-team gate suite.

### Added
- **App-global shared rate limiter**: `AppState` now owns one `Arc<RateLimiter>`; `AgentLoop::with_rate_limiter` lets every agent (and the multimodal client) share a single 20 RPM bucket, so concurrent multi-folder sends plus media generation cannot collectively exceed the cap.
- **Multi-folder parallel in the live flow**: in `handle_send`, additional selected project folders run their agent step concurrently (`futures::join_all`, shared limiter); each folder's response is appended to the chat, labeled by folder.
- **Multimodal in the live flow**: a visual-intent prompt (`is_visual_intent`) fires `MultimodalManager::generate_image` after the agent step; the resulting URL or a graceful error is appended to the chat. Shares the global limiter.
- **Red-team gate suite** (`src-tauri/tests/red_team.rs`, 17 tests): path traversal, shell injection, forbidden programs, indirect shells, command substitution, hardcoded secrets, destructive commands, and WASM host-import/garbage isolation — all asserted blocked (0 penetration). Malicious commands are intercepted at the sandbox entry (exit-code alignment marks them failed).

### Fixed
- **Multimodal endpoints confirmed against the live API and corrected.** Image: `POST /v1/images/generations` returns a generated image URL (`data[0].url`) in ~40–50s. Video: `POST /v1/video/generations` (Agnes is asymmetric — plural `images`, singular `video`; the previous `/v1/videos/generations` default was a 404). Because generation is slow, the multimodal client now uses a dedicated long timeout (`[multimodal] timeout_seconds`, default 180s) instead of the 30s text/tool timeout that caused the earlier "error sending request" failure.

### Gates
- `cargo clippy -D warnings` clean (default + `--features mobile`); `cargo test` green (123 + 49 + 17 + 2). Real-machine GUI verified: main view renders; a visual-intent send (「畫一隻戴帽子的柴犬插圖」) returns a real Agnes-generated image URL in the chat (~41s).

## [0.8.0] - 2026-06-14

Theme: close the remaining roadmap phases — parallel dispatch, hardened sandboxes, mobile bindings, multimodal.

### Added
- **DAG-layered parallel execution** (`src-tauri/src/parallel.rs`): `compute_dag_layers` (Kahn topological layering with cycle detection) and `run_layers_parallel` (same-layer concurrency via tokio `JoinSet`, deterministic index-ordered output). `Orchestrator::execute_multi_folder_parallel` runs independent folder builds concurrently, each with its own SQLite connection.
- **WASM sandbox** (`sandbox::run_wasm_func`): executes untrusted WASM through the `wasmi` pure-Rust interpreter with an empty `Linker` (no host imports → no I/O/syscalls/network) and fuel metering (bounds infinite loops). `SandboxConfig.wasm_fuel` is config-driven.
- **Docker sandbox** (`sandbox::run_in_docker_sandbox`): runs compile-level tasks in a container with `--network=none`, `--rm`, workspace mounted at `/work`; vectorized args (no shell). Detects a missing docker CLI and degrades gracefully. `SandboxConfig.docker_image` / `docker_network` / `docker_enabled` are config-driven.
- **UniFFI mobile bindings** (`src-tauri/src/mobile.rs` + `src/agnes.udl`), behind the `mobile` cargo feature: exports `agnes_version`, `agnes_default_config`, `agnes_is_visual_intent`, `agnes_estimate_tokens` for iOS/Android shells via `uniffi-bindgen`. The default desktop build is unaffected.
- **Multimodal media** (`src-tauri/src/multimodal.rs`): `MultimodalManager` clients for Agnes Image 2.1 Flash / Agnes-Video-V2.0 with deterministic `is_visual_intent` activation; every media call passes through the shared rate limiter (counts toward 20 RPM). Endpoints/models in `MultimodalConfig`.

### Changed
- `Orchestrator::dispatch_subagents` rewritten to call validation once and order results by DAG layer, removing the previous O(n²) loop that re-ran the full batch up to 50 times. Output set and verdicts are unchanged.

### Engineering notes
- WASM uses `wasmi` (interpreter) rather than `wasmtime` (JIT): no JIT attack surface, no system dependencies, fast compile; empty linker + fuel already give full isolation for untrusted snippets.
- Gates: `cargo clippy -D warnings` clean (default and `--features mobile`); `cargo test` green (123 + 49 + 2).

## [0.7.0] - 2026-06-14

Theme: strengthen security and cut Agnes API request count.

### Added
- Global token-bucket rate limiter (`src-tauri/src/rate_limiter.rs`): one shared limiter gates every Agnes API call site in the app, including the memory distillation pipeline (Alpha/Beta/Integrator, up to 3 calls) and the retrieval call. `acquire()` waits for refill instead of rejecting, so bursts stay under the 20 requests/minute free-tier cap. `max_rpm = 0` disables the cap for testing.
- 429 exponential backoff in the API client: on HTTP 429 the client waits, then retries with multiplier-based backoff, capped, for a configurable number of attempts. All parameters live in `config.rs` `ApiConfig` (`retry_initial_backoff_secs`, `retry_max_backoff_secs`, `retry_max_attempts`, `retry_backoff_multiplier`, `max_rpm`) — no magic numbers.
- Per-session `TokenBudgeter` with a hard lock: once the session budget is exhausted only deterministic (non-API) operations may continue; a live budget meter shows in the title bar.
- QA regression corpus under `src-tauri/tests/fixtures/qa_corpus/` replayed by `cargo test qa_replay`, plus end-to-end tests in `src-tauri/tests/e2e_tests.rs`.

### Changed
- Stage 0 now performs a local SQLite FTS5 memory lookup; on a hit it skips the retrieval API call entirely (0 requests).
- Stage 1 and Stage 2 of the funnel RAG were merged into a single API call (2 calls → 1), saving one request per task.
- All 22 validation agents are now implemented and routed (was 17/22).

### Security
- The global rate limiter plus 429 backoff protect the key and account from rate-limit lockout. No single subsystem — memory archival included — can bypass the shared 20 RPM cap.

## [0.6.0] - 2026-06

Claude Desktop-style UI/UX: collapsible Think blocks, activity cards, right-hand change panel.

## [0.5.0] - 2026-06

22-agent routing: division-of-labor routing algorithm, panel follows session, interface zoom.
