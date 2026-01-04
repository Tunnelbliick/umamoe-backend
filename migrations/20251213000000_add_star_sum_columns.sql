-- Add columns for storing the sum of stars for each spark type
ALTER TABLE inheritance
ADD COLUMN IF NOT EXISTS blue_stars_sum INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS pink_stars_sum INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS green_stars_sum INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS white_stars_sum INTEGER DEFAULT 0;

-- Create function to calculate sums
CREATE OR REPLACE FUNCTION calculate_star_sums() RETURNS TRIGGER AS $$
BEGIN
    NEW.blue_stars_sum := COALESCE((SELECT SUM(x % 10) FROM unnest(NEW.blue_sparks) AS x), 0);
    NEW.pink_stars_sum := COALESCE((SELECT SUM(x % 10) FROM unnest(NEW.pink_sparks) AS x), 0);
    NEW.green_stars_sum := COALESCE((SELECT SUM(x % 10) FROM unnest(NEW.green_sparks) AS x), 0);
    NEW.white_stars_sum := COALESCE((SELECT SUM(x % 10) FROM unnest(NEW.white_sparks) AS x), 0);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to automatically update sums on insert or update
DROP TRIGGER IF EXISTS update_star_sums ON inheritance;
CREATE TRIGGER update_star_sums
BEFORE INSERT OR UPDATE OF blue_sparks, pink_sparks, green_sparks, white_sparks ON inheritance
FOR EACH ROW
EXECUTE FUNCTION calculate_star_sums();

-- Populate the columns for existing rows
UPDATE inheritance SET
    blue_stars_sum = COALESCE((SELECT SUM(x % 10) FROM unnest(blue_sparks) AS x), 0),
    pink_stars_sum = COALESCE((SELECT SUM(x % 10) FROM unnest(pink_sparks) AS x), 0),
    green_stars_sum = COALESCE((SELECT SUM(x % 10) FROM unnest(green_sparks) AS x), 0),
    white_stars_sum = COALESCE((SELECT SUM(x % 10) FROM unnest(white_sparks) AS x), 0);

-- Add indexes for efficient filtering
CREATE INDEX IF NOT EXISTS idx_inheritance_blue_stars_sum ON inheritance(blue_stars_sum);
CREATE INDEX IF NOT EXISTS idx_inheritance_pink_stars_sum ON inheritance(pink_stars_sum);
CREATE INDEX IF NOT EXISTS idx_inheritance_green_stars_sum ON inheritance(green_stars_sum);
CREATE INDEX IF NOT EXISTS idx_inheritance_white_stars_sum ON inheritance(white_stars_sum);
