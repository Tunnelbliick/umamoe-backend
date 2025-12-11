-- Migration: Add missing indexes for search optimization
-- Date: 2025-12-08
-- Purpose: Add indexes for rank, rarity, chara_id, and trainer name to speed up searches

-- Enable pg_trgm extension for efficient text search (ILIKE)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Indexes for Inheritance filtering
CREATE INDEX IF NOT EXISTS idx_inheritance_parent_rank ON inheritance(parent_rank);
CREATE INDEX IF NOT EXISTS idx_inheritance_parent_rarity ON inheritance(parent_rarity);
CREATE INDEX IF NOT EXISTS idx_inheritance_main_chara_id ON inheritance(main_chara_id);
CREATE INDEX IF NOT EXISTS idx_inheritance_main_parent_id ON inheritance(main_parent_id);
CREATE INDEX IF NOT EXISTS idx_inheritance_parent_left_id ON inheritance(parent_left_id);
CREATE INDEX IF NOT EXISTS idx_inheritance_parent_right_id ON inheritance(parent_right_id);

-- Index for Trainer Name search (Trigram index for ILIKE '%...%')
CREATE INDEX IF NOT EXISTS idx_trainer_name_trgm ON trainer USING gin (name gin_trgm_ops);

-- Analyze tables to update statistics
ANALYZE inheritance;
ANALYZE trainer;
