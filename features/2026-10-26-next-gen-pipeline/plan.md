---
kind: plan
total_phases: 6
created: 2026-11-08
phase: 2
agent: codex/gpt-6.1-sol
yolo: true
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
- '@rusty-biscuit/evidence-store'
---

# Evidence store implementation plan

## Summary and Definition of Done

Build a private TypeScript Cloudflare Worker backed by D1 metadata and R2 Standard report storage. Deliver strict compatibility with existing evidence, atomic ingestion, scoped authentication, snapshot-bounded reads, recoverable uploads, durable change feeds, guarded retention, and reproducible deployment and recovery tools. Explicit ingestion demonstrates both host and hosted evidence; existing producers, Git-note publication, scheduling, and reuse remain unchanged.

Completion means focused automated contract tests pass, the development service has demonstrated concurrent rollback and host/hosted ingestion, and an isolated restore retrieves the acknowledged bytes while rejecting old cursors. OpenAPI, examples, operating documentation, measured capacity estimates, and redacted demonstration results are checked in. Evaluation publication stays disabled. No historical import, reader switch, automatic producer wiring, new release policy, workspace-wide tests, commits, or lifecycle-directory moves belong to this work.

All phases execute unattended in order. Waves are numbered globally; tasks in the same wave may use concurrent subagents with disjoint file ownership. Each brief must repeat that the entire session tree is non-interactive. A failed correctness checkpoint produces a nonzero exit and a precise diagnostic, never a request for approval. Infrastructure credentials are runtime prerequisites, not decisions to delegate to a human during execution.

## Phase 1 — Contracts and feasibility

### Necessary Rules

1. **Ownership and tools.** Create `tools/evidence-store/` as private pnpm package `@rusty-biscuit/evidence-store`, add that directory to `pnpm-workspace.yaml`, and use TypeScript, Wrangler, Vitest's Workers pool, and `jose`. Put Worker code in `src/`, operator/ingestion code in `cli/`, SQL in `migrations/`, OpenAPI and examples in `api/`, and tests in `tests/`. Use the existing workspace TypeScript/Vitest conventions from `tools/test-audit/package.json`; pin added dependencies and update `pnpm-lock.yaml`. This is a service/tool, not a new Cargo area. Add no Python and no Rust in this stage; Web Crypto supplies SHA-256 and token digests. Evidence: spec's TypeScript Worker boundary and `d1-model.md` language boundary.

2. **Executable compatibility wins.** Port `scripts/ci/schema.py::{loads_strict,canonical,identity,validate_receipt,receipt_cell_problems,validate_observation,validate_collection,_cell_consistency,_cell_backends}` and their reachable helpers to `src/contracts/`. Preserve accepted legacy shapes, unknown-field rejection, exact integer version checks, optional-field presence, and broad nonempty proof-digest strings. Do not tighten proof digests to hex merely because report SHA-256 fields require hex. Match Python `json.dumps(sort_keys=True,separators=(",", ":"))`, including ASCII escaping, surrogate handling, Unicode key ordering, and numeric serialization. Use a lossless JSON tokenizer so integer tokens and float tokens remain distinguishable. Reject duplicate keys, nonfinite values, invalid UTF-8, and trailing data. Keep number lexemes until Python-compatible canonicalization; include large integers and exponent forms in parity fixtures. Evidence: `schema.py:1209`, `is_version`, `loads_strict`, and `observation_identity`; ordinary `JSON.parse`/`JSON.stringify` cannot establish this parity.

3. **Receipt adaptation is local.** `cli/ingest.ts` accepts v3 JSON or a v2 receipt and converts v2 using the exact default-hook behavior of `observation_notes.receipt_observations` and `collection_of`. Resolve report bytes on the submitting host, hash them, preserve original report-path strings, sort observations, and submit the resulting v3 collection with `source_version:2`. The Worker never opens a claimed host path. API ingestion accepts the adapted collection with source version 2 or 3, not a raw receipt requiring remote filesystem access. Empty v3 collections succeed with empty IDs and no revision change; v2 requires nonempty cells. Evidence: `receipt_observations` reads `host.report_dir` and only adds reports for complete cells. No v1 adapter.

