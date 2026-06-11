//! 中央訊息流渲染：user 卡片、assistant 純文字直排、活動卡片（工具呼叫收闔列）、
//! Think 區塊、檔案變更 chips。tool 訊息依序與 assistant 內的工具標籤配對，不再單獨洗版。

use eframe::egui;

use crate::ui_theme::*;
use crate::{t, t_fmt, truncate_chars};

/// 超過此行數的碼塊/工具輸出預設收合
pub(crate) const COLLAPSE_LINES_THRESHOLD: usize = 8;
/// 收合標題的摘要字元數
pub(crate) const COLLAPSE_SUMMARY_CHARS: usize = 60;
/// 工具標籤前的自由文字超過此行數時收闔為「思考過程」
const THINK_LINES_THRESHOLD: usize = 3;
/// 活動列標題的內容摘要字元數
const ACTIVITY_TITLE_CHARS: usize = 48;

/// 訊息流互動結果：呼叫端（持有 UiState 鎖）統一套用。
pub(crate) enum ChatAction {
    /// 點擊變更 chip → 開右面板「變更」Tab 並選中該筆
    OpenChange(i64),
    /// 點擊讀寫活動列的開啟檔案 → 右面板「檔案」Tab 唯讀檢視
    OpenFile(String),
}

#[derive(Clone, Copy, PartialEq)]
enum ToolKind {
    WriteFile,
    RunCommand,
    RunMcp,
    ReadFile,
}

enum Segment {
    Text(String),
    Think(String),
    Code(String),
    Tool { kind: ToolKind, param: String, body: String },
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

fn flush_text(segs: &mut Vec<Segment>, text: &mut String) {
    if !text.trim().is_empty() {
        segs.push(Segment::Text(std::mem::take(text)));
    } else {
        text.clear();
    }
}

/// 把 assistant content 切成段落序列：自由文字、<think>、``` 碼塊、工具標籤。
fn parse_segments(content: &str) -> Vec<Segment> {
    let mut segs: Vec<Segment> = Vec::new();
    let mut text = String::new();
    let mut block = String::new();
    let mut in_code = false;
    let mut in_think = false;
    let mut tool_close: Option<(String, ToolKind, String)> = None;

    for line in content.lines() {
        if let Some((close, _, _)) = &tool_close {
            if line.trim_start().starts_with(close.as_str()) {
                let (_, kind, param) = tool_close.take().unwrap();
                segs.push(Segment::Tool { kind, param, body: std::mem::take(&mut block) });
            } else {
                block.push_str(line);
                block.push('\n');
            }
            continue;
        }
        if in_code {
            if line.trim_start().starts_with("```") {
                segs.push(Segment::Code(std::mem::take(&mut block)));
                in_code = false;
            } else {
                block.push_str(line);
                block.push('\n');
            }
            continue;
        }
        if in_think {
            if line.trim_start().starts_with("</think>") {
                segs.push(Segment::Think(std::mem::take(&mut block)));
                in_think = false;
            } else {
                block.push_str(line);
                block.push('\n');
            }
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            flush_text(&mut segs, &mut text);
            in_code = true;
        } else if let Some(rest) = trimmed.strip_prefix("<think>") {
            flush_text(&mut segs, &mut text);
            if let Some(end) = rest.find("</think>") {
                segs.push(Segment::Think(rest[..end].to_string()));
            } else {
                if !rest.trim().is_empty() {
                    block.push_str(rest);
                    block.push('\n');
                }
                in_think = true;
            }
        } else if trimmed.starts_with("<write_file") {
            flush_text(&mut segs, &mut text);
            tool_close = Some((
                "</write_file>".into(),
                ToolKind::WriteFile,
                extract_attr(trimmed, "path"),
            ));
        } else if trimmed.starts_with("<run_command>") {
            flush_text(&mut segs, &mut text);
            tool_close = Some(("</run_command>".into(), ToolKind::RunCommand, String::new()));
        } else if trimmed.starts_with("<run_mcp") {
            flush_text(&mut segs, &mut text);
            tool_close = Some(("</run_mcp>".into(), ToolKind::RunMcp, String::new()));
        } else if trimmed.starts_with("<read_file") {
            flush_text(&mut segs, &mut text);
            segs.push(Segment::Tool {
                kind: ToolKind::ReadFile,
                param: extract_attr(trimmed, "path"),
                body: String::new(),
            });
        } else {
            text.push_str(line);
            text.push('\n');
        }
    }

    // 尾端未閉合的區塊照樣輸出
    if let Some((_, kind, param)) = tool_close.take() {
        segs.push(Segment::Tool { kind, param, body: block });
    } else if in_code {
        segs.push(Segment::Code(block));
    } else if in_think {
        segs.push(Segment::Think(block));
    }
    flush_text(&mut segs, &mut text);
    segs
}

// ─── 基礎輸出 ────────────────────────────────────────────────────────────────

/// user 訊息：整寬淡色卡（assistant 為純文字直排，視覺對比由此卡片建立）。
pub(crate) fn render_user_message(ui: &mut egui::Ui, content: &str) {
    egui::Frame::default()
        .fill(BG_TERTIARY)
        .corner_radius(RADIUS_CARD)
        .inner_margin(SPACING_CARD_INNER)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(content.trim_end()).size(FONT_BODY).color(TEXT_PRIMARY),
                )
                .wrap(),
            );
        });
}

