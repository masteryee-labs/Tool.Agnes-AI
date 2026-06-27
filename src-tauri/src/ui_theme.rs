//! 主題層：極簡黑+白暗色模式的色彩、圓角、間距與字級。
//! 所有 UI 模組一律引用此處具名常數；新數值禁止在 UI 代碼裸寫。
//!
//! 設計理念：對標 Claude Code / Codex / Devin / Antigravity 2.0 的暗色終端美學——
//! 純黑底 + 白字 + 灰階層次，無品牌色干擾，讓使用者聚焦於內容本身。

use eframe::egui;

// ─── 背景層次（純黑→深灰漸層）─────────────────────────────────────────────────
/// 視窗 / 中央內容區（最深黑）
pub const BG_PRIMARY: egui::Color32 = egui::Color32::from_rgb(18, 18, 18);
/// 側欄（左側欄 / 右面板 / 頂列）（比主背景暗一階，幾乎純黑）
pub const BG_SIDEBAR: egui::Color32 = egui::Color32::from_rgb(12, 12, 12);
/// 卡片（訊息卡 / 設定列 / 輸入卡）
pub const BG_CARD: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
/// 輸入欄位 / user 訊息卡（比卡片再亮一階）
pub const BG_TERTIARY: egui::Color32 = egui::Color32::from_rgb(36, 36, 36);
/// hover 高亮
pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(44, 44, 44);
/// 碼塊 / 工具輸出底（比卡片暗一階）
pub const BG_CODE: egui::Color32 = egui::Color32::from_rgb(22, 22, 22);
/// 邊框（低對比灰線）
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(48, 48, 48);

// ─── 文字（白→灰階）──────────────────────────────────────────────────────────
/// 主文字（近白）
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(235, 235, 235);
/// 次要文字（中灰）
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(165, 165, 165);
/// 弱化文字（深灰）
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(100, 100, 100);
/// 強調色背景上的文字（純黑——用於白色按鈕上）
pub const TEXT_ON_ACCENT: egui::Color32 = egui::Color32::from_rgb(18, 18, 18);

// ─── 強調色（極簡：白為主強調，灰階輔助）─────────────────────────────────────
/// 主強調色：純白（按鈕、活躍狀態、送出鍵）
pub const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(235, 235, 235);
/// 連結 / 選取（淺灰藍，低調）
pub const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(130, 170, 255);
/// 成功（柔和綠，僅用於狀態徽章）
pub const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(100, 200, 120);
/// 錯誤（柔和紅，僅用於錯誤狀態）
pub const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(220, 100, 100);
/// 警告（柔和黃，僅用於警告狀態）
pub const ACCENT_YELLOW: egui::Color32 = egui::Color32::from_rgb(220, 190, 90);

// ─── Diff 行底色 ─────────────────────────────────────────────────────────────
pub const DIFF_ADDED_BG: egui::Color32 = egui::Color32::from_rgb(26, 48, 32);
pub const DIFF_REMOVED_BG: egui::Color32 = egui::Color32::from_rgb(52, 28, 28);

// ─── 圓角 ────────────────────────────────────────────────────────────────────
pub const RADIUS_CARD: u8 = 10;
pub const RADIUS_BUTTON: u8 = 8;
pub const RADIUS_INPUT: u8 = 14;
/// 膠囊（u8 上限即全圓）
pub const RADIUS_BADGE: u8 = 255;

// ─── 間距 ────────────────────────────────────────────────────────────────────
/// 訊息與訊息之間
pub const SPACING_MESSAGE: f32 = 14.0;
/// 卡片內距
pub const SPACING_CARD_INNER: f32 = 12.0;
pub const SPACING_XS: f32 = 4.0;
pub const SPACING_SM: f32 = 8.0;

// ─── 尺寸 ────────────────────────────────────────────────────────────────────
pub const TOPBAR_HEIGHT: f32 = 36.0;
pub const SIDEBAR_BTN_HEIGHT: f32 = 34.0;
pub const SEGMENT_HEIGHT: f32 = 28.0;
pub const SEND_BTN_SIZE: f32 = 30.0;
pub const TOKEN_BAR_WIDTH: f32 = 80.0;
pub const TOKEN_BAR_HEIGHT: f32 = 10.0;
/// 右面板「變更」清單區的最大高度（其下留給 diff 視圖）
pub const PANEL_LIST_MAX_HEIGHT: f32 = 200.0;
/// 側欄「最近」清單列高（hover 命中區估算用）
pub const RECENT_ROW_HEIGHT: f32 = 24.0;
/// 側欄「最近」清單筆數上限
pub const RECENT_SESSIONS_MAX: usize = 8;
/// 活動列展開子卡的左邊框粗細
pub const ACTIVITY_BORDER_WIDTH: f32 = 2.0;

// ─── 全域字級（使用者回饋：預設字太小）──────────────────────────────────────
pub const FONT_HEADING: f32 = 24.0;
pub const FONT_TITLE: f32 = 28.0;
pub const FONT_BODY: f32 = 16.0;
pub const FONT_BUTTON: f32 = 15.5;
pub const FONT_LABEL: f32 = 13.5;
pub const FONT_SMALL: f32 = 13.0;
pub const FONT_CAPTION: f32 = 12.0;
pub const FONT_MONO: f32 = 14.5;

/// 統一套用 egui Visuals 與 text_styles（每幀呼叫，沿用既有字級設定）。
pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(FONT_HEADING),
    );
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(FONT_BODY));
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(FONT_BUTTON),
    );
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(FONT_SMALL));
    style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(FONT_MONO));

    style.visuals.dark_mode = true;
    style.visuals.extreme_bg_color = BG_SIDEBAR;
    style.visuals.window_fill = BG_PRIMARY;
    style.visuals.panel_fill = BG_PRIMARY;
    style.visuals.window_stroke = egui::Stroke::new(1.0, BORDER);
    style.visuals.widgets.inactive.bg_fill = BG_CARD;
    style.visuals.widgets.inactive.weak_bg_fill = BG_CARD;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.hovered.bg_fill = BG_HOVER;
    style.visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.active.bg_fill = BG_HOVER;
    style.visuals.widgets.active.weak_bg_fill = BG_HOVER;
    style.visuals.widgets.noninteractive.bg_fill = BG_PRIMARY;
    style.visuals.selection.bg_fill = ACCENT_BLUE;
    style.visuals.override_text_color = Some(TEXT_PRIMARY);
    ctx.set_style(style);
}

/// 線性插值兩色（toggle 開關動畫用）。
pub fn lerp_color(off: egui::Color32, on: egui::Color32, how_on: f32) -> egui::Color32 {
    let mix = |a: u8, b: u8| -> u8 { (a as f32 + how_on * (b as f32 - a as f32)) as u8 };
    egui::Color32::from_rgb(mix(off.r(), on.r()), mix(off.g(), on.g()), mix(off.b(), on.b()))
}
