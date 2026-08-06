ALTER TABLE api_keys ADD COLUMN allowed_models TEXT[] NOT NULL DEFAULT '{}';
