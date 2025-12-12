-- Migration: Optimize spark scoring and indexing
-- Date: 2025-12-12
-- Purpose: 
-- 1. Enable intarray extension for faster array operations and indexes
-- 2. Add GIN indexes for spark/factor ARRAYS using gin__int_ops
-- 3. Add optimized scoring function

-- Enable intarray extension
CREATE EXTENSION IF NOT EXISTS intarray;

-- Add GIN indexes for fast filtering (&& operator)
CREATE INDEX IF NOT EXISTS idx_inheritance_blue_sparks_gin ON inheritance USING gin (blue_sparks gin__int_ops);
CREATE INDEX IF NOT EXISTS idx_inheritance_pink_sparks_gin ON inheritance USING gin (pink_sparks gin__int_ops);
CREATE INDEX IF NOT EXISTS idx_inheritance_green_sparks_gin ON inheritance USING gin (green_sparks gin__int_ops);
CREATE INDEX IF NOT EXISTS idx_inheritance_white_sparks_gin ON inheritance USING gin (white_sparks gin__int_ops);

-- main_white_factors is an array, so GIN is appropriate
CREATE INDEX IF NOT EXISTS idx_inheritance_main_white_factors_gin ON inheritance USING gin (main_white_factors gin__int_ops);

-- main_blue/pink/green_factors are SCALARS (int), so we use B-Tree (default)
CREATE INDEX IF NOT EXISTS idx_inheritance_main_blue_factors ON inheritance(main_blue_factors);
CREATE INDEX IF NOT EXISTS idx_inheritance_main_pink_factors ON inheritance(main_pink_factors);
CREATE INDEX IF NOT EXISTS idx_inheritance_main_green_factors ON inheritance(main_green_factors);

-- Optimized scoring function

CREATE OR REPLACE FUNCTION calculate_sparks_score(sparks int[], factor_ids int[]) RETURNS int AS $$
DECLARE
    score int := 0;
    spark int;
    factor int;
    seen_factors int[] := '{}';
BEGIN
    IF sparks IS NULL OR factor_ids IS NULL OR array_length(factor_ids, 1) IS NULL THEN
        RETURN 0;
    END IF;

    FOREACH spark IN ARRAY sparks LOOP
        factor := spark / 10;
        
        -- Check if factor is in factor_ids
        IF factor = ANY(factor_ids) THEN
            score := score + (spark % 10);
            
            -- Check if we already counted this factor
            IF NOT (factor = ANY(seen_factors)) THEN
                seen_factors := array_append(seen_factors, factor);
                score := score + 100; -- Add 100 for the distinct factor
            END IF;
        END IF;
    END LOOP;
    
    RETURN score;
END;
$$ LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE;
