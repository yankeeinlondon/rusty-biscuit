---
spec: /Volumes/coding/rusty-biscuit/features/2026-10-26-next-gen-pipeline/spec.md
plan: features/2026-10-26-next-gen-pipeline/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
  - pnpm-workspace.yaml
  - pnpm-lock.yaml
  - tools/evidence-store/package.json
  - tools/evidence-store/tsconfig.json
  - tools/evidence-store/vitest.config.ts
  - tools/evidence-store/justfile
  - tools/evidence-store/src/contracts/api.ts
  - tools/evidence-store/src/contracts/evidence.ts
  - tools/evidence-store/src/contracts/index.ts
  - tools/evidence-store/src/contracts/json.ts
  - tools/evidence-store/cli/capture-parity.ts
  - tools/evidence-store/cli/inventory.ts
  - tools/evidence-store/cli/prove-d1.ts
  - tools/evidence-store/spike/d1-guard/schema.sql
  - tools/evidence-store/spike/d1-guard/worker.ts
  - tools/evidence-store/spike/d1-guard/scenarios.ts
  - tools/evidence-store/api/openapi.yaml
  - tools/evidence-store/api/schemas/
  - tools/evidence-store/api/examples/
  - tools/evidence-store/tests/node/api-contract.test.ts
  - tools/evidence-store/tests/node/inventory.test.ts
  - tools/evidence-store/tests/node/parity-fixtures.test.ts
  - tools/evidence-store/tests/node/prove-d1.test.ts
  - tools/evidence-store/tests/workers/d1-spike.test.ts
  - tools/evidence-store/tests/workers/env.d.ts
  - tools/evidence-store/tests/fixtures/parity/
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - tools/evidence-store/README.md
  - tools/evidence-store/api/README.md
  - tools/evidence-store/docs/contract-inventory.md
skills_files_updated_during_phase_1: []
packages:
  - "@rusty-biscuit/evidence-store"
