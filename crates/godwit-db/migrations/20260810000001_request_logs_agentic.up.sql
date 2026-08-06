-- File: crates/godwit-db/migrations/20260810000001_request_logs_agentic.up.sql
ALTER TABLE request_logs ADD COLUMN tool_calls_count INTEGER;
ALTER TABLE request_logs ADD COLUMN agentic_iteration INTEGER;
