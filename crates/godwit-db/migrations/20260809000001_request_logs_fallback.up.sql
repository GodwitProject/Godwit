-- Add fallback tracking columns to request_logs
ALTER TABLE request_logs 
    ADD COLUMN attempt_number INTEGER DEFAULT 1,
    ADD COLUMN fallback_triggered BOOLEAN DEFAULT FALSE;

-- Index for querying fallback-heavy requests
CREATE INDEX idx_request_logs_fallback_triggered ON request_logs(fallback_triggered) 
    WHERE fallback_triggered = TRUE;
