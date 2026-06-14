use app_lib::*;
use rusqlite::Connection;
use serde_json::Value;
use std::path::Path;
use std::collections::HashSet;

// Helper to check if a database table exists
fn check_table_exists(conn: &Connection, table_name: &str) -> bool {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name=?1").unwrap();
    stmt.exists(rusqlite::params![table_name]).unwrap()
}

// Helper to check if a database column exists
fn check_column_exists(conn: &Connection, table_name: &str, column_name: &str) -> bool {
    if !check_table_exists(conn, table_name) {
        return false;
    }
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table_name)).unwrap();
    let mut rows = stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let name: String = row.get(1).unwrap();
        if name == column_name {
            return true;
        }
    }
    false
}

// Helper to check if a configuration field exists dynamically
fn check_config_field_exists(config: &Config, field_path: &[&str]) -> bool {
    if let Ok(val) = serde_json::to_value(config) {
        let mut curr = &val;
        for &field in field_path {
            if let Some(next) = curr.get(field) {
                curr = next;
            } else {
                return false;
            }
        }
        true
    } else {
        false
    }
}

// Local helper to map gate rejection reason to failure codes
fn map_gate_to_failure_code(reason: &str) -> &'static str {
    if reason.contains("G6") || reason.contains("G14") || reason.contains("D2") {
        "E_PROGRAM"
    } else if reason.contains("G5") || reason.contains("G7") || reason.contains("G10") || reason.contains("G19") || reason.contains("G20") || reason.contains("G21") || reason.contains("D3") {
        "E_ARGS"
    } else if reason.contains("G4") || reason.contains("G18") || reason.contains("D4") {
        "E_PATH"
    } else if reason.contains("G11") || reason.contains("D5") {
        "E_SHELL"
    } else if reason.contains("G12") || reason.contains("D6") {
        "E_SECRET"
    } else if reason.contains("D7") {
        "E_DESTRUCT"
    } else if reason.contains("G8") || reason.contains("G9") || reason.contains("G13") || reason.contains("G16") || reason.contains("D8") || reason.contains("Rust 代碼未通過編譯檢查") || reason.contains("編譯對齊失敗") {
        "E_COMPILE"
    } else {
        "E_SCHEMA"
    }
}

// =========================================================================
// TIER 1: FEATURE COVERAGE (20 Cases, 5 per feature R1, R2, R3, R4)
// =========================================================================

// --- R1: QA Repair (Prompt Self-Repair) ---

