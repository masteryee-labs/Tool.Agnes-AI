use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use rusqlite::Connection;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub index: usize,
    pub text: String,
    pub overlap_head: String,
    pub overlap_tail: String,
}

fn default_memory_max_repairs() -> usize {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub md_token_cap: usize,
    pub context_high_watermark: usize,
    pub chunk_size: usize,
    pub overlap_lines: usize,
    pub distill_trigger_delta: usize,
    pub local_hit_threshold: f64,
    #[serde(default = "default_memory_max_repairs")]
    pub max_repairs: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            md_token_cap: 2000,
            context_high_watermark: 800000,
            chunk_size: 100000,
            overlap_lines: 50,
            distill_trigger_delta: 50000,
            local_hit_threshold: 0.65,
            max_repairs: 3,
        }
    }
}

/// Token estimator (CJK character ≈ 1 token, ASCII ≈ 4 characters / token).
/// Runs purely locally with 0 token cost.
pub fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0;
    let mut ascii_len = 0;
    for c in text.chars() {
        if c.is_ascii() {
            ascii_len += 1;
        } else {
            if ascii_len > 0 {
                tokens += usize::div_ceil(ascii_len, 4);
                ascii_len = 0;
            }
            tokens += 1;
        }
    }
    if ascii_len > 0 {
        tokens += usize::div_ceil(ascii_len, 4);
    }
    tokens
}

/// Sliding window chunking based on lines and tokens.
pub fn sliding_window_chunk(text: &str, chunk_size_tokens: usize, overlap_lines: usize) -> Vec<Chunk> {
    let lines: Vec<&str> = text.lines().collect();
    let mut chunks = Vec::new();
    if lines.is_empty() {
        return chunks;
    }

    let mut start_line = 0;
    let mut chunk_index = 0;

    while start_line < lines.len() {
        let mut end_line = start_line;
        let mut current_tokens = 0;
        
        while end_line < lines.len() {
            let line_tokens = estimate_tokens(lines[end_line]) + 1;
            if current_tokens + line_tokens > chunk_size_tokens && end_line > start_line {
                break;
            }
            current_tokens += line_tokens;
            end_line += 1;
        }

        let chunk_lines = &lines[start_line..end_line];
        let chunk_text = chunk_lines.join("\n");

        let overlap_head = if chunk_index > 0 && chunk_lines.len() >= overlap_lines {
            chunk_lines[..overlap_lines].join("\n")
        } else {
            String::new()
        };

        let overlap_tail = if end_line < lines.len() && chunk_lines.len() >= overlap_lines {
            chunk_lines[chunk_lines.len() - overlap_lines..].join("\n")
        } else {
            String::new()
        };

        chunks.push(Chunk {
            index: chunk_index,
            text: chunk_text,
            overlap_head,
            overlap_tail,
        });

        let next_start = if end_line < lines.len() {
            if end_line > overlap_lines {
                end_line - overlap_lines
            } else {
                end_line
            }
        } else {
            end_line
        };

        if next_start <= start_line {
            start_line = end_line;
        } else {
            start_line = next_start;
        }
        chunk_index += 1;
    }

    chunks
}

/// Parse a JSON string array from model output, tolerating ```json fences.
fn parse_json_string_array(text: &str) -> Vec<String> {
    serde_json::from_str(text)
        .or_else(|_| {
            let cleaned = text.trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            serde_json::from_str(cleaned)
        })
        .unwrap_or_default()
}

/// Deterministic distillation audit (TokenOverlapAuditor, 0 token).
/// Rejects empty output and "distillation" that grew instead of shrinking.
pub fn audit_distillation(original: &str, distilled: &str) -> Result<(), String> {
    if distilled.trim().is_empty() {
        return Err("[REJECT: TokenOverlapAuditor | 蒸餾結果為空]".to_string());
    }
    let orig_tokens = estimate_tokens(original);
    let dist_tokens = estimate_tokens(distilled);
    if dist_tokens >= orig_tokens {
        return Err(format!(
            "[REJECT: TokenOverlapAuditor | 蒸餾後 token ({}) 未小於原文 ({})]",
            dist_tokens, orig_tokens
        ));
    }
    Ok(())
}

