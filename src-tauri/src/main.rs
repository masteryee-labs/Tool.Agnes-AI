//! Agnes AI v0.5.0 — Native Rust GUI (egui/wgpu, zero Chromium)
//! Layout: Left sidebar (nav/projects) | Central (chat/input) | Right (22-agent panel + token budget)

use std::sync::{Arc, Mutex};
use eframe::egui;
use rusqlite::Connection;
use app_lib::{AppState, Config, AgentLoop, AgentExecutionState, AuditResult, PendingState};

// ─── Color Palette (Codex/Antigravity dark theme) ───────────────────────────
const BG_PRIMARY:   egui::Color32 = egui::Color32::from_rgb(18,  18,  18);
const BG_SECONDARY: egui::Color32 = egui::Color32::from_rgb(30,  30,  30);
const BG_TERTIARY:  egui::Color32 = egui::Color32::from_rgb(40,  40,  40);
const BG_HOVER:     egui::Color32 = egui::Color32::from_rgb(50,  50,  50);
const BG_CARD:      egui::Color32 = egui::Color32::from_rgb(35,  35,  35);
const BORDER:       egui::Color32 = egui::Color32::from_rgb(60,  60,  60);
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(230, 230, 230);
const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(160, 160, 160);
const TEXT_MUTED:   egui::Color32 = egui::Color32::from_rgb(100, 100, 100);
const ACCENT_BLUE:  egui::Color32 = egui::Color32::from_rgb(80,  140, 255);
const ACCENT_ORANGE:egui::Color32 = egui::Color32::from_rgb(255, 170, 60);
const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(80,  200, 120);
const ACCENT_RED:   egui::Color32 = egui::Color32::from_rgb(255, 80,  80);
const ACCENT_YELLOW:egui::Color32 = egui::Color32::from_rgb(255, 220, 60);

// ─── 全域字級（使用者回饋：預設字太小）──────────────────────────────────────
const FONT_HEADING: f32 = 24.0;
const FONT_BODY:    f32 = 16.0;
const FONT_BUTTON:  f32 = 15.5;
const FONT_SMALL:   f32 = 13.0;
const FONT_MONO:    f32 = 14.5;

// ─── i18n ────────────────────────────────────────────────────────────────────

// 注意：值內不得內嵌 icon——icon 由呼叫端統一前綴，避免重複疊加。
const TRANSLATIONS: &[(&str, (&str, &str))] = &[
    ("new_conversation",    ("新增對話",               "New Conversation")),
    ("conversation_history",("對話歷史",               "Conversation History")),
    ("projects",            ("專案",                   "Projects")),
    ("settings",            ("設定",                   "Settings")),
    ("ask_placeholder",     ("什麼都能做，@ 提及，/ 指令", "Ask anything, @ to mention, / for actions")),
    ("add_folder",          ("新增資料夾",             "Add Folder")),
    ("agent_status",        ("22 代理人狀態",          "22-Agent Status")),
    ("global_warning",      ("全域模式已啟用 — 所有操作需逐項確認", "Global mode active — all operations require confirmation")),
    ("pending_approval",    ("待確認",                 "Pending Approval")),
    ("work_mode_project",   ("專案模式",               "Project Mode")),
    ("work_mode_global",    ("全域模式",               "Global Mode")),
    ("general",             ("一般",                   "General")),
    ("permissions",         ("權限",                   "Permissions")),
    ("security",            ("安全",                   "Security")),
    ("save",                ("儲存",                   "Save")),
    ("terminal_shell",      ("整合的終端 Shell",       "Terminal Shell")),
    ("auto_review",         ("自動審查",               "Auto Review")),
    ("full_access",         ("完整存取權",             "Full Access")),
    ("token_budget",        ("Token 預算",             "Token Budget")),
    ("welcome_question",    ("我們該在 {} 中建立什麼？", "What should we build in {}?")),
    ("selected_folders",    ("{} 個資料夾已選取",       "{} folder(s) selected")),
    ("language",            ("語言",                   "Language")),
    ("menu_file",           ("檔案",                   "File")),
    ("menu_view",           ("檢視",                   "View")),
    ("menu_window",         ("視窗",                   "Window")),
    ("back_to_app",         ("返回應用程式",           "Back to App")),
    ("exit_app",            ("結束",                   "Exit")),
    ("search_settings",     ("搜尋設定…",              "Search settings…")),
    ("personal",            ("個人",                   "Personal")),
    ("integrations",        ("整合",                   "Integrations")),
    ("api_models",          ("API 與模型",             "API & Models")),
    ("mcp_servers",         ("MCP 伺服器",             "MCP Servers")),
    ("mcp_servers_desc",    ("連接外部工具和資料來源。", "Connect external tools and data sources.")),
    ("add_server",          ("新增伺服器",             "Add Server")),
    ("servers",             ("伺服器",                 "Servers")),
    ("work_mode",           ("工作模式",               "Work Mode")),
    ("work_mode_desc",      ("選擇 Agnes 的執行範圍",   "Choose the execution scope for Agnes")),
    ("mode_project_desc",   ("僅限選定的專案資料夾，路徑圈禁", "Restricted to selected project folders")),
    ("mode_global_desc",    ("全電腦操作，逐項確認後才執行", "Full computer access, per-action confirmation")),
    ("default_perm",        ("預設權限",               "Default Permissions")),
    ("default_perm_desc",   ("預設情況下，Agnes 可讀取及編輯其工作區中的檔案。需要時可要求額外存取權限",
                             "By default Agnes can read and edit files in its workspace. It can request extra access when needed")),
    ("auto_review_desc",    ("Agnes 會自動審查工具呼叫（22 道交叉驗證），通過才執行",
                             "Agnes auto-reviews tool calls (22-gate validation) before executing")),
    ("full_access_desc",    ("以完整存取權執行時可編輯任何檔案並執行指令。這會大幅增加風險",
                             "Full access can edit any file and run commands. This greatly increases risk")),
    ("shell_desc",          ("選擇在整合終端中開啟的 Shell。", "Choose the shell used by the integrated terminal.")),
    ("language_desc",       ("應用程式 UI 的語言",      "Language of the application UI")),
    ("api_key",             ("API 金鑰",               "API Key")),
    ("api_key_desc",        ("僅儲存於本機 config.local.toml，永不進入版本控制",
                             "Stored locally in config.local.toml only, never committed")),
    ("api_key_saved",       ("已儲存（指紋 {}）",       "Saved (fingerprint {})")),
    ("model",               ("模型",                   "Model")),
    ("model_desc",          ("任務主模型名稱",          "Primary model for tasks")),
    ("session_budget",      ("Session Token 預算",     "Session Token Budget")),
    ("session_budget_desc", ("達到預算後鎖定 API 呼叫，僅允許確定性操作",
                             "API calls lock at budget; only deterministic ops continue")),
    ("sandbox_timeout",     ("沙盒逾時（秒）",          "Sandbox Timeout (s)")),
    ("sandbox_timeout_desc",("單一指令的最長執行時間",   "Maximum runtime for a single command")),
    ("max_retries",         ("最大重試次數",            "Max Retries")),
    ("max_retries_desc",    ("沙盒執行失敗的自愈重試上限", "Self-healing retry cap on sandbox failure")),
    ("no_results",          ("沒有符合的設定",          "No matching settings")),
    ("clear_chat",          ("清除當前對話",           "Clear Chat")),
    ("add_project_folder",  ("新增專案資料夾…",        "Add Project Folder…")),
    ("untitled",            ("(無標題)",               "(Untitled)")),
    ("abort",               ("中止",                   "Abort")),
    ("running_hint",        ("正在執行任務中，請稍候…", "Executing task, please wait…")),
    ("local_work",          ("本機作業",               "Local")),
    ("tab_projects",        ("專案",                   "Projects")),
    ("tab_global",          ("全域",                   "Global")),
    ("new_project",         ("新增專案",               "New Project")),
    ("global_tab_desc",     ("全域模式：直接操控整台電腦，所有操作逐項確認後執行",
                             "Global mode: operate the whole computer; every action is confirmed first")),
    ("no_sessions",         ("尚無對話，按上方「＋ 新增對話」開始",
                             "No sessions yet — press \"+ New Conversation\" above")),
    ("api_key_saved_ok",    ("已儲存 ✓",               "Saved ✓")),
    ("api_key_current",     ("目前金鑰",               "Current key")),
    ("api_key_not_set",     ("尚未設定",               "Not set")),
    ("server_name",         ("名稱",                   "Name")),
    ("command",             ("指令",                   "Command")),
    ("args_hint",           ("引數（空白分隔）",        "Args (space-separated)")),
    ("add",                 ("新增",                   "Add")),
    ("cancel",              ("取消",                   "Cancel")),
    ("from_mcp_json",       ("來自工作區 .mcp.json（Claude 格式，唯讀）",
                             "From workspace .mcp.json (Claude format, read-only)")),
    ("skills",              ("技能 Skills",            "Skills")),
    ("skills_desc",         ("從工作區 .claude/skills/<名稱>/SKILL.md 載入（Claude 格式）。對話輸入 /名稱 即可呼叫",
                             "Loaded from .claude/skills/<name>/SKILL.md in the workspace (Claude format). Type /name in chat to invoke")),
    ("no_skills",           ("此工作區沒有技能。建立 .claude/skills/<名稱>/SKILL.md 即可新增",
                             "No skills in this workspace. Create .claude/skills/<name>/SKILL.md to add one")),
    ("mcp_started_hint",    ("已加入並嘗試啟動伺服器：{}", "Added and starting server: {}")),
    ("folders",             ("資料夾",                 "Folders")),
    ("welcome_global",      ("全域模式：需要我操作電腦做什麼？",
                             "Global mode — what should I do on this computer?")),
    ("ui_scale",            ("介面縮放",               "UI Scale")),
    ("ui_scale_desc",       ("整體介面與文字的放大倍率", "Magnification for the whole interface and text")),
    ("panel_scope_global",  ("範疇：全域",             "Scope: Global")),
    ("panel_scope_idle",    ("尚未執行審查",           "No audits yet")),
    ("legend_agents",       ("✓ 通過　✗ 否決　~ 跳過　· 休眠",
                             "✓ pass   ✗ reject   ~ skip   · dormant")),
];

fn t_with(key: &str, lang: &str, arg: &str) -> String {
    t(key, lang).replace("{}", arg)
}

/// UTF-8 安全截斷：以字元為單位（位元組切片 &s[..n] 落在多位元組字元中間會 panic）。
fn truncate_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

fn t(key: &str, lang: &str) -> String {
    for &(k, (zh, en)) in TRANSLATIONS {
        if k == key {
            return if lang == "zh" { zh.to_string() } else { en.to_string() };
        }
    }
    key.to_string()
}

fn t_fmt(key: &str, lang: &str, n: usize) -> String {
    let tmpl = t(key, lang);
    tmpl.replace("{}", &n.to_string())
}

// ─── Data Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProjectFolder {
    pub id:    String,
    pub name:  String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    role:    String,
    content: String,
}

