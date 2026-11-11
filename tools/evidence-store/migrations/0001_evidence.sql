-- 0001: evidence records.
--
-- Creates repositories (stable GitHub IDs, restore generation, retention
-- epoch, evidence revision, and both change-stream watermarks), immutable
-- observations with the in-batch identity-conflict guard, immutable collection
-- manifests and their position-ordered membership, append-only report-location
-- events keyed by caller-supplied event UUIDs, and the two change streams.
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

CREATE TABLE repositories (
    repository_id TEXT PRIMARY KEY NOT NULL CHECK (
        length(repository_id) BETWEEN 1 AND 20
        AND repository_id GLOB '[1-9]*'
        AND repository_id NOT GLOB '*[^0-9]*'
    ),
    -- Stable GitHub owner ID matched against the OIDC `repository_owner_id`
    -- claim. NULL never matches, so an unset owner denies hosted identities.
    owner_id TEXT CHECK (owner_id IS NULL OR (
        owner_id GLOB '[1-9]*' AND owner_id NOT GLOB '*[^0-9]*'
    )),
    slug TEXT NOT NULL UNIQUE,
    -- Restore generation: replaced on every restore so cursors and snapshots
    -- minted before it are rejected even when counters repeat.
    generation TEXT NOT NULL CHECK (length(generation) > 0),
    retention_epoch INTEGER NOT NULL DEFAULT 0 CHECK (retention_epoch >= 0),
    evidence_revision INTEGER NOT NULL DEFAULT 0 CHECK (evidence_revision >= 0),
    -- Highest sequence appended to each stream. Kept here, not derived with
    -- MAX(seq), so expiring old tombstones can never move a watermark back.
    observation_event_watermark INTEGER NOT NULL DEFAULT 0
        CHECK (observation_event_watermark >= 0),
    location_event_watermark INTEGER NOT NULL DEFAULT 0
        CHECK (location_event_watermark >= 0),
    registered_at_ms INTEGER NOT NULL CHECK (registered_at_ms >= 0)
);

CREATE TABLE observations (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    observation_id TEXT NOT NULL CHECK (
        length(observation_id) BETWEEN 8 AND 64
        AND observation_id NOT GLOB '*[^0-9a-f]*'
    ),
    environment TEXT NOT NULL CHECK (environment IN (
        'ubuntu-latest', 'macos-latest', 'windows-latest', 'wsl2-ubuntu'
    )),
    head TEXT NOT NULL CHECK (length(head) = 40 AND head NOT GLOB '*[^0-9a-f]*'),
    tree TEXT NOT NULL CHECK (length(tree) = 40 AND tree NOT GLOB '*[^0-9a-f]*'),
    -- Identity: (repository_id, observation_id); equality: payload_json plus
    -- environment/head/tree. The columns below the payload record the first
    -- accepted publication and never take part in equality.
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)
        AND json_type(payload_json) = 'object'),
    source_version INTEGER NOT NULL CHECK (source_version IN (2, 3)),
    accepted_principal TEXT NOT NULL CHECK (length(accepted_principal) > 0),
    accepted_request_id TEXT NOT NULL CHECK (length(accepted_request_id) > 0),
    accepted_collection_id TEXT NOT NULL,
    received_at_ms INTEGER NOT NULL CHECK (received_at_ms >= 0),
    package TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.package')) STORED NOT NULL,
    gate TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.gate')) STORED NOT NULL
        CHECK (gate IN ('lint', 'check', 'L1', 'L2', 'browser')),
    outcome TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.outcome')) STORED NOT NULL
        CHECK (outcome IN ('pass', 'fail')),
    completion TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.completion')) STORED NOT NULL
        CHECK (completion IN ('complete', 'interrupted', 'partial')),
    gate_input_identity TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.gate_input_identity')) STORED NOT NULL,
    test_filter TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.test_filter')) STORED,
    retry_of TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.retry_of')) STORED,
    UNIQUE (repository_id, observation_id),
    CHECK (json_type(payload_json, '$.observation_id') = 'text'),
    CHECK (observation_id = json_extract(payload_json, '$.observation_id'))
);