/// Dynamic RAG Funnel retrieval system.
pub struct MemoryManager {
    pub workspace_path: PathBuf,
    pub memory_tags_path: PathBuf,
    /// 跨 Session 記憶目錄（.agent/memory/）
    pub agent_memory_path: PathBuf,
}

impl MemoryManager {
    pub fn new(workspace_path: PathBuf) -> Self {
        let memory_tags_path = workspace_path.join("memory_tags");
        let agent_memory_path = workspace_path.join(".agent").join("memory");
        Self {
            workspace_path,
            agent_memory_path,
            memory_tags_path,
        }
    }

    /// Stage 0: Pure SQLite FTS5 search (0 token).
    /// Returns files that score high enough.
    pub fn stage0_local_fts5(
        &self,
        conn: &Connection,
        query: &str,
        threshold: f64,
    ) -> Result<Vec<PathBuf>, String> {
        let mut stmt = conn.prepare(
            "SELECT file_path, score FROM (
                SELECT file_path, bm25(memory_index) as score 
                FROM memory_index 
                WHERE memory_index MATCH ?1
             ) WHERE score <= ?2 ORDER BY score ASC"
        ).map_err(|e| format!("SQLite FTS5 query prepare failed: {}", e))?;

        // bm25 scores are negative (lower is better/more relevant in rusqlite standard bm25)
        // Let's retrieve files where bm25 score indicates high match.
        let rows = stmt.query_map([query, &(-threshold).to_string()], |row| {
            let path_str: String = row.get(0)?;
            Ok(PathBuf::from(path_str))
        }).map_err(|e| format!("SQLite FTS5 query failed: {}", e))?;

