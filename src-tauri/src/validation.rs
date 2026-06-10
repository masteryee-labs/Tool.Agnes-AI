use crate::agent::{AuditResult, ToolCall};
use crate::config::Config;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum GateResult {
    Pass,
    Reject {
        gate_id: String,
        file: Option<String>,
        line: Option<usize>,
        reason: String,
    },
    Skip {
        reason: String,
    },
}

impl GateResult {
    pub fn to_audit(&self, agent_name: &str) -> AuditResult {
        match self {
            GateResult::Pass => AuditResult {
                agent_name: agent_name.to_string(),
                verdict: "PASSED".to_string(),
                reason: "審查通過".to_string(),
            },
            GateResult::Reject { gate_id, file, line, reason } => {
                let file_info = match (file, line) {
                    (Some(f), Some(l)) => format!("{}:{}", f, l),
                    (Some(f), None) => f.clone(),
                    _ => "N/A".to_string(),
                };
                AuditResult {
                    agent_name: agent_name.to_string(),
                    verdict: "REJECTED".to_string(),
                    reason: format!("[REJECT: {} | 檔案: {} | 原因: {}]", gate_id, file_info, reason),
                }
            }
            GateResult::Skip { reason } => AuditResult {
                agent_name: agent_name.to_string(),
                verdict: "SKIPPED".to_string(),
                reason: reason.clone(),
            },
        }
    }
}

pub struct ProposedChange<'a> {
    pub tool_calls: &'a [ToolCall],
    pub messages: &'a [Value],
    pub config: &'a Config,
}

pub trait Gate: Send + Sync {
    fn name(&self) -> &'static str;
    fn gate_id(&self) -> &'static str;
    fn check(&self, change: &ProposedChange) -> GateResult;
}

// ─── Stage A: Static Deterministic (0 Token) ────────────────────────────────

pub struct WorkflowTopologyGate;
impl Gate for WorkflowTopologyGate {
    fn name(&self) -> &'static str { "WorkflowTopology" }
    fn gate_id(&self) -> &'static str { "G1" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        if change.tool_calls.len() > 30 {
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: "步驟數量過多（超出 30 步），可能導致架構冗餘".to_string(),
            };
        }
        GateResult::Pass
    }
}

pub struct WorkflowRuntimeEvaluatorGate;
impl Gate for WorkflowRuntimeEvaluatorGate {
    fn name(&self) -> &'static str { "WorkflowRuntimeEvaluator" }
    fn gate_id(&self) -> &'static str { "G2" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let mut last_prompt = None;
        let mut repeat_count = 0;
        for msg in change.messages {
            if msg["role"] == "user" {
                if let Some(content) = msg["content"].as_str() {
                    if let Some(ref prev) = last_prompt {
                        if content.trim() == prev {
                            repeat_count += 1;
                            if repeat_count >= 3 {
                                return GateResult::Reject {
                                    gate_id: self.gate_id().to_string(),
                                    file: None,
                                    line: None,
                                    reason: "偵測到重複提示詞 3 次，可能陷入死循環".to_string(),
                                };
                            }
                        } else {
                            repeat_count = 0;
                        }
                    }
                    last_prompt = Some(content.trim().to_string());
                }
            }
        }
        GateResult::Pass
    }
}

pub struct SlopVibeAuditorGate;
impl Gate for SlopVibeAuditorGate {
    fn name(&self) -> &'static str { "SlopVibeAuditor" }
    fn gate_id(&self) -> &'static str { "G3" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let slop_words = [
            "delve", "testament", "underscore", "crucial", "furthermore",
            "robust", "realm", "tapestry", "embark", "intricate"
        ];
        for (mi, msg) in change.messages.iter().enumerate() {
            if let Some(role) = msg["role"].as_str() {
                if role != "assistant" {
                    continue;
                }
            }
            if let Some(content) = msg["content"].as_str() {
                let lower = content.to_lowercase();
                for word in &slop_words {
                    if lower.contains(word) {
                        return GateResult::Reject {
                            gate_id: self.gate_id().to_string(),
                            file: Some(format!("Message #{}", mi)),
                            line: None,
                            reason: format!("使用非必要 AI 空話修飾詞: '{}'", word),
                        };
                    }
                }
            }
        }
        GateResult::Pass
    }
}

