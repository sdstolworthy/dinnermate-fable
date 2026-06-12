CREATE TABLE rooms (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT,
    location_lat DOUBLE PRECISION NOT NULL,
    location_lng DOUBLE PRECISION NOT NULL,
    location_label TEXT NOT NULL,
    radius_m INTEGER NOT NULL,
    cuisines TEXT[] NOT NULL DEFAULT '{}',
    price_min SMALLINT NOT NULL,
    price_max SMALLINT NOT NULL,
    min_rating REAL NOT NULL,
    created_by UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE room_restaurants (
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    restaurant_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    cuisine TEXT NOT NULL,
    price_level SMALLINT NOT NULL,
    rating REAL NOT NULL,
    rating_count INTEGER NOT NULL,
    address TEXT NOT NULL,
    photo_url TEXT,
    lat DOUBLE PRECISION NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (room_id, restaurant_id)
);

CREATE TABLE participants (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    display_name TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (room_id, user_id)
);

CREATE TABLE swipes (
    room_id UUID NOT NULL,
    participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    restaurant_id TEXT NOT NULL,
    liked BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (room_id, participant_id, restaurant_id),
    FOREIGN KEY (room_id, restaurant_id)
        REFERENCES room_restaurants(room_id, restaurant_id) ON DELETE CASCADE
);

CREATE TABLE lists (
    id UUID PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    owner_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE list_items (
    id UUID PRIMARY KEY,
    list_id UUID NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    cuisine TEXT,
    price_level SMALLINT,
    rating REAL,
    address TEXT,
    photo_url TEXT,
    added_by_user_id UUID NOT NULL,
    source_restaurant_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_swipes_room_liked ON swipes (room_id) WHERE liked;
CREATE INDEX idx_lists_owner ON lists (owner_user_id);
