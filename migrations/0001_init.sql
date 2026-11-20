-- Schema for campus-hangouts. Timestamps are INTEGER microseconds since the Unix epoch (UTC),
-- ids are hyphenated UUID TEXT, tag/interest lists are JSON arrays in TEXT.

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    display_name  TEXT NOT NULL,
    bio           TEXT,
    interests     TEXT NOT NULL DEFAULT '[]',
    discord_id    TEXT UNIQUE,
    is_demo       INTEGER NOT NULL DEFAULT 0,
    privacy_attendance      TEXT NOT NULL DEFAULT 'friends' CHECK (privacy_attendance IN ('private', 'friends')),
    privacy_buddy_matching  INTEGER NOT NULL DEFAULT 1,
    privacy_personalize     INTEGER NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL
);

CREATE TABLE crews (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_by  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  INTEGER NOT NULL
);

CREATE TABLE crew_members (
    crew_id   TEXT NOT NULL REFERENCES crews(id) ON DELETE CASCADE,
    user_id   TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role      TEXT NOT NULL CHECK (role IN ('owner', 'member')),
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (crew_id, user_id)
);

CREATE TABLE events (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL CHECK (kind IN ('official', 'hangout')),
    title         TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    tags          TEXT NOT NULL DEFAULT '[]',
    location_name TEXT NOT NULL,
    location_room TEXT,
    location_lat  REAL,
    location_lon  REAL,
    online_url    TEXT,
    starts_at     INTEGER NOT NULL,
    ends_at       INTEGER NOT NULL,
    capacity      INTEGER,
    host_id       TEXT REFERENCES users(id) ON DELETE SET NULL,
    crew_id       TEXT REFERENCES crews(id) ON DELETE SET NULL,
    external_id   TEXT UNIQUE,
    url           TEXT,
    image_url     TEXT,
    organizer     TEXT,
    embedding     BLOB,
    created_at    INTEGER NOT NULL
);
CREATE INDEX events_starts_at ON events(starts_at);

CREATE TABLE intents (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id   TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    level      TEXT NOT NULL CHECK (level IN ('interested', 'going', 'going_with')),
    with_user  TEXT REFERENCES users(id) ON DELETE SET NULL,
    status     TEXT NOT NULL CHECK (status IN ('confirmed', 'waitlisted')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, event_id)
);
CREATE INDEX intents_event ON intents(event_id, created_at);

CREATE TABLE friendships (
    requester  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    addressee  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status     TEXT NOT NULL CHECK (status IN ('pending', 'accepted')),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (requester, addressee),
    CHECK (requester <> addressee)
);

CREATE TABLE buddy_requests (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id   TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, event_id)
);

CREATE TABLE availability (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    local_date TEXT NOT NULL,
    starts_at  INTEGER NOT NULL,
    ends_at    INTEGER NOT NULL,
    PRIMARY KEY (user_id, local_date)
);

CREATE TABLE gear (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    setup   TEXT NOT NULL CHECK (setup IN ('console', 'laptop', 'other')),
    game    TEXT NOT NULL,
    PRIMARY KEY (user_id, setup, game)
);

CREATE TABLE presence (
    user_id  TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    presence TEXT NOT NULL CHECK (presence IN ('here', 'interested', 'out')),
    at       INTEGER NOT NULL
);

CREATE TABLE checkins (
    user_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    at       INTEGER NOT NULL,
    PRIMARY KEY (user_id, event_id)
);
