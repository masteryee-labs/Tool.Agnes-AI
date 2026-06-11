//! 右側面板：三 Tab 膠囊切換——「🤖 代理人」（22 代理分組 + 待審批）、
//! 「✎ 變更」（file_changes 清單 + diff 視圖）、「📄 檔案」（唯讀檔案檢視器）。

use eframe::egui;
use rusqlite::Connection;

use crate::ui_theme::*;
use crate::{t, t_fmt, truncate_chars, AgnesApp, ChatMessage, UiState};
use app_lib::AgentLoop;

/// diff 視圖的單列（行號預先算好，show_rows 虛擬化時不需回掃前綴）。
pub(crate) struct DiffRowView {
    pub no: String,
    /// 0 = Context, 1 = Added, 2 = Removed
    pub kind: u8,
    pub text: String,
}

/// 把 line_diff 輸出轉成帶行號的視圖列（Added/Context 用新行號，Removed 用舊行號）。
pub(crate) fn build_diff_rows(before: &str, after: &str, max_lines: usize) -> Vec<DiffRowView> {
    let (lines, _) = app_lib::line_diff(before, after, max_lines);
    let mut old_no = 0usize;
    let mut new_no = 0usize;
    lines
        .into_iter()
        .map(|l| match l.kind {
            app_lib::DiffLineKind::Context => {
                old_no += 1;
                new_no += 1;
                DiffRowView { no: format!("{:>4}", new_no), kind: 0, text: l.text }
            }
            app_lib::DiffLineKind::Added => {
                new_no += 1;
                DiffRowView { no: format!("{:>4}", new_no), kind: 1, text: l.text }
            }
            app_lib::DiffLineKind::Removed => {
                old_no += 1;
                DiffRowView { no: format!("{:>4}", old_no), kind: 2, text: l.text }
            }
        })
        .collect()
}

/// 選中一筆變更：算 diff 列、切到「變更」Tab。內容在點擊當下複製一次，不每幀重算。
pub(crate) fn select_change(st: &mut UiState, change_id: i64, diff_max_lines: usize) {
    let Some((before, after)) = st
        .file_changes
        .iter()
        .find(|c| c.id == change_id)
        .map(|c| (c.before_content.clone(), c.after_content.clone()))
    else {
        return;
    };
    st.diff_rows = build_diff_rows(&before, &after, diff_max_lines);
    st.selected_change_id = Some(change_id);
    st.diff_full_view = false;
    st.right_panel_tab = 1;
    st.right_panel_open = true;
}

