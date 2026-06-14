use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::memory::MemoryConfig;

// ─── Default constants ───────────────────────────────────────────────────────
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
pub const DEFAULT_MAX_RETRIES: u32 = 3;
#[allow(dead_code)]
pub const DEFAULT_API_TIMEOUT_SECS: u64 = 30;
#[allow(dead_code)]
pub const AGENTS_DIR: &str = ".agent/rules";
#[allow(dead_code)]
pub const DEFAULT_LOCALE_CHARSET: &str = "en-US";
pub const GITIGNORE_LINE: &str = "# Agnes-AI generated\nconfig.local.toml\n*.db\n.agnes/\n";

// ─── Claude 互通層上限（skills.rs）────────────────────────────────────────────
/// 注入系統提示的單一 SKILL.md 內文字元上限
pub const SKILL_BODY_MAX_CHARS: usize = 8000;
/// CLAUDE.md 專案規則注入字元上限
pub const CLAUDE_MD_MAX_CHARS: usize = 8000;
/// 技能清單最多載入數
pub const SKILLS_LIST_MAX: usize = 50;
/// MCP 工具清單注入系統提示的字元上限
pub const MCP_TOOLS_PROMPT_MAX_CHARS: usize = 4000;

// ─── 金鑰本機持久化 ──────────────────────────────────────────────────────────

/// 金鑰安全工具：讀取 / 寫入 config.local.toml，並確保 .gitignore 已屏蔽。
pub mod key_persistence {
    use super::*;
    use sha2::{Digest, Sha256};

    /// 從 config.local.toml 讀取 API key。如果不存在則回傳空字串。
    #[allow(dead_code)]
    pub fn read_api_key(base_dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let config_path = base_dir.join("config.local.toml");
        if !config_path.exists() {
            return Ok(String::new());
        }
        let content = fs::read_to_string(&config_path)?;
        // 解析 toml 尋找 api.key
        if let Ok(config) = content.parse::<toml::Value>() {
            if let Some(api) = config.get("api") {
                if let Some(key) = api.get("key").and_then(|v| v.as_str()) {
                    return Ok(key.to_string());
                }
            }
        }
        Ok(String::new())
    }

    /// 將 API key 寫入 config.local.toml。
    #[allow(dead_code)]
    pub fn write_api_key(
        base_dir: &Path,
        key: &str,
        extra_sections: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = base_dir.join("config.local.toml");

        // 確保 .gitignore 包含 config.local.toml
        Config::ensure_gitignore(base_dir)?;

        let new_entry = format!(
            "[api]\nkey = \"{}\"\n\n{}\n",
            key, extra_sections
        );
        fs::write(&config_path, new_entry)?;
        Ok(())
    }

    /// 計算字串的 SHA-256 hash（用於金鑰模糊比對，不存原始 key）。
    #[allow(dead_code)]
    pub fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub api: ApiConfig,
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub file_changes: FileChangesConfig,
    #[serde(default)]
    pub model_routing: ModelRoutingConfig,
}

// ─── ModelRoutingConfig ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelRoutingConfig {
    #[serde(default = "default_routing_low")]
    pub low: String,
    #[serde(default = "default_routing_main")]
    pub main: String,
    #[serde(default = "default_routing_high")]
    pub high: String,
}

fn default_routing_low() -> String { "agnes-2.0-flash".to_string() }
fn default_routing_main() -> String { "agnes-2.0-flash".to_string() }
fn default_routing_high() -> String { "claude-3-5-sonnet".to_string() }

impl Default for ModelRoutingConfig {
    fn default() -> Self {
        Self {
            low: default_routing_low(),
            main: default_routing_main(),
            high: default_routing_high(),
        }
    }
}