decisions_for_review:
  - |-
    Phase 1 is reported as FAILED at the Wave 2 feasibility checkpoint. The plan
    requires both "Measure capacity" and "Prove D1" to pass before deployment
    work, and neither could: CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID are
    not set (prove-d1 exits 2 and records no result), the GitHub CLI token on
    this host is invalid and no GITHUB_TOKEN/GH_TOKEN is set (artifact metadata
    unmeasured), and the inventory's own no-pruning forecast puts R2 storage
    past the 80% refusal line in month 1 (about 9.8 GB against the 8 GiB cap,
    driven by a p95 report size near 1 MB). Wave 1 (API contract, parity
    fixtures, contract inventory) is complete and its tests pass. The
    alternative, recording success from local Miniflare D1, is forbidden by
    the plan. To clear it: provide the two Cloudflare variables and a valid
    GitHub token, rerun `just prove-d1` and `just inventory` in
    tools/evidence-store, and decide how the R2 forecast is admitted (for
    example store only the latest pass per cell until a retention verifier
    exists, cap report bytes, or accept the breach as an alert). Changing
    retention is an owner decision the plan forbids an agent to make.
  - |-
    Used @cloudflare/vitest-plugin 1.7.0 instead of the plan's "Vitest Workers
    pool" package (@cloudflare/vitest-pool-workers): the pool package's README
    marks it deprecated and renamed to vitest-plugin, and it needs vitest 4,
    so this package pins vitest 4.1.12 while tools/test-audit stays on 3.2.4.
    Reverting means swapping one devDependency and the vitest.config.ts import.
  - |-
    No service Worker or service wrangler.jsonc exists yet. Phase 1 adds only
    contracts plus a separate disposable spike Worker (spike/d1-guard) that
    prove-d1 deploys under a random name and deletes. The plan's development
    and restore-test Wrangler environments are left to the phase that builds
    the Worker, instead of shipping a config that points at a stub.
  - |-
    The parity capture tool is TypeScript (cli/capture-parity.ts) that pipes an
    embedded Python program to `python3 -I`, so no .py file is added while the
    frozen fixtures still come from the real scripts/ci/schema.py. The hosted
    cases use the helpers in scripts/ci/test_hosted_observations.py, so a change
    there shows as drift in `just capture-parity --check`. Alternative: freeze
    the fixtures once with an uncommitted one-shot command (not reproducible).
  - |-
    The D1 CPU sample in the spike is a proxy (JSON.parse + key-sorted
    serialization + SHA-256 at 256 observations and about 1 MiB), because the
    lossless Python-compatible canonicalizer is Phase 2 work. If the deployed
    proxy passes but the real port is slower, Phase 2 should rerun the sample
    with the port before advertising the 256-observation bound.
  - |-
    Added a fifth role, "retention-verifier", to ROLES alongside reader,
    producer, verifier, and operator, because the plan requires separately
    provisioned retention certificates. `just lint` is the strict TypeScript
    compiler (no ESLint is used anywhere in the workspace's TypeScript
    packages); adding ESLint later would be a new dependency decision.
  - |-
    Python contract quirks were frozen rather than corrected (see
    tools/evidence-store/docs/contract-inventory.md): integer values of 8 or
    more digits pass identity-field checks through str(); counts accept floats
    and booleans; failure_detail_truncated is never type-checked; `1e400`
    loads as infinity and canonicalizes to the non-JSON token Infinity; a
    string or null count part makes Python raise TypeError. Phase 2 must
    choose a located error for the TypeError case (it cannot crash) and should
    reject the Infinity case, since the plan says nonfinite values are refused;
    both are departures from Python that need recording.
  - |-
    Phase 2 went ahead although Phase 1's feasibility checkpoint failed. The
    plan says both feasibility tasks must pass "before service deployment
    work". Phase 2 deploys nothing: it adds the contract port, migrations, and
    local Workers-runtime tests. Stopping instead would leave every later
    phase blocked on credentials that the code work does not need. If the
    reviewer reads the checkpoint as blocking all later work, this phase's
    code stays valid, but deployment-facing claims must wait for `just
    prove-d1` and `just inventory` to pass.
  - |-
    Local CPU indicator: validating a 256-observation, ~0.9 MiB submission
    with the real port takes about 29 ms in Node on this Mac, against
    Workers Free's 10 ms CPU limit. Pure-JavaScript SHA-256 (@noble/hashes,
    used so identity checks stay synchronous inside the validator) is about
    11 ms of that. Realistic collections (p95 88 KB) need about 3 ms. The
    advertised limits were NOT lowered: the plan sets them from the deployed
    sample in the Prove D1 task, which is still blocked. Options when it
    runs:
    - lower the bounds to the largest passing power of two (for example
      256 KiB and 64 observations), as the plan prescribes;
    - compute identities with the runtime's native SHA-256 (`node:crypto`
      under nodejs_compat, or async Web Crypto, which means an async
      validator);
    - use Workers Paid CPU limits.
    Changing the hash backend touches only `src/contracts/canonical.ts`.
  - |-
    Phase 2 has no credential verification; that is Phase 3. `src/auth.ts`
    defines the `Principal` (a publisher record in the OpenAPI `Principal`
    shape, plus per-repository role grants). The deployable Worker's
    authenticator refuses every request with 401, and tests inject an
    authenticator through `createWorker({authenticate})`. The publisher is
    stored as JSON text of that record in `accepted_principal`, so export
    returns it as `acceptance.accepted_by`. Alternative: a free-form
    principal string. That would need a lookup to rebuild `accepted_by`.
    Phase 3 only has to supply a real authenticator.
  - |-
    Collection export needed working cursors in this phase, so
    `src/cursor.ts` (HMAC-SHA-256 over repository, restore generation,
    retention epoch, query digest, position, issue time) landed here rather
    than in Phase 3. Refusals:
    - a generation change answers 400 `invalid_cursor` ("predates a store
      restore");
    - a retention-epoch change answers 409 `snapshot_pruned`;
    - a cursor older than 24 h answers 409 `cursor_expired`.
    The export query digest includes `page_size`, so changing the page size
    mid-listing is refused instead of skipping or repeating members. Phase 3
    extends the cursor state with snapshot watermarks for history and change
    feeds.
  - |-
    The contract port departs from Python where Python would crash or accept
    what the plan forbids. All of these are recorded in
    docs/contract-inventory.md, and each has a named test.
    - `1e400` is refused as nonfinite (Python loads it as infinity).
    - A string, null, list, or object count part is a located
      `malformed-receipt` problem (Python raises TypeError).
    - A huge-integer-plus-float count overflow is a located problem (Python
      raises OverflowError).
    - A list or object `package`/`gate`/producer `kind`, or a non-string
      receipt `report_dir`, is a located problem (Python crashes).
    - The adapter reports every receipt problem or every unreadable report
      (Python stops at the first).
    - `~user` report directories are reported unreadable (Python looks the
      user up).
    - Nesting deeper than 1000 levels is refused (Python's limit is about
      10,000).
    Absolute and `..` report paths are joined exactly as Python does, without
    normalization or confinement, because the adapter runs on the submitting
    host and the plan says not to normalize historic paths. Plan rule 2 also
    names `_cell_consistency` and `_cell_backends`; neither is reachable from
    the receipt or collection validators, so they were not ported.
  - |-
    Storage choices the plan left open:
    - An unknown repository is detected inside the one batch: every insert
      selects FROM repositories, so nothing can be written, and the missing
      revision row answers 404.
    - An empty collection stores its manifest (count 0) so it can be exported
      exactly, but never advances the revision or emits events.
    - The revision advances by one per newly inserted observation.
    - `collection_members` has no foreign key to `observations`, so a pruned
      member stays listed and export answers 410 `collection_pruned`.
    - A request with several contract problems answers with the first
      problem's location and message plus "(and N more problems)", because
      the published ErrorBody has one location. Returning all of them would
      need an OpenAPI change.
    - `source_version: 2` with zero observations is refused (a version-2
      receipt always has at least one cell).
  - |-
    Service configuration reaches the Worker as one text binding,
    SERVICE_CONFIG (the service-config schema as JSON). A missing or invalid
    document answers 503 to every request, rather than running with a partial
    allowlist. Its `repositories` list is a second gate on top of D1
    registration: a host grant on an unlisted repository is inert, an OIDC
    token must match a listed repository and owner, and registering a
    repository must match a listed entry (otherwise 403). wrangler.jsonc ships
    empty allowlists, which deny everything until Phase 6 fills in the real
    repository and workflow. Alternative: drop `repositories` from the config
    and trust D1 registration alone; that would mean deleting the allowlist
    checks in src/auth.ts and src/operator.ts plus their tests.
  - |-
    Rate limiting uses three Cloudflare rate-limit bindings (READ_LIMITER 300,
    MUTATION_LIMITER 60, OPERATOR_LIMITER 30 per 60 s, keyed by the
    authenticated principal). A missing binding answers 503 (never unmetered).
    Query POSTs (history, changes, audit) count as reads. The config's `quotas`
    rate fields are validated but not read at runtime, because binding limits
    are fixed in wrangler.jsonc; the two must be kept equal by hand.
    Alternative: remove the rate fields from the config schema, or meter in D1
    (costs a row write per request against the 100,000/day free allowance).
  - |-
    OIDC refusals are split: a token failing signature, issuer, audience, or
    time checks answers 401; a valid token from an untrusted workflow, ref,
    event, owner, repository, or pull-request/fork context answers 403. The api
    README previously listed every OIDC failure under 401; it now documents the
    split. Authenticated principals denied a role are audited best-effort as
    `credential.denied` (an audit write failure never turns the 403 into a
    503); auth-time refusals are not audited because no trusted identity
    exists. Alternative: 401 for everything (one test table to flip).
  - |-
    Migration 0003 (never deployed) gained `repository_ids_json` and
    `roles_json` on `credentials`, so a credential issued with empty lists still
    reports its requested scope. Authorization reads only `credential_grants`.
    Audit `actor` is stored as the principal JSON (the same text as an
    observation's publisher), and `credential.issue` writes one audit row per
    granted repository so the repository filter finds it. Registering over a
    row whose owner is NULL answers 409 (owners never change).
  - |-
    appendLocation (verifier-only location events) was built in Phase 3,
    because the read checkpoint (withdrawal replay, newest-location-wins,
    location feed) needs it and no later task names it. Object locators: an
    `uploads` row for the same reference must exist (else 404), and `verified`
    with an object locator is refused with 400, because only upload
    finalization (Phase 4) may verify object bytes. A distinct location is
    (reference_kind, reference_digest, path, location_key); path is included
    so two files with identical bytes are separate locations.
  - |-
    Change-feed replay horizons were added as migration 0006
    (`observation_event_horizon`, `location_event_horizon`, both 0 now). An
    `after_sequence` below the horizon, or an absent one once the horizon is
    above 0, answers 409 `cursor_expired` instead of silently starting at the
    horizon. Phase 5's tombstone expiry must advance these columns. A location
    event whose row was later pruned is rendered in the location feed as a
    `pruned` item with the tombstone's job_id. A pinned snapshot whose
    retention epoch or watermarks are ahead of the store answers 400; only an
    older epoch is 409 `snapshot_pruned`, and another generation is 409
    `stale_snapshot`.
  - |-
    The operator CLI's bootstrap runs Wrangler as `node
    node_modules/wrangler/bin/wrangler.js d1 execute … --remote --command
    <SQL>` (no shell, so no OS-specific quoting). The SQL carries only the
    token digest. The bootstrap audit row names the new operator credential as
    its own actor. On Windows the token file is restricted with
    `icacls /inheritance:r /grant:r %USERNAME%:F` before the secret is written;
    that path is untested because the cross-OS rigs run Cargo packages only.
    No deployed run was possible: the Cloudflare and GitHub credentials are
    still unset.
source_files_during_phase_2:
  - tools/evidence-store/vitest.config.ts
  - tools/evidence-store/wrangler.jsonc
  - tools/evidence-store/migrations/0001_evidence.sql
  - tools/evidence-store/migrations/0002_retention.sql
  - tools/evidence-store/migrations/0003_credentials.sql
  - tools/evidence-store/migrations/0004_uploads.sql
  - tools/evidence-store/migrations/0005_evaluations.sql
  - tools/evidence-store/src/auth.ts
  - tools/evidence-store/src/cursor.ts
  - tools/evidence-store/src/http.ts
  - tools/evidence-store/src/ingestion.ts
  - tools/evidence-store/src/worker.ts
  - tools/evidence-store/src/db/ingest.ts
  - tools/evidence-store/src/db/repositories.ts
  - tools/evidence-store/src/contracts/adapter.ts
  - tools/evidence-store/src/contracts/canonical.ts
  - tools/evidence-store/src/contracts/index.ts
  - tools/evidence-store/src/contracts/json.ts
  - tools/evidence-store/src/contracts/loader.ts
  - tools/evidence-store/src/contracts/node-report-reader.ts
  - tools/evidence-store/src/contracts/parse.ts
  - tools/evidence-store/src/contracts/validate.ts
  - tools/evidence-store/tests/node/contracts-parity.test.ts
  - tools/evidence-store/tests/node/contracts-robustness.test.ts
  - tools/evidence-store/tests/service/apply-migrations.ts
  - tools/evidence-store/tests/service/db-ingest.test.ts
  - tools/evidence-store/tests/service/db-migrations.test.ts
  - tools/evidence-store/tests/service/env.d.ts
  - tools/evidence-store/tests/service/helpers.ts
  - tools/evidence-store/tests/service/ingestion.test.ts
docs_updated_during_phase_2:
  - tools/evidence-store/README.md
  - tools/evidence-store/docs/contract-inventory.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - tools/evidence-store/src/env.ts
  - tools/evidence-store/src/requests.ts
  - tools/evidence-store/src/http.ts
  - tools/evidence-store/src/worker.ts
  - tools/evidence-store/src/auth.ts
  - tools/evidence-store/src/config.ts
  - tools/evidence-store/src/tokens.ts
  - tools/evidence-store/src/credential-request.ts
  - tools/evidence-store/src/operator.ts
  - tools/evidence-store/src/reads.ts
  - tools/evidence-store/src/locations.ts
  - tools/evidence-store/src/cursor.ts
  - tools/evidence-store/src/ingestion.ts
  - tools/evidence-store/src/db/audit.ts
  - tools/evidence-store/src/db/reads.ts
  - tools/evidence-store/src/db/ingest.ts
  - tools/evidence-store/cli/operator.ts
  - tools/evidence-store/migrations/0003_credentials.sql
  - tools/evidence-store/migrations/0006_feed_horizons.sql
  - tools/evidence-store/wrangler.jsonc
  - tools/evidence-store/vitest.config.ts
  - tools/evidence-store/justfile
  - tools/evidence-store/tests/service/env.d.ts
  - tools/evidence-store/tests/service/helpers.ts
  - tools/evidence-store/tests/service/auth.test.ts
  - tools/evidence-store/tests/service/operator.test.ts
  - tools/evidence-store/tests/service/reads.test.ts
  - tools/evidence-store/tests/service/changes.test.ts
  - tools/evidence-store/tests/service/locations.test.ts
  - tools/evidence-store/tests/service/db-migrations.test.ts
  - tools/evidence-store/tests/service/ingestion.test.ts
  - tools/evidence-store/tests/node/requests.test.ts
  - tools/evidence-store/tests/node/service-config.test.ts
  - tools/evidence-store/tests/node/operator-cli.test.ts
  - tools/evidence-store/tests/node/credential-request.test.ts
docs_updated_during_phase_3:
  - tools/evidence-store/README.md
  - tools/evidence-store/api/README.md
  - tools/evidence-store/docs/contract-inventory.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
message_to_agent: |-
  Phase 3 is implemented; `just test` (668 at first gate run) and `just
  lint` pass in tools/evidence-store. Phase 1's deployed feasibility
  checkpoint is still open (Cloudflare/GitHub credentials unset), and the
  Phase 2 tree is still uncommitted alongside Phase 3.
  Seams for Phase 4:
  - Routes live in `src/worker.ts`: add upload/finalize/fetch to
    `matchRepositoryRoute` with their OperationId and rate class; roles come
    from `OPERATIONS` and are already enforced before your handler runs.
  - Parse JSON bodies with `src/requests.ts` readers (strict, located 400s).
  - `src/locations.ts` refuses `verified` object locators on purpose: finalize
    must insert the verified `object` location itself (reuse its insert, and
    `renderLocationEvent` in `src/reads.ts` with `readLocationEvent` in
    `src/db/reads.ts`).
  - Real auth: `createAuthenticator({ jwks })` in `src/auth.ts`; tokens in
    `src/tokens.ts`. Tests mostly inject principals via `harness`/`principal`
    in `tests/service/helpers.ts`.
  - `SERVICE_CONFIG` and three rate-limit bindings are now required bindings
    (vitest.config.ts and wrangler.jsonc have them).
  - For the Phase 4 CLI: `cli/operator.ts` has reusable owner-only file
    writing, strict file loading, and the API error printing; Windows icacls
    behavior is untested.
  Migrations 0001-0006 are still undeployed and may be edited until the first
  deployment applies them.
---

# Implementation Log for 2026-10-26-next-gen-pipeline (6 phases)

## Phase 1

- 2026-11-11: preflight. `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` are
  not set; the installed global `wrangler` is v1.19.12 and `wrangler whoami`
  fails; `gh auth status` reports the stored token as invalid and neither
  `GITHUB_TOKEN` nor `GH_TOKEN` is set. Remote notes refs are readable over SSH
  (`git ls-remote origin 'refs/notes/*'`). Wave 1 needs none of the missing
  credentials; Wave 2's deployed checkpoint does.
- Package foundation: `tools/evidence-store/` added to `pnpm-workspace.yaml` as
  private `@rusty-biscuit/evidence-store`, with exact pins (`jose` 6.2.17,
  `@noble/hashes` 2.4.0, `wrangler` 4.160.1, `vitest` 4.1.12,
  `@cloudflare/vitest-plugin` 1.7.0, `@cloudflare/workers-types`
  5.20261110.1, `yaml` 2.9.1, and the test-audit toolchain pins for
  `typescript`, `@types/node`, `tsx`). The lockfile diff only adds packages:
  no existing package's resolution was removed or changed.
- D1 spike (`tools/evidence-store/spike/d1-guard/`, `cli/prove-d1.ts`):
  - schema = the d1-model tables plus `collections`, `collection_members`,
    `observation_events`, an `accepted_request_id` column, and the
    `observations_identity_guard` BEFORE INSERT trigger
    (`RAISE(ABORT, 'identity_conflict')` when payload/environment/head/tree
    differ). One `batch()` inserts observations (via `json_each(?)`), the
    manifest, and membership, then selects which IDs this request inserted;
    identical rows fall through to `ON CONFLICT DO NOTHING` and fire no
    revision/event trigger.
  - scenarios (`spike/d1-guard/scenarios.ts`) shared by the local test and the
    deployed driver: idempotent replay, sequential conflict rollback, late
    statement (foreign key) failure rollback, generated-column CHECK and
    immutability trigger, N concurrent conflicting pairs with fresh siblings,
    query catalog q1/q3/q4/q5/q7/q8/q9 (current and stale)/q10/q13/q14/q15 with
    bound parameters, and a CPU sample at the request bound.
  - local Miniflare run (`tests/workers/d1-spike.test.ts`, 4 pairs, 16-member
    CPU sample): every check passes; the conflict answers 409 and leaves no
    sibling, manifest, member, event, or revision change. This is local SQLite
    and is **not** a feasibility result.
  - deployed run: `pnpm --dir tools/evidence-store exec tsx cli/prove-d1.ts
    --out /tmp/evidence-prove-d1.json` → **exit 2**: "set CLOUDFLARE_API_TOKEN
    and CLOUDFLARE_ACCOUNT_ID (no interactive login is attempted). No
    feasibility result was recorded." The **Prove D1 checkpoint has not
    passed.**
- Capacity inventory (`cli/inventory.ts`): `pnpm --dir tools/evidence-store
  exec tsx cli/inventory.ts --out /tmp/evidence-inventory.json` → **exit 1**,
  `feasible: false`. Redacted results (2026-11-11):
  - remote refs: `refs/notes/ci-local/{macos-latest,scope,windows-latest,wsl2-ubuntu}`;
    no `ci-remote/*` refs on origin.
  - notes/observations: macos-latest 158 notes, 1,653 observations;
    windows-latest 22 / 875; wsl2-ubuntu 12 / 418 (2,946 total: 1,900 from v3
    collections, 1,046 from v2 receipt cells). 4 unparseable notes, all v1
    receipts on macos-latest (counted, not dropped). 211 observed cells
    (macos 74, windows 71, wsl2 66).
  - sizes (bytes, p50 / p95 / max): observation canonical 1,040 / 1,413 /
    3,390; v3 collection 15,511 / 88,470 / 156,811; report 33,518 /
    1,005,371 / 2,451,110 (1,826 sized: 573 host-dir, 1,253 artifact; 891 v2
    reports unsized and not on this host); proof (47 local files) 921 /
    615,392 / 1,251,470. On this host 24 reports present (11.5 MB) and 1,430
    missing (139.8 MB recorded).
  - daily attempts over 30 days: 1,922 observations, mean 64.07/day, max
    387/day (2026-10-16); unresolved cells stayed at 2 (windows-latest).
  - 12-month forecast, p95 sizes and 2x daily volume, **no pruning**: D1
    140.6 MB of 500 MB (28%, no alert); D1 rows written 6,072/day of 100,000;
    R2 Class A 20,274/month of 800,000; **R2 bytes 70.9 GB against the 8 GiB
    cap — crosses 70% and 80% in month 1** (9.76 GB).
  - failures reported by the command, verbatim:
    1. `missing required source github-actions: GITHUB_TOKEN and GH_TOKEN are unset and `gh auth status` exited 1 (token in default is invalid)`
    2. `missing required source cloudflare-account-usage: account-wide D1/R2 consumption is unmeasurable: CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID unset`
    3. `projected breach of r2_live_plus_staging_bytes: crosses 80% of 8589934592 in month 1`
  - context for the reviewer (not a change to the failing result): the R2
    breach comes from the long report tail (p95 ≈ 982 KiB against p50 ≈ 33
    KiB) multiplied by every observation with no pruning. Under the ruled
    retention, live bytes are bounded by retained cells plus the 14-day
    grace window — roughly 128 obs/day × 14 days × (p95 report + proofs)
    ≈ 2–2.5 GB plus ≈ 0.3 GB for retained cells — but the plan provisions no
    production retention verifier in this stage, so the no-pruning interval
    is what the checkpoint must admit.
- Define API (done): `tools/evidence-store/api/openapi.yaml` (OpenAPI 3.1, 18
  operations, `x-roles` per operation, `x-limits`, one shared `ErrorBody`),
  five strict configuration schemas under `api/schemas/`, 32 examples under
  `api/examples/` (the observation in them validates under
  `schema.validate_collection` and carries its real identity), and
  `api/README.md` (role matrix, limits, errors, credential lifecycle, OIDC
  allowlist, evaluation publication disabled). Choices the plan left open,
  recorded in the OpenAPI document: decimal-string repository/owner IDs;
  32-hex upload/job/credential IDs; caller-supplied UUIDs for location events
  and retention certificates; `location_key` = SHA-256 of the canonical
  locator; an `after_sequence`/`last_sequence` resume position for change
  feeds older than the 24-hour cursor (still bounded by the 30-day tombstone
  window); finalize appends the `verified` object location itself; 201 on
  first create and 200 on identical replay except ingestion (always 200);
  `operator` can never be granted through the API.
- Trace parity (done): `cli/capture-parity.ts` regenerates
  `tests/fixtures/parity/` (268 cases: 13 canonical, 9 collection, 6
  receipt-adapter, 35 loader, 205 validator; 297 files, ≈336 KB) from the real
  `schema.py`, `observation_notes.receipt_observations`/`collection_of`, and
  `hosted_observations.build_observation`; `--check` reports drift.
  `docs/contract-inventory.md` maps every field to its validator, its cases,
  and the robustness cases Phase 2 must add, and lists the legacy
  acceptances the port must preserve (see decisions_for_review).
- Requirement → test mapping for this phase:
  - API constants and the published contract agree →
    `tests/node/api-contract.test.ts` (5 tests: operations/roles, error codes,
    limits, examples name real operations, schemas are closed).
  - frozen parity ground truth is self-consistent and still matches Python →
    `tests/node/parity-fixtures.test.ts` (7 tests, including
    `capture-parity --check`, skipped with a visible reason when `python3` is
    absent).
  - capacity inventory arithmetic, missing-source failure, unparseable-note
    counting, empty-scan failure, git note walk →
    `tests/node/inventory.test.ts` (8 tests).
  - prove-d1 fails closed and names the missing variables without recording a
    result (one spawned wiring case), Wrangler output parsing →
    `tests/node/prove-d1.test.ts` (4 tests).
  - in-batch conflict guard, rollback, idempotent replay, generated columns,
    triggers, query catalog, CPU sample path → `tests/workers/d1-spike.test.ts`
    (2 tests, local Miniflare only; the deployed equivalent is `just
    prove-d1`, not run).
  - Input Robustness Matrix: Phase 1 adds no loader or validator, so it
    applies to the frozen cases only; the validator and loader cells are
    captured as Python ground truth for Phase 2 to test against.
- Gates (2026-11-11, macOS, no subagent running):
  - `tools/evidence-store`: `just test` → 5 files, 26 tests passed; `just
    lint` (strict `tsc --noEmit`) → exit 0.
  - Scope calculator over the whole uncommitted tree selects `repo-deps`,
    `test-toolkit`, `darkmatter`, `sniff` (change_class `package`), but the tree
    also carries unrelated uncommitted rust-pre-push work under `scripts/`.
    Restricted to this phase's 362 paths, it selects only `test-toolkit` (L1
    + lint, from `pnpm-workspace.yaml`/`pnpm-lock.yaml`) and one narrowed
    `sniff` L1 test.
  - `just test test-toolkit` → 533 passed, 4 skipped (5 slow tests at or above
    5 s, all pre-existing archive-guard/kache/workflow-contract tests);
    companion suites `pnpm --dir tools/test-audit typecheck` → exit 0 and
    `pnpm --dir tools/test-audit test` → 189 passed, 29 skipped;
    `cargo clippy -p test-toolkit --all-targets -- -D warnings` → exit 0;
    `just archive-path-guard` → 2 passed.
  - `sniff` narrowed cell
    `filesystem::repo::lockfile::pnpm::tests::typed_parse_matches_the_generic_reference_on_the_workspace_lockfile`
    → passed.
  - Cross-OS: no Cargo package changed and the new code is Node tooling, so
    `just cross-check` (Cargo-package based) does not apply; the CLI
    cross-OS requirement is Phase 4's.
- **Phase 1 outcome: FAILED at the Wave 2 feasibility checkpoint.** Wave 1
  tasks are complete and checked off; "Measure capacity" and "Prove D1" stay
  unchecked because neither checkpoint passed (missing Cloudflare and GitHub
  credentials, and a projected R2 breach in month 1 with no pruning). The
  plan forbids deployment work until both pass.

## Phase 2

- 2026-11-11: preflight. `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` are
  still unset, so Phase 1's deployed feasibility checkpoint is still open. Plan
  says that checkpoint gates "service deployment work". Phase 2 deploys nothing
  (local validation, migrations, and Workers-pool tests only), so it proceeded.
  See decisions_for_review.
- Wave 3 ran as two parallel subagents with disjoint files; Wave 4 was done
  afterwards in the main session.
- **Port contracts** (done): `src/contracts/{loader,canonical,validate,adapter,parse,node-report-reader}.ts`.
  - `loadStrict` is a lossless tokenizer: number lexemes are kept, objects
    have no prototype (`__proto__` is an ordinary key), and duplicate keys,
    BOM, blank input, trailing content, NaN/Infinity, overflowing floats, and
    invalid UTF-8 are refused with loader codes and JSON pointers.
  - `canonical`/`identity`/`observationIdentity`/`collectionIdentity` match
    Python `json.dumps(sort_keys=True, separators=(",",":"))` (code-point key
    order, ASCII escaping, `repr(float)` numbers).
  - `validateCollection`/`validateReceipt`/`validateObservation`/`receiptCellProblems`
    reproduce the exact Python problem strings; `code` is the text before
    the first colon, and every problem has a JSON pointer.
  - `adaptReceipt` is pure, with an injected report reader. The Node reader
    (`node-report-reader.ts`, the only `node:`-importing contract file)
    expands `~` like `Path.expanduser`.
  - `parseCollection`/`parseReceipt` are the public byte-level entry points.
  - All 268 frozen parity cases pass (`tests/node/contracts-parity.test.ts`,
    270 tests including two guards: per-kind case counts, and named
    departures).
  - `tests/node/contracts-robustness.test.ts` (108 tests) covers every
    former "Port still needs" inventory row, the shared-validator shape matrix,
    the loader document shapes once, a wiring case per public consumer, and
    adapter behavior (nothing produced on malformed input, every unreadable
    report located, incomplete cells never read, absolute/`..` report paths
    preserved as Python resolves them).
  - Departures are recorded in `docs/contract-inventory.md` ("Departures from
    the Python") and in decisions_for_review.
- **Build migrations** (done): `migrations/0001_evidence.sql` …
  `0005_evaluations.sql`, `src/db/{ingest,repositories}.ts`, `wrangler.jsonc`
  (environments `development`, `restore-test`; placeholder IDs, nothing
  deployed), a third vitest project `service` (workerd, D1 `EVIDENCE_DB` from
  `migrations/`, R2 `EVIDENCE_BUCKET`, `CURSOR_HMAC_KEY`).
  - Schema: the identity-guard trigger from the spike, generated cell columns,
    first-publisher columns outside identity equality, immutable
    collections/members/location events, event streams with trigger-maintained
    watermarks, retention certificates/jobs/members/locks/release pins with a
    delete guard (only members of a `deleting-rows` job; pruned tombstones in
    the same statement), credentials/grants/audit, uploads/parts/quota
    reservations, and evaluations with no writer.
  - `tests/service/db-migrations.test.ts` and `db-ingest.test.ts` (24 tests).
- **Integrate ingestion** (done): `src/worker.ts` (native routing,
  `createWorker({authenticate})`, request IDs, every non-`ApiError` failure →
  503 `unavailable` without detail), `src/ingestion.ts` (`parseSubmission`
  applies the 1 MiB body, 256-observation, and 128 KiB canonical-observation
  bounds plus the full contract before any write; `ingest` runs one atomic
  batch; `exportCollection` returns exact member pages with stored canonical
  text spliced verbatim), `src/cursor.ts` (HMAC-SHA-256 cursors binding
  repository, generation, retention epoch, query digest, position; 24 h
  expiry), `src/http.ts` (error envelope from `ERROR_CODES`), `src/auth.ts`
  (`Principal`, fail-closed `refuseAll`, role/grant `authorize`).
  - `tests/service/ingestion.test.ts` (25 tests, through the Worker `fetch`,
    frozen authentic fixtures with one edit per case).
  - Two mutation checks were run: validation disabled, and the
    manifest-level pruned check disabled. Each made a test fail. The second
    check found that the pruned test was masked by the page-level check, so a
    later-page pruned case was added.
- CPU indicator (local Node on this Mac, **not** the deployed sample): a 256-
  observation, 935,258-byte submission takes ~29 ms in `parseSubmission`
  after adding a no-escape fast path to the canonical string writer (was ~48
  ms). Breakdown: load 6 ms, validate 12 ms (includes per-observation SHA-256),
  whole canonical 2.5 ms, whole SHA-256 (@noble/hashes) 5.6 ms. This is well
  above Workers Free's 10 ms CPU limit at the declared bound. Realistic
  collections (p95 88 KB per Phase 1's inventory) are about a tenth of that.
  The advertised limits were **not** changed. The plan sets them from the
  deployed sample, which is still blocked on credentials.
- Requirement → test mapping:
  - strict loader document shapes (duplicate key, trailing/concatenated,
    nonfinite, invalid UTF-8, BOM, blank, JSON5/YAML/JSONL) →
    `contracts-robustness.test.ts` "shared loader" block, once. Wiring is
    covered by `parseCollection`/`parseReceipt` tests, and on the HTTP path
    by `ingestion.test.ts` "refuses loader failures through the shared strict
    loader".
  - Python parity (canonical bytes, identities, validator messages/codes,
    adapter output) → `contracts-parity.test.ts`, one test per manifest case.
  - field-shape matrix per shared validator → `contracts-robustness.test.ts`.
  - adapter: malformed input publishes nothing → `contracts-robustness.test.ts`
    "adapter" block.
  - submission envelope (unknown key, missing collection, version 3.0/true/1/
    null, non-object) → `ingestion.test.ts` "refuses submission envelope
    shapes". Every one asserts zero writes.
  - bounds before mutation (1 MiB body, 257 observations, >128 KiB observation)
    → `ingestion.test.ts`, 413 with location and zero writes.
  - atomic conflict rollback, concurrent duplicate, concurrent conflicting
    pair with siblings, mixed batches, empty collection, unknown repository,
    preserved first publisher/timestamp/source version (including replay
    through the other adapter) → `db-ingest.test.ts` (storage layer) and
    `ingestion.test.ts` (Worker `fetch`).
  - exact export: write → read → write → read round trip, multi-page export
    rebuilding the collection ID, v2 provenance → `ingestion.test.ts`.
  - export pruning → 410 on the first page and on a later page →
    `ingestion.test.ts`.
  - cursor refusals (tampered, other query, other repository, malformed,
    expired, after pruning, after restore generation change) and `page_size`
    validation → `ingestion.test.ts`.
  - deletion guard, release-pin guard, immutability, generated-column checks
    → `db-migrations.test.ts`/`db-ingest.test.ts`.
  - roles/grants (403), fail-closed deployed build (401), unsupported API
    version, unknown routes, storage failure → retryable 503 without detail →
    `ingestion.test.ts`.
- Code-smell search (`??`, `.filter(`, `catch`, defaults) over the new code:
  - `options.now ?? Date.now` is the clock seam.
  - `segments[2] ?? ""` falls through to the repository-ID regex, which
    answers 404.
  - The `.filter` calls in `src/db/ingest.ts` split accepted from
    already-present rows; they are not dropping invalid input.
  - The contract-port hits are listed in its inventory as Python equivalents.
- Gates (2026-11-11, macOS, no subagent running), in `tools/evidence-store`:
  `just lint` → exit 0; `just test` → 10 files, 453 tests passed, none
  skipped (`python3` present, so `capture-parity --check` ran).
- Cross-OS: no Cargo package changed and the code is Worker/Node TypeScript,
  so `just cross-check` does not apply. `node-report-reader.ts`'s Windows
  `USERPROFILE`/drive-path behavior is untested on Windows. It is first used
  by the Phase 4 CLI, whose cross-OS requirement covers it.
- Scope calculator over the uncommitted tree (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`) → `packages: []`, `change_class:
  documentation`. CI's scope registry covers Cargo packages, and
  tools/evidence-store is a pnpm tool outside it. The gates were run anyway,
  because this phase lists source files. No other package depends on
  `@rusty-biscuit/evidence-store`, so there were no dependent packages to
  test.
- **Phase 2 outcome: implementation complete.** The three Phase 2 tasks
  (Port contracts, Build migrations, Integrate ingestion) are checked off.
  Phase 1's Wave 2 feasibility tasks stay unchecked; they are blocked on
  credentials.

## Phase 3

- 2026-11-11: preflight. `CLOUDFLARE_API_TOKEN`/`CLOUDFLARE_ACCOUNT_ID` are
  still unset, so Phase 1's deployed feasibility checkpoint stays open. Phase 3
  deploys nothing (Worker code, migrations, CLI, and local Workers-pool
  tests), so it proceeded on the same basis as Phase 2. The Phase 2 tree is
  still uncommitted, so the working tree holds Phases 2 and 3 together.
- Shared skeleton written in the main session before fan-out:
  `src/env.ts` (bindings, now including `SERVICE_CONFIG` and the three rate
  limiters), `src/requests.ts` (strict request-field readers: unknown key,
  explicit null, wrong type, range, and array elements decided once),
  `readBody` moved into `src/http.ts`, and `src/worker.ts` rewritten as a
  route table. The order of checks is: API version, configuration,
  authentication, role on the repository (operator routes need an operator
  credential), per-principal rate, handler. `publishEvaluation` answers 403
  `evaluation_publication_disabled` to every authenticated caller before its
  body is read. Bindings were added to `vitest.config.ts` and both Wrangler
  environments.
- Wave 5 ran as two parallel foreground subagents with disjoint files, then
  was integrated here (one migration-list assertion updated for 0006, one
  stale test name fixed).
- **Enforce identities** (done): `src/{auth,config,tokens,credential-request,operator}.ts`,
  `src/db/audit.ts`, `cli/operator.ts`, a `just operator` recipe, and 0003
  scope columns.
  - Host tokens `ebt_<id>.<secret>`: the SHA-256 digest is checked with a
    constant-time comparison against primary D1 on every request. An expiry
    or revocation applies on the very next request. Rotation shortens the
    replaced credential to at most 7 days.
  - GitHub OIDC is verified with `jose`: exact issuer and audience, RS256,
    60 s tolerance, `workflow_ref` pinning, refusal of pull-request and fork
    contexts, the configured workflow/ref/event allowlist, and the owner ID
    both in the configuration and in D1.
  - Operator routes: register, rename, issue, revoke, and audit listing. The
    audit records credential IDs and principals, never a token or digest.
  - Operator CLI: bootstrap through Wrangler (digest only), register from
    GitHub's stable IDs, issue (the secret goes only to an exclusive
    owner-only file), and revoke.
- **Implement reads** (done): `src/{reads,locations}.ts`, `src/db/reads.ts`,
  `src/cursor.ts` (an optional signed `bounds` field, still one cursor
  format), `readObservation` now taking the latest `accepted` event, and
  migration `0006_feed_horizons.sql`.
  - Endpoints built: snapshot, observation detail (retry successors, newest
    event per location), candidate history over caller-supplied heads, two
    change feeds with separate cursors and replay horizons, and verifier-only
    location events with idempotent replay and 409 on a conflicting replay.
  - Every list is bounded by records and by 1 MiB encoded, and every later
    page is bounded by the captured snapshot.
- Requirement → test mapping:
  - shared request readers (field-shape matrix once, loader document shapes
    once) → `tests/node/requests.test.ts` (11 tests).
  - service configuration loader (control from the schema's example, field
    matrix, document shapes, 503 wiring) → `tests/node/service-config.test.ts`
    and `tests/service/auth.test.ts`.
  - credential request validator (matrix once; wiring per consumer: API
    `/roles/0` and grant file `/roles/0`) → `tests/node/credential-request.test.ts`,
    `tests/service/operator.test.ts`, and `tests/node/operator-cli.test.ts`.
  - token expiry, revocation, rotation overlap, wrong or unknown secret,
    malformed or missing header, cross-repository denial, unlisted
    repository, empty grants, operator/host route separation, and forged
    provenance (the export's `accepted_by` is the authenticated credential)
    → `tests/service/auth.test.ts` "host tokens".
  - every role × every routed repository operation (40 cases): denied roles
    get 403 with no evidence written and a `credential.denied` audit row;
    `publishEvaluation` is 403 for every role → `auth.test.ts` "every role
    against every routed repository operation".
  - OIDC: trusted control, wrong issuer/audience, expired beyond and within
    tolerance, future nbf, foreign key, `alg: none`/HS256, wrong
    ref/workflow/event/owner/repository, `pull_request`/`pull_request_target`,
    nonempty `head_ref`/`base_ref`, non-numeric IDs, empty allowlist,
    verifier never granted, JWKS failure → 503 → `auth.test.ts` "GitHub OIDC"
    (signed test JWTs with a local JWKS; GitHub is never contacted).
  - rate limit 429 with retry guidance, missing limiter 503, redaction (no
    secret or digest in any response body or audit row) → `auth.test.ts`.
  - register/issue/revoke/audit semantics and audit content →
    `tests/service/operator.test.ts`.
  - CLI: secret file mode 0600 and never printed; refusal before network;
    GitHub IDs; missing environment variables → exit 2; bootstrap digest-only
    SQL and cleanup; spawned control plus two malformed grant files →
    `tests/node/operator-cli.test.ts`.
  - snapshot, observation detail, 404 lookup versus 200 empty history,
    history scope (failures, incomplete attempts, every environment), page
    and byte limits, `unpageable_record`, 257 heads → 413, cursor swaps,
    generation/epoch/expiry refusals, pinned snapshots, writes during a
    listing never entering it, and pruned members → `tests/service/reads.test.ts`.
  - change feeds: fields, resume and replay, `last_sequence`, watermark and
    horizon refusals, stream and cursor swaps, snapshot isolation, repository
    isolation, withdrawal replay, and pruned tombstones in both streams →
    `tests/service/changes.test.ts`.
  - location events: 201/200 replay keeps the original record, 409 conflict
    with no write, reference matching for reports and both proof files,
    `verification_method` rules, locator shapes, object-locator rules, and
    producer/reader → 403 with nothing written → `tests/service/locations.test.ts`.
  - Subagent mutation checks: removing the allowlist filter, removing the
    `head_ref`/`base_ref` check, and breaking credential reads each made a
    test fail; all were restored.
- Plan checkpoint: every route enforces repository and role grants (role
  matrix). No producer can create a location attestation (`locations.test.ts`
  and the role matrix) or an evaluation (403 for every role). Cursor tests
  prove no new write enters an old snapshot (`reads.test.ts` and
  `changes.test.ts`).
- Code-smell search (`??`, `.filter(`, `catch`, defaults) over the new code:
  - every `??` default follows a reader that refuses explicit null, so a
    default applies only to an absent field: `page_size` → 100,
    `expires_in_days` → 90, `after_sequence` → origin, and audit filters
    absent → none. These are listed in `docs/contract-inventory.md`
    ("Service requests and configuration").
  - `src/db/audit.ts` swallows its own failure only for the best-effort
    denial audit.
  - the `.filter` in `src/locations.ts` selects candidate reports and does
    not drop input.
  - the CLI's `catch` blocks rethrow located errors.
- Deferred, with reasons:
  - Rate limits use Cloudflare bindings; configuration quotas are validated
    but not read at runtime (see decisions_for_review).
  - Nothing was deployed or run against real Cloudflare or GitHub (no
    credentials).
  - The Windows ACL path of the CLI is untested.
- Gates (2026-11-11, macOS, run once after both subagents finished and again
  after the docs/log edits, nothing else running), in `tools/evidence-store`:
  `just lint` → exit 0; `just test` → 19 files, 668 tests passed, none
  skipped, about 2 s.
- Scope calculator over the uncommitted tree (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`) → `packages: []`, `change_class:
  documentation`. The pnpm tool sits outside CI's Cargo scope registry. The
  gates ran anyway because this phase lists source files. No package depends
  on `@rusty-biscuit/evidence-store`, so there were no dependents to test.
- Cross-OS: no Cargo package changed, so `just cross-check` does not apply.
  The Worker runs only in workerd. The operator CLI is Node with no shell
  quoting; its Windows ACL step is untested (Phase 4's CLI cross-OS
  requirement should cover it).
- **Phase 3 outcome: implementation complete.** "Enforce identities" and
  "Implement reads" are checked off. Phase 1's Wave 2 feasibility tasks stay
  unchecked; they are still blocked on credentials.