struct UiState {
    // Projects
    projects:             Vec<ProjectFolder>,
    selected_project_idx: Option<usize>,
    selected_paths:       std::collections::HashSet<String>,
    // Chat
    chat_input:            String,
    active_messages:       Vec<ChatMessage>,
    conversations:         Vec<app_lib::ConversationSummary>,
    active_conversation_id: Option<String>,
    // Settings / i18n / mode
    language:     String,   // "zh" | "en"
    settings_open: bool,
    settings_section: usize, // 0 一般 | 1 權限 | 2 API 與模型 | 3 安全 | 4 MCP 伺服器 | 5 技能
    settings_search: String,
    api_key_input: String,
    /// 金鑰儲存成功的常駐回饋（顯示「已儲存 ✓」直到離開設定頁）
    api_key_saved_feedback: bool,
    work_mode: String, // "project" | "global"
    // MCP 新增伺服器表單
    mcp_form_open: bool,
    mcp_form_name: String,
    mcp_form_command: String,
    mcp_form_args: String,
    // Agent panel
    audit_results: Vec<AuditResult>,
    status_message: String,
    // Sidebar tab: 0 = 專案, 1 = 全域
    sidebar_tab: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            projects:              Vec::new(),
            selected_project_idx:  None,
            selected_paths:        std::collections::HashSet::new(),
            chat_input:            String::new(),
            active_messages:       Vec::new(),
            conversations:         Vec::new(),
            active_conversation_id: None,
            language:     "zh".into(),
            settings_open: false,
            settings_section: 0,
            settings_search: String::new(),
            api_key_input: String::new(),
            api_key_saved_feedback: false,
            work_mode:     "project".into(),
            mcp_form_open: false,
            mcp_form_name: String::new(),
            mcp_form_command: String::new(),
            mcp_form_args: String::new(),
            audit_results: Vec::new(),
            status_message: String::new(),
            sidebar_tab:   0,
        }
    }
}

// ─── QA 自我截圖模式 ─────────────────────────────────────────────────────────
// AGNES_QA_SHOT=<png路徑> 啟動時：暖機數幀後對「本應用程式視窗自身」截圖存檔並退出。
// 不讀取螢幕、不控制滑鼠鍵盤——影像來自 egui 自己的渲染管線。

/// 截圖前的暖機幀數（等字型載入與版面穩定）
const QA_WARMUP_FRAMES: u32 = 12;
/// 互動 QA：任務完成後再等待的安定幀數（讓訊息流完成渲染）
const QA_SETTLE_FRAMES: u32 = 8;
/// 互動 QA：等待 agent 完成的逾時秒數
const QA_SEND_TIMEOUT_SECS: u64 = 180;

fn save_color_image_png(img: &egui::ColorImage, path: &std::path::Path) -> Result<(), String> {
    let [w, h] = img.size;
    let mut rgba = Vec::with_capacity(w * h * 4);
    for px in &img.pixels {
        rgba.extend_from_slice(&px.to_array());
    }
    let buf = image::RgbaImage::from_raw(w as u32, h as u32, rgba)
        .ok_or_else(|| "RGBA buffer size mismatch".to_string())?;
    buf.save(path).map_err(|e| e.to_string())
}

/// Codex 風格設定列：左側標題+描述，右側控制項，卡片背景。
fn settings_row(
    ui: &mut egui::Ui,
    search: &str,
    title: &str,
    desc: &str,
    control: impl FnOnce(&mut egui::Ui),
) -> bool {
    if !search.trim().is_empty()
        && !title.to_lowercase().contains(&search.trim().to_lowercase())
        && !desc.to_lowercase().contains(&search.trim().to_lowercase())
    {
        return false;
    }
    egui::Frame::default()
        .fill(BG_CARD)
        .corner_radius(8)
        .inner_margin(14.0)
        .show(ui, |ui| {
            // egui 正規左右佈局：左標題描述、右控制項
            egui::Sides::new().show(
                ui,
                |ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width((ui.available_width() - 240.0).max(200.0));
                        ui.label(egui::RichText::new(title).size(16.0).color(TEXT_PRIMARY).strong());
                        if !desc.is_empty() {
                            ui.label(egui::RichText::new(desc).size(13.5).color(TEXT_SECONDARY));
                        }
                    });
                },
                control,
            );
        });
    ui.add_space(8.0);
    true
}

/// Codex 風格 toggle 開關。
/// 以 Button 為互動基底（allocate_exact_size 的手動命中區在部分巢狀版面收不到點擊），
/// 外觀由自訂繪製覆蓋。
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = egui::vec2(44.0, 24.0);
    let mut response = ui.add_sized(
        desired_size,
        egui::Button::new("").frame(false),
    );
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let rect = response.rect;
    let how_on = ui.ctx().animate_bool(response.id, *on);
    let bg = egui::Color32::from_rgb(
        (60.0 + how_on * (43.0 - 60.0)) as u8,
        (60.0 + how_on * (134.0 - 60.0)) as u8,
        (60.0 + how_on * (255.0 - 60.0)) as u8,
    );
    let radius = rect.height() / 2.0;
    ui.painter().rect_filled(rect, radius, bg);
    let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        radius - 3.0,
        egui::Color32::WHITE,
    );
    response
}

/// 設定導航項：以 Button 為基底的可選擇列（selectable_label 在此面板收不到點擊）。
fn nav_item(ui: &mut egui::Ui, selected: bool, label: String) -> egui::Response {
    let fill = if selected { egui::Color32::from_rgb(40, 60, 100) } else { egui::Color32::TRANSPARENT };
    ui.add_sized(
        egui::vec2(ui.available_width(), 26.0),
        egui::Button::new(egui::RichText::new(label).size(14.5)).fill(fill).corner_radius(6),
    )
}

// ─── Main App ────────────────────────────────────────────────────────────────

struct AgnesApp {
    app_state: Arc<AppState>,
    ui_state:  Arc<Mutex<UiState>>,
    qa_shot:   Option<std::path::PathBuf>,
    qa_frames: u32,
    /// 互動 QA：啟動後自動經 handle_send 送出的 prompt（與使用者操作完全相同的代碼路徑）
    qa_send:   Option<String>,
    qa_sent:   bool,
    qa_done_frames: u32,
    qa_deadline: Option<std::time::Instant>,
}

