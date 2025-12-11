-- Clean up redundant and incorrectly ordered indexes
-- Keep only the essential ones with correct ordering

-- Drop all the redundant/wrong indexes
DROP INDEX IF EXISTS idx_support_card_search;
DROP INDEX IF EXISTS idx_support_card_account_exp;
DROP INDEX IF EXISTS idx_support_card_exp_desc;
DROP INDEX IF EXISTS idx_support_card_limit_break;
DROP INDEX IF EXISTS idx_support_card_account;
DROP INDEX IF EXISTS idx_support_card_exp_lb;
DROP INDEX IF EXISTS idx_support_card_lb_exact_exp;
DROP INDEX IF EXISTS idx_support_card_exp_account;

-- Create the CORRECT index with DESC on experience
-- This is the PRIMARY index for: ORDER BY experience DESC with any limit_break filter
CREATE INDEX IF NOT EXISTS idx_support_card_exp_desc_account 
ON support_card(experience DESC, account_id ASC);

-- Keep the specialized index for support_card_id queries (this one is good!)
-- idx_support_card_id_lb_exp already exists and is correct

-- Update statistics
ANALYZE support_card;