4. **Identity and export.** Database uniqueness is `(repository_id, observation_id)` and equality includes canonical observation bytes plus environment/head/tree, not arrival time. Preserve the first accepted publisher, source version, and timestamp on replay, including replay through another adapter. Store immutable collection manifests and membership so `GET collections/{collection_id}` exports precisely the submitted adapted collection, rather than the union of a commit's observations. Collection identity is SHA-256 of canonical v3 collection bytes; source-version metadata is provenance, not observation identity. Candidate history remains the union. Evidence: spec's exact export requirement, context conflicts, and model query 13's context-wide union.

5. **Atomic conflict guard.** Use one D1 `batch()` per collection. An insert-conflict trigger compares existing canonical payload and context and calls `RAISE(ABORT, 'identity_conflict')` on any difference; identical rows are ignored without revision triggers. Insert collection membership and manifest inside the same batch. Do not use a preflight SELECT as the conflict authority or `INSERT OR REPLACE`. No writes may survive a rejected graph or conflict. Evidence: spec explicitly corrects the model's post-batch conflict example. A deliberate conflicting-context request can reuse a valid content ID and proves the guard without manufacturing a SHA collision.

6. **API and finite bounds.** Use `/v1/repos/{repository_id}` with `POST /collections`, `GET /collections/{id}`, `GET /observations/{id}`, `POST /history/query`, `GET /snapshot`, `POST /locations`, `POST /changes/query`, and upload/fetch operations below. Unknown repositories and missing IDs return 404. JSON request maximum is 1 MiB; one observation canonical payload is at most 128 KiB; collection maximum is 256 observations; reachable-heads maximum is 256; pages default to 100 and cap at 256 records and 1 MiB encoded bytes. Reject a single unpageable record explicitly. Export uses bounded member pages with canonical reconstruction by the CLI. Apply bounds before mutation; never split a collection/retry graph. Bind arrays as JSON to `json_each(?)` to avoid D1's 100-parameter cap. Unsupported API versions return `unsupported_api_version`. These bounds exceed the notes transport ceiling without approaching D1's 2 MB row limit. Evidence: spec's bounds requirement and Cloudflare limits linked below.

7. **Primary snapshots and cursors.** One primary SELECT captures repository generation, evidence revision, observation-event watermark, and location-event watermark. HMAC-sign opaque cursors containing generation, repository, query digest, both bounds, and last sequence; expire cursors after 24 hours. History accepts caller-supplied reachable heads without claiming to verify ancestry. Return `has_more` and `next_cursor` on every list. All subsequent reads use captured bounds; no replicas. Retain separate observation/location change-event envelopes through pruning, without retaining deleted report contents or full deleted observations. Record withdrawal and pruning tombstones so consumers cannot silently miss deletion. Expired change cursors return `cursor_expired`, requiring a fresh snapshot. Evidence: model queries 3/14 and spec's finite pagination, restore-generation, and pruning requirements.

8. **Credentials and roles.** Choose per-host 256-bit random bearer tokens with a public credential ID, server-side SHA-256 secret digest, 90-day expiry, rotation at day 60, and at most seven days of overlap; revocation is checked on each request against primary D1. Scope credentials to explicit repository IDs and reader/producer/verifier roles. Ken Snyder is the initial operator owner. Deliver secrets once to a mode-restricted output file, never stdout, argv, logs, or repository files; unattended consumers use a protected token file. Operator credentials are separate and never issued to producers. Operator bootstrap uses scoped Cloudflare API access through Wrangler, then subsequent management uses the operator API. Audit credential IDs and actions, never token values. Evidence: spec recommends this least-complex unattended mechanism and requires immediate revocation and role separation.

