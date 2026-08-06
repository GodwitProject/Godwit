use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub struct LoadBalancerState {
    pub rr_index: AtomicUsize,
    pub in_flight: AtomicUsize,
    pub latency_ewma: AtomicU64,
}

impl LoadBalancerState {
    pub fn new() -> Self {
        Self {
            rr_index: AtomicUsize::new(0),
            in_flight: AtomicUsize::new(0),
            latency_ewma: AtomicU64::new(0f64.to_bits()),
        }
    }

    pub fn get_ewma(&self) -> f64 {
        f64::from_bits(self.latency_ewma.load(Ordering::Relaxed))
    }

    pub fn update_ewma(&self, duration_ms: f64) {
        let alpha = 0.2;
        let old_bits = self.latency_ewma.load(Ordering::Relaxed);
        let old = f64::from_bits(old_bits);
        let new = if old_bits == 0 {
            duration_ms
        } else {
            alpha * duration_ms + (1.0 - alpha) * old
        };
        self.latency_ewma.store(new.to_bits(), Ordering::Relaxed);
    }
}

pub struct LoadBalancer {
    states: DashMap<Uuid, Arc<LoadBalancerState>>,
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }

    fn get_state(&self, model_id: Uuid) -> Arc<LoadBalancerState> {
        self.states
            .entry(model_id)
            .or_insert_with(|| Arc::new(LoadBalancerState::new()))
            .clone()
    }

    pub fn select_provider(
        &self,
        strategy: LoadBalanceStrategy,
        model_ids: &[Uuid],
    ) -> Option<usize> {
        if model_ids.is_empty() {
            return None;
        }

        match strategy {
            LoadBalanceStrategy::RoundRobin => {
                let first_state = self.get_state(model_ids[0]);
                let idx = first_state.rr_index.fetch_add(1, Ordering::SeqCst) % model_ids.len();
                Some(idx)
            }
            LoadBalanceStrategy::LeastBusy => {
                let mut min_idx = 0;
                let mut min_in_flight = usize::MAX;
                for (i, id) in model_ids.iter().enumerate() {
                    let state = self.get_state(*id);
                    let in_flight = state.in_flight.load(Ordering::SeqCst);
                    if in_flight < min_in_flight {
                        min_in_flight = in_flight;
                        min_idx = i;
                    }
                }
                Some(min_idx)
            }
            LoadBalanceStrategy::Latency => {
                let mut min_idx = 0;
                let mut min_ewma = f64::INFINITY;
                for (i, id) in model_ids.iter().enumerate() {
                    let state = self.get_state(*id);
                    let ewma = state.get_ewma();
                    if ewma < min_ewma {
                        min_ewma = ewma;
                        min_idx = i;
                    }
                }
                Some(min_idx)
            }
        }
    }

    pub fn increment_in_flight(&self, model_id: Uuid) -> InFlightGuard {
        let state = self.get_state(model_id);
        state.in_flight.fetch_add(1, Ordering::SeqCst);
        InFlightGuard { state }
    }

    pub fn record_latency(&self, model_id: Uuid, duration_ms: f64) {
        let state = self.get_state(model_id);
        state.update_ewma(duration_ms);
    }
}

pub struct InFlightGuard {
    state: Arc<LoadBalancerState>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.state.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastBusy,
    Latency,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_cycles_through_indices() {
        let lb = LoadBalancer::new();
        let models = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let idx0 = lb.select_provider(LoadBalanceStrategy::RoundRobin, &models);
        let idx1 = lb.select_provider(LoadBalanceStrategy::RoundRobin, &models);
        let idx2 = lb.select_provider(LoadBalanceStrategy::RoundRobin, &models);
        let idx3 = lb.select_provider(LoadBalanceStrategy::RoundRobin, &models);

        assert_eq!(idx0, Some(0));
        assert_eq!(idx1, Some(1));
        assert_eq!(idx2, Some(2));
        assert_eq!(idx3, Some(0));
    }

    #[test]
    fn least_busy_prefers_idle_model() {
        let lb = LoadBalancer::new();
        let models = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let _guard0 = lb.increment_in_flight(models[0]);
        let _guard1 = lb.increment_in_flight(models[0]);
        let _guard2 = lb.increment_in_flight(models[2]);

        let selected = lb.select_provider(LoadBalanceStrategy::LeastBusy, &models);
        assert_eq!(selected, Some(1));
    }

    #[test]
    fn least_busy_ties_break_by_index() {
        let lb = LoadBalancer::new();
        let models = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let selected = lb.select_provider(LoadBalanceStrategy::LeastBusy, &models);
        assert_eq!(selected, Some(0));
    }

    #[test]
    fn latency_prefers_lower_ewma() {
        let lb = LoadBalancer::new();
        let models = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        lb.record_latency(models[0], 1000.0);
        lb.record_latency(models[1], 100.0);

        let selected = lb.select_provider(LoadBalanceStrategy::Latency, &models);
        assert_eq!(selected, Some(1));
    }

    #[test]
    fn latency_ewma_updates_correctly() {
        let lb = LoadBalancer::new();
        let model = Uuid::new_v4();

        lb.record_latency(model, 1000.0);
        lb.record_latency(model, 200.0);

        let state = lb.get_state(model);
        let ewma = state.get_ewma();

        let expected = 0.2 * 200.0 + 0.8 * 1000.0;
        assert!((ewma - expected).abs() < 0.01, "expected {}, got {}", expected, ewma);
    }

    #[test]
    fn stress_concurrent_round_robin() {
        use std::thread;

        let lb = Arc::new(LoadBalancer::new());
        let models = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let mut handles = vec![];
        for _ in 0..10 {
            let lb = Arc::clone(&lb);
            let models = models.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _ = lb.select_provider(LoadBalanceStrategy::RoundRobin, &models);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let final_idx = lb.select_provider(LoadBalanceStrategy::RoundRobin, &models);
        assert!(final_idx.is_some());
    }

    #[test]
    fn stress_concurrent_in_flight() {
        use std::thread;

        let lb = Arc::new(LoadBalancer::new());
        let model = Uuid::new_v4();

        let mut handles = vec![];
        for _ in 0..10 {
            let lb = Arc::clone(&lb);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _guard = lb.increment_in_flight(model);
                    thread::sleep(std::time::Duration::from_micros(10));
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let state = lb.get_state(model);
        assert_eq!(state.in_flight.load(Ordering::SeqCst), 0);
    }
}
