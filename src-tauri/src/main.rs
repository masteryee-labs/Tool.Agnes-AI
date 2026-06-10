//! Agnes AI v0.4.0 — Native Rust GUI (egui/wgpu, zero Chromium)
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

// ─── i18n ────────────────────────────────────────────────────────────────────

const TRANSLATIONS: &[(&str, (&str, &str))] = &[
    ("new_conversation",    ("＋ 新增對話",            "+ New Conversation")),
    ("conversation_history",("🕒 對話歷史",            "🕒 History")),
    ("projects",            ("📁 專案",                "📁 Projects")),
    ("settings",            ("⚙ 設定",                "⚙ Settings")),
    ("ask_placeholder",     ("問什麼都可以，@ 提及，/ 指令", "Ask anything, @ to mention, / for actions")),
    ("add_folder",          ("+ 新增資料夾",           "+ Add Folder")),
    ("agent_status",        ("🤖 22 代理人狀態",       "🤖 22-Agent Status")),
    ("global_warning",      ("🌐 全域模式已啟用 — 所有操作需逐項確認", "🌐 Global mode active — all operations require confirmation")),
    ("pending_approval",    ("⚠ 待確認",              "⚠ Pending")),
    ("work_mode_project",   ("📁 專案模式",            "📁 Project Mode")),
    ("work_mode_global",    ("🌍 全域模式",            "🌍 Global Mode")),
    ("general",             ("一般",                   "General")),
    ("permissions",         ("權限",                   "Permissions")),
    ("security",            ("安全",                   "Security")),
    ("save",                ("儲存",                   "Save")),
    ("terminal_shell",      ("整合終端 Shell",         "Terminal Shell")),
    ("auto_review",         ("自動審查",               "Auto Review")),
    ("full_access",         ("完整存取",               "Full Access")),
    ("token_budget",        ("Token 預算",             "Token Budget")),
    ("welcome_title",       ("歡迎使用 Agnes AI",      "Welcome to Agnes AI")),
    ("welcome_body",        ("在下方輸入指令開始工作。支援多資料夾專案、22 道安全驗證、全域確認模式。",
                             "Type a command below to start. Supports multi-folder projects, 22-gate validation, and global confirmation mode.")),
    ("selected_folders",    ("{} 個資料夾已選取",       "{} folder(s) selected")),
    ("language",            ("語言",                   "Language")),
];

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
    conversations:         Vec<(String, String, String)>,
    active_conversation_id: Option<String>,
    // Settings / i18n / mode
    language:     String,   // "zh" | "en"
    settings_open: bool,
    settings_tab:  usize,
    work_mode:     String,  // "project" | "global"
    // Agent panel
    audit_results: Vec<AuditResult>,
    status_message: String,
    // Sidebar tab: 0 = New Chat, 1 = History
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
            settings_tab:  0,
            work_mode:     "project".into(),
            audit_results: Vec::new(),
            status_message: String::new(),
            sidebar_tab:   0,
        }
    }
}

// ─── Main App ────────────────────────────────────────────────────────────────

struct AgnesApp {
    app_state: Arc<AppState>,
    ui_state:  Arc<Mutex<UiState>>,
}

