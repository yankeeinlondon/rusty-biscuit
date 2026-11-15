-- 0002: verifier-driven retention and its deletion guards.
--
-- Creates retention certificates, retention jobs and their members, cell
-- locks, and release pins, then guards evidence deletion: an observation or
-- location row may be deleted only while a job in `deleting-rows` lists that
-- observation as a member, a release-pinned observation can never become a
-- member, and a locked cell refuses new observations. Each sanctioned
-- deletion appends a `pruned` tombstone and advances the evidence revision in
-- the same statement.
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

-- A submitted certificate, stored verbatim. `certificate_id` is the
-- caller-supplied UUID; replaying it with identical content is idempotent.
CREATE TABLE retention_certificates (
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    certificate_id TEXT NOT NULL CHECK (length(certificate_id) = 36),
    certificate_json TEXT NOT NULL CHECK (json_valid(certificate_json)
        AND json_type(certificate_json) = 'object'),
    certificate_sha256 TEXT NOT NULL CHECK (length(certificate_sha256) = 64),
    generation TEXT NOT NULL,
    retention_epoch INTEGER NOT NULL,
    evidence_revision INTEGER NOT NULL,
    verifier_policy_digest TEXT NOT NULL,
    submitted_by TEXT NOT NULL,
    submitted_request_id TEXT NOT NULL,
    submitted_at_ms INTEGER NOT NULL,
    PRIMARY KEY (repository_id, certificate_id)
);

CREATE TRIGGER retention_certificates_no_update BEFORE UPDATE ON retention_certificates
BEGIN
    SELECT RAISE(ABORT, 'retention certificates are immutable');
END;

-- Persisted prune job state; the job advances withdraw -> delete objects ->
-- delete rows, and recovery resumes from `state` and the progress counters.
CREATE TABLE retention_jobs (
    job_id TEXT PRIMARY KEY NOT NULL CHECK (
        length(job_id) = 32 AND job_id NOT GLOB '*[^0-9a-f]*'
    ),
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    certificate_id TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'scheduled', 'withdrawing', 'deleting-objects', 'deleting-rows',
        'completed', 'invalidated'
    )),
    generation TEXT NOT NULL,
    retention_epoch INTEGER NOT NULL,
    evidence_revision INTEGER NOT NULL,
    member_count INTEGER NOT NULL DEFAULT 0 CHECK (member_count >= 0),
    locations_withdrawn INTEGER NOT NULL DEFAULT 0 CHECK (locations_withdrawn >= 0),
    objects_deleted INTEGER NOT NULL DEFAULT 0 CHECK (objects_deleted >= 0),
    observations_deleted INTEGER NOT NULL DEFAULT 0 CHECK (observations_deleted >= 0),
    not_before_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE (repository_id, job_id),
    UNIQUE (repository_id, certificate_id),
    FOREIGN KEY (repository_id, certificate_id)
        REFERENCES retention_certificates(repository_id, certificate_id)
);

CREATE INDEX retention_jobs_state ON retention_jobs(state, not_before_ms);

-- The certificate's superseded observations. The composite foreign key keeps
-- a member in its job's repository.
CREATE TABLE retention_job_members (
    job_id TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    observation_id TEXT NOT NULL,
    package TEXT NOT NULL,
    environment TEXT NOT NULL,
    gate TEXT NOT NULL,
    -- First certified supersession; the 14-day grace is measured from here.
    superseded_at_ms INTEGER NOT NULL,
    PRIMARY KEY (job_id, observation_id),
    FOREIGN KEY (repository_id, job_id)
        REFERENCES retention_jobs(repository_id, job_id)
);

CREATE INDEX retention_job_members_observation
    ON retention_job_members(repository_id, observation_id);

-- Evidence cited by a release. A pinned observation is never deletable.
CREATE TABLE release_pins (
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    release TEXT NOT NULL CHECK (length(release) > 0),
    observation_id TEXT NOT NULL,
    pinned_at_ms INTEGER NOT NULL,
    PRIMARY KEY (repository_id, release, observation_id)
);

CREATE INDEX release_pins_observation ON release_pins(repository_id, observation_id);

CREATE TRIGGER retention_job_members_not_pinned BEFORE INSERT ON retention_job_members
WHEN EXISTS (
    SELECT 1 FROM release_pins p
    WHERE p.repository_id = NEW.repository_id
      AND p.observation_id = NEW.observation_id
)
BEGIN
    SELECT RAISE(ABORT, 'release_pinned');
END;

