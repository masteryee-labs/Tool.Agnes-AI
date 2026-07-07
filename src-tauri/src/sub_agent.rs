//! 真子代理：Planner / Generator / Evaluator（Phase 5B）
//!
//! 對齊 Anthropic 三代理 Harness 架構 + Loop Engineering 子代理組件。
//! 這是獨立的 `SubAgentInstance`，不是 22 道驗證 gate（那是 Sensor）。
//!
//! evaluator-optimizer 模式：
//!   Planner 拆解 → Generator 實作 → Evaluator 獨立驗證
//!                       ↑                  │
//!                       │    REJECT        │
//!                       └── Delta 回饋 ────┘
//!                             最多 N 輪

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::agent::AgentLoop;
use crate::config::Config;
use crate::db;
use crate::mcp::McpManager;
use crate::rate_limiter::RateLimiter;
use crate::TokenBudgeter;

// ─── 子代理角色定義 ──────────────────────────────────────────────────────────

/// 三角色：Planner / Generator / Evaluator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubAgentRole {
    Planner,
    Generator,
    Evaluator,
}

impl SubAgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubAgentRole::Planner => "Planner",
            SubAgentRole::Generator => "Generator",
            SubAgentRole::Evaluator => "Evaluator",
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            SubAgentRole::Planner => PLANNER_PROMPT,
            SubAgentRole::Generator => GENERATOR_PROMPT,
            SubAgentRole::Evaluator => EVALUATOR_PROMPT,
        }
    }

    /// 從字串解析角色（DB 讀回用）。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Planner" => Some(SubAgentRole::Planner),
            "Generator" => Some(SubAgentRole::Generator),
            "Evaluator" => Some(SubAgentRole::Evaluator),
            _ => None,
        }
    }
}

const PLANNER_PROMPT: &str = r#"你是規劃者（Planner）。把目標分解為可執行的原子子任務列表，每項標明：
1. 子任務 ID（T1, T2, ...）
2. 子任務描述（一行，動詞開頭）
3. 目標檔案路徑（相對於 workspace 根）
4. 風險級（Low/Med/High/Critical）

規則：
- 只輸出子任務列表，每行一個，格式：`T1 [風險級] 目標檔案路徑: 子任務描述`
- 子任務必須是原子操作（一個子任務 = 一次寫檔或一次指令）
- 禁止寬容：不確定的部分標 `[High]` 風險
- Delta-only：不重述目標，只輸出分解結果
- 檔案路徑必須是相對路徑（如 Docs/test.txt，不是 C:\...\Docs\test.txt）
"#;

const GENERATOR_PROMPT: &str = r#"你是生成者（Generator）。一次實作一個子任務，增量開發。

工具使用規則（最重要）：
- 你必須使用 <write_file path="相對路徑">內容</write_file> 工具標籤來建立或修改檔案
- 禁止只輸出文字描述而不呼叫工具——文字描述不會產生任何檔案
- 範例：
  <write_file path="Docs/test.txt">
  這是檔案內容
  </write_file>
- 路徑必須是相對於 workspace 根的路徑（如 Docs/test.txt）

其他規則：
- Delta-only：只輸出需要變更的部分，禁止重寫未變更的檔案
- 遵守鋼鐵戒律：金鑰不硬編碼、零 Magic Number、產物路徑分流
- 收到 Evaluator 的 REJECT 時：只修被指出的問題，禁止重寫整檔
- 先簡述你要做什麼（一行），然後立即輸出 <write_file> 工具標籤
"#;

const EVALUATOR_PROMPT: &str = r#"你是評估者（Evaluator）。獨立驗證生成結果是否符合子任務規格。

驗證規則（嚴格）：
- 禁止對生成者寬容——你是獨立的驗證者，不是生成者的同事
- 禁止只看生成者的文字描述就判定 PASS——必須檢查實際產出
- 如果子任務要求建立檔案，生成者必須使用了 <write_file> 工具標籤
- 如果生成者只輸出文字而沒有 <write_file> 標籤 → 必須 REJECT
- 如果生成者的文字聲稱「已建立檔案」但沒有實際的工具標籤 → 必須 REJECT

輸出格式嚴格二選一：
  - 通過：`[PASS] 簡述符合規格的理由（含已驗證的工具呼叫）`
  - 駁回：`[REJECT: 子任務ID | 檔案:行號 | 原因 | 修復指示]`

REJECT 必須結構化，讓 Generator 能 Delta-only 修正。
檢查項：規格符合度、工具是否實際呼叫、鋼鐵戒律、Delta-only 格式、編譯可行性
"#;