// ─── ApiConfig ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub key: String,
    #[serde(default = "default_session_budget")]
    pub session_budget: u64,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_api_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_repairs")]
    pub max_repairs: u32,
    #[serde(default = "default_cost_per_token")]
    pub cost_per_token: f64,
    /// 每分鐘最大 API 請求數（Agnes-2.0-Flash 免費方案：20 RPM）。0 = 不限速。
    #[serde(default = "default_max_rpm")]
    pub max_rpm: u32,
    /// 429 指數退避初始等待秒數
    #[serde(default = "default_retry_initial_backoff")]
    pub retry_initial_backoff_secs: u64,
    /// 429 指數退避最大等待秒數
    #[serde(default = "default_retry_max_backoff")]
    pub retry_max_backoff_secs: u64,
    /// 429 最大重試次數（用盡後回傳錯誤）
    #[serde(default = "default_retry_max_attempts")]
    pub retry_max_attempts: u32,
    /// 每次退避的倍增因子
    #[serde(default = "default_retry_backoff_multiplier")]
    pub retry_backoff_multiplier: f64,
}

fn default_session_budget() -> u64 { 500000 }
fn default_base_url() -> String { "https://apihub.agnes-ai.com/v1/chat/completions".to_string() }
fn default_model() -> String { "agnes-2.0-flash".to_string() }
fn default_api_timeout() -> u64 { DEFAULT_API_TIMEOUT_SECS }
fn default_max_repairs() -> u32 { 3 }
fn default_cost_per_token() -> f64 { 0.000001 }
fn default_max_rpm() -> u32 { 20 }
fn default_retry_initial_backoff() -> u64 { 3 }
fn default_retry_max_backoff() -> u64 { 60 }
fn default_retry_max_attempts() -> u32 { 5 }
fn default_retry_backoff_multiplier() -> f64 { 2.0 }

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
            session_budget: default_session_budget(),
            base_url: default_base_url(),
            model: default_model(),
            timeout_seconds: default_api_timeout(),
            max_repairs: default_max_repairs(),
            cost_per_token: default_cost_per_token(),
            max_rpm: default_max_rpm(),
            retry_initial_backoff_secs: default_retry_initial_backoff(),
            retry_max_backoff_secs: default_retry_max_backoff(),
            retry_max_attempts: default_retry_max_attempts(),
            retry_backoff_multiplier: default_retry_backoff_multiplier(),
        }
    }
}

// ─── SandboxConfig ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SandboxConfig {
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 沙盒對齊失敗時回饋給模型的 stderr 行數上限（Delta-only 回饋）
    #[serde(default = "default_stderr_feedback_lines")]
    pub stderr_feedback_lines: usize,
}

fn default_timeout() -> u64 { DEFAULT_TIMEOUT_SECONDS }
fn default_max_retries() -> u32 { DEFAULT_MAX_RETRIES }
fn default_stderr_feedback_lines() -> usize { 20 }

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
            max_retries: DEFAULT_MAX_RETRIES,
            stderr_feedback_lines: default_stderr_feedback_lines(),
        }
    }
}

// ─── SecurityConfig ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SecurityConfig {
    #[serde(default = "default_require_approval")]
    pub require_approval: bool,
    #[serde(default = "default_auto_review")]
    pub auto_review: bool,
    #[serde(default)]
    pub full_access: bool,
}

fn default_require_approval() -> bool { true }
fn default_auto_review() -> bool { false }

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_approval: true,
            auto_review: false,
            full_access: false,
        }
    }
}

// ─── GeneralConfig ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneralConfig {
    #[serde(default = "default_work_mode")]
    pub project_mode: String,       // "project" or "global"
    #[serde(default = "default_shell")]
    pub shell: String,              // "PowerShell", "cmd", "sh"
    #[serde(default = "default_language")]
    pub language: String,           // "zh-TW", "en-US", "auto"
    #[serde(default = "default_locale_charset")]
    pub locale_charset: String,
    /// 介面縮放倍率（egui pixels_per_point；高解析螢幕預設放大）
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    #[serde(default)]
    pub active_project_id: Option<String>,
    /// 右側面板（代理人/變更/檔案）啟動時是否展開
    #[serde(default = "default_right_panel_open")]
    pub right_panel_open_default: bool,
    /// 變更 Tab diff 視圖的輸出行數上限（stats 仍全量計算）
    #[serde(default = "default_diff_view_max_lines")]
    pub diff_view_max_lines: usize,
    /// 檔案 Tab 唯讀檢視器可載入的檔案大小上限（bytes）
    #[serde(default = "default_file_viewer_max_bytes")]
    pub file_viewer_max_bytes: usize,
}

