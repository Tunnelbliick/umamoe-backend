-- Migration: Optimize Query Performance
-- Date: 2026-01-04
-- Purpose: Address slow queries identified in production logs

-- ============================================================================
-- ISSUE 0: Slow cardinality(ARRAY(... INTERSECT ...)) pattern
-- Create a fast function to count array overlaps
-- ============================================================================

CREATE OR REPLACE FUNCTION count_array_overlap(arr int[], search_values int[])
RETURNS int AS $$
    SELECT COUNT(*)::int
    FROM unnest(arr) AS elem
    WHERE elem = ANY(search_values);
$$ LANGUAGE SQL IMMUTABLE PARALLEL SAFE;

-- ============================================================================
-- ISSUE 1: Affinity sorting with array access (affinity_scores[X])
-- The ORDER BY (COALESCE(i.affinity_scores[X], 0) + COALESCE(i.race_affinity, 0)) DESC
-- cannot use indexes. We need expression indexes for common player_chara_id values.
-- ============================================================================

-- Create expression index for base_affinity + race_affinity (default sort)
CREATE INDEX IF NOT EXISTS idx_inheritance_affinity_default 
ON inheritance ((COALESCE(base_affinity, 0) + COALESCE(race_affinity, 0)) DESC);

-- Create index on race_affinity for the addition
CREATE INDEX IF NOT EXISTS idx_inheritance_race_affinity 
ON inheritance (race_affinity DESC NULLS LAST);

-- ============================================================================
-- ISSUE 2: Trainer join optimization
-- The INNER JOIN trainer t ON i.account_id = t.account_id is slow
-- because follower_num filter prevents index usage
-- ============================================================================

-- Composite index covering the common filter + join pattern
CREATE INDEX IF NOT EXISTS idx_inheritance_account_rarity_win
ON inheritance (account_id, parent_rarity DESC, win_count DESC, white_count DESC);

-- Index on trainer for the follower filter with account_id
CREATE INDEX IF NOT EXISTS idx_trainer_account_follower
ON trainer (account_id) 
INCLUDE (name, follower_num, last_updated)
WHERE follower_num IS NULL OR follower_num < 1000;

-- ============================================================================
-- ISSUE 3: Circle GlobalRanks - expensive window function on every query
-- Solution: Materialized view for circle rankings, refreshed periodically
-- ============================================================================

-- Drop if exists to allow recreation
DROP MATERIALIZED VIEW IF EXISTS circle_live_ranks;

CREATE MATERIALIZED VIEW circle_live_ranks AS
SELECT 
    circle_id,
    RANK() OVER (ORDER BY monthly_point DESC NULLS LAST) as live_rank,
    RANK() OVER (ORDER BY yesterday_points DESC NULLS LAST) as live_yesterday_rank
FROM circles
WHERE (archived IS NULL OR archived = false)
  AND last_updated >= ((date_trunc('month', CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo') + interval '12 hours') AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Europe/Berlin'
  AND last_updated < ((date_trunc('month', CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo') + interval '1 month' + interval '12 hours') AT TIME ZONE 'Asia/Tokyo') AT TIME ZONE 'Europe/Berlin';

-- Index for fast lookups
CREATE UNIQUE INDEX idx_circle_live_ranks_id ON circle_live_ranks (circle_id);
CREATE INDEX idx_circle_live_ranks_rank ON circle_live_ranks (live_rank);

-- ============================================================================
-- ISSUE 4: Support card join optimization (1:1 relationship)
-- Since it's 1:1, we can index the join column better
-- ============================================================================

-- Ensure support_card has proper index for account_id lookup
CREATE UNIQUE INDEX IF NOT EXISTS idx_support_card_account_unique
ON support_card (account_id);

-- Composite index for support card filtering
CREATE INDEX IF NOT EXISTS idx_support_card_filter
ON support_card (account_id, support_card_id, limit_break_count, experience);

-- ============================================================================
-- ISSUE 5: Spark array overlap (&&) optimization
-- The GIN indexes should help, but we need to ensure they're being used
-- ============================================================================

-- Ensure GIN indexes exist with proper operator class
CREATE INDEX IF NOT EXISTS idx_inheritance_blue_sparks_gin 
ON inheritance USING GIN (blue_sparks);

CREATE INDEX IF NOT EXISTS idx_inheritance_pink_sparks_gin 
ON inheritance USING GIN (pink_sparks);

CREATE INDEX IF NOT EXISTS idx_inheritance_green_sparks_gin 
ON inheritance USING GIN (green_sparks);

CREATE INDEX IF NOT EXISTS idx_inheritance_white_sparks_gin 
ON inheritance USING GIN (white_sparks);

-- ============================================================================
-- ISSUE 6: Circle search with ILIKE - ensure trigram indexes are used
-- ============================================================================

-- Trigram index for circle name (should exist but confirm)
CREATE INDEX IF NOT EXISTS idx_circles_name_trgm 
ON circles USING gin (name gin_trgm_ops);

-- ============================================================================
-- REFRESH FUNCTION: For the materialized view
-- ============================================================================

CREATE OR REPLACE FUNCTION refresh_circle_live_ranks()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY circle_live_ranks;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- ANALYZE: Update statistics for query planner
-- ============================================================================

ANALYZE inheritance;
ANALYZE trainer;
ANALYZE support_card;
ANALYZE circles;