CREATE INDEX observations_cell
    ON observations(repository_id, package, gate, environment, seq);
CREATE INDEX observations_head
    ON observations(repository_id, head, environment, observation_id);
CREATE INDEX observations_retry
    ON observations(repository_id, retry_of);
CREATE INDEX observations_history
    ON observations(repository_id, package, gate, head, seq);

-- The conflict authority. It runs inside the same batch as the insert, so a
-- concurrent writer that won the race is seen here and the whole batch rolls
-- back; identical rows fall through to ON CONFLICT DO NOTHING and fire no
-- AFTER INSERT trigger.
CREATE TRIGGER observations_identity_guard BEFORE INSERT ON observations
WHEN EXISTS (
    SELECT 1 FROM observations o
    WHERE o.repository_id = NEW.repository_id
      AND o.observation_id = NEW.observation_id
      AND (o.payload_json IS NOT NEW.payload_json
           OR o.environment IS NOT NEW.environment
           OR o.head IS NOT NEW.head
           OR o.tree IS NOT NEW.tree)
)
BEGIN
    SELECT RAISE(ABORT, 'identity_conflict');
END;

CREATE TRIGGER observations_no_update BEFORE UPDATE ON observations
BEGIN
    SELECT RAISE(ABORT, 'observations are immutable');
END;

-- Change stream for observations: `accepted` on insert, `pruned` tombstones
-- on sanctioned deletion. Rows carry IDs and cell context only, never the
-- payload, so a tombstone outlives the observation it describes.
CREATE TABLE observation_events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    kind TEXT NOT NULL CHECK (kind IN ('accepted', 'pruned')),
    observation_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    package TEXT NOT NULL,
    gate TEXT NOT NULL,
    head TEXT NOT NULL,
    tree TEXT NOT NULL,
    outcome TEXT NOT NULL,
    completion TEXT NOT NULL,
    job_id TEXT,
    recorded_at_ms INTEGER NOT NULL,
    CHECK ((kind = 'pruned') = (job_id IS NOT NULL))
);

CREATE INDEX observation_events_feed
    ON observation_events(repository_id, seq);
CREATE INDEX observation_events_observation
    ON observation_events(repository_id, observation_id);

CREATE TRIGGER observation_events_watermark AFTER INSERT ON observation_events
BEGIN
    UPDATE repositories SET observation_event_watermark = NEW.seq
    WHERE repository_id = NEW.repository_id;
END;

CREATE TRIGGER observation_events_no_update BEFORE UPDATE ON observation_events
BEGIN
    SELECT RAISE(ABORT, 'change events are immutable');
END;

-- One revision step and one `accepted` event per newly inserted observation.
CREATE TRIGGER observations_accepted AFTER INSERT ON observations
BEGIN
    UPDATE repositories SET evidence_revision = evidence_revision + 1
    WHERE repository_id = NEW.repository_id;
    INSERT INTO observation_events(
        repository_id, kind, observation_id, environment, package, gate,
        head, tree, outcome, completion, recorded_at_ms
    )
    VALUES (
        NEW.repository_id, 'accepted', NEW.observation_id, NEW.environment,
        NEW.package, NEW.gate, NEW.head, NEW.tree, NEW.outcome, NEW.completion,
        NEW.received_at_ms
    );
END;

