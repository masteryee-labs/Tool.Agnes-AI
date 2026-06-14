# Changelog

All notable changes to Agnes AI are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
