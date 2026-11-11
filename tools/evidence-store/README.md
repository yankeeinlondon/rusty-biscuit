# evidence-store

A private service that will record test evidence (the observation collections
CI and test hosts already produce) in Cloudflare D1, and keep the reports and
proof files those observations cite in a private R2 bucket. It is a
TypeScript Cloudflare Worker plus the command-line tools that operate it and
submit evidence to it.

**Status: under construction.** Git notes remain the only authoritative
evidence store. Nothing in CI reads from or writes to this service, and no
Worker is deployed yet. What exists today:

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
- `cli/operator.ts` (`just operator …`): operator bootstrap through Wrangler,
  repository registration from GitHub's stable IDs, credential issue and
  revoke, and the recovery commands `backup`, `verify-backup`, `restore`, and
  `recover-jobs`. Secrets go only to owner-only files, never to stdout or argv. See
  [`api/README.md`](api/README.md#credentials).
- `cli/ingest.ts` (`just evidence-ingest`, `evidence-resume`,
  `evidence-finalize`): submits one collection or version-2 receipt and
  stages the files it references. See [Ingesting evidence](#ingesting-evidence).
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

```sh
just test                 # node, workers (spike), and service projects (local Miniflare D1)
just lint                 # strict TypeScript check
just capture-parity --check   # confirm the frozen fixtures still match Python
just inventory --out /tmp/evidence-inventory.json
just operator issue --endpoint https://… --token-file /abs/operator.token --grant /abs/host.grant.json
CLOUDFLARE_API_TOKEN=… CLOUDFLARE_ACCOUNT_ID=… just prove-d1
```

`prove-d1` never logs in interactively. It exits 2 and names the missing
variables when it has no credentials. When it does run, it deletes the
database and Worker it created.

## Ingesting evidence

`just evidence-ingest` submits one version-3 collection, or a version-2
receipt that it converts on your machine (reading the receipt's reports from
its `report_dir`). With `--proof-dir`, it also uploads every report and proof
file the collection references, looked up by name inside that directory. Every
flag, the token file, the input, and each mapped file are checked before
anything is sent; a problem names its location (for example
`/observations/0/reports/1`) and exits 2.

```sh
just evidence-ingest ./collection.json --endpoint https://evidence.example.workers.dev \
  --repo 873451209 --token-file ~/.config/evidence-store/producer.token \
  --state ./ingest-0001.json --proof-dir ~/ci-evidence/run-0001
```

Progress is saved to the `--state` file, with copies of the input and the
submitted collection beside it, so an interrupted run continues with
`just evidence-resume ./ingest-0001.json` and never reruns a test. A producer
credential only stages uploads; a verifier then publishes them with
`just evidence-finalize ./ingest-0001.json --token-file PATH`. The routes,
roles, and retry rules are in [`api/README.md`](api/README.md).