// ─── 子代理執行結果 ──────────────────────────────────────────────────────────

/// 子代理單次執行結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    pub role: String,
    pub status: SubAgentStatus,
    pub output: String,
    pub reject_reason: Option<String>,
    pub tokens_used: u64,
    /// Generator 實際執行的工具呼叫（供 Evaluator 獨立驗證）
    pub executed_tools: Vec<String>,
}

/// 子代理執行狀態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubAgentStatus {
    Pass,
    Reject,
    Failed,
}

impl SubAgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubAgentStatus::Pass => "PASS",
            SubAgentStatus::Reject => "REJECT",
            SubAgentStatus::Failed => "FAILED",
        }
    }
}

// ─── 子代理實例 ──────────────────────────────────────────────────────────────

/// 獨立子代理實例：持有獨立 AgentLoop + 角色特定 system prompt + 獨立 conversation_id。
///
/// 對齊 Anthropic：Evaluator 絕不是 Generator 的同一實例——
/// 獨立 AgentLoop + 不同 system prompt + 不同 conversation_id，防「自己說服自己」。
pub struct SubAgentInstance {
    pub role: SubAgentRole,
    pub agent_loop: AgentLoop,
    pub conversation_id: String,
    pub worktree_path: Option<PathBuf>,
}

impl SubAgentInstance {
    /// 建構子代理實例。
    ///
    /// - `role`：角色（決定 system prompt）
    /// - `config`：全域組態（含 sub_agent 設定）
    /// - `workspace_path`：工作目錄（worktree 隔離時為 worktree 路徑）
    /// - `rate_limiter`：App 級共享令牌桶
    /// - `conversation_id`：獨立對話 ID（與其他子代理不共用）
    /// - `worktree_path`：若有 worktree 隔離，傳入 worktree 路徑
    pub fn new(
        role: SubAgentRole,
        config: Config,
        workspace_path: String,
        rate_limiter: Arc<RateLimiter>,
        key_rotator: Arc<crate::key_rotation::KeyRotator>,
        conversation_id: String,
        worktree_path: Option<PathBuf>,
    ) -> Self {
        let mut agent_loop = AgentLoop::with_rate_limiter_and_rotator(
            config,
            workspace_path,
            rate_limiter,
            key_rotator,
        );
        agent_loop.set_conversation_id(&conversation_id);
        Self {
            role,
            agent_loop,
            conversation_id,
            worktree_path,
        }
    }

    /// 執行子代理：呼叫 LLM API 並回傳結果。
    ///
    /// - `input`：輸入訊息（子任務描述 / Generator 產出 / 等）
    /// - `mcp_manager`：MCP 伺服器管理器
    /// - `token_budgeter`：Token 預算器
    /// - `db_path`：SQLite 路徑
    pub async fn execute(
        &mut self,
        input: &str,
        mcp_manager: &McpManager,
        token_budgeter: &Mutex<TokenBudgeter>,
        db_path: &std::path::Path,
    ) -> Result<SubAgentResult, String> {
        // 構建 messages：system prompt + user input
        let mut messages = vec![
            serde_json::json!({
                "role": "system",
                "content": self.role.system_prompt(),
            }),
            serde_json::json!({
                "role": "user",
                "content": input,
            }),
        ];

        let tokens_before = {
            let budget = token_budgeter.lock().await;
            budget.total_spent()
        };

        let step_result = self
            .agent_loop
            .run_step(&mut messages, mcp_manager, token_budgeter, db_path)
            .await;

        let tokens_after = {
            let budget = token_budgeter.lock().await;
            budget.total_spent()
        };
        let tokens_used = tokens_after.saturating_sub(tokens_before);

        match step_result {
            Ok(step) => {
                let output = step.response_text.clone();
                let executed_tools: Vec<String> = step
                    .executed_tools
                    .iter()
                    .map(|t| {
                        if let Some(p) = &t.path {
                            format!("{}({})", t.name, p)
                        } else {
                            t.name.clone()
                        }
                    })
                    .collect();
                // 計算型防線：檢查 execution_results 中是否有 ERROR
                let exec_errors: Vec<String> = step
                    .execution_results
                    .iter()
                    .filter(|r| r.contains("[ERROR]") || r.contains("[AUDIT REJECTED]"))
                    .cloned()
                    .collect();
                if !exec_errors.is_empty() {
                    eprintln!(
                        "[SubAgent] {} execution errors: {:?}",
                        self.role.as_str(),
                        exec_errors
                    );
                }
                let (status, reject_reason) = self.parse_result(&output, &executed_tools);
                Ok(SubAgentResult {
                    role: self.role.as_str().to_string(),
                    status,
                    output,
                    reject_reason,
                    tokens_used,
                    executed_tools,
                })
            }
            Err(e) => {
                eprintln!("[SubAgent] {} execute failed: {}", self.role.as_str(), e);
                Ok(SubAgentResult {
                    role: self.role.as_str().to_string(),
                    status: SubAgentStatus::Failed,
                    output: String::new(),
                    reject_reason: Some(e),
                    tokens_used,
                    executed_tools: Vec::new(),
                })
            }
        }
    }