9. **Hosted authentication.** Verify GitHub JWT signatures with `jose`, fixed issuer `https://token.actions.githubusercontent.com`, audience `rusty-biscuit-evidence-store`, expiration/not-before with 60-second clock tolerance, numeric repository ID, exact repository-owner ID, and registered workflow/ref/event tuples. Do not derive permissions from producer JSON. First allow only `.github/workflows/evidence-store-demo.yml` on `refs/heads/main` for `workflow_dispatch`, producer plus reader, with no verifier/operator privileges. Pin the workflow path from `workflow_ref` and reject fork/pull-request contexts and mismatched `repository_id`. OIDC tokens authenticate requests directly; no long-lived token exchange. Repository registration obtains stable IDs from the authenticated GitHub repository API, not a guessed slug. Evidence: spec's exact allowlist and fork-denial requirements; automatic CI wiring is outside delivery.

10. **Proof object protocol.** Reference reports by their original path, and expected/completion proof by `proof.expected_listing_digest`/`completion_record_digest` and `schema.py::PROOF_EXPECTED_FILE/PROOF_COMPLETION_FILE`. Do not require historical proofs to grow a size field: bind byte length at upload initiation and verify the original digest. Producer may stage referenced bytes but cannot attest locations. Use `POST /uploads` to allocate a repository/observation/reference-bound upload ID, `PUT /uploads/{id}/parts/{index}` for sequential 1 MiB chunks, and verifier-only `POST /uploads/{id}/finalize`; object maximum 64 MiB, upload expiry 24 hours. Each chunk is byte-checked on replay. Stream whole-object SHA-256 with a pinned incremental TypeScript/WASM implementation compatible with Workers; use Web Crypto for small values. Complete R2 multipart upload into a private staging key, stream-verify full digest/length, then insert the verified location. Key identity includes repository, observation ID, reference kind/digest, and upload ID; retries reuse it. Interrupted/invalid uploads leave the observation untouched; cleanup removes unfinalized staging after 48 hours. Finalization is idempotent across R2/D1 failure. Evidence: immutable references and two-resource recovery boundary; proof lacks sizes today.

11. **Location semantics.** Keep typed `host-dir`, `artifact`, and `object` locators; never fetch arbitrary URLs or interpret historic paths as Worker paths. Original observation `reports[].store` remains `host-dir|artifact`; R2 is a separate object location, not a schema change. Key events by repository plus caller-supplied event UUID; equality excludes server timestamps and records actual authenticated verifier. Identical events return original results; conflicting replays return 409. Verifier byte attestations last 24 hours; finalization alone asserts integrity, never test coverage. Newest event per location wins before expiry filtering. Readers may fetch finalized, currently available referenced objects only; withdrawal denies new retrieval. Evidence: `OBSERVATION_REPORT_STORES`, model query 7, and spec's verifier-only attestation rule.

12. **Errors and retry.** Respond with `{error:{code,message,location,retryable,retry_after_ms},request_id}` using 400 invalid input/cursor, 401 authentication, 403 denied, 404 missing, 409 identity conflict/stale snapshot, 413 size limit, 429 quota/rate limit, and 503 unavailable. Map platform failures to unavailable, never empty success. Limit each credential to 60 mutation and 300 read requests/minute using Cloudflare rate-limiting bindings, separate operator limits of 30 mutations/minute. CLI retries only 429/503/network failures, at most five attempts with full jitter capped at 30 seconds, honors a larger Retry-After up to 60 seconds, and times out each request after 60 seconds. Retain collection, bytes, and resume state until all requested durable acknowledgments; print a concrete resume command without secrets on persistent failure. Evidence: spec's outage and lost-response contracts.

13. **Retention is verifier-driven, fail-closed.** Add retention certificates and jobs separate from evaluations. Certificates name an exact current snapshot, complete cell candidate IDs, retained IDs, supersession edges with strict-ancestry/retry evidence, verifier policy identity, and release pins. Only separately provisioned retention-verifier credentials may submit them. Reuse qualification remains the trusted verifier's responsibility; the service checks exact candidate-set equality, same-cell edges, valid retry chains, nonempty protected maximal sets, generation/revision, release pins, and the 14-day grace measured from first certified supersession. Never infer supersession from server arrival or producer assertions. No production retention verifier is provisioned in this stage: real evidence remains protected until the later verifier supplies certificates; synthetic isolated tests exercise the sanctioned path now. Implement withdraw → delete object → delete rows under persisted job state, with database deletion guards permitting only authorized job members. Release pins and unresolved/maximal observations cannot be job members. A revision change invalidates an unstarted certificate; after withdrawals, block concurrent writes to the affected cell during bounded deletion batches and release the lock on completion/recovery. Keep shared objects until no retained reference remains. Evidence: spec's retention ruling overrides append-only delete triggers, while trusted reuse verifier and release decisions belong to the later consumer. No producer/operator bypass may delete evidence.

