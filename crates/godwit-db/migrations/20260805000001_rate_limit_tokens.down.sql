ALTER TABLE api_keys DROP COLUMN IF EXISTS rate_limit_tokens_per_minute;
ALTER TABLE organizations DROP COLUMN IF EXISTS rate_limit_tokens_per_minute;