/// 載入作業系統 CJK 字型（egui default_fonts 不含中文字形，缺此步全部渲染為方框亂碼）。
fn load_cjk_fonts(ctx: &egui::Context) {
    let candidates: &[&str] = &[
        // Windows：微軟正黑體（zh-TW 優先）→ 微軟雅黑 → 細明體
        "C:\\Windows\\Fonts\\msjh.ttc",
        "C:\\Windows\\Fonts\\msjhl.ttc",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\mingliu.ttc",
        // macOS
        "/System/Library/Fonts/PingFang.ttc",
        // Linux
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];

    let mut fonts = egui::FontDefinitions::default();
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            // CJK 作為 fallback 接在內建拉丁字型之後，emoji 字型保持原序
            fonts.families.entry(egui::FontFamily::Proportional).or_default().push("cjk".to_owned());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().push("cjk".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

impl AgnesApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        load_cjk_fonts(&cc.egui_ctx);

        let db_path = app_lib::resolve_db_path();

        let config = match Config::load() {
            Ok(cfg)  => Arc::new(std::sync::Mutex::new(cfg)),
            Err(_)   => {
                let d = Config::default();
                let _ = d.save();
                Arc::new(std::sync::Mutex::new(d))
            }
        };

        let app_state = Arc::new(AppState::new(db_path, config).unwrap());
        let ui_state  = Arc::new(Mutex::new(UiState::default()));

        // Load initial data
        let conn = Connection::open(&app_state.db_path).unwrap();
        let _ = app_lib::init_db(&conn);

        let mut loaded_projects = Vec::new();
        if let Ok(projects) = app_lib::get_all_projects(&conn) {
            for p in projects {
                let folders: Vec<String> = serde_json::from_str(&p.folders).unwrap_or_default();
                loaded_projects.push(ProjectFolder { id: p.id, name: p.name, paths: folders });
            }
        }

        if loaded_projects.is_empty() {
            let default_path = std::env::current_dir()
                .unwrap_or_default().to_string_lossy().to_string();
            let json = serde_json::to_string(&vec![default_path.clone()]).unwrap_or_default();
            if let Ok(id) = app_lib::create_project(&conn, "Default Project", &json) {
                loaded_projects.push(ProjectFolder {
                    id,
                    name:  "Default Project".into(),
                    paths: vec![default_path],
                });
            }
        }

        // 升級既有資料庫：無歸屬的舊對話掛到第一個專案下，側欄才看得到
        if let Some(first) = loaded_projects.first() {
            let _ = app_lib::assign_orphan_conversations(&conn, &first.id);
        }
        let conversations = app_lib::get_conversations(&conn).unwrap_or_default();

        {
            let mut st = ui_state.lock().unwrap();
            st.projects = loaded_projects;
            if !st.projects.is_empty() {
                st.selected_project_idx = Some(0);
                let first_paths = st.projects[0].paths.clone();
                for p in &first_paths {
                    st.selected_paths.insert(p.clone());
                }
            }
            st.conversations = conversations;
            let config_lock = app_state.config.lock().unwrap();
            st.language = if config_lock.general.language.contains("zh") {
                "zh".into()
            } else {
                "en".into()
            };
            st.work_mode = config_lock.general.project_mode.clone();
            st.sidebar_tab = if st.work_mode == "global" { 1 } else { 0 };
        }

        // 啟動 MCP 伺服器：config.local.toml 啟用項 + 各專案資料夾的 .mcp.json（Claude 格式）。
        // 此前 start_servers 無人呼叫——設定了 MCP 也永遠不會啟動。
        {
            let mut to_start = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for c in app_state.config.lock().unwrap().mcp_servers.iter().filter(|c| c.enabled) {
                if seen.insert(c.name.clone()) {
                    to_start.push(c.clone());
                }
            }
            let st = ui_state.lock().unwrap();
            for p in &st.projects {
                for path in &p.paths {
                    for c in app_lib::load_mcp_json(std::path::Path::new(path)) {
                        if seen.insert(c.name.clone()) {
                            to_start.push(c);
                        }
                    }
                }
            }
            drop(st);
            if !to_start.is_empty() {
                let manager = app_state.mcp_manager.clone();
                app_state.engine_runtime.spawn(async move {
                    manager.start_servers(&to_start).await;
                });
            }
        }

        // QA 自我截圖模式：AGNES_QA_SHOT=輸出路徑，AGNES_QA_VIEW=settings|history 切換視圖
        let qa_shot = std::env::var("AGNES_QA_SHOT").ok().map(std::path::PathBuf::from);
        if qa_shot.is_some() {
            let mut st = ui_state.lock().unwrap();
            match std::env::var("AGNES_QA_VIEW").as_deref() {
                Ok("settings") => st.settings_open = true,
                // 舊名 history 保留為全域 Tab 的別名（qa_runner 相容）
                Ok("global") | Ok("history") => st.sidebar_tab = 1,
                Ok("chat") => {
                    // 載入最近一筆對話以渲染訊息流（驗證氣泡/碼塊/工具輸出樣式）
                    if let Some(conv) = st.conversations.first().cloned() {
                        if let Ok(conn) = Connection::open(&app_state.db_path) {
                            if let Ok(msgs) = app_lib::get_messages_for_conversation(&conn, &conv.id) {
                                st.active_messages = msgs.into_iter()
                                    .map(|m| ChatMessage { role: m.role, content: m.content })
                                    .collect();
                                st.active_conversation_id = Some(conv.id);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // 互動 QA 模式：AGNES_QA_SEND=<prompt> 啟動後自動經 handle_send 送出，
        // 走與使用者完全相同的代碼路徑（輸入框 → handle_send → API → 工具 → UI 更新）。
        // 此模式強制 auto_review=true（僅記憶體內，不寫回 config.local.toml），
        // 讓工具實際執行以驗證端對端流程。
        let qa_send = std::env::var("AGNES_QA_SEND").ok().filter(|s| !s.trim().is_empty());
        if qa_send.is_some() {
            app_state.config.lock().unwrap().security.auto_review = true;
        }

        Self {
            app_state, ui_state, qa_shot, qa_frames: 0,
            qa_send, qa_sent: false, qa_done_frames: 0, qa_deadline: None,
        }
    }

    /// QA 截圖鉤子：每幀呼叫。
    /// 純截圖模式：暖機後立即截圖。
    /// 互動模式（qa_send）：暖機後送出 prompt → 等 agent 完成 → 安定幀 → 截圖。
    fn qa_screenshot_hook(&mut self, ctx: &egui::Context) {
        let Some(path) = self.qa_shot.clone() else { return };
        self.qa_frames += 1;

        if let Some(prompt) = self.qa_send.clone() {
            if !self.qa_sent && self.qa_frames >= QA_WARMUP_FRAMES {
                self.ui_state.lock().unwrap().chat_input = prompt;
                self.handle_send(ctx);
                self.qa_sent = true;
                self.qa_deadline = Some(
                    std::time::Instant::now()
                        + std::time::Duration::from_secs(QA_SEND_TIMEOUT_SECS),
                );
                println!("[QA] prompt sent via handle_send, waiting for agent…");
            }
            if self.qa_sent {
                let complete = self.app_state.agent_state.try_lock()
                    .map(|s| matches!(*s, AgentExecutionState::Complete))
                    .unwrap_or(false);
                let timed_out = self.qa_deadline
                    .map(|d| std::time::Instant::now() > d)
                    .unwrap_or(false);
                if complete || timed_out {
                    self.qa_done_frames += 1;
                    if self.qa_done_frames == QA_SETTLE_FRAMES {
                        if timed_out && !complete {
                            eprintln!("[QA] TIMEOUT waiting for agent — capturing current state");
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                    }
                }
            }
        } else if self.qa_frames == QA_WARMUP_FRAMES {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(img) = shot {
            match save_color_image_png(&img, &path) {
                Ok(()) => println!("[QA] screenshot saved: {}", path.display()),
                Err(e) => eprintln!("[QA] screenshot save failed: {}", e),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }

    /// Codex 風格全頁式設定：左側設定導航 + 右側卡片式設定列。
    fn render_settings_page(&self, ctx: &egui::Context, lang: &str) {
        let mut st = self.ui_state.lock().unwrap();

        // ── 左側：設定導航 ──
        egui::SidePanel::left("settings_nav")
            .default_width(230.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                if ui.add(
                    egui::Button::new(
                        egui::RichText::new(format!("←  {}", t("back_to_app", lang))).size(14.0),
                    ).frame(false),
                ).clicked() {
                    st.settings_open = false;
                    st.settings_search.clear();
                    st.api_key_saved_feedback = false;
                    // 設定頁的提示訊息不帶回主畫面（主畫面以紅色錯誤樣式顯示會誤導）
                    st.status_message.clear();
                }
                ui.add_space(10.0);

                // 搜尋框
                egui::Frame::default()
                    .fill(BG_TERTIARY)
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔍").size(13.0).color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut st.settings_search)
                                    .hint_text(t("search_settings", lang))
                                    .desired_width(f32::INFINITY)
                                    .frame(false),
                            );
                        });
                    });
                ui.add_space(14.0);

                // 導航分組
                ui.label(egui::RichText::new(t("personal", lang)).size(12.0).color(TEXT_MUTED));
                ui.add_space(4.0);
                let personal: &[(usize, &str, &str)] = &[
                    (0, "⚙", "general"),
                    (1, "🛡", "permissions"),
                    (2, "🔑", "api_models"),
                    (3, "🔒", "security"),
                ];
                for (idx, icon, key) in personal {
                    if nav_item(
                        ui,
                        st.settings_section == *idx,
                        format!("{}  {}", icon, t(key, lang)),
                    ).clicked() {
                        st.settings_section = *idx;
                    }
                }

                ui.add_space(12.0);
                ui.label(egui::RichText::new(t("integrations", lang)).size(12.0).color(TEXT_MUTED));
                ui.add_space(4.0);
                let integrations: &[(usize, &str, &str)] = &[
                    (4, "🔌", "mcp_servers"),
                    (5, "✨", "skills"),
                ];
                for (idx, icon, key) in integrations {
                    if nav_item(
                        ui,
                        st.settings_section == *idx,
                        format!("{}  {}", icon, t(key, lang)),
                    ).clicked() {
                        st.settings_section = *idx;
                    }
                }
            });

        // ── 右側：設定內容 ──
        // 注意：用樸素的 horizontal+vertical 置中。vertical_centered+set_max_width+
        // 內層 with_layout 的巢狀會讓子元件繪製正常但互動矩形被裁掉（點擊全部失效）。
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(24.0);
                let total = ui.available_width();
                let inner = total.min(860.0);
                let margin = ((total - inner) / 2.0).max(24.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.vertical(|ui| {
                        ui.set_width(inner - margin);
                        self.render_settings_section(ui, &mut st, lang);
                    });
                });
                ui.add_space(40.0);
            });
        });
    }

    fn render_settings_section(&self, ui: &mut egui::Ui, st: &mut UiState, lang: &str) {
        let section = st.settings_section;
        let search = st.settings_search.clone();
        let section_title_key = match section {
            0 => "general", 1 => "permissions", 2 => "api_models", 3 => "security",
            4 => "mcp_servers", _ => "skills",
        };
        ui.label(
            egui::RichText::new(t(section_title_key, lang))
                .size(26.0).color(TEXT_PRIMARY).strong(),
        );
        ui.add_space(18.0);

        let mut cfg = self.app_state.config.lock().unwrap().clone();
        let mut cfg_changed = false;
        let mut shown = 0;

        match section {
            0 => {
                // 工作模式：兩張選擇卡片（Codex 風格）
                if search.trim().is_empty() {
                    ui.label(egui::RichText::new(t("work_mode", lang)).size(17.0).color(TEXT_PRIMARY).strong());
                    ui.label(egui::RichText::new(t("work_mode_desc", lang)).size(12.5).color(TEXT_SECONDARY));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let card_w = (ui.available_width() - 12.0) / 2.0;
                        let modes = [
                            ("project", "📁", "work_mode_project", "mode_project_desc"),
                            ("global", "🌍", "work_mode_global", "mode_global_desc"),
                        ];
                        for (mode, icon, title_key, desc_key) in modes {
                            let selected = st.work_mode == mode;
                            let stroke = if selected {
                                egui::Stroke::new(1.5, ACCENT_BLUE)
                            } else {
                                egui::Stroke::new(1.0, BORDER)
                            };
                            let frame_resp = egui::Frame::default()
                                .fill(BG_CARD)
                                .stroke(stroke)
                                .corner_radius(10)
                                .inner_margin(14.0)
                                .show(ui, |ui| {
                                    ui.set_width(card_w - 28.0);
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(icon).size(18.0));
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(t(title_key, lang)).size(14.5).strong());
                                            ui.label(egui::RichText::new(t(desc_key, lang)).size(11.5).color(TEXT_SECONDARY));
                                        });
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let mark = if selected { "🔘" } else { "⚪" };
                                            ui.label(egui::RichText::new(mark).size(13.0));
                                        });
                                    });
                                });
                            // 顯式唯一 Id 的命中區，覆蓋整張卡片
                            let resp = ui.interact(
                                frame_resp.response.rect,
                                ui.id().with(("work_mode_card", mode)),
                                egui::Sense::click(),
                            );
                            if resp.clicked() {
                                st.work_mode = mode.to_string();
                                cfg.general.project_mode = mode.to_string();
                                cfg_changed = true;
                            }
                        }
                    });
                    ui.add_space(16.0);
                    shown += 1;
                }

                shown += settings_row(ui, &search, &t("language", lang), &t("language_desc", lang), |ui| {
                    let current = if st.language == "zh" { "繁體中文" } else { "English" };
                    egui::ComboBox::from_id_salt("lang_combo")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(st.language == "zh", "繁體中文 (zh-TW)").clicked() {
                                st.language = "zh".into();
                            }
                            if ui.selectable_label(st.language == "en", "English (en-US)").clicked() {
                                st.language = "en".into();
                            }
                        });
                }) as usize;
                if st.language == "zh" && cfg.general.language != "zh-TW" {
                    cfg.general.language = "zh-TW".into();
                    cfg_changed = true;
                } else if st.language == "en" && cfg.general.language != "en-US" {
                    cfg.general.language = "en-US".into();
                    cfg_changed = true;
                }

                shown += settings_row(ui, &search, &t("terminal_shell", lang), &t("shell_desc", lang), |ui| {
                    egui::ComboBox::from_id_salt("shell_combo")
                        .selected_text(cfg.general.shell.clone())
                        .show_ui(ui, |ui| {
                            for shell in ["PowerShell", "cmd", "sh"] {
                                if ui.selectable_label(cfg.general.shell == shell, shell).clicked() {
                                    cfg.general.shell = shell.to_string();
                                    cfg_changed = true;
                                }
                            }
                        });
                }) as usize;

                shown += settings_row(ui, &search, &t("ui_scale", lang), &t("ui_scale_desc", lang), |ui| {
                    let current = format!("{:.0}%", cfg.general.ui_scale * 100.0);
                    egui::ComboBox::from_id_salt("ui_scale_combo")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for scale in [1.0_f32, 1.1, 1.25, 1.4, 1.5, 1.75] {
                                let label = format!("{:.0}%", scale * 100.0);
                                let selected = (cfg.general.ui_scale - scale).abs() < f32::EPSILON;
                                if ui.selectable_label(selected, label).clicked() {
                                    cfg.general.ui_scale = scale;
                                    cfg_changed = true;
                                }
                            }
                        });
                }) as usize;
            }
            1 => {
                let mut default_perm = !cfg.security.full_access;
                shown += settings_row(ui, &search, &t("default_perm", lang), &t("default_perm_desc", lang), |ui| {
                    if toggle_switch(ui, &mut default_perm).changed() {
                        cfg.security.full_access = !default_perm;
                        cfg_changed = true;
                    }
                }) as usize;

                shown += settings_row(ui, &search, &t("auto_review", lang), &t("auto_review_desc", lang), |ui| {
                    if toggle_switch(ui, &mut cfg.security.auto_review).changed() {
                        cfg_changed = true;
                    }
                }) as usize;

                shown += settings_row(ui, &search, &t("full_access", lang), &t("full_access_desc", lang), |ui| {
                    if toggle_switch(ui, &mut cfg.security.full_access).changed() {
                        cfg_changed = true;
                    }
                }) as usize;
            }
            2 => {
                // 目前金鑰以遮罩顯示（頭 5 尾 4），使用者才確認得了「存進去的是哪把」
                let key_state_line = if cfg.api.key.is_empty() {
                    t("api_key_not_set", lang)
                } else {
                    let chars: Vec<char> = cfg.api.key.chars().collect();
                    let masked = if chars.len() > 12 {
                        format!(
                            "{}…{}",
                            chars[..5].iter().collect::<String>(),
                            chars[chars.len() - 4..].iter().collect::<String>(),
                        )
                    } else {
                        "•".repeat(chars.len())
                    };
                    let fingerprint = app_lib::key_persistence::hash_key(&cfg.api.key)[..8].to_string();
                    format!(
                        "{}：{}（{}）",
                        t("api_key_current", lang),
                        masked,
                        t_with("api_key_saved", lang, &fingerprint),
                    )
                };
                let saved_feedback = st.api_key_saved_feedback;
                shown += settings_row(
                    ui, &search, &t("api_key", lang),
                    &format!("{}\n{}", t("api_key_desc", lang), key_state_line),
                    |ui| {
                        if ui.button(t("save", lang)).clicked() && !st.api_key_input.trim().is_empty() {
                            cfg.api.key = st.api_key_input.trim().to_string();
                            st.api_key_input.clear();
                            st.api_key_saved_feedback = true;
                            cfg_changed = true;
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut st.api_key_input)
                                .password(true)
                                .hint_text("sk-…")
                                .desired_width(170.0),
                        );
                        if saved_feedback {
                            ui.label(
                                egui::RichText::new(t("api_key_saved_ok", lang))
                                    .color(ACCENT_GREEN).size(14.0).strong(),
                            );
                        }
                    },
                ) as usize;

                shown += settings_row(ui, &search, &t("model", lang), &t("model_desc", lang), |ui| {
                    if ui.add(egui::TextEdit::singleline(&mut cfg.api.model).desired_width(180.0)).changed() {
                        cfg_changed = true;
                    }
                }) as usize;

                shown += settings_row(ui, &search, &t("session_budget", lang), &t("session_budget_desc", lang), |ui| {
                    let mut budget = cfg.api.session_budget;
                    if ui.add(egui::DragValue::new(&mut budget).speed(1000)).changed() {
                        cfg.api.session_budget = budget;
                        cfg_changed = true;
                    }
                }) as usize;
            }
            3 => {
                shown += settings_row(ui, &search, &t("sandbox_timeout", lang), &t("sandbox_timeout_desc", lang), |ui| {
                    if ui.add(egui::DragValue::new(&mut cfg.sandbox.timeout_seconds).range(1..=600)).changed() {
                        cfg_changed = true;
                    }
                }) as usize;

                shown += settings_row(ui, &search, &t("max_retries", lang), &t("max_retries_desc", lang), |ui| {
                    if ui.add(egui::DragValue::new(&mut cfg.sandbox.max_retries).range(0..=10)).changed() {
                        cfg_changed = true;
                    }
                }) as usize;
            }
            4 => {
                ui.label(egui::RichText::new(t("mcp_servers_desc", lang)).size(14.0).color(TEXT_SECONDARY));
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(t("servers", lang)).size(17.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("＋ {}", t("add_server", lang))).clicked() {
                            st.mcp_form_open = !st.mcp_form_open;
                        }
                    });
                });
                ui.add_space(8.0);

                // 新增伺服器表單：寫入 config.local.toml 並立即嘗試啟動
                if st.mcp_form_open {
                    egui::Frame::default()
                        .fill(BG_CARD)
                        .stroke(egui::Stroke::new(1.0, ACCENT_BLUE))
                        .corner_radius(8)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            egui::Grid::new("mcp_add_form").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                                ui.label(t("server_name", lang));
                                ui.add(egui::TextEdit::singleline(&mut st.mcp_form_name)
                                    .hint_text("filesystem").desired_width(280.0));
                                ui.end_row();
                                ui.label(t("command", lang));
                                ui.add(egui::TextEdit::singleline(&mut st.mcp_form_command)
                                    .hint_text("npx").desired_width(280.0));
                                ui.end_row();
                                ui.label(t("args_hint", lang));
                                ui.add(egui::TextEdit::singleline(&mut st.mcp_form_args)
                                    .hint_text("-y @modelcontextprotocol/server-filesystem C:\\data")
                                    .desired_width(280.0));
                                ui.end_row();
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let can_add = !st.mcp_form_name.trim().is_empty()
                                    && !st.mcp_form_command.trim().is_empty();
                                if ui.add_enabled(can_add, egui::Button::new(t("add", lang))).clicked() {
                                    let server = app_lib::McpServerConfig {
                                        name: st.mcp_form_name.trim().to_string(),
                                        command: st.mcp_form_command.trim().to_string(),
                                        args: st.mcp_form_args.split_whitespace()
                                            .map(str::to_string).collect(),
                                        env: Default::default(),
                                        enabled: true,
                                    };
                                    cfg.mcp_servers.push(server.clone());
                                    cfg_changed = true;
                                    st.status_message = t_with("mcp_started_hint", lang, &server.name);
                                    let manager = self.app_state.mcp_manager.clone();
                                    self.app_state.engine_runtime.spawn(async move {
                                        if let Err(e) = manager.start_server(&server).await {
                                            eprintln!("[MCP] {}", e);
                                        }
                                    });
                                    st.mcp_form_name.clear();
                                    st.mcp_form_command.clear();
                                    st.mcp_form_args.clear();
                                    st.mcp_form_open = false;
                                }
                                if ui.button(t("cancel", lang)).clicked() {
                                    st.mcp_form_open = false;
                                }
                            });
                        });
                    ui.add_space(10.0);
                }

                if cfg.mcp_servers.is_empty() {
                    ui.label(egui::RichText::new("—").color(TEXT_MUTED));
                }
                let mut servers = cfg.mcp_servers.clone();
                let mut delete_idx: Option<usize> = None;
                for (i, server) in servers.iter_mut().enumerate() {
                    let name = server.name.clone();
                    let command_line = format!("{} {}", server.command, server.args.join(" "));
                    let was_enabled = server.enabled;
                    shown += settings_row(ui, &search, &name, &command_line, |ui| {
                        if ui.small_button("🗑").clicked() {
                            delete_idx = Some(i);
                        }
                        if toggle_switch(ui, &mut server.enabled).changed() {
                            cfg_changed = true;
                            // 切換即時生效：開→啟動、關→停止
                            let manager = self.app_state.mcp_manager.clone();
                            let cfg_clone = server.clone();
                            let enable = !was_enabled;
                            self.app_state.engine_runtime.spawn(async move {
                                let result = if enable {
                                    manager.start_server(&cfg_clone).await
                                } else {
                                    manager.stop_server(&cfg_clone.name).await
                                };
                                if let Err(e) = result {
                                    eprintln!("[MCP] {}", e);
                                }
                            });
                        }
                    }) as usize;
                }
                if let Some(i) = delete_idx {
                    let removed = servers.remove(i);
                    cfg_changed = true;
                    let manager = self.app_state.mcp_manager.clone();
                    self.app_state.engine_runtime.spawn(async move {
                        let _ = manager.stop_server(&removed.name).await;
                    });
                }
                cfg.mcp_servers = servers;

                // 工作區 .mcp.json（Claude 格式）唯讀清單
                let workspace = st.selected_project_idx
                    .and_then(|i| st.projects.get(i))
                    .and_then(|p| p.paths.first().cloned());
                if let Some(ws) = workspace {
                    let json_servers = app_lib::load_mcp_json(std::path::Path::new(&ws));
                    if !json_servers.is_empty() {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new(t("from_mcp_json", lang))
                            .size(13.0).color(TEXT_SECONDARY));
                        ui.add_space(6.0);
                        for server in &json_servers {
                            let line = format!("{} {}", server.command, server.args.join(" "));
                            shown += settings_row(ui, &search, &server.name, &line, |ui| {
                                ui.label(egui::RichText::new("🔒").size(13.0).color(TEXT_MUTED));
                            }) as usize;
                        }
                    }
                }

                if !st.status_message.is_empty() {
                    ui.label(egui::RichText::new(&st.status_message).size(13.0).color(ACCENT_YELLOW));
                }
            }
            _ => {
                ui.label(egui::RichText::new(t("skills_desc", lang)).size(14.0).color(TEXT_SECONDARY));
                ui.add_space(14.0);
                let workspace = st.selected_project_idx
                    .and_then(|i| st.projects.get(i))
                    .and_then(|p| p.paths.first().cloned());
                let skills = workspace
                    .map(|ws| app_lib::load_skills(std::path::Path::new(&ws)))
                    .unwrap_or_default();
                if skills.is_empty() {
                    ui.label(egui::RichText::new(t("no_skills", lang)).color(TEXT_MUTED));
                }
                for skill in &skills {
                    shown += settings_row(
                        ui, &search,
                        &format!("/{}", skill.name),
                        &skill.description,
                        |ui| {
                            ui.label(egui::RichText::new("✨").size(14.0));
                        },
                    ) as usize;
                }
            }
        }

        if shown == 0 && !search.trim().is_empty() {
            ui.label(egui::RichText::new(t("no_results", lang)).color(TEXT_MUTED));
        }

        if cfg_changed {
            let _ = cfg.save();
            *self.app_state.config.lock().unwrap() = cfg;
        }
    }

    /// 載入指定對話進主畫面（點擊 Session 續聊——歷史與審查狀態從 SQLite 取回）。
    fn load_conversation(&self, st: &mut UiState, cid: &str) {
        if let Ok(conn) = app_lib::open_connection(&self.app_state.db_path) {
            if let Ok(msgs) = app_lib::get_messages_for_conversation(&conn, cid) {
                st.active_messages = msgs.into_iter()
                    .map(|m| ChatMessage { role: m.role, content: m.content })
                    .collect();
                st.active_conversation_id = Some(cid.to_string());
            }
            // 右側 22 代理人面板還原為該 Session 最後一輪審查
            st.audit_results = app_lib::get_conversation_audits(&conn, cid)
                .map(|rows| rows.into_iter().map(|(agent_name, verdict, reason)| AuditResult {
                    agent_name, verdict, reason,
                }).collect())
                .unwrap_or_default();
        }
    }

    /// Session 列：點擊載入續聊、🗑 刪除。`filter` 為 project_id 過濾值；
    /// `select_project` 給定時，點擊 Session 同步切換目前專案（工作區跟著對齊）。
    fn render_session_rows(
        &self,
        ui: &mut egui::Ui,
        st: &mut UiState,
        lang: &str,
        filter: &str,
        select_project: Option<usize>,
    ) {
        let sessions: Vec<app_lib::ConversationSummary> = st.conversations.iter()
            .filter(|c| c.project_id.as_deref() == Some(filter))
            .cloned()
            .collect();

        if sessions.is_empty() {
            ui.label(egui::RichText::new(t("no_sessions", lang)).size(12.5).color(TEXT_MUTED));
            return;
        }

        let mut to_delete: Option<String> = None;
        for conv in &sessions {
            ui.horizontal(|ui| {
                let selected = st.active_conversation_id.as_deref() == Some(conv.id.as_str());
                let untitled = t("untitled", lang);
                let label = if conv.title.trim().is_empty() { untitled.as_str() } else { conv.title.as_str() };
                let label = truncate_chars(label, 14);
                if ui.selectable_label(
                    selected,
                    egui::RichText::new(format!("💬 {}", label)).size(13.5),
                ).on_hover_text(&conv.updated_at).clicked() {
                    self.load_conversation(st, &conv.id);
                    if let Some(idx) = select_project {
                        st.selected_project_idx = Some(idx);
                        st.selected_paths.clear();
                        let paths = st.projects.get(idx).map(|p| p.paths.clone()).unwrap_or_default();
                        for path in paths {
                            st.selected_paths.insert(path);
                        }
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🗑").clicked() {
                        to_delete = Some(conv.id.clone());
                    }
                });
            });
        }

        if let Some(cid) = to_delete {
            if let Ok(conn) = Connection::open(&self.app_state.db_path) {
                let _ = app_lib::delete_conversation(&conn, &cid);
                if st.active_conversation_id.as_deref() == Some(cid.as_str()) {
                    st.active_conversation_id = None;
                    st.active_messages.clear();
                    st.audit_results.clear();
                }
                if let Ok(convs) = app_lib::get_conversations(&conn) {
                    st.conversations = convs;
                }
            }
        }
    }

    /// 專案 Tab（Antigravity 風格）：＋新增專案；每個專案可展開，
    /// 底下巢狀該專案的對話 Session 與資料夾管理。
    fn render_projects_tab(&self, ui: &mut egui::Ui, st: &mut UiState, lang: &str) {
        // ＋ 新增專案：直接挑資料夾，專案名取資料夾名
        if ui.add(
            egui::Button::new(
                egui::RichText::new(format!("＋  {}", t("new_project", lang))).size(13.5),
            )
            .min_size(egui::Vec2::new(ui.available_width(), 28.0))
            .corner_radius(6),
        ).clicked() {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                let path_str = folder.to_string_lossy().to_string();
                let name = folder.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| t("new_project", lang));
                let json = serde_json::to_string(&vec![path_str.clone()]).unwrap_or_default();
                if let Ok(conn) = Connection::open(&self.app_state.db_path) {
                    if let Ok(id) = app_lib::create_project(&conn, &name, &json) {
                        st.projects.push(ProjectFolder { id, name, paths: vec![path_str.clone()] });
                        st.selected_project_idx = Some(st.projects.len() - 1);
                        st.selected_paths.clear();
                        st.selected_paths.insert(path_str);
                        st.active_conversation_id = None;
                        st.active_messages.clear();
                        st.audit_results.clear();
                    }
                }
            }
        }
        ui.add_space(6.0);

        let selected_idx = st.selected_project_idx;
        let projects_cloned = st.projects.clone();

        egui::ScrollArea::vertical().id_salt("projects_scroll").show(ui, |ui| {
            for (idx, p) in projects_cloned.iter().enumerate() {
                let is_selected = selected_idx == Some(idx);
                let header_text = if is_selected {
                    egui::RichText::new(format!("📂 {}", p.name)).size(14.0).color(ACCENT_BLUE).strong()
                } else {
                    egui::RichText::new(format!("📁 {}", p.name)).size(14.0).color(TEXT_PRIMARY)
                };
                let resp = egui::CollapsingHeader::new(header_text)
                    .id_salt(("project_hdr", p.id.as_str()))
                    .default_open(is_selected)
                    .show(ui, |ui| {
                        self.render_session_rows(ui, st, lang, &p.id, Some(idx));
                        ui.add_space(4.0);

                        // 資料夾管理收進子摺疊，Session 才是主角
                        egui::CollapsingHeader::new(
                            egui::RichText::new(format!("🗂 {}", t("folders", lang)))
                                .size(12.5).color(TEXT_SECONDARY),
                        )
                        .id_salt(("project_folders", p.id.as_str()))
                        .default_open(false)
                        .show(ui, |ui| {
                            for path in &p.paths {
                                let mut checked = st.selected_paths.contains(path);
                                let short = std::path::Path::new(path)
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.clone());
                                if ui.checkbox(&mut checked, &short).on_hover_text(path).changed() {
                                    if checked {
                                        st.selected_paths.insert(path.clone());
                                    } else {
                                        st.selected_paths.remove(path);
                                    }
                                }
                            }
                            if ui.button(format!("＋ {}", t("add_folder", lang))).clicked() {
                                if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
                                    let path_str = folder_path.to_string_lossy().to_string();
                                    if let Some(proj) = st.projects.get_mut(idx) {
                                        proj.paths.push(path_str.clone());
                                    }
                                    st.selected_paths.insert(path_str);
                                    if let Some(proj) = st.projects.get(idx) {
                                        if let Ok(conn) = Connection::open(&self.app_state.db_path) {
                                            let json = serde_json::to_string(&proj.paths).unwrap_or_default();
                                            let _ = app_lib::update_project_folders(&conn, &proj.id, &json);
                                        }
                                    }
                                }
                            }
                        });
                    });
                // 點專案名 = 選取該專案（工作區切換）
                if resp.header_response.clicked() && !is_selected {
                    st.selected_project_idx = Some(idx);
                    st.selected_paths.clear();
                    for path in &p.paths {
                        st.selected_paths.insert(path.clone());
                    }
                    st.active_conversation_id = None;
                    st.active_messages.clear();
                    st.audit_results.clear();
                }
            }
        });

        let folder_count = st.selected_paths.len();
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(t_fmt("selected_folders", lang, folder_count))
                .size(12.0).color(TEXT_MUTED),
        );
    }

    /// 全域 Tab：操控整台電腦的對話 Session（與專案完全分流）。
    fn render_global_tab(&self, ui: &mut egui::Ui, st: &mut UiState, lang: &str) {
        ui.label(
            egui::RichText::new(t("global_tab_desc", lang))
                .size(12.5).color(ACCENT_ORANGE),
        );
        ui.add_space(8.0);
        egui::ScrollArea::vertical().id_salt("global_scroll").show(ui, |ui| {
            self.render_session_rows(ui, st, lang, app_lib::GLOBAL_PROJECT_ID, None);
        });
    }

    /// 輸入卡（Codex 風格）：多行輸入 + 工具列（＋選單、模式、模型、送出/中止）。
    /// 回傳 true 表示使用者觸發送出（呼叫端負責 drop 鎖後執行 handle_send）。
    fn render_input_card(&self, ui: &mut egui::Ui, st: &mut UiState, is_running: bool, lang: &str) -> bool {
        let mut send_requested = false;
        let model_name = self.app_state.config.lock().unwrap().api.model.clone();

        egui::Frame::default()
            .fill(BG_SECONDARY)
            .stroke(egui::Stroke::new(1.0, BORDER))
            .corner_radius(12)
            .inner_margin(12.0)
            .show(ui, |ui| {
                // 多行輸入區
                let text_edit = egui::TextEdit::multiline(&mut st.chat_input)
                    .desired_width(f32::INFINITY)
                    .desired_rows(2)
                    .hint_text(t("ask_placeholder", lang))
                    .interactive(!is_running)
                    .frame(false);
                let response = ui.add(text_edit);

                ui.add_space(8.0);

                // 工具列
                ui.horizontal(|ui| {
                    // ＋ 選單
                    ui.menu_button(egui::RichText::new("＋").size(17.0).strong(), |ui| {
                        if ui.button(format!("🗑 {}", t("clear_chat", lang))).clicked() {
                            st.active_messages.clear();
                            st.audit_results.clear();
                            st.active_conversation_id = None;
                            ui.close_menu();
                            ui.ctx().request_repaint();
                        }
                        if ui.button(format!("📂 {}", t("add_project_folder", lang))).clicked() {
                            ui.close_menu();
                            ui.ctx().request_repaint();
                            if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
                                let path_str = folder_path.to_string_lossy().to_string();
                                if let Some(idx) = st.selected_project_idx {
                                    let mut updated_paths = None;
                                    let mut project_id = None;
                                    if let Some(proj) = st.projects.get(idx) {
                                        let mut paths = proj.paths.clone();
                                        paths.push(path_str.clone());
                                        updated_paths = Some(paths);
                                        project_id = Some(proj.id.clone());
                                    }
                                    if let (Some(paths), Some(pid)) = (updated_paths, project_id) {
                                        st.selected_paths.insert(path_str.clone());
                                        if let Ok(conn) = Connection::open(&self.app_state.db_path) {
                                            let json = serde_json::to_string(&paths).unwrap_or_default();
                                            let _ = app_lib::update_project_folders(&conn, &pid, &json);
                                        }
                                        if let Some(proj) = st.projects.get_mut(idx) {
                                            proj.paths = paths;
                                        }
                                    }
                                }
                            }
                        }
                    });

                    // 專案選擇（Codex 底欄風格）
                    let project_label = st.selected_project_idx
                        .and_then(|i| st.projects.get(i))
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| t("projects", lang));
                    ui.menu_button(
                        egui::RichText::new(format!("📁 {} ⏷", project_label)).size(13.0).color(TEXT_SECONDARY),
                        |ui| {
                            let projects = st.projects.clone();
                            for (idx, p) in projects.iter().enumerate() {
                                if ui.selectable_label(st.selected_project_idx == Some(idx), &p.name).clicked() {
                                    st.selected_project_idx = Some(idx);
                                    st.selected_paths.clear();
                                    for path in &p.paths {
                                        st.selected_paths.insert(path.clone());
                                    }
                                    ui.close_menu();
                                }
                            }
                        },
                    );

                    // 模式徽章（專案=本機作業 / 全域）
                    let (mode_icon, mode_key, mode_color) = if st.work_mode == "global" {
                        ("🌍", "work_mode_global", ACCENT_ORANGE)
                    } else {
                        ("💻", "local_work", TEXT_SECONDARY)
                    };
                    if ui.add_enabled(
                        !is_running,
                        egui::Button::new(
                            egui::RichText::new(format!("{} {} ⏷", mode_icon, t(mode_key, lang)))
                                .color(mode_color).size(13.0),
                        ).frame(false),
                    ).clicked() {
                        st.work_mode = if st.work_mode == "global" { "project".into() } else { "global".into() };
                        st.sidebar_tab = if st.work_mode == "global" { 1 } else { 0 };
                        let mut cfg = self.app_state.config.lock().unwrap().clone();
                        cfg.general.project_mode = st.work_mode.clone();
                        let _ = cfg.save();
                        *self.app_state.config.lock().unwrap() = cfg;
                    }

                    // 右對齊：送出 / 中止 + 模型名
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_running {
                            let abort_btn = egui::Button::new(
                                egui::RichText::new(format!("■ {}", t("abort", lang)))
                                    .color(TEXT_PRIMARY).strong().size(14.0),
                            )
                            .fill(ACCENT_RED)
                            .corner_radius(14)
                            .min_size(egui::Vec2::new(64.0, 30.0));
                            if ui.add(abort_btn).clicked() {
                                if let Ok(mut s) = self.app_state.agent_state.try_lock() {
                                    *s = AgentExecutionState::Idle;
                                }
                                ui.ctx().request_repaint();
                            }
                        } else {
                            let send_btn = egui::Button::new(
                                egui::RichText::new("↑").color(TEXT_PRIMARY).strong().size(16.0),
                            )
                            .fill(ACCENT_BLUE)
                            .corner_radius(15)
                            .min_size(egui::Vec2::new(30.0, 30.0));
                            let clicked = ui.add(send_btn).clicked();

                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let shift_pressed = ui.input(|i| i.modifiers.shift);
                            if clicked || (enter_pressed && !shift_pressed && response.has_focus()) {
                                if st.chat_input.ends_with('\n') {
                                    st.chat_input.pop();
                                }
                                let trimmed = st.chat_input.trim().to_string();
                                if !trimmed.is_empty() {
                                    st.chat_input = trimmed;
                                    send_requested = true;
                                }
                            }
                        }
                        ui.label(egui::RichText::new(&model_name).color(TEXT_MUTED).size(12.0));
                    });
                });
            });

        send_requested
    }

    fn handle_send(&self, ctx: &egui::Context) {
        let (_prompt, conversation_id, config, workspace_path) = {
            let mut st = self.ui_state.lock().unwrap();
            let prompt = st.chat_input.clone();
            if prompt.trim().is_empty() { return; }
            st.chat_input.clear();

            let conv_id = match &st.active_conversation_id {
                Some(id) => id.clone(),
                None => {
                    let Ok(conn) = Connection::open(&self.app_state.db_path) else {
                        st.status_message = "資料庫開啟失敗".into();
                        return;
                    };
                    // Session 歸屬：全域模式掛 global 哨兵，否則掛目前專案
                    let scope_project_id = if st.work_mode == "global" {
                        Some(app_lib::GLOBAL_PROJECT_ID.to_string())
                    } else {
                        st.selected_project_idx
                            .and_then(|i| st.projects.get(i))
                            .map(|p| p.id.clone())
                    };
                    // 標題以「字元」截斷——位元組切片遇中文必 panic
                    let new_id = app_lib::create_conversation(
                        &conn,
                        &truncate_chars(&prompt, 20),
                        scope_project_id.as_deref(),
                    ).unwrap_or_default();
                    st.active_conversation_id = Some(new_id.clone());
                    new_id
                }
            };

            st.active_messages.push(ChatMessage { role: "user".into(), content: prompt.clone() });

            if let Ok(conn) = Connection::open(&self.app_state.db_path) {
                let _ = app_lib::add_conversation_message(&conn, &conv_id, "user", &prompt);
            }

            // 工作區確定性選擇：依專案資料夾的宣告順序取第一個被勾選者，
            // 不依賴 HashSet 迭代順序（那是隨機的）
            let workspace = st.selected_project_idx
                .and_then(|i| st.projects.get(i))
                .and_then(|p| {
                    p.paths.iter()
                        .find(|path| st.selected_paths.contains(*path))
                        .cloned()
                        .or_else(|| p.paths.first().cloned())
                })
                .or_else(|| {
                    let mut sorted: Vec<String> = st.selected_paths.iter().cloned().collect();
                    sorted.sort();
                    sorted.into_iter().next()
                })
                .unwrap_or_default();

            (prompt, conv_id, self.app_state.config.clone(), workspace)
        };

        // Reload conversation list
        if let Ok(conn) = Connection::open(&self.app_state.db_path) {
            if let Ok(convs) = app_lib::get_conversations(&conn) {
                self.ui_state.lock().unwrap().conversations = convs;
            }
        }

        let app_state_spawn = self.app_state.clone();
        let app_state_task  = self.app_state.clone();
        let ui_state  = self.ui_state.clone();
        let ctx_clone = ctx.clone();

        app_state_spawn.engine_runtime.spawn(async move {
            {
                let mut s = app_state_task.agent_state.lock().await;
                *s = AgentExecutionState::Running(std::time::Instant::now());
            }

            let agent_loop = AgentLoop::new(config.lock().unwrap().clone(), workspace_path);
            let mut messages = Vec::new();

            if let Ok(conn) = Connection::open(&app_state_task.db_path) {
                if let Ok(history) = app_lib::get_messages_for_conversation(&conn, &conversation_id) {
                    for h in history {
                        messages.push(serde_json::json!({ "role": h.role, "content": h.content }));
                    }
                }
            }

            let step_result = agent_loop.run_step(
                &mut messages,
                &app_state_task.mcp_manager,
                &app_state_task.token_budgeter,
                &app_state_task.db_path,
            ).await;

            // 使用者按了中止：丟棄遲到的結果，不寫庫、不更新 UI
            let aborted = matches!(
                *app_state_task.agent_state.lock().await,
                AgentExecutionState::Idle,
            );

            match step_result {
                Ok(step) if !aborted => {
                    if let Ok(conn) = app_lib::open_connection(&app_state_task.db_path) {
                        let _ = app_lib::add_conversation_message(
                            &conn, &conversation_id, "assistant", &step.response_text,
                        );
                        // 工具執行結果持久化並顯示——使用者必須看得到 AI 實際做了什麼
                        for res in &step.execution_results {
                            let _ = app_lib::add_conversation_message(
                                &conn, &conversation_id, "tool", res,
                            );
                        }
                        let mut st = ui_state.lock().unwrap();
                        st.active_messages.push(ChatMessage {
                            role:    "assistant".into(),
                            content: step.response_text.clone(),
                        });
                        for res in &step.execution_results {
                            st.active_messages.push(ChatMessage {
                                role:    "tool".into(),
                                content: res.clone(),
                            });
                        }
                        // 審查結果按對話持久化——右側面板跟著 Session 走，切換可還原
                        let rows: Vec<(String, String, String)> = step.audits.iter()
                            .map(|a| (a.agent_name.clone(), a.verdict.clone(), a.reason.clone()))
                            .collect();
                        if let Err(e) = app_lib::replace_conversation_audits(&conn, &conversation_id, &rows) {
                            eprintln!("[AUDIT] 持久化失敗（{} 筆）: {}", rows.len(), e);
                        }
                        st.audit_results = step.audits;
                    }
                    if step.requires_approval {
                        let mut pending = app_state_task.pending_state.lock().await;
                        *pending = Some(PendingState {
                            pending_tools:    step.executed_tools,
                            pending_response: step.response_text,
                            workspace_path:   agent_loop.workspace_path.to_string_lossy().to_string(),
                            conversation_id:  conversation_id.clone(),
                        });
                    }
                }
                Ok(_) => {} // aborted：丟棄
                Err(e) => {
                    if !aborted {
                        ui_state.lock().unwrap().status_message = format!("Error: {}", e);
                    }
                }
            }

            if !aborted {
                let mut s = app_state_task.agent_state.lock().await;
                *s = AgentExecutionState::Complete;
            }
            ctx_clone.request_repaint();
        });
    }
}