14. **Recovery boundary.** Back up metadata and private bytes to operator-owned storage outside the live bucket every day, with manifest digests and generation; keep seven daily backups. Backups include credential digests, not recoverable bearer secrets. Quiesce mutations for the final backup boundary; uploads pending at that boundary remain pending, not acknowledged. Export D1 and list/copy every referenced R2 object plus pending-upload/job metadata. Restore only into isolated resources, rotate generation before opening access, verify objects, and replay retained post-backup ingestion bundles where available. Emit explicit acknowledged-ID loss lists; never claim zero loss from metadata alone. Rotate operator/cursor keys on restore, preserve repository IDs and immutable IDs, invalidate evaluations. Evidence: spec's separate metadata/object backup and pre-restore cursor rejection.

15. **Budget and spikes.** Confirmed on 2026-11-08: Workers Free has 100,000 requests/day and 10 ms CPU/request; D1 Free allows 5 million rows read/day, 100,000 rows written/day, 5 GB/account but only **500 MB/database**; R2 Standard includes 10 GB-month, 1 million Class A and 10 million Class B operations/month. Include backups, indexes, event history, staging, and shared account use in estimates. R2 allowances are not a hard spending cap: reserve projected bytes/operations in D1 before admitting work, disable paid upgrades, and use conservative caps of 8 GB live-plus-staging, 800,000 A and 8 million B/month, subtracting other account usage. Alert at 70% and refuse new capacity-consuming work at 80% of each allowance; preserve existing evidence. A projected breach within 12 months or unmeasurable baseline is a nonzero feasibility failure, not permission to change retention. The only runtime spike is the deployed D1 guard/binding check plus one-host quick CPU sample in Wave 2; no OS matrix or performance campaign. Evidence: spec's free-tier ruling and official sources below. Local notes refs were absent during planning, so an empty local scan is not a volume estimate.

16. **Concrete implementation seams.** Use native Worker `fetch` routing without a routing framework; `@noble/hashes` SHA-256 supplies incremental upload verification, while Web Crypto handles JWT-compatible primitives and cursor HMAC. Use `src/{worker,auth,ingestion,reads,uploads,locations,retention}.ts`, `src/db/`, and `src/contracts/`; CLI exports live in `cli/{ingest,operator,inventory}.ts`. Bindings are `EVIDENCE_DB`, `EVIDENCE_BUCKET`, and `CURSOR_HMAC_KEY`; deployment IDs come from explicit Wrangler environments `development` and `restore-test`. Configuration and resume paths are literal native Node paths, never historic report-reference strings. Evidence: existing report paths are provenance rather than portable reader paths, and the spec requires only one service/object store.

17. **Pruning and snapshot validity.** Include a repository `retention_epoch` in snapshots and cursors in addition to restore generation. Advance it atomically before deleting rows; reads against an older epoch return 409 `snapshot_pruned` rather than silently omitting formerly bounded rows. Store minimal tombstones in the two event streams for 30 days, then reject older feed cursors with `cursor_expired`. Tombstones contain IDs and sequence/context only, not deleted payloads or secrets. Daily backup rotation also removes pruned bytes from backups after their seven-day backup window. Evidence: snapshot watermarks alone cannot preserve deleted records, and the spec requires both bounded reads and sanctioned pruning.

### Wave 1 — Contract foundation