        let paths: Vec<PathBuf> = rows.flatten().collect();
        Ok(paths)
    }

    /// Distill a memory text and write to tag folder. Splits if it exceeds the token cap.
    pub fn save_memory(
        &self,
        conn: &Connection,
        tag_folder: &str,
        summary_slug: &str,
        content: &str,
        cfg: &MemoryConfig,
    ) -> Result<PathBuf, String> {
        let cap = cfg.md_token_cap;
        let overlap = cfg.overlap_lines;
        let tokens = estimate_tokens(content);
        if tokens > cap {
            let chunks = sliding_window_chunk(content, cap, overlap);
            let mut last_path = PathBuf::new();
            for (idx, chunk) in chunks.iter().enumerate() {
                let chunk_slug = format!("{}_part{}", summary_slug, idx + 1);
                last_path = self.save_single_memory_file(conn, tag_folder, &chunk_slug, &chunk.text)?;
            }
            Ok(last_path)
        } else {
            self.save_single_memory_file(conn, tag_folder, summary_slug, content)
        }
    }

    fn save_single_memory_file(
        &self,
        conn: &Connection,
        tag_folder: &str,
        summary_slug: &str,
        content: &str,
    ) -> Result<PathBuf, String> {
        let folder = self.memory_tags_path.join(tag_folder);
        if let Err(e) = fs::create_dir_all(&folder) {
            return Err(format!("Failed to create memory tags folder: {}", e));
        }

        let uuid = Uuid::new_v4().to_string();
        let short_uuid = &uuid[..8];
        let file_name = format!("{}_{}.md", short_uuid, summary_slug);
        let file_path = folder.join(&file_name);

        if let Err(e) = fs::write(&file_path, content) {
            return Err(format!("Failed to write memory file: {}", e));
        }

        // Index in SQLite FTS5
        let relative_path = file_path.strip_prefix(&self.workspace_path)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .to_string();

        conn.execute(
            "INSERT OR REPLACE INTO memory_index (file_path, tag, content) VALUES (?1, ?2, ?3)",
            rusqlite::params![relative_path, tag_folder, content],
        ).map_err(|e| format!("Failed to index memory in SQLite FTS5: {}", e))?;

        Ok(file_path)
    }

    /// Single LLM call helper shared by funnel stages and distillation agents.
    /// 共用全域令牌桶：呼叫前先 acquire，確保記憶管線的每次 Agnes API 呼叫都
    /// 計入 20 RPM 上限，蒸餾組 alpha/beta/integrator 連發也不會突破限速觸發 429。
    /// 金鑰由共享 `KeyRotator` 提供，每次呼叫取一把 Key（多帳號輪詢）。
    #[allow(clippy::too_many_arguments)]
    async fn llm_call(
        client: &reqwest::Client,
        limiter: &crate::rate_limiter::RateLimiter,
        api_url: &str,
        key_rotator: &std::sync::Arc<crate::key_rotation::KeyRotator>,
        model: &str,
        system_prompt: &str,
        user_msg: &str,
        temperature: f64,
    ) -> Result<String, String> {
        let payload = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_msg}
            ],
            "temperature": temperature,
        });

        limiter.acquire().await;
        let api_key = key_rotator.next_key().map_err(|e| e.to_string())?;
        let res = client.post(api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !res.status().is_success() {
            return Err(format!("API returned error status: {}", res.status()));
        }

        let res_json: serde_json::Value = res.json().await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

        Ok(res_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string())
    }

    /// Stage 1: Find Tag Folders (Domains).
    /// Calls the model to select matching directories under memory_tags/
    pub async fn stage1_find_tags(
        &self,
        client: &reqwest::Client,
        limiter: &crate::rate_limiter::RateLimiter,
        api_url: &str,
        key_rotator: &std::sync::Arc<crate::key_rotation::KeyRotator>,
        model: &str,
        user_prompt: &str,
    ) -> Result<Vec<String>, String> {
        if !self.memory_tags_path.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.memory_tags_path)
            .map_err(|e| format!("Failed to read memory_tags: {}", e))?;
        
        let mut tag_folders = Vec::new();
        for e in entries.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = e.file_name().to_str() {
                    tag_folders.push(name.to_string());
                }
            }
        }

        if tag_folders.is_empty() {
            return Ok(Vec::new());
        }

        let system_prompt = "你是一個領域/標籤篩選專家。請根據使用者的提示詞，從給定的標籤資料夾列表中選出最相關的標籤。\n必須僅回傳一個 JSON 格式的字串陣列，例如: [\"git\", \"rust\"]，不要有任何 Markdown 標記（如 ```json）或額外解釋。如果沒有相關標籤，回傳 []。";
        let user_msg = format!("使用者提示詞: {}\n標籤資料夾列表: {:?}", user_prompt, tag_folders);

        let text = Self::llm_call(client, limiter, api_url, key_rotator, model, system_prompt, &user_msg, 0.1).await?;

        let parsed: Vec<String> = parse_json_string_array(&text);

        let matched: Vec<String> = parsed.into_iter()
            .filter(|t| tag_folders.contains(t))
            .collect();

        Ok(matched)
    }

    /// Stage 2: Find Relevant Files.
    /// Calls the model to select matching .md files in the selected tag folders.
    #[allow(clippy::too_many_arguments)]
    pub async fn stage2_find_files(
        &self,
        client: &reqwest::Client,
        limiter: &crate::rate_limiter::RateLimiter,
        api_url: &str,
        key_rotator: &std::sync::Arc<crate::key_rotation::KeyRotator>,
        model: &str,
        user_prompt: &str,
        selected_tags: &[String],
    ) -> Result<Vec<PathBuf>, String> {
        let mut available_files = Vec::new();

        for tag in selected_tags {
            let tag_dir = self.memory_tags_path.join(tag);
            if tag_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(tag_dir) {
                    for e in entries.flatten() {
                        let path = e.path();
                        if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
                            available_files.push(path);
                        }
                    }
                }
            }
        }

        if available_files.is_empty() {
            return Ok(Vec::new());
        }

        let file_names: Vec<String> = available_files.iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
            .collect();

        let system_prompt = "你是一個檔案篩選專家。請根據使用者的提示詞，從給定的 Markdown 檔案列表中選出最可能包含有用歷史記憶的檔案。\n必須僅回傳一個 JSON 格式的字串陣列，例如: [\"file1.md\", \"file2.md\"]，不要有任何 Markdown 標記（如 ```json）或額外解釋。如果沒有相關檔案，回傳 []。";
        let user_msg = format!("使用者提示詞: {}\n檔案列表: {:?}", user_prompt, file_names);

        let text = Self::llm_call(client, limiter, api_url, key_rotator, model, system_prompt, &user_msg, 0.1).await?;

        let parsed: Vec<String> = parse_json_string_array(&text);

        let mut matched_paths = Vec::new();
        for name in parsed {
            for path in &available_files {
                if path.file_name().map(|n| n.to_string_lossy() == name).unwrap_or(false) {
                    matched_paths.push(path.clone());
                    break;
                }
            }
        }

        Ok(matched_paths)
    }

    /// Stage 1 + Stage 2 合併版本：一次 API 呼叫同時選出標籤與檔案（節省 1 RPM）。
    /// 當 Stage 0 (FTS5) 未命中時由 agent.rs 呼叫，取代原先兩次序列 LLM 呼叫。
    #[allow(clippy::too_many_arguments)]
    pub async fn stage12_merged(
        &self,
        client: &reqwest::Client,
        limiter: &crate::rate_limiter::RateLimiter,
        api_url: &str,
        key_rotator: &std::sync::Arc<crate::key_rotation::KeyRotator>,
        model: &str,
        user_prompt: &str,
        cfg: &MemoryConfig,
    ) -> Result<Vec<PathBuf>, String> {
        if !self.memory_tags_path.exists() {
            return Ok(Vec::new());
        }

        // 本地收集所有「tag/file.md」路徑（0 API 成本）
        let mut tag_files: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.memory_tags_path) {
            for entry in entries.flatten() {
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let tag_name = entry.file_name().to_string_lossy().to_string();
                let tag_dir = self.memory_tags_path.join(&tag_name);
                if let Ok(files) = std::fs::read_dir(&tag_dir) {
                    for f in files.flatten() {
                        let path = f.path();
                        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                            let file_name = f.file_name().to_string_lossy().to_string();
                            tag_files.push(format!("{}/{}", tag_name, file_name));
                        }
                    }
                }
            }
        }

        if tag_files.is_empty() {
            return Ok(Vec::new());
        }

        let list = tag_files.join(", ");
        let system_prompt = "你是記憶檔案篩選專家。根據使用者提示詞，從給定的「標籤/檔名」清單中選出最可能含有相關歷史記憶的檔案。\n必須僅回傳一個 JSON 字串陣列，格式：[\"tag1/file1.md\", \"tag2/file2.md\"]，不要 Markdown 標記或額外說明。無相關則回傳 []。";
        let user_msg = format!("使用者提示詞: {}\n可用記憶檔案清單: {}", user_prompt, list);

        let mut attempts = 0;
        let max_repairs = cfg.max_repairs;
        let mut messages = vec![
            serde_json::json!({"role": "system", "content": system_prompt}),
            serde_json::json!({"role": "user", "content": user_msg.clone()})
        ];

        let text = loop {
            let payload = serde_json::json!({
                "model": model,
                "messages": messages,
                "temperature": 0.1,
            });

            limiter.acquire().await;
            let api_key = key_rotator.next_key().map_err(|e| e.to_string())?;
            let res = client.post(api_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("API request failed: {}", e))?;

            if !res.status().is_success() {
                return Err(format!("API returned error status: {}", res.status()));
            }

            let res_json: serde_json::Value = res.json().await
                .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

            let raw_text = res_json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            let cleaned_text = raw_text.replace('\0', "");

            let parsed_result: Result<Vec<String>, _> = serde_json::from_str(&cleaned_text)
                .or_else(|_| {
                    let cleaned = cleaned_text.trim_start_matches("```json")
                        .trim_start_matches("```")
                        .trim_end_matches("```")
                        .trim();
                    serde_json::from_str(cleaned)
                });

            match parsed_result {
                Ok(_) => {
                    break cleaned_text;
                }
                Err(err) => {
                    attempts += 1;
                    if attempts > max_repairs {
                        return Err(format!("Memory tags JSON validation failed after {} attempts: {}", max_repairs, err));
                    }
                    messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": raw_text
                    }));
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": format!(
                            "[REJECT: E_SCHEMA] Your output failed validation check (not a valid JSON array of strings).\nError: {}\n\nPlease only return a JSON array string matching the schema, with no explanation, conversational prefix/suffix, or backticks.",
                            err
                        )
                    }));
                }
            }
        };

        let parsed: Vec<String> = parse_json_string_array(&text);

        let mut matched = Vec::new();
        for entry in parsed {
            let full = self.memory_tags_path.join(&entry);
            if full.exists() && full.extension().map(|e| e == "md").unwrap_or(false) {
                matched.push(full);
            }
        }

        Ok(matched)
    }

    /// Group 1 distillation pipeline (Memory Distillation & Hallucination Defense):
    ///   1. sliding_window_chunk 切塊（重疊區保留邏輯銜接）
    ///   2. ContextDistillerAlpha / Beta 並行壓縮前後半段（flash 級模型）
    ///   3. DistillationIntegrator 整合並消弭斷層
    ///   4. audit_distillation 確定性審查（0 token，一票否決）
    ///
    /// 不持有 SQLite 連線（rusqlite Connection 非 Sync，不能跨 await）；
    /// 歸檔由呼叫端在 await 完成後以 save_memory 執行。
    #[allow(clippy::too_many_arguments)]
    pub async fn distill_text(
        &self,
        client: &reqwest::Client,
        limiter: &crate::rate_limiter::RateLimiter,
        api_url: &str,
        key_rotator: &std::sync::Arc<crate::key_rotation::KeyRotator>,
        model: &str,
        conversation_text: &str,
        cfg: &MemoryConfig,
    ) -> Result<String, String> {
        const DISTILLER_PROMPT: &str = "你是脈絡蒸餾專家。將輸入文本壓縮為高密度的 Markdown 記憶摘要。\n硬性要求：保留所有關鍵參數、數值、檔案路徑、決策與結論；刪除寒暄、重複與過程性對話；禁止新增原文沒有的內容；輸出必須明顯短於原文。";
        const INTEGRATOR_PROMPT: &str = "你是蒸餾邏輯整合官。將前半段與後半段的蒸餾摘要重組為單一連貫的 Markdown 記憶檔。\n硬性要求：消弭兩段交界的斷層與重複（兩段共享重疊區）；保留全部關鍵參數；禁止新增原文沒有的內容；輸出極簡精煉。";

        let chunks = sliding_window_chunk(conversation_text, cfg.chunk_size, cfg.overlap_lines);
        if chunks.is_empty() {
            return Err("蒸餾輸入為空".to_string());
        }

        let mid = usize::div_ceil(chunks.len(), 2);
        let front: String = chunks[..mid].iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n");
        let back: String = chunks[mid..].iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n");

        // ContextDistillerAlpha 與 Beta 並行執行
        let distilled = if back.trim().is_empty() {
            Self::llm_call(client, limiter, api_url, key_rotator, model, DISTILLER_PROMPT, &front, 0.1).await?
        } else {
            let (alpha, beta) = tokio::join!(
                Self::llm_call(client, limiter, api_url, key_rotator, model, DISTILLER_PROMPT, &front, 0.1),
                Self::llm_call(client, limiter, api_url, key_rotator, model, DISTILLER_PROMPT, &back, 0.1),
            );
            let alpha = alpha?;
            let beta = beta?;
            // DistillationIntegrator 重組
            let merged_input = format!("[前半段蒸餾]\n{}\n\n[後半段蒸餾]\n{}", alpha, beta);
            Self::llm_call(client, limiter, api_url, key_rotator, model, INTEGRATOR_PROMPT, &merged_input, 0.1).await?
        };

        // TokenOverlapAuditor 確定性審查（一票否決）
        audit_distillation(conversation_text, &distilled)?;

        Ok(distilled)
    }

    /// Stage 3: Inject File Contents.
    /// Combines selected file contents into a markdown context block.
    pub fn stage3_inject_contents(&self, files: &[PathBuf]) -> String {
        if files.is_empty() {
            return String::new();
        }

        let mut context = String::from("\n=== RAG MEMORY CONTEXT ===\n");
        for file in files {
            if let Ok(content) = std::fs::read_to_string(file) {
                let name = file.file_name().unwrap_or_default().to_string_lossy();
                context.push_str(&format!("[File: {}]\n{}\n\n", name, content));
            }
        }
        context.push_str("==========================\n");
        context
    }

    // ─── 跨 Session 記憶（Phase 5D：.agent/memory/ 三檔）──────────────────────

    /// 確保 .agent/memory/ 目錄存在。
    fn ensure_agent_memory_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.agent_memory_path)
    }

    /// loop_state.md 的完整路徑。
    pub fn loop_state_path(&self) -> PathBuf {
        self.agent_memory_path.join("loop_state.md")
    }

    /// lessons.md 的完整路徑。
    pub fn lessons_path(&self) -> PathBuf {
        self.agent_memory_path.join("lessons.md")
    }

    /// pitfalls.md 的完整路徑。
    pub fn pitfalls_path(&self) -> PathBuf {
        self.agent_memory_path.join("pitfalls.md")
    }

    /// 讀取 loop_state.md（當前任務進度）。
    pub fn read_loop_state(&self) -> String {
        let path = self.loop_state_path();
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    /// 追加 ≤3 行到 loop_state.md（每子任務蒸餾）。
    pub fn append_loop_state(&self, lines: &str) -> std::io::Result<()> {
        self.ensure_agent_memory_dir()?;
        let path = self.loop_state_path();
        let mut content = std::fs::read_to_string(&path).unwrap_or_default();
        content.push_str(lines);
        content.push('\n');
        std::fs::write(&path, content)
    }

    /// 達 40 行時中段蒸餾：已完成子任務壓成 1 行摘要，只留未完成詳細。
    pub fn distill_loop_state(&self) -> std::io::Result<()> {
        let path = self.loop_state_path();
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 40 {
            return Ok(());
        }
        // 取前半壓成摘要 + 後半保留
        let midpoint = lines.len() / 2;
        let summary = format!("[蒸餾] 前 {} 行已完成子任務摘要", midpoint);
        let mut distilled = Vec::new();
        distilled.push(summary);
        distilled.extend(lines[midpoint..].iter().map(|s| s.to_string()));
        std::fs::write(&path, distilled.join("\n"))
    }

    /// 任務完成時清空 loop_state.md + 晉升 1 條 lesson。
    pub fn clear_loop_state_and_promote(&self, lesson: &str) -> std::io::Result<()> {
        self.ensure_agent_memory_dir()?;
        // 清空 loop_state
        std::fs::write(self.loop_state_path(), "")?;
        // 晉升 lesson
        self.append_lesson(lesson)
    }

    /// 讀取 lessons.md（跨 Session 教訓），回傳行列表。
    pub fn read_lessons(&self) -> Vec<String> {
        let path = self.lessons_path();
        std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect()
    }

    /// 追加 1 條 lesson（FIFO 30 條上限）。
    pub fn append_lesson(&self, line: &str) -> std::io::Result<()> {
        self.ensure_agent_memory_dir()?;
        let path = self.lessons_path();
        let mut lessons = self.read_lessons();
        lessons.push(line.to_string());
        // FIFO 30 條上限
        if lessons.len() > 30 {
            let drain = lessons.len() - 30;
            lessons.drain(..drain);
        }
        std::fs::write(&path, lessons.join("\n") + "\n")
    }

    /// 讀取 pitfalls.md（跨 Session 雷庫），回傳行列表。
    pub fn read_pitfalls(&self) -> Vec<String> {
        let path = self.pitfalls_path();
        std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect()
    }

    /// 追加 1 條 pitfall（去重 + 每領域 ≤5 條）。
    pub fn append_pitfall(&self, domain: &str, line: &str) -> std::io::Result<()> {
        self.ensure_agent_memory_dir()?;
        let path = self.pitfalls_path();
        let entry = format!("[{}] {}", domain, line);
        let mut pitfalls = self.read_pitfalls();
        // 去重：完全相同的行不重複加入
        if pitfalls.iter().any(|p| p == &entry) {
            return Ok(());
        }
        // 每領域 ≤5 條：計算同領域已有幾條
        let domain_prefix = format!("[{}] ", domain);
        let domain_count = pitfalls
            .iter()
            .filter(|p| p.starts_with(&domain_prefix))
            .count();
        if domain_count >= 5 {
            // 移除該領域最舊的一條
            if let Some(pos) = pitfalls.iter().position(|p| p.starts_with(&domain_prefix)) {
                pitfalls.remove(pos);
            }
        }
        pitfalls.push(entry);
        std::fs::write(&path, pitfalls.join("\n") + "\n")
    }
}