impl AgnesApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
        }

        Self { app_state, ui_state }
    }

    /// 側欄：對話歷史列表（點擊載入、🗑 刪除）。
    fn render_history_list(&self, ui: &mut egui::Ui, st: &mut UiState, lang: &str) {
        ui.label(
            egui::RichText::new(t("conversation_history", lang))
                .size(11.0).color(TEXT_SECONDARY).strong(),
        );
        ui.add_space(4.0);

        let convs = st.conversations.clone();
        let mut to_delete: Option<String> = None;

        egui::ScrollArea::vertical().id_salt("history_scroll").show(ui, |ui| {
            for (cid, title, created) in &convs {
                ui.horizontal(|ui| {
                    let selected = st.active_conversation_id.as_deref() == Some(cid.as_str());
                    let label = if title.trim().is_empty() { "(無標題)" } else { title.as_str() };
                    if ui.selectable_label(selected, label).on_hover_text(created).clicked() {
                        if let Ok(conn) = Connection::open(&self.app_state.db_path) {
                            if let Ok(msgs) = app_lib::get_messages_for_conversation(&conn, cid) {
                                st.active_messages = msgs.into_iter()
                                    .map(|m| ChatMessage { role: m.role, content: m.content })
                                    .collect();
                                st.active_conversation_id = Some(cid.clone());
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("🗑").clicked() {
                            to_delete = Some(cid.clone());
                        }
                    });
                });
            }
            if convs.is_empty() {
                ui.label(egui::RichText::new("—").size(10.0).color(TEXT_MUTED));
            }
        });

        if let Some(cid) = to_delete {
            if let Ok(conn) = Connection::open(&self.app_state.db_path) {
                let _ = app_lib::delete_conversation(&conn, &cid);
                if st.active_conversation_id.as_deref() == Some(cid.as_str()) {
                    st.active_conversation_id = None;
                    st.active_messages.clear();
                }
                if let Ok(convs) = app_lib::get_conversations(&conn) {
                    st.conversations = convs;
                }
            }
        }
    }

    /// 側欄：專案與多資料夾選取列表。
    fn render_projects_list(&self, ui: &mut egui::Ui, st: &mut UiState, lang: &str) {
        ui.label(egui::RichText::new(t("projects", lang)).size(11.0).color(TEXT_SECONDARY).strong());
        ui.add_space(4.0);

        let selected_idx = st.selected_project_idx;
        let projects_cloned = st.projects.clone();

        for (idx, p) in projects_cloned.iter().enumerate() {
            let is_selected = selected_idx == Some(idx);
            if ui.selectable_label(is_selected, &p.name).clicked() {
                st.selected_project_idx = Some(idx);
                st.selected_paths.clear();
                for path in &p.paths {
                    st.selected_paths.insert(path.clone());
                }
            }

            if is_selected {
                ui.indent("folder_indent", |ui| {
                    for path in &p.paths {
                        let mut checked = st.selected_paths.contains(path);
                        let short = std::path::Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.clone());
                        if ui.checkbox(&mut checked, &short)
                            .on_hover_text(path)
                            .changed()
                        {
                            if checked {
                                st.selected_paths.insert(path.clone());
                            } else {
                                st.selected_paths.remove(path);
                            }
                        }
                    }

                    if ui.button(t("add_folder", lang)).clicked() {
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
            }
        }

        let folder_count = st.selected_paths.len();
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(t_fmt("selected_folders", lang, folder_count))
                .size(10.0).color(TEXT_MUTED),
        );
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
                    let conn = Connection::open(&self.app_state.db_path).unwrap();
                    let new_id = app_lib::create_conversation(
                        &conn,
                        &prompt[..prompt.len().min(20)],
                    ).unwrap_or_default();
                    st.active_conversation_id = Some(new_id.clone());
                    new_id
                }
            };

            st.active_messages.push(ChatMessage { role: "user".into(), content: prompt.clone() });

            {
                let conn = Connection::open(&self.app_state.db_path).unwrap();
                let _ = app_lib::add_conversation_message(&conn, &conv_id, "user", &prompt);
            }

            let workspace = st.selected_paths.iter().next().cloned()
                .or_else(|| {
                    st.selected_project_idx
                        .and_then(|i| st.projects.get(i))
                        .and_then(|p| p.paths.first().cloned())
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

            match agent_loop.run_step(
                &mut messages,
                &app_state_task.mcp_manager,
                &app_state_task.token_budgeter,
                &app_state_task.db_path,
            ).await {
                Ok(step) => {
                    if let Ok(conn) = Connection::open(&app_state_task.db_path) {
                        let _ = app_lib::add_conversation_message(
                            &conn, &conversation_id, "assistant", &step.response_text,
                        );
                        let mut st = ui_state.lock().unwrap();
                        st.active_messages.push(ChatMessage {
                            role:    "assistant".into(),
                            content: step.response_text.clone(),
                        });
                        st.audit_results = step.audits;
                    }
                    if step.requires_approval {
                        let mut pending = app_state_task.pending_state.lock().await;
                        *pending = Some(PendingState {
                            pending_tools:    step.executed_tools,
                            pending_response: step.response_text,
                        });
                    }
                }
                Err(e) => {
                    ui_state.lock().unwrap().status_message = format!("Error: {}", e);
                }
            }

            {
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
        ctx.set_pixels_per_point(1.0);
        let mut style = (*ctx.style()).clone();
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
                            .size(15.0).color(ACCENT_BLUE).strong(),
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
                            .size(11.0).color(TEXT_SECONDARY),
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
                            .size(10.0).color(TEXT_MUTED),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        for item in &["Window", "View", "File"] {
                            let _ = ui.button(egui::RichText::new(*item).size(12.0));
                        }
                    });
                });
            });

        // ── Left sidebar ──────────────────────────────────────────────────────
        egui::SidePanel::left("sidebar")
            .default_width(220.0)
            .min_width(180.0)
            .max_width(320.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let tabs = [("💬", "new_conversation"), ("🕒", "conversation_history")];
                    let cur_tab = self.ui_state.lock().unwrap().sidebar_tab;
                    for (i, (icon, key)) in tabs.iter().enumerate() {
                        let label = format!("{} {}", icon, t(key, &lang));
                        if ui.selectable_label(cur_tab == i, label).clicked() {
                            let mut st = self.ui_state.lock().unwrap();
                            st.sidebar_tab = i;
                            if i == 0 {
                                st.active_conversation_id = None;
                                st.active_messages.clear();
                            }
                        }
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                let mut st = self.ui_state.lock().unwrap();

                if st.sidebar_tab == 1 {
                    self.render_history_list(ui, &mut st, &lang);
                } else {
                    self.render_projects_list(ui, &mut st, &lang);
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);

                if ui.button(format!("⚙ {}", t("settings", &lang))).clicked() {
                    st.settings_open = true;
                }

                ui.add_space(6.0);
                let mode_text  = if st.work_mode == "global" { t("work_mode_global", &lang) } else { t("work_mode_project", &lang) };
                let mode_color = if st.work_mode == "global" { ACCENT_ORANGE } else { ACCENT_BLUE };
                if ui.add(
                    egui::Button::new(egui::RichText::new(&mode_text).color(mode_color).size(13.0))
                ).on_hover_text(
                    if st.work_mode == "global" { t("global_warning", &lang) } else { "Switch to Global (Hermes) mode".into() }
                ).clicked() {
                    st.work_mode = if st.work_mode == "global" { "project".into() } else { "global".into() };
                    let mut cfg = self.app_state.config.lock().unwrap().clone();
                    cfg.general.project_mode = st.work_mode.clone();
                    let _ = cfg.save();
                    *self.app_state.config.lock().unwrap() = cfg;
                }
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

                ui.label(egui::RichText::new(t("agent_status", &lang)).size(13.0).color(TEXT_PRIMARY).strong());
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
                        ui.label(egui::RichText::new(*g_name).color(ACCENT_BLUE).strong().size(11.0));
                        for a in *agents {
                            let (status_color, status_text) = match audits.iter().find(|x| x.agent_name == *a) {
                                Some(r) if r.verdict == "PASSED"   => (ACCENT_GREEN,  "✓"),
                                Some(r) if r.verdict == "REJECTED" => (ACCENT_RED,    "✗"),
                                Some(r) if r.verdict == "SKIPPED"  => (TEXT_SECONDARY, "~"),
                                _ => (TEXT_MUTED, "·"),
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("  {}", a)).size(10.0).color(TEXT_PRIMARY));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(status_text).color(status_color).size(11.0));
                                });
                            });
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
                            .size(12.0).color(ACCENT_ORANGE).strong(),
                    );
                    for tool in &pending.pending_tools {
                        egui::Frame::default()
                            .fill(BG_CARD)
                            .corner_radius(4)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(&tool.name).size(11.0).strong());
                                if let Some(ref path) = tool.path {
                                    ui.label(egui::RichText::new(path).size(10.0).color(TEXT_SECONDARY));
                                }
                            });
                    }
                    ui.horizontal(|ui| {
                        if ui.button("✓ Approve").clicked() {
                            let app_state = self.app_state.clone();
                            let ctx2 = ctx.clone();
                            self.app_state.engine_runtime.spawn(async move {
                                let mut lock = app_state.pending_state.lock().await;
                                if let Some(ref p) = *lock {
                                    let lp = AgentLoop::new(
                                        app_state.config.lock().unwrap().clone(),
                                        String::new(),
                                    );
                                    for tool in &p.pending_tools {
                                        let _ = lp.execute_tool(tool, &app_state.mcp_manager).await;
                                    }
                                }
                                *lock = None;
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
                ui.label(egui::RichText::new(&st.status_message).size(11.0).color(ACCENT_RED));
                ui.add_space(4.0);
            }

            // Message area
            let chat_is_empty = st.active_messages.is_empty();
            let avail_height  = ui.available_height() - 50.0;

            egui::ScrollArea::vertical()
                .max_height(avail_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap); // Enable word wrapping globally in scroll area
                    if chat_is_empty {
                        // Welcome / empty state
                        ui.add_space(avail_height * 0.25);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(t("welcome_title", &lang))
                                    .size(24.0).color(TEXT_PRIMARY).strong(),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(t("welcome_body", &lang))
                                    .size(15.0).color(TEXT_SECONDARY),
                            );
                        });
                    } else {
                        for msg in &st.active_messages {
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
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(&msg.content)
                                                    .font(egui::FontId::monospace(15.5))
                                            )
                                            .wrap()
                                        );
                                    } else {
                                        render_message_content(ui, &msg.content);
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
                                    ui.label(egui::RichText::new("🤖 Agnes AI").color(ACCENT_BLUE).strong().size(16.0));
                                    ui.add(egui::Spinner::new());
                                    ui.label(egui::RichText::new(if lang == "zh" { "正在執行任務中，請稍候..." } else { "Executing task, please wait..." })
                                        .color(TEXT_SECONDARY).size(16.0));
                                });
                            });
                    }
                });

            // Input box pinned to bottom - Codex / Antigravity 2.0 style
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                egui::Frame::default()
                    .fill(BG_SECONDARY)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .corner_radius(8)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        // 1. Multiline Text Area
                        let text_edit = egui::TextEdit::multiline(&mut st.chat_input)
                            .desired_width(f32::INFINITY)
                            .desired_rows(2)
                            .hint_text(t("ask_placeholder", &lang))
                            .interactive(!is_running) // Disable text area input if running
                            .frame(false); // Clean frame-less look inside the card
                        
                        let response = ui.add(text_edit);
                        
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        
                        // 2. Toolbar below text area
                        ui.horizontal(|ui| {
                            // Left options menu
                            ui.menu_button(egui::RichText::new("＋").size(17.0).strong(), |ui| {
                                if ui.button(if lang == "zh" { "🗑 清除當前對話" } else { "🗑 Clear Chat" }).clicked() {
                                    st.active_messages.clear();
                                    st.active_conversation_id = None;
                                    ui.close_menu();
                                    ui.ctx().request_repaint();
                                }
                                if ui.button(if lang == "zh" { "📂 新增專案資料夾..." } else { "📂 Add Project Folder..." }).clicked() {
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
                            
                            // Permissions / Security Level indicator
                            let perm_text = if st.work_mode == "global" { "🌍 全域模式" } else { "📁 專案模式" };
                            let perm_color = if st.work_mode == "global" { ACCENT_ORANGE } else { ACCENT_BLUE };
                            if ui.add_enabled(!is_running, egui::Button::new(egui::RichText::new(perm_text).color(perm_color).size(15.0))).clicked() {
                                st.work_mode = if st.work_mode == "global" { "project".into() } else { "global".into() };
                                let mut cfg = self.app_state.config.lock().unwrap().clone();
                                cfg.general.project_mode = st.work_mode.clone();
                                let _ = cfg.save();
                                *self.app_state.config.lock().unwrap() = cfg;
                            }
                            
                            // Model selection
                            ui.label(egui::RichText::new(" agnes-2.0-flash ").color(TEXT_SECONDARY).size(15.0));
                            
                            // Right aligned items: send button / Abort button
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if is_running {
                                    // Render red Abort button
                                    let abort_btn = egui::Button::new(
                                        egui::RichText::new("■ 中止").color(TEXT_PRIMARY).strong().size(15.0)
                                    )
                                    .fill(ACCENT_RED)
                                    .corner_radius(12)
                                    .min_size(egui::Vec2::new(64.0, 32.0));
                                    
                                    if ui.add(abort_btn).clicked() {
                                        if let Ok(mut s) = self.app_state.agent_state.try_lock() {
                                            *s = AgentExecutionState::Idle;
                                        }
                                        ui.ctx().request_repaint();
                                    }
                                } else {
                                    // Render standard send button
                                    let send_btn = egui::Button::new(
                                        egui::RichText::new("▲").color(TEXT_PRIMARY).strong().size(15.0)
                                    )
                                    .fill(ACCENT_BLUE)
                                    .corner_radius(12)
                                    .min_size(egui::Vec2::new(32.0, 32.0));
                                    
                                    let clicked = ui.add(send_btn).clicked();
                                    
                                    // Send trigger logic: Enter without shift when typing in the text edit or button click
                                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let shift_pressed = ui.input(|i| i.modifiers.shift);
                                    let send_triggered = clicked || (enter_pressed && !shift_pressed && response.has_focus());
                                    
                                    if send_triggered {
                                        if st.chat_input.ends_with('\n') {
                                            st.chat_input.pop();
                                        }
                                        let trimmed = st.chat_input.trim().to_string();
                                        if !trimmed.is_empty() {
                                            st.chat_input = trimmed;
                                            drop(st);
                                            self.handle_send(ctx);
                                        }
                                    }
                                }
                            });
                        });
                    });
            });
        });

        // ── Settings Modal ────────────────────────────────────────────────────
        let settings_open = self.ui_state.lock().unwrap().settings_open;
        if settings_open {
            let mut open_flag = true;
            let mut settings_changed = false;

            egui::Window::new(t("settings", &lang))
                .default_size([520.0, 420.0])
                .resizable(true)
                .open(&mut open_flag)
                .show(ctx, |ui| {
                    let mut st = self.ui_state.lock().unwrap();

                    // Tabs
                    ui.horizontal(|ui| {
                        for (i, key) in ["general", "permissions", "security"].iter().enumerate() {
                            if ui.selectable_label(st.settings_tab == i, t(key, &lang)).clicked() {
                                st.settings_tab = i;
                            }
                        }
                    });
                    ui.separator();
                    ui.add_space(8.0);

                    match st.settings_tab {
                        0 => {
                            // General: language, work mode
                            ui.strong(t("language", &lang));
                            ui.horizontal(|ui| {
                                if ui.selectable_label(st.language == "zh", "繁體中文 (zh-TW)").clicked() {
                                    st.language = "zh".into();
                                    settings_changed = true;
                                }
                                if ui.selectable_label(st.language == "en", "English (en-US)").clicked() {
                                    st.language = "en".into();
                                    settings_changed = true;
                                }
                            });
                            ui.add_space(10.0);
                            ui.strong(if lang == "zh" { "工作模式" } else { "Work Mode" });
                            ui.horizontal(|ui| {
                                if ui.selectable_label(st.work_mode == "project", t("work_mode_project", &lang)).clicked() {
                                    st.work_mode = "project".into();
                                    settings_changed = true;
                                }
                                if ui.selectable_label(st.work_mode == "global", t("work_mode_global", &lang)).clicked() {
                                    st.work_mode = "global".into();
                                    settings_changed = true;
                                }
                            });
                        }
                        1 => {
                            // Permissions: shell selection
                            let mut cfg = self.app_state.config.lock().unwrap().clone();
                            ui.label(t("terminal_shell", &lang));
                            ui.horizontal(|ui| {
                                for shell in &["PowerShell", "cmd", "sh"] {
                                    if ui.selectable_label(cfg.general.shell == *shell, *shell).clicked() {
                                        cfg.general.shell = shell.to_string();
                                        let _ = cfg.save();
                                        *self.app_state.config.lock().unwrap() = cfg.clone();
                                    }
                                }
                            });
                        }
                        2 => {
                            // Security
                            let mut cfg = self.app_state.config.lock().unwrap().clone();
                            let mut changed = false;
                            if ui.checkbox(&mut cfg.security.require_approval, t("auto_review", &lang)).changed() {
                                changed = true;
                            }
                            if ui.checkbox(&mut cfg.security.full_access, t("full_access", &lang)).changed() {
                                changed = true;
                            }
                            if changed {
                                let _ = cfg.save();
                                *self.app_state.config.lock().unwrap() = cfg;
                            }
                        }
                        _ => {}
                    }

                    if settings_changed {
                        let mut cfg = self.app_state.config.lock().unwrap().clone();
                        cfg.general.language     = if st.language == "zh" { "zh-TW".into() } else { "en-US".into() };
                        cfg.general.project_mode = st.work_mode.clone();
                        let _ = cfg.save();
                        *self.app_state.config.lock().unwrap() = cfg;
                    }
                });

            if !open_flag {
                self.ui_state.lock().unwrap().settings_open = false;
            }
        }
    }
}

/// Render assistant message text: ``` fenced blocks as monospace cards, rest as wrapped labels.
fn render_message_content(ui: &mut egui::Ui, content: &str) {
    let mut in_code = false;
    let mut buffer = String::new();

    let flush = |ui: &mut egui::Ui, text: &str, code: bool| {
        if text.trim().is_empty() {
            return;
        }
        if code {
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(20, 24, 33))
                .corner_radius(4)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(text.trim_end())
                                .font(egui::FontId::monospace(13.5))
                                .color(TEXT_PRIMARY),
                        )
                        .wrap(),
                    );
                });
        } else {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text.trim_end()).size(15.0).color(TEXT_PRIMARY),
                )
                .wrap(),
            );
        }
    };

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            flush(ui, &buffer, in_code);
            buffer.clear();
            in_code = !in_code;
            continue;
        }
        buffer.push_str(line);
        buffer.push('\n');
    }
    flush(ui, &buffer, in_code);
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([800.0, 500.0])
            .with_decorations(true)
            .with_title("Agnes AI v0.4.0 — Multi Agent Security Engine"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Agnes AI",
        options,
        Box::new(|cc| Ok(Box::new(AgnesApp::new(cc)))),
    );
}
