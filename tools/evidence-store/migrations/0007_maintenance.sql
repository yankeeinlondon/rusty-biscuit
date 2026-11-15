-- 0007: persisted mutation maintenance mode.
--
-- Creates the single-row `service_state` table, whose `maintenance_mode` is
-- `open` or `quiesced`. While quiesced, the triggers below refuse every
-- evidence, location, upload, retention, and credential write with
-- `maintenance_mode`, so a backup taken during the quiesced window holds every
-- write acknowledged before `maintenance_since_ms` and none after it. The
-- Worker also checks the mode before a mutation starts; these triggers are the
-- authority, because they run inside the same D1 write as the change. The
-- audit log is deliberately not blocked: a refused request is still audited.
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

CREATE TABLE service_state (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    maintenance_mode TEXT NOT NULL CHECK (maintenance_mode IN ('open', 'quiesced')),
    maintenance_reason TEXT,
    maintenance_since_ms INTEGER,
    -- The backup boundary this quiesce serves, when one does.
    maintenance_boundary_id TEXT,
    CHECK ((maintenance_mode = 'quiesced') = (maintenance_since_ms IS NOT NULL)),
    CHECK ((maintenance_mode = 'quiesced') = (maintenance_reason IS NOT NULL))
);

INSERT INTO service_state(id, maintenance_mode) VALUES (1, 'open');

CREATE TRIGGER service_state_no_delete BEFORE DELETE ON service_state
BEGIN
    SELECT RAISE(ABORT, 'service state is permanent');
END;

CREATE TRIGGER observations_maintenance BEFORE INSERT ON observations
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER collections_maintenance BEFORE INSERT ON collections
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER collection_members_maintenance BEFORE INSERT ON collection_members
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER report_locations_maintenance BEFORE INSERT ON report_locations
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER report_locations_delete_maintenance BEFORE DELETE ON report_locations
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER observations_delete_maintenance BEFORE DELETE ON observations
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER uploads_insert_maintenance BEFORE INSERT ON uploads
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER uploads_update_maintenance BEFORE UPDATE ON uploads
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER upload_parts_maintenance BEFORE INSERT ON upload_parts
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER retention_certificates_maintenance BEFORE INSERT ON retention_certificates
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER retention_jobs_insert_maintenance BEFORE INSERT ON retention_jobs
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER retention_jobs_update_maintenance BEFORE UPDATE ON retention_jobs
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER release_pins_maintenance BEFORE INSERT ON release_pins
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER repositories_insert_maintenance BEFORE INSERT ON repositories
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

-- Credential issue is refused while quiesced; revocation is not, because
-- stopping a leaked credential outranks the backup boundary. A restore from
-- a backup taken in that window must replay revocations from the audit log
-- (the restore command does).
CREATE TRIGGER credentials_insert_maintenance BEFORE INSERT ON credentials
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;

CREATE TRIGGER credential_grants_maintenance BEFORE INSERT ON credential_grants
WHEN (SELECT maintenance_mode FROM service_state WHERE id = 1) = 'quiesced'
BEGIN
    SELECT RAISE(ABORT, 'maintenance_mode');
END;
