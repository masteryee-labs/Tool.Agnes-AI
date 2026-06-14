# E2E Test Suite Infrastructure & Methodology

This document outlines the End-to-End (E2E) testing framework, testing philosophy, feature inventory, architecture, real-world workloads, and coverage thresholds for the Agnes-AI project.

---

## 1. Test Philosophy & 4-Tier Methodology

The testing strategy for Agnes-AI adheres to an **opaque-box, requirement-driven testing** philosophy. We treat the system under test as a black box (focusing on interfaces, inputs, and outputs rather than internal logic branches) and prioritize verification based strictly on requirements derived from `ORIGINAL_REQUEST.md`.

To achieve maximum reliability and safety in an autonomous AI agent context, the test suite utilizes a **4-Tier Test Methodology**:

```
+-----------------------------------------------------------------+
| Tier 4: Real-World Workload Testing (High-Fidelity Scenarios)   |
+-----------------------------------------------------------------+
| Tier 3: Pairwise Combinatorial Testing (Feature Interactions)   |
+-----------------------------------------------------------------+
| Tier 2: Boundary Value Analysis (Corner Cases & Error States)   |
+-----------------------------------------------------------------+
| Tier 1: Category-Partition Testing (Functional Feature Paths)   |
+-----------------------------------------------------------------+
```

### Tier 1: Category-Partition (Feature Coverage)
- **Objective**: Divide each feature's input domain into distinct functional partitions (positive path testing).
- **Execution**: Design specific test cases to cover each partition, ensuring that basic capabilities perform according to specifications under standard conditions.

### Tier 2: Boundary Value Analysis (BVA)
- **Objective**: Target limits, extreme values, empty inputs, capacity limits, and error conditions.
- **Execution**: Write tests focusing on the edge inputs (e.g., token usage precisely at 80% or 100%, 0-value budgets, truncated logs, empty directories, maximum path lengths, and forbidden OS filenames on Windows).

### Tier 3: Pairwise Combinatorial Testing
- **Objective**: Identify integration issues arising from feature interactions (e.g., model routing changes occurring mid-repair, budget depletion triggering during cleanup, etc.).
- **Execution**: Apply combinatorial logic to test interactions between key features (QA Repair, Regression Replay, Asymmetric Routing, and Build/Clippy/Cleanup), keeping the test matrix efficient but thorough.

### Tier 4: Real-World Workload Testing (Application Scenarios)
- **Objective**: Validate the system against realistic, multi-step scenarios representing actual human-agent interaction and autonomous workflows.
- **Execution**: Implement high-fidelity integration test paths that exercise orchestration, recovery loops, concurrency, and self-healing.

---

## 2. Feature Inventory

The test suite covers four core features, directly mapped from the requirements in `ORIGINAL_REQUEST.md`:

### R1: QA Repair (Prompt Self-Repair Pipeline)
- **Description**: The mechanism to intercept failed tool calls, map failures to repair instructions, and pre-inject historical repairs.
- **Keys**:
  - Intercepting validation gate failures (e.g., structure, schema, paths, commands) via `.agent/rules/qa_validation.toon`.
  - Fetching repair instructions from the `repair_table`.
  - Appending delta feedback (error code, error line, repair payload) back to the agent instead of full-context resubmission.
  - Recording successful repair instructions in `memory_tags/qa_pipeline/`.
  - Pre-injecting historical repairs into future system prompts for similar tasks.

### R2: Regression Replay
- **Description**: An offline, zero-token regression engine that replays historical or synthetic failure fixtures.
- **Keys**:
  - Populating `src-tauri/tests/fixtures/qa_corpus/` with distinct failure files (at least one for each error code: `E_SCHEMA`, `E_PROGRAM`, `E_ARGS`, `E_PATH`, `E_SHELL`, `E_SECRET`, `E_DESTRUCT`, `E_COMPILE`).
  - Executing test cases offline without sending network requests to LLM APIs.
  - Asserting that the deterministic validation gates intercept each fixture with the expected error code.

### R3: Asymmetric Routing & Token Economy
- **Description**: Dynamic mapping of model tiers and token budget management based on task classification.
- **Keys**:
  - Routing simple tasks (tags, classification, semantic audit) to flash-grade models.
  - Routing code generation and complex logic to main-grade models.
  - Upgrading to high-grade models on repeated self-repair failures.
  - Tracking session token budgets in SQLite `token_ledger` table.
  - Triggering warning events at 80% usage and locking further API calls at 100% usage.

