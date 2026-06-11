//! 主題層：Claude Desktop 深色版的色彩、圓角、間距與字級。
//! 所有 UI 模組一律引用此處具名常數；新數值禁止在 UI 代碼裸寫。

use eframe::egui;

// ─── 背景層次 ────────────────────────────────────────────────────────────────
/// 視窗 / 中央內容區
pub const BG_PRIMARY: egui::Color32 = egui::Color32::from_rgb(22, 22, 21);
/// 側欄（左側欄 / 右面板 / 頂列）
pub const BG_SIDEBAR: egui::Color32 = egui::Color32::from_rgb(16, 16, 15);
/// 卡片（訊息卡 / 設定列 / 輸入卡）
pub const BG_CARD: egui::Color32 = egui::Color32::from_rgb(33, 33, 32);
/// 輸入欄位 / user 訊息卡（比卡片再亮一階）
pub const BG_TERTIARY: egui::Color32 = egui::Color32::from_rgb(40, 40, 38);
/// hover 高亮
pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(45, 45, 44);
/// 碼塊 / 工具輸出底（比卡片暗一階）
pub const BG_CODE: egui::Color32 = egui::Color32::from_rgb(26, 26, 25);
/// 邊框
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(58, 58, 56);

// ─── 文字 ────────────────────────────────────────────────────────────────────
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(232, 230, 225);
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(160, 158, 152);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(110, 108, 102);

// ─── 強調色 ──────────────────────────────────────────────────────────────────
/// 品牌橘（Claude 風主強調）
pub const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(217, 119, 87);
/// 連結 / 選取藍
pub const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(102, 153, 255);
pub const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(94, 190, 125);
pub const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(235, 90, 90);
pub const ACCENT_YELLOW: egui::Color32 = egui::Color32::from_rgb(240, 200, 80);

// ─── Diff 行底色 ─────────────────────────────────────────────────────────────
pub const DIFF_ADDED_BG: egui::Color32 = egui::Color32::from_rgb(28, 52, 36);
pub const DIFF_REMOVED_BG: egui::Color32 = egui::Color32::from_rgb(58, 30, 30);

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