- [x] **Define API**
  - Write OpenAPI 3.1, strict JSON configuration schemas, example requests/responses, role matrix, limits, and error codes under `tools/evidence-store/api/`. Include operator registration/credential routes, upload/reference unions, retention certificates, and disabled evaluation publication (403 `evaluation_publication_disabled` for every role).
  - Create package/lockfile/justfile foundation and `src/contracts/` exported interfaces. This task owns shared package setup; remaining waves consume its interfaces.
- [x] **Trace parity**
  - Capture frozen real-shape fixtures and canonical expected bytes/IDs from existing `schema.py`, `observation_notes.receipt_observations`, `hosted_observations.build_observation`, and existing schema/observation test fixtures. Include hook, rig, seed, hosted pass/fail, incomplete, retries, Unicode, and v2 adapter controls.
  - Add a contract inventory linking every field and shared validator to robustness cases below. Read the current working tree; do not overwrite unrelated existing changes.

### Wave 2 — Feasibility checkpoint

- [ ] **Measure capacity**
  - Add a read-only TypeScript inventory command. Query remote note-ref/blob metadata and recent GitHub artifact metadata with existing unattended authentication; read available local retained report sizes. Sample the latest 30 completed relevant runs and 30 days of observations, reporting missing/expired bytes separately. Use fetched bytes only in scratch storage; do not publish or import historical records.
  - Record observed cell count, metadata/report/proof size percentiles, daily attempts, unresolved-history growth, and 12-month forecasts including the no-production-pruning interval. Use p95 sizes and twice observed daily volume for capacity admission; include account-wide consumption. Missing required data exits nonzero with the exact failing source. Write redacted results to this feature's `implementation-log.md`.
- [ ] **Prove D1**
  - Create a disposable development database with non-interactive scoped `CLOUDFLARE_API_TOKEN` and account ID; run generated columns, triggers, prepared bindings, query catalog, and transaction rollback scenarios. Run 20 paired concurrent ingests with one conflicting context and fresh nonconflicting siblings; assert rejected batches leave no sibling, manifest, event, or revision increment.
  - Sample canonicalization/validation at the declared maximum once on the development Worker. Success requires D1 correctness and requests completing within Free CPU limits. CPU failure reduces advertised limits to the largest passing powers-of-two payload/count bound from the same sample and updates OpenAPI/tests; reject collections above those limits whole. D1 correctness failure exits nonzero, records exact rollback failure, and leaves deployment disabled; do not change stores or weaken atomicity. This replaces the spec's human-return instruction with a deterministic failed checkpoint, not a human gate.
  - Missing infrastructure credentials terminate this checkpoint with required environment-variable names, without interactive login. No feasibility success is recorded from local SQLite alone.

Checkpoint: both feasibility tasks pass before service deployment work. Wave 2 tasks own separate scripts/results and may run concurrently.

## Phase 2 — Validation and immutable storage

### Wave 3 — Independent implementations

- [ ] **Port contracts**
  - Implement strict loader, Python-compatible canonicalizer, v3 validators, and v2 client adapter from Phase 1 inventory. Preserve code-authoritative legacy acceptance and IDs; do not normalize historical paths. Expose `parseCollection`, `parseReceipt`, `adaptReceipt`, and canonical identity functions for in-process tests.
  - Public results carry JSON-pointer locations and stable codes. Complete parity/robustness checks, including no publication on malformed adapter input.
- [ ] **Build migrations**
  - Translate the model into numbered migrations, adding collection manifests/membership, credential grants/audit, upload state, event identities/streams, generation, quota reservations, retention certificates/jobs/locks, and release pins. Add indexes for cell/head/retry/history/expiry queries and guarded deletion; evaluations exist but have no public writer.
  - Implement the batch repository abstraction and conflict trigger, preserving first-publisher metadata and revision behavior. Empty/idempotent deliveries emit no evidence change events. Freeze migrations after a deployment has applied them; subsequent changes use new numbers.

### Wave 4 — Storage integration

