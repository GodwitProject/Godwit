DROP TABLE IF EXISTS password_reset_tokens;
DROP TABLE IF EXISTS password_history;
ALTER TABLE users
  DROP COLUMN IF EXISTS must_change_password,
  DROP COLUMN IF EXISTS password_expires_at,
  DROP COLUMN IF EXISTS password_changed_at;
