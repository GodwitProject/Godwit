ALTER TABLE models
    DROP COLUMN IF EXISTS provider_profile_id,
    DROP COLUMN IF EXISTS capability,
    DROP COLUMN IF EXISTS pricing;

DROP TABLE IF EXISTS provider_profiles;
