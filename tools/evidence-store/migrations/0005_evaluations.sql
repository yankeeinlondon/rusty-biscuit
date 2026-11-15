-- 0005: derived evaluations.
--
-- Creates the immutable evaluations table from the data model. No public
-- route writes it: evaluation publication answers 403 until a trusted verifier
-- policy exists. A restore invalidates evaluations through the repository
-- generation, never by deleting them.
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

CREATE TABLE evaluations (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    evaluation_id TEXT NOT NULL UNIQUE,
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    generation TEXT NOT NULL,
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

CREATE INDEX evaluations_lookup ON evaluations(
    repository_id, purpose, head, tree, plan_identity, verifier_identity,
    evidence_revision, seq
);

CREATE TRIGGER evaluations_no_update BEFORE UPDATE ON evaluations
BEGIN
    SELECT RAISE(ABORT, 'evaluations are immutable');
END;

CREATE TRIGGER evaluations_no_delete BEFORE DELETE ON evaluations
BEGIN
    SELECT RAISE(ABORT, 'evaluations are immutable');
END;