### R4: Build/Clippy/Cleanup
- **Description**: Maintaining build health, running clippy checks, and executing phase 0 cleanups.
- **Keys**:
  - Deleting leftover Tauri directories and configuration files (`tauri.conf.json`, `capabilities/`, `gen/`).
  - Deleting `nul` residue files safely, respecting OS device limits on Windows (using `\\.\nul`).
  - Ensuring the build is warning-free via `cargo clippy --all-targets -D warnings` and `cargo check`.

---

## 3. Test Architecture

The E2E test suite is integrated into the native Rust test structure of the `src-tauri` workspace.

### Test Runner
E2E integration tests are run via Cargo:
```powershell
# From the project root or src-tauri folder
cargo test --test e2e_tests
```

### Test Case Format
Tests are written as standard Rust integration tests within `src-tauri/tests/e2e_tests.rs`. Rather than failing at compile-time when features are unimplemented, tests are designed to dynamically probe the engine state and fail at runtime with descriptive messages, allowing clean compilation at all times.

### Directory Layout
The testing infrastructure utilizes the following files and directories:
- `src-tauri/tests/e2e_tests.rs` — Main E2E integration test file containing test definitions.
- `src-tauri/tests/fixtures/qa_corpus/` — Directory holding JSON failure fixtures.
  - Contains subdirectories named after deterministic error codes containing test cases (e.g., `E_PATH/dummy_sample.json`, `E_SCHEMA/...`, etc.).

---

## 4. Real-World Application Scenarios (Tier 4)

Tier 4 contains high-fidelity scenarios representing real-world workloads:

### A. Full Agent Workflow
Simulates a complete, multi-step development request. It triggers RAG memory retrieval (漏斗 RAG), routes task planning to a flash model, invokes the main model to write Rust source code, runs deterministic gates (D1–D8) on output tools, and performs a sandbox exit-code compilation validation.

### B. Regression Suite Replay
Simulates an offline regression run. It scans the `src-tauri/tests/fixtures/qa_corpus/` folder, parses all regression test files, passes them through the Rust-side validation gates, and asserts that 100% of the files are correctly intercepted with their designated error code without any network usage.

### C. Budget Depletion Scenario
Simulates a long-running, token-heavy agent session. It accumulates estimated and actual token usage, verifies that an 80% usage threshold triggers warning logs in the SQLite database, and checks that hitting 100% usage locks LLM requests immediately while allowing local file read/write and deterministic tasks to proceed.

### D. Clippy Check Self-Healing
Simulates code compilation warning recovery. It creates a Rust source file with non-compliant code (e.g., unused imports, style warnings), triggering `E_COMPILE` in the validation gate. The engine extracts compiler errors, passes the delta to the repair loop, and updates the code until it compiles cleanly under `cargo clippy --all-targets -D warnings`.

### E. Multitask Concurrency & Routing
Simulates parallel operations in the agent pipeline. It dispatches multiple sub-agents concurrently using Tokio's `JoinSet` (e.g., one doing text distillation, one writing code, one doing audits). It verifies that asymmetric routing maps them to appropriate model tiers concurrently, while ensuring there are no database table locks or race conditions.

---

## 5. Coverage Thresholds & Test Cases

To guarantee thorough coverage, the E2E test suite adheres to strict minimum case counts, totaling **49 distinct test cases**:

| Tier | Description | Minimum Cases |
|---|---|---|
| **Tier 1** | Category-Partition (Feature Coverage) | 20 (>=5 per feature) |
| **Tier 2** | Boundary Value Analysis (BVA) | 20 (>=5 per feature) |
| **Tier 3** | Pairwise Combinatorial Testing | 4 |
| **Tier 4** | Real-World Application Scenarios | 5 |
| **Total** | | **49** |

### Detailed Test Case Inventory

#### Tier 1: Category-Partition (Feature Coverage) - 20 Cases
- **R1: QA Repair (Prompt Self-Repair)**
  1. `test_r1_qa_repair_e_schema`: Schema violation repair cycle. Verifies incorrect schema triggers `E_SCHEMA`, maps to repair instructions, and recovers.
  2. `test_r1_qa_repair_e_path`: Path traversal repair cycle. Verifies out-of-workspace paths trigger `E_PATH`, maps to correct boundary constraints, and recovers.
  3. `test_r1_qa_repair_e_args`: Argument sanitization repair cycle. Verifies dangerous arguments/characters trigger `E_ARGS`, maps to list-based arguments, and recovers.
  4. `test_r1_qa_repair_e_compile`: Compile warning/error repair cycle. Verifies code compilation failure triggers `E_COMPILE`, returns compiler output slice, and heals.
  5. `test_r1_qa_repair_persist_success`: Persistent memory injection. Verifies successful repair writes tag instructions to `memory_tags/qa_pipeline/` and injects them in subsequent task system prompts.
