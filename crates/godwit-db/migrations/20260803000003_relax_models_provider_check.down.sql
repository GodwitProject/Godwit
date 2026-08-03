-- Restore the original two-value constraint from 20260801000001_initial.sql.
-- Rows whose provider is one of the five protocols added by the up-migration must be
-- removed first, otherwise re-adding the narrower constraint fails validation.

DELETE FROM models WHERE provider NOT IN ('openai','anthropic');

ALTER TABLE models DROP CONSTRAINT IF EXISTS models_provider_check;

ALTER TABLE models ADD CONSTRAINT models_provider_check
    CHECK (provider IN ('openai','anthropic'));
