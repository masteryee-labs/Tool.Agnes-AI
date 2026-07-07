//! ─── 多模態媒體生成（MultimodalMediaSpecialist，動態激活）────────────────────
//!
//! 接 Agnes Image 2.1 Flash / Agnes-Video-V2.0。設計守則：
//!  - 動態激活：僅在意圖為視覺生成時由 Orchestrator 喚醒；非視覺任務此組休眠、零成本。
//!  - 共用全域 rate_limiter：每次媒體 API 呼叫一樣計入 20 RPM，與文字/記憶呼叫同桶。
//!  - 意圖偵測為確定性 Rust（0 token），不為「要不要生圖」多花一次模型呼叫。

use crate::config::MultimodalConfig;
use crate::rate_limiter::RateLimiter;

/// 媒體生成結果：url 或 base64 擇一（依供應商回應），raw 保留原始 JSON 供除錯。
#[derive(Debug, Clone)]
pub struct MediaResult {
    pub kind: String,
    pub url: Option<String>,
    pub b64: Option<String>,
    pub raw: String,
}

/// 視覺意圖偵測（確定性，0 token）：含生圖/生影關鍵詞才激活多模態組。
pub fn is_visual_intent(prompt: &str) -> bool {
    let p = prompt.to_lowercase();
    const VISUAL_KW: &[&str] = &[
        "生成圖", "畫一", "畫個", "畫張", "圖片", "插圖", "圖示", "海報", "logo", "icon",
        "image", "picture", "draw", "render", "影片", "動畫", "短片", "video", "animation",
    ];
    VISUAL_KW.iter().any(|kw| p.contains(kw))
}

/// 組裝 image 生成請求（純函式，便於測試）。
pub fn build_image_request(model: &str, prompt: &str, size: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "prompt": prompt,
        "n": 1,
        "size": size,
    })
}

/// 組裝 video 生成請求（純函式，便於測試）。
pub fn build_video_request(model: &str, prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "prompt": prompt,
    })
}

/// 多模態媒體管理器。持有端點/模型組態與共享金鑰輪詢器；HTTP client 與 rate_limiter 由呼叫端注入。
pub struct MultimodalManager {
    cfg: MultimodalConfig,
    key_rotator: std::sync::Arc<crate::key_rotation::KeyRotator>,
}

impl MultimodalManager {
    pub fn new(cfg: MultimodalConfig, key_rotator: std::sync::Arc<crate::key_rotation::KeyRotator>) -> Self {
        Self { cfg, key_rotator }
    }

    /// 生成圖片（Agnes Image 2.1 Flash）。共用 rate_limiter，計入 20 RPM。
    pub async fn generate_image(
        &self,
        limiter: &RateLimiter,
        prompt: &str,
    ) -> Result<MediaResult, String> {
        let payload = build_image_request(&self.cfg.image_model, prompt, &self.cfg.default_image_size);
        self.post_media(limiter, &self.cfg.image_endpoint, &payload, "image")
            .await
    }

    /// 生成影片（Agnes-Video-V2.0）。共用 rate_limiter，計入 20 RPM。
    pub async fn generate_video(
        &self,
        limiter: &RateLimiter,
        prompt: &str,
    ) -> Result<MediaResult, String> {
        let payload = build_video_request(&self.cfg.video_model, prompt);
        self.post_media(limiter, &self.cfg.video_endpoint, &payload, "video")
            .await
    }

    async fn post_media(
        &self,
        limiter: &RateLimiter,
        endpoint: &str,
        payload: &serde_json::Value,
        kind: &str,
    ) -> Result<MediaResult, String> {
        // 媒體生成耗時（實測 Agnes Image 單張 ~50s），用專屬長逾時客戶端，
        // 不可沿用文字/工具用的 30s 短逾時池，否則必逾時失敗。
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.cfg.timeout_seconds))
            .build()
            .map_err(|e| format!("無法初始化多模態 HTTP 客戶端: {}", e))?;
        limiter.acquire().await; // 計入全域 20 RPM 令牌桶
        let api_key = self.key_rotator.next_key().map_err(|e| format!("API key 未設定，無法呼叫多模態服務: {}", e))?;
        let res = client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(payload)
            .send()
            .await
            .map_err(|e| format!("媒體生成請求失敗: {}", e))?;
        if !res.status().is_success() {
            return Err(format!("媒體 API 回傳錯誤狀態: {}", res.status()));
        }
        let json: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("解析媒體回應失敗: {}", e))?;
        let url = json["data"][0]["url"].as_str().map(|s| s.to_string());
        let b64 = json["data"][0]["b64_json"].as_str().map(|s| s.to_string());
        Ok(MediaResult {
            kind: kind.to_string(),
            url,
            b64,
            raw: json.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_intent_positive() {
        assert!(is_visual_intent("幫我畫一隻貓的插圖"));
        assert!(is_visual_intent("generate an image of a logo"));
        assert!(is_visual_intent("做一段產品動畫短片"));
    }

    #[test]
    fn test_visual_intent_negative() {
        assert!(!is_visual_intent("寫一個 Rust 函式計算費式數列"));
        assert!(!is_visual_intent("refactor the database module"));
    }

    #[test]
    fn test_image_request_shape() {
        let r = build_image_request("agnes-image-2.1-flash", "a red apple", "1024x1024");
        assert_eq!(r["model"], "agnes-image-2.1-flash");
        assert_eq!(r["prompt"], "a red apple");
        assert_eq!(r["size"], "1024x1024");
        assert_eq!(r["n"], 1);
    }

    #[test]
    fn test_video_request_shape() {
        let r = build_video_request("agnes-video-v2.0", "ocean waves");
        assert_eq!(r["model"], "agnes-video-v2.0");
        assert_eq!(r["prompt"], "ocean waves");
    }

    #[tokio::test]
    async fn test_empty_key_rejected() {
        // 無金鑰時 build_rotator 回 None，模擬呼叫端無法建構 MultimodalManager。
        // 此測試改驗證 KeyRotator::new 對空集合回 None。
        assert!(crate::key_rotation::KeyRotator::new(Vec::new(), 15).is_none());
        // 確保 rate_limiter 仍可正常建構（不影響其他路徑）
        let _limiter = RateLimiter::new(20);
    }
}
