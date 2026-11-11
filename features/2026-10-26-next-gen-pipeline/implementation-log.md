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
message_to_agent: |-
  Phase 1 did not pass its feasibility checkpoint; read decisions_for_review
  first. Wave 1 is done: tools/evidence-store is a pnpm workspace package
  (vitest 4 projects "node" and "workers", @cloudflare/vitest-plugin), with
  src/contracts/{api,evidence,json}.ts holding the shared constants and shapes,
  api/openapi.yaml + api/examples + api/schemas (kept in sync with
  src/contracts/api.ts by tests/node/api-contract.test.ts), and 268 frozen
  Python parity cases under tests/fixtures/parity (manifest.json lists every
  case; regenerate or verify with `just capture-parity [--check]`).
  docs/contract-inventory.md maps every field to its validator, its cases, and
  the robustness cases Phase 2 still has to add. The D1 conflict-guard design
  (BEFORE INSERT trigger raising identity_conflict, ON CONFLICT DO NOTHING for
  identical rows, accepted_request_id to tell accepted from already-present
  inside the same batch) is proven only on local Miniflare
  (tests/workers/d1-spike.test.ts); the deployed proof (`just prove-d1`) still
  needs CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID. Do not record
  feasibility success from local results.
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