impl eframe::App for AgnesApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Style ─────────────────────────────────────────────────────────────
        // 介面縮放：使用者可調（先前強制 1.0 是高解析螢幕「字太小」的元兇）
        let ui_scale = self.app_state.config.lock().unwrap().general.ui_scale
            .clamp(app_lib::UI_SCALE_MIN, app_lib::UI_SCALE_MAX);
        ctx.set_pixels_per_point(ui_scale);
        let mut style = (*ctx.style()).clone();
        // 全域字級拉高：egui 預設 Body/Button 12.5、Small 9——桌面高解析下過小
        style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(FONT_HEADING));
        style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(FONT_BODY));
        style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(FONT_BUTTON));
        style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(FONT_SMALL));
        style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(FONT_MONO));
        style.visuals.dark_mode = true;
        style.visuals.extreme_bg_color        = BG_PRIMARY;
        style.visuals.window_fill             = BG_SECONDARY;
        style.visuals.panel_fill              = BG_SECONDARY;
        style.visuals.window_stroke           = egui::Stroke::new(1.0, BORDER);
        style.visuals.widgets.inactive.bg_fill     = BG_SECONDARY;
        style.visuals.widgets.inactive.fg_stroke   = egui::Stroke::new(1.0, TEXT_PRIMARY);
        style.visuals.widgets.hovered.bg_fill      = BG_HOVER;
        style.visuals.widgets.hovered.fg_stroke    = egui::Stroke::new(1.0, TEXT_PRIMARY);
        style.visuals.widgets.active.bg_fill       = BG_HOVER;
        style.visuals.widgets.noninteractive.bg_fill = BG_SECONDARY;
        style.visuals.selection.bg_fill            = ACCENT_BLUE;
        style.visuals.override_text_color          = Some(TEXT_PRIMARY);
        ctx.set_style(style);

        let lang = self.ui_state.lock().unwrap().language.clone();

        // ── Agent running flag (try_lock = synchronous, never blocks render) ──
        let is_running = self.app_state.agent_state.try_lock()
            .map(|s| matches!(*s, AgentExecutionState::Running(_)))
            .unwrap_or(false);
        if is_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // ── Token budget snapshot (try_lock = synchronous, never blocks render) ───
        let (budget_ratio, spent, budget_total) = {
            if let Ok(b) = self.app_state.token_budgeter.try_lock() {
                (b.budget_ratio(), b.total_spent(), b.session_budget)
            } else {
                (0.0_f64, 0_u64, self.app_state.config.lock().unwrap().api.session_budget)
            }
        };

        // ── Top menu bar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar")
            .exact_height(36.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 0.0);
                    ui.label(
                        egui::RichText::new("Agnes AI")
                            .size(16.0).color(ACCENT_BLUE).strong(),
                    );
                    ui.separator();

                    // Language toggle in title bar
                    let lang_btn = if lang == "zh" { "EN" } else { "中" };
                    if ui.small_button(lang_btn).on_hover_text(t("language", &lang)).clicked() {
                        let mut st = self.ui_state.lock().unwrap();
                        st.language = if st.language == "zh" { "en".into() } else { "zh".into() };
                        let mut cfg = self.app_state.config.lock().unwrap().clone();
                        cfg.general.language = if st.language == "zh" { "zh-TW".into() } else { "en-US".into() };
                        let _ = cfg.save();
                        *self.app_state.config.lock().unwrap() = cfg;
                    }
                    ui.separator();

                    // Token budget bar
                    let budget_color = if budget_ratio >= 1.0 {
                        ACCENT_RED
                    } else if budget_ratio >= 0.8 {
                        ACCENT_YELLOW
                    } else {
                        ACCENT_GREEN
                    };

                    ui.label(
                        egui::RichText::new(t("token_budget", &lang))
                            .size(13.0).color(TEXT_SECONDARY),
                    );
                    let bar_rect = ui.allocate_space(egui::Vec2::new(80.0, 10.0)).1;
                    ui.painter().rect_filled(bar_rect, 3.0, BG_TERTIARY);
                    let fill_w = bar_rect.width() * (budget_ratio as f32).min(1.0);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(bar_rect.min, egui::Vec2::new(fill_w, bar_rect.height())),
                        3.0,
                        budget_color,
                    );
                    ui.label(
                        egui::RichText::new(format!("{}/{}", spent, budget_total))
                            .size(12.0).color(TEXT_MUTED),
                    );
                    if ui.small_button("↻")
                        .on_hover_text(if lang == "zh" { "重設 Token 計數" } else { "Reset token counter" })
                        .clicked()
                    {
                        if let Ok(mut b) = self.app_state.token_budgeter.try_lock() {
                            b.spent_prompt = 0;
                            b.spent_completion = 0;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button(egui::RichText::new(t("menu_view", &lang)).size(13.5), |ui| {
                            if ui.button(format!("🌐 {}", t("language", &lang))).clicked() {
                                let mut st = self.ui_state.lock().unwrap();
                                st.language = if st.language == "zh" { "en".into() } else { "zh".into() };
                                let mut cfg = self.app_state.config.lock().unwrap().clone();
                                cfg.general.language = if st.language == "zh" { "zh-TW".into() } else { "en-US".into() };
                                let _ = cfg.save();
                                *self.app_state.config.lock().unwrap() = cfg;
                                ui.close_menu();
                            }
                            let mode_label = {
                                let st = self.ui_state.lock().unwrap();
                                if st.work_mode == "global" { t("work_mode_project", &lang) } else { t("work_mode_global", &lang) }
                            };
                            if ui.button(format!("⇄ {}", mode_label)).clicked() {
                                let mut st = self.ui_state.lock().unwrap();
                                st.work_mode = if st.work_mode == "global" { "project".into() } else { "global".into() };
                                st.sidebar_tab = if st.work_mode == "global" { 1 } else { 0 };
                                let mut cfg = self.app_state.config.lock().unwrap().clone();
                                cfg.general.project_mode = st.work_mode.clone();
                                let _ = cfg.save();
                                *self.app_state.config.lock().unwrap() = cfg;
                                ui.close_menu();
                            }
                        });
                        ui.menu_button(egui::RichText::new(t("menu_file", &lang)).size(13.5), |ui| {
                            if ui.button(format!("＋ {}", t("new_conversation", &lang))).clicked() {
                                let mut st = self.ui_state.lock().unwrap();
                                st.active_conversation_id = None;
                                st.active_messages.clear();
                                st.audit_results.clear();
                                ui.close_menu();
                            }
                            if ui.button(format!("⚙ {}", t("settings", &lang))).clicked() {
                                self.ui_state.lock().unwrap().settings_open = true;
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(format!("⏻ {}", t("exit_app", &lang))).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    });
                });
            });

        // ── 設定頁（Codex 全頁式，接管選單列以下全部區域）────────────────────
        let settings_open = self.ui_state.lock().unwrap().settings_open;
        if settings_open {
            self.render_settings_page(ctx, &lang);
            self.qa_screenshot_hook(ctx);
            return;
        }

        // ── Left sidebar（Antigravity 風格：專案/全域 Tab + 巢狀 Session）──────
        egui::SidePanel::left("sidebar")
            .default_width(240.0)
            .min_width(200.0)
            .max_width(340.0)
            .show(ctx, |ui| {
                let mut st = self.ui_state.lock().unwrap();
                ui.add_space(8.0);

                // Tab 列：📁 專案 | 🌍 全域 —— 切 Tab 即切工作模式
                let mut switch_to: Option<usize> = None;
                ui.horizontal(|ui| {
                    let half = (ui.available_width() - 6.0) / 2.0;
                    let tabs = [
                        (0_usize, "📁", "tab_projects", ACCENT_BLUE),
                        (1_usize, "🌍", "tab_global", ACCENT_ORANGE),
                    ];
                    for (tab_idx, icon, key, color) in tabs {
                        let active = st.sidebar_tab == tab_idx;
                        let (fill, text_color) = if active {
                            (egui::Color32::from_rgb(40, 60, 100), color)
                        } else {
                            (BG_TERTIARY, TEXT_SECONDARY)
                        };
                        if ui.add_sized(
                            egui::vec2(half, 30.0),
                            egui::Button::new(
                                egui::RichText::new(format!("{} {}", icon, t(key, &lang)))
                                    .size(14.0).color(text_color).strong(),
                            ).fill(fill).corner_radius(6),
                        ).clicked() && !active {
                            switch_to = Some(tab_idx);
                        }
                    }
                });
                if let Some(tab_idx) = switch_to {
                    st.sidebar_tab = tab_idx;
                    st.work_mode = if tab_idx == 1 { "global".into() } else { "project".into() };
                    st.active_conversation_id = None;
                    st.active_messages.clear();
                    st.audit_results.clear();
                    let mut cfg = self.app_state.config.lock().unwrap().clone();
                    cfg.general.project_mode = st.work_mode.clone();
                    let _ = cfg.save();
                    *self.app_state.config.lock().unwrap() = cfg;
                }

                ui.add_space(8.0);

                // ＋ 新增對話：在目前 Tab 的範疇下開新 Session
                let new_conv_btn = egui::Button::new(
                    egui::RichText::new(format!("＋  {}", t("new_conversation", &lang)))
                        .size(14.0).color(TEXT_PRIMARY).strong(),
                )
                .fill(egui::Color32::from_rgb(40, 60, 100))
                .corner_radius(8)
                .min_size(egui::Vec2::new(ui.available_width(), 36.0));
                if ui.add(new_conv_btn).clicked() {
                    st.active_conversation_id = None;
                    st.active_messages.clear();
                    st.audit_results.clear();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                if st.sidebar_tab == 1 {
                    self.render_global_tab(ui, &mut st, &lang);
                } else {
                    self.render_projects_tab(ui, &mut st, &lang);
                }

                // 底部釘住：⚙ 設定
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(8.0);
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new(format!("⚙  {}", t("settings", &lang))).size(14.0),
                        ).frame(false)
                    ).clicked() {
                        st.settings_open = true;
                    }
                    ui.separator();
                });
            });

        // ── Right Agent Panel ─────────────────────────────────────────────────
        egui::SidePanel::right("agent_panel")
            .default_width(260.0)
            .min_width(220.0)
            .max_width(360.0)
            .show(ctx, |ui| {
                // Paint a brighter background manually so agent panel stands out
                let panel_rect = ui.max_rect();
                ui.painter().rect_filled(panel_rect, 0.0, BG_TERTIARY);
                ui.painter().line_segment(
                    [panel_rect.left_top(), panel_rect.left_bottom()],
                    egui::Stroke::new(1.0, BORDER),
                );

                ui.label(egui::RichText::new(format!("🤖 {}", t("agent_status", &lang))).size(14.5).color(TEXT_PRIMARY).strong());

                // 範疇副標：面板狀態跟著目前 Session/專案走
                let (scope_text, scope_color) = {
                    let st = self.ui_state.lock().unwrap();
                    if st.work_mode == "global" {
                        (t("panel_scope_global", &lang), ACCENT_ORANGE)
                    } else {
                        let project = st.selected_project_idx
                            .and_then(|i| st.projects.get(i))
                            .map(|p| p.name.clone())
                            .unwrap_or_default();
                        let session = st.active_conversation_id.as_deref()
                            .and_then(|cid| st.conversations.iter().find(|c| c.id == cid))
                            .map(|c| truncate_chars(&c.title, 10));
                        match session {
                            Some(s) => (format!("📂 {} / 💬 {}", project, s), ACCENT_BLUE),
                            None => (format!("📂 {}", project), TEXT_SECONDARY),
                        }
                    }
                };
                ui.label(egui::RichText::new(scope_text).size(12.0).color(scope_color));
                let has_audits = !self.ui_state.lock().unwrap().audit_results.is_empty();
                ui.label(
                    egui::RichText::new(if has_audits { t("legend_agents", &lang) } else { t("panel_scope_idle", &lang) })
                        .size(11.5).color(TEXT_MUTED),
                );
                ui.add_space(6.0);

                let groups: &[(&str, &[&str])] = &[
                    ("G1 記憶蒸餾", &["ContextDistillerAlpha", "ContextDistillerBeta", "DistillationIntegrator", "FactHallucinationAuditor", "TokenOverlapAuditor"]),
                    ("G2 工作流",   &["WorkflowTopology", "WorkflowRuntimeEvaluator", "SlopVibeAuditor", "SlopPathPurgeSpecialist"]),
                    ("G3 指揮",     &["OrchestratorAgent", "LocaleCalibrationSpecialist", "LeadSystemArchitect"]),
                    ("G4 效能",     &["PerformanceArchitectureEngineer", "ResourceAnalyticsEngineer", "MemoryEfficiencyReviewer"]),
                    ("G5 安全",     &["SecurityArchitectureDesigner", "DefensiveCodingSpecialist", "SecurityComplianceAuditor"]),
                    ("G6 工程",     &["CoreEngineCoder", "IntegrationEngineer", "MultimodalMediaSpecialist", "SandboxRuntimeTester"]),
                ];

                let audits = self.ui_state.lock().unwrap().audit_results.clone();

                egui::ScrollArea::vertical().id_salt("agent_scroll").show(ui, |ui| {
                    for (g_name, agents) in groups {
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new(*g_name).color(ACCENT_BLUE).strong().size(13.0));
                        for a in *agents {
                            let audit = audits.iter().find(|x| x.agent_name == *a);
                            let (status_color, status_text, name_color) = match audit {
                                Some(r) if r.verdict == "PASSED"   => (ACCENT_GREEN,  "✓", TEXT_PRIMARY),
                                Some(r) if r.verdict == "REJECTED" => (ACCENT_RED,    "✗", ACCENT_RED),
                                Some(r) if r.verdict == "SKIPPED"  => (TEXT_SECONDARY, "~", TEXT_SECONDARY),
                                // DORMANT：按路由休眠（未激活 = 零成本），名稱同步轉灰
                                Some(_) => (TEXT_MUTED, "·", TEXT_MUTED),
                                None    => (TEXT_MUTED, "·", TEXT_PRIMARY),
                            };
                            let row = ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("  {}", a)).size(12.5).color(name_color));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(status_text).color(status_color).size(13.0));
                                });
                            });
                            // hover 顯示該代理本輪裁決原因（07_UI_SPEC：點代理看 gate 結果）
                            if let Some(r) = audit {
                                row.response.on_hover_text(&r.reason);
                            }
                        }
                        ui.add_space(3.0);
                    }
                });

                // ConfirmationGate (try_lock = synchronous, safe in render loop)
                let pending_state = self.app_state.pending_state
                    .try_lock().ok().and_then(|g| g.clone());

                if let Some(pending) = pending_state {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(t("pending_approval", &lang))
                            .size(13.5).color(ACCENT_ORANGE).strong(),
                    );
                    for tool in &pending.pending_tools {
                        egui::Frame::default()
                            .fill(BG_CARD)
                            .corner_radius(4)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(&tool.name).size(13.0).strong());
                                if let Some(ref path) = tool.path {
                                    ui.label(egui::RichText::new(path).size(12.0).color(TEXT_SECONDARY));
                                }
                            });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("✓ Approve").clicked() {
                            let app_state = self.app_state.clone();
                            let ui_state = self.ui_state.clone();
                            let ctx2 = ctx.clone();
                            self.app_state.engine_runtime.spawn(async move {
                                let taken = app_state.pending_state.lock().await.take();
                                if let Some(p) = taken {
                                    // 沿用送出當下的工作區——空工作區會讓路徑圈禁失效
                                    let lp = AgentLoop::new(
                                        app_state.config.lock().unwrap().clone(),
                                        p.workspace_path.clone(),
                                    );
                                    let mut results = Vec::new();
                                    for tool in &p.pending_tools {
                                        results.push(lp.execute_tool(tool, &app_state.mcp_manager).await);
                                    }
                                    // 結果入庫 + 顯示於聊天流
                                    if let Ok(conn) = Connection::open(&app_state.db_path) {
                                        for r in &results {
                                            let _ = app_lib::add_conversation_message(
                                                &conn, &p.conversation_id, "tool", r,
                                            );
                                        }
                                    }
                                    let mut st = ui_state.lock().unwrap();
                                    for r in results {
                                        st.active_messages.push(ChatMessage {
                                            role: "tool".into(),
                                            content: r,
                                        });
                                    }
                                }
                                ctx2.request_repaint();
                            });
                        }
                        if ui.button("✕ Reject").clicked() {
                            // try_lock is safe: if lock held briefly by another task,
                            // the pending state clears on the next frame instead.
                            if let Ok(mut lock) = self.app_state.pending_state.try_lock() {
                                *lock = None;
                            }
                        }
                    });
                }
            });

        // ── Central Panel ─────────────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut st = self.ui_state.lock().unwrap();

            // Global mode warning banner
            if st.work_mode == "global" {
                egui::Frame::default()
                    .fill(egui::Color32::from_rgba_premultiplied(255, 120, 50, 15))
                    .stroke(egui::Stroke::new(1.0, ACCENT_ORANGE))
                    .corner_radius(6)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🌐").size(16.0));
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(t("global_warning", &lang))
                                        .strong().color(ACCENT_ORANGE),
                                );
                            });
                        });
                    });
                ui.add_space(8.0);
            }

            // Status bar (error / info)
            if !st.status_message.is_empty() {
                ui.label(egui::RichText::new(&st.status_message).size(13.0).color(ACCENT_RED));
                ui.add_space(4.0);
            }

            // ── Codex 風格空狀態：置中大標題提問 + 置中輸入卡 ──
            let chat_is_empty = st.active_messages.is_empty();
            if chat_is_empty && !is_running {
                let avail = ui.available_height();
                ui.add_space((avail * 0.30).max(40.0));
                let project_name = st.selected_project_idx
                    .and_then(|i| st.projects.get(i))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Agnes AI".to_string());
                // 全域模式不顯示專案名——範疇是整台電腦（GUI QA 實測抓到的誤導文案）
                let heading = if st.work_mode == "global" {
                    t("welcome_global", &lang)
                } else {
                    t_with("welcome_question", &lang, &project_name)
                };
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(heading)
                            .size(28.0).color(TEXT_PRIMARY).strong(),
                    );
                });
                ui.add_space(28.0);
                let mut send = false;
                // 樸素置中（vertical_centered+set_max_width 巢狀會吃掉子元件點擊）
                let total = ui.available_width();
                let inner = total.min(760.0);
                let margin = ((total - inner) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.vertical(|ui| {
                        ui.set_width(inner);
                        send = self.render_input_card(ui, &mut st, is_running, &lang);
                    });
                });
                if send {
                    drop(st);
                    self.handle_send(ctx);
                }
                return;
            }

            // ── 對話進行中：訊息流 + 底部輸入卡 ──
            let avail_height = ui.available_height() - 130.0;

            egui::ScrollArea::vertical()
                .max_height(avail_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap); // Enable word wrapping globally in scroll area
                    {
                        for (msg_idx, msg) in st.active_messages.iter().enumerate() {
                            let is_user = msg.role == "user";
                            let is_tool = msg.role == "tool";
                            
                            let bg = if is_user {
                                BG_TERTIARY
                            } else if is_tool {
                                egui::Color32::from_rgb(20, 24, 33)
                            } else {
                                BG_CARD
                            };
                            
                            let name_color = if is_user {
                                ACCENT_BLUE
                            } else if is_tool {
                                ACCENT_GREEN
                            } else {
                                ACCENT_ORANGE
                            };
                            
                            let name = if is_user {
                                if lang == "zh" { "你" } else { "You" }
                            } else if is_tool {
                                if lang == "zh" { "🛠 執行結果" } else { "🛠 Tool Output" }
                            } else {
                                "Agnes AI"
                            };

                            egui::Frame::default()
                                .fill(bg)
                                .corner_radius(8)
                                .stroke(egui::Stroke::new(1.0, BORDER))
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                    ui.label(
                                        egui::RichText::new(name).color(name_color).strong().size(15.5),
                                    );
                                    ui.add_space(4.0);
                                    if is_tool {
                                        render_collapsible_tool_output(ui, &msg.content, msg_idx);
                                    } else {
                                        render_message_content(ui, &msg.content, msg_idx);
                                    }
                                });
                            ui.add_space(8.0);
                        }
                    }

                    // Render pulsing loader spinner card if agent is running
                    if is_running {
                        let pulsing_bg = egui::Color32::from_rgb(30, 34, 42);
                        egui::Frame::default()
                            .fill(pulsing_bg)
                            .corner_radius(8)
                            .stroke(egui::Stroke::new(1.0, ACCENT_BLUE))
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Agnes AI").color(ACCENT_BLUE).strong().size(16.0));
                                    ui.add(egui::Spinner::new());
                                    ui.label(egui::RichText::new(t("running_hint", &lang))
                                        .color(TEXT_SECONDARY).size(16.0));
                                });
                            });
                    }
                });

            // 底部釘住輸入卡（Codex / Antigravity 2.0 風格）
            let mut send = false;
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(8.0);
                let total = ui.available_width();
                let inner = total.min(860.0);
                let margin = ((total - inner) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.vertical(|ui| {
                        ui.set_width(inner);
                        send = self.render_input_card(ui, &mut st, is_running, &lang);
                    });
                });
            });
            if send {
                drop(st);
                self.handle_send(ctx);
            }
        });

        // ── QA 自我截圖鉤子 ───────────────────────────────────────────────────
        self.qa_screenshot_hook(ctx);
    }
}

