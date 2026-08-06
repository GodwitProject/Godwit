use lazy_static::lazy_static;
use prometheus::{CounterVec, GaugeVec, HistogramVec, HistogramOpts, Opts, Registry, TextEncoder};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref REQUESTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("godwit_requests_total", "Total requests"),
        &["model", "provider", "status"]
    ).unwrap();

    pub static ref REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("godwit_request_duration_seconds", "Request duration")
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        &["model", "provider"]
    ).unwrap();

    pub static ref TOKENS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("godwit_tokens_total", "Total tokens"),
        &["type", "model"]
    ).unwrap();

    pub static ref COST_USD_TOTAL: CounterVec = CounterVec::new(
        Opts::new("godwit_cost_usd_total", "Total cost in USD"),
        &["org", "team", "api_key"]
    ).unwrap();

    pub static ref ACTIVE_REQUESTS: GaugeVec = GaugeVec::new(
        Opts::new("godwit_active_requests", "Active requests"),
        &["model", "provider"]
    ).unwrap();
}

pub fn register_metrics() -> Result<(), prometheus::Error> {
    REGISTRY.register(Box::new(REQUESTS_TOTAL.clone()))?;
    REGISTRY.register(Box::new(REQUEST_DURATION.clone()))?;
    REGISTRY.register(Box::new(TOKENS_TOTAL.clone()))?;
    REGISTRY.register(Box::new(COST_USD_TOTAL.clone()))?;
    REGISTRY.register(Box::new(ACTIVE_REQUESTS.clone()))?;
    Ok(())
}

pub fn get_metrics() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    encoder.encode_to_string(&metric_families)
}

pub struct MetricsCollector;

impl MetricsCollector {
    pub fn record_request(model: &str, provider: &str, status: &str, duration_secs: f64) {
        REQUESTS_TOTAL
            .with_label_values(&[model, provider, status])
            .inc();
        REQUEST_DURATION
            .with_label_values(&[model, provider])
            .observe(duration_secs);
    }

    pub fn record_tokens(token_type: &str, model: &str, count: u32) {
        TOKENS_TOTAL
            .with_label_values(&[token_type, model])
            .inc_by(count as f64);
    }

    pub fn record_cost(org: &str, team: &str, api_key: &str, cost_usd: f64) {
        COST_USD_TOTAL
            .with_label_values(&[org, team, api_key])
            .inc_by(cost_usd);
    }

    pub fn increment_active(model: &str, provider: &str) {
        ACTIVE_REQUESTS
            .with_label_values(&[model, provider])
            .inc();
    }

    pub fn decrement_active(model: &str, provider: &str) {
        ACTIVE_REQUESTS
            .with_label_values(&[model, provider])
            .dec();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        let _ = register_metrics();
    }

    #[test]
    fn test_record_request_increments_counter() {
        setup();
        MetricsCollector::record_request("gpt-4", "openai", "success", 0.5);

        let metric_families = REGISTRY.gather();
        let requests_total = metric_families
            .iter()
            .find(|mf| mf.get_name() == "godwit_requests_total")
            .expect("godwit_requests_total metric should exist");

        assert!(!requests_total.get_metric().is_empty());
    }

    #[test]
    fn test_record_tokens_by_type() {
        setup();
        MetricsCollector::record_tokens("input", "gpt-4", 100);
        MetricsCollector::record_tokens("output", "gpt-4", 50);

        let metric_families = REGISTRY.gather();
        let tokens_total = metric_families
            .iter()
            .find(|mf| mf.get_name() == "godwit_tokens_total")
            .expect("godwit_tokens_total metric should exist");

        assert!(!tokens_total.get_metric().is_empty());
    }

    #[test]
    fn test_active_requests_gauge() {
        setup();
        MetricsCollector::increment_active("gpt-4", "openai");
        MetricsCollector::increment_active("gpt-4", "openai");
        MetricsCollector::decrement_active("gpt-4", "openai");

        let metric_families = REGISTRY.gather();
        let active_requests = metric_families
            .iter()
            .find(|mf| mf.get_name() == "godwit_active_requests")
            .expect("godwit_active_requests metric should exist");

        let metrics = active_requests.get_metric();
        assert!(!metrics.is_empty());

        let gauge_value = metrics[0].get_gauge().get_value();
        assert_eq!(gauge_value, 1.0);
    }

    #[test]
    fn test_record_cost() {
        setup();
        MetricsCollector::record_cost("org-1", "team-1", "key-1", 0.05);

        let metric_families = REGISTRY.gather();
        let cost_total = metric_families
            .iter()
            .find(|mf| mf.get_name() == "godwit_cost_usd_total")
            .expect("godwit_cost_usd_total metric should exist");

        assert!(!cost_total.get_metric().is_empty());
    }

    #[test]
    fn test_request_duration_histogram() {
        setup();
        MetricsCollector::record_request("claude-3", "anthropic", "success", 1.5);

        let metric_families = REGISTRY.gather();
        let duration = metric_families
            .iter()
            .find(|mf| mf.get_name() == "godwit_request_duration_seconds")
            .expect("godwit_request_duration_seconds metric should exist");

        assert!(!duration.get_metric().is_empty());
    }
}
