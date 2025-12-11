-- Add specialized index for support card range queries with ORDER BY experience
-- This fixes slow queries like: WHERE limit_break_count >= 4 ORDER BY experience DESC

-- Drop the partial index (we'll replace it with a better full index)
DROP INDEX IF EXISTS idx_support_card_lb_exp;

-- Create an index optimized for: ORDER BY experience DESC with range filters on limit_break
-- PostgreSQL can use this for backward index scans and filter limit_break in the index
CREATE INDEX IF NOT EXISTS idx_support_card_exp_lb 
ON support_card(experience DESC, limit_break_count, account_id ASC);

-- Also create index for queries that filter by limit_break first, then sort
-- This handles: WHERE limit_break_count = 4 ORDER BY experience DESC
CREATE INDEX IF NOT EXISTS idx_support_card_lb_exact_exp
ON support_card(limit_break_count, experience DESC, account_id ASC);

-- ANALYZE to update statistics
ANALYZE support_card;