    /// 解析執行結果，判定 PASS / REJECT / FAILED。
    ///
    /// - `output`：LLM 文字回應
    /// - `executed_tools`：實際執行的工具清單（供 Generator 自我驗證）
    fn parse_result(&self, output: &str, executed_tools: &[String]) -> (SubAgentStatus, Option<String>) {
        match self.role {
            SubAgentRole::Evaluator => {
                if output.contains("[PASS]") {
                    (SubAgentStatus::Pass, None)
                } else if output.contains("[REJECT") {
                    let reason = output
                        .find("[REJECT")
                        .map(|i| &output[i..])
                        .unwrap_or(output)
                        .to_string();
                    (SubAgentStatus::Reject, Some(reason))
                } else {
                    // Evaluator 未明確判定 → 視為 REJECT（禁止寬容）
                    (
                        SubAgentStatus::Reject,
                        Some("[REJECT: Evaluator 未明確判定 PASS | 全文 | 輸出未含 [PASS] 或 [REJECT] | 請重新驗證並明確標示]".to_string()),
                    )
                }
            }
            SubAgentRole::Generator => {
                // Generator 的成功與否由 Evaluator 判定
                // 但這裡加一道計算型防線：如果輸出含 <write_file> 但 executed_tools 為空
                // → 表示工具標籤被解析但未執行（可能被安全閘攔截）
                if output.is_empty() {
                    return (
                        SubAgentStatus::Failed,
                        Some("生成者輸出為空".to_string()),
                    );
                }
                // 如果輸出含 write_file 標籤但沒有實際執行 → 標記讓 Evaluator 知道
                if output.contains("<write_file") && executed_tools.is_empty() {
                    return (
                        SubAgentStatus::Pass, // 仍回 Pass 讓 Evaluator 做最終判定
                        Some("警告：輸出含 <write_file> 標籤但工具未實際執行（可能被安全閘攔截）".to_string()),
                    );
                }
                (SubAgentStatus::Pass, None)
            }
            SubAgentRole::Planner => {
                if output.is_empty() {
                    (
                        SubAgentStatus::Failed,
                        Some("規劃者輸出為空".to_string()),
                    )
                } else {
                    (SubAgentStatus::Pass, None)
                }
            }
        }
    }
}

// ─── Evaluator-Optimizer 迴圈 ────────────────────────────────────────────────

/// evaluator-optimizer 迴圈結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatorOptimizerResult {
    pub final_status: SubAgentStatus,
    pub rounds: u32,
    pub final_output: String,
    pub total_tokens: u64,
    pub history: Vec<SubAgentResult>,
}

