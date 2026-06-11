use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;
use crate::sandbox;
use crate::config::Config;
use crate::mcp::McpManager;

const AGNES_SYSTEM_PROMPT: &str = r#"You are Agnes AI, an advanced, highly-secure multi-agent coding and task-execution assistant.
You operate natively on the user's system to solve complex coding, refactoring, and directory tasks.

To interact with the environment, you MUST use the following XML-based tool call tags. Any other text formatting will NOT execute tools.

Available Tools:

1. Write File:
   Use this to create or overwrite a file.
   Format:
   <write_file path="relative/path/to/file.ext">
   File content goes here. Keep comments and structure clean.
   </write_file>

2. Read File:
   Use this to inspect the content of a file.
   Format:
   <read_file path="relative/path/to/file.ext"/>

3. Run Command:
   Use this to run terminal commands (like cargo, git, npm, etc.) in the workspace sandbox.
   Format:
   <run_command>
   command args
   </run_command>

4. Call MCP Tool:
   Use this to invoke tools on configured Model Context Protocol (MCP) servers.
   Format:
   <run_mcp server="server_name" tool="tool_name">
   {"arg_name": "arg_value"}
   </run_mcp>

Rules for Tool Use and Output formatting:
- Explain your reasoning and plan briefly first, then output the necessary XML tags to execute the action.
- You can output multiple tool calls in a single turn. They will be executed sequentially.
- Path Traversal: Do NOT use parent directory components (e.g., '..') in file paths. All paths must be within the active workspace.
- Paths are relative to the workspace root itself. Do NOT prefix paths with the workspace folder name (write src/foo.rs, NOT project/src/foo.rs).
- Avoid AI slang, slop, or decorative padding phrases (such as "delve", "testament", "underscore", "crucial", "furthermore", "pivotal"). Be precise, engineering-focused, and direct.
- Code Completeness: Never write placeholder code, TODOs, or leave sections commented out. Write 100% complete, compilable code.
"#;

// ──────────────────────────────────────────────────────────────────────────────
// Public data types
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub path: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub agent_name: String,
    pub verdict: String, // "PASSED" or "REJECTED"
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub response_text: String,
    pub executed_tools: Vec<ToolCall>,
    pub execution_results: Vec<String>,
    pub audits: Vec<AuditResult>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingState {
    pub pending_tools: Vec<ToolCall>,
    pub pending_response: String,
    /// 送出當下的工作區——核准執行時必須沿用，否則路徑圈禁失效（檔案逃逸到 CWD）
    pub workspace_path: String,
    /// 所屬對話——核准後的執行結果要寫回正確的對話
    pub conversation_id: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// AgentEngine: loads and runs all 17 agents sequentially (one-vote veto)
// ──────────────────────────────────────────────────────────────────────────────

pub struct AgentEngine;

impl AgentEngine {
    /// Run all 22 validation passes. If any rejects, the task fails.
    pub fn run_validation(
        &self,
        config: &Config,
        tool_calls: &[ToolCall],
        messages: &[serde_json::Value],
    ) -> Vec<AuditResult> {
        crate::validation::run_all_gates(config, tool_calls, messages)
    }

    /// Check if any agent rejected (one-vote veto).
    pub fn any_rejected(audits: &[AuditResult]) -> bool {
        audits.iter().any(|a| a.verdict == "REJECTED")
    }

    /// Collect rejection details.
    pub fn rejection_details(audits: &[AuditResult]) -> String {
        let mut s = String::from("審查未通過！\n");
        for a in audits {
            if a.verdict == "REJECTED" {
                s.push_str(&format!("  {} : {}\n", a.agent_name, a.reason));
            }
        }
        s
    }
}

/// 跨檔引用的編譯錯誤碼——單檔無法獨立驗證，遇此跳過交由 crate 層級檢查。
const CRATE_LEVEL_CODES: [&str; 3] = ["E0432", "E0433", "E0583"];

fn stderr_has_real_error(stderr: &str) -> bool {
    stderr.lines().any(|l| {
        (l.contains("error[") || l.trim_start().starts_with("error:"))
            && !l.contains("aborting due to")
    })
}

/// 寫檔後沙盒硬性對齊：以 rustc 對單一 .rs 檔做編譯檢查（--emit=metadata，不產生工件）。
/// 防堵「寫入了編譯不過的代碼卻回報 SUCCESS」的虛假回報缺口——cargo check 只涵蓋
/// build graph 內的檔案，孤兒檔案需要單檔檢查。
/// 限制：單檔檢查無法解析跨檔引用；偵測到 E0432/E0433/E0583 時整檔跳過。rustc 不存在時跳過。
pub fn check_rs_compiles(path: &std::path::Path, max_lines: usize) -> Option<String> {
    let meta_out = std::env::temp_dir().join("agnes_align.rmeta");
    let output = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "lib", "--emit", "metadata", "-o"])
        .arg(&meta_out)
        .arg(path)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(_) => return None, // rustc 不可用：跳過對齊（不阻斷非 Rust 環境）
    };
    if output.status.code() == Some(0) {
        return None;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if CRATE_LEVEL_CODES.iter().any(|code| stderr.contains(code)) {
        return None;
    }
    if !stderr_has_real_error(&stderr) {
        return None;
    }
    Some(stderr.lines().take(max_lines).collect::<Vec<_>>().join("\n"))
}

