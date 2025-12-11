-- Optimize stats queries with indexes and materialized views
-- This migration dramatically speeds up COUNT(*) queries

-- Index for daily_stats date lookups (7-day average)
CREATE INDEX IF NOT EXISTS idx_daily_stats_date 
ON daily_stats(date DESC);

-- ==============================================================================
-- MATERIALIZED VIEW FOR FAST TABLE COUNTS
-- ==============================================================================
-- This precomputes all COUNT(*) queries and refreshes them periodically
-- Orders of magnitude faster than counting millions of rows each time

-- Drop if exists first (for re-running migration)
DROP MATERIALIZED VIEW IF EXISTS stats_counts CASCADE;

CREATE MATERIALIZED VIEW stats_counts AS
SELECT 
  (SELECT COUNT(*) FROM trainer) as trainer_count,
  (SELECT COUNT(*) FROM circles) as circles_count,
  (SELECT COUNT(*) FROM team_stadium) as team_stadium_count,
  (SELECT COUNT(*) FROM inheritance) as inheritance_count,
  (SELECT COUNT(*) FROM support_card) as support_card_count,
  (SELECT COALESCE(AVG(unique_visitors::float8), 0) 
   FROM daily_stats 
   WHERE date >= CURRENT_DATE - INTERVAL '7 days') as unique_visitors_7_day,
  NOW() as last_refreshed;

-- Create a unique index to enable CONCURRENTLY refresh
-- Note: We need at least one unique index for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX idx_stats_counts_singleton ON stats_counts((1));

-- Initial refresh
REFRESH MATERIALIZED VIEW stats_counts;

-- ==============================================================================
-- FUNCTION TO REFRESH STATS COUNTS
-- ==============================================================================
-- Call this periodically (e.g., every hour via cron or background task)

CREATE OR REPLACE FUNCTION refresh_stats_counts()
RETURNS void AS $$
BEGIN
  REFRESH MATERIALIZED VIEW CONCURRENTLY stats_counts;
END;
$$ LANGUAGE plpgsql;

-- ==============================================================================
-- OPTIMIZE SEARCH RESULT COUNTS
-- ==============================================================================
-- Add indexes to speed up common search patterns

-- Index for follower_num filters (very common in searches)
CREATE INDEX IF NOT EXISTS idx_trainer_follower_num 
ON trainer(follower_num) 
WHERE follower_num IS NULL OR follower_num < 1000;

-- Composite index for inheritance searches
CREATE INDEX IF NOT EXISTS idx_inheritance_search 
ON inheritance(account_id, main_parent_id, parent_left_id, parent_right_id);

-- Composite index for support card searches
CREATE INDEX IF NOT EXISTS idx_support_card_search 
ON support_card(account_id, support_card_id, limit_break_count);

-- Index for inheritance parent filters
CREATE INDEX IF NOT EXISTS idx_inheritance_parents 
ON inheritance(main_parent_id, parent_left_id, parent_right_id) 
WHERE main_parent_id IS NOT NULL;

-- GIN indexes for array searches (sparks)
CREATE INDEX IF NOT EXISTS idx_inheritance_blue_sparks 
ON inheritance USING GIN (blue_sparks);

CREATE INDEX IF NOT EXISTS idx_inheritance_pink_sparks 
ON inheritance USING GIN (pink_sparks);

CREATE INDEX IF NOT EXISTS idx_inheritance_green_sparks 
ON inheritance USING GIN (green_sparks);

CREATE INDEX IF NOT EXISTS idx_inheritance_white_sparks 
ON inheritance USING GIN (white_sparks);

-- ==============================================================================
-- OPTIMIZED INDEXES FOR COMMON QUERY PATTERNS
-- ==============================================================================
-- These dramatically speed up the most common default searches

-- Index for inheritance search sorted by win_count (most common default query)
CREATE INDEX IF NOT EXISTS idx_inheritance_win_count_desc 
ON inheritance(win_count DESC, account_id ASC);

-- Index for trainer with follower filtering (used in almost every query)
CREATE INDEX IF NOT EXISTS idx_trainer_account_follower 
ON trainer(account_id, follower_num) 
WHERE follower_num IS NULL OR follower_num < 1000;

-- Composite index for inheritance joins with filtering
CREATE INDEX IF NOT EXISTS idx_inheritance_account_wins 
ON inheritance(account_id, win_count DESC, parent_rank DESC);

-- CRITICAL: Index for support card searches by experience (most common sort)
-- This is the PRIMARY index for support card queries sorted by experience
CREATE INDEX IF NOT EXISTS idx_support_card_exp_desc 
ON support_card(experience DESC, limit_break_count DESC, account_id ASC);

-- CRITICAL: Covering index for limit_break filtering + experience sorting
-- This single index handles ALL limit_break filters efficiently
-- PostgreSQL can use this for: WHERE limit_break >= X ORDER BY experience DESC
CREATE INDEX IF NOT EXISTS idx_support_card_lb_exp 
ON support_card(limit_break_count, experience DESC, account_id ASC) 
WHERE limit_break_count IS NOT NULL;

-- CRITICAL: Index for support_card_id + limit_break + experience queries
-- Handles: WHERE support_card_id = X AND limit_break >= Y ORDER BY experience DESC
CREATE INDEX IF NOT EXISTS idx_support_card_id_lb_exp 
ON support_card(support_card_id, limit_break_count, experience DESC, account_id ASC) 
WHERE support_card_id IS NOT NULL;

-- Index for support card searches sorted by limit_break
CREATE INDEX IF NOT EXISTS idx_support_card_limit_break 
ON support_card(limit_break_count DESC, experience DESC, account_id ASC);

-- Index for support card + trainer joins
CREATE INDEX IF NOT EXISTS idx_support_card_account 
ON support_card(account_id, support_card_id, limit_break_count, experience);

-- Composite index for inheritance + account lookups (optimizes joins)
CREATE INDEX IF NOT EXISTS idx_inheritance_account_id 
ON inheritance(account_id) 
WHERE inheritance_id IS NOT NULL;

-- Index for trainer account lookups with follower filtering
CREATE INDEX IF NOT EXISTS idx_trainer_account_id_filtered 
ON trainer(account_id) 
WHERE follower_num IS NULL OR follower_num < 1000;

-- ==============================================================================
-- USAGE NOTES
-- ==============================================================================
-- 1. Set up a cron job or background task to refresh stats every hour:
--    SELECT refresh_stats_counts();
--
-- 2. In your application, query the materialized view instead of counting:
--    SELECT trainer_count, circles_count FROM stats_counts;
--
-- 3. For search result counts, consider:
--    - Using EXPLAIN ANALYZE to verify index usage
--    - Caching results for common queries (already implemented in code)
--    - Using approximate counts for very large result sets