/// 執行 evaluator-optimizer 迴圈：
/// Generator 實作 → Evaluator 驗證 → REJECT → Delta 回饋 → 重複（最多 N 輪）
///
/// - `subtask`：子任務描述
/// - `generator`：Generator 子代理實例
/// - `evaluator`：Evaluator 子代理實例（獨立）
/// - `mcp_manager`：MCP 管理器
/// - `token_budgeter`：Token 預算器
/// - `db_path`：SQLite 路徑
/// - `max_rounds`：最多修正幾輪（從 SubAgentConfig.max_repair_rounds）
pub async fn run_evaluator_optimizer_loop(
    subtask: &str,
    generator: &mut SubAgentInstance,
    evaluator: &mut SubAgentInstance,
    mcp_manager: &McpManager,
    token_budgeter: &Mutex<TokenBudgeter>,
    db_path: &std::path::Path,
    max_rounds: i64,
) -> Result<EvaluatorOptimizerResult, String> {
    let mut history: Vec<SubAgentResult> = Vec::new();
    let mut current_input = format!(
        "子任務：{}\n\n請使用 <write_file path=\"相對路徑\">內容</write_file> 工具標籤實作。",
        subtask
    );
    let mut total_tokens = 0u64;

    for round in 1..=max_rounds.max(1) {
        // Generator 實作
        let gen_result = generator
            .execute(&current_input, mcp_manager, token_budgeter, db_path)
            .await?;
        total_tokens += gen_result.tokens_used;
        let gen_output = gen_result.output.clone();
        let gen_tools = gen_result.executed_tools.clone();
        history.push(gen_result);

        if gen_output.is_empty() {
            // Generator 失敗（可能 API 暫時錯誤）→ 等 2 秒重試一次
            if round == 1 {
                eprintln!("[SubAgent] Generator failed on round 1, retrying after 2s…");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
            return Ok(EvaluatorOptimizerResult {
                final_status: SubAgentStatus::Failed,
                rounds: round as u32,
                final_output: String::new(),
                total_tokens,
                history,
            });
        }

        // 計算型防線：Generator 沒有實際執行任何工具 → 直接 REJECT 不浪費 Evaluator token
        if gen_tools.is_empty() && gen_output.contains("<write_file") {
            // 工具標籤存在但未執行 → 可能被安全閘攔截
            let reject_reason = "[REJECT: 工具未執行 | write_file 標籤被安全閘攔截 | 檢查路徑是否在 workspace 內 | 請確認路徑是相對路徑]".to_string();
            current_input = format!(
                "上一輪產出被攔截：\n{}\n\n問題：write_file 工具標籤存在但未實際執行。\n請確認路徑是相對於 workspace 根的路徑（如 Docs/test.txt），不是絕對路徑。\n請 Delta-only 修正後重新輸出 <write_file> 標籤。",
                reject_reason
            );
            continue;
        }
        if gen_tools.is_empty() {
            // 完全沒有工具呼叫 → 直接 REJECT
            let reject_reason = "[REJECT: 無工具呼叫 | 生成者未使用 <write_file> 工具 | 只輸出文字不會建立檔案 | 請使用 <write_file path=\"相對路徑\">內容</write_file>]".to_string();
            current_input = format!(
                "上一輪產出被駁回：\n{}\n\n關鍵問題：你只輸出了文字描述，沒有使用 <write_file> 工具標籤。\n文字描述不會建立任何檔案。\n請立即使用 <write_file path=\"相對路徑\">內容</write_file> 工具標籤來實作子任務。",
                reject_reason
            );
            continue;
        }

        // Evaluator 獨立驗證（附上工具執行清單讓 Evaluator 做最終判定）
        let tools_summary = if gen_tools.is_empty() {
            "（無工具執行）".to_string()
        } else {
            gen_tools.iter().map(|t| format!("- {}", t)).collect::<Vec<_>>().join("\n")
        };
        let eval_input = format!(
            "子任務規格：{}\n\n生成者產出（文字）：\n{}\n\n生成者實際執行的工具：\n{}\n\n請驗證是否符合規格。重點檢查：工具是否確實執行、檔案是否會被建立。",
            subtask, gen_output, tools_summary
        );
        let eval_result = evaluator
            .execute(&eval_input, mcp_manager, token_budgeter, db_path)
            .await?;
        total_tokens += eval_result.tokens_used;

        match eval_result.status {
            SubAgentStatus::Pass => {
                history.push(eval_result);
                return Ok(EvaluatorOptimizerResult {
                    final_status: SubAgentStatus::Pass,
                    rounds: round as u32,
                    final_output: gen_output,
                    total_tokens,
                    history,
                });
            }
            SubAgentStatus::Reject => {
                let reject_reason = eval_result.reject_reason.clone().unwrap_or_default();
                history.push(eval_result);
                // Delta 回饋：只送 REJECT 原因 + 修復指示，不重送整個上下文
                current_input = format!(
                    "上一輪產出被 Evaluator 駁回：\n{}\n\n請 Delta-only 修正（只改被指出的問題，禁止重寫整檔）。",
                    reject_reason
                );
            }
            SubAgentStatus::Failed => {
                history.push(eval_result);
                return Ok(EvaluatorOptimizerResult {
                    final_status: SubAgentStatus::Failed,
                    rounds: round as u32,
                    final_output: gen_output,
                    total_tokens,
                    history,
                });
            }
        }
    }

    // 達最大輪數仍未通過
    Ok(EvaluatorOptimizerResult {
        final_status: SubAgentStatus::Reject,
        rounds: max_rounds as u32,
        final_output: history
            .last()
            .map(|h| h.output.clone())
            .unwrap_or_default(),
        total_tokens,
        history,
    })
}

// ─── DB 記錄輔助 ─────────────────────────────────────────────────────────────

/// 將子代理執行記錄寫入 DB。
pub fn record_sub_agent_run(
    conn: &Connection,
    goal_id: &str,
    role: SubAgentRole,
    conversation_id: &str,
    worktree_path: Option<&str>,
    status: &str,
    result_summary: &str,
) -> Result<String, String> {
    let run_id = db::create_sub_agent_run(conn, goal_id, role.as_str(), conversation_id, worktree_path)
        .map_err(|e| format!("建立 sub_agent_run 失敗: {}", e))?;
    db::update_sub_agent_run(conn, &run_id, status, result_summary)
        .map_err(|e| format!("更新 sub_agent_run 失敗: {}", e))?;
    Ok(run_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_roundtrip() {
        for role in [SubAgentRole::Planner, SubAgentRole::Generator, SubAgentRole::Evaluator] {
            let s = role.as_str();
            assert_eq!(SubAgentRole::parse(s), Some(role));
        }
        assert_eq!(SubAgentRole::parse("Unknown"), None);
    }

    #[test]
    fn evaluator_parse_pass() {
        let instance_str = "[PASS] 符合規格";
        let mgr = SubAgentInstance {
            role: SubAgentRole::Evaluator,
            agent_loop: AgentLoop::new(Config::default(), ".".to_string()),
            conversation_id: "test".to_string(),
            worktree_path: None,
        };
        let (status, reason) = mgr.parse_result(instance_str, &[]);
        assert_eq!(status, SubAgentStatus::Pass);
        assert!(reason.is_none());
    }

    #[test]
    fn evaluator_parse_reject() {
        let reject_str = "[REJECT: task-1 | src/main.rs:42 | 缺錯誤處理 | 加 Result 包裹]";
        let mgr = SubAgentInstance {
            role: SubAgentRole::Evaluator,
            agent_loop: AgentLoop::new(Config::default(), ".".to_string()),
            conversation_id: "test".to_string(),
            worktree_path: None,
        };
        let (status, reason) = mgr.parse_result(reject_str, &[]);
        assert_eq!(status, SubAgentStatus::Reject);
        assert!(reason.is_some());
    }

    #[test]
    fn evaluator_no_verdict_defaults_reject() {
        let mgr = SubAgentInstance {
            role: SubAgentRole::Evaluator,
            agent_loop: AgentLoop::new(Config::default(), ".".to_string()),
            conversation_id: "test".to_string(),
            worktree_path: None,
        };
        let (status, _) = mgr.parse_result("看起來還行", &[]);
        assert_eq!(status, SubAgentStatus::Reject);
    }

    #[test]
    fn generator_empty_output_fails() {
        let mgr = SubAgentInstance {
            role: SubAgentRole::Generator,
            agent_loop: AgentLoop::new(Config::default(), ".".to_string()),
            conversation_id: "test".to_string(),
            worktree_path: None,
        };
        let (status, _) = mgr.parse_result("", &[]);
        assert_eq!(status, SubAgentStatus::Failed);
    }

    #[test]
    fn generator_write_tag_but_no_exec_warns() {
        let mgr = SubAgentInstance {
            role: SubAgentRole::Generator,
            agent_loop: AgentLoop::new(Config::default(), ".".to_string()),
            conversation_id: "test".to_string(),
            worktree_path: None,
        };
        let output = r#"<write_file path="test.txt">hello</write_file>"#;
        let (status, reason) = mgr.parse_result(output, &[]);
        assert_eq!(status, SubAgentStatus::Pass); // 仍 Pass 讓 Evaluator 判定
        assert!(reason.is_some()); // 但有警告
    }

    #[test]
    fn generator_with_executed_tools_passes() {
        let mgr = SubAgentInstance {
            role: SubAgentRole::Generator,
            agent_loop: AgentLoop::new(Config::default(), ".".to_string()),
            conversation_id: "test".to_string(),
            worktree_path: None,
        };
        let tools = vec!["write_file(test.txt)".to_string()];
        let (status, reason) = mgr.parse_result("已建立檔案", &tools);
        assert_eq!(status, SubAgentStatus::Pass);
        assert!(reason.is_none());
    }

    #[test]
    fn config_defaults() {
        let cfg = crate::config::SubAgentConfig::default();
        assert_eq!(cfg.max_repair_rounds, 5);
        assert!(cfg.worktree_isolation);
    }
}
