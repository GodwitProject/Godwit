-- File: crates/godwit-db/migrations/20260811000001_batches.up.sql
CREATE TABLE IF NOT EXISTS batches (
    id BIGSERIAL PRIMARY KEY,
    public_id VARCHAR(64) NOT NULL UNIQUE,
    user_id BIGSERIAL NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_id BIGSERIAL REFERENCES teams(id) ON DELETE CASCADE,
    
    -- Batch metadata
    operation_type VARCHAR(32) NOT NULL, -- 'chat', 'embedding', 'moderation', 'rerank'
    status VARCHAR(32) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed', 'cancelled'
    
    -- Request counts
    total_requests INTEGER NOT NULL DEFAULT 0,
    completed_requests INTEGER NOT NULL DEFAULT 0,
    failed_requests INTEGER NOT NULL DEFAULT 0,
    
    -- Cost tracking (using Decimal for precision)
    total_cost_usd NUMERIC(12,4) DEFAULT 0,
    total_input_tokens BIGINT DEFAULT 0,
    total_output_tokens BIGINT DEFAULT 0,
    
    -- Timing
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ DEFAULT NULL,
    
    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS batch_requests (
    id BIGSERIAL PRIMARY KEY,
    batch_id BIGINT NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    
    -- Individual request tracking
    request_id VARCHAR(64) NOT NULL UNIQUE,
    status VARCHAR(32) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed'
    
    -- Request details
    model_id VARCHAR(128) NOT NULL,
    provider_profile_id BIGINT NOT NULL REFERENCES provider_profiles(id),
    
    -- Input/output
    input_tokens BIGINT DEFAULT 0,
    output_tokens BIGINT DEFAULT 0,
    cost_usd NUMERIC(12,4) DEFAULT 0,
    
    -- Error handling
    error_code VARCHAR(64),
    error_message TEXT,
    
    -- Timing
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ DEFAULT NULL,
    
    -- Retry tracking
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    
    -- Original request payload (for replay)
    request_payload JSONB NOT NULL,
    response_payload JSONB
);

-- Indexes for batches
CREATE INDEX idx_batches_public_id ON batches(public_id);
CREATE INDEX idx_batches_user_id ON batches(user_id);
CREATE INDEX idx_batches_team_id ON batches(team_id);
CREATE INDEX idx_batches_status ON batches(status);
CREATE INDEX idx_batches_operation_type ON batches(operation_type);
CREATE INDEX idx_batches_created_at ON batches(created_at);
CREATE INDEX idx_batches_completed_at ON batches(completed_at);

-- Indexes for batch_requests
CREATE INDEX idx_batch_requests_batch_id ON batch_requests(batch_id);
CREATE INDEX idx_batch_requests_request_id ON batch_requests(request_id);
CREATE INDEX idx_batch_requests_status ON batch_requests(status);
CREATE INDEX idx_batch_requests_model_id ON batch_requests(model_id);
CREATE INDEX idx_batch_requests_created_at ON batch_requests(created_at);
CREATE INDEX idx_batch_requests_retry_needed ON batch_requests(retry_count, max_retries) 
    WHERE status = 'failed' AND retry_count < max_retries;

-- Composite indexes for common queries
CREATE INDEX idx_batches_user_status ON batches(user_id, status);
CREATE INDEX idx_batch_requests_batch_status ON batch_requests(batch_id, status);
