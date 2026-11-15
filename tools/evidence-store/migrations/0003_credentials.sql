-- 0003: host and operator credentials, their grants, and the audit log.
--
-- Creates credentials (public ID plus the SHA-256 digest of the secret; the
-- secret itself is never stored), per-repository role grants, and the
-- append-only audit log, which records credential IDs and actions but never
-- token values.
--
-- Applied migrations are frozen: once a deployment has applied this file it is
-- never edited. Every later schema change is a new, higher-numbered migration.

CREATE TABLE credentials (
    credential_id TEXT PRIMARY KEY NOT NULL CHECK (
        length(credential_id) = 32 AND credential_id NOT GLOB '*[^0-9a-f]*'
    ),
    -- `operator` credentials carry the operator role implicitly and have no
    -- grants; `host` credentials act only through `credential_grants`.
    kind TEXT NOT NULL CHECK (kind IN ('host', 'operator')),
    label TEXT NOT NULL CHECK (length(label) BETWEEN 1 AND 128),
    secret_sha256 TEXT NOT NULL UNIQUE CHECK (
        length(secret_sha256) = 64 AND secret_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    issued_at_ms INTEGER NOT NULL,
    rotate_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    replaces TEXT REFERENCES credentials(credential_id),
    -- The scope as issued, so the public record survives an empty role list
    -- (which leaves no grant rows). Authorization reads `credential_grants`.
    repository_ids_json TEXT NOT NULL DEFAULT '[]' CHECK (
        json_valid(repository_ids_json) AND json_type(repository_ids_json) = 'array'
    ),
    roles_json TEXT NOT NULL DEFAULT '[]' CHECK (
        json_valid(roles_json) AND json_type(roles_json) = 'array'
    ),
    revoked_at_ms INTEGER,
    revocation_reason TEXT,
    CHECK (issued_at_ms <= rotate_at_ms AND rotate_at_ms <= expires_at_ms),
    CHECK ((revoked_at_ms IS NULL) = (revocation_reason IS NULL))
);

-- Only expiry (rotation overlap) and revocation may change; identity, kind,
-- and the secret digest never do.
CREATE TRIGGER credentials_fixed_identity
BEFORE UPDATE OF credential_id, kind, label, secret_sha256, issued_at_ms, replaces,
    repository_ids_json, roles_json
ON credentials
BEGIN
    SELECT RAISE(ABORT, 'credential identity is immutable');
END;

CREATE TABLE credential_grants (
    credential_id TEXT NOT NULL REFERENCES credentials(credential_id),
    repository_id TEXT NOT NULL REFERENCES repositories(repository_id),
    role TEXT NOT NULL CHECK (role IN (
        'reader', 'producer', 'verifier', 'retention-verifier'
    )),
    PRIMARY KEY (credential_id, repository_id, role)
);

CREATE INDEX credential_grants_repository ON credential_grants(repository_id, role);

CREATE TRIGGER credential_grants_host_only BEFORE INSERT ON credential_grants
WHEN (SELECT kind FROM credentials WHERE credential_id = NEW.credential_id) IS NOT 'host'
BEGIN
    SELECT RAISE(ABORT, 'grants belong to host credentials');
END;

CREATE TABLE audit_log (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    at_ms INTEGER NOT NULL,
    action TEXT NOT NULL CHECK (action IN (
        'repository.register', 'repository.rename', 'credential.issue',
        'credential.revoke', 'credential.denied', 'retention.certificate',
        'retention.job', 'retention.recover', 'operator.bootstrap',
        'restore.generation', 'maintenance.quiesce', 'maintenance.resume'
    )),
    -- The authenticated principal as `publisherKey` JSON (the API's
    -- `Principal`), e.g. `{"kind":"credential","credential_id":"<id>"}`.
    -- Never a token or a secret digest.
    actor TEXT NOT NULL CHECK (length(actor) > 0),
    actor_credential_id TEXT,
    repository_id TEXT,
    target_credential_id TEXT,
    request_id TEXT NOT NULL CHECK (length(request_id) > 0),
    detail_json TEXT CHECK (detail_json IS NULL OR json_valid(detail_json))
);

CREATE INDEX audit_log_repository ON audit_log(repository_id, seq);
CREATE INDEX audit_log_credential ON audit_log(target_credential_id, seq);

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit log is append-only');
END;

CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit log is append-only');
END;
