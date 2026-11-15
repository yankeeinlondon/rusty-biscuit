-- 0004: proof uploads and capacity accounting.
--
-- Creates uploads (an upload bound to one repository, observation, and
-- reference kind/digest, with its declared length, part count, expiry, and
-- private R2 staging key), their received parts, and quota reservations that
-- admit capacity-consuming work against the free-tier allowances before it
-- runs.
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

-- No foreign key to observations: the upload handler checks the observation
-- and its reference before allocating, and a sanctioned prune removes uploads
-- with the observation they served.
CREATE TABLE uploads (
    upload_id TEXT PRIMARY KEY NOT NULL CHECK (
        length(upload_id) = 32 AND upload_id NOT GLOB '*[^0-9a-f]*'
    ),
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    observation_id TEXT NOT NULL,
    reference_kind TEXT NOT NULL CHECK (reference_kind IN (
        'report', 'expected-listing', 'completion-record'
    )),
    reference_digest TEXT NOT NULL
        CHECK (length(reference_digest) BETWEEN 1 AND 256),
    path TEXT NOT NULL CHECK (length(path) > 0),
    bytes INTEGER NOT NULL CHECK (bytes BETWEEN 0 AND 67108864),
    part_bytes INTEGER NOT NULL DEFAULT 1048576 CHECK (part_bytes = 1048576),
    part_count INTEGER NOT NULL CHECK (part_count = (bytes + 1048575) / 1048576),
    state TEXT NOT NULL CHECK (state IN ('open', 'finalized', 'rejected', 'expired')),
    staging_key TEXT NOT NULL UNIQUE,
    -- Whole-object SHA-256, set when finalization verifies the bytes.
    sha256 TEXT CHECK (sha256 IS NULL OR length(sha256) = 64),
    created_by TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL CHECK (expires_at_ms > created_at_ms),
    finalized_at_ms INTEGER,
    CHECK ((state = 'finalized') = (finalized_at_ms IS NOT NULL)),
    CHECK (state != 'finalized' OR sha256 IS NOT NULL)
);

CREATE INDEX uploads_reference ON uploads(
    repository_id, observation_id, reference_kind, reference_digest, state
);
CREATE INDEX uploads_expiry ON uploads(state, expires_at_ms);

-- The binding is fixed at allocation, and a terminal state never changes.
CREATE TRIGGER uploads_fixed_binding
BEFORE UPDATE OF upload_id, repository_id, observation_id, reference_kind,
    reference_digest, path, bytes, part_bytes, part_count, staging_key,
    created_by, created_at_ms
ON uploads
BEGIN
    SELECT RAISE(ABORT, 'upload binding is immutable');
END;

CREATE TRIGGER uploads_terminal_state BEFORE UPDATE OF state ON uploads
WHEN OLD.state != 'open' AND NEW.state IS NOT OLD.state
BEGIN
    SELECT RAISE(ABORT, 'upload state is terminal');
END;

-- One row per received part. `sha256` lets a replayed part be byte-checked
-- against the original rather than overwriting it.
CREATE TABLE upload_parts (
    upload_id TEXT NOT NULL REFERENCES uploads(upload_id),
    part_index INTEGER NOT NULL CHECK (part_index BETWEEN 0 AND 63),
    bytes INTEGER NOT NULL CHECK (bytes BETWEEN 1 AND 1048576),
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    r2_etag TEXT NOT NULL,
    received_at_ms INTEGER NOT NULL,
    PRIMARY KEY (upload_id, part_index)
);

CREATE TRIGGER upload_parts_no_update BEFORE UPDATE ON upload_parts
BEGIN
    SELECT RAISE(ABORT, 'upload parts are immutable');
END;

-- Capacity admitted before work runs. `period` is the allowance window the
-- amount counts against (`YYYY-MM` for monthly R2 operations and storage,
-- `YYYY-MM-DD` for daily D1/Worker allowances).
CREATE TABLE quota_reservations (
    reservation_id TEXT PRIMARY KEY NOT NULL CHECK (
        length(reservation_id) = 32 AND reservation_id NOT GLOB '*[^0-9a-f]*'
    ),
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    resource TEXT NOT NULL CHECK (resource IN (
        'r2_bytes', 'r2_class_a', 'r2_class_b', 'd1_rows_written',
        'd1_rows_read', 'worker_requests'
    )),
    period TEXT NOT NULL CHECK (length(period) IN (7, 10)),
    amount INTEGER NOT NULL CHECK (amount >= 0),
    upload_id TEXT REFERENCES uploads(upload_id),
    state TEXT NOT NULL CHECK (state IN ('reserved', 'committed', 'released')),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX quota_reservations_period
    ON quota_reservations(resource, period, state);
CREATE INDEX quota_reservations_upload ON quota_reservations(upload_id);

-- Measured account-wide consumption outside this service, subtracted from
-- each allowance before admission.
CREATE TABLE capacity_baseline (
    resource TEXT NOT NULL CHECK (resource IN (
        'r2_bytes', 'r2_class_a', 'r2_class_b', 'd1_rows_written',
        'd1_rows_read', 'worker_requests'
    )),
    period TEXT NOT NULL CHECK (length(period) IN (7, 10)),
    external_used INTEGER NOT NULL CHECK (external_used >= 0),
    measured_at_ms INTEGER NOT NULL,
    PRIMARY KEY (resource, period)
);