- [ ] **Integrate ingestion**
  - Connect authenticated contract-ready handlers to one atomic batch; validate the complete supplied graph before writes and enforce context conflicts inside the batch. Return accepted/already-present IDs plus collection ID and revision. Handle lost responses safely.
  - Validate collection export membership against pruning: an export missing pruned members returns 410 `collection_pruned`, never a falsely exact partial collection.
  - Run focused local Workers-pool tests for duplicate races, mixed conflicts, rollback, empty collections, unknown repositories, and preserved publishers. Repeat only the final implementation's D1 correctness cases in Phase 6; do not repeat the feasibility spike.

## Phase 3 — Authorization and bounded reading

### Wave 5 — Separate service surfaces

- [ ] **Enforce identities**
  - Implement host token lifecycle, operator bootstrap/registration commands, exact GitHub OIDC allowlist, role middleware, request IDs, redacted errors, and audit trail. Derive grants exclusively from authenticated identity.
  - Test expiry, revocation, rotation overlap, cross-repository denial, forged provenance, every role/operation pair, wrong issuer/audience/ref/workflow/owner, and fork identities using signed test JWTs and a local JWKS fixture. Do not contact GitHub in unit tests.
- [ ] **Implement reads**
  - Implement observation lookup, exact collection export pages, candidate history including failures/incomplete attempts, primary snapshots, HMAC cursors, latest-location queries, and independent durable changes streams.
  - Exercise page-size and encoded-byte limits, query/repository cursor swaps, ongoing concurrent writes, withdrawal replay, empty successful history versus lookup failure, generation changes, and pruned-member tombstones. Finite pages must terminate at their captured bounds.

Checkpoint: all routes enforce repository and role grants; no producer can create a location attestation or evaluation. Cursor tests prove no new write enters an old snapshot.

## Phase 4 — Proof transfer and explicit ingestion

### Wave 6 — Transfer and client

- [ ] **Implement transfers**
  - Implement bounded resumable upload, immutable private R2 staging, streaming integrity verification, verifier-only idempotent finalization, and authorized fetch. Validate expected-listing and completion JSON with existing executable contracts, cell identity, and proof digests; no passing location certifies coverage.
  - Port proof structure checks from `schema.validate_completion_record`, `completion.manifest_problems`, `hosted_observations._expected_manifest/_completion_record`, and `local_evidence._proof_problem`. Bind expected-manifest package/environment/gate/head/target/toolchain to the observation and collection; do not port plan selection, skip approvals, or coverage acceptance into the store. Reject a proof that requires plan-dependent validation as a coverage claim; integrity storage remains permitted. Oversized existing proofs return explicit size errors while their observations remain recorded.
  - Test interrupted parts, wrong size/digest, missing proof, completion mismatch, unreferenced upload, finalize-after-R2-success/D1-failure, withdrawal during retrieval, independent locations, and staging cleanup.
- [ ] **Build ingestion**
  - Implement `just evidence-ingest INPUT --repo ID --token-file PATH --proof-dir PATH --state PATH` and `just evidence-resume STATE` in the service justfile. The CLI reads configuration before any network mutation, converts v2 locally, preserves original bundles, submits once, uploads explicitly mapped proof files, and stores durable resume acknowledgments atomically.
  - Producer-only ingestion reports staged bytes pending verifier finalization; the demo operator runs a separate verifier credential to finalize. Keep claimed historical provenance distinct from actual publisher. No call site is added to `publish-receipt`, `publish_directory`, hooks, `ci.yml`, `seed.py`, or `affected_scope.py`.
  - Test public CLI library results in process; spawn CLI only for control plus malformed-file/wiring cases. Commands must work with Node on macOS, Linux, Windows, and WSL2 without shell-specific quoting or interactive stdin.

Checkpoint: no report content or token appears in errors/logs; upload retry never reruns a test or modifies an observation.

## Phase 5 — Retention and recovery

### Wave 7 — Independent operational paths

