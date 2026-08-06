-- Rollback fallback tracking columns
DROP INDEX IF EXISTS idx_request_logs_fallback_triggered;
ALTER TABLE request_logs 
    DROP COLUMN IF EXISTS attempt_number,
    DROP COLUMN IF EXISTS fallback_triggered;