fn default_work_mode() -> String { "project".to_string() }
fn default_shell() -> String { "PowerShell".to_string() }
fn default_language() -> String { "auto".to_string() }
fn default_locale_charset() -> String { "en-US".to_string() }
fn default_ui_scale() -> f32 { 1.25 }
fn default_right_panel_open() -> bool { true }
fn default_diff_view_max_lines() -> usize { 800 }
fn default_file_viewer_max_bytes() -> usize { 512_000 }
/// 介面縮放允許範圍（防止誤設成 0 讓視窗不可用）
pub const UI_SCALE_MIN: f32 = 1.0;
pub const UI_SCALE_MAX: f32 = 1.75;

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            project_mode: "project".to_string(),
            shell: "PowerShell".to_string(),
            language: "auto".to_string(),
            locale_charset: "en-US".to_string(),
            ui_scale: default_ui_scale(),
            active_project_id: None,
            right_panel_open_default: default_right_panel_open(),
            diff_view_max_lines: default_diff_view_max_lines(),
            file_viewer_max_bytes: default_file_viewer_max_bytes(),
        }
    }
}

// ─── FileChangesConfig ───────────────────────────────────────────────────────

/// 截斷標記：file_changes 單筆內容超過 content_max_bytes 時附加於截斷處。
pub const FILE_CHANGE_TRUNCATION_MARKER: &str = "\n…[內容已截斷：超過單筆保存上限]";

/// file_changes 表（write_file before/after 全文快照）保留策略。
/// 無上限時 DB 會隨寫檔次數無界成長，故三道閘：單筆截斷、每對話筆數、刪對話級聯。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FileChangesConfig {
    /// 單筆 before/after 內容上限（bytes）。超過則保留前 N bytes（UTF-8 邊界
    /// 向下對齊）並附 FILE_CHANGE_TRUNCATION_MARKER；存檔長度可能略超出
    /// 上限（標記本身的長度），上限只約束原文部分。
    #[serde(default = "default_file_change_content_max_bytes")]
    pub content_max_bytes: usize,
    /// 每對話保留的變更筆數上限，超過時刪除最舊（id 最小）。0 = 不保留任何紀錄。
    #[serde(default = "default_file_change_keep_per_conversation")]
    pub keep_per_conversation: usize,
}

/// 與 file_viewer_max_bytes 同量級：500 KB 足以涵蓋正常源碼檔，擋住二進位級大檔。
fn default_file_change_content_max_bytes() -> usize { 512_000 }
fn default_file_change_keep_per_conversation() -> usize { 200 }

impl Default for FileChangesConfig {
    fn default() -> Self {
        Self {
            content_max_bytes: default_file_change_content_max_bytes(),
            keep_per_conversation: default_file_change_keep_per_conversation(),
        }
    }
}

// ─── McpServerConfig ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// 傳給伺服器行程的環境變數（Claude .mcp.json 的 env 欄位）
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default = "default_mcp_enabled")]
    pub enabled: bool,
}

fn default_mcp_enabled() -> bool { true }

// ─── Config helpers ──────────────────────────────────────────────────────────

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if let Ok(mut dir) = std::env::current_dir() {
            loop {
                let path = dir.join("config.local.toml");
                if path.exists() {
                    let content = fs::read_to_string(&path)?;
                    let config: Config = toml::from_str(&content)?;
                    return Ok(config);
                }
                if let Some(parent) = dir.parent() {
                    dir = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }
        let content = fs::read_to_string("config.local.toml")
            .or_else(|_| fs::read_to_string("../config.local.toml"))?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        let mut target_path = PathBuf::from("config.local.toml");
        if let Ok(mut dir) = std::env::current_dir() {
            loop {
                let path = dir.join("config.local.toml");
                if path.exists() {
                    target_path = path;
                    break;
                }
                if let Some(parent) = dir.parent() {
                    dir = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }
        fs::write(target_path, content)?;
        Ok(())
    }

    /// Ensure .gitignore contains our entries. Called once on first run.
    pub fn ensure_gitignore(base_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let gitignore_path = base_dir.join(".gitignore");
        if gitignore_path.exists() {
            let existing = fs::read_to_string(&gitignore_path)?;
            for line in GITIGNORE_LINE.lines() {
                if !line.starts_with('#') && !line.trim().is_empty() && !existing.contains(line) {
                    fs::write(&gitignore_path, format!("{}\n{}", existing, line))?;
                }
            }
        }
        Ok(())
    }
}


// ─── Language translation system ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Language {
    ZhTW,
    EnUS,
    Auto,
}

impl Language {
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "zh-TW" | "zh_TW" => Language::ZhTW,
            "en-US" | "en_US" | "en" => Language::EnUS,
            _ => Language::Auto,
        }
    }

    #[allow(dead_code)]
    pub fn resolve(&self, system_lang: &str) -> Language {
        match self {
            Language::Auto => {
                if system_lang.starts_with("zh") || system_lang.starts_with("zh-TW") {
                    Language::ZhTW
                } else {
                    Language::EnUS
                }
            }
            other => other.clone(),
        }
    }
}