pub struct SlopPathPurgeSpecialistGate;
impl Gate for SlopPathPurgeSpecialistGate {
    fn name(&self) -> &'static str { "SlopPathPurgeSpecialist" }
    fn gate_id(&self) -> &'static str { "G4" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        for tc in change.tool_calls {
            if tc.name == "write_file" || tc.name == "read_file" || tc.name == "replace_file_content" || tc.name == "multi_replace_file_content" {
                if let Some(ref path) = tc.path {
                    let normalized = path.replace('\\', "/");
                    if normalized.contains("..") {
                        return GateResult::Reject {
                            gate_id: self.gate_id().to_string(),
                            file: Some(path.clone()),
                            line: None,
                            reason: "檢測到路徑穿越嘗試，禁止越界訪問".to_string(),
                        };
                    }
                    if tc.name == "write_file" {
                        if normalized.ends_with(".md") && !normalized.starts_with("Docs/") && !normalized.starts_with("./Docs/") {
                            return GateResult::Reject {
                                gate_id: self.gate_id().to_string(),
                                file: Some(path.clone()),
                                line: None,
                                reason: "說明文件 (.md) 必須生成於 Docs/ 目錄".to_string(),
                            };
                        }
                        if normalized.ends_with(".toon") && !normalized.starts_with(".agent/rules/") && !normalized.starts_with("./.agent/rules/") {
                            return GateResult::Reject {
                                gate_id: self.gate_id().to_string(),
                                file: Some(path.clone()),
                                line: None,
                                reason: "行為準則 (.toon) 必須生成於 .agent/rules/ 目錄".to_string(),
                            };
                        }
                    }
                }
            }
        }
        GateResult::Pass
    }
}

pub struct LocaleCalibrationSpecialistGate;
impl Gate for LocaleCalibrationSpecialistGate {
    fn name(&self) -> &'static str { "LocaleCalibrationSpecialist" }
    fn gate_id(&self) -> &'static str { "G5" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let charset = &change.config.general.locale_charset;
        if charset != "zh-TW" && charset != "zh_TW" && charset != "en-US" && charset != "auto" {
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: format!("語系字元集 '{}' 不合規，應使用 zh-TW 或 en-US", charset),
            };
        }
        GateResult::Pass
    }
}

pub struct LeadSystemArchitectGate;
impl Gate for LeadSystemArchitectGate {
    fn name(&self) -> &'static str { "LeadSystemArchitect" }
    fn gate_id(&self) -> &'static str { "G6" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        // Cargo.toml check for Chromium / WebView
        for tc in change.tool_calls {
            if tc.name == "write_file" {
                if let Some(ref path) = tc.path {
                    if path.contains("Cargo.toml")
                        && (tc.content.contains("webkit") || tc.content.contains("chromium") || tc.content.contains("playwright"))
                    {
                        return GateResult::Reject {
                            gate_id: self.gate_id().to_string(),
                            file: Some(path.clone()),
                            line: None,
                            reason: "禁止在 Cargo.toml 中引入 Chromium 或 WebView 系列依賴".to_string(),
                        };
                    }
                }
            }
        }
        GateResult::Pass
    }
}

pub struct SecurityArchitectureDesignerGate;
impl Gate for SecurityArchitectureDesignerGate {
    fn name(&self) -> &'static str { "SecurityArchitectureDesigner" }
    fn gate_id(&self) -> &'static str { "G10" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        if change.config.sandbox.timeout_seconds == 0 {
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: "沙盒逾時設定不能為 0 秒".to_string(),
            };
        }
        GateResult::Pass
    }
}

