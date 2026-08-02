ALTER TABLE models DROP CONSTRAINT IF EXISTS models_org_public_id_profile_unique;
ALTER TABLE models ADD CONSTRAINT models_organization_id_public_id_key UNIQUE (organization_id, public_id);
