use crate::agent::{AgentEngine, AuditResult, ToolCall};
use crate::config::Config;
use crate::db;
use crate::sandbox;
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────────────────────
// Role definitions — 22 agents mapped to 6 groups with dependency ordering
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgent {
    pub role: String,
    pub group: String,
    pub priority: u8,
    pub prerequisites: Vec<String>,
    pub produces_output: bool,
}

impl SubAgent {
    pub(crate) fn all_agents() -> Vec<Self> {
        vec![
            // Group 1: Meta-workflow (priority 0-3)
            SubAgent { role: "WorkflowArchitectureOptimizer".into(), group: "Meta-Workflow".into(), priority: 0, prerequisites: vec![], produces_output: false },
            SubAgent { role: "WorkflowRuntimeEvaluator".into(), group: "Meta-Workflow".into(), priority: 1, prerequisites: vec!["WorkflowArchitectureOptimizer".into()], produces_output: false },
            SubAgent { role: "SlopVibeAuditor".into(), group: "Meta-Workflow".into(), priority: 2, prerequisites: vec!["WorkflowRuntimeEvaluator".into()], produces_output: false },
            SubAgent { role: "SlopPathPurgeSpecialist".into(), group: "Meta-Workflow".into(), priority: 3, prerequisites: vec!["WorkflowArchitectureOptimizer".into()], produces_output: false },
            // Group 2: Management (priority 4-6)
            SubAgent { role: "OrchestratorAgent".into(), group: "Management".into(), priority: 4, prerequisites: vec!["WorkflowArchitectureOptimizer".into()], produces_output: false },
            SubAgent { role: "LocaleCalibrationSpecialist".into(), group: "Management".into(), priority: 5, prerequisites: vec![], produces_output: false },
            SubAgent { role: "LeadSystemArchitect".into(), group: "Management".into(), priority: 6, prerequisites: vec!["WorkflowArchitectureOptimizer".into(), "SlopPathPurgeSpecialist".into()], produces_output: false },
            // Group 3: Performance (priority 7-9)
            SubAgent { role: "PerformanceArchitectureEngineer".into(), group: "Performance".into(), priority: 7, prerequisites: vec!["SlopVibeAuditor".into()], produces_output: false },
            SubAgent { role: "PerformanceAnalyticsEngineer".into(), group: "Performance".into(), priority: 8, prerequisites: vec!["PerformanceArchitectureEngineer".into()], produces_output: false },
            SubAgent { role: "PerformanceEfficiencyReviewer".into(), group: "Performance".into(), priority: 9, prerequisites: vec!["PerformanceAnalyticsEngineer".into()], produces_output: false },
            // Group 4: Security (priority 10-12)
            SubAgent { role: "SecurityArchitectureDesigner".into(), group: "Security".into(), priority: 10, prerequisites: vec!["LeadSystemArchitect".into()], produces_output: false },
            SubAgent { role: "DefensiveCodingSpecialist".into(), group: "Security".into(), priority: 11, prerequisites: vec!["PerformanceEfficiencyReviewer".into()], produces_output: false },
            SubAgent { role: "SecurityComplianceAuditor".into(), group: "Security".into(), priority: 12, prerequisites: vec!["SecurityArchitectureDesigner".into(), "DefensiveCodingSpecialist".into()], produces_output: false },
            // Group 5: Engineering (priority 13-16)
            SubAgent { role: "CoreEngineCoder".into(), group: "Engineering".into(), priority: 13, prerequisites: vec!["SlopPathPurgeSpecialist".into(), "PerformanceEfficiencyReviewer".into()], produces_output: true },
            SubAgent { role: "IntegrationEngineer".into(), group: "Engineering".into(), priority: 14, prerequisites: vec!["SecurityComplianceAuditor".into()], produces_output: true },
            SubAgent { role: "MultimodalMediaSpecialist".into(), group: "Engineering".into(), priority: 15, prerequisites: vec!["CoreEngineCoder".into(), "IntegrationEngineer".into()], produces_output: true },
            SubAgent { role: "SandboxRuntimeTester".into(), group: "Engineering".into(), priority: 16, prerequisites: vec!["IntegrationEngineer".into()], produces_output: false },
            // Group 6: Memory Distillation (priority 17-21)
            SubAgent { role: "ContextDistillerAlpha".into(), group: "Memory-Distillation".into(), priority: 17, prerequisites: vec![], produces_output: false },
            SubAgent { role: "ContextDistillerBeta".into(), group: "Memory-Distillation".into(), priority: 18, prerequisites: vec![], produces_output: false },
            SubAgent { role: "DistillationIntegrator".into(), group: "Memory-Distillation".into(), priority: 19, prerequisites: vec!["ContextDistillerAlpha".into(), "ContextDistillerBeta".into()], produces_output: false },
            SubAgent { role: "FactHallucinationAuditor".into(), group: "Memory-Distillation".into(), priority: 20, prerequisites: vec!["DistillationIntegrator".into()], produces_output: false },
            SubAgent { role: "TokenOverlapAuditor".into(), group: "Memory-Distillation".into(), priority: 21, prerequisites: vec!["DistillationIntegrator".into()], produces_output: false },
        ]
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// PendingAction — user approval gate
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub agent_role: String,
    pub action_type: String,
    pub target_path: String,
    pub description: String,
    pub risk: ActionRiskLevel,
    pub preview: String,
    pub created_at: String,
    pub approved: bool,
    pub rejected: bool,
    pub rejection_reason: String,
}

impl PendingAction {
    fn new(agent_role: &str, tool: &ToolCall) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            agent_role: agent_role.to_string(),
            action_type: tool.name.clone(),
            target_path: tool.path.clone().unwrap_or_else(|| "N/A".to_string()),
            description: Self::classify_action(&tool.name, &tool.path, &tool.content),
            risk: Self::classify_risk(&tool.name, &tool.content),
            preview: tool.content.chars().take(200).collect(),
            created_at: Utc::now().to_rfc3339(),
            approved: false,
            rejected: false,
            rejection_reason: String::new(),
        }
    }

    fn classify_action(name: &str, path: &Option<String>, content: &str) -> String {
        match name {
            "write_file" => format!("Write file: {}", path.as_deref().unwrap_or("unknown")),
            "read_file" => format!("Read file: {}", path.as_deref().unwrap_or("unknown")),
            "run_command" => {
                let max = content.len().min(80);
                let short = &content[..max];
                format!("Run command: {}", short)
            }
            other => format!("Tool call: {}", other),
        }
    }

    fn classify_risk(name: &str, content: &str) -> ActionRiskLevel {
        if name == "run_command" {
            let upper = content.to_uppercase();
            if upper.contains("RM ") || upper.contains("DEL ") || upper.contains("SHUTDOWN") || upper.contains("FORMAT") || upper.contains("DISKPART") {
                return ActionRiskLevel::Critical;
            }
            return ActionRiskLevel::High;
        }
        if name == "write_file" {
            return ActionRiskLevel::Medium;
        }
        ActionRiskLevel::Low
    }

    // ── Helper methods for approval workflow ──

    pub fn is_approved(&self) -> bool {
        self.approved
    }

    pub fn is_rejected(&self) -> bool {
        self.rejected
    }

    pub fn is_pending(&self) -> bool {
        !self.approved && !self.rejected
    }

    pub fn approve(&mut self) {
        self.approved = true;
    }

    pub fn reject(&mut self, reason: String) {
        self.rejected = true;
        self.rejection_reason = reason;
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// ConfirmationGate — gate that requires user approval before execution
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationGate {
    pub gate_id: String,
    pub pending_actions: Vec<PendingAction>,
    pub all_audits_passed: bool,
    pub rejection_details: String,
}

impl ConfirmationGate {
    #[allow(dead_code)]
    fn new(audits: &[AuditResult], tool_calls: &[ToolCall], calling_agent: &str) -> Self {
        let all_passed = !AgentEngine::any_rejected(audits);
        let rejection = if !all_passed {
            AgentEngine::rejection_details(audits)
        } else {
            String::new()
        };

        let mut pending = Vec::new();
        if all_passed && !tool_calls.is_empty() {
            for tc in tool_calls {
                if tc.name != "read_file" || tc.content.len() > 10 {
                    pending.push(PendingAction::new(calling_agent, tc));
                }
            }
        }

        Self {
            gate_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            pending_actions: pending,
            all_audits_passed: all_passed,
            rejection_details: rejection,
        }
    }

    pub fn needs_approval(&self) -> bool {
        !self.all_audits_passed || !self.pending_actions.is_empty()
    }

    pub fn high_risk_actions(&self) -> Vec<&PendingAction> {
        self.pending_actions.iter()
            .filter(|a| matches!(a.risk, ActionRiskLevel::High | ActionRiskLevel::Critical))
            .collect()
    }

    pub fn summary(&self) -> String {
        if !self.all_audits_passed {
            return format!("[AUDIT REJECTED]\n{}", self.rejection_details);
        }
        let low = self.pending_actions.iter().filter(|a| matches!(a.risk, ActionRiskLevel::Low)).count();
        let med = self.pending_actions.iter().filter(|a| matches!(a.risk, ActionRiskLevel::Medium)).count();
        let hi = self.pending_actions.iter().filter(|a| matches!(a.risk, ActionRiskLevel::High)).count();
        let crit = self.pending_actions.iter().filter(|a| matches!(a.risk, ActionRiskLevel::Critical)).count();
        format!(
            "Pending: {} low, {} medium, {} high, {} critical ({} total)",
            low, med, hi, crit, self.pending_actions.len()
        )
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Orchestrator — multi-subagent dispatcher + execution engine
// ──────────────────────────────────────────────────────────────────────────────

pub struct Orchestrator {
    pub config: Config,
    pub workspace_folders: Vec<PathBuf>,
}

impl Orchestrator {
    pub fn new(config: Config) -> Self {
        Self { config, workspace_folders: vec![PathBuf::new()] }
    }

    pub fn set_workspaces(&mut self, folders: Vec<PathBuf>) {
        self.workspace_folders = folders;
    }

    // ── Core: Dispatch subagents in dependency order ──

    pub fn dispatch_subagents(
        &self,
        _conn: &Connection,
        tool_calls: &[ToolCall],
        messages: &[serde_json::Value],
    ) -> (Vec<AuditResult>, Vec<PendingAction>) {
        let agents = SubAgent::all_agents();
        let mut all_audits: Vec<AuditResult> = Vec::with_capacity(22);
        let mut pending: Vec<PendingAction> = Vec::new();

        // Topological sort: run agents in priority order
        let mut executed = HashMap::new();
        let mut max_iterations = 50;

        while executed.len() < agents.len() && max_iterations > 0 {
            max_iterations -= 1;
            let mut made_progress = false;

            for agent in &agents {
                if executed.contains_key(&agent.role) {
                    continue;
                }
                let prereqs_met = agent.prerequisites.iter().all(|p| executed.contains_key(p));
                if !prereqs_met {
                    continue;
                }

                // Run all 17 validations, pick this agent's result
                let audits = AgentEngine.run_validation(&self.config, tool_calls, messages);
                for audit in &audits {
                    if audit.agent_name == agent.role {
                        all_audits.push(audit.clone());
                        executed.insert(agent.role.clone(), audit.verdict.clone());
                        made_progress = true;
                    }
                }
            }

            if !made_progress {
                break;
            }
        }

        // Auto-skip agents whose prereqs failed
        let agent_roles: Vec<&str> = agents.iter().map(|a| a.role.as_str()).collect();
        let already_done: Vec<String> = all_audits.iter().map(|a| a.agent_name.clone()).collect();
        for role in agent_roles {
            if !already_done.contains(&role.to_string()) {
                all_audits.push(AuditResult {
                    agent_name: role.to_string(),
                    verdict: "SKIPPED (prerequisite failed)".into(),
                    reason: "前置角色驗證未通過，此角色跳過。".into(),
                });
            }
        }

        // Build pending actions
        if !AgentEngine::any_rejected(&all_audits) && !tool_calls.is_empty() {
            for tc in tool_calls {
                pending.push(PendingAction::new("OrchestratorAgent", tc));
            }
        }

        (all_audits, pending)
    }

    // ── Execute task with healing loop ──

    pub fn execute_task_with_healing(
        &self,
        conn: &Connection,
        task_id: &str,
        program: &str,
        args: &[&str],
        workspace: Option<&PathBuf>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        db::update_task_status(conn, task_id, "IN_PROGRESS")?;

        let max_retries = self.config.sandbox.max_retries;
        let mut attempts = 0u32;
        let mut success = false;
        let command_str = format!("{} {}", program, args.join(" "));

        while attempts < max_retries && !success {
            attempts += 1;
            println!("[ORCHESTRATOR] Executing: ID={}, {}/{}", task_id, attempts, max_retries);

            let result = sandbox::run_in_sandbox(
                program, args,
                &self.config.general.shell,
                self.config.security.full_access,
                workspace,
            );

            db::add_execution_log(conn, task_id, &command_str, &result.stdout, &result.stderr, result.exit_code)?;

            if result.is_aligned_success {
                success = true;
                println!("[PASS] Task succeeded: ID={}", task_id);
                db::update_task_status(conn, task_id, "SUCCESS")?;
            } else {
                println!("[REJECT] Task failed: ID={}, ExitCode={:?}", task_id, result.exit_code);
            }
        }

        if !success {
            db::update_task_status(conn, task_id, "FAILED")?;
        }

        Ok(success)
    }

    // ── Multi-folder execution ──

    pub fn execute_multi_folder(
        &self,
        conn: &Connection,
        task_id: &str,
        program: &str,
        args: &[&str],
    ) -> Result<HashMap<String, bool>, Box<dyn std::error::Error>> {
        let mut results: HashMap<String, bool> = HashMap::new();

        for folder in &self.workspace_folders {
            let folder_name = folder.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            println!("[ORCHESTRATOR] Executing in folder: {}", folder_name);

            match self.execute_task_with_healing(conn, task_id, program, args, Some(folder)) {
                Ok(success) => { results.insert(folder_name, success); }
                Err(e) => {
                    eprintln!("[ORCHESTRATOR] Error in {}: {}", folder_name, e);
                    results.insert(folder_name, false);
                }
            }
        }

        Ok(results)
    }

    // ── Global mode: computer automation with safety gate chain ──

    pub fn global_execute(
        &self,
        conn: &Connection,
        task_id: &str,
        prompt: &str,
        tool_calls: &[ToolCall],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut log: Vec<String> = Vec::new();
        log.push(format!("[GLOBAL MODE] Task: {}", prompt));

        // Gate 1: Destructive pattern check
        let blocked = self.check_destructive_patterns(tool_calls);
        if !blocked.is_empty() {
            log.push("[GATE 1 BLOCKED] Destructive patterns:".into());
            for b in &blocked { log.push(format!("  {}", b)); }
            db::update_task_status(conn, task_id, "FAILED")?;
            return Ok(log);
        }

        // Gate 2: Path scope verification
        let blocked_paths = self.verify_global_paths(tool_calls);
        if !blocked_paths.is_empty() {
            log.push("[GATE 2 BLOCKED] Unauthorized paths:".into());
            for b in &blocked_paths { log.push(format!("  {}", b)); }
            db::update_task_status(conn, task_id, "FAILED")?;
            return Ok(log);
        }

        // Gate 3: Subagent dispatch
        let messages: Vec<serde_json::Value> = vec![
            serde_json::json!({"role": "user", "content": prompt}),
        ];
        let (audits, pending) = self.dispatch_subagents(conn, tool_calls, &messages);

        if AgentEngine::any_rejected(&audits) {
            log.push("[GATE 3 BLOCKED] Audit rejection:".into());
            log.push(AgentEngine::rejection_details(&audits));
            db::update_task_status(conn, task_id, "FAILED")?;
            return Ok(log);
        }

        // Gate 4: Execution log
        for action in &pending {
            log.push(format!("[EXEC] {} {} → {}", action.agent_role, action.action_type, action.target_path));
        }

        db::update_task_status(conn, task_id, "SUCCESS")?;
        log.push(format!("[COMPLETE] {} finished. {} actions.", task_id, pending.len()));
        Ok(log)
    }

    fn check_destructive_patterns(&self, tool_calls: &[ToolCall]) -> Vec<String> {
        let mut blocked: Vec<String> = Vec::new();
        for tc in tool_calls {
            let upper = tc.content.to_uppercase();
            let patterns = ["FORMAT ", "DISKPART", "PARTITION", "SHUTDOWN /S", "REBOOT /F", "NET USER /DELETE", "NET LOCALGROUP", "ICACLS", "CACLS /T"];
            for pat in &patterns {
                if upper.contains(pat) {
                    blocked.push(format!("Destructive '{}': {}", pat, &tc.content[..tc.content.len().min(100)]));
                }
            }
        }
        blocked
    }

    fn verify_global_paths(&self, tool_calls: &[ToolCall]) -> Vec<String> {
        let mut blocked: Vec<String> = Vec::new();
        let allowed = self.get_allowed_global_roots();

        for tc in tool_calls {
            if let Some(ref path) = tc.path {
                let normalized = path.replace('\\', "/");
                let mut is_allowed = false;

                for root in &allowed {
                    if normalized.starts_with(root) { is_allowed = true; break; }
                }
                for ws in &self.workspace_folders {
                    if let Ok(canonical) = std::fs::canonicalize(ws) {
                        if normalized.starts_with(canonical.to_string_lossy().as_ref()) { is_allowed = true; break; }
                    }
                }

                if !is_allowed {
                    blocked.push(format!("Not in scope: {}", path));
                }
            }
        }
        blocked
    }

    fn get_allowed_global_roots(&self) -> Vec<String> {
        let mut roots = Vec::new();
        for ws in &self.workspace_folders {
            if let Ok(canonical) = std::fs::canonicalize(ws) {
                roots.push(canonical.to_string_lossy().to_string().to_lowercase());
            }
        }
        if let Ok(home) = std::env::var("HOME") { roots.push(home.to_lowercase()); }
        if let Ok(home) = std::env::var("USERPROFILE") { roots.push(home.to_lowercase()); }
        roots.push(r"c:\users".to_lowercase());
        roots.push(r"c:\program files".to_lowercase());
        roots.push(r"c:\program files (x86)".to_lowercase());
        roots.push(r"c:\programdata".to_lowercase());
        roots
    }
}