-- The converse race: pinning an observation that an unfinished job will delete.
CREATE TRIGGER release_pins_not_member BEFORE INSERT ON release_pins
WHEN EXISTS (
    SELECT 1 FROM retention_job_members m
    JOIN retention_jobs j ON j.job_id = m.job_id
    WHERE m.repository_id = NEW.repository_id
      AND m.observation_id = NEW.observation_id
      AND j.state NOT IN ('completed', 'invalidated')
)
BEGIN
    SELECT RAISE(ABORT, 'retention_member');
END;

CREATE TRIGGER retention_job_members_no_update BEFORE UPDATE ON retention_job_members
BEGIN
    SELECT RAISE(ABORT, 'retention job members are immutable');
END;

-- Held on a `{package, environment, gate}` cell while a job deletes in it;
-- released on completion or recovery.
CREATE TABLE cell_locks (
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    package TEXT NOT NULL,
    environment TEXT NOT NULL,
    gate TEXT NOT NULL,
    job_id TEXT NOT NULL REFERENCES retention_jobs(job_id),
    acquired_at_ms INTEGER NOT NULL,
    PRIMARY KEY (repository_id, package, environment, gate)
);

CREATE INDEX cell_locks_job ON cell_locks(job_id);

-- A locked cell refuses new observations; an identical replay of an existing
-- row is unaffected because it inserts nothing.
CREATE TRIGGER observations_cell_lock BEFORE INSERT ON observations
WHEN EXISTS (
    SELECT 1 FROM cell_locks c
    WHERE c.repository_id = NEW.repository_id
      AND c.package = json_extract(NEW.payload_json, '$.cell.package')
      AND c.environment = NEW.environment
      AND c.gate = json_extract(NEW.payload_json, '$.cell.gate')
) AND NOT EXISTS (
    SELECT 1 FROM observations o
    WHERE o.repository_id = NEW.repository_id
      AND o.observation_id = NEW.observation_id
)
BEGIN
    SELECT RAISE(ABORT, 'cell_locked');
END;

-- Deletion guard. No route, operator or otherwise, can delete evidence
-- without a job that a retention verifier's certificate created.
CREATE TRIGGER observations_delete_guard BEFORE DELETE ON observations
WHEN NOT EXISTS (
    SELECT 1 FROM retention_job_members m
    JOIN retention_jobs j ON j.job_id = m.job_id
    WHERE m.repository_id = OLD.repository_id
      AND m.observation_id = OLD.observation_id
      AND j.state = 'deleting-rows'
)
BEGIN
    SELECT RAISE(ABORT, 'deletion_not_authorized');
END;

CREATE TRIGGER observations_pruned AFTER DELETE ON observations
BEGIN
    UPDATE repositories SET evidence_revision = evidence_revision + 1
    WHERE repository_id = OLD.repository_id;
    INSERT INTO observation_events(
        repository_id, kind, observation_id, environment, package, gate,
        head, tree, outcome, completion, job_id, recorded_at_ms
    )
    SELECT OLD.repository_id, 'pruned', OLD.observation_id, OLD.environment,
           OLD.package, OLD.gate, OLD.head, OLD.tree, OLD.outcome,
           OLD.completion, m.job_id, j.updated_at_ms
    FROM retention_job_members m
    JOIN retention_jobs j ON j.job_id = m.job_id
    WHERE m.repository_id = OLD.repository_id
      AND m.observation_id = OLD.observation_id
      AND j.state = 'deleting-rows'
    LIMIT 1;
END;

CREATE TRIGGER report_locations_delete_guard BEFORE DELETE ON report_locations
WHEN NOT EXISTS (
    SELECT 1 FROM retention_job_members m
    JOIN retention_jobs j ON j.job_id = m.job_id
    WHERE m.repository_id = OLD.repository_id
      AND m.observation_id = OLD.observation_id
      AND j.state = 'deleting-rows'
)
BEGIN
    SELECT RAISE(ABORT, 'deletion_not_authorized');
END;

CREATE TRIGGER report_locations_pruned AFTER DELETE ON report_locations
BEGIN
    UPDATE repositories SET evidence_revision = evidence_revision + 1
    WHERE repository_id = OLD.repository_id;
    INSERT INTO location_events(
        repository_id, kind, event_id, observation_id, location_key, job_id,
        recorded_at_ms
    )
    SELECT OLD.repository_id, 'pruned', OLD.event_id, OLD.observation_id,
           OLD.location_key, m.job_id, j.updated_at_ms
    FROM retention_job_members m
    JOIN retention_jobs j ON j.job_id = m.job_id
    WHERE m.repository_id = OLD.repository_id
      AND m.observation_id = OLD.observation_id
      AND j.state = 'deleting-rows'
    LIMIT 1;
END;
