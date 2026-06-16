mod config;
mod db;
pub mod diffview;
mod locale;
mod rate_limiter;
mod parallel;
mod sandbox;
mod orchestrator;
mod agent;
mod mcp;
mod memory;
mod multimodal;
mod skills;
mod validation;

#[cfg(feature = "mobile")]
mod mobile;

// UniFFI scaffolding：行動端綁定的進入點（僅 mobile feature 編譯）
#[cfg(feature = "mobile")]
uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests_integration;

pub use config::Config;
pub use config::McpServerConfig;
pub use config::key_persistence;
pub use config::{UI_SCALE_MAX, UI_SCALE_MIN};
pub use db::*;
pub use diffview::*;
pub use locale::*;
pub use sandbox::*;
pub use rate_limiter::RateLimiter;
pub use orchestrator::Orchestrator;
pub use orchestrator::{SubAgent, ConfirmationGate, PendingAction, ActionRiskLevel};
pub use agent::{AgentLoop, ToolCall, AuditResult, AgentStep, PendingState, AgentEngine, split_command_line, check_rs_compiles, run_rs_tests};
pub use mcp::McpManager;
pub use memory::*;
pub use multimodal::{is_visual_intent, MediaResult, MultimodalManager};
pub use skills::{build_skills_system_prompt, load_mcp_json, load_skills, SkillInfo};
pub use validation::*;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use std::time::Instant;

/// Agent execution state machine for non-blocking egui integration.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentExecutionState {
    Idle,
    Running(Instant),
    Complete,
    Error(String),
}

/// Session Token budget and spent accounting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenBudgeter {
    pub session_budget: u64,
    pub spent_prompt: u64,
    pub spent_completion: u64,
}

impl TokenBudgeter {
    pub fn new(session_budget: u64) -> Self {
        Self {
            session_budget,
            spent_prompt: 0,
            spent_completion: 0,
        }
    }

    pub fn record_usage(&mut self, prompt: u64, completion: u64) {
        self.spent_prompt += prompt;
        self.spent_completion += completion;
    }

    pub fn total_spent(&self) -> u64 {
        self.spent_prompt + self.spent_completion
    }

    pub fn budget_ratio(&self) -> f64 {
        if self.session_budget == 0 {
            0.0
        } else {
            self.total_spent() as f64 / self.session_budget as f64
        }
    }

    pub fn is_locked(&self) -> bool {
        self.total_spent() >= self.session_budget
    }
}

/// Shared application state — replaces tauri::State.
///
/// Contains a pooled `reqwest::Client` (shared via `Arc<Mutex<reqwest::Client>>`)
/// so that HTTP connections are reused across all agent API calls.
pub struct AppState {
    pub db_path: PathBuf,
    pub config: Arc<std::sync::Mutex<config::Config>>,
    pub mcp_manager: McpManager,
    pub pending_state: Mutex<Option<agent::PendingState>>,
    pub agent_state: Mutex<AgentExecutionState>,
    pub token_budgeter: Mutex<TokenBudgeter>,
    pub engine_runtime: Runtime,
    /// Pooled reqwest client — shared across all API calls.
    pub http_client: Arc<Mutex<reqwest::Client>>,
    /// App 級共享令牌桶：所有 Agent / 多模態呼叫共用同一 20 RPM 桶，
    /// 即使多資料夾並行或多模態同時觸發也不會突破限速。
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(db_path: PathBuf, config: Arc<std::sync::Mutex<config::Config>>) -> Result<Self, String> {
        let mcp_manager = McpManager::new();
        let engine_runtime = Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        let session_budget = config.lock().unwrap().api.session_budget;
        let token_budgeter = Mutex::new(TokenBudgeter::new(session_budget));

        let max_rpm = config.lock().unwrap().api.max_rpm;
        let rate_limiter = Arc::new(RateLimiter::new(max_rpm));

        let timeout_secs = config.lock().unwrap().sandbox.timeout_seconds;
        let http_client = Arc::new(Mutex::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .user_agent("Agnes-AI/0.8.3")
                .build()
                .map_err(|e| format!("Failed to build pooled HTTP client: {}", e))?,
        ));