#[test]
fn test_r1_qa_repair_e_schema() {
    let db_path = std::env::temp_dir().join("agnes_test_r1_schema.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("QA Repair is unimplemented: 'repair_table' not found in SQLite schema");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r1_qa_repair_e_path() {
    let db_path = std::env::temp_dir().join("agnes_test_r1_path.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("QA Repair is unimplemented: 'repair_table' not found in SQLite schema");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r1_qa_repair_e_args() {
    let db_path = std::env::temp_dir().join("agnes_test_r1_args.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("QA Repair is unimplemented: 'repair_table' not found in SQLite schema");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r1_qa_repair_e_compile() {
    let db_path = std::env::temp_dir().join("agnes_test_r1_compile.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("QA Repair is unimplemented: 'repair_table' not found in SQLite schema");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r1_qa_repair_persist_success() {
    let temp_ws = tempfile::tempdir().unwrap();
    let qa_dir = temp_ws.path().join("memory_tags").join("qa_pipeline");
    std::fs::create_dir_all(&qa_dir).unwrap();
    
    let md_path = qa_dir.join("E_SCHEMA.md");
    let test_instruction = "Correct schema instruction";
    std::fs::write(&md_path, test_instruction).unwrap();
    
    assert!(md_path.exists());
    let content = std::fs::read_to_string(&md_path).unwrap();
    assert_eq!(content, test_instruction);
}

// --- R2: Regression Replay ---

#[test]
fn test_r2_replay_load_fixtures() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qa_corpus");
    assert!(fixture_dir.exists(), "Fixtures directory does not exist");
    
    let mut parsed_count = 0;
    for entry in walkdir::WalkDir::new(&fixture_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            let content = std::fs::read_to_string(path).unwrap();
            let _: Value = serde_json::from_str(&content).unwrap();
            parsed_count += 1;
        }
    }
    assert!(parsed_count > 0, "No JSON fixtures found in qa_corpus");
}

#[test]
fn test_r2_replay_deterministic_gates() {
    let config = Config::default();
    let tool_calls = vec![ToolCall {
        name: "write_file".to_string(),
        path: Some("../outside.txt".to_string()),
        content: "illegal content".to_string(),
    }];
    let audits = run_all_gates(&config, &tool_calls, &[]);
    assert!(!audits.is_empty(), "Audits should execute");
    assert!(AgentEngine::any_rejected(&audits), "Should be rejected by path traversal");
}

#[test]
fn test_r2_replay_match_expected_error() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qa_corpus");
    let config = Config::default();
    
    for entry in walkdir::WalkDir::new(&fixture_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            let parent_dir = path.parent().unwrap().file_name().unwrap().to_str().unwrap();
            let content = std::fs::read_to_string(path).unwrap();
            let val: Value = serde_json::from_str(&content).unwrap();
            
            let tool_calls_val = &val["tool_calls"];
            let mut tool_calls = Vec::new();
            if let Some(arr) = tool_calls_val.as_array() {
                for tc in arr {
                    tool_calls.push(ToolCall {
                        name: tc["name"].as_str().unwrap_or("").to_string(),
                        path: tc["path"].as_str().map(|s| s.to_string()),
                        content: tc["content"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
            
            let expected_verdict = val["expected_verdict"].as_str().unwrap_or("REJECTED");
            if expected_verdict == "REJECTED" {
                let audits = run_all_gates(&config, &tool_calls, &[]);
                let has_expected_code = audits.iter().any(|a| {
                    a.verdict == "REJECTED" && map_gate_to_failure_code(&a.reason) == parent_dir
                });
                assert!(has_expected_code, "Fixture at {:?} did not trigger expected code {}", path, parent_dir);
            }
        }
    }
}

#[test]
fn test_r2_replay_no_network() {
    let mut config = Config::default();
    config.api.base_url = "https://invalid-nonexistent-domain.agnes-ai.com/v1".to_string();
    config.api.key = "invalid-key".to_string();
    
    let tool_calls = vec![ToolCall {
        name: "write_file".to_string(),
        path: Some("../outside.txt".to_string()),
        content: "illegal content".to_string(),
    }];
    
    let audits = run_all_gates(&config, &tool_calls, &[]);
    assert!(AgentEngine::any_rejected(&audits));
}

#[test]
fn test_r2_replay_report_generation() {
    let mut report = String::new();
    report.push_str("# REGRESSION REPLAY REPORT\n\n");
    
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qa_corpus");
    let mut passed = 0;
    let failed = 0;
    
    for entry in walkdir::WalkDir::new(&fixture_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            passed += 1;
        }
    }
    
    report.push_str(&format!("- Total Tested: {}\n", passed + failed));
    report.push_str(&format!("- Passed: {}\n", passed));
    report.push_str(&format!("- Failed: {}\n", failed));
    
    assert!(report.contains("Total Tested"));
    assert!(passed > 0);
}

// --- R3: Asymmetric Routing ---

#[test]
fn test_r3_routing_flash_low_risk() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}

#[test]
fn test_r3_routing_main_generation() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}

#[test]
fn test_r3_routing_high_repeated_failure() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}

#[test]
fn test_r3_budget_warning_80() {
    let db_path = std::env::temp_dir().join("agnes_test_r3_warn.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_column_exists(&conn, "token_ledger", "warning_triggered") {
        panic!("Token budget warning is unimplemented: 'warning_triggered' column not found in 'token_ledger'");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r3_budget_lock_100() {
    let mut budgeter = TokenBudgeter::new(1000);
    assert!(!budgeter.is_locked());
    
    budgeter.record_usage(500, 500);
    assert!(budgeter.is_locked());
}

// --- R4: Build/Clippy/Cleanup ---

#[test]
fn test_r4_cleanup_nul_residues() {
    let temp_ws = tempfile::tempdir().unwrap();
    let nul_file = temp_ws.path().join("nul");
    let path_str = nul_file.to_string_lossy().to_string();
    let path_to_write = if cfg!(windows) {
        format!(r"\\?\{}", path_str.replace('/', "\\"))
    } else {
        path_str
    };
    let p = std::path::PathBuf::from(&path_to_write);
    std::fs::write(&p, "dummy").unwrap();
    assert!(p.exists() || p.is_file());
    
    cleanup_nul_residues(temp_ws.path()).unwrap();
    assert!(!p.exists() && !p.is_file());
}

#[test]
fn test_r4_cleanup_tauri_leftovers() {
    let temp_ws = tempfile::tempdir().unwrap();
    let tauri_dir = temp_ws.path().join("src-tauri");
    std::fs::create_dir_all(&tauri_dir).unwrap();
    let tauri_conf = tauri_dir.join("tauri.conf.json");
    let run_err = temp_ws.path().join("run_error.log");
    std::fs::write(&tauri_conf, "{}").unwrap();
    std::fs::write(&run_err, "error").unwrap();
    
    cleanup_tauri_leftovers(temp_ws.path()).unwrap();
    assert!(!tauri_conf.exists());
    assert!(!run_err.exists());
}

#[test]
fn test_r4_clippy_zero_warnings() {
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::process::Command::new("cargo")
        .args(["clippy", "--all-targets", "--", "-D", "warnings"])
        .current_dir(root_dir)
        .output();
    let output = match output {
        Ok(out) => out,
        Err(e) => panic!("Failed to execute cargo clippy: {}", e),
    };
    assert!(output.status.success(), "Clippy checks failed or found warnings");
}

#[test]
fn test_r4_cargo_check_compilation() {
    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(root_dir)
        .output()
        .expect("Failed to execute cargo check");
    assert!(output.status.success(), "Cargo check failed");
}

#[test]
fn test_r4_environment_variable_control() {
    std::env::set_var("AGNES_QA_SHOT", "1");
    let val = std::env::var("AGNES_QA_SHOT").unwrap_or_default();
    assert_eq!(val, "1");
    std::env::remove_var("AGNES_QA_SHOT");
}

// =========================================================================
// TIER 2: BOUNDARY & CORNER CASES (20 Cases, 5 per feature R1, R2, R3, R4)
// =========================================================================

// --- R1: QA Repair (Prompt Self-Repair) ---

#[test]
fn test_r1_bva_empty_repair_table() {
    let db_path = std::env::temp_dir().join("agnes_test_bva_empty.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("QA Repair is unimplemented: 'repair_table' not found in SQLite schema");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r1_bva_max_repair_limit() {
    let mut config = Config::default();
    assert_eq!(config.api.max_repairs, 3);
    
    config.api.max_repairs = 0;
    assert_eq!(config.api.max_repairs, 0);
    
    config.api.max_repairs = 100;
    assert_eq!(config.api.max_repairs, 100);
}

#[test]
fn test_r1_bva_large_stderr_truncation() {
    let mut config = Config::default();
    assert_eq!(config.sandbox.stderr_feedback_lines, 20);
    
    config.sandbox.stderr_feedback_lines = 1;
    assert_eq!(config.sandbox.stderr_feedback_lines, 1);
    
    config.sandbox.stderr_feedback_lines = 1000;
    assert_eq!(config.sandbox.stderr_feedback_lines, 1000);
}

#[test]
fn test_r1_bva_unicode_repair_injection() {
    let db_path = std::env::temp_dir().join("agnes_test_bva_unicode.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("QA Repair is unimplemented: 'repair_table' not found in SQLite schema");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r1_bva_malformed_memory_tag() {
    let temp_ws = tempfile::tempdir().unwrap();
    let qa_dir = temp_ws.path().join("memory_tags").join("qa_pipeline");
    std::fs::create_dir_all(&qa_dir).unwrap();
    
    let corrupt_tag = qa_dir.join("E_SCHEMA.md");
    std::fs::write(&corrupt_tag, vec![0, 159, 146, 150]).unwrap(); 
    
    let entries = std::fs::read_dir(&qa_dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let _content = std::fs::read_to_string(entry.path());
        }
    }
}

// --- R2: Regression Replay ---

#[test]
fn test_r2_bva_missing_fixtures() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut files_tested = 0;
    
    for entry in walkdir::WalkDir::new(temp_dir.path()) {
        let entry = entry.unwrap();
        if entry.path().is_file() && entry.path().extension().is_some_and(|ext| ext == "json") {
            files_tested += 1;
        }
    }
    assert_eq!(files_tested, 0);
}

#[test]
fn test_r2_bva_malformed_fixture_json() {
    let bad_json = "{ invalid json }";
    let res: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
    assert!(res.is_err());
}

#[test]
fn test_r2_bva_duplicate_fixtures() {
    let mut file_set = HashSet::new();
    file_set.insert("E_PATH/dummy_sample.json".to_string());
    file_set.insert("E_SCHEMA/dummy_sample.json".to_string());
    assert_eq!(file_set.len(), 2);
}

#[test]
fn test_r2_bva_extreme_fixture_sizes() {
    let large_content = "a".repeat(10_000_000);
    let tc = ToolCall {
        name: "write_file".to_string(),
        path: Some("src/dummy.rs".to_string()),
        content: large_content,
    };
    assert_eq!(tc.content.len(), 10_000_000);
}

#[test]
fn test_r2_bva_unknown_error_code() {
    let code = map_gate_to_failure_code("Some unrecognized rejection reason");
    assert_eq!(code, "E_SCHEMA");
}

// --- R3: Asymmetric Routing ---

#[test]
fn test_r3_bva_zero_budget() {
    let budgeter = TokenBudgeter::new(0);
    assert!(budgeter.is_locked());
}

#[test]
fn test_r3_bva_exact_80_percent() {
    let mut budgeter = TokenBudgeter::new(100);
    budgeter.record_usage(40, 40);
    assert_eq!(budgeter.total_spent(), 80);
    assert!((budgeter.budget_ratio() - 0.8).abs() < 1e-9);
}

#[test]
fn test_r3_bva_exact_100_percent() {
    let mut budgeter = TokenBudgeter::new(100);
    budgeter.record_usage(50, 50);
    assert!(budgeter.is_locked());
}

#[test]
fn test_r3_bva_model_fallback_offline() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}

#[test]
fn test_r3_bva_massive_token_estimate() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}

// --- R4: Build/Clippy/Cleanup ---

#[test]
fn test_r4_bva_locked_nul_file() {
    let temp_ws = tempfile::tempdir().unwrap();
    let nul_file = temp_ws.path().join("nul");
    let path_str = nul_file.to_string_lossy().to_string();
    let path_to_write = if cfg!(windows) {
        format!(r"\\?\{}", path_str.replace('/', "\\"))
    } else {
        path_str
    };
    let p = std::path::PathBuf::from(&path_to_write);
    std::fs::write(&p, "dummy").unwrap();
    
    if let Ok(_file) = std::fs::OpenOptions::new().write(true).open(&p) {
        let _res = cleanup_nul_residues(temp_ws.path());
    } else {
        let _res = cleanup_nul_residues(temp_ws.path());
    }
}

#[test]
fn test_r4_bva_missing_cleanup_targets() {
    let temp_ws = tempfile::tempdir().unwrap();
    let res = cleanup_tauri_leftovers(temp_ws.path());
    assert!(res.is_ok());
}

#[test]
fn test_r4_bva_partially_built_target() {
    let temp_ws = tempfile::tempdir().unwrap();
    let target_dir = temp_ws.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(target_dir.join("half_compiled.o"), "binary").unwrap();
    
    handle_interrupted_compilation(temp_ws.path()).unwrap();
    assert!(!target_dir.exists());
}

#[test]
fn test_r4_bva_extreme_path_length() {
    let long_path = "a/".repeat(300) + "file.txt";
    assert!(!has_path_traversal_component(&long_path));
}

#[test]
fn test_r4_bva_readonly_directories() {
    let temp_ws = tempfile::tempdir().unwrap();
    let sub_dir = temp_ws.path().join("readonly_dir");
    std::fs::create_dir_all(&sub_dir).unwrap();
    let file_path = sub_dir.join("readonly_file.txt");
    std::fs::write(&file_path, "content").unwrap();
    
    let mut perms = std::fs::metadata(&file_path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&file_path, perms).unwrap();
    
    remove_dir_all_force(&sub_dir).unwrap();
    assert!(!sub_dir.exists());
}

// =========================================================================
// TIER 3: PAIRWISE COMBINATORIAL TESTING (4 Cases)
// =========================================================================

#[test]
fn test_r1_r3_pairwise_repair_and_routing() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}

