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
- `src/contracts/`: the TypeScript shapes of the evidence documents and the
  API constants (`LIMITS`, `ROLES`, `OPERATIONS`, `ERROR_CODES`).
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
just test                 # Node and Workers-runtime tests (local Miniflare D1)
just lint                 # strict TypeScript check
just capture-parity --check   # confirm the frozen fixtures still match Python
just inventory --out /tmp/evidence-inventory.json
CLOUDFLARE_API_TOKEN=… CLOUDFLARE_ACCOUNT_ID=… just prove-d1
```

`prove-d1` never logs in interactively. It exits 2 and names the missing
variables when it has no credentials. When it does run, it deletes the
database and Worker it created.
