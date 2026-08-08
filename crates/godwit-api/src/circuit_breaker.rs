use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{RwLock, Arc};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    failures: AtomicUsize,
    successes: AtomicUsize,
    last_failure: RwLock<Option<Instant>>,
    state: RwLock<CircuitState>,
    threshold: usize,
    timeout: Duration,
    half_open_max: usize,
    half_open_requests: AtomicUsize,
}

impl CircuitBreaker {
    pub fn new(threshold: usize, timeout: Duration, half_open_max: usize) -> Self {
        Self {
            failures: AtomicUsize::new(0),
            successes: AtomicUsize::new(0),
            last_failure: RwLock::new(None),
            state: RwLock::new(CircuitState::Closed),
            threshold,
            timeout,
            half_open_max,
            half_open_requests: AtomicUsize::new(0),
        }
    }

    pub fn can_execute(&self) -> bool {
        let current_state = self.state();
        match current_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure.read().unwrap();
                if let Some(last) = *last_failure {
                    last.elapsed() >= self.timeout
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => {
                let reserved = self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                if reserved < self.half_open_max {
                    true
                } else {
                    self.half_open_requests.fetch_sub(1, Ordering::SeqCst);
                    false
                }
            }
        }
    }

    pub fn record_success(&self) {
        self.successes.fetch_add(1, Ordering::SeqCst);
        let mut state = self.state.write().unwrap();
        match *state {
            CircuitState::HalfOpen => {
                *state = CircuitState::Closed;
                self.failures.store(0, Ordering::SeqCst);
                self.half_open_requests.store(0, Ordering::SeqCst);
            }
            CircuitState::Closed => {
                self.failures.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&self) {
        self.failures.fetch_add(1, Ordering::SeqCst);
        *self.last_failure.write().unwrap() = Some(Instant::now());
        
        let mut state = self.state.write().unwrap();
        match *state {
            CircuitState::Closed => {
                if self.failures.load(Ordering::SeqCst) >= self.threshold {
                    *state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                *state = CircuitState::Open;
                self.half_open_requests.store(0, Ordering::SeqCst);
            }
            CircuitState::Open => {}
        }
    }

    pub fn state(&self) -> CircuitState {
        let mut guard = self.state.write().unwrap();
        let current = *guard;
        if current == CircuitState::Open {
            let last_failure = self.last_failure.read().unwrap();
            if let Some(last) = *last_failure {
                if last.elapsed() >= self.timeout {
                    *guard = CircuitState::HalfOpen;
                    return CircuitState::HalfOpen;
                }
            }
        }
        current
    }
}

use dashmap::DashMap;

pub struct CircuitBreakerRegistry {
    breakers: DashMap<String, Arc<CircuitBreaker>>,
    threshold: usize,
    timeout: Duration,
    half_open_max: usize,
}

impl CircuitBreakerRegistry {
    pub fn new(threshold: usize, timeout: Duration, half_open_max: usize) -> Self {
        Self {
            breakers: DashMap::new(),
            threshold,
            timeout,
            half_open_max,
        }
    }

    pub fn get(&self, provider_id: &str) -> Arc<CircuitBreaker> {
        self.breakers
            .entry(provider_id.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(
                    self.threshold,
                    self.timeout,
                    self.half_open_max,
                ))
            })
            .clone()
    }

    pub fn record_success(&self, provider_id: &str) {
        let breaker = self.get(provider_id);
        breaker.record_success();
    }

    pub fn record_failure(&self, provider_id: &str) {
        let breaker = self.get(provider_id);
        breaker.record_failure();
    }

    pub fn all_states(&self) -> Vec<(String, CircuitState)> {
        self.breakers
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().state()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_in_closed_state() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(60), 3);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn circuit_opens_after_threshold_failures() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60), 2);
        for _ in 0..3 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn circuit_half_opens_after_timeout() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(100), 2);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.can_execute());
    }

    #[test]
    fn success_in_half_open_closes_circuit() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50), 2);
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(60));
        
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn failure_in_half_open_reopens_circuit() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50), 2);
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(60));
        
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn half_open_limits_concurrent_requests() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50), 2);
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(60));
        
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.can_execute());
        assert!(cb.can_execute());
        assert!(!cb.can_execute());
    }

    #[test]
    fn registry_creates_breaker_on_demand() {
        let registry = CircuitBreakerRegistry::new(5, Duration::from_secs(60), 3);
        let breaker = registry.get("provider-1");
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn registry_records_per_provider() {
        let registry = CircuitBreakerRegistry::new(2, Duration::from_secs(60), 2);
        registry.record_failure("provider-1");
        registry.record_failure("provider-1");
        
        let p1 = registry.get("provider-1");
        let p2 = registry.get("provider-2");
        
        assert_eq!(p1.state(), CircuitState::Open);
        assert_eq!(p2.state(), CircuitState::Closed);
    }
}
