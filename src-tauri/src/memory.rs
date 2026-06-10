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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub md_token_cap: usize,
    pub context_high_watermark: usize,
    pub chunk_size: usize,
    pub overlap_lines: usize,
    pub distill_trigger_delta: usize,
    pub local_hit_threshold: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            md_token_cap: 2000,
            context_high_watermark: 800000,
            chunk_size: 100000,
            overlap_lines: 50,
            distill_trigger_delta: 50000,
            local_hit_threshold: 0.8,
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

/// Dynamic RAG Funnel retrieval system.
pub struct MemoryManager {
    pub workspace_path: PathBuf,
    pub memory_tags_path: PathBuf,
}

impl MemoryManager {
    pub fn new(workspace_path: PathBuf) -> Self {
        let memory_tags_path = workspace_path.join("memory_tags");
        Self {
            workspace_path,
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

    /// Stage 1: Find Tag Folders (Domains).
    /// Calls the model to select matching directories under memory_tags/
    pub async fn stage1_find_tags(
        &self,
        client: &reqwest::Client,
        api_url: &str,
        api_key: &str,
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

        let payload = serde_json::json!({
            "model": "agnes-2.0-flash",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_msg}
            ],
            "temperature": 0.1,
        });

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
        
        let text = res_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim();

        let parsed: Vec<String> = serde_json::from_str(text)
            .or_else(|_| {
                let cleaned = text.trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                serde_json::from_str(cleaned)
            })
            .unwrap_or_default();

        let matched: Vec<String> = parsed.into_iter()
            .filter(|t| tag_folders.contains(t))
            .collect();

        Ok(matched)
    }

    /// Stage 2: Find Relevant Files.
    /// Calls the model to select matching .md files in the selected tag folders.
    pub async fn stage2_find_files(
        &self,
        client: &reqwest::Client,
        api_url: &str,
        api_key: &str,
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

        let payload = serde_json::json!({
            "model": "agnes-2.0-flash",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_msg}
            ],
            "temperature": 0.1,
        });

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
        
        let text = res_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim();

        let parsed: Vec<String> = serde_json::from_str(text)
            .or_else(|_| {
                let cleaned = text.trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                serde_json::from_str(cleaned)
            })
            .unwrap_or_default();

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
}

