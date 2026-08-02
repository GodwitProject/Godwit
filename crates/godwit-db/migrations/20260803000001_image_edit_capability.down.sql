ALTER TABLE models DROP CONSTRAINT chk_models_capabilities;

ALTER TABLE models ADD CONSTRAINT chk_models_capabilities
    CHECK (capabilities <@ ARRAY['chat','image_generation','video_generation','audio_tts','audio_stt','embedding']);
