use crate::agent::{AuditResult, ToolCall};
use crate::config::Config;
use crate::no_window::NoWindowExt;
use serde_json::Value;
use std::collections::HashMap;

/// 休眠裁決字串（panel 以灰點顯示；未激活 = 不思考不輸出零成本）
pub const VERDICT_DORMANT: &str = "DORMANT";

// ─── 代理人分工路由（core.agents.toon / memory_distillation.toon）────────────
// 按任務特徵決定誰激活、誰休眠。確定性規則，0 token。
// 回傳：休眠代理 → 休眠原因。不在表中 = 激活。

/// 任務特徵快照（一次掃描，所有路由規則共用）
struct TaskTraits {
    has_write: bool,
    has_rs_write: bool,
    has_cmd: bool,
    has_media: bool,
    conv_tokens: usize,
}

const WRITE_TOOLS: [&str; 3] = ["write_file", "replace_file_content", "multi_replace_file_content"];

fn scan_traits(tool_calls: &[ToolCall], messages: &[Value]) -> TaskTraits {
    let has_write = tool_calls.iter().any(|tc| WRITE_TOOLS.contains(&tc.name.as_str()));
    let has_rs_write = tool_calls.iter().any(|tc| {
        WRITE_TOOLS.contains(&tc.name.as_str())
            && tc.path.as_deref().is_some_and(|p| p.ends_with(".rs"))
    });
    let has_cmd = tool_calls.iter().any(|tc| tc.name == "run_command" || tc.name == "run_mcp");
    let has_media = messages.iter().rev().find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .map(|c| {
            let lower = c.to_lowercase();
            lower.contains("image") || lower.contains("video")
                || lower.contains("圖片") || lower.contains("影片")
        })
        .unwrap_or(false);
    let conv_tokens = messages.iter()
        .filter_map(|m| m["content"].as_str())
        .map(crate::memory::estimate_tokens)
        .sum();
    TaskTraits { has_write, has_rs_write, has_cmd, has_media, conv_tokens }
}

/// 路由演算法：回傳「休眠代理 → 原因」。
pub fn route_dormant_agents(
    config: &Config,
    tool_calls: &[ToolCall],
    messages: &[Value],
) -> HashMap<&'static str, String> {
    let t = scan_traits(tool_calls, messages);
    let mut dormant = HashMap::new();

    // 寫檔審查組：無檔案寫入即休眠
    if !t.has_write {
        for agent in [
            "SlopPathPurgeSpecialist",   // G4 路徑分流
            "LeadSystemArchitect",       // G6 依賴白名單
            "ResourceAnalyticsEngineer", // G8 阻塞 I/O
            "SecurityComplianceAuditor", // G12 金鑰掃描
            "CoreEngineCoder",           // G13 殘渣標記
            "IntegrationEngineer",       // G14 blocking HTTP
        ] {
            dormant.insert(agent, "休眠：本步驟無檔案寫入".to_string());
        }
    }

    // 指令執行審查組：無指令亦無寫檔即休眠
    if !t.has_cmd && !t.has_write {
        for agent in [
            "PerformanceArchitectureEngineer", // G7 無限循環/超長等待
            "SecurityArchitectureDesigner",    // G10 沙盒參數
            "DefensiveCodingSpecialist",       // G11 Shell 注入
        ] {
            dormant.insert(agent, "休眠：本步驟無指令執行".to_string());
        }
    }

    // 編譯級審查組（重量級）：僅 Rust 代碼寫入時激活——純聊天不再跑 clippy/cargo check
    if !t.has_rs_write {
        dormant.insert("MemoryEfficiencyReviewer", "休眠：無 Rust 代碼寫入，不需 Clippy".to_string());
        dormant.insert("SandboxRuntimeTester", "休眠：無 Rust 代碼寫入，不需編譯檢查".to_string());
    }

    // 多模態：無媒體需求即休眠
    if !t.has_media {
        dormant.insert("MultimodalMediaSpecialist", "休眠：無多模態媒體需求".to_string());
    }

    // 記憶蒸餾組：對話量未達水位即整組休眠（memory_distillation.toon 激活條件）
    let delta = config.memory.distill_trigger_delta;
    if t.conv_tokens < delta {
        let reason = format!("休眠：對話 {} tokens < 蒸餾水位 {}", t.conv_tokens, delta);
        dormant.insert("ContextDistillerAlpha", reason.clone());
        dormant.insert("ContextDistillerBeta", reason.clone());
        dormant.insert("DistillationIntegrator", reason);
    }

    // 恆常激活：WorkflowTopology、WorkflowRuntimeEvaluator、SlopVibeAuditor、
    // LocaleCalibrationSpecialist、FactHallucinationAuditor、TokenOverlapAuditor、OrchestratorAgent
    dormant
}

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
        // 標準 AI 空話清單（與 .agent/rules/core.agents.toon 的 banned_words 一致，無重複）
        let slop_words = [
            "delve", "testament", "underscore", "crucial", "furthermore",
            "pivotal", "moreover", "robust", "realm", "tapestry", "embark", "intricate",
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

pub struct DestructiveCommandGate;
impl Gate for DestructiveCommandGate {
    fn name(&self) -> &'static str { "DestructiveCommand" }
    fn gate_id(&self) -> &'static str { "D7" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let destructive_patterns = [
            "FORMAT ", "DISKPART", "PARTITION", "SHUTDOWN /S", "REBOOT /F",
            "NET USER /DELETE", "NET LOCALGROUP", "ICACLS", "CACLS /T"
        ];
        for tc in change.tool_calls {
            let content_upper = tc.content.to_uppercase();
            for pattern in &destructive_patterns {
                if content_upper.contains(pattern) {
                    return GateResult::Reject {
                        gate_id: self.gate_id().to_string(),
                        file: tc.path.clone(),
                        line: None,
                        reason: format!("偵測到破壞性指令特徵: '{}'", pattern),
                    };
                }
            }
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
            c.no_window();
            c.arg("/C").arg("chcp 65001 >nul && cargo clippy --message-format=json --quiet");
            c
        } else {
            let mut c = std::process::Command::new("cargo");
            c.args(["clippy", "--message-format=json", "--quiet"]);
            c
        };

        let output = match cmd.current_dir(&project_root).no_window().output()
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
            c.no_window();
            c.arg("/C").arg("chcp 65001 >nul && cargo check");
            c
        } else {
            let mut c = std::process::Command::new("cargo");
            c.args(["check"]);
            c
        };

        let output = match cmd.current_dir(&run_dir).no_window().output()
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