// ─── 訊息渲染：收合式區塊（對標 Claude/Codex/Antigravity 摺疊長輸出）──────────

/// 超過此行數的碼塊/工具輸出預設收合
const COLLAPSE_LINES_THRESHOLD: usize = 8;
/// 收合標題的摘要字元數
const COLLAPSE_SUMMARY_CHARS: usize = 60;

fn emit_text(ui: &mut egui::Ui, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    ui.add(
        egui::Label::new(
            egui::RichText::new(text.trim_end()).size(FONT_BODY).color(TEXT_PRIMARY),
        )
        .wrap(),
    );
}

fn emit_mono_frame(ui: &mut egui::Ui, text: &str) {
    egui::Frame::default()
        .fill(egui::Color32::from_rgb(20, 24, 33))
        .corner_radius(4)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text.trim_end())
                        .font(egui::FontId::monospace(FONT_MONO))
                        .color(TEXT_PRIMARY),
                )
                .wrap(),
            );
        });
}

/// 短內容直接顯示；長內容收合為「標題（N 行）」可展開列。
fn emit_collapsible(ui: &mut egui::Ui, title: &str, body: &str, salt: (usize, usize)) {
    let lines = body.lines().count();
    if lines <= COLLAPSE_LINES_THRESHOLD {
        if !title.is_empty() {
            ui.label(egui::RichText::new(title).size(13.0).color(TEXT_SECONDARY));
        }
        emit_mono_frame(ui, body);
        return;
    }
    egui::CollapsingHeader::new(
        egui::RichText::new(format!("{}（{} 行）", title, lines))
            .size(13.5)
            .color(TEXT_SECONDARY),
    )
    .id_salt(("collapse_blk", salt))
    .default_open(false)
    .show(ui, |ui| emit_mono_frame(ui, body));
}

