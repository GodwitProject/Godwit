use dashmap::DashMap;
use godwit_core::PasteurError;
use std::sync::Mutex;
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug)]
pub struct TokenBucket {
    capacity: u32,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self, now: Instant) {
        if self.capacity == 0 {
            return;
        }
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let rate = self.capacity as f64 / 60.0;
        self.tokens = (self.tokens + elapsed * rate).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Refills the bucket from `now`, then reports how many seconds until it could
    /// accommodate an additional `amount` tokens. Returns `None` if the bucket can
    /// already fit `amount` immediately, otherwise the (ceil'd) deficit estimate.
    /// This performs a *tentative* check only — it never debits tokens, so it can be
    /// used to decide whether every bucket in a multi-bucket decision succeeds before
    /// committing any single debit (atomic rate limiting with rollback).
    pub fn deficit_retry_after(&mut self, now: Instant, amount: u32) -> Option<u64> {
        self.refill(now);
        if self.capacity == 0 {
            return None;
        }
        let deficit = amount as f64 - self.tokens;
        if deficit <= 0.0 {
            return None;
        }
        let rate = self.capacity as f64 / 60.0;
        if rate <= 0.0 {
            return None;
        }
        Some((deficit / rate).ceil() as u64)
    }

    /// Debits `amount` tokens from the bucket. Only call after a successful
    /// `deficit_retry_after` check.
    pub fn debit(&mut self, amount: u32) {
        self.tokens -= amount as f64;
    }
}

#[derive(Debug, Default)]
pub struct RateLimiter {
    rpm_buckets: DashMap<(Uuid, String), Mutex<TokenBucket>>,
    tpm_buckets: DashMap<(Uuid, String), Mutex<TokenBucket>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Synchronises the stored bucket with the configured limit and performs a
    /// *tentative* check for `amount`. When `debit` is true and the bucket can fit the
    /// amount, the tokens are consumed. Returns the number of seconds until the bucket
    /// could fit `amount` (i.e. `Some(retry_after)`) if it currently cannot, otherwise
    /// `None`.
    fn check_bucket(
        buckets: &DashMap<(Uuid, String), Mutex<TokenBucket>>,
        key: (Uuid, String),
        limit: Option<i32>,
        amount: u32,
        debit: bool,
    ) -> Option<u64> {
        let limit = match limit {
            Some(l) if l > 0 => l as u32,
            _ => return None,
        };
        let entry = buckets.entry(key).or_insert_with(|| Mutex::new(TokenBucket::new(limit)));
        let mut bucket = entry.value().lock().expect("rate limit bucket mutex poisoned");
        if bucket.capacity != limit {
            bucket.capacity = limit;
            bucket.tokens = bucket.tokens.min(limit as f64);
        }
        let retry_after = bucket.deficit_retry_after(Instant::now(), amount);
        if retry_after.is_none() && debit {
            bucket.debit(amount);
        }
        retry_after
    }

    pub fn check_and_consume(
        &self,
        api_key_id: Uuid,
        org_id: Uuid,
        model: &str,
        rpm_limit_key: Option<i32>,
        tpm_limit_key: Option<i32>,
        rpm_limit_org: Option<i32>,
        tpm_limit_org: Option<i32>,
        estimated_tokens: u32,
    ) -> Result<(), (PasteurError, Option<u64>)> {
        let model = model.to_string();

        // Order matters for shared buckets (rpm & tpm for the same key appear twice)
        // but any single request only debits each bucket once. Use the model id alone
        // for org-level buckets so a single metric doesn't get debited twice.
        let caches: [(&DashMap<(Uuid, String), Mutex<TokenBucket>>, (Uuid, String), Option<i32>, u32); 4] = [
            (&self.rpm_buckets, (api_key_id, model.clone()), rpm_limit_key, 1),
            (&self.tpm_buckets, (api_key_id, model.clone()), tpm_limit_key, estimated_tokens),
            (&self.rpm_buckets, (org_id, model.clone()), rpm_limit_org, 1),
            (&self.tpm_buckets, (org_id, model.clone()), tpm_limit_org, estimated_tokens),
        ];

        // Pass 1: tentatively verify every configured bucket without debiting. If any
        // limit would be exceeded, return 429 without consuming *any* earlier buckets
        // (atomic check-then-commit). Report the slowest refill so the client doesn't
        // retry before every limit has recovered.
        let mut slowest_retry_after: Option<u64> = None;
        for (buckets, key, limit, amount) in &caches {
            if let Some(retry_after) = Self::check_bucket(buckets, key.clone(), *limit, *amount, false)
            {
                slowest_retry_after =
                    Some(slowest_retry_after.map_or(retry_after, |m| m.max(retry_after)));
            }
        }

        if let Some(retry_after) = slowest_retry_after {
            return Err((PasteurError::RateLimited, Some(retry_after)));
        }

        // Pass 2: all limits fit, so commit every debit.
        for (buckets, key, limit, amount) in &caches {
            Self::check_bucket(buckets, key.clone(), *limit, *amount, true);
        }

        Ok(())
    }
}