/// 以唯讀檢視器開啟檔案：點擊當幀讀一次並快取（相對路徑以目前工作區解析）。
pub(crate) fn open_file_in_viewer(st: &mut UiState, path: &str, max_bytes: usize, lang: &str) {
    let resolved = resolve_workspace_path(st, path);
    st.file_viewer_path = Some(resolved.to_string_lossy().to_string());
    st.right_panel_tab = 2;
    st.right_panel_open = true;
    let content = match std::fs::metadata(&resolved) {
        Ok(meta) if meta.len() > max_bytes as u64 => {
            Err(t_fmt("file_too_large", lang, max_bytes / 1024))
        }
        Ok(_) => std::fs::read_to_string(&resolved).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    st.file_viewer_content = Some(content);
}

fn resolve_workspace_path(st: &UiState, path: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let base = st
        .selected_project_idx
        .and_then(|i| st.projects.get(i))
        .and_then(|proj| {
            proj.paths
                .iter()
                .find(|x| st.selected_paths.contains(*x))
                .cloned()
                .or_else(|| proj.paths.first().cloned())
        });
    match base {
        Some(b) => std::path::Path::new(&b).join(p),
        None => p.to_path_buf(),
    }
}

impl AgnesApp {
    /// 右側面板入口：膠囊 Tab 列 + 各 Tab 內容。
    pub(crate) fn render_right_panel(
        &self,
        ctx: &egui::Context,
        lang: &str,
        diff_max_lines: usize,
    ) {
        egui::SidePanel::right("agent_panel")
            .default_width(300.0)
            .min_width(240.0)
            .max_width(480.0)
            .frame(
                egui::Frame::default()
                    .fill(BG_SIDEBAR)
                    .inner_margin(SPACING_SM)
                    .stroke(egui::Stroke::new(1.0, BORDER)),
            )
            .show(ctx, |ui| {
                let mut st = self.ui_state.lock().unwrap();

                // ── 膠囊 Tab 列 ──
                egui::Frame::default()
                    .fill(BG_CARD)
                    .corner_radius(RADIUS_BADGE)
                    .inner_margin(egui::Margin::same(3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = SPACING_XS;
                            let third = (ui.available_width() - SPACING_XS * 2.0) / 3.0;
                            let tabs = [
                                (0usize, "🤖", "tab_agents"),
                                (1usize, "✎", "tab_changes"),
                                (2usize, "📄", "tab_file"),
                            ];
                            for (idx, icon, key) in tabs {
                                let active = st.right_panel_tab == idx;
                                let (fill, color) = if active {
                                    (BG_HOVER, TEXT_PRIMARY)
                                } else {
                                    (egui::Color32::TRANSPARENT, TEXT_SECONDARY)
                                };
                                if ui
                                    .add_sized(
                                        egui::vec2(third, SEGMENT_HEIGHT),
                                        egui::Button::new(
                                            egui::RichText::new(format!("{} {}", icon, t(key, lang)))
                                                .size(FONT_SMALL)
                                                .color(color),
                                        )
                                        .fill(fill)
                                        .corner_radius(RADIUS_BADGE),
                                    )
                                    .clicked()
                                {
                                    st.right_panel_tab = idx;
                                }
                            }
                        });
                    });
                ui.add_space(SPACING_SM);

                match st.right_panel_tab {
                    1 => self.render_changes_tab(ui, &mut st, lang, diff_max_lines),
                    2 => render_file_tab(ui, &mut st, lang),
                    _ => self.render_agents_tab(ui, &mut st, ctx, lang),
                }
            });
    }

    /// 代理人 Tab：22 代理 G1-G6 分組視圖 + 待審批 Confirmation 區（功能沿用）。
    fn render_agents_tab(
        &self,
        ui: &mut egui::Ui,
        st: &mut UiState,
        ctx: &egui::Context,
        lang: &str,
    ) {
        ui.label(
            egui::RichText::new(format!("🤖 {}", t("agent_status", lang)))
                .size(FONT_LABEL)
                .color(TEXT_PRIMARY)
                .strong(),
        );

        // 範疇副標：面板狀態跟著目前 Session/專案走
        let (scope_text, scope_color) = if st.work_mode == "global" {
            (t("panel_scope_global", lang), ACCENT_ORANGE)
        } else {
            let project = st
                .selected_project_idx
                .and_then(|i| st.projects.get(i))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let session = st
                .active_conversation_id
                .as_deref()
                .and_then(|cid| st.conversations.iter().find(|c| c.id == cid))
                .map(|c| truncate_chars(&c.title, 10));
            match session {
                Some(s) => (format!("📂 {} / 💬 {}", project, s), ACCENT_BLUE),
                None => (format!("📂 {}", project), TEXT_SECONDARY),
            }
        };
        ui.label(egui::RichText::new(scope_text).size(FONT_CAPTION).color(scope_color));
        let has_audits = !st.audit_results.is_empty();
        ui.label(
            egui::RichText::new(if has_audits {
                t("legend_agents", lang)
            } else {
                t("panel_scope_idle", lang)
            })
            .size(FONT_CAPTION)
            .color(TEXT_MUTED),
        );
        ui.add_space(SPACING_XS);

        let groups: &[(&str, &[&str])] = &[
            ("G1 記憶蒸餾", &["ContextDistillerAlpha", "ContextDistillerBeta", "DistillationIntegrator", "FactHallucinationAuditor", "TokenOverlapAuditor"]),
            ("G2 工作流",   &["WorkflowTopology", "WorkflowRuntimeEvaluator", "SlopVibeAuditor", "SlopPathPurgeSpecialist"]),
            ("G3 指揮",     &["OrchestratorAgent", "LocaleCalibrationSpecialist", "LeadSystemArchitect"]),
            ("G4 效能",     &["PerformanceArchitectureEngineer", "ResourceAnalyticsEngineer", "MemoryEfficiencyReviewer"]),
            ("G5 安全",     &["SecurityArchitectureDesigner", "DefensiveCodingSpecialist", "SecurityComplianceAuditor"]),
            ("G6 工程",     &["CoreEngineCoder", "IntegrationEngineer", "MultimodalMediaSpecialist", "SandboxRuntimeTester"]),
        ];

        let audits = st.audit_results.clone();

        egui::ScrollArea::vertical().id_salt("agent_scroll").show(ui, |ui| {
            for (g_name, agents) in groups {
                ui.add_space(SPACING_XS / 2.0);
                ui.label(
                    egui::RichText::new(*g_name)
                        .color(ACCENT_ORANGE)
                        .strong()
                        .size(FONT_SMALL),
                );
                for a in *agents {
                    let audit = audits.iter().find(|x| x.agent_name == *a);
                    let (status_color, status_text, name_color) = match audit {
                        Some(r) if r.verdict == "PASSED" => (ACCENT_GREEN, "✓", TEXT_PRIMARY),
                        Some(r) if r.verdict == "REJECTED" => (ACCENT_RED, "✗", ACCENT_RED),
                        Some(r) if r.verdict == "SKIPPED" => (TEXT_SECONDARY, "~", TEXT_SECONDARY),
                        // DORMANT：按路由休眠（未激活 = 零成本），名稱同步轉灰
                        Some(_) => (TEXT_MUTED, "·", TEXT_MUTED),
                        None => (TEXT_MUTED, "·", TEXT_PRIMARY),
                    };
                    let row = ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("  {}", a))
                                .size(FONT_CAPTION)
                                .color(name_color),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(status_text)
                                    .color(status_color)
                                    .size(FONT_SMALL),
                            );
                        });
                    });
                    // hover 顯示該代理本輪裁決原因（07_UI_SPEC：點代理看 gate 結果）
                    if let Some(r) = audit {
                        row.response.on_hover_text(&r.reason);
                    }
                }
                ui.add_space(SPACING_XS / 2.0);
            }
        });

        // ConfirmationGate (try_lock = synchronous, safe in render loop)
        let pending_state = self.app_state.pending_state.try_lock().ok().and_then(|g| g.clone());

        if let Some(pending) = pending_state {
            ui.separator();
            ui.label(
                egui::RichText::new(t("pending_approval", lang))
                    .size(FONT_LABEL)
                    .color(ACCENT_ORANGE)
                    .strong(),
            );
            for tool in &pending.pending_tools {
                egui::Frame::default()
                    .fill(BG_CARD)
                    .corner_radius(RADIUS_BUTTON)
                    .inner_margin(SPACING_XS + 2.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&tool.name).size(FONT_SMALL).strong());
                        if let Some(ref path) = tool.path {
                            ui.label(
                                egui::RichText::new(path)
                                    .size(FONT_CAPTION)
                                    .color(TEXT_SECONDARY),
                            );
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
                            let mut lp = AgentLoop::new(
                                app_state.config.lock().unwrap().clone(),
                                p.workspace_path.clone(),
                            );
                            // 審批後補執行的寫檔也要記入 file_changes
                            lp.set_conversation_id(&p.conversation_id);
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
                            // 審批執行可能寫檔——同步刷新變更清單
                            let changes =
                                app_lib::get_file_changes(&app_state.db_path, &p.conversation_id)
                                    .unwrap_or_default();
                            let mut st = ui_state.lock().unwrap();
                            for r in results {
                                st.active_messages.push(ChatMessage {
                                    role: "tool".into(),
                                    content: r,
                                });
                            }
                            st.file_changes = changes;
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
    }

    /// 變更 Tab：目前 session 的 file_changes 清單；選中後下方顯示 diff/全文視圖。
    fn render_changes_tab(
        &self,
        ui: &mut egui::Ui,
        st: &mut UiState,
        lang: &str,
        diff_max_lines: usize,
    ) {
        if st.file_changes.is_empty() {
            ui.label(egui::RichText::new(t("no_changes", lang)).size(FONT_SMALL).color(TEXT_MUTED));
            return;
        }

        let mut pending_select: Option<i64> = None;
        {
            let UiState { file_changes, diff_stats_cache, selected_change_id, .. } = &mut *st;
            egui::ScrollArea::vertical()
                .id_salt("changes_list")
                .max_height(PANEL_LIST_MAX_HEIGHT)
                .show(ui, |ui| {
                    for rec in file_changes.iter() {
                        let (added, removed) = *diff_stats_cache.entry(rec.id).or_insert_with(|| {
                            let (_, stats) = app_lib::line_diff(
                                &rec.before_content,
                                &rec.after_content,
                                diff_max_lines,
                            );
                            (stats.added, stats.removed)
                        });
                        let name = std::path::Path::new(&rec.file_path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| rec.file_path.clone());
                        let selected = *selected_change_id == Some(rec.id);
                        let fill = if selected { BG_HOVER } else { egui::Color32::TRANSPARENT };
                        let frame_resp = egui::Frame::default()
                            .fill(fill)
                            .corner_radius(RADIUS_BUTTON)
                            .inner_margin(egui::Margin::symmetric(6, 4))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("✎ {}", name))
                                            .size(FONT_SMALL)
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("−{}", removed))
                                                    .size(FONT_SMALL)
                                                    .color(ACCENT_RED),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!("+{}", added))
                                                    .size(FONT_SMALL)
                                                    .color(ACCENT_GREEN),
                                            );
                                        },
                                    );
                                });
                            });
                        let resp = ui.interact(
                            frame_resp.response.rect,
                            ui.id().with(("change_row", rec.id)),
                            egui::Sense::click(),
                        );
                        if resp.on_hover_text(&rec.file_path).clicked() {
                            pending_select = Some(rec.id);
                        }
                    }
                });
        }
        if let Some(id) = pending_select {
            select_change(st, id, diff_max_lines);
        }

        // ── diff 視圖 ──
        let Some(sel_id) = st.selected_change_id else { return };
        ui.add_space(SPACING_XS);
        ui.separator();

        let mut close = false;
        let mut toggle_full: Option<bool> = None;
        {
            let UiState { file_changes, diff_rows, diff_full_view, .. } = &mut *st;
            let Some(rec) = file_changes.iter().find(|c| c.id == sel_id) else { return };

            // 頂部路徑列 + diff/全文切換 + ✕ 關閉
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(truncate_chars(&rec.file_path, 36))
                        .size(FONT_CAPTION)
                        .color(TEXT_SECONDARY),
                )
                .on_hover_text(&rec.file_path);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").clicked() {
                        close = true;
                    }
                    let full = *diff_full_view;
                    if ui
                        .selectable_label(full, egui::RichText::new(t("view_full", lang)).size(FONT_CAPTION))
                        .clicked()
                    {
                        toggle_full = Some(true);
                    }
                    if ui
                        .selectable_label(!full, egui::RichText::new(t("view_diff", lang)).size(FONT_CAPTION))
                        .clicked()
                    {
                        toggle_full = Some(false);
                    }
                });
            });
            ui.add_space(SPACING_XS);

            let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
            if *diff_full_view {
                let lines: Vec<&str> = rec.after_content.lines().collect();
                egui::ScrollArea::vertical().id_salt("change_full_scroll").show_rows(
                    ui,
                    row_h,
                    lines.len(),
                    |ui, range| {
                        for idx in range {
                            ui.label(
                                egui::RichText::new(format!("{:>4} {}", idx + 1, lines[idx]))
                                    .font(egui::FontId::monospace(FONT_MONO))
                                    .color(TEXT_PRIMARY),
                            );
                        }
                    },
                );
            } else {
                egui::ScrollArea::vertical().id_salt("change_diff_scroll").show_rows(
                    ui,
                    row_h,
                    diff_rows.len(),
                    |ui, range| {
                        for row in &diff_rows[range] {
                            let (bg, prefix) = match row.kind {
                                1 => (DIFF_ADDED_BG, "+"),
                                2 => (DIFF_REMOVED_BG, "-"),
                                _ => (egui::Color32::TRANSPARENT, " "),
                            };
                            if bg != egui::Color32::TRANSPARENT {
                                let rect = egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    egui::vec2(ui.available_width(), row_h),
                                );
                                ui.painter().rect_filled(rect, 0.0, bg);
                            }
                            ui.label(
                                egui::RichText::new(format!("{} {}{}", row.no, prefix, row.text))
                                    .font(egui::FontId::monospace(FONT_MONO))
                                    .color(TEXT_PRIMARY),
                            );
                        }
                    },
                );
            }
        }
        if let Some(full) = toggle_full {
            st.diff_full_view = full;
        }
        if close {
            st.selected_change_id = None;
            st.diff_rows.clear();
        }
    }
}

