CREATE TABLE IF NOT EXISTS pii_patterns (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    pattern TEXT NOT NULL,
    replacement VARCHAR(100) NOT NULL DEFAULT '[REDACTED]',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO pii_patterns (name, pattern, replacement) VALUES
    ('email', '[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}', '[EMAIL]'),
    ('phone', '\+?[\d\s-()]{10,}', '[PHONE]'),
    ('credit_card', '\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b', '[CARD]'),
    ('ssn', '\b\d{3}-\d{2}-\d{4}\b', '[SSN]')
ON CONFLICT (name) DO NOTHING;

CREATE TABLE IF NOT EXISTS alerting_config (
    id BIGSERIAL PRIMARY KEY,
    org_id UUID,
    team_id UUID,
    api_key_id UUID,
    budget_threshold_percent INTEGER NOT NULL DEFAULT 80,
    webhook_url TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alerting_config_org ON alerting_config(org_id);
CREATE INDEX idx_alerting_config_team ON alerting_config(team_id);