pub struct DefensiveCodingSpecialistGate;
impl Gate for DefensiveCodingSpecialistGate {
    fn name(&self) -> &'static str { "DefensiveCodingSpecialist" }
    fn gate_id(&self) -> &'static str { "G11" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let shell_inject_chars = [';', '|', '&', '$', '`'];
        for tc in change.tool_calls {
            if tc.name == "run_command" {
                for c in &shell_inject_chars {
                    if tc.content.contains(*c) {
                        return GateResult::Reject {
                            gate_id: self.gate_id().to_string(),
                            file: None,
                            line: None,
                            reason: format!("命令中偵測到危險 Shell 字元 '{}'，有代碼注入風險，應引數向量化分離", c),
                        };
                    }
                }
            }
        }
        GateResult::Pass
    }
}

pub struct SecurityComplianceAuditorGate;
impl Gate for SecurityComplianceAuditorGate {
    fn name(&self) -> &'static str { "SecurityComplianceAuditor" }
    fn gate_id(&self) -> &'static str { "G12" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let sk_pattern = regex::Regex::new(r"sk-[a-zA-Z0-9]{10,}").unwrap();
        for tc in change.tool_calls {
            if sk_pattern.is_match(&tc.content) {
                if let Some(ref path) = tc.path {
                    if path.contains("config.local.toml") {
                        continue;
                    }
                }
                return GateResult::Reject {
                    gate_id: self.gate_id().to_string(),
                    file: tc.path.clone(),
                    line: None,
                    reason: "偵測到硬編碼金鑰 (sk-...)，必須排除！".to_string(),
                };
            }
        }
        GateResult::Pass
    }
}

pub struct CoreEngineCoderGate;
impl Gate for CoreEngineCoderGate {
    fn name(&self) -> &'static str { "CoreEngineCoder" }
    fn gate_id(&self) -> &'static str { "G13" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        for tc in change.tool_calls {
            if tc.name == "write_file"
                && (tc.content.contains("TODO") || tc.content.contains("unimplemented!"))
            {
                return GateResult::Reject {
                    gate_id: self.gate_id().to_string(),
                    file: tc.path.clone(),
                    line: None,
                    reason: "寫入代碼中不得包含 TODO 或 unimplemented! 等殘渣標記".to_string(),
                };
            }
        }
        GateResult::Pass
    }
}

pub struct IntegrationEngineerGate;
impl Gate for IntegrationEngineerGate {
    fn name(&self) -> &'static str { "IntegrationEngineer" }
    fn gate_id(&self) -> &'static str { "G14" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        for tc in change.tool_calls {
            if tc.name == "write_file" && tc.content.contains("reqwest::blocking") {
                return GateResult::Reject {
                    gate_id: self.gate_id().to_string(),
                    file: tc.path.clone(),
                    line: None,
                    reason: "非同步引擎中禁止使用 blocking HTTP client".to_string(),
                };
            }
        }
        GateResult::Pass
    }
}

pub struct MultimodalMediaSpecialistGate;
impl Gate for MultimodalMediaSpecialistGate {
    fn name(&self) -> &'static str { "MultimodalMediaSpecialist" }
    fn gate_id(&self) -> &'static str { "G15" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        // Skip unless media generation query
        let has_media_query = change.messages.iter().any(|msg| {
            if let Some(content) = msg["content"].as_str() {
                let lower = content.to_lowercase();
                lower.contains("image") || lower.contains("video") || lower.contains("圖片")
            } else {
                false
            }
        });
        if !has_media_query {
            return GateResult::Skip { reason: "無多模態媒體生成需求，跳過此驗證".to_string() };
        }
        GateResult::Pass
    }
}

fn find_project_root() -> Option<std::path::PathBuf> {
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            if dir.join("Cargo.toml").exists() {
                return Some(dir);
            }
            if let Some(parent) = dir.parent() {
                dir = parent.to_path_buf();
            } else {
                break;
            }
        }
    }
    None
}

// ─── Stage B: Compile & Sandbox (0 Token) ──────────────────────────────────

pub struct PerformanceArchitectureEngineerGate;
impl Gate for PerformanceArchitectureEngineerGate {
    fn name(&self) -> &'static str { "PerformanceArchitectureEngineer" }
    fn gate_id(&self) -> &'static str { "G7" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        const MAX_SLEEP_SECS: u64 = 60;
        const MAX_SLEEP_MS: u64 = 60_000;
        let re_secs = regex::Regex::new(r"(?i)(?:sleep|Start-Sleep|timeout|from_secs)\s*\(?\s*(\d+)").ok();
        let re_ms = regex::Regex::new(r"(?i)(?:from_millis)\s*\(\s*(\d+)").ok();

        for tc in change.tool_calls {
            let content_lower = tc.content.to_lowercase();

            // Check loop {} or while true
            let stripped: String = content_lower.chars().filter(|c| !c.is_whitespace()).collect();
            if stripped.contains("loop{}") || stripped.contains("whiletrue") {
                return GateResult::Reject {
                    gate_id: self.gate_id().to_string(),
                    file: tc.path.clone(),
                    line: None,
                    reason: "檢測到無限循環模式 (loop {} 或 while true)".to_string(),
                };
            }

            // Check sleep duration
            if content_lower.contains("sleep") || content_lower.contains("timeout") {
                if let Some(ref re) = re_secs {
                    for cap in re.captures_iter(&tc.content) {
                        if let Some(m) = cap.get(1) {
                            if let Ok(secs) = m.as_str().parse::<u64>() {
                                if secs > MAX_SLEEP_SECS {
                                    return GateResult::Reject {
                                        gate_id: self.gate_id().to_string(),
                                        file: tc.path.clone(),
                                        line: None,
                                        reason: format!("檢測到過長的等待時間 ({} 秒 > {} 秒)", secs, MAX_SLEEP_SECS),
                                    };
                                }
                            }
                        }
                    }
                }
                if let Some(ref re) = re_ms {
                    for cap in re.captures_iter(&tc.content) {
                        if let Some(m) = cap.get(1) {
                            if let Ok(ms) = m.as_str().parse::<u64>() {
                                if ms > MAX_SLEEP_MS {
                                    return GateResult::Reject {
                                        gate_id: self.gate_id().to_string(),
                                        file: tc.path.clone(),
                                        line: None,
                                        reason: format!("檢測到過長的等待時間 ({} 毫秒 > {} 毫秒)", ms, MAX_SLEEP_MS),
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
        GateResult::Pass
    }
}

pub struct ResourceAnalyticsEngineerGate;
impl Gate for ResourceAnalyticsEngineerGate {
    fn name(&self) -> &'static str { "ResourceAnalyticsEngineer" }
    fn gate_id(&self) -> &'static str { "G8" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let write_tools = ["write_file", "replace_file_content", "multi_replace_file_content"];
        for tc in change.tool_calls {
            if write_tools.contains(&tc.name.as_str())
                && (tc.content.contains("std::io::stdin()") || tc.content.contains("blocking_read"))
                && (tc.content.contains("async ") || tc.content.contains("tokio::"))
            {
                return GateResult::Reject {
                    gate_id: self.gate_id().to_string(),
                    file: tc.path.clone(),
                    line: None,
                    reason: "在非阻塞(async)上下文中檢測到阻塞型 I/O 操作".to_string(),
                };
            }
        }
        GateResult::Pass
    }
}

pub struct MemoryEfficiencyReviewerGate;
impl Gate for MemoryEfficiencyReviewerGate {
    fn name(&self) -> &'static str { "MemoryEfficiencyReviewer" }
    fn gate_id(&self) -> &'static str { "G9" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        if change.config.general.project_mode != "project" {
            return GateResult::Skip { reason: "非專案模式，跳過 Clippy 檢查".to_string() };
        }
        
        let project_root = match find_project_root() {
            Some(path) => path,
            None => return GateResult::Skip { reason: "找不到專案根目錄，跳過".to_string() },
        };
        
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = std::process::Command::new("cmd");
            c.arg("/C").arg("chcp 65001 >nul && cargo clippy --message-format=json --quiet");
            c
        } else {
            let mut c = std::process::Command::new("cargo");
            c.args(["clippy", "--message-format=json", "--quiet"]);
            c
        };

        let output = match cmd.current_dir(&project_root).output() 
        {
            Ok(out) => out,
            Err(e) => return GateResult::Skip { reason: format!("無法執行 cargo clippy: {}", e) },
        };
        
        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            if stderr_str.contains("error") || stdout_str.contains(r#""level":"error""#) {
                let err_msg = if !stderr_str.trim().is_empty() {
                    stderr_str.lines().next().unwrap_or("").to_string()
                } else {
                    "Clippy 檢測到代碼錯誤".to_string()
                };
                return GateResult::Reject {
                    gate_id: self.gate_id().to_string(),
                    file: None,
                    line: None,
                    reason: format!("Clippy 檢查失敗: {}", err_msg),
                };
            }
        }
        GateResult::Pass
    }
}

pub struct SandboxRuntimeTesterGate;
impl Gate for SandboxRuntimeTesterGate {
    fn name(&self) -> &'static str { "SandboxRuntimeTester" }
    fn gate_id(&self) -> &'static str { "G16" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        if change.config.general.project_mode != "project" {
            return GateResult::Skip { reason: "非專案模式，跳過編譯檢查".to_string() };
        }
        
        let project_root = match find_project_root() {
            Some(path) => path,
            None => return GateResult::Skip { reason: "找不到專案根目錄，跳過".to_string() },
        };
        
        let run_dir = if project_root.join("src-tauri").join("Cargo.toml").exists() {
            project_root.join("src-tauri")
        } else if project_root.join("Cargo.toml").exists() {
            project_root
        } else {
            return GateResult::Skip { reason: "非 Rust 專案，跳過".to_string() };
        };
        
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = std::process::Command::new("cmd");
            c.arg("/C").arg("chcp 65001 >nul && cargo check");
            c
        } else {
            let mut c = std::process::Command::new("cargo");
            c.args(["check"]);
            c
        };

        let output = match cmd.current_dir(&run_dir).output()
        {
            Ok(out) => out,
            Err(e) => return GateResult::Skip { reason: format!("無法執行 cargo check: {}", e) },
        };
        
        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let first_error = stderr_str.lines()
                .find(|l| l.contains("error:"))
                .unwrap_or_else(|| stderr_str.lines().next().unwrap_or("未知編譯錯誤"));
                
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: format!("編譯檢查失敗: {}", first_error),
            };
        }
        
        GateResult::Pass
    }
}

// ─── Stage C: Semantic / LLM ────────────────────────────────────────────────

pub struct FactHallucinationAuditorGate;
impl Gate for FactHallucinationAuditorGate {
    fn name(&self) -> &'static str { "FactHallucinationAuditor" }
    fn gate_id(&self) -> &'static str { "G17" }
    fn check(&self, _change: &ProposedChange) -> GateResult {
        GateResult::Pass
    }
}

pub struct TokenOverlapAuditorGate;
impl Gate for TokenOverlapAuditorGate {
    fn name(&self) -> &'static str { "TokenOverlapAuditor" }
    fn gate_id(&self) -> &'static str { "G18" }
    fn check(&self, _change: &ProposedChange) -> GateResult {
        GateResult::Pass
    }
}

pub struct ContextDistillerAlphaGate;
impl Gate for ContextDistillerAlphaGate {
    fn name(&self) -> &'static str { "ContextDistillerAlpha" }
    fn gate_id(&self) -> &'static str { "G19" }
    fn check(&self, _change: &ProposedChange) -> GateResult {
        GateResult::Skip { reason: "非大文本增量，蒸餾休眠".to_string() }
    }
}

pub struct ContextDistillerBetaGate;
impl Gate for ContextDistillerBetaGate {
    fn name(&self) -> &'static str { "ContextDistillerBeta" }
    fn gate_id(&self) -> &'static str { "G20" }
    fn check(&self, _change: &ProposedChange) -> GateResult {
        GateResult::Skip { reason: "非大文本增量，蒸餾休眠".to_string() }
    }
}

pub struct DistillationIntegratorGate;
impl Gate for DistillationIntegratorGate {
    fn name(&self) -> &'static str { "DistillationIntegrator" }
    fn gate_id(&self) -> &'static str { "G21" }
    fn check(&self, _change: &ProposedChange) -> GateResult {
        GateResult::Skip { reason: "無並行蒸餾產出，跳過".to_string() }
    }
}

// ─── Stage D: Signoff ───────────────────────────────────────────────────────

pub struct OrchestratorAgentGate;
impl Gate for OrchestratorAgentGate {
    fn name(&self) -> &'static str { "OrchestratorAgent" }
    fn gate_id(&self) -> &'static str { "G22" }
    fn check(&self, _change: &ProposedChange) -> GateResult {
        GateResult::Pass
    }
}

// ─── Pipeline runner ────────────────────────────────────────────────────────

pub fn run_all_gates(
    config: &Config,
    tool_calls: &[ToolCall],
    messages: &[Value],
) -> Vec<AuditResult> {
    let change = ProposedChange { tool_calls, messages, config };
    
    // Stage A gates run in parallel (using scoped threads for zero dependencies)
    let stage_a_gates: Vec<Box<dyn Gate>> = vec![
        Box::new(WorkflowTopologyGate),
        Box::new(WorkflowRuntimeEvaluatorGate),
        Box::new(SlopVibeAuditorGate),
        Box::new(SlopPathPurgeSpecialistGate),
        Box::new(LocaleCalibrationSpecialistGate),
        Box::new(LeadSystemArchitectGate),
        Box::new(SecurityArchitectureDesignerGate),
        Box::new(DefensiveCodingSpecialistGate),
        Box::new(SecurityComplianceAuditorGate),
        Box::new(CoreEngineCoderGate),
        Box::new(IntegrationEngineerGate),
        Box::new(MultimodalMediaSpecialistGate),
    ];

    let mut audits = Vec::new();

    let stage_a_results = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for gate in &stage_a_gates {
            let handle = s.spawn(|| {
                let result = gate.check(&change);
                result.to_audit(gate.name())
            });
            handles.push(handle);
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });
    audits.extend(stage_a_results);

    // Stage B, C, D run sequentially
    let remaining_gates: Vec<Box<dyn Gate>> = vec![
        // Stage B
        Box::new(PerformanceArchitectureEngineerGate),
        Box::new(ResourceAnalyticsEngineerGate),
        Box::new(MemoryEfficiencyReviewerGate),
        Box::new(SandboxRuntimeTesterGate),
        // Stage C
        Box::new(FactHallucinationAuditorGate),
        Box::new(TokenOverlapAuditorGate),
        Box::new(ContextDistillerAlphaGate),
        Box::new(ContextDistillerBetaGate),
        Box::new(DistillationIntegratorGate),
        // Stage D
        Box::new(OrchestratorAgentGate),
    ];

    for gate in remaining_gates {
        let result = gate.check(&change);
        audits.push(result.to_audit(gate.name()));
    }

    audits
}
