ALTER TABLE models DROP CONSTRAINT IF EXISTS chk_models_capabilities;

ALTER TABLE models ADD COLUMN IF NOT EXISTS capability TEXT NOT NULL DEFAULT 'chat';

UPDATE models SET capability = capabilities[1] WHERE TRUE;

ALTER TABLE models DROP COLUMN IF EXISTS capabilities;