- [ ] **Guard retention**
  - Implement certificate validation, protected-set checks, release-pin protection, 14-day grace, withdrawal events, cell locks, and resumable bounded prune jobs. Do not provision a production retention-verifier grant. Keep deletion permission inaccessible through generic operator SQL routes; no public SQL endpoint exists.
  - Test current pass, unresolved failure, same-head conflict, unrelated heads, retry resolution, stale/partial snapshot, missing store data, release-cited rows, shared objects, replay, and crashes at every withdraw/delete boundary. These tests use an isolated verifier fixture, not a producer-supplied passing claim.
  - Add daily staging cleanup and budget admission accounting. Admission denial never evicts protected evidence or suppresses failures. Append deletion tombstones before removal.
- [ ] **Implement recovery**
  - Add non-interactive `backup`, `restore`, `verify-backup`, and `recover-jobs` operator commands with explicit source/destination IDs and manifests. Restore forbids the live source as destination. Backups cover schema/version, D1, objects, pending uploads, event counters, and credentials/grants; secrets remain external.
  - Persist mutation maintenance mode and require all acknowledged writes to fall on a documented backup boundary. Recovery emits recovered/unrecoverable acknowledged IDs and missing-object lists. Deployment rollback restores a compatible prior Worker build; destructive schema rollback uses isolated restore and generation rotation.

Checkpoint: synthetic pruning and fault-injected recovery prove rows are never left advertising deleted bytes as available. Protected evidence survives quota exhaustion and cleanup.

## Phase 6 — Deployment and acceptance

### Wave 8 — Final integration

- [ ] **Publish operations**
  - Add `docs/cicd/evidence-store.md` with task-oriented examples, role/credential rotation, deployment/migrations, bounded ingestion/resume, proof staging/finalization, outage diagnosis, quota alerts, retention, daily backups, restore, and rollback. Include upload/recovery Mermaid flows. Explain that notes remain authoritative and retention qualification/evaluation consumers are not enabled.
  - Update `docs/cicd/evidence.md` with a short service/tool link and its non-authoritative status, `docs/dependencies.md` for added tooling, and `.claude/skills/rust-devops/ci-cd.md` for the explicit ingestion workflow. Do not link current docs to dated feature directories. Review changed symbols' comments for drift and record spec departures in `implementation-log.md` without rewriting the spec.
  - Expose service `just typecheck`, `test`, `lint`, `test-real`, `deploy`, and operator recipes. Use pinned pnpm scripts; do not invoke `cargo fmt`. Wire local service checks through explicit root recipes only; do not add scheduled CI cells or alter the canonical scope registry for this delivery.
- [ ] **Demonstrate service**
  - Deploy development Worker/D1/private R2 from checked-in configuration and numbered migrations using existing unattended Cloudflare credentials. Produce redacted endpoint/resource/version manifests. Register the real repository and issue separate temporary producer/verifier/reader demonstration grants.
  - Ingest one authentic retained host collection and one authentic hosted collection with original IDs; stage and finalize their report, listing, and completion bytes. Use a manual-only `.github/workflows/evidence-store-demo.yml` on main for the OIDC-authenticated explicit hosted ingestion. It reads an explicitly named retained artifact/run and runs no tests, historical import, or permanent publication hook. Dispatch only after that exact committed workflow is available on main; the execution command reports an unavailable workflow as a failed external prerequisite, never pushes/merges code itself.
  - Run focused deployed correctness cases: concurrent mixed-identity rollback, revoked/expired/unauthorized identities, interrupted upload/finalization replay, location withdrawal without revival, change-feed replay, and finite snapshot pagination during writes. Capture assertions, not raw credentials/reports.
  - Back up that instance, restore into isolated D1/R2 resources, retrieve and digest-check all demonstration proofs, resume a deliberately interrupted job, and reject every pre-restore cursor. Record the recovery boundary and acknowledged-loss result. Remove disposable test grants/resources after capturing redacted evidence; retain the working development service and genuine demonstration records.
  - Run `pnpm --filter @rusty-biscuit/evidence-store typecheck`, `test`, and `lint`; run `test-real` once for the deployed acceptance suite. No Cargo suite is required because no Rust changes are planned. A missing external prerequisite yields a nonzero command and incomplete acceptance, never a fabricated deployment result or a human approval phase.

