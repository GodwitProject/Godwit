-- A public model id may be exposed through multiple provider profiles within the
-- same organization (e.g. OpenAI gpt-4o vs Azure OpenAI gpt-4o). Replace the
-- overly restrictive unique constraint with one that includes the profile.
ALTER TABLE models DROP CONSTRAINT IF EXISTS models_organization_id_public_id_key;
ALTER TABLE models ADD CONSTRAINT models_org_public_id_profile_unique UNIQUE (organization_id, public_id, provider_profile_id);
