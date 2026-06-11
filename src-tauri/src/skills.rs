//! Claude 格式互通層：
//! - `.claude/skills/<name>/SKILL.md`（YAML frontmatter：name / description）
//! - `CLAUDE.md` / `.claude/CLAUDE.md` 專案規則
//! - `.mcp.json`（`{"mcpServers": {"name": {"command", "args", "env"}}}`）
//!
//! 全部為確定性檔案解析，0 token API 成本。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::{
    McpServerConfig, CLAUDE_MD_MAX_CHARS, SKILLS_LIST_MAX, SKILL_BODY_MAX_CHARS,
};

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub dir: PathBuf,
    pub body: String,
}

/// 以字元為單位截斷（位元組切片在 CJK 邊界會 panic）。
fn cap_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars).collect();
    format!("{}\n…(truncated)", truncated)
}

/// 解析 SKILL.md 的 YAML frontmatter（--- 區塊內的 name / description），回傳 (name, description, body)。
/// 無 frontmatter 時 name 取目錄名（由呼叫端補），description 為空。
fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>, String) {
    // Windows 編輯器常以 UTF-8 BOM 存檔——不剝除會讓 frontmatter 偵測失敗（GUI QA 實測抓到）
    let content = content.trim_start_matches('\u{feff}');
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, None, content.to_string());
    }
    let after = &trimmed[3..];
    let Some(end) = after.find("\n---") else {
        return (None, None, content.to_string());
    };
    let header = &after[..end];
    let body = after[end + 4..].trim_start_matches(['\r', '\n']).to_string();

    let mut name = None;
    let mut description = None;
    for line in header.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().trim_matches(['"', '\'']).to_string());
        } else if let Some(v) = line.strip_prefix("description:") {
            description = Some(v.trim().trim_matches(['"', '\'']).to_string());
        }
    }
    (name, description, body)
}

/// 掃描工作區的 `.claude/skills/*/SKILL.md`，載入技能清單。
pub fn load_skills(workspace: &Path) -> Vec<SkillInfo> {
    let skills_root = workspace.join(".claude").join("skills");
    let Ok(entries) = std::fs::read_dir(&skills_root) else {
        return Vec::new();
    };
    let mut skills = Vec::new();
    for entry in entries.flatten() {
        if skills.len() >= SKILLS_LIST_MAX {
            break;
        }
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let skill_md = dir.join("SKILL.md");
        let Ok(content) = std::fs::read_to_string(&skill_md) else {
            continue;
        };
        let (name, description, body) = parse_frontmatter(&content);
        let dir_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        skills.push(SkillInfo {
            name: name.filter(|n| !n.is_empty()).unwrap_or(dir_name),
            description: description.unwrap_or_default(),
            dir,
            body,
        });
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// 載入專案規則：`CLAUDE.md` 優先，其次 `.claude/CLAUDE.md`，再次 `AGENTS.md`。
pub fn load_project_rules(workspace: &Path) -> Option<String> {
    for candidate in ["CLAUDE.md", ".claude/CLAUDE.md", "AGENTS.md"] {
        let path = workspace.join(candidate);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !content.trim().is_empty() {
                return Some(cap_chars(content.trim(), CLAUDE_MD_MAX_CHARS));
            }
        }
    }
    None
}

/// 使用者輸入以 `/skill-name` 起頭時，回傳被點名的技能。
fn invoked_skill<'a>(skills: &'a [SkillInfo], user_prompt: &str) -> Option<&'a SkillInfo> {
    let first_word = user_prompt.trim().strip_prefix('/')?.split_whitespace().next()?;
    skills.iter().find(|s| s.name == first_word)
}

/// 組合技能/規則系統提示。無任何規則與技能時回傳 None（不佔 token）。
pub fn build_skills_system_prompt(workspace: &Path, user_prompt: &str) -> Option<String> {
    let rules = load_project_rules(workspace);
    let skills = load_skills(workspace);
    if rules.is_none() && skills.is_empty() {
        return None;
    }

    let mut prompt = String::new();
    if let Some(rules) = rules {
        prompt.push_str("## Project rules (CLAUDE.md)\nFollow these project-specific instructions:\n\n");
        prompt.push_str(&rules);
        prompt.push_str("\n\n");
    }
    if !skills.is_empty() {
        prompt.push_str("## Available skills (.claude/skills/)\n");
        prompt.push_str("Users invoke a skill by starting their message with /<skill-name>. ");
        prompt.push_str("When a task matches a skill's description, follow that skill's instructions.\n\n");
        for s in &skills {
            prompt.push_str(&format!("- /{} — {}\n", s.name, s.description));
        }
        if let Some(skill) = invoked_skill(&skills, user_prompt) {
            prompt.push_str(&format!(
                "\n## Invoked skill: /{}\nThe user invoked this skill. Follow its instructions exactly:\n\n{}\n",
                skill.name,
                cap_chars(&skill.body, SKILL_BODY_MAX_CHARS),
            ));
        }
    }
    Some(prompt)
}

