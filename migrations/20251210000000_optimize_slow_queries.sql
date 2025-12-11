-- Migration: Add missing indexes for complex search queries
-- Date: 2025-12-10
-- Purpose: Optimize slow queries identified in logs involving inheritance filtering and sorting

-- 1. Composite index for inheritance filtering (rarity, win_count, white_count)
-- This covers the WHERE clause: i.parent_rarity >= $1 AND i.win_count >= $2 AND i.white_count >= $3
CREATE INDEX IF NOT EXISTS idx_inheritance_filter_composite 
ON inheritance (parent_rarity DESC, win_count DESC, white_count DESC, main_white_count DESC);

-- 2. Index for main_chara_id exclusion
-- Used in: i.main_chara_id != $1
CREATE INDEX IF NOT EXISTS idx_inheritance_main_chara_id_hash 
ON inheritance USING HASH (main_chara_id);

-- 3. Optimized GIN indexes for spark array filtering
-- The existing GIN indexes might not be optimal for the specific && operator usage
-- We ensure these exist and are analyzed
-- (Existing indexes: idx_inheritance_blue_sparks, idx_inheritance_pink_sparks, etc.)

-- 4. Composite index for Support Card sorting + Trainer join
-- Used in: ORDER BY sc.experience DESC NULLS LAST, t.account_id ASC
CREATE INDEX IF NOT EXISTS idx_support_card_experience_account 
ON support_card (experience DESC NULLS LAST, account_id ASC);

-- 5. Composite index for Trainer follower filtering + account_id
-- Used in: WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
-- Existing index idx_trainer_follower_num covers the WHERE, but we need account_id for the JOIN
CREATE INDEX IF NOT EXISTS idx_trainer_follower_account_composite 
ON trainer (account_id, follower_num) 
WHERE follower_num IS NULL OR follower_num < 1000;

-- 6. Analyze tables to ensure query planner uses new indexes
ANALYZE inheritance;
ANALYZE support_card;
ANALYZE trainer;