pub(crate) fn emit_text(ui: &mut egui::Ui, text: &str) {
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

pub(crate) fn emit_mono_frame(ui: &mut egui::Ui, text: &str) {
    egui::Frame::default()
        .fill(BG_CODE)
        .corner_radius(RADIUS_BUTTON)
        .inner_margin(SPACING_SM)
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
pub(crate) fn emit_collapsible(
    ui: &mut egui::Ui,
    title: &str,
    body: &str,
    salt: (usize, usize),
    lang: &str,
) {
    let lines = body.lines().count();
    if lines <= COLLAPSE_LINES_THRESHOLD {
        if !title.is_empty() {
            ui.label(egui::RichText::new(title).size(FONT_SMALL).color(TEXT_SECONDARY));
        }
        emit_mono_frame(ui, body);
        return;
    }
    egui::CollapsingHeader::new(
        egui::RichText::new(format!("{} {}", title, t_fmt("lines_n", lang, lines)))
            .size(FONT_LABEL)
            .color(TEXT_SECONDARY),
    )
    .id_salt(("collapse_blk", salt))
    .default_open(false)
    .show(ui, |ui| emit_mono_frame(ui, body));
}

/// 工具執行結果渲染：短結果直出，長結果收合（標題=首行摘要）。
pub(crate) fn render_collapsible_tool_output(
    ui: &mut egui::Ui,
    content: &str,
    salt: usize,
    lang: &str,
) {
    let lines = content.lines().count();
    if lines <= COLLAPSE_LINES_THRESHOLD {
        emit_mono_frame(ui, content);
        return;
    }
    let summary = truncate_chars(content.lines().next().unwrap_or(""), COLLAPSE_SUMMARY_CHARS);
    egui::CollapsingHeader::new(
        egui::RichText::new(format!("{}… {}", summary, t_fmt("lines_n", lang, lines)))
            .size(FONT_LABEL)
            .color(TEXT_SECONDARY),
    )
    .id_salt(("tool_out", salt))
    .default_open(false)
    .show(ui, |ui| emit_mono_frame(ui, content));
}

// ─── Think 區塊 ──────────────────────────────────────────────────────────────

/// 思考段落：3 行以內直接顯示（弱色斜體），超過收闔為「✱ 思考過程（N 行）›」。
fn render_think(ui: &mut egui::Ui, text: &str, salt: (usize, usize), lang: &str) {
    let lines = text.lines().count();
    if lines <= THINK_LINES_THRESHOLD {
        ui.add(
            egui::Label::new(
                egui::RichText::new(text.trim_end())
                    .size(FONT_BODY)
                    .italics()
                    .color(TEXT_SECONDARY),
            )
            .wrap(),
        );
        return;
    }
    egui::CollapsingHeader::new(
        egui::RichText::new(format!("✱ {} ›", t_fmt("thinking_lines", lang, lines)))
            .size(FONT_LABEL)
            .italics()
            .color(TEXT_MUTED),
    )
    .id_salt(("think", salt))
    .default_open(false)
    .show(ui, |ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(text.trim_end())
                    .size(FONT_BODY)
                    .italics()
                    .color(TEXT_SECONDARY),
            )
            .wrap(),
        );
    });
}

// ─── 活動卡片 ────────────────────────────────────────────────────────────────

