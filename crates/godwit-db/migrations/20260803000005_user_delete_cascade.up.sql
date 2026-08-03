ALTER TABLE api_keys DROP CONSTRAINT api_keys_user_id_fkey;
ALTER TABLE api_keys ADD CONSTRAINT api_keys_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE request_logs DROP CONSTRAINT request_logs_user_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;

-- Not in the original brief, added after the TDD step exposed it: deleting a
-- user cascades to delete their api_keys (above), but request_logs.api_key_id
-- still referenced those rows with the default NO ACTION, so the api_keys
-- cascade-delete itself failed with an FK violation. Mirror the treatment
-- already used for request_logs.user_id above: SET NULL preserves the
-- request_logs row (the audit trail) while letting the referenced api_keys
-- row actually be deleted, at the cost of losing which specific key made a
-- historical request once that key is deleted.
ALTER TABLE request_logs DROP CONSTRAINT request_logs_api_key_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_api_key_id_fkey
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE SET NULL;
