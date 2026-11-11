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
  - the operator routes for repository registration, credentials, and the
    audit trail.

  Evaluation publication answers 403 to everyone. Uploads, object fetch, and
  retention are not built yet. `wrangler.jsonc` declares the `development`
  and `restore-test` environments; nothing is deployed.
- `cli/operator.ts` (`just operator …`): operator bootstrap through Wrangler,
  repository registration from GitHub's stable IDs, and credential issue and
  revoke. Secrets go only to owner-only files, never to stdout or argv. See
  [`api/README.md`](api/README.md#credentials).
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