fn activity_header(kind: ToolKind, param: &str, body: &str, lang: &str) -> String {
    let (icon, label_key) = match kind {
        ToolKind::RunCommand => ("⌨", "act_run"),
        ToolKind::WriteFile => ("✎", "act_write"),
        ToolKind::ReadFile => ("📖", "act_read"),
        ToolKind::RunMcp => ("🔌", "act_mcp"),
    };
    let detail = match kind {
        ToolKind::WriteFile | ToolKind::ReadFile => truncate_chars(param, ACTIVITY_TITLE_CHARS),
        ToolKind::RunCommand | ToolKind::RunMcp => {
            truncate_chars(body.lines().next().unwrap_or("").trim(), ACTIVITY_TITLE_CHARS)
        }
    };
    if detail.is_empty() {
        format!("{} {} ›", icon, t(label_key, lang))
    } else {
        format!("{} {}: {} ›", icon, t(label_key, lang), detail)
    }
}

/// 單一活動列：收闔態單行弱色文字；展開為帶左邊框子卡（參數 mono + 配對結果）。
#[allow(clippy::too_many_arguments)]
fn render_activity_row(
    ui: &mut egui::Ui,
    kind: ToolKind,
    param: &str,
    body: &str,
    result: Option<&str>,
    salt: (usize, usize),
    lang: &str,
    actions: &mut Vec<ChatAction>,
) {
    let header = activity_header(kind, param, body, lang);
    egui::CollapsingHeader::new(
        egui::RichText::new(header).size(FONT_LABEL).color(TEXT_SECONDARY),
    )
    .id_salt(("activity", salt))
    .default_open(false)
    .show(ui, |ui| {
        let frame_resp = egui::Frame::default()
            .fill(BG_CODE)
            .corner_radius(RADIUS_BUTTON)
            .inner_margin(SPACING_CARD_INNER)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if !param.is_empty() {
                    ui.label(
                        egui::RichText::new(param)
                            .font(egui::FontId::monospace(FONT_MONO))
                            .color(TEXT_SECONDARY),
                    );
                }
                if !body.trim().is_empty() {
                    emit_collapsible(ui, "", body, (salt.0, salt.1 * 2 + 1), lang);
                }
                if matches!(kind, ToolKind::WriteFile | ToolKind::ReadFile)
                    && !param.is_empty()
                    && ui.small_button(format!("📄 {}", t("open_file", lang))).clicked()
                {
                    actions.push(ChatAction::OpenFile(param.to_string()));
                }
                if let Some(res) = result {
                    ui.add_space(SPACING_XS);
                    ui.label(
                        egui::RichText::new(t("result_label", lang))
                            .size(FONT_CAPTION)
                            .color(TEXT_MUTED),
                    );
                    render_collapsible_tool_output(ui, res, salt.0 * 1000 + salt.1, lang);
                }
            });
        // 左邊框（活動子卡的視覺錨點）
        let rect = frame_resp.response.rect;
        ui.painter().line_segment(
            [rect.left_top(), rect.left_bottom()],
            egui::Stroke::new(ACTIVITY_BORDER_WIDTH, BORDER),
        );
    });
}

// ─── assistant 訊息整體渲染 ──────────────────────────────────────────────────

