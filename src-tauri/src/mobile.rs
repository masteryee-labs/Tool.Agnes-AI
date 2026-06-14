//! ─── UniFFI 行動端綁定（iOS / Android 殼層）─────────────────────────────────
//!
//! 以 `--features mobile` 編譯啟用。採 UniFFI proc-macro（官方現行推薦法），
//! lib crate 已含 staticlib / cdylib，可直接產生各語言綁定：
//!
//! ```text
//! cargo build --features mobile --release
//! uniffi-bindgen generate --library target/release/app_lib.dll \
//!     --language swift   --out-dir bindings/swift     # iOS
//! uniffi-bindgen generate --library target/release/app_lib.so  \
//!     --language kotlin  --out-dir bindings/kotlin    # Android
//! ```
//!
//! 介面定義（UDL 視角）見 `src/agnes.udl`。此處僅匯出「確定性、可離線」的核心
//! 功能：版本、組態摘要、視覺意圖偵測、token 估算——重活（async 引擎）留在桌面端。

use crate::config::Config;

/// 行動端可讀的組態摘要（跨語言 Record）。
#[derive(uniffi::Record)]
pub struct MobileConfigSummary {
    pub version: String,
    pub image_model: String,
    pub video_model: String,
    pub max_rpm: u32,
}

/// 引擎版本字串。
#[uniffi::export]
pub fn agnes_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 預設組態摘要（行動殼層啟動畫面用）。
#[uniffi::export]
pub fn agnes_default_config() -> MobileConfigSummary {
    let cfg = Config::default();
    MobileConfigSummary {
        version: env!("CARGO_PKG_VERSION").to_string(),
        image_model: cfg.multimodal.image_model,
        video_model: cfg.multimodal.video_model,
        max_rpm: cfg.api.max_rpm,
    }
}

/// 視覺意圖偵測（確定性，可在行動端離線判斷是否需呼叫多模態，省一次往返）。
#[uniffi::export]
pub fn agnes_is_visual_intent(prompt: String) -> bool {
    crate::multimodal::is_visual_intent(&prompt)
}

/// 本地 token 估算（確定性，0 API）。
#[uniffi::export]
pub fn agnes_estimate_tokens(text: String) -> u64 {
    crate::memory::estimate_tokens(&text) as u64
}