pub fn estimate_request_tokens(req: &godwit_core::ChatCompletionRequest) -> u32 {
    let mut total: usize = req
        .messages
        .iter()
        .map(|m| {
            let text = match &m.content {
                godwit_core::ChatContent::Text(t) => t.len(),
                godwit_core::ChatContent::Parts(parts) => parts
                    .iter()
                    .map(|p| match p {
                        godwit_core::ChatContentPart::Text { text } => text.len(),
                        godwit_core::ChatContentPart::ImageUrl { image_url } => {
                            image_url.url.len()
                        }
                    })
                    .sum(),
            };
            text + m.role.len()
        })
        .sum::<usize>()
        / 4;
    if let Some(max_tokens) = req.max_tokens {
        total += max_tokens.max(0) as usize;
    }
    total.max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use godwit_core::{ChatCompletionRequest, ChatContent, ChatMessage};
    use std::time::Duration;

    #[test]
    fn token_bucket_allows_within_capacity() {
        let mut bucket = TokenBucket::new(10);
        assert!(bucket.deficit_retry_after(Instant::now(), 5).is_none());
        bucket.debit(5);
        assert!(bucket.deficit_retry_after(Instant::now(), 5).is_none());
        bucket.debit(5);
        assert!(bucket.deficit_retry_after(Instant::now(), 1).is_some());
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(60);
        assert!(bucket.deficit_retry_after(Instant::now(), 60).is_none());
        bucket.debit(60);
        assert!(bucket.deficit_retry_after(Instant::now(), 1).is_some());
        bucket.last_refill = Instant::now() - Duration::from_secs(2);
        assert!(bucket.deficit_retry_after(Instant::now(), 1).is_none());
    }

    #[test]
    fn retry_after_is_computed_from_actual_deficit() {
        // Capacity 60 tokens refills at 1 token/sec. Drain 55, leaving 5.
        let mut bucket = TokenBucket::new(60);
        bucket.tokens = 5.0;
        bucket.last_refill = Instant::now();
        // A 20-token request needs 15 more tokens than are currently available.
        let retry_after = bucket.deficit_retry_after(Instant::now(), 20);
        assert_eq!(retry_after, Some(15));
        // And the tentative check must not have debited anything (only a subnormal
        // refill from the microseconds elapsed between our two Instant::now() calls).
        assert!((bucket.tokens - 5.0).abs() < 0.001, "tokens = {}", bucket.tokens);
    }

    #[test]
    fn rate_limiter_rolls_back_earlier_buckets_on_later_failure() {
        let limiter = RateLimiter::new();
        let api_key = Uuid::new_v4();
        let org = Uuid::new_v4();
        // rpm limit of 1, tpm limit too small for a 100-token request.
        let result = limiter.check_and_consume(
            api_key, org, "gpt-4o", Some(1), Some(10), None, None, 100,
        );
        // The tpm bucket fails, so the request is rejected.
        assert!(matches!(result, Err((PasteurError::RateLimited, _))));
        // The earlier rpm debit must have been rolled back: a small follow-up that
        // only touches rpm should still pass instead of tripping an exhausted rpm.
        assert!(limiter
            .check_and_consume(api_key, org, "gpt-4o", Some(1), Some(10), None, None, 1)
            .is_ok());
    }

    #[test]
    fn rate_limiter_reports_large_tpm_retry_after() {
        let limiter = RateLimiter::new();
        let api_key = Uuid::new_v4();
        let org = Uuid::new_v4();
        // tpm capacity 1000, request wants 3000 tokens -> deficit 2000 at ~16.7 tps.
        let result = limiter.check_and_consume(
            api_key, org, "gpt-4o", None, Some(1000), None, None, 3000,
        );
        match result {
            Err((PasteurError::RateLimited, Some(retry_after))) => {
                assert!(
                    (110..=130).contains(&retry_after),
                    "expected ~120s retry after, got {retry_after}"
                );
            }
            other => panic!("expected rate limited error, got {other:?}"),
        }
    }

    #[test]
    fn rate_limiter_allows_when_no_limits() {
        let limiter = RateLimiter::new();
        let result = limiter.check_and_consume(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "gpt-4o",
            None,
            None,
            None,
            None,
            100,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn rate_limiter_blocks_excess_requests() {
        let limiter = RateLimiter::new();
        let api_key = Uuid::new_v4();
        let org = Uuid::new_v4();
        for _ in 0..3 {
            assert!(limiter
                .check_and_consume(api_key, org, "gpt-4o", Some(3), None, None, None, 1)
                .is_ok());
        }
        let result = limiter.check_and_consume(api_key, org, "gpt-4o", Some(3), None, None, None, 1);
        assert!(matches!(result, Err((PasteurError::RateLimited, _))));
    }

    #[test]
    fn rate_limiter_tracks_key_and_org_separately() {
        let limiter = RateLimiter::new();
        let api_key_a = Uuid::new_v4();
        let api_key_b = Uuid::new_v4();
        let org = Uuid::new_v4();
        assert!(limiter
            .check_and_consume(api_key_a, org, "gpt-4o", Some(1), None, None, None, 1)
            .is_ok());
        assert!(limiter
            .check_and_consume(api_key_b, org, "gpt-4o", Some(1), None, None, None, 1)
            .is_ok());
    }

    #[test]
    fn estimate_tokens_counts_message_length_and_max_tokens() {
        let req = ChatCompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text("hello world".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                cache_control: None,
            }],
            max_tokens: Some(100),
            ..Default::default()
        };
        let estimated = estimate_request_tokens(&req);
        assert!(estimated > 10);
    }
}
