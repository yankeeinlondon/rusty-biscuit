# Evidence store API contract

This directory is the machine-readable contract of the evidence store's HTTP
API. The evidence store is a private Cloudflare Worker that records test
_observations_ (one attempt to run one `{package, environment, gate}` cell) in
a D1 database and keeps their report and proof files in a private R2 bucket.
Nothing here is Worker code; the Worker and its tools are checked against these
documents.

| Path | What it is |
| --- | --- |
| `openapi.yaml` | OpenAPI 3.1 description of every route, request, response, and error. `x-roles` on each operation lists the roles allowed to call it; top-level `x-limits` lists every bound. |
| `schemas/*.schema.json` | Strict JSON Schemas (draft 2020-12) for the files the service and its tools read: `service-config`, `credential-grant`, `ingest-config`, `resume-state`, and `backup-manifest`. |
| `examples/<operationId>.request.json` / `.response.json` | One example per operation. `GET` operations have only a response; the two binary bodies (`putUploadPart` request, `fetchObject` response) have no JSON example. |
| `examples/error.<code>.json` | Example error bodies. |

The constants these documents publish live in `src/contracts/api.ts`;
`tests/node/api-contract.test.ts` fails when the two drift. Run it with:

```sh
pnpm exec vitest run --project node tests/node/api-contract.test.ts
```

## Routes and the role matrix

Repository routes live under `/v1/repos/{repository_id}`, where
`repository_id` is the repository's stable numeric GitHub ID (a slug can
change; the ID cannot). Operator routes live under `/v1/operator`. A request
under any version other than `v1` answers `unsupported_api_version`.

A _role_ is a permission a credential holds on a repository:

- **reader** reads evidence, snapshots, change feeds, and stored files
- **producer** submits collections and uploads the files they reference
- **verifier** checks bytes: it finalizes uploads and records report locations
- **retention-verifier** certifies which superseded evidence may be pruned
- **operator** registers repositories, manages credentials, and reads the audit trail

| Operation | Method and path | reader | producer | verifier | retention-verifier | operator |
| --- | --- | :-: | :-: | :-: | :-: | :-: |
| `ingestCollection` | `POST /collections` | | ✓ | | | |
| `exportCollection` | `GET /collections/{collection_id}` | ✓ | | ✓ | ✓ | |
| `getObservation` | `GET /observations/{observation_id}` | ✓ | | ✓ | ✓ | |
| `queryHistory` | `POST /history/query` | ✓ | | ✓ | ✓ | |
| `getSnapshot` | `GET /snapshot` | ✓ | | ✓ | ✓ | |
| `appendLocation` | `POST /locations` | | | ✓ | | |
| `queryChanges` | `POST /changes/query` | ✓ | | ✓ | ✓ | |
| `createUpload` | `POST /uploads` | | ✓ | ✓ | | |
| `putUploadPart` | `PUT /uploads/{upload_id}/parts/{index}` | | ✓ | ✓ | | |
| `finalizeUpload` | `POST /uploads/{upload_id}/finalize` | | | ✓ | | |
| `fetchObject` | `GET /objects/{observation_id}/{reference_kind}/{reference_digest}` | ✓ | | ✓ | ✓ | |
| `submitRetentionCertificate` | `POST /retention/certificates` | | | | ✓ | |
| `getRetentionJob` | `GET /retention/jobs/{job_id}` | | | | ✓ | ✓ |
| `publishEvaluation` | `POST /evaluations` | | | | | |
| `registerRepository` | `POST /v1/operator/repositories` | | | | | ✓ |
| `issueCredential` | `POST /v1/operator/credentials` | | | | | ✓ |
| `revokeCredential` | `POST /v1/operator/credentials/{credential_id}/revoke` | | | | | ✓ |
| `listAudit` | `POST /v1/operator/audit/query` | | | | | ✓ |

A producer can stage file bytes but never vouch for them: only a verifier
finalizes an upload or records a location. **Evaluation publication is
disabled.** `POST /evaluations` has no allowed role and answers 403
`evaluation_publication_disabled` to every caller until a trusted verifier and
its policy exist.

## Limits

Every limit is applied before anything is written. A collection or retry
chain is never split to fit: an oversized one is rejected whole.