/// G17 防幻覺：助理文字宣稱「已建立/已寫入檔案」但本步驟沒有任何對應工具呼叫
/// → 虛假回報，一票否決。純確定性字串比對，0 token。
pub struct FactHallucinationAuditorGate;
impl Gate for FactHallucinationAuditorGate {
    fn name(&self) -> &'static str { "FactHallucinationAuditor" }
    fn gate_id(&self) -> &'static str { "G17" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let claim_phrases = [
            "已建立檔案", "已寫入檔案", "已成功建立", "已成功寫入", "檔案已建立", "檔案已寫入",
            "i have created the file", "i've created the file",
            "file has been created", "file has been written",
        ];
        let last_assistant = change.messages.iter().rev()
            .find(|m| m["role"] == "assistant")
            .and_then(|m| m["content"].as_str())
            .unwrap_or("");
        let lower = last_assistant.to_lowercase();
        let claims_write = claim_phrases.iter().any(|p| lower.contains(p));
        let has_write_tool = change.tool_calls.iter()
            .any(|tc| WRITE_TOOLS.contains(&tc.name.as_str()));
        if claims_write && !has_write_tool {
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: "宣稱已建立/寫入檔案，但本步驟無任何寫檔工具呼叫——虛假回報".to_string(),
            };
        }
        GateResult::Pass
    }
}

/// G18 Token 上限審計：提議寫入的 .md 記憶/文件檔超過 md_token_cap 一票否決
/// （memory_distillation.toon TokenOverlapAuditor 規則，禁止 LLM 呼叫）。
pub struct TokenOverlapAuditorGate;
impl Gate for TokenOverlapAuditorGate {
    fn name(&self) -> &'static str { "TokenOverlapAuditor" }
    fn gate_id(&self) -> &'static str { "G18" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        let cap = change.config.memory.md_token_cap;
        for tc in change.tool_calls {
            if tc.name == "write_file"
                && tc.path.as_deref().is_some_and(|p| p.ends_with(".md"))
            {
                let tokens = crate::memory::estimate_tokens(&tc.content);
                if tokens > cap {
                    return GateResult::Reject {
                        gate_id: self.gate_id().to_string(),
                        file: tc.path.clone(),
                        line: None,
                        reason: format!(".md 檔 {} tokens 超過 md_token_cap {}，必須分裂", tokens, cap),
                    };
                }
            }
        }
        GateResult::Pass
    }
}

