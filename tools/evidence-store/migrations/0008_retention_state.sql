-- 0008: retention grace state and cell-lock coverage of uploads and locations.
--
-- Creates `retention_supersessions`, the time each observation was first
-- certified as superseded. The 14-day pruning grace is measured from it, so a
-- later certificate naming the same observation never restarts the grace. A
-- row is deleted only with its observation, by a job in `deleting-rows`.
--
-- Extends the cell lock of 0002 (which refuses new observations) to the two
-- other writes that could make a cell's evidence available while a retention
-- job deletes in it: a new upload of one of the cell's observations, and a
-- location event other than `withdrawn` (a withdrawal only ever denies
-- retrieval, and the job appends its own withdrawals before it locks).
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

CREATE TABLE retention_supersessions (
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    observation_id TEXT NOT NULL,
    first_certified_at_ms INTEGER NOT NULL CHECK (first_certified_at_ms >= 0),
    -- The certificate that first certified the supersession.
    certificate_id TEXT NOT NULL,
    PRIMARY KEY (repository_id, observation_id)
);

CREATE TRIGGER retention_supersessions_no_update BEFORE UPDATE ON retention_supersessions
BEGIN
    SELECT RAISE(ABORT, 'retention supersessions are immutable');
END;

CREATE TRIGGER retention_supersessions_delete_guard BEFORE DELETE ON retention_supersessions
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

CREATE TRIGGER retention_supersessions_maintenance BEFORE INSERT ON retention_supersessions
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER uploads_cell_lock BEFORE INSERT ON uploads
WHEN EXISTS (
    SELECT 1 FROM observations o
    JOIN cell_locks c
      ON c.repository_id = o.repository_id AND c.package = o.package
     AND c.environment = o.environment AND c.gate = o.gate
    WHERE o.repository_id = NEW.repository_id
      AND o.observation_id = NEW.observation_id
)
BEGIN
    SELECT RAISE(ABORT, 'cell_locked');
END;

CREATE TRIGGER report_locations_cell_lock BEFORE INSERT ON report_locations
WHEN NEW.state <> 'withdrawn' AND EXISTS (
    SELECT 1 FROM observations o
    JOIN cell_locks c
      ON c.repository_id = o.repository_id AND c.package = o.package
     AND c.environment = o.environment AND c.gate = o.gate
    WHERE o.repository_id = NEW.repository_id
      AND o.observation_id = NEW.observation_id
)
BEGIN
    SELECT RAISE(ABORT, 'cell_locked');
END;