fn extract_attr(line: &str, attr: &str) -> String {
    let needle = format!("{}=\"", attr);
    line.find(&needle)
        .and_then(|i| {
            let rest = &line[i + needle.len()..];
            rest.find('"').map(|j| rest[..j].to_string())
        })
        .unwrap_or_default()
}

/// 助理訊息渲染：一般文字直出；``` 碼塊與工具呼叫 XML 區塊收合（避免長文洗版）。
fn render_message_content(ui: &mut egui::Ui, content: &str, msg_idx: usize) {
    let mut text_buf = String::new();
    let mut block_buf = String::new();
    let mut in_code = false;
    let mut tool_close: Option<(String, String)> = None; // (結束標籤, 收合標題)
    let mut block_idx = 0usize;

    for line in content.lines() {
        // 工具區塊內：累積直到結束標籤
        if tool_close.is_some() {
            let hit_close = {
                let (close_tag, _) = tool_close.as_ref().unwrap();
                line.trim_start().starts_with(close_tag.as_str())
            };
            if hit_close {
                let (_, title) = tool_close.take().unwrap();
                emit_collapsible(ui, &title, &block_buf, (msg_idx, block_idx));
                block_idx += 1;
                block_buf.clear();
            } else {
                block_buf.push_str(line);
                block_buf.push('\n');
            }
            continue;
        }
        // 碼塊內：累積直到 ``` 結束
        if in_code {
            if line.trim_start().starts_with("```") {
                emit_collapsible(ui, "📄 程式碼", &block_buf, (msg_idx, block_idx));
                block_idx += 1;
                block_buf.clear();
                in_code = false;
            } else {
                block_buf.push_str(line);
                block_buf.push('\n');
            }
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            emit_text(ui, &text_buf);
            text_buf.clear();
            in_code = true;
        } else if trimmed.starts_with("<write_file") {
            emit_text(ui, &text_buf);
            text_buf.clear();
            let path = extract_attr(trimmed, "path");
            tool_close = Some(("</write_file>".into(), format!("✍ write_file：{}", path)));
        } else if trimmed.starts_with("<run_command>") {
            emit_text(ui, &text_buf);
            text_buf.clear();
            tool_close = Some(("</run_command>".into(), "⚡ run_command".into()));
        } else if trimmed.starts_with("<run_mcp") {
            emit_text(ui, &text_buf);
            text_buf.clear();
            tool_close = Some(("</run_mcp>".into(), "🔌 run_mcp".into()));
        } else if trimmed.starts_with("<read_file") {
            emit_text(ui, &text_buf);
            text_buf.clear();
            let path = extract_attr(trimmed, "path");
            ui.label(
                egui::RichText::new(format!("📖 read_file：{}", path))
                    .size(13.5)
                    .color(TEXT_SECONDARY),
            );
        } else {
            text_buf.push_str(line);
            text_buf.push('\n');
        }
    }

    // 尾端未閉合的區塊照樣輸出
    if let Some((_, title)) = tool_close.take() {
        emit_collapsible(ui, &title, &block_buf, (msg_idx, block_idx));
    } else if in_code {
        emit_collapsible(ui, "📄 程式碼", &block_buf, (msg_idx, block_idx));
    }
    emit_text(ui, &text_buf);
}