/// 沙盒硬性對齊第二階段：若 .rs 含 #[cfg(test)]，用 `rustc --test` 編譯成測試執行檔
/// 並實際執行，取真實 Exit Code。防堵「測試斷言邏輯錯誤卻回報 SUCCESS」——
/// metadata 編譯不會評估 cfg(test) 代碼，故編譯檢查抓不到測試失敗，必須真的跑。
/// 自包含檔案才驗證；跨檔引用（CRATE_LEVEL_CODES）或無測試時跳過。rustc 不存在時跳過。
pub fn run_rs_tests(path: &std::path::Path, max_lines: usize) -> Option<String> {
    // 無測試模組則無需執行
    let Ok(src) = std::fs::read_to_string(path) else { return None };
    if !src.contains("#[test]") && !src.contains("#[cfg(test)]") {
        return None;
    }
    // 唯一 binary 名：pid + 奈秒時戳，避免同行程內並行對齊互相覆蓋
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let bin_name = format!("agnes_align_test_{}_{}", std::process::id(), unique);
    let mut test_bin = std::env::temp_dir().join(bin_name);
    if cfg!(target_os = "windows") {
        test_bin.set_extension("exe");
    }
    // 編譯測試執行檔
    let compile = std::process::Command::new("rustc")
        .args(["--test", "--edition", "2021", "-o"])
        .arg(&test_bin)
        .arg(path)
        .output();
    let compile = match compile {
        Ok(o) => o,
        Err(_) => return None,
    };
    if compile.status.code() != Some(0) {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        if CRATE_LEVEL_CODES.iter().any(|code| stderr.contains(code)) {
            return None; // 跨檔依賴，無法獨立跑測試
        }
        // 編譯期錯誤已由 check_rs_compiles 處理，這裡不重複報
        return None;
    }
    // 執行測試（locale 校準確保中文輸出不亂碼），取真實 Exit Code
    let mut run_cmd = std::process::Command::new(&test_bin);
    crate::locale::set_locale_env(&mut run_cmd, None, None);
    let run = match run_cmd.output() {
        Ok(o) => o,
        Err(_) => {
            let _ = std::fs::remove_file(&test_bin);
            return None;
        }
    };
    let _ = std::fs::remove_file(&test_bin);
    if run.status.code() == Some(0) {
        return None; // 測試全綠
    }
    // 失敗：擷取 stdout（測試斷言細節）+ stderr（panic）
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    let mut summary: Vec<String> = stdout.lines()
        .filter(|l| l.contains("FAILED") || l.contains("panicked") || l.contains("left:") || l.contains("right:") || l.contains("test result"))
        .map(str::to_string)
        .collect();
    summary.extend(stderr.lines().filter(|l| l.contains("panicked")).map(str::to_string));
    summary.truncate(max_lines);
    Some(summary.join("\n"))
}

