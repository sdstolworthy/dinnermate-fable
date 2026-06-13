-- Meal-time selection: when the group plans to eat (UTC); NULL = anytime.
ALTER TABLE rooms ADD COLUMN eat_at TIMESTAMPTZ;
