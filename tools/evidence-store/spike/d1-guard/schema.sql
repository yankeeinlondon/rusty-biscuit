-- Feasibility spike schema: the d1-model tables plus the collection manifest,
-- membership, observation events, and the in-batch identity-conflict guard
-- the spec requires. Disposable: the service's numbered migrations are built
-- separately and are not derived from this file.

CREATE TABLE repositories (
    repository_id TEXT PRIMARY KEY NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    evidence_revision INTEGER NOT NULL DEFAULT 0
        CHECK (evidence_revision >= 0)
);

CREATE TABLE observations (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    observation_id TEXT NOT NULL,
    environment TEXT NOT NULL CHECK (environment IN (
        'ubuntu-latest', 'macos-latest', 'windows-latest', 'wsl2-ubuntu'
    )),
    head TEXT NOT NULL,
    tree TEXT NOT NULL,
    source_version INTEGER NOT NULL CHECK (source_version IN (2, 3)),
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)
        AND json_type(payload_json) = 'object'),
    accepted_principal TEXT NOT NULL,
    accepted_request_id TEXT NOT NULL,
    received_at_ms INTEGER NOT NULL,
    package TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.package')) STORED NOT NULL,
    gate TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.gate')) STORED NOT NULL
        CHECK (gate IN ('lint', 'check', 'L1', 'L2', 'browser')),
    outcome TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.outcome')) STORED NOT NULL
        CHECK (outcome IN ('pass', 'fail')),
    gate_input_identity TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.cell.gate_input_identity')) STORED NOT NULL,
    retry_of TEXT GENERATED ALWAYS AS
        (json_extract(payload_json, '$.retry_of')) STORED,
    UNIQUE (repository_id, observation_id),
    CHECK (observation_id = json_extract(payload_json, '$.observation_id')),
    CHECK (json_type(payload_json, '$.observation_id') = 'text')
);

CREATE INDEX observations_cell
    ON observations(repository_id, package, gate, environment, seq);
CREATE INDEX observations_head
    ON observations(repository_id, head, environment, observation_id);
CREATE INDEX observations_retry
    ON observations(repository_id, retry_of);

-- The conflict authority. It runs inside the same batch as the insert, so a
-- concurrent writer that won the race is seen here and the whole batch rolls
-- back; identical deliveries fall through to ON CONFLICT DO NOTHING and fire
-- no AFTER INSERT trigger.
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

CREATE TABLE observation_events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id TEXT NOT NULL,
    observation_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('accepted', 'pruned'))
);

CREATE TRIGGER observations_revision AFTER INSERT ON observations
BEGIN
    UPDATE repositories SET evidence_revision = evidence_revision + 1
    WHERE repository_id = NEW.repository_id;
    INSERT INTO observation_events(repository_id, observation_id, kind)
    VALUES (NEW.repository_id, NEW.observation_id, 'accepted');
END;

CREATE TRIGGER observations_no_update BEFORE UPDATE ON observations
BEGIN
    SELECT RAISE(ABORT, 'observations are immutable');
END;

CREATE TABLE collections (
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    collection_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    head TEXT NOT NULL,
    tree TEXT NOT NULL,
    source_version INTEGER NOT NULL CHECK (source_version IN (2, 3)),
    observation_count INTEGER NOT NULL CHECK (observation_count >= 0),
    accepted_principal TEXT NOT NULL,
    received_at_ms INTEGER NOT NULL,
    PRIMARY KEY (repository_id, collection_id)
);

CREATE TABLE collection_members (
    repository_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    observation_id TEXT NOT NULL,
    PRIMARY KEY (repository_id, collection_id, position),
    FOREIGN KEY (repository_id, collection_id)
        REFERENCES collections(repository_id, collection_id),
    FOREIGN KEY (repository_id, observation_id)
        REFERENCES observations(repository_id, observation_id)
);

CREATE TABLE report_locations (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id TEXT NOT NULL,
    observation_id TEXT NOT NULL,
    report_path TEXT NOT NULL,
    location_key TEXT NOT NULL,
    location_kind TEXT NOT NULL CHECK (location_kind IN (
        'host-dir', 'artifact', 'object'
    )),
    locator_json TEXT NOT NULL CHECK (json_valid(locator_json)
        AND json_type(locator_json) = 'object'),
    state TEXT NOT NULL CHECK (state IN (
        'verified', 'unavailable', 'withdrawn'
    )),
    verification_method TEXT CHECK (verification_method IN (
        'bytes', 'foreign-host-provenance'
    )),
    checked_at_ms INTEGER NOT NULL,
    valid_until_ms INTEGER,
    checked_by TEXT NOT NULL,
    FOREIGN KEY (repository_id, observation_id)
        REFERENCES observations(repository_id, observation_id),
    CHECK (state != 'verified' OR
        (verification_method IS NOT NULL AND valid_until_ms IS NOT NULL
         AND valid_until_ms > checked_at_ms))
);

CREATE INDEX report_locations_history ON report_locations(
    repository_id, observation_id, report_path, location_key, seq
);

CREATE TRIGGER report_locations_revision AFTER INSERT ON report_locations
BEGIN
    UPDATE repositories SET evidence_revision = evidence_revision + 1
    WHERE repository_id = NEW.repository_id;
END;

CREATE TABLE evaluations (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    evaluation_id TEXT NOT NULL UNIQUE,
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    purpose TEXT NOT NULL CHECK (purpose IN ('ci-reuse', 'release-readiness')),
    head TEXT NOT NULL,
    tree TEXT NOT NULL,
    plan_identity TEXT NOT NULL,
    verifier_identity TEXT NOT NULL,
    evidence_revision INTEGER NOT NULL,
    observation_watermark INTEGER NOT NULL,
    location_watermark INTEGER NOT NULL,
    required_cells_json TEXT NOT NULL CHECK (json_valid(required_cells_json)
        AND json_type(required_cells_json) = 'array'),
    decisions_json TEXT NOT NULL CHECK (json_valid(decisions_json)
        AND json_type(decisions_json) = 'array'),
    satisfied INTEGER NOT NULL CHECK (satisfied IN (0, 1)),
    evaluated_at_ms INTEGER NOT NULL,
    valid_until_ms INTEGER NOT NULL CHECK (valid_until_ms > evaluated_at_ms)
);
