ALTER TABLE api_keys DROP CONSTRAINT api_keys_user_id_fkey;
ALTER TABLE api_keys ADD CONSTRAINT api_keys_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE request_logs DROP CONSTRAINT request_logs_user_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;

-- Not in the original brief, added after the TDD step exposed it: deleting a
-- user cascades to delete their api_keys (above), but request_logs.api_key_id
-- still referenced those rows with the default NO ACTION, so the api_keys
-- cascade-delete itself failed with an FK violation. SET NULL isn't viable
-- here (unlike request_logs.user_id above): historical request_logs rows are
-- looked up/reported by which api_key made the request, so the value needs to
-- survive the key's deletion, not be nulled. There is no ON DELETE mode that
-- both deletes the referenced api_keys row and leaves the referencing column
-- unchanged, so the constraint is dropped outright: api_key_id becomes a
-- best-effort historical pointer (may go stale/orphaned once its api_keys row
-- is gone) rather than an enforced FK. No code in the repo joins through this
-- column today (grep confirmed write-only usage in godwit-api/src/proxy.rs).
ALTER TABLE request_logs DROP CONSTRAINT request_logs_api_key_id_fkey;