-- An exact submitted collection. `collection_id` is the SHA-256 of the
-- canonical version-3 collection; the first publisher is preserved.
CREATE TABLE collections (
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    collection_id TEXT NOT NULL CHECK (
        length(collection_id) = 64 AND collection_id NOT GLOB '*[^0-9a-f]*'
    ),
    environment TEXT NOT NULL CHECK (environment IN (
        'ubuntu-latest', 'macos-latest', 'windows-latest', 'wsl2-ubuntu'
    )),
    head TEXT NOT NULL,
    tree TEXT NOT NULL,
    observation_count INTEGER NOT NULL
        CHECK (observation_count BETWEEN 0 AND 256),
    source_version INTEGER NOT NULL CHECK (source_version IN (2, 3)),
    accepted_principal TEXT NOT NULL,
    accepted_request_id TEXT NOT NULL,
    received_at_ms INTEGER NOT NULL,
    PRIMARY KEY (repository_id, collection_id)
);

CREATE TRIGGER collections_identity_guard BEFORE INSERT ON collections
WHEN EXISTS (
    SELECT 1 FROM collections c
    WHERE c.repository_id = NEW.repository_id
      AND c.collection_id = NEW.collection_id
      AND (c.environment IS NOT NEW.environment
           OR c.head IS NOT NEW.head
           OR c.tree IS NOT NEW.tree
           OR c.observation_count IS NOT NEW.observation_count)
)
BEGIN
    SELECT RAISE(ABORT, 'identity_conflict');
END;

CREATE TRIGGER collections_no_update BEFORE UPDATE ON collections
BEGIN
    SELECT RAISE(ABORT, 'collections are immutable');
END;

CREATE TRIGGER collections_no_delete BEFORE DELETE ON collections
BEGIN
    SELECT RAISE(ABORT, 'collections are immutable');
END;

-- Membership deliberately has no foreign key to observations: a member whose
-- observation was pruned stays listed, so an export can tell a pruned
-- collection (410) from an exact one.
CREATE TABLE collection_members (
    repository_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position BETWEEN 0 AND 255),
    observation_id TEXT NOT NULL,
    PRIMARY KEY (repository_id, collection_id, position),
    UNIQUE (repository_id, collection_id, observation_id),
    FOREIGN KEY (repository_id, collection_id)
        REFERENCES collections(repository_id, collection_id)
);

CREATE INDEX collection_members_observation
    ON collection_members(repository_id, observation_id);

CREATE TRIGGER collection_members_identity_guard BEFORE INSERT ON collection_members
WHEN EXISTS (
    SELECT 1 FROM collection_members m
    WHERE m.repository_id = NEW.repository_id
      AND m.collection_id = NEW.collection_id
      AND ((m.position = NEW.position AND m.observation_id IS NOT NEW.observation_id)
           OR (m.observation_id = NEW.observation_id AND m.position IS NOT NEW.position))
)
BEGIN
    SELECT RAISE(ABORT, 'identity_conflict');
END;

CREATE TRIGGER collection_members_no_update BEFORE UPDATE ON collection_members
BEGIN
    SELECT RAISE(ABORT, 'collections are immutable');
END;

CREATE TRIGGER collection_members_no_delete BEFORE DELETE ON collection_members
BEGIN
    SELECT RAISE(ABORT, 'collections are immutable');
END;

