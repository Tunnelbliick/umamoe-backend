-- Create an index optimized for backward scans on experience
-- This allows PostgreSQL to scan from highest experience down and filter limit_break as it goes
-- Works for ALL limit_break values (0-4) - just scans backwards and filters inline

-- This is the PRIMARY index for support card queries sorted by experience
CREATE INDEX IF NOT EXISTS idx_support_card_exp_account 
ON support_card(experience DESC, account_id ASC);

-- PostgreSQL will use this index by:
-- 1. Scanning backwards from highest experience
-- 2. Checking limit_break_count >= X for each row (cheap filter)
-- 3. Stopping after finding LIMIT rows
-- This is MUCH faster than sorting 289k+ rows

-- Update statistics
ANALYZE support_card;
