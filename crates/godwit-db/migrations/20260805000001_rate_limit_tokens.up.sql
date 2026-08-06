ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS rate_limit_tokens_per_minute INTEGER;
ALTER TABLE organizations ADD COLUMN IF NOT EXISTS rate_limit_tokens_per_minute INTEGER;