/// 工具執行結果渲染：短結果直出，長結果收合（標題=首行摘要）。
fn render_collapsible_tool_output(ui: &mut egui::Ui, content: &str, msg_idx: usize) {
    let lines = content.lines().count();
    if lines <= COLLAPSE_LINES_THRESHOLD {
        ui.add(
            egui::Label::new(
                egui::RichText::new(content)
                    .font(egui::FontId::monospace(FONT_MONO))
                    .color(TEXT_PRIMARY),
            )
            .wrap(),
        );
        return;
    }
    let summary = truncate_chars(content.lines().next().unwrap_or(""), COLLAPSE_SUMMARY_CHARS);
    egui::CollapsingHeader::new(
        egui::RichText::new(format!("{}…（{} 行）", summary, lines))
            .size(13.5)
            .color(TEXT_SECONDARY),
    )
    .id_salt(("tool_out", msg_idx))
    .default_open(false)
    .show(ui, |ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(content)
                    .font(egui::FontId::monospace(FONT_MONO))
                    .color(TEXT_PRIMARY),
            )
            .wrap(),
        );
    });
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([800.0, 500.0])
            .with_decorations(true)
            .with_title("Agnes AI v0.5.0 — Multi Agent Security Engine"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Agnes AI",
        options,
        Box::new(|cc| Ok(Box::new(AgnesApp::new(cc)))),
    );
}

#[cfg(test)]
mod ui_tests {
    use super::truncate_chars;

    #[test]
    fn truncate_chars_handles_cjk_boundaries() {
        // 舊版位元組切片 &s[..20] 在這裡會 panic（每個中文字 3 bytes）
        let s = "請幫我建立一個完整的應用程式專案企劃";
        let cut = truncate_chars(s, 20);
        assert_eq!(cut.chars().count(), 18); // 全長 18 字 < 20，完整保留
        let cut7 = truncate_chars(s, 7);
        assert_eq!(cut7, "請幫我建立一個");
    }

    #[test]
    fn truncate_chars_ascii_and_empty() {
        assert_eq!(truncate_chars("hello world", 5), "hello");
        assert_eq!(truncate_chars("", 10), "");
    }
}