-- Append-only location history. `event_id` is the caller-supplied UUID that
-- makes a replayed event idempotent; `location_key` is the SHA-256 of the
-- canonical locator; `checked_by` is the authenticated verifier, never a
-- claimed one.
CREATE TABLE report_locations (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id TEXT NOT NULL,
    event_id TEXT NOT NULL CHECK (length(event_id) = 36),
    observation_id TEXT NOT NULL,
    reference_kind TEXT NOT NULL CHECK (reference_kind IN (
        'report', 'expected-listing', 'completion-record'
    )),
    reference_digest TEXT NOT NULL
        CHECK (length(reference_digest) BETWEEN 1 AND 256),
    report_path TEXT NOT NULL CHECK (length(report_path) > 0),
    location_key TEXT NOT NULL CHECK (
        length(location_key) = 64 AND location_key NOT GLOB '*[^0-9a-f]*'
    ),
    location_kind TEXT NOT NULL CHECK (location_kind IN (
        'host-dir', 'artifact', 'object'
    )),
    locator_json TEXT NOT NULL CHECK (json_valid(locator_json)
        AND json_type(locator_json) = 'object'),
    state TEXT NOT NULL CHECK (state IN ('verified', 'unavailable', 'withdrawn')),
    verification_method TEXT CHECK (verification_method IN (
        'bytes', 'foreign-host-provenance'
    )),
    checked_at_ms INTEGER NOT NULL,
    valid_until_ms INTEGER,
    checked_by TEXT NOT NULL CHECK (length(checked_by) > 0),
    UNIQUE (repository_id, event_id),
    FOREIGN KEY (repository_id, observation_id)
        REFERENCES observations(repository_id, observation_id),
    CHECK ((state = 'verified') = (verification_method IS NOT NULL)),
    CHECK ((state = 'verified') = (valid_until_ms IS NOT NULL)),
    CHECK (valid_until_ms IS NULL OR valid_until_ms > checked_at_ms)
);

CREATE INDEX report_locations_history ON report_locations(
    repository_id, observation_id, reference_kind, reference_digest,
    location_key, seq
);

-- Same event UUID with a different event body is a conflict; an identical
-- replay falls through to ON CONFLICT DO NOTHING. Timestamps and the verifier
-- are excluded from equality.
CREATE TRIGGER report_locations_identity_guard BEFORE INSERT ON report_locations
WHEN EXISTS (
    SELECT 1 FROM report_locations l
    WHERE l.repository_id = NEW.repository_id
      AND l.event_id = NEW.event_id
      AND (l.observation_id IS NOT NEW.observation_id
           OR l.reference_kind IS NOT NEW.reference_kind
           OR l.reference_digest IS NOT NEW.reference_digest
           OR l.report_path IS NOT NEW.report_path
           OR l.location_key IS NOT NEW.location_key
           OR l.location_kind IS NOT NEW.location_kind
           OR l.locator_json IS NOT NEW.locator_json
           OR l.state IS NOT NEW.state
           OR l.verification_method IS NOT NEW.verification_method)
)
BEGIN
    SELECT RAISE(ABORT, 'identity_conflict');
END;

CREATE TRIGGER report_locations_no_update BEFORE UPDATE ON report_locations
BEGIN
    SELECT RAISE(ABORT, 'report location history is immutable');
END;

-- Change stream for locations: one row per location event (`kind` is its
-- state) plus `pruned` tombstones. IDs and context only.
CREATE TABLE location_events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    kind TEXT NOT NULL CHECK (kind IN (
        'verified', 'unavailable', 'withdrawn', 'pruned'
    )),
    event_id TEXT NOT NULL,
    observation_id TEXT NOT NULL,
    location_key TEXT NOT NULL,
    job_id TEXT,
    recorded_at_ms INTEGER NOT NULL,
    CHECK ((kind = 'pruned') = (job_id IS NOT NULL))
);

CREATE INDEX location_events_feed ON location_events(repository_id, seq);

CREATE TRIGGER location_events_watermark AFTER INSERT ON location_events
BEGIN
    UPDATE repositories SET location_event_watermark = NEW.seq
    WHERE repository_id = NEW.repository_id;
END;

CREATE TRIGGER location_events_no_update BEFORE UPDATE ON location_events
BEGIN
    SELECT RAISE(ABORT, 'change events are immutable');
END;

CREATE TRIGGER report_locations_appended AFTER INSERT ON report_locations
BEGIN
    UPDATE repositories SET evidence_revision = evidence_revision + 1
    WHERE repository_id = NEW.repository_id;
    INSERT INTO location_events(
        repository_id, kind, event_id, observation_id, location_key, recorded_at_ms
    )
    VALUES (
        NEW.repository_id, NEW.state, NEW.event_id, NEW.observation_id,
        NEW.location_key, NEW.checked_at_ms
    );
END;