Checkpoint: record every definition-of-done artifact and result in `implementation-log.md`; mark implementation complete, ready for review only after all acceptance checks pass. Leave the active feature directory in place.

### Input Robustness Matrix

All invalid input produces a located public error, runs no test/process, writes no evidence/object/location, and publishes nothing. Upload integrity failures are the documented exception: the immutable observation and nonusable staging state may already exist. Test field shapes once per shared validator; each additional field has a positive control and one invalid wiring case. Build mutations from frozen authentic fixtures, one edit per case.

| Shared validator | Fields and required shape coverage | Explicit valid exceptions |
|---|---|---|
| Strict evidence object/version | Collection, receipt, observation, cell, producer, host, report, build, proof; absent/null/wrong type/empty object and unknown key; integer versus float/bool version | Optional objects are omitted, never null; preserve runtime acceptance from `schema.py` |
| String/enum/identity | Environment/head/tree/base, package/gate/outcome/completion, IDs/retry, scope/input identities, producer fields, report path/store/digest, proof strings | Empty `features` remains a string; optional strings follow existing per-field validators; do not invent stricter digest checks |
| Lists and members | Observations, cells, reports, backends, failed names; absent/null/scalar/mixed invalid/all invalid/empty | `observations:[]` is valid zero proof; incomplete observations may have no reports; v2 `cells:[]` errors; other empty rules match executable validators |
| Numeric/boolean/counts | Counts, duration, exit code, bytes, producer attempt/run fields and truncation flag; absent/null/wrong type/empty where representable, negative/fractional boundaries | Preserve legacy validator behavior; count consistency and failure-detail rules remain enforced |
| Graph/context | Sorted unique IDs, identity bytes, retry targets/cell/cycles, collection environment/head/tree, adapter metadata | Missing retry means no retry; no dangling external retry even when target is already in D1 |
| Service configuration | Repository/workflow allowlist, issuer/audience, credential grants/expiry, quota limits, endpoint/token-file/state/proof map | Missing page size defaults to 100; missing cursor starts at snapshot origin; empty heads returns empty candidates; empty credential grants/allowlists deny all, not unrestricted access |
| Request unions | Upload kind/size/parts, typed locators, event identity, snapshot/cursor, retention certificate/job/release pins | Optional fields omitted; explicit null errors; size zero is valid only for a referenced empty file; empty prune set is an idempotent no-op |

Document loaders are strict evidence/config/request JSON, strict expected/completion JSON using the same loader, and strict resume/backup manifest JSON using the same loader: test duplicate keys, trailing garbage, nonfinite numbers, invalid UTF-8, BOM rejection, blank input, and concatenated documents once through the shared loader, plus one public wiring test per consumer. JSON5/YAML/JSONL are unsupported and must not be silently accepted. OpenAPI YAML and Wrangler JSONC are tool-owned build inputs, not additional runtime configuration loaders.

Test public `parseCollection`/`adaptReceipt`, Worker `fetch` responses, and exported CLI operations in process. Assert location plus zero writes with database/object inspection. Spawn one control and two invalid CLI cases for shared loader wiring only. Before completion, search new code for `default`, nullable optional handling, `filter_map`, `.filter`, `unwrap_or_default`, `.ok()`, and parse catches that return empty values; explain intentional defaults in the inventory. A review finding records the state of the validator's other shapes and the loader's other document shapes, then adds the missing public-result coverage.

### External evidence

Planning verified Cloudflare's current [D1 limits](https://developers.cloudflare.com/d1/platform/limits/), [D1 free allowances](https://developers.cloudflare.com/d1/platform/pricing/), [R2 Standard free allowances](https://developers.cloudflare.com/r2/pricing/), and [Workers limits](https://developers.cloudflare.com/workers/platform/limits/). D1 `batch()` counts as one invocation query, while each statement retains its row/parameter limits. Use the official [transactional batch contract](https://developers.cloudflare.com/d1/worker-api/d1-database/#batch) for the deployed rollback assertion. These checks settle the platform limits; the first-phase measurement establishes this repository's actual consumption.