/// G19/G20 蒸餾器：路由層判定對話量達水位才激活；激活即代表蒸餾排程
/// 已由 run_step 記憶管線（distill_text + 水位記號）承接，此處驗證前置條件。
pub struct ContextDistillerAlphaGate;
impl Gate for ContextDistillerAlphaGate {
    fn name(&self) -> &'static str { "ContextDistillerAlpha" }
    fn gate_id(&self) -> &'static str { "G19" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        if change.config.memory.chunk_size == 0 || change.config.memory.overlap_lines == 0 {
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: "蒸餾已激活但 chunk_size/overlap_lines 為 0，滑動視窗無法重疊".to_string(),
            };
        }
        GateResult::Pass
    }
}

pub struct ContextDistillerBetaGate;
impl Gate for ContextDistillerBetaGate {
    fn name(&self) -> &'static str { "ContextDistillerBeta" }
    fn gate_id(&self) -> &'static str { "G20" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        // Beta 與 Alpha 並行、共用同一組視窗參數；上限關係錯置即否決
        if change.config.memory.overlap_lines * 2 >= change.config.memory.chunk_size {
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: "overlap_lines*2 >= chunk_size，重疊區吞掉整個視窗".to_string(),
            };
        }
        GateResult::Pass
    }
}

pub struct DistillationIntegratorGate;
impl Gate for DistillationIntegratorGate {
    fn name(&self) -> &'static str { "DistillationIntegrator" }
    fn gate_id(&self) -> &'static str { "G21" }
    fn check(&self, change: &ProposedChange) -> GateResult {
        // 整合器產出受 md_token_cap 約束；cap 為 0 時所有產出必被否決 → 組態矛盾
        if change.config.memory.md_token_cap == 0 {
            return GateResult::Reject {
                gate_id: self.gate_id().to_string(),
                file: None,
                line: None,
                reason: "md_token_cap 為 0，蒸餾產出無法落地".to_string(),
            };
        }
        GateResult::Pass
    }
}

// ─── Pipeline runner ────────────────────────────────────────────────────────

