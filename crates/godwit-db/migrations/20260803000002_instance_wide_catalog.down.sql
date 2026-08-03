ALTER TABLE models DROP CONSTRAINT models_provider_profile_id_public_id_key;
ALTER TABLE models ADD COLUMN organization_id UUID REFERENCES organizations(id);

ALTER TABLE provider_profiles DROP CONSTRAINT provider_profiles_name_key;
ALTER TABLE provider_profiles
    DROP COLUMN allow_wildcard,
    ADD COLUMN organization_id UUID REFERENCES organizations(id);

-- Note: organization_id values are not recoverable after the up-migration's
-- de-duplication; this down-migration restores the columns as nullable only.
