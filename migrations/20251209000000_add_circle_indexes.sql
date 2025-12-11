-- Migration: Add indexes for circle search and ranking
-- Date: 2025-12-09
-- Purpose: Optimize circle list query and global ranking calculation

-- Enable pg_trgm extension if not already enabled
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- 1. Indexes for Global Ranking CTE
-- We filter by last_updated and archived, and sort by monthly_point/yesterday_points
CREATE INDEX IF NOT EXISTS idx_circles_ranking_monthly 
ON circles (last_updated, monthly_point DESC) 
WHERE (archived IS NULL OR archived = false);

CREATE INDEX IF NOT EXISTS idx_circles_ranking_yesterday 
ON circles (last_updated, yesterday_points DESC) 
WHERE (archived IS NULL OR archived = false);

-- 2. Index for Circle Name Search (Trigram)
CREATE INDEX IF NOT EXISTS idx_circles_name_trgm ON circles USING gin (name gin_trgm_ops);

-- 3. Indexes for Circle Member Fans (Subquery)
-- Used for filtering by circle_id, year, month and joining with trainer
CREATE INDEX IF NOT EXISTS idx_circle_member_fans_search 
ON circle_member_fans_monthly (circle_id, year, month);

CREATE INDEX IF NOT EXISTS idx_circle_member_fans_viewer 
ON circle_member_fans_monthly (viewer_id);

-- 4. Index for Leader Viewer ID
CREATE INDEX IF NOT EXISTS idx_circles_leader_viewer_id ON circles(leader_viewer_id);

-- Analyze to update stats
ANALYZE circles;
ANALYZE circle_member_fans_monthly;
