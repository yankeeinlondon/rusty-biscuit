# evidence-store

A private service that will record test evidence (the observation collections
CI and test hosts already produce) in Cloudflare D1, and keep the reports and
proof files those observations cite in a private R2 bucket. It is a
TypeScript Cloudflare Worker plus the command-line tools that operate it and
submit evidence to it.

**Status: built, not yet deployed.** Git notes remain the only authoritative
evidence store. Nothing in CI reads from or writes to this service; evidence
reaches it only by explicit ingestion. Evaluation publication and retention
are not enabled. To deploy, operate, back up, or restore it, follow the
[operating guide](../../docs/cicd/evidence-store.md). What it contains:

- `api/`: the versioned HTTP API contract (`openapi.yaml`), strict JSON
  Schemas for the configuration files, and request/response examples. See
  [`api/README.md`](api/README.md) for roles, limits, and error codes.
- `src/contracts/`: the TypeScript port of the evidence contract. A strict
  JSON loader that keeps number spellings, the Python-compatible canonical
  form and SHA-256 identities, the collection and receipt validators, and the
  version-2 receipt adapter. Public entry points are `parseCollection`,
  `parseReceipt`, and `adaptReceipt`; each answers with located problems
  (JSON pointers) instead of throwing. It also holds the API constants
  (`LIMITS`, `ROLES`, `OPERATIONS`, `ERROR_CODES`).
- `migrations/`: numbered D1 migrations. Once a deployment applies one it is
  frozen, and later changes get a new number. The schema enforces the
  identity-conflict guard, immutability, and retention deletion guards.
- `src/` and `src/db/`: the Worker. Every request is authenticated, with a
  host bearer token checked against D1 on every request or a GitHub Actions
  OIDC token checked against the configured workflow allowlist. It is then
  authorized by role on the repository and rate-limited per credential. It
  serves:
  - atomic collection ingestion and exact, paged collection export;
  - observation lookup, a primary-read snapshot, candidate history over
    caller-supplied heads, and the two change feeds (observations and
    locations), all paged against the captured snapshot with signed cursors;
  - verifier-only location events;
  - resumable proof and report uploads, verifier finalization, and
    authorized object fetch;
  - verifier-driven retention: retention certificates, guarded prune jobs,
    and capacity admission for uploads;
  - the operator routes for repository registration, credentials, the audit
    trail, maintenance mode (quiescing writes for a backup), and recovering
    interrupted retention jobs;
  - a daily scheduled run: staging cleanup, retention jobs, change-feed
    tombstone expiry, and capacity checks.

  Evaluation publication answers 403 to everyone. No retention-verifier
  credential is provisioned, so real evidence is never pruned yet.
  `wrangler.jsonc` declares the `development` and `restore-test`
  environments; nothing is deployed.
- `cli/provision.ts` (`just provision`): creates an environment's D1
  database, R2 bucket, `database_id`, and cursor secret; safe to rerun.
- `cli/deploy.ts` (`just deploy`): applies the numbered migrations and
  deploys the Worker to one environment, then writes a redacted manifest.
- `cli/operator.ts` (`just operator …`): operator bootstrap through Wrangler,
  repository registration from GitHub's stable IDs, credential issue and
  revoke, and the recovery commands `backup`, `verify-backup`, `restore`, and
  `recover-jobs`. Secrets go only to owner-only files, never to stdout or argv. See
  [`api/README.md`](api/README.md#credentials).
- `cli/ingest.ts` (`just evidence-ingest`, `evidence-resume`,
  `evidence-finalize`): submits one collection or version-2 receipt and
  stages the files it references. See [Send evidence to the service](../../docs/cicd/evidence-store.md#send-evidence-to-the-service).
- `tests/fixtures/parity/`: frozen inputs with the exact canonical bytes and
  identities produced by the Python contract in `scripts/ci/schema.py`. A
  TypeScript port must reproduce them byte for byte.
- `docs/contract-inventory.md`: every evidence field, the Python validator
  that decides it, and the parity cases that cover it.
- `cli/inventory.ts`: a read-only capacity measurement of existing notes and
  reports, with a 12-month forecast against the free-tier allowances.
- `spike/d1-guard/` and `cli/prove-d1.ts`: the D1 feasibility checkpoint,
  which proves on a disposable database that a conflicting collection is
  rolled back whole.

## Install

```sh
just install
```

## Common tasks

Run these from this directory, or from the repository root as
`just evidence-store <recipe>`:

```sh
just typecheck            # strict TypeScript check
just test                 # node, workers (spike), and service projects (local Miniflare D1)
just lint
just capture-parity --check   # confirm the frozen fixtures still match Python
just inventory --out /tmp/evidence-inventory.json
just provision                                               # needs CLOUDFLARE_API_TOKEN, CLOUDFLARE_ACCOUNT_ID
just deploy --manifest /abs/deploy.json                     # needs CLOUDFLARE_API_TOKEN, CLOUDFLARE_ACCOUNT_ID
just test-real --report /abs/results.json                    # deployed acceptance suite
```

Operating the service (`operator`, `evidence-ingest`, `evidence-resume`,
`evidence-finalize`, `demo-hosted`, backups, and restores) is covered task by
task in the [operating guide](../../docs/cicd/evidence-store.md), and every
route, limit, and error code in [`api/README.md`](api/README.md).

`prove-d1` (the D1 feasibility checkpoint) never logs in interactively. It
exits 2 and names the missing variables when it has no credentials. When it
does run, it deletes the database and Worker it created.
