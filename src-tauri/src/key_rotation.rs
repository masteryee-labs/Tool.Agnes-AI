//! 多 API Key 輪詢器。
//!
//! Agnes AI 免費方案每帳號有獨立速率上限（20 RPM）。使用者可註冊多個帳號、
//! 各別取得 API Key，本模組在這些 Key 之間輪詢：
//!
//! - **計數輪詢**：每 `rotate_every` 次 API 呼叫後自動切到下一把 Key，把流量
//!   平均分散到所有帳號，盡量不觸及任一帳號的速率上限。
//! - **429 強制換 Key**：收到 420/429 時呼叫 [`KeyRotator::mark_rate_limited`]，
//!   立即跳到下一把 Key 重試，不必乾等退避——這是多帳號方案的核心收益。
//! - **單 Key 退化**：只有一把 Key 時不輪詢，行為等同舊版，向後相容。
//!
//! 執行緒安全：內部以 `std::sync::Mutex` 保護計數與索引，可安全跨 await 點
//! 共享（`Arc<KeyRotator>`）。臨界區極短（取 key + 加計數），不會阻塞非同步
//! 執行緒池。

use std::sync::{Arc, Mutex};

/// 預設每把 Key 連續使用幾次後輪到下一把。
/// 低於免費方案 20 RPM 上限，留出安全餘裕避免單帳號被卡。
pub const DEFAULT_ROTATE_EVERY: u32 = 15;

/// 多 API Key 輪詢器。
pub struct KeyRotator {
    keys: Vec<String>,
    rotate_every: u32,
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    /// 目前使用中的 Key 索引（在 `keys` 內）。
    current_index: usize,
    /// 目前 Key 已連續使用的呼叫次數。
    calls_since_rotation: u32,
}

/// 取 key 的 SHA-256 指紋前 8 碼（與 `key_persistence::hash_key` 一致），
/// 供 UI / 日誌顯示，不暴露原始金鑰。
fn fingerprint(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())[..8].to_string()
}

impl KeyRotator {
    /// 建構輪詢器。
    ///
    /// - `keys`：至少一把有效金鑰；空集合會回傳 `None`（呼叫端應退回「未設定」錯誤）。
    /// - `rotate_every`：每把 Key 連續使用幾次後輪替；0 視為 [`DEFAULT_ROTATE_EVERY`]。
    pub fn new(keys: Vec<String>, rotate_every: u32) -> Option<Arc<Self>> {
        if keys.iter().all(|k| k.trim().is_empty()) {
            return None;
        }
        let keys: Vec<String> = keys
            .into_iter()
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
            .collect();
        if keys.is_empty() {
            return None;
        }
        let rotate_every = if rotate_every == 0 {
            DEFAULT_ROTATE_EVERY
        } else {
            rotate_every
        };
        Some(Arc::new(Self {
            keys,
            rotate_every,
            state: Mutex::new(State {
                current_index: 0,
                calls_since_rotation: 0,
            }),
        }))
    }

    /// 金鑰數量。
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// 是否為單 Key 模式（不輪詢）。
    pub fn is_single(&self) -> bool {
        self.keys.len() <= 1
    }

    /// 取得下一把要使用的 Key（會推進計數並在閾值時輪替）。
    ///
    /// 回傳 `Err` 僅在金鑰全空時（建構時已過濾，正常路徑不會發生）。
    pub fn next_key(&self) -> Result<String, String> {
        let mut s = self.state.lock().unwrap();
        let key = self.keys[s.current_index].clone();
        s.calls_since_rotation += 1;
        // 多 Key 才輪替；單 Key 時只累加計數不換索引
        if self.keys.len() > 1 && s.calls_since_rotation >= self.rotate_every {
            s.current_index = (s.current_index + 1) % self.keys.len();
            s.calls_since_rotation = 0;
        }
        Ok(key)
    }

    /// 目前使用中的 Key（不推進計數）——供 UI 顯示遮罩金鑰用。
    pub fn current_key(&self) -> String {
        let s = self.state.lock().unwrap();
        self.keys[s.current_index].clone()
    }

    /// 目前 Key 的指紋（前 8 碼）。
    pub fn current_fingerprint(&self) -> String {
        fingerprint(&self.current_key())
    }

    /// 所有 Key 的指紋清單（供 UI 顯示金鑰組）。
    pub fn fingerprints(&self) -> Vec<String> {
        self.keys.iter().map(|k| fingerprint(k)).collect()
    }

    /// 標記目前 Key 被速率限制（429/420），立即輪到下一把。
    /// 單 Key 模式下為 no-op（沒有別把可換，由呼叫端的退避重試處理）。
    pub fn mark_rate_limited(&self) {
        if self.keys.len() <= 1 {
            return;
        }
        let mut s = self.state.lock().unwrap();
        s.current_index = (s.current_index + 1) % self.keys.len();
        s.calls_since_rotation = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rot(keys: &[&str], every: u32) -> Arc<KeyRotator> {
        KeyRotator::new(keys.iter().map(|s| s.to_string()).collect(), every).unwrap()
    }

    #[test]
    fn empty_keys_returns_none() {
        assert!(KeyRotator::new(Vec::new(), 15).is_none());
        assert!(KeyRotator::new(vec!["".to_string()], 15).is_none());
    }

    #[test]
    fn single_key_never_rotates() {
        let r = rot(&["sk-a"], 2);
        assert_eq!(r.next_key().unwrap(), "sk-a");
        assert_eq!(r.next_key().unwrap(), "sk-a");
        assert_eq!(r.next_key().unwrap(), "sk-a");
        assert!(r.is_single());
    }

    #[test]
    fn rotates_after_threshold() {
        let r = rot(&["sk-a", "sk-b"], 2);
        assert_eq!(r.next_key().unwrap(), "sk-a");
        assert_eq!(r.next_key().unwrap(), "sk-a");
        // 第 3 次取用應已輪到 b
        assert_eq!(r.next_key().unwrap(), "sk-b");
        assert_eq!(r.next_key().unwrap(), "sk-b");
        // 第 5 次取用應回到 a
        assert_eq!(r.next_key().unwrap(), "sk-a");
    }

    #[test]
    fn mark_rate_limited_skips_immediately() {
        let r = rot(&["sk-a", "sk-b", "sk-c"], 100);
        assert_eq!(r.next_key().unwrap(), "sk-a");
        r.mark_rate_limited();
        assert_eq!(r.next_key().unwrap(), "sk-b");
        r.mark_rate_limited();
        assert_eq!(r.next_key().unwrap(), "sk-c");
        r.mark_rate_limited();
        // 回到 a（環狀）
        assert_eq!(r.next_key().unwrap(), "sk-a");
    }

    #[test]
    fn mark_rate_limited_noop_on_single() {
        let r = rot(&["sk-a"], 5);
        r.mark_rate_limited();
        assert_eq!(r.next_key().unwrap(), "sk-a");
    }

    #[test]
    fn fingerprints_match_hash_key() {
        use crate::config::key_persistence;
        let r = rot(&["sk-secret"], 5);
        assert_eq!(r.current_fingerprint(), hash_key("sk-secret")[..8]);
        assert_eq!(
            r.current_fingerprint(),
            key_persistence::hash_key("sk-secret")[..8].to_string()
        );
    }

    #[test]
    fn zero_rotate_every_uses_default() {
        let r = rot(&["sk-a", "sk-b"], 0);
        // 不 panic 即可，內部使用 DEFAULT_ROTATE_EVERY
        assert_eq!(r.key_count(), 2);
    }

    fn hash_key(k: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(k.as_bytes());
        format!("{:x}", h.finalize())
    }
}
