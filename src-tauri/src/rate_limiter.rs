use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 令牌桶速率限制器。依 Agnes-2.0-Flash 免費方案 20 RPM 上限設計。
/// acquire() 在無令牌時等待直到補充完成，不拒絕請求。
/// max_rpm = 0 進入無限制模式（測試用）。
pub struct RateLimiter {
    capacity: f64,
    /// (current_tokens, last_refill_instant)
    state: Arc<Mutex<(f64, Instant)>>,
    refill_rate: f64, // tokens per second
}

impl RateLimiter {
    pub fn new(max_rpm: u32) -> Self {
        if max_rpm == 0 {
            return Self {
                capacity: f64::MAX,
                state: Arc::new(Mutex::new((f64::MAX, Instant::now()))),
                refill_rate: 0.0,
            };
        }
        let cap = max_rpm as f64;
        Self {
            capacity: cap,
            state: Arc::new(Mutex::new((cap, Instant::now()))),
            refill_rate: cap / 60.0,
        }
    }

    fn refill_and_get(&self) -> f64 {
        let mut s = self.state.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(s.1).as_secs_f64();
        if self.refill_rate > 0.0 {
            s.0 = (s.0 + elapsed * self.refill_rate).min(self.capacity);
        }
        s.1 = now;
        s.0
    }

    /// 消耗一個令牌；無令牌時等待直到補充完成。
    pub async fn acquire(&self) {
        loop {
            let tokens = self.refill_and_get();
            if tokens >= 1.0 {
                self.state.lock().unwrap().0 -= 1.0;
                return;
            }
            let deficit = 1.0 - tokens;
            let wait_ms = if self.refill_rate > 0.0 {
                ((deficit / self.refill_rate) * 1000.0) as u64
            } else {
                0
            };
            tokio::time::sleep(Duration::from_millis(wait_ms.max(100))).await;
        }
    }

    #[cfg(test)]
    pub fn current_tokens(&self) -> f64 {
        self.refill_and_get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_first_acquire_immediate() {
        let limiter = RateLimiter::new(20);
        let start = Instant::now();
        limiter.acquire().await;
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "First acquire should be near-instant"
        );
    }

    #[tokio::test]
    async fn test_bucket_drains_after_capacity() {
        let limiter = RateLimiter::new(3); // 3 RPM
        limiter.acquire().await;
        limiter.acquire().await;
        limiter.acquire().await;
        let t = limiter.current_tokens();
        assert!(t < 0.5, "Bucket should be near-empty after 3 acquires, got {}", t);
    }

    #[test]
    fn test_unlimited_mode_has_max_capacity() {
        let limiter = RateLimiter::new(0);
        assert_eq!(limiter.capacity, f64::MAX);
    }

    #[tokio::test]
    async fn test_burst_beyond_capacity_blocks_until_refill() {
        // 蒸餾組一次最多連發 alpha/beta/integrator 三個 LLM 呼叫，全部共用此桶。
        // 抽乾整桶後的下一次取用必須等待補充，確保記憶歸檔不會突破 RPM 觸發 429。
        let limiter = RateLimiter::new(60); // refill 1 token/sec
        for _ in 0..60 {
            limiter.acquire().await; // 桶起始為滿，60 次皆即時
        }
        let start = Instant::now();
        limiter.acquire().await; // 桶空 → 阻塞至補充 ~1 秒
        assert!(
            start.elapsed() >= Duration::from_millis(500),
            "令牌用盡後的取用必須阻塞等待補充，量得 {:?}",
            start.elapsed()
        );
    }
}