- **R2: Regression Replay**
  6. `test_r2_replay_load_fixtures`: JSON deserialization. Verifies all JSON fixtures in `src-tauri/tests/fixtures/qa_corpus/` are parsed correctly without errors.
  7. `test_r2_replay_deterministic_gates`: Validation gate execution. Verifies fixtures are evaluated purely via deterministic gates offline (0-token).
  8. `test_r2_replay_match_expected_error`: Expected failure code assertion. Verifies fixture error code matches the expected target code (`E_PATH`, `E_SECRET`, etc.).
  9. `test_r2_replay_no_network`: Air-gapped isolation. Verifies the regression suite runs and asserts offline (no network requests sent).
  10. `test_r2_replay_report_generation`: Regression run report. Verifies the runner outputs a structured report summarizing pass/fail counts for fixtures.
- **R3: Asymmetric Routing**
  11. `test_r3_routing_flash_low_risk`: Low risk task routing. Verifies tag matching, text distillation, and low-risk semantic checks route to the cost-efficient flash model.
  12. `test_r3_routing_main_generation`: Code generation routing. Verifies complex coding/reasoning tasks route to the main-grade model.
  13. `test_r3_routing_high_repeated_failure`: Upgrade on failure. Verifies that when a task experiences repeated validation failures, the engine upgrades to the high-grade model for the final attempt.
  14. `test_r3_budget_warning_80`: Under-budget warning. Verifies token usage at 80% triggers a warning and logs to SQLite `token_ledger`.
  15. `test_r3_budget_lock_100`: Budget depletion locking. Verifies token usage at 100% blocks new LLM requests while keeping local validation running.
- **R4: Build/Clippy/Cleanup**
  16. `test_r4_cleanup_nul_residues`: Windows device name cleanup. Verifies that `nul` files (including device-specific names) are cleaned using safe UNC pathing.
  17. `test_r4_cleanup_tauri_leftovers`: Tauri directories removal. Verifies that `tauri.conf.json`, `capabilities/`, and `gen/` are deleted and not referenced.
  18. `test_r4_clippy_zero_warnings`: Code quality validation. Verifies running `cargo clippy --all-targets -D warnings` completes with 0 warnings.
  19. `test_r4_cargo_check_compilation`: Clean build check. Verifies the project builds from scratch without compile-time warnings.
  20. `test_r4_environment_variable_control`: Build env-vars. Verifies the build process respects custom environment configurations (e.g. `AGNES_QA_SHOT`).

#### Tier 2: Boundary Value Analysis (BVA) - 20 Cases
- **R1: QA Repair (Prompt Self-Repair)**
  21. `test_r1_bva_empty_repair_table`: Empty rule map. Verifies behavior when the repair table has no entry for an unknown gate failure.
  22. `test_r1_bva_max_repair_limit`: Threshold cutoff. Verifies that exceeding `Config.qa.max_repairs` (default 3) stops self-repair, upgrades to high-grade, or fails.
  23. `test_r1_bva_large_stderr_truncation`: Large stderr output. Verifies truncation of massive compiler errors to `Config.qa.stderr_max_lines` (prevents token blowup).
  24. `test_r1_bva_unicode_repair_injection`: Multi-byte characters. Verifies that CJK or special characters in error messages are safely handled during prompt repair injection.
  25. `test_r1_bva_malformed_memory_tag`: Invalid memory tag files. Verifies that corrupt files in `memory_tags/qa_pipeline/` are ignored or gracefully handled instead of crashing the engine.
- **R2: Regression Replay**
  26. `test_r2_bva_missing_fixtures`: Empty directory. Verifies regression suite handles empty fixture directories gracefully without panic.
  27. `test_r2_bva_malformed_fixture_json`: Invalid JSON structure. Verifies that syntax-corrupted fixture JSON files are reported as failures or skipped with warnings.
  28. `test_r2_bva_duplicate_fixtures`: Identity collisions. Verifies that duplicate filenames or IDs across fixture categories do not conflict in reports.
  29. `test_r2_bva_extreme_fixture_sizes`: Large payload fixtures. Verifies that extremely large tool execution payloads in fixtures do not exhaust system memory.
  30. `test_r2_bva_unknown_error_code`: Non-standard error codes. Verifies that custom/unknown error codes in fixtures are processed safely by fallback gates.