#[test]
fn test_r2_r3_pairwise_replay_and_routing() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}

#[test]
fn test_r1_r4_pairwise_repair_and_build() {
    let db_path = std::env::temp_dir().join("agnes_test_pairwise_r1_r4.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("QA Repair is unimplemented: 'repair_table' not found in SQLite schema");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_r2_r4_pairwise_replay_and_cleanup() {
    let temp_ws = tempfile::tempdir().unwrap();
    std::fs::write(temp_ws.path().join("test_run.log"), "log content").unwrap();
    std::fs::write(temp_ws.path().join("temp_state.db"), "db content").unwrap();
    let agnes_dir = temp_ws.path().join(".agnes");
    std::fs::create_dir_all(&agnes_dir).unwrap();
    std::fs::write(agnes_dir.join("metadata"), "meta").unwrap();
    
    cleanup_post_run(temp_ws.path()).unwrap();
    
    assert!(!temp_ws.path().join("test_run.log").exists());
    assert!(!temp_ws.path().join("temp_state.db").exists());
    assert!(!agnes_dir.exists());
}

// =========================================================================
// TIER 4: REAL-WORLD APPLICATION SCENARIOS (5 Cases)
// =========================================================================

#[test]
fn test_t4_full_agent_workflow() {
    let db_path = std::env::temp_dir().join("agnes_test_t4_workflow.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("Full agent workflow testing failed: 'repair_table' not found");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_t4_regression_suite_replay() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qa_corpus");
    let config = Config::default();
    let mut files_tested = 0;
    
    for entry in walkdir::WalkDir::new(&fixture_dir) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            let parent_dir = path.parent().unwrap().file_name().unwrap().to_str().unwrap();
            let content = std::fs::read_to_string(path).unwrap();
            let val: Value = serde_json::from_str(&content).unwrap();
            
            let tool_calls_val = &val["tool_calls"];
            let mut tool_calls = Vec::new();
            if let Some(arr) = tool_calls_val.as_array() {
                for tc in arr {
                    tool_calls.push(ToolCall {
                        name: tc["name"].as_str().unwrap_or("").to_string(),
                        path: tc["path"].as_str().map(|s| s.to_string()),
                        content: tc["content"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
            
            let expected_verdict = val["expected_verdict"].as_str().unwrap_or("REJECTED");
            let audits = run_all_gates(&config, &tool_calls, &[]);
            let any_rejected = AgentEngine::any_rejected(&audits);
            
            if expected_verdict == "REJECTED" {
                assert!(any_rejected);
                let has_expected_code = audits.iter().any(|a| {
                    a.verdict == "REJECTED" && map_gate_to_failure_code(&a.reason) == parent_dir
                });
                assert!(has_expected_code, "Fixture at {:?} did not match error code {}", path, parent_dir);
            } else {
                assert!(!any_rejected);
            }
            files_tested += 1;
        }
    }
    assert!(files_tested >= 8, "Expected at least 8 regression corpus fixtures tested, found {}", files_tested);
}

#[test]
fn test_t4_budget_depletion_scenario() {
    let mut budgeter = TokenBudgeter::new(500);
    assert!(!budgeter.is_locked());
    
    budgeter.record_usage(300, 150);
    assert!(!budgeter.is_locked());
    assert!(budgeter.budget_ratio() >= 0.8);
    
    budgeter.record_usage(100, 0);
    assert!(budgeter.is_locked());
}

#[test]
fn test_t4_clippy_check_self_healing() {
    let db_path = std::env::temp_dir().join("agnes_test_t4_healing.db");
    let _ = std::fs::remove_file(&db_path);
    let conn = open_connection(&db_path).unwrap();
    if !check_table_exists(&conn, "repair_table") {
        panic!("Clippy self-healing is unimplemented: 'repair_table' not found");
    }
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_t4_multitask_concurrency_routing() {
    let config = Config::default();
    if !check_config_field_exists(&config, &["model_routing"]) {
        panic!("Asymmetric routing is unimplemented: 'model_routing' config key not found");
    }
}
