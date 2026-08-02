ALTER TABLE models ADD COLUMN IF NOT EXISTS capabilities TEXT[] NOT NULL DEFAULT ARRAY['chat'];

UPDATE models SET capabilities = ARRAY[capability] WHERE TRUE;

ALTER TABLE models DROP COLUMN IF EXISTS capability;

ALTER TABLE models ADD CONSTRAINT chk_models_capabilities
    CHECK (capabilities <@ ARRAY['chat','image_generation','video_generation','audio_tts','audio_stt','embedding']);
