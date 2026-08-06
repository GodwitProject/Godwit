DROP INDEX IF EXISTS idx_request_logs_tags;
ALTER TABLE request_logs DROP COLUMN tags;
