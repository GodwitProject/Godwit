-- Backfill default pricing for models without pricing
UPDATE models 
SET pricing = jsonb_build_object(
    'input_price_per_million', 0.0,
    'output_price_per_million', 0.0
)
WHERE pricing IS NULL OR pricing = '{}'::jsonb;

-- Add NOT NULL constraint after backfill
ALTER TABLE models 
ALTER COLUMN pricing SET DEFAULT '{}'::jsonb;