/// 22 代理人分工執行器：
/// 1. 路由演算法決定激活集合（休眠者直接記 DORMANT，不執行、零成本）
/// 2. Stage A（靜態確定性）激活閘門並行執行
/// 3. Stage B/C（編譯期/語意）激活閘門依序執行
/// 4. Stage D：OrchestratorAgent 總和簽核——彙整前 21 道裁決，任一 REJECT 即整案 REJECT
pub fn run_all_gates(
    config: &Config,
    tool_calls: &[ToolCall],
    messages: &[Value],
) -> Vec<AuditResult> {
    let change = ProposedChange { tool_calls, messages, config };
    let dormant = route_dormant_agents(config, tool_calls, messages);

    let run_or_dormant = |gate: &dyn Gate| -> AuditResult {
        if let Some(reason) = dormant.get(gate.name()) {
            return AuditResult {
                agent_name: gate.name().to_string(),
                verdict: VERDICT_DORMANT.to_string(),
                reason: reason.clone(),
            };
        }
        gate.check(&change).to_audit(gate.name())
    };

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
        Box::new(DestructiveCommandGate),
    ];

    let mut audits = Vec::new();

    let stage_a_results = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for gate in &stage_a_gates {
            let handle = s.spawn(|| run_or_dormant(gate.as_ref()));
            handles.push(handle);
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });
    audits.extend(stage_a_results);

    // Stage B, C run sequentially
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
    ];

    for gate in remaining_gates {
        audits.push(run_or_dormant(gate.as_ref()));
    }

    // Stage D：總和簽核（看得到前 21 道的真實裁決，不再是恆 PASS 空殼）
    let rejected: Vec<&AuditResult> = audits.iter().filter(|a| a.verdict == "REJECTED").collect();
    let active_count = audits.iter().filter(|a| a.verdict != VERDICT_DORMANT).count();
    let signoff = if rejected.is_empty() {
        AuditResult {
            agent_name: "OrchestratorAgent".to_string(),
            verdict: "PASSED".to_string(),
            reason: format!(
                "整合審查完成：{} 道激活全數通過，{} 道按任務路由休眠",
                active_count,
                audits.len() - active_count,
            ),
        }
    } else {
        AuditResult {
            agent_name: "OrchestratorAgent".to_string(),
            verdict: "REJECTED".to_string(),
            reason: format!(
                "[REJECT: G22 | 原因: {} 道審查未過（{}）]",
                rejected.len(),
                rejected.iter().map(|a| a.agent_name.as_str()).collect::<Vec<_>>().join("、"),
            ),
        }
    };
    audits.push(signoff);

    audits
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    use crate::agent::ToolCall;

    fn chat_messages(user: &str, assistant: &str) -> Vec<Value> {
        vec![
            serde_json::json!({"role": "user", "content": user}),
            serde_json::json!({"role": "assistant", "content": assistant}),
        ]
    }

    #[test]
    fn pure_chat_puts_heavy_gates_dormant() {
        let config = Config::default();
        let messages = chat_messages("你好，介紹一下這個專案", "這是 Agnes AI。");
        let audits = run_all_gates(&config, &[], &messages);
        assert_eq!(audits.len(), 23);
        for name in ["MemoryEfficiencyReviewer", "SandboxRuntimeTester",
                     "SecurityComplianceAuditor", "DefensiveCodingSpecialist"] {
            let a = audits.iter().find(|a| a.agent_name == name).unwrap();
            assert_eq!(a.verdict, VERDICT_DORMANT, "{} 應休眠", name);
        }
        let g22 = audits.iter().find(|a| a.agent_name == "OrchestratorAgent").unwrap();
        assert_eq!(g22.verdict, "PASSED");
        assert!(g22.reason.contains("休眠"));
    }

    #[test]
    fn rs_write_activates_compile_gates() {
        let mut config = Config::default();
        config.general.project_mode = "global".into(); // 閘門內部 Skip，避免測試真跑 cargo
        let tool_calls = vec![ToolCall {
            name: "write_file".into(),
            path: Some("src/lib.rs".into()),
            content: "pub fn f() {}".into(),
        }];
        let messages = chat_messages("寫一個函式", "好的");
        let audits = run_all_gates(&config, &tool_calls, &messages);
        for name in ["MemoryEfficiencyReviewer", "SandboxRuntimeTester"] {
            let a = audits.iter().find(|a| a.agent_name == name).unwrap();
            assert_ne!(a.verdict, VERDICT_DORMANT, "{} 應激活", name);
        }
    }

    #[test]
    fn g17_rejects_claim_without_write_tool() {
        let config = Config::default();
        let messages = chat_messages("建立 main.rs", "已建立檔案 main.rs，內容如下。");
        let audits = run_all_gates(&config, &[], &messages);
        let g17 = audits.iter().find(|a| a.agent_name == "FactHallucinationAuditor").unwrap();
        assert_eq!(g17.verdict, "REJECTED");
        let g22 = audits.iter().find(|a| a.agent_name == "OrchestratorAgent").unwrap();
        assert_eq!(g22.verdict, "REJECTED");
        assert!(g22.reason.contains("FactHallucinationAuditor"));
    }

    #[test]
    fn g18_rejects_oversized_md() {
        let config = Config::default();
        let big = "記".repeat(config.memory.md_token_cap + 100);
        let tool_calls = vec![ToolCall {
            name: "write_file".into(),
            path: Some("Docs/huge.md".into()),
            content: big,
        }];
        let messages = chat_messages("寫文件", "好的");
        let audits = run_all_gates(&config, &tool_calls, &messages);
        let g18 = audits.iter().find(|a| a.agent_name == "TokenOverlapAuditor").unwrap();
        assert_eq!(g18.verdict, "REJECTED");
        assert!(g18.reason.contains("md_token_cap"));
    }

    #[test]
    fn distill_group_wakes_on_large_conversation() {
        let config = Config::default();
        let long = "上下文".repeat(config.memory.distill_trigger_delta / 2);
        let messages = chat_messages(&long, "收到");
        let audits = run_all_gates(&config, &[], &messages);
        for name in ["ContextDistillerAlpha", "ContextDistillerBeta", "DistillationIntegrator"] {
            let a = audits.iter().find(|a| a.agent_name == name).unwrap();
            assert_eq!(a.verdict, "PASSED", "{} 應激活且通過", name);
        }
    }
}
