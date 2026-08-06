use dashmap::DashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::rate_limit::TokenBucket;

pub struct LoginLimiter {
    capacity: u32,
    buckets: DashMap<String, Mutex<TokenBucket>>,
}

impl LoginLimiter {
    pub fn new(capacity: u32) -> Self {
        let capacity = if capacity > 0 { capacity } else { 0 };
        Self { capacity, buckets: DashMap::new() }
    }

    pub fn attempt_allowed(&self, ip: &str, debit_on_fail: bool) -> Option<u64> {
        if self.capacity == 0 {
            return None; // disabled
        }
        let entry = self.buckets.entry(ip.to_string()).or_insert_with(|| {
            Mutex::new(TokenBucket::new(self.capacity))
        });
        let mut bucket = entry.value().lock().expect("login limiter bucket poisoned");
        let retry_after = bucket.deficit_retry_after(Instant::now(), 1);
        if retry_after.is_none() && debit_on_fail {
            bucket.debit(1);
        }
        retry_after
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_capacity_then_blocks() {
        let limiter = LoginLimiter::new(2);
        assert!(limiter.attempt_allowed("1.2.3.4", true).is_none());
        assert!(limiter.attempt_allowed("1.2.3.4", true).is_none());
        let retry = limiter.attempt_allowed("1.2.3.4", true);
        assert!(retry.is_some(), "expected rate limited");
    }

    #[test]
    fn separate_ip_has_own_bucket() {
        let limiter = LoginLimiter::new(1);
        assert!(limiter.attempt_allowed("1.1.1.1", true).is_none());
        assert!(limiter.attempt_allowed("1.1.1.1", true).is_some());
        assert!(limiter.attempt_allowed("2.2.2.2", true).is_none());
    }

    #[test]
    fn non_debit_check_does_not_consume() {
        let limiter = LoginLimiter::new(1);
        assert!(limiter.attempt_allowed("5.5.5.5", false).is_none());
        assert!(limiter.attempt_allowed("5.5.5.5", false).is_none());
    }

    #[test]
    fn zero_capacity_disables() {
        let limiter = LoginLimiter::new(0);
        for _ in 0..10 {
            assert!(limiter.attempt_allowed("9.9.9.9", true).is_none());
        }
    }
}
