-- Migration: Optimize Circle Join Performance
-- Date: 2025-12-11
-- Purpose: Fix type cast issues preventing index usage in circle searches

-- ============================================================================
-- PROBLEM: leader_viewer_id::text = t.account_id prevents index usage
-- SOLUTION: Create expression indexes on the casted values
-- ============================================================================

-- Index for joining circles to trainer via leader_viewer_id
-- This allows the join circles.leader_viewer_id::text = trainer.account_id to use an index
CREATE INDEX IF NOT EXISTS idx_circles_leader_viewer_text 
ON circles ((leader_viewer_id::text));

-- Index for joining circle_member_fans_monthly to trainer via viewer_id
CREATE INDEX IF NOT EXISTS idx_circle_member_fans_viewer_text 
ON circle_member_fans_monthly ((viewer_id::text));

-- ============================================================================
-- ADDITIONAL OPTIMIZATION: Composite index for member search
-- ============================================================================

-- Composite index for the member name search subquery
-- Covers: WHERE year = X AND month = Y with circle_id output
CREATE INDEX IF NOT EXISTS idx_circle_member_fans_ym_circle 
ON circle_member_fans_monthly (year, month, circle_id);

-- ============================================================================
-- INDEX FOR CIRCLE LIST BASE FILTER
-- ============================================================================

-- Index for the common WHERE clause in circle list queries
-- Covers: last_updated range + archived filter + monthly_rank sort
CREATE INDEX IF NOT EXISTS idx_circles_list_filter 
ON circles (last_updated, monthly_rank) 
WHERE (archived IS NULL OR archived = false);

-- Analyze to update statistics
ANALYZE circles;
ANALYZE circle_member_fans_monthly;
ANALYZE trainer;
