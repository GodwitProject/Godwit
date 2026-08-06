CREATE TABLE IF NOT EXISTS metrics_requests (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    model VARCHAR(255) NOT NULL,
    provider VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL,
    api_key_id UUID,
    org_id UUID,
    team_id UUID,
    request_id UUID NOT NULL,
    latency_ms INTEGER NOT NULL
);

CREATE INDEX idx_metrics_requests_timestamp ON metrics_requests(timestamp);
CREATE INDEX idx_metrics_requests_model ON metrics_requests(model);
CREATE INDEX idx_metrics_requests_provider ON metrics_requests(provider);

CREATE TABLE IF NOT EXISTS metrics_latency (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    model VARCHAR(255) NOT NULL,
    provider VARCHAR(100) NOT NULL,
    p50_ms INTEGER NOT NULL,
    p95_ms INTEGER NOT NULL,
    p99_ms INTEGER NOT NULL
);

CREATE INDEX idx_metrics_latency_timestamp ON metrics_latency(timestamp);

CREATE TABLE IF NOT EXISTS alerting_webhooks (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type VARCHAR(100) NOT NULL,
    target_url TEXT NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_attempt TIMESTAMPTZ
);

CREATE INDEX idx_alerting_webhooks_status ON alerting_webhooks(status);
