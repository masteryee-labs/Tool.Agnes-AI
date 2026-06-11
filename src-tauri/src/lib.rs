mod config;
mod db;
mod locale;
mod sandbox;
mod orchestrator;
mod agent;
mod mcp;
mod memory;
mod validation;

#[cfg(test)]
mod tests_integration;

pub use config::Config;
pub use config::key_persistence;
pub use db::*;
pub use locale::*;
pub use sandbox::*;
pub use orchestrator::Orchestrator;
pub use orchestrator::{SubAgent, ConfirmationGate, PendingAction, ActionRiskLevel};
pub use agent::{AgentLoop, ToolCall, AuditResult, AgentStep, PendingState, AgentEngine, split_command_line};
pub use mcp::McpManager;
pub use memory::*;
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
}

impl AppState {
    pub fn new(db_path: PathBuf, config: Arc<std::sync::Mutex<config::Config>>) -> Result<Self, String> {
        let mcp_manager = McpManager::new();
        let engine_runtime = Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

        let session_budget = config.lock().unwrap().api.session_budget;
        let token_budgeter = Mutex::new(TokenBudgeter::new(session_budget));

        let timeout_secs = config.lock().unwrap().sandbox.timeout_seconds;
        let http_client = Arc::new(Mutex::new(
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .user_agent("Agnes-AI/0.3.0")
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
