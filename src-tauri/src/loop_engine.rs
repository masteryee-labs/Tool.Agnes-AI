//! 自主迴圈引擎（Phase 5A）
//!
//! 目標驅動自主迴圈：Discover → Plan → Execute → Verify → Iterate
//!
//! 對齊 Loop Engineering Automations 組件 + Harness Engineering 迴圈心跳。
//! 這是外層迴圈，每輪呼叫 AgentLoop.run_step 作為 Execute 階段的內層單輪。
//!
//! 退出條件（禁止無限循環）：
//! 1. 目標達成（exit_condition 滿足）→ SUCCESS
//! 2. 達迭代上限 → FAILED
//! 3. 同一失敗碼連續 N 輪 → 升級 premium 重試一次 → 再失敗 → FAILED

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::{Config, LoopEngineConfig};
use crate::db;
use crate::mcp::McpManager;
use crate::rate_limiter::RateLimiter;
use crate::sub_agent::{
    run_evaluator_optimizer_loop, SubAgentInstance, SubAgentRole, SubAgentStatus,
};
use crate::worktree::{WorktreeHandle, WorktreeManager};
use crate::TokenBudgeter;

// ─── 迴圈階段 ────────────────────────────────────────────────────────────────

/// 自主迴圈的五個階段（對齊 AGENTS.md Loop Engineering）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopPhase {
    Discover,
    Plan,
    Execute,
    Verify,
    Iterate,
}

impl LoopPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoopPhase::Discover => "Discover",
            LoopPhase::Plan => "Plan",
            LoopPhase::Execute => "Execute",
            LoopPhase::Verify => "Verify",
            LoopPhase::Iterate => "Iterate",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LoopPhase::Discover => "🔍",
            LoopPhase::Plan => "📋",
            LoopPhase::Execute => "⚡",
            LoopPhase::Verify => "✓",
            LoopPhase::Iterate => "↻",
        }
    }
}

// ─── 迴圈狀態（UI 視覺化用）──────────────────────────────────────────────────

/// 自主迴圈即時狀態（供 UI 即時顯示當前階段/迭代數/剩餘預算）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopState {
    pub goal_id: String,
    pub goal_description: String,
    pub current_phase: LoopPhase,
    pub iteration: i64,
    pub max_iterations: i64,
    pub sub_agent_runs: Vec<SubAgentRunSummary>,
    pub total_tokens: u64,
    pub budget_limit: u64,
    pub last_error: Option<String>,
    pub status: LoopStatus,
}

/// 迴圈最終狀態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopStatus {
    Running,
    Success,
    Failed,
    Stopped,
}

impl LoopStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoopStatus::Running => "RUNNING",
            LoopStatus::Success => "SUCCESS",
            LoopStatus::Failed => "FAILED",
            LoopStatus::Stopped => "STOPPED",
        }
    }
}

/// 子代理執行摘要（UI 顯示用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentRunSummary {
    pub role: String,
    pub status: String,
    pub summary: String,
    pub round: u32,
}

// ─── AutonomousLoop ──────────────────────────────────────────────────────────

/// 自主迴圈引擎：目標驅動 Discover→Plan→Execute→Verify→Iterate。
pub struct AutonomousLoop {
    pub config: Config,
    pub workspace_path: PathBuf,
    pub rate_limiter: Arc<RateLimiter>,
    pub goal_id: String,
    /// 迴圈狀態快照（供 UI 即時讀取）
    pub state: Arc<Mutex<LoopState>>,
}

/// 單輪迭代結果。
#[derive(Debug, Clone)]
struct IterationResult {
    success: bool,
    error_code: String,
    error_message: Option<String>,
    tokens_used: u64,
}