/// assistant 訊息：純文字直排；工具標籤→活動列（連續多列聚成一組，預設收闔）；
/// 工具標籤前的長文字→Think 收闔；最後結論永遠直接顯示。
/// `results` 為其後依序配對的 "tool" 訊息內容。
pub(crate) fn render_assistant_message(
    ui: &mut egui::Ui,
    msg_idx: usize,
    content: &str,
    results: &[&str],
    lang: &str,
    actions: &mut Vec<ChatAction>,
) {
    let segs = parse_segments(content);
    let last_tool = segs.iter().rposition(|s| matches!(s, Segment::Tool { .. }));
    let mut result_idx = 0usize;
    let mut i = 0usize;

    while i < segs.len() {
        match &segs[i] {
            Segment::Text(txt) => {
                let before_tools = last_tool.map(|lt| i < lt).unwrap_or(false);
                if before_tools && txt.lines().count() > THINK_LINES_THRESHOLD {
                    render_think(ui, txt, (msg_idx, i), lang);
                } else {
                    emit_text(ui, txt);
                }
            }
            Segment::Think(txt) => render_think(ui, txt, (msg_idx, i), lang),
            Segment::Code(code) => {
                emit_collapsible(ui, &format!("📄 {}", t("code_block", lang)), code, (msg_idx, i), lang);
            }
            Segment::Tool { .. } => {
                // 連續工具段落聚成一組
                let start = i;
                let mut end = i;
                while end + 1 < segs.len() && matches!(segs[end + 1], Segment::Tool { .. }) {
                    end += 1;
                }
                let count = end - start + 1;
                if count == 1 {
                    if let Segment::Tool { kind, param, body } = &segs[start] {
                        let res = results.get(result_idx).copied();
                        result_idx += 1;
                        render_activity_row(ui, *kind, param, body, res, (msg_idx, start), lang, actions);
                    }
                } else {
                    let group_resp = egui::Frame::default()
                        .fill(BG_CARD)
                        .stroke(egui::Stroke::new(1.0, BORDER))
                        .corner_radius(RADIUS_CARD)
                        .inner_margin(SPACING_SM)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::CollapsingHeader::new(
                                egui::RichText::new(format!(
                                    "⚙ {} ›",
                                    t_fmt("actions_ran", lang, count),
                                ))
                                .size(FONT_LABEL)
                                .color(TEXT_SECONDARY),
                            )
                            .id_salt(("act_group", msg_idx, start))
                            .default_open(false)
                            .show(ui, |ui| {
                                for (off, seg) in segs[start..=end].iter().enumerate() {
                                    if let Segment::Tool { kind, param, body } = seg {
                                        let res = results.get(result_idx + off).copied();
                                        render_activity_row(
                                            ui, *kind, param, body, res,
                                            (msg_idx, start + off), lang, actions,
                                        );
                                    }
                                }
                            });
                        });
                    let _ = group_resp;
                    result_idx += count;
                }
                i = end;
            }
        }
        ui.add_space(SPACING_XS);
        i += 1;
    }

    // 未配對到工具標籤的多出結果（例如審批後補執行）照樣可見
    for (k, extra) in results.iter().enumerate().skip(result_idx) {
        ui.label(
            egui::RichText::new(format!("🛠 {}", t("tool_output", lang)))
                .size(FONT_CAPTION)
                .color(TEXT_MUTED),
        );
        render_collapsible_tool_output(ui, extra, msg_idx * 1000 + 500 + k, lang);
        ui.add_space(SPACING_XS);
    }
}

// ─── 檔案變更 chips ──────────────────────────────────────────────────────────

/// 對話尾端的變更 chips 列：「✎ 檔名 +a −r」，點擊→右面板「變更」Tab 選中該筆。
/// diff stats 以 change_id 快取，不每幀重算。
pub(crate) fn render_change_chips(
    ui: &mut egui::Ui,
    changes: &[app_lib::FileChangeRecord],
    stats_cache: &mut std::collections::HashMap<i64, (usize, usize)>,
    diff_max_lines: usize,
    actions: &mut Vec<ChatAction>,
) {
    if changes.is_empty() {
        return;
    }
    ui.add_space(SPACING_SM);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::new(SPACING_SM, SPACING_XS);
        for rec in changes {
            let (added, removed) = *stats_cache.entry(rec.id).or_insert_with(|| {
                let (_, stats) =
                    app_lib::line_diff(&rec.before_content, &rec.after_content, diff_max_lines);
                (stats.added, stats.removed)
            });
            let name = std::path::Path::new(&rec.file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| rec.file_path.clone());
            let frame_resp = egui::Frame::default()
                .fill(BG_CARD)
                .stroke(egui::Stroke::new(1.0, BORDER))
                .corner_radius(RADIUS_BADGE)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = SPACING_XS;
                        ui.label(
                            egui::RichText::new(format!("✎ {}", name))
                                .size(FONT_SMALL)
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            egui::RichText::new(format!("+{}", added))
                                .size(FONT_SMALL)
                                .color(ACCENT_GREEN),
                        );
                        ui.label(
                            egui::RichText::new(format!("−{}", removed))
                                .size(FONT_SMALL)
                                .color(ACCENT_RED),
                        );
                    });
                });
            let resp = ui.interact(
                frame_resp.response.rect,
                ui.id().with(("change_chip", rec.id)),
                egui::Sense::click(),
            );
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                actions.push(ChatAction::OpenChange(rec.id));
            }
        }
    });
}