/// Translation lookup for UI strings. Each key maps to zh-TW and en-US variants.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LanguagePack {
    pub translations: std::collections::HashMap<String, (String, String)>,
}

impl Default for LanguagePack {
    fn default() -> Self {
        let mut t = std::collections::HashMap::new();
        t.insert("idle".into(), ("閒置".into(), "Idle".into()));
        t.insert("running".into(), ("執行中".into(), "Running".into()));
        t.insert("complete".into(), ("已完成".into(), "Complete".into()));
        t.insert("error".into(), ("錯誤".into(), "Error".into()));
        t.insert("pass".into(), ("通過".into(), "PASS".into()));
        t.insert("reject".into(), ("拒絕".into(), "REJECT".into()));
        t.insert("pending_approval".into(), ("等待審核".into(), "Pending Approval".into()));
        t.insert("agent_audit".into(), ("AI 代理人審查".into(), "Agent Audit".into()));
        t.insert("project_mode".into(), ("專案模式".into(), "Project Mode".into()));
        t.insert("global_mode".into(), ("全域模式".into(), "Global Mode".into()));
        t.insert("require_approval".into(), ("需要審核".into(), "Require Approval".into()));
        t.insert("full_access".into(), ("完全存取".into(), "Full Access".into()));
        t.insert("auto_review".into(), ("自動審核".into(), "Auto Review".into()));
        t.insert("save".into(), ("儲存".into(), "Save".into()));
        t.insert("load".into(), ("載入".into(), "Load".into()));
        t.insert("workspace".into(), ("工作區".into(), "Workspace".into()));
        t.insert("project".into(), ("專案".into(), "Project".into()));
        t.insert("new_project".into(), ("新建專案".into(), "New Project".into()));
        t.insert("delete".into(), ("刪除".into(), "Delete".into()));
        t.insert("execute".into(), ("執行".into(), "Execute".into()));
        t.insert("abort".into(), ("中止".into(), "Abort".into()));
        t.insert("approve".into(), ("批准".into(), "Approve".into()));
        t.insert("deny".into(), ("拒絕".into(), "Deny".into()));
        t.insert("language".into(), ("語言".into(), "Language".into()));
        t.insert("security".into(), ("安全".into(), "Security".into()));
        t.insert("settings".into(), ("設定".into(), "Settings".into()));
        t.insert("task_list".into(), ("任務列表".into(), "Task List".into()));
        t.insert("audit_log".into(), ("審查日誌".into(), "Audit Log".into()));
        t.insert("execution_log".into(), ("執行日誌".into(), "Execution Log".into()));
        Self { translations: t }
    }
}

impl LanguagePack {
    #[allow(dead_code)]
    pub fn get(&self, key: &str, lang: &Language) -> String {
        match self.translations.get(key) {
            Some((zh, en)) => match lang {
                Language::ZhTW => zh.clone(),
                _ => en.clone(),
            },
            None => key.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn to_toml_table(&self) -> String {
        let mut s = String::new();
        s.push_str("[translations]\n");
        for (key, (zh, en)) in &self.translations {
            s.push_str(&format!("{} = \"{}\"\n", key, zh));
            s.push_str(&format!("{}_en = \"{}\"\n", key, en));
        }
        s
    }
}