/// 把已連線 MCP 伺服器的工具清單組成系統提示——模型必須知道 run_mcp 能呼叫什麼。
pub fn build_mcp_tools_prompt(
    all: &HashMap<String, Vec<crate::mcp::McpTool>>,
) -> Option<String> {
    if all.is_empty() {
        return None;
    }
    let mut s = String::from(
        "## Connected MCP servers and tools\nCall them with <run_mcp server=\"NAME\" tool=\"TOOL\">{\"arg\": \"value\"}</run_mcp>.\n",
    );
    let mut names: Vec<&String> = all.keys().collect();
    names.sort();
    for server in names {
        for t in &all[server] {
            s.push_str(&format!("- {} / {} — {}\n", server, t.name, t.description));
        }
    }
    Some(cap_chars(&s, crate::config::MCP_TOOLS_PROMPT_MAX_CHARS))
}

// ─── .mcp.json（Claude 標準 MCP 設定）────────────────────────────────────────

/// 解析工作區根目錄的 `.mcp.json`。格式：
/// `{"mcpServers": {"server-name": {"command": "npx", "args": ["..."], "env": {"K": "V"}}}}`
pub fn load_mcp_json(workspace: &Path) -> Vec<McpServerConfig> {
    let path = workspace.join(".mcp.json");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    parse_mcp_json(&content)
}

fn parse_mcp_json(content: &str) -> Vec<McpServerConfig> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    let Some(servers) = root.get("mcpServers").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut configs = Vec::new();
    for (name, def) in servers {
        let Some(command) = def.get("command").and_then(|v| v.as_str()) else {
            continue; // SSE/HTTP 型（url 欄位）暫不支援，僅 stdio
        };
        let args = def
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let env: HashMap<String, String> = def
            .get("env")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        configs.push(McpServerConfig {
            name: name.clone(),
            command: command.to_string(),
            args,
            env,
            enabled: true,
        });
    }
    configs.sort_by(|a, b| a.name.cmp(&b.name));
    configs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_parses_name_and_description() {
        let md = "---\nname: my-skill\ndescription: Does a thing.\n---\n\n# Body\ncontent";
        let (name, desc, body) = parse_frontmatter(md);
        assert_eq!(name.as_deref(), Some("my-skill"));
        assert_eq!(desc.as_deref(), Some("Does a thing."));
        assert!(body.starts_with("# Body"));
    }

    #[test]
    fn frontmatter_strips_utf8_bom() {
        let md = "\u{feff}---\nname: bom-skill\ndescription: BOM survives.\n---\nbody";
        let (name, desc, _) = parse_frontmatter(md);
        assert_eq!(name.as_deref(), Some("bom-skill"));
        assert_eq!(desc.as_deref(), Some("BOM survives."));
    }

    #[test]
    fn frontmatter_missing_returns_full_body() {
        let md = "# Just markdown\nno frontmatter";
        let (name, desc, body) = parse_frontmatter(md);
        assert!(name.is_none());
        assert!(desc.is_none());
        assert_eq!(body, md);
    }

    #[test]
    fn mcp_json_parses_claude_format() {
        let json = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"],
                    "env": {"LOG_LEVEL": "info"}
                },
                "remote-only": {"url": "https://example.com/sse"}
            }
        }"#;
        let configs = parse_mcp_json(json);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "filesystem");
        assert_eq!(configs[0].command, "npx");
        assert_eq!(configs[0].args.len(), 3);
        assert_eq!(configs[0].env.get("LOG_LEVEL").map(String::as_str), Some("info"));
        assert!(configs[0].enabled);
    }

    #[test]
    fn mcp_json_invalid_returns_empty() {
        assert!(parse_mcp_json("not json").is_empty());
        assert!(parse_mcp_json("{}").is_empty());
    }

    #[test]
    fn invoked_skill_matches_slash_prefix() {
        let skills = vec![SkillInfo {
            name: "deploy".into(),
            description: "Deploys".into(),
            dir: PathBuf::new(),
            body: "steps".into(),
        }];
        assert!(invoked_skill(&skills, "/deploy to prod").is_some());
        assert!(invoked_skill(&skills, "deploy without slash").is_none());
        assert!(invoked_skill(&skills, "/unknown").is_none());
    }

    #[test]
    fn cap_chars_safe_on_cjk() {
        let s = "中文字串測試".repeat(10);
        let capped = cap_chars(&s, 5);
        assert!(capped.starts_with("中文字串測"));
        assert!(capped.ends_with("…(truncated)"));
    }
}
