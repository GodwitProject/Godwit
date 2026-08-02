CREATE TABLE provider_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    protocol TEXT NOT NULL,
    base_url TEXT,
    auth JSONB NOT NULL DEFAULT '{}',
    config JSONB NOT NULL DEFAULT '{}',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

ALTER TABLE models
    ADD COLUMN IF NOT EXISTS provider_profile_id UUID REFERENCES provider_profiles(id),
    ADD COLUMN IF NOT EXISTS capability TEXT NOT NULL DEFAULT 'chat',
    ADD COLUMN IF NOT EXISTS pricing JSONB NOT NULL DEFAULT '{}',
    -- config already exists on models from the initial migration; this is a no-op for existing databases.
    ADD COLUMN IF NOT EXISTS config JSONB NOT NULL DEFAULT '{}';

ALTER TABLE models
    ADD CONSTRAINT chk_models_capability CHECK (capability IN ('chat', 'image_generation', 'video_generation', 'audio_tts', 'audio_stt', 'embedding'));

CREATE INDEX idx_models_provider_profile_id ON models(provider_profile_id);

-- Backfill default profiles from existing models.provider values.
INSERT INTO provider_profiles (organization_id, name, protocol, base_url, auth, config, enabled)
SELECT DISTINCT
    organization_id,
    provider AS name,
    provider AS protocol,
    CASE provider
        WHEN 'openai' THEN 'https://api.openai.com/v1'
        WHEN 'anthropic' THEN 'https://api.anthropic.com'
        ELSE NULL
    END AS base_url,
    '{}'::jsonb AS auth,
    '{}'::jsonb AS config,
    true AS enabled
FROM models;

-- Backfill models.provider_profile_id.
UPDATE models
SET provider_profile_id = provider_profiles.id
FROM provider_profiles
WHERE models.organization_id = provider_profiles.organization_id
  AND models.provider = provider_profiles.name;

ALTER TABLE models ALTER COLUMN provider_profile_id SET NOT NULL;
