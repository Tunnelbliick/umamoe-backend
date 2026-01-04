-- Migration: Remove star sums trigger for better ingest performance
-- Date: 2026-01-04
-- Purpose: The ingestor will compute star sums itself to avoid trigger overhead

-- Drop the trigger (keep the function in case we need it later for manual recalculation)
DROP TRIGGER IF EXISTS update_star_sums ON inheritance;

-- Optionally drop the function too (uncomment if you don't need it)
-- DROP FUNCTION IF EXISTS calculate_star_sums();
