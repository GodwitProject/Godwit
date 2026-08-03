ALTER TABLE api_keys DROP CONSTRAINT api_keys_user_id_fkey;
ALTER TABLE api_keys ADD CONSTRAINT api_keys_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id);

ALTER TABLE request_logs DROP CONSTRAINT request_logs_user_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id);

ALTER TABLE request_logs DROP CONSTRAINT request_logs_api_key_id_fkey;
ALTER TABLE request_logs ADD CONSTRAINT request_logs_api_key_id_fkey
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id);
