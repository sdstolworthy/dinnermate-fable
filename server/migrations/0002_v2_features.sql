CREATE TABLE list_members (
    list_id UUID NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (list_id, user_id)
);
CREATE INDEX idx_list_members_user ON list_members (user_id);
INSERT INTO list_members (list_id, user_id, joined_at)
    SELECT id, owner_user_id, created_at FROM lists;

ALTER TABLE room_restaurants
    ADD COLUMN hours JSONB,
    ADD COLUMN utc_offset_minutes INTEGER;

CREATE TABLE restaurant_details_cache (
    restaurant_id TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL
);
