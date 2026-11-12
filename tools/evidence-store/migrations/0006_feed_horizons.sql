-- 0006: change-feed replay horizons.
--
-- Adds each repository's observation and location replay horizon: the lowest
-- stream position after which a complete replay is still guaranteed. A change
-- query resuming before it answers 409 `cursor_expired`. Both start at 0 (the
-- whole stream is replayable); expiring tombstones after
-- `change_tombstone_days` advances them and never moves them back. Also
-- indexes location events by event ID, which renders an event whose row was
-- pruned as its `pruned` tombstone.
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

ALTER TABLE repositories ADD COLUMN observation_event_horizon INTEGER NOT NULL
    DEFAULT 0 CHECK (observation_event_horizon >= 0);

ALTER TABLE repositories ADD COLUMN location_event_horizon INTEGER NOT NULL
    DEFAULT 0 CHECK (location_event_horizon >= 0);

CREATE INDEX location_events_event
    ON location_events(repository_id, event_id, seq);
