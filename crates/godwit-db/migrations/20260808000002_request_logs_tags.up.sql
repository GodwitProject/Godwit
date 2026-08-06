-- File: crates/godwit-db/migrations/20260808000002_request_logs_tags.up.sql
ALTER TABLE request_logs ADD COLUMN tags TEXT[] DEFAULT '{}';
CREATE INDEX idx_request_logs_tags ON request_logs USING GIN(tags);
