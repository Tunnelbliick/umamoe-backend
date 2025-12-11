-- Query 1: As logged (Any support card with LB >= 4)
EXPLAIN ANALYZE SELECT COUNT(*) 
FROM inheritance i
INNER JOIN trainer t ON i.account_id = t.account_id
LEFT JOIN LATERAL (
    SELECT support_card_id
    FROM support_card
    WHERE account_id = i.account_id
    AND limit_break_count >= 4
    LIMIT 1
) sc ON true
WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
 AND sc.support_card_id IS NOT NULL 
 AND i.parent_rarity >= 2 
 AND i.win_count >= 0 
 AND i.white_count >= 0 
 AND i.main_white_count >= 0 
 AND (t.follower_num IS NULL OR t.follower_num <= 1000);

-- Query 2: With specific support card (Simulating what the user wants)
-- Assuming support_card_id = 30001 (Kitasan Black, common)
EXPLAIN ANALYZE SELECT COUNT(*) 
FROM inheritance i
INNER JOIN trainer t ON i.account_id = t.account_id
LEFT JOIN LATERAL (
    SELECT support_card_id
    FROM support_card
    WHERE account_id = i.account_id
    AND support_card_id = 30001
    AND limit_break_count >= 4
    LIMIT 1
) sc ON true
WHERE (t.follower_num IS NULL OR t.follower_num < 1000)
 AND sc.support_card_id IS NOT NULL 
 AND i.parent_rarity >= 2 
 AND i.win_count >= 0 
 AND i.white_count >= 0 
 AND i.main_white_count >= 0 
 AND (t.follower_num IS NULL OR t.follower_num <= 1000);
