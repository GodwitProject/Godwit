-- Downgrade: drop the new composite unique constraint.
-- We do not re-add the old UNIQUE(organization_id, public_id) constraint
-- because data migrated under the new schema may legitimately contain
-- duplicate public_ids across provider profiles. Re-adding it would fail.
ALTER TABLE models DROP CONSTRAINT IF EXISTS models_org_public_id_profile_unique;