- **R3: Asymmetric Routing**
  31. `test_r3_bva_zero_budget`: Immediate lock. Verifies setting token budget to 0 immediately blocks all model requests.
  32. `test_r3_bva_exact_80_percent`: Threshold boundary. Verifies behavior exactly at 80% usage boundary to ensure no off-by-one errors in warnings.
  33. `test_r3_bva_exact_100_percent`: Threshold boundary. Verifies behavior exactly at 100% usage boundary to ensure immediate lock activation.
  34. `test_r3_bva_model_fallback_offline`: Provider failure. Verifies system routing gracefully defaults to local offline error handling when model endpoints are unreachable.
  35. `test_r3_bva_massive_token_estimate`: Estimate overshoot. Verifies that if a single estimated prompt exceeds the remaining budget, the request is blocked before calling the API.
- **R4: Build/Clippy/Cleanup**
  36. `test_r4_bva_locked_nul_file`: Locked OS file. Verifies cleanup handles file locks on Windows (e.g. attempting to delete `nul` when open).
  37. `test_r4_bva_missing_cleanup_targets`: Idempotency. Verifies that running cleanup when leftovers are already deleted does not throw errors.
  38. `test_r4_bva_partially_built_target`: Interrupted compilation. Verifies build system handles half-compiled target states without corruption.
  39. `test_r4_bva_extreme_path_length`: Windows MAX_PATH limits. Verifies that build/cleanup tasks handle very long nested paths (>260 characters).
  40. `test_r4_bva_readonly_directories`: Permission boundaries. Verifies that cleanup processes log warnings and do not crash when encountering read-only target directories.

#### Tier 3: Pairwise Combinatorial Testing - 4 Cases
- 41. `test_r1_r3_pairwise_repair_and_routing`: QA Repair + Asymmetric Routing. Verifies that self-repair updates are routed to different model tiers depending on failures and complexity, and that model upgrades correctly retain the repair context.
- 42. `test_r2_r3_pairwise_replay_and_routing`: Regression Replay + Asymmetric Routing. Verifies that offline regression replays evaluate and simulate model routing metadata (asserting that dummy model selections in fixtures don't trigger real network calls).
- 43. `test_r1_r4_pairwise_repair_and_build`: QA Repair + Build/Clippy/Cleanup. Verifies that if self-repair generates code changes, the build system correctly detects changes, re-runs clippy, and compiles without triggering redundant cleanups.
- 44. `test_r2_r4_pairwise_replay_and_cleanup`: Regression Replay + Build/Clippy/Cleanup. Verifies that running the regression replay does not leave temporary fixtures/artifacts behind, cleaning up workspace state post-run.

#### Tier 4: Real-World Workload Testing (Application Scenarios) - 5 Cases
- 45. `test_t4_full_agent_workflow`: Full Agent Workflow. End-to-end task simulation where a user requests a featureset, leading to sub-agent dispatch, RAG document lookup, code writing, and validation checks.
- 46. `test_t4_regression_suite_replay`: Regression Suite Replay. Parallel/sequential execution of all 8 regression corpus fixtures (`E_SCHEMA`, `E_PROGRAM`, `E_ARGS`, `E_PATH`, `E_SHELL`, `E_SECRET`, `E_DESTRUCT`, `E_COMPILE`) and full assertion validation.
- 47. `test_t4_budget_depletion_scenario`: Budget Depletion Scenario. Runs a multi-step task that exceeds budget thresholds, verifying that warning events are fired, and the engine gracefully transitions to a read-only/offline mode.
- 48. `test_t4_clippy_check_self_healing`: Clippy Check Self-Healing. Generates a file containing code style/clippy warnings, runs validation gate D8, triggers the self-repair loop with clippy warning output, and asserts warning-free resolution.
- 49. `test_t4_multitask_concurrency_routing`: Multitask Concurrency & Routing. Runs parallel tasks using tokio `JoinSet` simulating multiple sub-agents utilizing asymmetric routing (some flash, some main) concurrently, and asserts no database conflicts in `token_ledger` or file collisions in workspace.
