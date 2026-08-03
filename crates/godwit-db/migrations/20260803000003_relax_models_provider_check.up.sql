-- The initial migration constrained models.provider to ('openai','anthropic') via an
-- inline column CHECK (auto-named models_provider_check by Postgres). Five of the seven
-- protocols now backed by real adapters (gemini, vllm, sglang, llama_cpp, ollama) could
-- therefore never be inserted into the catalog. Relax the constraint to cover all seven.
--
-- Note: models.provider itself is redundant now that models.provider_profile_id points at
-- a provider_profiles row carrying the protocol; dropping the column is deliberately left
-- to a separate change.

ALTER TABLE models DROP CONSTRAINT IF EXISTS models_provider_check;

ALTER TABLE models ADD CONSTRAINT models_provider_check
    CHECK (provider IN ('openai','anthropic','gemini','vllm','sglang','llama_cpp','ollama'));