        Ok(Self {
            db_path,
            config,
            mcp_manager,
            pending_state: Mutex::new(None),
            agent_state: Mutex::new(AgentExecutionState::Idle),
            token_budgeter,
            engine_runtime,
            http_client,
            rate_limiter,
        })
    }
}

/// Resolve DB path by walking up from current directory to find config.local.toml.
pub fn resolve_db_path() -> PathBuf {
    if let Ok(dir) = std::env::current_dir() {
        let mut current = dir.clone();
        loop {
            let config_path = current.join("config.local.toml");
            if config_path.exists() {
                return current.join("agnes_state.db");
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
    }
    PathBuf::from("agnes_state.db")
}

#[allow(clippy::permissions_set_readonly_false)]
pub fn cleanup_nul_residues(workspace_root: &std::path::Path) -> std::io::Result<()> {
    let abs_root = if workspace_root.is_absolute() {
        workspace_root.to_path_buf()
    } else {
        std::env::current_dir()?.join(workspace_root)
    };
    let targets = vec![
        abs_root.join("nul"),
        abs_root.join("src-tauri").join("nul"),
    ];
    for target in targets {
        let path_str = target.to_string_lossy().to_string();
        let p = if cfg!(windows) {
            let normalized = path_str.replace('/', "\\");
            let unc_path = if !normalized.starts_with(r"\\?\") {
                format!(r"\\?\{}", normalized)
            } else {
                normalized
            };
            std::path::PathBuf::from(unc_path)
        } else {
            target
        };
        if p.exists() || p.is_file() {
            if let Ok(metadata) = std::fs::metadata(&p) {
                let mut permissions = metadata.permissions();
                if permissions.readonly() {
                    permissions.set_readonly(false);
                    let _ = std::fs::set_permissions(&p, permissions);
                }
            }
            std::fs::remove_file(p)?;
        }
    }
    Ok(())
}

pub fn cleanup_tauri_leftovers(workspace_root: &std::path::Path) -> std::io::Result<()> {
    let tauri_conf = workspace_root.join("src-tauri").join("tauri.conf.json");
    if tauri_conf.exists() {
        std::fs::remove_file(tauri_conf)?;
    }
    let run_error_root = workspace_root.join("run_error.log");
    if run_error_root.exists() {
        std::fs::remove_file(run_error_root)?;
    }
    let run_error_tauri = workspace_root.join("src-tauri").join("run_error.log");
    if run_error_tauri.exists() {
        std::fs::remove_file(run_error_tauri)?;
    }
    Ok(())
}

#[allow(clippy::permissions_set_readonly_false)]
pub fn remove_dir_all_force(dir: &std::path::Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut permissions = metadata.permissions();
            if permissions.readonly() {
                permissions.set_readonly(false);
                let _ = std::fs::set_permissions(path, permissions);
            }
        }
    }
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

pub fn handle_interrupted_compilation(workspace_root: &std::path::Path) -> std::io::Result<()> {
    let target_dir = workspace_root.join("target");
    remove_dir_all_force(&target_dir)
}

#[allow(clippy::permissions_set_readonly_false)]
pub fn cleanup_post_run(workspace_root: &std::path::Path) -> std::io::Result<()> {
    if let Ok(entries) = std::fs::read_dir(workspace_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ext_lower == "log" || ext_lower == "db" || ext_lower == "db-journal" || ext_lower == "db-wal" || ext_lower == "db-shm" {
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            let mut permissions = metadata.permissions();
                            if permissions.readonly() {
                                permissions.set_readonly(false);
                                let _ = std::fs::set_permissions(&path, permissions);
                            }
                        }
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
    let agnes_dir = workspace_root.join(".agnes");
    if agnes_dir.exists() {
        let _ = remove_dir_all_force(&agnes_dir);
    }
    Ok(())
}