/// 引號感知的指令切割：支援 "雙引號" 與 '單引號' 包住含空白的引數
/// （split_whitespace 會把 "C:\\Program Files\\x" 切碎）。
pub fn split_command_line(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in input.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else {
                    current.push(c);
                }
            }
            None => match c {
                '"' | '\'' => quote = Some(c),
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        parts.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(c),
            },
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// 工作區路徑正規化：Windows 8.3 短檔名（如 MASTER~1）與長路徑在前綴比對時
/// 不相等，會讓路徑圈禁誤判越權、阻擋一切寫入。統一展開為去 \\?\ 前綴的長路徑。
fn normalize_workspace(p: PathBuf) -> PathBuf {
    if p.as_os_str().is_empty() {
        return p;
    }
    match std::fs::canonicalize(&p) {
        Ok(c) => {
            let s = c.to_string_lossy().to_string();
            PathBuf::from(s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s))
        }
        Err(_) => p,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// AgentLoop — main orchestration entry point
// ──────────────────────────────────────────────────────────────────────────────

pub struct AgentLoop {
    pub config: Config,
    pub workspace_path: PathBuf,
}

impl AgentLoop {
    pub fn new(config: Config, workspace_path: String) -> Self {
        Self {
            config,
            workspace_path: normalize_workspace(PathBuf::from(workspace_path)),
        }
    }

    pub fn run_audits(&self, tool_calls: &[ToolCall], messages: &[serde_json::Value]) -> Vec<AuditResult> {
        AgentEngine.run_validation(&self.config, tool_calls, messages)
    }

    fn get_repair_prompt(&self, audits: &[AuditResult]) -> String {
        let mut instructions = Vec::new();
        for a in audits {
            if a.verdict == "REJECTED" {
                if a.reason.contains("G3") {
                    instructions.push("Avoid decorative AI words. Use concrete technical terminology.");
                } else if a.reason.contains("G4") {
                    instructions.push("Markdown files must go to Docs/ and TOON files to .agent/rules/.");
                } else if a.reason.contains("G6") {
                    instructions.push("Do not include Chromium or WebView dependencies (webkit, chromium, playwright) in Cargo.toml.");
                } else if a.reason.contains("G11") {
                    instructions.push("Pass arguments as a vector/array, do not use shell string concatenation with ; | & $ `.");
                } else if a.reason.contains("G12") {
                    instructions.push("Do not hardcode API keys (sk-...). Use {{API_KEY}} or environment variables.");
                } else if a.reason.contains("G13") {
                    instructions.push("Do not leave TODO or unimplemented! placeholders in the code.");
                } else if a.reason.contains("G14") {
                    instructions.push("Do not use blocking HTTP client calls; use async reqwest.");
                } else {
                    instructions.push("Correct the tool call structure or arguments according to the validation error.");
                }
            }
        }
        if instructions.is_empty() {
            "Please correct the tool call instructions according to validation errors.".to_string()
        } else {
            instructions.join(" ")
        }
    }

    pub async fn run_step(
        &self,
        messages: &mut Vec<serde_json::Value>,
        mcp_manager: &McpManager,
        token_budgeter: &tokio::sync::Mutex<crate::TokenBudgeter>,
        db_path: &std::path::Path,
    ) -> Result<AgentStep, String> {
        let api_key = &self.config.api.key;
        if api_key.is_empty() {
            return Err("API key 未設定，無法連接 API 服務。".to_string());
        }

        if token_budgeter.lock().await.is_locked() {
            return Err("Token budget exceeded! Session budget has been locked.".to_string());
        }

        let api_url = self.config.api.base_url.clone();
        let model = self.config.api.model.clone();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.api.timeout_seconds))
            .build()
            .map_err(|e| format!("無法初始化 HTTP 客戶端: {}", e))?;

        let memory_manager = crate::MemoryManager::new(self.workspace_path.clone());
        let mut rag_context = String::new();
        let last_user_prompt = messages.iter()
            .rev()
            .find(|m| m["role"] == "user")
            .and_then(|m| m["content"].as_str())
            .unwrap_or("");
        
        if !last_user_prompt.is_empty() {
            let mut bypassed = false;
            // Stage 0: FTS5 local search bypass
            if let Ok(conn) = crate::open_connection(db_path) {
                let threshold = self.config.memory.local_hit_threshold;
                if let Ok(files) = memory_manager.stage0_local_fts5(&conn, last_user_prompt, threshold) {
                    if !files.is_empty() {
                        let full_paths: Vec<PathBuf> = files.iter()
                            .map(|f| memory_manager.workspace_path.join(f))
                            .collect();
                        rag_context = memory_manager.stage3_inject_contents(&full_paths);
                        bypassed = true;
                    }
                }
            }

            if !bypassed {
                if let Ok(tags) = memory_manager.stage1_find_tags(&client, &api_url, api_key, &model, last_user_prompt).await {
                    if !tags.is_empty() {
                        if let Ok(files) = memory_manager.stage2_find_files(&client, &api_url, api_key, &model, last_user_prompt, &tags).await {
                            rag_context = memory_manager.stage3_inject_contents(&files);
                        }
                    }
                }
            }
        }

        // Claude 互通層：CLAUDE.md 專案規則 + .claude/skills 技能（確定性檔案解析，
        // 修復迴圈外只算一次，0 API 成本）
        let skills_context =
            crate::skills::build_skills_system_prompt(&self.workspace_path, last_user_prompt);

        // 已連線 MCP 伺服器的工具清單——不注入的話模型不知道 run_mcp 能呼叫什麼
        let mcp_tools_context = {
            let all_tools = mcp_manager.get_all_tools().await;
            crate::skills::build_mcp_tools_prompt(&all_tools)
        };

        let mut repair_attempts = 0;
        let max_repairs = self.config.api.max_repairs;

        loop {
            if token_budgeter.lock().await.is_locked() {
                return Err("Token budget exceeded! Session budget has been locked.".to_string());
            }

            let mut request_messages = messages.clone();
            // Inject general system instructions prompt at the start
            request_messages.insert(0, serde_json::json!({
                "role": "system",
                "content": AGNES_SYSTEM_PROMPT
            }));
            
            // Claude 互通層與 RAG 依序排在主系統提示之後
            let mut insert_at = 1;
            if let Some(ctx) = &skills_context {
                request_messages.insert(insert_at, serde_json::json!({
                    "role": "system",
                    "content": ctx,
                }));
                insert_at += 1;
            }
            if let Some(ctx) = &mcp_tools_context {
                request_messages.insert(insert_at, serde_json::json!({
                    "role": "system",
                    "content": ctx,
                }));
                insert_at += 1;
            }

            // Inject RAG context as a system message if it is not empty
            if !rag_context.is_empty() {
                request_messages.insert(insert_at, serde_json::json!({
                    "role": "system",
                    "content": format!("Here is relevant historical context retrieved from memory:\n{}", rag_context)
                }));
            }

            let payload = serde_json::json!({
                "model": model,
                "messages": request_messages,
                "temperature": 0.2,
            });

            let response = client
                .post(&api_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&payload)
                .send()
                .await;

            let response_text = match response {
                Ok(res) => {
                    if res.status().is_success() {
                        let res_json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
                        
                        let usage = &res_json["usage"];
                        let prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0);
                        let completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0);
                        let total_tokens = usage["total_tokens"].as_u64().unwrap_or(0);

                        {
                            let mut budget = token_budgeter.lock().await;
                            budget.record_usage(prompt_tokens, completion_tokens);
                        }

                        if let Ok(conn) = rusqlite::Connection::open(db_path) {
                            let _ = crate::add_token_log(
                                &conn,
                                None,
                                &model,
                                prompt_tokens as i64,
                                completion_tokens as i64,
                                total_tokens as i64,
                                (total_tokens as f64) * self.config.api.cost_per_token,
                            );
                        }

                        res_json["choices"][0]["message"]["content"]
                            .as_str()
                            .unwrap_or("")
                            .to_string()
                    } else {
                        return Err(format!("API 伺服器回傳錯誤代碼: {}", res.status()));
                    }
                }
                Err(e) => return Err(format!("無法連接 API 伺服器: {}", e)),
            };

            let tool_calls = self.parse_tool_calls(&response_text);
            
            // Temporarily push the current assistant response for auditing
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": response_text.clone(),
            }));
            
            let audits = self.run_audits(&tool_calls, messages);
            
            // Pop the temporary assistant message
            messages.pop();
            
            let any_rejected = AgentEngine::any_rejected(&audits);

            if !any_rejected {
                // 核准語意：自動審查關閉 → 必須人工核准；全域模式 → 一律逐項核准
                // （confirmation_gate.toon：全域模式所有風險級 auto_approve=false）。
                // 舊欄位 require_approval 不再參與判斷——它曾被舊設定頁誤存為 false，
                // 會讓核准流程永遠不可達。
                let requires_approval = (!self.config.security.auto_review
                    || self.config.general.project_mode == "global")
                    && !tool_calls.is_empty();

                let mut execution_results = Vec::new();
                if !requires_approval {
                    for tool in &tool_calls {
                        let result = self.execute_tool(tool, mcp_manager).await;
                        execution_results.push(result);
                    }

                    // 寫檔後沙盒硬性對齊：.rs 必須通過編譯檢查，失敗砸回真實 rustc 錯誤自愈
                    if let Some(align_err) = self.post_write_alignment(&tool_calls) {
                        repair_attempts += 1;
                        if repair_attempts <= max_repairs {
                            messages.push(serde_json::json!({
                                "role": "assistant",
                                "content": response_text.clone(),
                            }));
                            messages.push(serde_json::json!({
                                "role": "user",
                                "content": format!(
                                    "[沙盒硬性對齊 REJECT] 你寫入的 Rust 代碼未通過編譯檢查。真實 rustc 錯誤：\n{}\n請用 write_file 重新寫入修復後的完整檔案。",
                                    align_err,
                                ),
                            }));
                            continue;
                        }
                        execution_results.push(format!(
                            "[SANDBOX REJECT] 編譯對齊失敗且自愈次數用盡：\n{}",
                            align_err,
                        ));
                    }
                }

                // 終端蒸餾歸檔：對話 token 增量觸及閾值時，喚醒第一組蒸餾管線。
                // 水位記號：只在「相對上次蒸餾再增長一個閾值」時觸發，防止每輪重複蒸餾。
                let conversation_text: String = messages.iter()
                    .filter_map(|m| m["content"].as_str())
                    .chain(std::iter::once(response_text.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
                let conv_tokens = crate::memory::estimate_tokens(&conversation_text) as i64;
                let conv_hash = {
                    use sha2::{Digest, Sha256};
                    let first_user = messages.iter()
                        .find(|m| m["role"] == "user")
                        .and_then(|m| m["content"].as_str())
                        .unwrap_or("");
                    let mut hasher = Sha256::new();
                    hasher.update(first_user.as_bytes());
                    format!("{:x}", hasher.finalize())
                };
                let last_distill_tokens = crate::open_connection(db_path)
                    .ok()
                    .and_then(|conn| crate::get_distill_marker(&conn, &conv_hash).ok())
                    .unwrap_or(0);
                if conv_tokens - last_distill_tokens
                    >= self.config.memory.distill_trigger_delta as i64
                {
                    // 蒸餾 await 階段不持有 SQLite 連線（Connection 非 Sync）
                    match memory_manager.distill_text(
                        &client, &api_url, api_key, &model,
                        &conversation_text, &self.config.memory,
                    ).await {
                        Ok(distilled) => {
                            let slug = format!("conv_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                            match crate::open_connection(db_path).map_err(|e| e.to_string())
                                .and_then(|conn| {
                                    let path = memory_manager.save_memory(
                                        &conn, "conversations", &slug, &distilled, &self.config.memory,
                                    )?;
                                    let _ = crate::set_distill_marker(&conn, &conv_hash, conv_tokens);
                                    Ok(path)
                                })
                            {
                                Ok(path) => execution_results.push(
                                    format!("[MEMORY] 對話已蒸餾歸檔: {}", path.display()),
                                ),
                                Err(e) => execution_results.push(
                                    format!("[MEMORY] 蒸餾歸檔失敗（不影響任務結果）: {}", e),
                                ),
                            }
                        }
                        Err(e) => execution_results.push(
                            format!("[MEMORY] 蒸餾失敗（不影響任務結果）: {}", e),
                        ),
                    }
                }

                return Ok(AgentStep {
                    response_text,
                    executed_tools: tool_calls,
                    execution_results,
                    audits,
                    requires_approval,
                });
            }

            // If audited rejected, trigger self-repair
            repair_attempts += 1;
            if repair_attempts > max_repairs {
                // Stop retrying, return failed step
                let mut execution_results = Vec::new();
                execution_results.push(format!("[AUDIT REJECTED] Max repair attempts reached: {}", AgentEngine::rejection_details(&audits)));
                
                return Ok(AgentStep {
                    response_text,
                    executed_tools: tool_calls,
                    execution_results,
                    audits,
                    requires_approval: false,
                });
            }

            let repair_prompt = self.get_repair_prompt(&audits);
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": response_text.clone(),
            }));
            messages.push(serde_json::json!({
                "role": "user",
                "content": format!(
                    "[QA REPAIR ATTEMPT {}/{}] The prior response failed validation errors:\n{}\n\nCorrection instruction: {}",
                    repair_attempts, max_repairs,
                    AgentEngine::rejection_details(&audits),
                    repair_prompt
                ),
            }));
        }
    }

    /// 寫檔後對所有 .rs 檔執行編譯對齊，回傳第一個真實錯誤。
    fn post_write_alignment(&self, tool_calls: &[ToolCall]) -> Option<String> {
        for tool in tool_calls {
            if tool.name != "write_file" {
                continue;
            }
            let Some(ref rel) = tool.path else { continue };
            if !rel.ends_with(".rs") {
                continue;
            }
            let Ok(full) = self.canonicalize_workspace_path(rel) else { continue };
            let lines = self.config.sandbox.stderr_feedback_lines;
            // 階段一：編譯檢查（抓編譯錯誤，如缺生命週期標註）
            if let Some(err) = check_rs_compiles(&full, lines) {
                return Some(format!("{}（編譯失敗）:\n{}", rel, err));
            }
            // 階段二：真實執行測試（抓測試斷言邏輯錯誤）
            if let Some(err) = run_rs_tests(&full, lines) {
                return Some(format!("{}（測試失敗）:\n{}", rel, err));
            }
        }
        None
    }

    /// Canonicalize a path relative to the workspace, blocking path traversal.
    /// In "global" mode (project_mode == "global"), full system access is allowed.
    fn canonicalize_workspace_path(&self, relative: &str) -> Result<PathBuf, String> {
        let raw = PathBuf::from(relative);

        // If the path already contains traversal components, block it
        if raw.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err("路徑穿越嘗試已拒絕：路徑包含 '..' 元件！".to_string());
        }

        let resolved = self.workspace_path.join(&raw);

        // In global mode, resolve as-is without restriction
        if self.config.general.project_mode == "global" {
            return Ok(resolved);
        }

        // Helper to strip Windows UNC prefix \\?\
        let strip_unc = |path: &std::path::Path| -> std::path::PathBuf {
            let s = path.to_string_lossy();
            match s.strip_prefix(r"\\?\") {
                Some(stripped) => std::path::PathBuf::from(stripped),
                None => path.to_path_buf(),
            }
        };

        // In project mode, verify the resolved path stays within workspace
        if let Ok(canonical) = std::fs::canonicalize(&resolved) {
            if let Ok(ws_canon) = std::fs::canonicalize(&self.workspace_path) {
                let clean_canon = strip_unc(&canonical);
                let clean_ws = strip_unc(&ws_canon);
                if !clean_canon.starts_with(&clean_ws) {
                    return Err(format!(
                        "路徑越權：解析路徑 {} 不在工作區 {} 內！",
                        clean_canon.display(),
                        clean_ws.display()
                    ));
                }
            }
        }

        // For non-existent files, do a manual prefix check
        if let Ok(ws_canon) = std::fs::canonicalize(&self.workspace_path) {
            let clean_ws = strip_unc(&ws_canon);
            let clean_resolved = strip_unc(&resolved);
            if !clean_resolved.starts_with(&clean_ws) {
                return Err(format!(
                    "路徑越權：解析路徑 {} 不在工作區 {} 內！",
                    clean_resolved.display(),
                    clean_ws.display()
                ));
            }
        }

        Ok(resolved)
    }

    pub fn parse_tool_calls(&self, text: &str) -> Vec<ToolCall> {
        let mut tool_calls = Vec::new();
        let mut start_idx = 0;

        // 1. write_file
        while let Some(open_tag) = text[start_idx..].find("<write_file path=\"") {
            let actual_open = start_idx + open_tag;
            let path_start = actual_open + "<write_file path=\"".len();
            if let Some(path_end) = text[path_start..].find('"') {
                let actual_path_end = path_start + path_end;
                let path = text[path_start..actual_path_end].to_string();
                let content_start = actual_path_end + 2;
                if let Some(close_tag) = text[content_start..].find("</write_file>") {
                    let actual_close = content_start + close_tag;
                    let content = text[content_start..actual_close].to_string();
                    tool_calls.push(ToolCall {
                        name: "write_file".to_string(),
                        path: Some(path),
                        content,
                    });
                    start_idx = actual_close + "</write_file>".len();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // 2. read_file
        start_idx = 0;
        while let Some(open_tag) = text[start_idx..].find("<read_file path=\"") {
            let actual_open = start_idx + open_tag;
            let path_start = actual_open + "<read_file path=\"".len();
            if let Some(path_end) = text[path_start..].find('"') {
                let actual_path_end = path_start + path_end;
                let path = text[path_start..actual_path_end].to_string();
                tool_calls.push(ToolCall {
                    name: "read_file".to_string(),
                    path: Some(path),
                    content: String::new(),
                });
                start_idx = actual_path_end + 3;
            } else {
                break;
            }
        }

        // 3. run_command
        start_idx = 0;
        while let Some(open_tag) = text[start_idx..].find("<run_command>") {
            let actual_open = start_idx + open_tag;
            let cmd_start = actual_open + "<run_command>".len();
            if let Some(close_tag) = text[cmd_start..].find("</run_command>") {
                let actual_close = cmd_start + close_tag;
                let content = text[cmd_start..actual_close].trim().to_string();
                tool_calls.push(ToolCall {
                    name: "run_command".to_string(),
                    path: None,
                    content,
                });
                start_idx = actual_close + "</run_command>".len();
            } else {
                break;
            }
        }

        // 4. run_mcp
        start_idx = 0;
        while let Some(open_tag) = text[start_idx..].find("<run_mcp server=\"") {
            let actual_open = start_idx + open_tag;
            let server_start = actual_open + "<run_mcp server=\"".len();
            if let Some(server_end) = text[server_start..].find('"') {
                let actual_server_end = server_start + server_end;
                let server = text[server_start..actual_server_end].to_string();
                let tool_part = &text[actual_server_end..];
                if let Some(tool_attr) = tool_part.find("tool=\"") {
                    let tool_start = actual_server_end + tool_attr + "tool=\"".len();
                    if let Some(tool_end) = text[tool_start..].find('"') {
                        let actual_tool_end = tool_start + tool_end;
                        let tool_name = text[tool_start..actual_tool_end].to_string();
                        let content_start = actual_tool_end + 2;
                        if let Some(close_tag) = text[content_start..].find("</run_mcp>") {
                            let actual_close = content_start + close_tag;
                            let content = text[content_start..actual_close].to_string();
                            tool_calls.push(ToolCall {
                                name: format!("mcp:{}:{}", server, tool_name),
                                path: None,
                                content,
                            });
                            start_idx = actual_close + "</run_mcp>".len();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        tool_calls
    }

    pub async fn execute_tool(&self, tool: &ToolCall, mcp_manager: &McpManager) -> String {
        if tool.name.starts_with("mcp:") {
            let parts: Vec<&str> = tool.name.split(':').collect();
            if parts.len() >= 3 {
                let server = parts[1];
                let tool_name = parts[2];
                let args_json: serde_json::Value = serde_json::from_str(&tool.content)
                    .unwrap_or(serde_json::Value::Null);
                match mcp_manager.call_mcp_tool(server, tool_name, args_json).await {
                    Ok(res) => res,
                    Err(e) => format!("[MCP ERROR] {}", e),
                }
            } else {
                "[ERROR] 無效的 MCP 工具呼叫格式".to_string()
            }
        } else {
            match tool.name.as_str() {
                "write_file" => {
                    if let Some(ref relative_path) = tool.path {
                        match self.canonicalize_workspace_path(relative_path) {
                            Ok(full_path) => {
                                if let Some(parent) = full_path.parent() {
                                    if let Err(e) = fs::create_dir_all(parent) {
                                        return format!("[ERROR] 無法建立目錄: {}", e);
                                    }
                                }
                                // Strip sensitive key content from stored content
                                let safe_content = self.strip_secrets(&tool.content);
                                match fs::write(&full_path, &safe_content) {
                                    Ok(_) => format!("[SUCCESS] 成功寫入檔案: {}", relative_path),
                                    Err(e) => format!("[ERROR] 無法寫入檔案 {}: {}", relative_path, e),
                                }
                            }
                            Err(e) => format!("[ERROR] 路徑驗證失敗: {}", e),
                        }
                    } else {
                        "[ERROR] 寫入檔案缺少路徑屬性".to_string()
                    }
                }
                "read_file" => {
                    if let Some(ref relative_path) = tool.path {
                        match self.canonicalize_workspace_path(relative_path) {
                            Ok(full_path) => match fs::read_to_string(&full_path) {
                                Ok(content) => content,
                                Err(e) => format!("[ERROR] 無法讀取檔案 {}: {}", relative_path, e),
                            },
                            Err(e) => format!("[ERROR] 路徑驗證失敗: {}", e),
                        }
                    } else {
                        "[ERROR] 讀取檔案缺少路徑屬性".to_string()
                    }
                }
                "run_command" => {
                    let cmd_parts: Vec<String> = split_command_line(&tool.content);
                    if cmd_parts.is_empty() {
                        return "[ERROR] 指令為空".to_string();
                    }
                    let program = cmd_parts[0].as_str();
                    let args: Vec<&str> = cmd_parts[1..].iter().map(|s| s.as_str()).collect();
                    let workspace = if self.config.general.project_mode == "project" {
                        Some(&self.workspace_path)
                    } else {
                        None
                    };
                    let result = sandbox::run_in_sandbox(
                        program,
                        &args,
                        &self.config.general.shell,
                        self.config.security.full_access,
                        workspace,
                    );
                    format!(
                        "Exit Code: {} | Is Aligned Success: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                        result.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "None".to_string()),
                        result.is_aligned_success,
                        result.stdout,
                        result.stderr,
                    )
                }
                _ => format!("[ERROR] 未知工具: {}", tool.name),
            }
        }
    }

    /// Remove secret/key values from content before writing.
    fn strip_secrets(&self, content: &str) -> String {
        let re = regex::Regex::new(r"sk-[a-zA-Z0-9]{10,}").unwrap();
        if re.is_match(content) {
            re.replace_all(content, "[REDACTED_KEY]").to_string()
        } else {
            content.to_string()
        }
    }
}