/// 檔案 Tab：唯讀檢視器（mono + 行號 + show_rows 虛擬化）。
fn render_file_tab(ui: &mut egui::Ui, st: &mut UiState, lang: &str) {
    let mut close = false;
    {
        let UiState { file_viewer_path, file_viewer_content, .. } = &mut *st;
        let Some(path) = file_viewer_path.as_deref() else {
            ui.label(
                egui::RichText::new(t("file_tab_hint", lang)).size(FONT_SMALL).color(TEXT_MUTED),
            );
            return;
        };

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(truncate_chars(path, 36))
                    .size(FONT_CAPTION)
                    .color(TEXT_SECONDARY),
            )
            .on_hover_text(path);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").clicked() {
                    close = true;
                }
            });
        });
        ui.add_space(SPACING_XS);

        match file_viewer_content {
            Some(Ok(text)) => {
                let lines: Vec<&str> = text.lines().collect();
                let row_h = ui.text_style_height(&egui::TextStyle::Monospace);
                egui::ScrollArea::vertical().id_salt("file_viewer_scroll").show_rows(
                    ui,
                    row_h,
                    lines.len(),
                    |ui, range| {
                        for idx in range {
                            ui.label(
                                egui::RichText::new(format!("{:>4} {}", idx + 1, lines[idx]))
                                    .font(egui::FontId::monospace(FONT_MONO))
                                    .color(TEXT_PRIMARY),
                            );
                        }
                    },
                );
            }
            Some(Err(e)) => {
                ui.label(egui::RichText::new(e.as_str()).size(FONT_SMALL).color(ACCENT_RED));
            }
            None => {
                ui.label(
                    egui::RichText::new(t("file_tab_hint", lang))
                        .size(FONT_SMALL)
                        .color(TEXT_MUTED),
                );
            }
        }
    }
    if close {
        st.file_viewer_path = None;
        st.file_viewer_content = None;
    }
}
