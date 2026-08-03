-- De-duplicate before dropping organization_id: no production tenants exist yet,
-- so we keep the first row per (name) / (provider_profile_id, public_id) and drop the rest.

-- Repoint models referencing a to-be-removed duplicate provider_profile (same name,
-- not the lowest ctid for that name) at the surviving row *before* removing the
-- duplicates, since models.provider_profile_id has a FK to provider_profiles.
WITH survivors AS (
    SELECT DISTINCT ON (name) id, name
    FROM provider_profiles
    ORDER BY name, ctid
)
UPDATE models m
SET provider_profile_id = s.id
FROM provider_profiles p
JOIN survivors s ON s.name = p.name
WHERE m.provider_profile_id = p.id
  AND p.id <> s.id;

WITH survivors AS (
    SELECT DISTINCT ON (name) id, name
    FROM provider_profiles
    ORDER BY name, ctid
)
DELETE FROM provider_profiles p
WHERE NOT EXISTS (SELECT 1 FROM survivors s WHERE s.id = p.id);

-- Now that models are repointed at survivors, collapse any (provider_profile_id, public_id)
-- duplicates this repointing may have created (e.g. the same public_id under two orgs'
-- profiles of the same name), keeping the lowest-ctid row.
DELETE FROM models m
USING models m2
WHERE m.provider_profile_id = m2.provider_profile_id
  AND m.public_id = m2.public_id
  AND m.ctid > m2.ctid;

ALTER TABLE provider_profiles
    DROP COLUMN organization_id,
    ADD COLUMN allow_wildcard BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE provider_profiles ADD CONSTRAINT provider_profiles_name_key UNIQUE (name);

ALTER TABLE models DROP COLUMN organization_id;
ALTER TABLE models ADD CONSTRAINT models_provider_profile_id_public_id_key UNIQUE (provider_profile_id, public_id);