| Limit | Value |
| --- | --- |
| JSON request body | 1 MiB |
| One observation, canonical bytes | 128 KiB |
| Observations per collection | 256 |
| Reachable heads per history query | 256 |
| Page size | 100 by default, at most 256 records and 1 MiB encoded |
| Upload part | 1 MiB, sent in order |
| Uploaded object | 64 MiB |
| Unfinished upload expiry | 24 hours (staging bytes removed after 48 hours) |
| Cursor expiry | 24 hours |
| Change-feed tombstones kept | 30 days |
| Location verification valid for | 24 hours |
| Grace before pruning superseded evidence | 14 days |
| Credential lifetime | 90 days; rotate at day 60; at most 7 days of overlap |
| OIDC clock tolerance | 60 seconds |
| Rate per credential | 60 mutations and 300 reads per minute; operators 30 mutations per minute |

## Errors

Every failure answers with one body shape:

```json
{
  "error": {
    "code": "identity_conflict",
    "message": "observation already recorded with different canonical bytes or collection context",
    "location": "/collection/observations/0",
    "retryable": false,
    "retry_after_ms": null
  },
  "request_id": "…"
}
```

`location` is a JSON pointer into the request, or `null`. Error bodies and logs
never contain credentials or report bytes. Clients retry only `retryable`
errors (429, 503) and network failures: at most five attempts, with jittered
backoff capped at 30 seconds, honoring a longer `Retry-After` up to 60 seconds.

| Code | HTTP | Retryable | When |
| --- | :-: | :-: | --- |
| `invalid_input` | 400 | no | The request breaks the schema or an evidence rule, or uploaded bytes do not match their reference. |
| `invalid_cursor` | 400 | no | A cursor was altered or belongs to another query or repository. |
| `unsupported_api_version` | 400 | no | The path names an API version other than `v1`. |
| `unauthenticated` | 401 | no | The bearer token is missing, malformed, expired, or revoked, or the OIDC token fails verification. |
| `denied` | 403 | no | The credential lacks the role or repository for this operation. |
| `evaluation_publication_disabled` | 403 | no | Any call to `POST /evaluations`. |
| `not_found` | 404 | no | Unknown repository or ID, or an object that is not currently available. |
| `identity_conflict` | 409 | no | An existing ID (observation, location event, upload part, certificate, repository owner) is resubmitted with different content. |
| `stale_snapshot` | 409 | no | A supplied snapshot belongs to an older store generation or is no longer current for a certificate. |
| `snapshot_pruned` | 409 | no | Pruning has happened since the snapshot was captured. |
| `cursor_expired` | 409 | no | The cursor is older than 24 hours, or a change-feed position is older than the 30-day tombstone window. Start from a fresh snapshot. |
| `collection_pruned` | 410 | no | A collection export would be missing pruned members. |
| `size_limit` | 413 | no | A bound in the limits table is exceeded. |
| `unpageable_record` | 413 | no | One record alone exceeds the page's byte bound. |
| `rate_limited` | 429 | yes | Per-credential request rate exceeded. |
| `quota_exhausted` | 429 | yes | The request would push a free-tier allowance past its 80% refusal threshold. |
| `unavailable` | 503 | yes | A platform failure; never reported as an empty success. |

## Credentials

Hosts (laptops and test rigs) authenticate with a bearer token sent as
`Authorization: Bearer ebt_<credential_id>.<secret>`.

- The secret is 256 random bits. The `credential_id` is public and appears in
  audit records; the secret never does.
- The service stores only the SHA-256 digest of the secret, so a database or
  backup copy cannot be turned back into a working token.
- `issueCredential` returns the token exactly once. The operator CLI writes it to
  an owner-only file and never prints it, passes it on a command line, or logs
  it. Unattended consumers read it from that file.
- Each credential names its repositories and roles. An empty list of either
  denies everything.
- Credentials expire after 90 days. Rotate at day 60 by issuing a replacement
  that names the old one in `replaces`; the old one then expires within 7 days.
- Revocation is checked against the primary database on every request, so it
  takes effect immediately. Revoking never rewrites past evidence.
- Operator credentials are created only by the Wrangler bootstrap and are never
  issued through the API.

## GitHub Actions (OIDC)

A workflow authenticates with its short-lived GitHub OIDC token, sent as a
bearer token; there is no exchange for a long-lived credential. The service
accepts it only when all of these hold:

- issuer `https://token.actions.githubusercontent.com` and audience
  `rusty-biscuit-evidence-store`;
- a valid signature, and expiry and not-before within 60 seconds;
- a numeric `repository_id` and `repository_owner_id` that match a registered
  repository;
- a workflow, ref, and event that match an entry in the service configuration's
  `trusted_workflows`; the workflow path is taken from the `workflow_ref` claim.

Fork and pull-request contexts are always refused, and an empty allowlist
denies every token. The only initial entry is
`.github/workflows/evidence-store-demo.yml` on `refs/heads/main` for
`workflow_dispatch`, granting producer and reader. OIDC identities never
receive verifier, retention-verifier, or operator.