impl AutonomousLoop {
    pub fn new(
        config: Config,
        workspace_path: PathBuf,
        rate_limiter: Arc<RateLimiter>,
        goal_id: String,
        goal_description: String,
    ) -> Self {
        let loop_cfg = &config.loop_engine;
        let state = LoopState {
            goal_id: goal_id.clone(),
            goal_description,
            current_phase: LoopPhase::Discover,
            iteration: 0,
            max_iterations: loop_cfg.max_iterations,
            sub_agent_runs: Vec::new(),
            total_tokens: 0,
            budget_limit: loop_cfg.per_iteration_budget * loop_cfg.max_iterations as u64,
            last_error: None,
            status: LoopStatus::Running,
        };
        Self {
            config,
            workspace_path,
            rate_limiter,
            goal_id,
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// 啟動自主迴圈。
    ///
    /// 流程：
    /// 1. Discover：讀 SQLite + lessons + pitfalls
    /// 2. Plan：Planner 拆解子任務
    /// 3. Execute：Generator 實作（evaluator-optimizer 迴圈）
    /// 4. Verify：22 道驗證 gate（Sensor 層）
    /// 5. Iterate：未過 → Delta 修；最多 max_iterations 輪
    pub async fn run(
        &self,
        goal_description: &str,
        exit_condition: &str,
        mcp_manager: &McpManager,
        token_budgeter: &Mutex<TokenBudgeter>,
        db_path: &Path,
    ) -> Result<LoopStatus, String> {
        let loop_cfg = self.config.loop_engine.clone();
        let mut error_code_counts: HashMap<String, i64> = HashMap::new();

        // 更新 goal 狀態為 RUNNING
        if let Ok(conn) = Connection::open(db_path) {
            let _ = db::update_goal_status(&conn, &self.goal_id, db::GOAL_STATUS_RUNNING);
        }

        for iteration in 1..=loop_cfg.max_iterations {
            // 更新迭代計數
            if let Ok(conn) = Connection::open(db_path) {
                let _ = db::increment_goal_iteration(&conn, &self.goal_id);
            }
            {
                let mut st = self.state.lock().await;
                st.iteration = iteration;
                st.current_phase = LoopPhase::Discover;
            }

            // ── Discover ──
            let discover_context = self.discover(db_path).await;

            // ── Plan ──
            {
                let mut st = self.state.lock().await;
                st.current_phase = LoopPhase::Plan;
            }
            let subtasks = self
                .plan(goal_description, &discover_context, mcp_manager, token_budgeter, db_path)
                .await?;

            if subtasks.is_empty() {
                // Planner 無法拆解 → 目標可能已完成或無法分解
                if self.check_exit_condition(goal_description, exit_condition, db_path) {
                    self.finish(db_path, LoopStatus::Success).await;
                    return Ok(LoopStatus::Success);
                }
                let err = "Planner 無法拆解子任務".to_string();
                *error_code_counts.entry("E_PLAN".to_string()).or_insert(0) += 1;
                {
                    let mut st = self.state.lock().await;
                    st.last_error = Some(err.clone());
                }
                if self.should_fail(&error_code_counts, &loop_cfg) {
                    self.finish(db_path, LoopStatus::Failed).await;
                    return Ok(LoopStatus::Failed);
                }
                continue;
            }

            // ── Execute + Verify（每個子任務走 evaluator-optimizer）──
            {
                let mut st = self.state.lock().await;
                st.current_phase = LoopPhase::Execute;
            }

            let mut all_success = true;
            for (idx, subtask) in subtasks.iter().enumerate() {
                let result = self
                    .execute_subtask(
                        subtask,
                        idx,
                        mcp_manager,
                        token_budgeter,
                        db_path,
                    )
                    .await;

                {
                    let mut st = self.state.lock().await;
                    st.current_phase = LoopPhase::Verify;
                }

                match result {
                    Ok(true) => {
                        let summary = SubAgentRunSummary {
                            role: "Generator+Evaluator".to_string(),
                            status: "PASS".to_string(),
                            summary: subtask.clone(),
                            round: (idx + 1) as u32,
                        };
                        self.state.lock().await.sub_agent_runs.push(summary);
                    }
                    Ok(false) => {
                        all_success = false;
                        *error_code_counts.entry("E_EVALUATOR".to_string()).or_insert(0) += 1;
                        let summary = SubAgentRunSummary {
                            role: "Generator+Evaluator".to_string(),
                            status: "REJECT".to_string(),
                            summary: format!("子任務 {} 未通過 Evaluator", idx + 1),
                            round: (idx + 1) as u32,
                        };
                        self.state.lock().await.sub_agent_runs.push(summary);
                    }
                    Err(e) => {
                        all_success = false;
                        *error_code_counts.entry("E_EXECUTE".to_string()).or_insert(0) += 1;
                        let mut st = self.state.lock().await;
                        st.last_error = Some(e.clone());
                        let summary = SubAgentRunSummary {
                            role: "Generator+Evaluator".to_string(),
                            status: "FAILED".to_string(),
                            summary: e,
                            round: (idx + 1) as u32,
                        };
                        st.sub_agent_runs.push(summary);
                    }
                }
            }

            // ── Iterate ──
            {
                let mut st = self.state.lock().await;
                st.current_phase = LoopPhase::Iterate;
            }

            if all_success {
                // 所有子任務通過 → 檢查退出條件
                if self.check_exit_condition(goal_description, exit_condition, db_path) {
                    self.finish(db_path, LoopStatus::Success).await;
                    return Ok(LoopStatus::Success);
                }
            }

            // 檢查是否應該失敗（同失敗碼連續 N 輪）
            if self.should_fail(&error_code_counts, &loop_cfg) {
                self.finish(db_path, LoopStatus::Failed).await;
                return Ok(LoopStatus::Failed);
            }

            // Token 預算耗盡
            if token_budgeter.lock().await.is_locked() {
                self.finish(db_path, LoopStatus::Failed).await;
                return Ok(LoopStatus::Failed);
            }
        }

        // 達迭代上限
        self.finish(db_path, LoopStatus::Failed).await;
        Ok(LoopStatus::Failed)
    }

    /// Discover 階段：讀 SQLite 真實狀態 + lessons + pitfalls。
    async fn discover(&self, db_path: &Path) -> String {
        let mut context = String::new();

        // 讀 SQLite 現有任務狀態
        if let Ok(conn) = Connection::open(db_path) {
            if let Ok(goal) = db::get_goal(&conn, &self.goal_id) {
                context.push_str(&format!(
                    "目標：{}\n退出條件：{}\n目前迭代：{}/{}\n",
                    goal.description, goal.exit_condition, goal.iteration, goal.max_iterations
                ));
            }
        }

        // 讀跨 Session 記憶
        let memory_mgr = crate::MemoryManager::new(self.workspace_path.clone());
        let lessons = memory_mgr.read_lessons();
        if !lessons.is_empty() {
            context.push_str("\n=== 跨 Session 教訓 ===\n");
            for lesson in lessons.iter().take(10) {
                context.push_str(&format!("- {}\n", lesson));
            }
        }

        let pitfalls = memory_mgr.read_pitfalls();
        if !pitfalls.is_empty() {
            context.push_str("\n=== 跨 Session 雷庫 ===\n");
            for pitfall in pitfalls.iter().take(10) {
                context.push_str(&format!("- {}\n", pitfall));
            }
        }

        context
    }

    /// Plan 階段：Planner 子代理拆解目標為子任務列表。
    async fn plan(
        &self,
        goal_description: &str,
        discover_context: &str,
        mcp_manager: &McpManager,
        token_budgeter: &Mutex<TokenBudgeter>,
        db_path: &Path,
    ) -> Result<Vec<String>, String> {
        let conv_id = format!("planner-{}", uuid::Uuid::new_v4());
        let mut planner = SubAgentInstance::new(
            SubAgentRole::Planner,
            self.config.clone(),
            self.workspace_path.to_string_lossy().to_string(),
            self.rate_limiter.clone(),
            conv_id.clone(),
            None,
        );

        let input = format!(
            "目標：{}\n\n{}\n\n請拆解為子任務列表。",
            goal_description, discover_context
        );

        let result = planner
            .execute(&input, mcp_manager, token_budgeter, db_path)
            .await?;

        // 解析子任務列表（每行一個）
        // 支援格式：
        //   T1 [風險級] 路徑: 描述（新版）
        //   [風險級] 路徑: 描述（舊版）
        //   - 描述 / * 描述（通用列表）
        let subtasks: Vec<String> = result
            .output
            .lines()
            .filter(|l| {
                let trimmed = l.trim();
                if trimmed.is_empty() { return false; }
                // 新版：T1, T2, ... 開頭
                if trimmed.len() >= 2 && trimmed.starts_with('T') && trimmed[1..].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    return true;
                }
                // 舊版/通用
                trimmed.starts_with('[')
                    || trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
            })
            .map(|l| l.trim().to_string())
            .collect();

        // 記錄到 DB
        if let Ok(conn) = Connection::open(db_path) {
            let _ = crate::sub_agent::record_sub_agent_run(
                &conn,
                &self.goal_id,
                SubAgentRole::Planner,
                &conv_id,
                None,
                result.status.as_str(),
                &format!("拆解出 {} 個子任務", subtasks.len()),
            );
        }

        Ok(subtasks)
    }

    /// Execute 階段：對單一子任務跑 evaluator-optimizer 迴圈。
    async fn execute_subtask(
        &self,
        subtask: &str,
        idx: usize,
        mcp_manager: &McpManager,
        token_budgeter: &Mutex<TokenBudgeter>,
        db_path: &Path,
    ) -> Result<bool, String> {
        let sub_cfg = &self.config.sub_agent;
        let agent_id = format!("gen-{}-{}", self.goal_id.short(), idx);

        // Worktree 隔離（若啟用且 workspace 是 git repo）
        let worktree_handle: Option<WorktreeHandle> = if sub_cfg.worktree_isolation {
            let wt_mgr = WorktreeManager::new(
                self.workspace_path.clone(),
                self.config.worktree.clone(),
            );
            if wt_mgr.is_git_repo() {
                wt_mgr.create(&agent_id).ok() // worktree 建立失敗 → 退回主 workspace
            } else {
                None
            }
        } else {
            None
        };

        let work_path = worktree_handle
            .as_ref()
            .map(|h| h.path.to_string_lossy().to_string())
            .unwrap_or_else(|| self.workspace_path.to_string_lossy().to_string());

        let gen_conv = format!("gen-{}", uuid::Uuid::new_v4());
        let eval_conv = format!("eval-{}", uuid::Uuid::new_v4());

        let mut generator = SubAgentInstance::new(
            SubAgentRole::Generator,
            self.config.clone(),
            work_path.clone(),
            self.rate_limiter.clone(),
            gen_conv.clone(),
            worktree_handle.as_ref().map(|h| h.path.clone()),
        );

        let mut evaluator = SubAgentInstance::new(
            SubAgentRole::Evaluator,
            self.config.clone(),
            self.workspace_path.to_string_lossy().to_string(),
            self.rate_limiter.clone(),
            eval_conv.clone(),
            None,
        );

        let result = run_evaluator_optimizer_loop(
            subtask,
            &mut generator,
            &mut evaluator,
            mcp_manager,
            token_budgeter,
            db_path,
            sub_cfg.max_repair_rounds,
        )
        .await?;

        // 記錄到 DB
        if let Ok(conn) = Connection::open(db_path) {
            let _ = crate::sub_agent::record_sub_agent_run(
                &conn,
                &self.goal_id,
                SubAgentRole::Generator,
                &gen_conv,
                worktree_handle.as_ref().map(|h| h.path.to_string_lossy().to_string()).as_deref(),
                result.final_status.as_str(),
                &format!("子任務 {}: {} 輪", idx + 1, result.rounds),
            );
        }

        // Worktree merge + cleanup
        if let Some(handle) = &worktree_handle {
            if result.final_status == SubAgentStatus::Pass {
                let wt_mgr = WorktreeManager::new(
                    self.workspace_path.clone(),
                    self.config.worktree.clone(),
                );
                let _ = wt_mgr.merge(handle);
            }
            if self.config.worktree.auto_cleanup {
                let wt_mgr = WorktreeManager::new(
                    self.workspace_path.clone(),
                    self.config.worktree.clone(),
                );
                let _ = wt_mgr.cleanup(handle);
            }
        }

        // 更新 token 統計
        {
            let mut st = self.state.lock().await;
            st.total_tokens += result.total_tokens;
        }

        Ok(result.final_status == SubAgentStatus::Pass)
    }

    /// Verify 階段：檢查退出條件是否滿足。
    ///
    /// 簡化版：若 exit_condition 包含 "tests pass"，跑 cargo test；
    /// 若包含 "compiles"，跑 cargo check；否則視為目標描述已達成（由 Evaluator 保證）。
    fn check_exit_condition(
        &self,
        _goal: &str,
        exit_condition: &str,
        _db_path: &Path,
    ) -> bool {
        let ec = exit_condition.to_lowercase();

        // 「檔案存在」語意：file:xxx.txt exists / 檔案 xxx 存在
        if ec.contains("exists") || ec.contains("存在") || ec.contains("建立") {
            // 從 exit_condition 中提取檔案路徑
            // 支援格式："file:Docs/test.txt exists" / "Docs/test.txt 存在" / "建立 Docs/test.txt"
            let path_candidates: Vec<String> = exit_condition
                .split_whitespace()
                .filter_map(|w| {
                    // 去掉 "file:" 前綴
                    let cleaned = w.strip_prefix("file:").unwrap_or(w);
                    // 去掉首尾標點
                    let cleaned = cleaned.trim_matches(|c: char| c == ',' || c == ':' || c == '"' || c == '\'' || c == '(' || c == ')');
                    // 必須包含 . 或 / 才可能是檔案路徑
                    if cleaned.contains('.') || cleaned.contains('/') {
                        Some(cleaned.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            for candidate in &path_candidates {
                let full_path = self.workspace_path.join(candidate);
                if full_path.exists() {
                    return true;
                }
            }
            // 如果 exit_condition 含「存在」但找不到檔案 → 未達成
            if ec.contains("exists") || ec.contains("存在") {
                return false;
            }
        }

        // cargo test
        if ec.contains("tests pass") || ec.contains("cargo test") {
            let output = crate::no_window::silent_command("cargo")
                .args(["test", "--manifest-path", "Cargo.toml"])
                .current_dir(&self.workspace_path)
                .output();
            return match output {
                Ok(o) => o.status.success(),
                Err(_) => false,
            };
        }
        // cargo check / compiles
        if ec.contains("compiles") || ec.contains("cargo check") {
            let output = crate::no_window::silent_command("cargo")
                .args(["check", "--manifest-path", "Cargo.toml"])
                .current_dir(&self.workspace_path)
                .output();
            return match output {
                Ok(o) => o.status.success(),
                Err(_) => false,
            };
        }
        // 「所有子任務通過 Evaluator」或無明確退出條件 → 所有子任務通過即視為達成
        true
    }

    /// 判定是否應該失敗（同失敗碼連續 N 輪 → 升級 premium → 再失敗 → FAILED）。
    fn should_fail(&self, error_counts: &HashMap<String, i64>, cfg: &LoopEngineConfig) -> bool {
        for count in error_counts.values() {
            if *count >= cfg.max_same_failures {
                // 已達連續失敗上限
                // TODO: 升級 premium 模型重試（目前直接 FAILED）
                return true;
            }
        }
        false
    }

    /// 迴圈結束：更新 DB + 狀態。
    async fn finish(&self, db_path: &Path, status: LoopStatus) {
        let db_status = match status {
            LoopStatus::Success => db::GOAL_STATUS_SUCCESS,
            LoopStatus::Failed => db::GOAL_STATUS_FAILED,
            _ => db::GOAL_STATUS_PENDING,
        };
        if let Ok(conn) = Connection::open(db_path) {
            let _ = db::update_goal_status(&conn, &self.goal_id, db_status);
        }
        let mut st = self.state.lock().await;
        st.status = status;
        st.current_phase = LoopPhase::Iterate;
    }

    /// 停止迴圈（使用者手動中止）。
    pub async fn stop(&self, db_path: &Path) {
        self.finish(db_path, LoopStatus::Stopped).await;
    }
}

// ─── 輔助 trait ──────────────────────────────────────────────────────────────

/// 取字串前 8 字元（用於 agent_id 縮短顯示）。
trait ShortStr {
    fn short(&self) -> String;
}

impl ShortStr for String {
    fn short(&self) -> String {
        if self.len() <= 8 {
            self.clone()
        } else {
            self[..8].to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_phase_str() {
        assert_eq!(LoopPhase::Discover.as_str(), "Discover");
        assert_eq!(LoopPhase::Plan.as_str(), "Plan");
        assert_eq!(LoopPhase::Execute.as_str(), "Execute");
        assert_eq!(LoopPhase::Verify.as_str(), "Verify");
        assert_eq!(LoopPhase::Iterate.as_str(), "Iterate");
    }

    #[test]
    fn loop_status_str() {
        assert_eq!(LoopStatus::Running.as_str(), "RUNNING");
        assert_eq!(LoopStatus::Success.as_str(), "SUCCESS");
        assert_eq!(LoopStatus::Failed.as_str(), "FAILED");
        assert_eq!(LoopStatus::Stopped.as_str(), "STOPPED");
    }

    #[test]
    fn should_fail_at_threshold() {
        let cfg = LoopEngineConfig {
            max_same_failures: 3,
            ..LoopEngineConfig::default()
        };
        let loop_engine = AutonomousLoop::new(
            Config::default(),
            PathBuf::from("."),
            Arc::new(RateLimiter::new(20)),
            "test-goal".to_string(),
            "test".to_string(),
        );
        let mut counts = HashMap::new();
        counts.insert("E_COMPILE".to_string(), 3);
        assert!(loop_engine.should_fail(&counts, &cfg));

        counts.insert("E_COMPILE".to_string(), 2);
        assert!(!loop_engine.should_fail(&counts, &cfg));
    }

    #[test]
    fn short_str_works() {
        assert_eq!("12345678".to_string().short(), "12345678");
        assert_eq!("123456789".to_string().short(), "12345678");
        assert_eq!("short".to_string().short(), "short");
    }

    #[test]
    fn loop_engine_config_defaults() {
        let cfg = LoopEngineConfig::default();
        assert_eq!(cfg.max_iterations, 10);
        assert_eq!(cfg.max_same_failures, 3);
        assert!(cfg.premium_retry);
    }
}
