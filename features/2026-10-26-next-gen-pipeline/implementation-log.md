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
    Owner ruling (2026-11-11): "store only the latest passing run per cell
    until retention exists". Implemented as an admission model, not as
    enforcement: the inventory projects R2 under the spec's retention (each
    cell's latest pass and unresolved failures, plus 14 days of superseded
    bytes) and reports the never-pruned projection as information only. The
    service itself does not limit a cell to its latest pass; nothing sends
    evidence automatically today, and at 80% it refuses uploads rather than
    evicting. The constraint that keeps the never-pruned case hypothetical,
    that producers must not send evidence automatically until a retention
    verifier certifies prunes, is recorded as ruling
    `evidence-store-retention-before-producers` on
    2026-11-06-rust-evidence-client (also in its cutover step 4, which now
    limits the import to latest passes and unresolved failures) and in
    docs/cicd/evidence-store.md. D1 stays projected without pruning (28%).
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
  - |-
    Upload parts do not use R2 multipart upload, although plan rule 10 says
    "complete R2 multipart upload into a private staging key". R2 requires
    every multipart part except the last to be at least 5 MiB, and the plan
    fixes parts at 1 MiB. Each 1 MiB part is instead stored as its own
    private staging object named by the part's SHA-256, and finalization
    streams the parts, in order, into the final object while hashing them.
    Naming parts by their hash makes two racing writes of one part safe: the
    first database row wins and the other is refused as a conflict.
    Alternatives: 5 MiB parts with real multipart upload (changes the
    published part size and the CLI), or buffering parts in the Worker (memory
    limits). Switching to multipart later touches src/uploads.ts and the
    part-size constant only.
  - |-
    Whole-object SHA-256 at finalization uses the Workers-native incremental
    `crypto.DigestStream` rather than @noble/hashes, which plan rule 16 names.
    Both hash incrementally; the native one costs far less CPU, which matters
    on Workers Free's 10 ms CPU limit. Even native hashing of a 64 MiB object
    will exceed 10 ms, so the deployed Phase 6 check must measure
    finalization CPU; large proofs may need Workers Paid or a lower object
    cap. Reverting to @noble/hashes changes one function in src/uploads.ts.
  - |-
    Fetching an object ignores the 24-hour expiry of its `verified` location
    event; only a newer `withdrawn` or `unavailable` event for that object
    denies retrieval. Plan rule 11 says verifier attestations last 24 hours,
    and the OpenAPI text said an expired object answers 404. Bytes in the
    service's own bucket were digest-checked at finalization and cannot
    change, so making them unreadable a day later would force a verifier to
    re-attest every object daily for nothing. The expiry still applies to
    every reader's view of host-dir and artifact locations. Alternative:
    honor the expiry (one condition in the fetch query) and add a periodic
    re-verification job.
  - |-
    Finalization records its `verified` object location event under an event
    ID derived from the upload ID, so a retry after "R2 write succeeded but
    the D1 write failed" inserts the event exactly once, and a replay after a
    withdrawal returns the original result without reviving the object.
    Rejected proofs (wrong digest, wrong length, a completion record or
    expected listing that fails the ported structural checks or names another
    cell) mark the upload `rejected` and answer 400; only structural and
    cell-binding checks are ported, never plan selection or coverage
    acceptance.
  - |-
    The ingestion command takes flags, as the plan's command line shows, plus
    an `--endpoint` flag the plan omitted (the service URL has to come from
    somewhere). `--proof-dir` is the explicit map: each report resolves to
    `proof-dir/<its recorded path>` and each proof file to its standard
    `{package}-{gate}.expected.json` / `.completion.json` name, which is the
    retained-directory layout producers already write. Every mapped file must
    exist and match its digest before anything is sent; a report path that is
    absolute or uses `..` is refused. Plain `http://` is accepted only for
    loopback hosts (local `wrangler dev` and tests). Producer ingestion stops
    at "staged"; a separate `just evidence-finalize STATE --token-file PATH`
    runs with the verifier credential. Alternative: a JSON map file (the
    Phase 1 ingest-config `proofs` list) for collections whose files live in
    several directories.
  - |-
    The daily staging cleanup is implemented and tested as a function
    (`cleanupStaging`), but no cron trigger calls it yet: Phase 5's "daily
    staging cleanup and budget admission accounting" task schedules it with
    the other daily work. Uploads do not reserve capacity in
    `quota_reservations` yet for the same reason.
  - |-
    Two createUpload requests for the same file that arrive at the same
    moment can each open their own upload. Either one can still be finalized
    correctly, and each finalized upload is a separate object location.
    Preventing this needs a partial unique index on uploads plus
    conflict handling in the handler. That was left out as unneeded
    complexity, since the CLI sends one request per file. Adding it later
    touches migration 0004 (or a new migration once deployed) and
    src/db/uploads.ts.
  - |-
    The proof checks never quote content from the uploaded document in an
    error, though the Python messages echo `repr` values. Errors give the
    Python problem code, the field name, and the observation's own stored
    value. Completion records bind run and attempt only for hosted
    producers: rig, seed, and hook observations record no attempt, and their
    run ID is a label local to the host. The expected listing's `target` is
    checked against `proof.target`, the only toolchain identity an
    observation records. Its `nextest_version` must be present but is not
    compared with the observation's value, because hosted producers copy that
    value from the completion record.
  - |-
    The ingestion just recipes run `node <pkg>/node_modules/tsx/dist/cli.mjs
    <pkg>/cli/ingest.ts <command> "$@"` from the caller's directory
    (`[no-cd]`). They do not go through `pnpm --dir … exec`, because pnpm
    changes into the package directory: relative INPUT, state, and proof
    paths would then resolve in the wrong place, and paths containing spaces
    would be split. `ingest` refuses to start if the state file already
    exists and points to `evidence-resume` instead. Retry backoff uses a
    1-second base, which CLIENT_RETRY did not specify. The resume-state
    schema now records `endpoint`, `repository_id`, and `token_file` (a path
    only) where it had `config_file`, because the CLI takes flags, not a
    config file.
  - |-
    Phase 5 adds a persisted maintenance mode (migration 0007). While the
    store is quiesced for a backup, database triggers refuse every evidence,
    location, upload, retention, repository, and credential-issue write, and
    the Worker answers 503 `unavailable` (retry after 60 s) before a mutation
    starts. The triggers, not the Worker check, decide: a write either
    committed before the quiesce and is in the backup, or it was refused and
    never acknowledged. Two exceptions: the audit log (so refusals are still
    recorded) and credential revocation (stopping a leaked credential matters
    more than the boundary). The backup records revocations made during its
    window and restore re-applies them; revocations made on the old service
    after the backup are not carried over, and the docs say to repeat them.
    Alternative: block revocation too (simpler restore, but a leak could not
    be stopped during a backup).
  - |-
    The plan says a revision change invalidates a retention job that has not
    started. The repository revision moves on every ingest, location event,
    and upload finalization, so a job that waited for the daily run would
    almost always be invalidated, and pruning would almost never happen. The
    grace period is counted from an observation's first certification and
    persists across certificates, so a verifier that resubmits after 14 days
    gets a job whose grace is already over. Such a job now starts
    (`withdrawing`) in the same database write that accepts the certificate,
    while the certificate's snapshot is proven current; only a job still in
    its grace period waits as `scheduled` and can be invalidated.
    Alternatives: compare only the certified cells' contents instead of the
    whole-repository revision (a weaker check than the plan's), or keep
    waiting for the daily run (pruning effectively disabled). Reverting is
    one CASE expression in `writeCertificate` (src/db/retention.ts) plus one
    test.
  - |-
    Retention details the plan left open (all in src/retention.ts and
    migration 0008):
    - A write into a cell locked by a retention job answers 503
      `unavailable` (retryable, 60 s), not a new 409 code, so clients retry
      without changes. The lock also covers uploads and every location event
      except `withdrawn`, so a `verified` event cannot land between the
      withdrawals and the deletions.
    - Withdrawal event IDs come from (job, newest location row), so a
      location re-verified after its withdrawal is withdrawn again rather
      than skipped as a replay.
    - The first-certification time is kept in a new `retention_supersessions`
      table, deleted only together with a pruned observation.
    - Refusals: a candidate set that is partial, or names an observation
      the store does not hold, is 409 `stale_snapshot`; an edge, chain, or
      retained ID outside the cell's candidates is 400; a release pin on a
      member is 400.
    - Replay equality compares the SHA-256 of the canonical certificate,
      so key order and whitespace do not matter.
    - Tombstone expiry deletes an expired `pruned` tombstone together with
      every earlier event of the same observation or location event, and
      advances the replay horizon in the same transaction.
  - |-
    Budget admission reserves capacity only for uploads (R2 storage bytes and
    monthly write operations) when an upload opens, commits them on
    finalization, and releases the storage of rejected, expired, cleaned, and
    pruned uploads. Spent write operations are never released. Collection
    ingestion is never refused for capacity, so a failure can never be
    suppressed, and admission never deletes anything. D1 rows and Worker
    requests are not metered per request (each would cost a D1 write against
    the 100,000/day allowance); the daily run reports R2 usage and logs a
    numbers-only alert at 70%. Alternative: meter D1 writes per request.
  - |-
    Recovery choices (cli/recovery.ts, src/recovery.ts):
    - Backups go to a local operator-owned directory (`--destination`), one
      subdirectory per backup, with seven kept. Scheduling the daily backup
      is the operator's job (Phase 6 documents it); the Worker cannot copy
      its own database.
    - The pending-upload, job, credential, and grant lists are kept in the
      manifest itself rather than in separate files.
    - Restore reorders the D1 export (tables, rows, indexes, then triggers)
      so restored rows do not fire triggers and duplicate change events.
    - Restore refuses a destination that shares an environment name, D1
      database name or ID, or bucket with the source. Because the checked-in
      `wrangler.jsonc` still uses placeholder IDs for both environments, it
      refuses until real IDs are set.
    - After restore, every repository's generation is rotated, the restored
      operator credentials are revoked and a new one is minted into an
      owner-only file, and the cursor key is replaced through
      `wrangler secret put` fed on stdin.
    - Restore appends an `unavailable` event for every object the backup
      lacks, marks pending uploads expired, and stays quiesced until a
      separate reopen.
    - Replay of retained ingestion bundles classifies acknowledged IDs as
      recovered, replayed, or unrecoverable. Without replay inputs, the
      report says post-boundary loss is unknown.
    - Three assumptions about real Wrangler output are unproven until the
      Phase 6 demonstration: the `d1 export` format, the `d1 execute --json`
      result shape, and the missing-object error text.
  - |-
    Phase 6 is reported as FAILED. "Publish operations" is done, but
    "Demonstrate service" could not run. There are no Cloudflare credentials
    (CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID are unset, and Wrangler's
    saved login has expired and cannot be renewed without a person) and no valid
    GitHub token. So nothing was deployed, no repository was registered, nothing
    was ingested, the demo workflow was not dispatched (it also has to be merged
    to main first), and no backup or restore ran. The plan says to mark the work
    complete only after the acceptance checks pass, so `implemented` is left
    unset on the spec, plan, and log. Everything that needs no credentials is
    built and tested: the deploy command, the dispatch command, the demo
    workflow, and the acceptance suite. To finish, supply the credentials,
    follow docs/cicd/evidence-store.md's first-time setup, merge the workflow,
    and run the demonstration and `just test-real`.
  - |-
    The deployed acceptance suite runs the same seven cases in two places:
    locally inside `just test`, against the Worker with real tokens, and
    deployed through `just test-real`. That way the cases themselves are proven
    before any deployment. Credential expiry can only be tested locally, because
    the shortest credential lasts one day. The deployed run reports that case as
    skipped, never as passed. Each run leaves small synthetic records (and two
    small stored files) in the repository named by EVIDENCE_STORE_ACCEPTANCE_REPO,
    because the store has no delete route. The intent is for that to be a
    dedicated acceptance repository, not the real one. Alternatives:
    - share one credential across roles, which avoids hitting the 30-per-minute
      operator limit (now absorbed by retries) but blurs the role checks;
    - add a second registered repository to test "granted elsewhere, used here"
      literally (now the reverse direction is tested), about ten lines.
  - |-
    Turned on `workers_dev` for the development and restore-test environments
    in wrangler.jsonc. Before this, a deployed Worker had no address, so the
    endpoint, the demo workflow, and the acceptance run had nothing to call.
    Every route still requires authentication. Alternative: a custom domain
    route, which needs a Cloudflare zone; switching means setting `routes` and
    turning `workers_dev` off again.
  - |-
    The hosted demonstration uses GitHub's identity token only as a producer
    (stage uploads, read), as the plan allows. Staged files are published
    afterward from a host with a verifier credential, using the resume state the
    workflow uploads as an artifact. The dispatcher refuses to run until the
    workflow is on main, and never pushes or merges. Smaller choices: the root
    `just evidence-store` recipe runs from the caller's directory, so relative
    ingest paths still work; `deploy --manifest` must be an absolute path, like
    the other operator commands; the pnpm `test-real` script relies on the
    justfile to set EVIDENCE_STORE_REAL=1 instead of adding a cross-platform env
    helper dependency. Run bare, it fails, so it can never pass with zero tests.
  - |-
    Unchecked ordering risk for the first restore. `operator restore` refuses
    a destination database that already has migrations, so `just deploy --env
    restore-test` cannot run before it. But restore sets the cursor key with
    `wrangler secret put`, which may need the restore-test Worker to exist
    already. The guide says to restore before deploying, but this has never run
    against Cloudflare. The deployed restore demonstration has to confirm it. A
    likely fix, if it fails, is to deploy the Worker without applying migrations
    first.
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
source_files_during_phase_4:
  - tools/evidence-store/src/uploads.ts
  - tools/evidence-store/src/db/uploads.ts
  - tools/evidence-store/src/contracts/proofs.ts
  - tools/evidence-store/src/locations.ts
  - tools/evidence-store/src/worker.ts
  - tools/evidence-store/migrations/0004_uploads.sql
  - tools/evidence-store/cli/ingest.ts
  - tools/evidence-store/justfile
  - tools/evidence-store/api/openapi.yaml
  - tools/evidence-store/api/examples/finalizeUpload.response.json
  - tools/evidence-store/api/schemas/ingest-config.schema.json
  - tools/evidence-store/api/schemas/resume-state.schema.json
  - tools/evidence-store/tests/service/helpers.ts
  - tools/evidence-store/tests/service/uploads.test.ts
  - tools/evidence-store/tests/service/fetch.test.ts
  - tools/evidence-store/tests/node/proofs.test.ts
  - tools/evidence-store/tests/node/ingest.test.ts
  - tools/evidence-store/tests/node/ingest-cli.test.ts
  - tools/evidence-store/tests/fixtures/ingest/fake-service.ts
docs_updated_during_phase_4:
  - tools/evidence-store/README.md
  - tools/evidence-store/api/README.md
  - tools/evidence-store/docs/contract-inventory.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - tools/evidence-store/migrations/0003_credentials.sql
  - tools/evidence-store/migrations/0004_uploads.sql
  - tools/evidence-store/migrations/0007_maintenance.sql
  - tools/evidence-store/migrations/0008_retention_state.sql
  - tools/evidence-store/src/backup-manifest.ts
  - tools/evidence-store/src/capacity.ts
  - tools/evidence-store/src/contracts/api.ts
  - tools/evidence-store/src/daily.ts
  - tools/evidence-store/src/db/audit.ts
  - tools/evidence-store/src/db/retention.ts
  - tools/evidence-store/src/db/uploads.ts
  - tools/evidence-store/src/maintenance.ts
  - tools/evidence-store/src/operator.ts
  - tools/evidence-store/src/recovery.ts
  - tools/evidence-store/src/retention.ts
  - tools/evidence-store/src/uploads.ts
  - tools/evidence-store/src/worker.ts
  - tools/evidence-store/cli/operator.ts
  - tools/evidence-store/cli/recovery.ts
  - tools/evidence-store/justfile
  - tools/evidence-store/wrangler.jsonc
  - tools/evidence-store/api/openapi.yaml
  - tools/evidence-store/api/schemas/backup-manifest.schema.json
  - tools/evidence-store/api/examples/error.quota_exhausted.json
  - tools/evidence-store/api/examples/error.stale_snapshot.json
  - tools/evidence-store/api/examples/finalizeUpload.response.json
  - tools/evidence-store/api/examples/getMaintenance.response.json
  - tools/evidence-store/api/examples/recoverJobs.request.json
  - tools/evidence-store/api/examples/recoverJobs.response.json
  - tools/evidence-store/api/examples/setMaintenance.request.json
  - tools/evidence-store/api/examples/setMaintenance.response.json
  - tools/evidence-store/tests/fixtures/recovery/
  - tools/evidence-store/tests/node/proofs.test.ts
  - tools/evidence-store/tests/node/recovery-cli.test.ts
  - tools/evidence-store/tests/node/recovery.test.ts
  - tools/evidence-store/tests/service/capacity.test.ts
  - tools/evidence-store/tests/service/db-migrations.test.ts
  - tools/evidence-store/tests/service/fetch.test.ts
  - tools/evidence-store/tests/service/helpers.ts
  - tools/evidence-store/tests/service/maintenance.test.ts
  - tools/evidence-store/tests/service/recover-jobs.test.ts
  - tools/evidence-store/tests/service/recovery-helpers.ts
  - tools/evidence-store/tests/service/retention-daily.test.ts
  - tools/evidence-store/tests/service/retention-fixtures.ts
  - tools/evidence-store/tests/service/retention-jobs.test.ts
  - tools/evidence-store/tests/service/retention.test.ts
  - tools/evidence-store/tests/service/uploads.test.ts
docs_updated_during_phase_5:
  - tools/evidence-store/README.md
  - tools/evidence-store/api/README.md
  - tools/evidence-store/docs/contract-inventory.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
message_to_agent: |-
  Phase 6 (the final phase) FAILED on external prerequisites; read the
  "Phase 6" section of this log. Docs, recipes, the deploy and dispatch
  commands, the manual demo workflow, and the acceptance suite (7 cases,
  passing locally) are done; `just test` (35 files, 901 tests) and `just lint`
  pass in tools/evidence-store. Not done: every deployed step of "Demonstrate
  service", plus Phase 1's "Measure capacity" and "Prove D1", all blocked on
  missing CLOUDFLARE_API_TOKEN / CLOUDFLARE_ACCOUNT_ID and a valid GitHub token.
  The demo workflow must be merged to main before `just demo-hosted` can
  dispatch it. `implemented` is deliberately not set.
source_files_during_phase_6:
  - .github/workflows/evidence-store-demo.yml
  - justfile
  - tools/evidence-store/justfile
  - tools/evidence-store/package.json
  - tools/evidence-store/vitest.config.ts
  - tools/evidence-store/wrangler.jsonc
  - tools/evidence-store/cli/deploy.ts
  - tools/evidence-store/cli/dispatch-demo.ts
  - tools/evidence-store/tests/acceptance/cases.ts
  - tools/evidence-store/tests/node/deploy.test.ts
  - tools/evidence-store/tests/node/dispatch-demo.test.ts
  - tools/evidence-store/tests/real/acceptance.test.ts
  - tools/evidence-store/tests/service/acceptance.test.ts
docs_updated_during_phase_6:
  - docs/cicd/evidence.md
  - docs/dependencies.md
  - tools/evidence-store/README.md
docs_created_during_phase_6:
  - docs/cicd/evidence-store.md
skills_files_updated_during_phase_6:
  - .claude/skills/rust-devops/ci-cd.md
source_code:
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
  - tools/evidence-store/src/env.ts
  - tools/evidence-store/src/requests.ts
  - tools/evidence-store/src/config.ts
  - tools/evidence-store/src/tokens.ts
  - tools/evidence-store/src/credential-request.ts
  - tools/evidence-store/src/operator.ts
  - tools/evidence-store/src/reads.ts
  - tools/evidence-store/src/locations.ts
  - tools/evidence-store/src/db/audit.ts
  - tools/evidence-store/src/db/reads.ts
  - tools/evidence-store/cli/operator.ts
  - tools/evidence-store/migrations/0006_feed_horizons.sql
  - tools/evidence-store/tests/service/auth.test.ts
  - tools/evidence-store/tests/service/operator.test.ts
  - tools/evidence-store/tests/service/reads.test.ts
  - tools/evidence-store/tests/service/changes.test.ts
  - tools/evidence-store/tests/service/locations.test.ts
  - tools/evidence-store/tests/node/requests.test.ts
  - tools/evidence-store/tests/node/service-config.test.ts
  - tools/evidence-store/tests/node/operator-cli.test.ts
  - tools/evidence-store/tests/node/credential-request.test.ts
  - tools/evidence-store/src/uploads.ts
  - tools/evidence-store/src/db/uploads.ts
  - tools/evidence-store/src/contracts/proofs.ts
  - tools/evidence-store/cli/ingest.ts
  - tools/evidence-store/api/examples/finalizeUpload.response.json
  - tools/evidence-store/api/schemas/ingest-config.schema.json
  - tools/evidence-store/api/schemas/resume-state.schema.json
  - tools/evidence-store/tests/service/uploads.test.ts
  - tools/evidence-store/tests/service/fetch.test.ts
  - tools/evidence-store/tests/node/proofs.test.ts
  - tools/evidence-store/tests/node/ingest.test.ts
  - tools/evidence-store/tests/node/ingest-cli.test.ts
  - tools/evidence-store/tests/fixtures/ingest/fake-service.ts
  - tools/evidence-store/migrations/0007_maintenance.sql
  - tools/evidence-store/migrations/0008_retention_state.sql
  - tools/evidence-store/src/backup-manifest.ts
  - tools/evidence-store/src/capacity.ts
  - tools/evidence-store/src/daily.ts
  - tools/evidence-store/src/db/retention.ts
  - tools/evidence-store/src/maintenance.ts
  - tools/evidence-store/src/recovery.ts
  - tools/evidence-store/src/retention.ts
  - tools/evidence-store/cli/recovery.ts
  - tools/evidence-store/api/schemas/backup-manifest.schema.json
  - tools/evidence-store/api/examples/error.quota_exhausted.json
  - tools/evidence-store/api/examples/error.stale_snapshot.json
  - tools/evidence-store/api/examples/getMaintenance.response.json
  - tools/evidence-store/api/examples/recoverJobs.request.json
  - tools/evidence-store/api/examples/recoverJobs.response.json
  - tools/evidence-store/api/examples/setMaintenance.request.json
  - tools/evidence-store/api/examples/setMaintenance.response.json
  - tools/evidence-store/tests/fixtures/recovery/
  - tools/evidence-store/tests/node/recovery-cli.test.ts
  - tools/evidence-store/tests/node/recovery.test.ts
  - tools/evidence-store/tests/service/capacity.test.ts
  - tools/evidence-store/tests/service/maintenance.test.ts
  - tools/evidence-store/tests/service/recover-jobs.test.ts
  - tools/evidence-store/tests/service/recovery-helpers.ts
  - tools/evidence-store/tests/service/retention-daily.test.ts
  - tools/evidence-store/tests/service/retention-fixtures.ts
  - tools/evidence-store/tests/service/retention-jobs.test.ts
  - tools/evidence-store/tests/service/retention.test.ts
  - .github/workflows/evidence-store-demo.yml
  - justfile
  - tools/evidence-store/cli/deploy.ts
  - tools/evidence-store/cli/dispatch-demo.ts
  - tools/evidence-store/tests/acceptance/cases.ts
  - tools/evidence-store/tests/node/deploy.test.ts
  - tools/evidence-store/tests/node/dispatch-demo.test.ts
  - tools/evidence-store/tests/real/acceptance.test.ts
  - tools/evidence-store/tests/service/acceptance.test.ts
documentation:
  - tools/evidence-store/README.md
  - tools/evidence-store/api/README.md
  - tools/evidence-store/docs/contract-inventory.md
  - docs/cicd/evidence.md
  - docs/dependencies.md
  - docs/cicd/evidence-store.md
  - .claude/skills/rust-devops/ci-cd.md
---
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

## Phase 4

- 2026-11-11: preflight. Phases 2 and 3 are now committed (`423595f3e`), so
  the uncommitted tree is this phase alone. Cloudflare/GitHub credentials are
  still unset; Phase 4 deploys nothing (Worker routes, CLI, local Workers-pool
  and Node tests), so it proceeds on the same basis as Phases 2 and 3.
- Design settled in the main session before fan-out (recorded in
  `decisions_for_review`): R2 multipart needs 5 MiB parts, so each 1 MiB part
  is its own content-addressed staging object and finalization streams the
  parts into the final object; whole-object SHA-256 uses the Workers-native
  `crypto.DigestStream`; finalization inserts a deterministic `verified`
  `object` location event so replays are idempotent; ingestion uses flags
  (`--endpoint` added) and resolves every reference inside `--proof-dir`
  before any network request; a separate `evidence-finalize` command runs the
  verifier credential.
- Wave 6 runs as two parallel foreground subagents: transfers (Worker) and
  ingestion (CLI).
- **Implement transfers** (done): `src/uploads.ts`, `src/db/uploads.ts`,
  `src/contracts/proofs.ts`, routes in `src/worker.ts` (a new
  `matchTransferRoute` for the 4-segment upload paths), and a shared
  `checkReference`/`locationInsert` in `src/locations.ts`.
  - `POST /uploads`: the reference must exist in the stored observation (a
    report's `bytes` must match). Above 64 MiB → 413 while the observation
    stays recorded. An open or finalized upload for the same reference is
    returned with 200; a conflicting length → 409.
  - `PUT …/parts/{index}`: each part is a content-addressed staging object.
    Parts are sequential, sizes are exact, an identical replay → 200, and
    different bytes → 409. An expired upload → 404 and is marked `expired`.
  - `POST …/finalize` (verifier only):
    - before writing, an R2 `head` checks every part's size;
    - the parts are streamed into the final object through
      `crypto.DigestStream` (it works in the vitest workers pool; the
      @noble/hashes fallback was not needed), and the length and digest
      are verified;
    - proof documents get the ported structural and cell-binding checks;
    - one D1 batch marks the upload finalized and inserts the deterministic
      `verified` object event. A failure rejects the upload, deletes the
      final object, and answers 400.
  - `GET /objects/…`: streams a finalized object with `Content-Length` and
    `X-Content-SHA256` unless a newer withdrawn or unavailable event exists.
    A missing R2 object → 503.
  - `cleanupStaging`: expires stale open uploads; removes staging older than
    48 h for unfinalized uploads, and leftover parts and orphans for the
    rest.
- **Build ingestion** (done): `cli/ingest.ts` with `ingest`, `resume`, and
  `finalize`, plus the just recipes `evidence-ingest`, `evidence-resume`, and
  `evidence-finalize`.
  - Every input is read and checked before any request: flags, token file,
    the strict input, v2 adaptation on this host, and the `--proof-dir` map
    with every file's digest checked.
  - The state file is written atomically before the first request and after
    every acknowledgment. The original input and the submitted collection
    are kept beside it.
  - Retry policy: 429, 503, and network failures only; 5 attempts; full
    jitter capped at 30 s; Retry-After honored up to 60 s; 60 s timeout per
    request. On a persistent failure the CLI prints
    `just evidence-resume <state>` and exits 1.
  - Nothing calls the CLI from publish-receipt, hooks, `ci.yml`, `seed.py`,
    or `affected_scope.py`.
- Main-session integration: dropped the dead `r2_upload_id` column from the
  undeployed migration 0004 (the subagent flagged its comment as stale);
  added "Uploaded proof documents" and "Ingestion CLI inputs" sections to
  `docs/contract-inventory.md`.
- Requirement → test mapping:
  - interrupted parts, wrong part size/index gap, identical vs different
    replay, expiry → `tests/service/uploads.test.ts` "parts" block.
  - wrong size (tampered staged part) and wrong digest → rejected with the
    observation unchanged; missing proof reference, unreferenced upload,
    completion mismatch (package/gate/head/environment), target mismatch,
    non-strict JSON proof → `uploads.test.ts` "finalize" and "POST /uploads".
  - finalize after R2 success and D1 failure (a `Proxy` database whose
    `batch()` throws once) → one event after retry → "finalizes once when the
    D1 batch fails after the object was written".
  - verifier only: producer → 403 with nothing written; reader → 403 on
    upload.
  - withdrawal during retrieval, withdrawal without revival on finalize
    replay, independent host-dir vs object locations, expiry ignored, lost
    R2 object → 503 → `tests/service/fetch.test.ts`.
  - staging cleanup → `uploads.test.ts` "cleanupStaging".
  - proof validators: each field shape once, control rows, binding →
    `tests/node/proofs.test.ts`.
  - CLI: v3 and v2 end to end, refusals before network (zero requests),
    lost response, 503/429/give-up/no-retry-on-400, resume from the server's
    `parts_received`, tampered retained collection, part 409, state field and
    document shapes, no token or report bytes in output or state →
    `tests/node/ingest.test.ts`; spawned control over loopback http (paths
    with spaces) plus malformed input and malformed state → exit 2 with a
    location → `tests/node/ingest-cli.test.ts`.
- Plan checkpoint: no report content or token appears in errors or logs
  (proof errors never quote document content; the CLI test asserts no
  token or report bytes in output or state). Upload retry never reruns a
  test or modifies an observation (interrupted, rejected, and replayed
  uploads leave the export unchanged).
- Code-smell search (`??`, `.filter(`, `catch`, defaults) over the new code:
  - the `.filter` in proofs.ts selects missing required fields;
  - the `catch` blocks in uploads.ts swallow only best-effort deletes and
    stream-abort promises after the main error is decided;
  - the CLI's `catch` blocks rethrow located `CliError`s;
  - the `??` uses only shape the summary printout.
- Gates (2026-11-11, macOS, run once after both subagents finished, nothing
  else running), in `tools/evidence-store`: `just lint` → exit 0; `just
  test` → 24 files, 769 tests passed, none skipped, about 3 s. No failures,
  no load-sensitive reruns.
- Scope calculator over the uncommitted tree → `packages: []`,
  `change_class: documentation` (the pnpm tool is outside CI's Cargo scope
  registry). The gates ran anyway because source files changed. No package
  depends on `@rusty-biscuit/evidence-store`, so there were no dependent
  tests to run.
- Cross-OS: no Cargo package changed, so `just cross-check` does not apply.
  The CLI uses `node:path`, no shell quoting, and no stdin; it is untested
  on Windows.
- Deferred: cron scheduling of cleanup and capacity reservations (Phase 5),
  and any deployed transfer run (Phase 6, credentials unset).
- **Phase 4 outcome: implementation complete.** "Implement transfers" and
  "Build ingestion" are checked off.

## Phase 5

- 2026-11-11: preflight. Phase 4 is still uncommitted (git status shows its
  files), so the uncommitted tree is Phases 4 and 5 together; the scope
  calculator result at the end covers both. Cloudflare/GitHub credentials
  remain unset; Phase 5 deploys nothing (Worker routes, migrations, operator
  CLI, local Workers-pool and Node tests).
- Shared foundation written in the main session before fan-out, so the two
  Wave 7 subagents own disjoint files:
  - `migrations/0007_maintenance.sql`: single-row `service_state` with a
    persisted `maintenance_mode` (`open` / `quiesced`), plus triggers that
    refuse every evidence, location, upload, retention, repository, and
    credential-issue write with `maintenance_mode` while quiesced. The audit
    log and credential revocation are not blocked.
  - `src/maintenance.ts`: `readMaintenance`, `assertWritable`, and the
    trigger-error mapping. The Worker checks the mode before every mutation
    route (503 `unavailable`, retry after 60 s) and maps a trigger refusal
    that races the check to the same answer.
  - `src/daily.ts` and the Worker's `scheduled` handler, with a daily cron
    (`17 3 * * *`) in both Wrangler environments. It skips everything while
    quiesced and currently runs the Phase 4 staging cleanup.
  - Migration 0003's audit actions gained `retention.recover`,
    `maintenance.quiesce`, and `maintenance.resume` (undeployed, so edited in
    place), mirrored in `src/db/audit.ts`, `src/operator.ts`, and OpenAPI.
  - `src/retention.ts` interface stub (`advanceRetentionJobs`) shared by the
    retention runner and the operator `recover-jobs` route.
  - `just test` after the foundation: 24 files, 769 tests passed.
- Wave 7 runs as two parallel foreground subagents: retention (certificates,
  jobs, capacity, daily steps) and recovery (maintenance routes, backup,
  restore, verify-backup, recover-jobs).
- The subagents ran in the background, not the foreground as briefed; the
  main session waited for both to finish within the same turn and did not
  run tests while they worked.
- **Guard retention** (done): `src/retention.ts`, `src/db/retention.ts`,
  `src/capacity.ts`, `migrations/0008_retention_state.sql`, budget
  admission in `src/uploads.ts`, retention routes in `src/worker.ts`, and
  the daily steps in `src/daily.ts`.
  - `POST /retention/certificates` (retention-verifier only) checks
    everything before writing: exact snapshot, exact candidate set per
    cell, a protected maximal set, same-cell acyclic edges, ancestry heads
    bound to stored heads, retry chains against stored `retry_of`, and
    release pins. It then writes the certificate, first-certification
    times, the job, the members, the pins, and the audit row in one batch.
  - `GET /retention/jobs/{id}` is answered to a retention verifier or an
    operator.
  - `advanceRetentionJobs` runs the job in order: withdraw, lock the cells,
    delete the objects, advance the epoch, delete the rows, then complete
    and unlock. Each step is a bounded, idempotent batch.
  - Main-session follow-up on the subagent's finding: a job whose grace
    has already ended now starts at acceptance (see `decisions_for_review`).
    New test: "starts a job at submission when its grace has already ended,
    so a later write cannot invalidate it". `api/README.md` and OpenAPI
    were updated to match; OpenAPI also had "earliest time" where the code
    uses the latest first certification (a drifted description, corrected
    to follow the code).
- **Implement recovery** (done): `src/recovery.ts` adds the operator routes
  `POST`/`GET /v1/operator/maintenance` and `POST /v1/operator/jobs/recover`.
  `src/backup-manifest.ts` is the strict manifest reader. `cli/recovery.ts`
  implements `backup`, `verify-backup`, `restore`, `recover-jobs`, and
  `maintenance`, dispatched from `cli/operator.ts` and run through
  `just operator …`. `api/schemas/backup-manifest.schema.json` was
  rewritten.
- Main-session integration:
  - Fixed the shared `retentionJob` test helper: its `submitted_by` was not
    publisher JSON, and the real runner parses that field.
  - Split the crash-at-every-boundary test, which had looped over every
    failure point in one 2.7 s test, into twelve range tests plus a guard
    test proving the ranges cover a clean run's operations. Each takes
    under 0.35 s.
  - Added the "Retention certificates" and "Backup manifest and recovery
    commands" sections to `docs/contract-inventory.md`.
  - Updated the README status list.
- Requirement → test mapping:
  - Retention, in `tests/service/retention.test.ts`:
    - "protects the current pass…"
    - "protects an unresolved failure…"
    - "protects a same-head pass/fail conflict…"
    - "protects observations on unrelated heads and binds ancestry claims
      to the stored heads"
    - "prunes a failed attempt resolved by a stored retry link…"
    - "refuses a stale or pruned snapshot…"
    - "refuses a partial candidate set and missing store data…"
    - "refuses edges that cross cells…"
    - "stores release pins idempotently and never prunes a release-cited
      observation"
    - "answers an identical replay with 200…"
    - "treats an empty prune set as a completed no-op job"
    - "refuses every role but retention-verifier with 403 and writes
      nothing"
    - "answers 503 and writes nothing when the D1 write fails"
    - field-shape walk, field wiring, and loader wiring.
  - Retention jobs, in `tests/service/retention-jobs.test.ts`:
    - "withdraws, deletes objects, then deletes rows, keeping a retained
      observation's identical bytes" (shared objects)
    - "answers snapshot_pruned to a snapshot or cursor captured before the
      epoch advanced"
    - "measures the grace from the first certified supersession…"
    - "starts a job at submission when its grace has already ended…"
    - "invalidates a scheduled job when the revision changed…"
    - "locks the cell during deletion…"
    - "does nothing while the store is quiesced"
    - "crash at every boundary" (13 tests: D1 statements, batches, and R2
      deletes, failing both before and after taking effect).
  - Daily run and tombstones, in `tests/service/retention-daily.test.ts`:
    the scheduled run, tombstone expiry, and the horizon moving to
    `cursor_expired`.
  - Budget admission, in `tests/service/capacity.test.ts`: reserve and
    commit, the storage threshold (429, nothing written, ingestion
    unaffected), the monthly class A threshold, release, and the
    numbers-only alert.
  - Maintenance, in `tests/service/maintenance.test.ts`:
    - the boundary and idempotent quiesce;
    - 503 with zero D1/R2 writes on all eight mutation routes;
    - reads and audit still work;
    - revocation is allowed;
    - a racing write is refused by the trigger;
    - reopening restores writes.
  - Recover jobs, in `tests/service/recover-jobs.test.ts`.
  - Recovery, in `tests/node/recovery.test.ts`:
    - the round trip into isolated resources over a fake Wrangler backed
      by `node:sqlite` built from the real migrations (identical rows, no
      duplicated events, rotated generations and keys, an `unavailable`
      event for a missing object, the pending upload expired);
    - proof of the statement reorder;
    - replay classification;
    - window revocations;
    - backup always reopens and handles a lost quiesce answer;
    - seven-backup rotation;
    - refusing the live source as destination, a tampered backup (nothing
      executed), and a non-empty destination;
    - 29 manifest field-shape cases plus loader wiring.
  - Spawned CLI wiring, in `tests/node/recovery-cli.test.ts`: one control
    and two malformed manifests.
- Plan checkpoint:
  - Synthetic pruning with fault injection at every D1 and R2 boundary
    never leaves a row advertising deleted bytes (`expectNoDanglingAvailability`
    after every injected crash). Restore appends `unavailable` for every
    object the backup lacks.
  - Protected evidence survives quota exhaustion (admission only refuses
    new uploads and never deletes) and cleanup (staging cleanup touches
    only unfinalized or orphaned staging).
- Code-smell search over the new code:
  - `??` defaults: graph adjacency lookups, absent optional ingest-state
    lists, and absent repeated CLI flags. All are intentional and recorded
    in the inventory.
  - `.filter`: selection only (present versus absent IDs, active operator
    credentials, replayable uploads whose digest still matches; the dropped
    ones are counted in `uploads_not_replayed`), never silently discarding
    invalid input.
- Gates (2026-11-11, macOS, run after both subagents had finished and
  nothing else was running), in `tools/evidence-store`:
  - `just lint` → exit 0.
  - `just test` → 32 files, 880 tests passed, none skipped.
  - No failures and no load-sensitive reruns. The slowest single test is
    now under 0.4 s.
- Scope calculator over the uncommitted tree (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`) → `packages: []`, `change_class:
  documentation`. The pnpm tool is outside CI's Cargo scope registry. The
  gates ran anyway, because source files changed. No package depends on
  `@rusty-biscuit/evidence-store`, so there were no dependent tests.
- Cross-OS: no Cargo package changed, so `just cross-check` does not apply.
  The recovery CLI uses `node:path`, spawns without a shell, and pipes the
  cursor key through stdin. Its Windows behavior (token-file ACL, paths) is
  untested.
- Deferred to Phase 6: any deployed backup or restore, and checking the
  three Wrangler output assumptions listed in `message_to_agent`.
- **Phase 5 outcome: implementation complete.** "Guard retention" and
  "Implement recovery" are checked off.

## Phase 6

- Started 2026-11-11 on macOS. Phases 1–5 are committed, so the
  uncommitted tree is this phase alone. External prerequisites were checked
  first and are **still missing**: `CLOUDFLARE_API_TOKEN`,
  `CLOUDFLARE_ACCOUNT_ID`, `GITHUB_TOKEN`, and `GH_TOKEN` are unset; Wrangler's
  stored OAuth token has expired and cannot refresh non-interactively; the
  `gh` CLI token is invalid. Nothing can be deployed, dispatched, or restored
  from this host.
- Work was split across three subagents with separate files (docs; deployed
  acceptance suite; deploy/dispatch/workflow tooling), then integrated and
  gated here once all three had finished.
- **Publish operations** (done):
  - `docs/cicd/evidence-store.md` (new): a task-by-task operating guide
    covering status (Git notes stay authoritative; retention qualification and
    evaluation consumers not enabled), roles and credential rotation,
    provisioning and deployment/migrations, bounded ingestion and resume, proof
    staging and finalization, outage diagnosis, quota alerts, retention, daily
    backups, restore, recovering jobs, rollback, the hosted demonstration, and
    the deployed acceptance run. It has Mermaid diagrams for the upload and
    recovery flows. It names no feature directory.
  - `docs/cicd/evidence.md`: short link section that also states the service
    is not authoritative. `docs/dependencies.md`: a `tools/evidence-store` note.
    `.claude/skills/rust-devops/ci-cd.md`: "The evidence store (explicit
    ingestion only)". `tools/evidence-store/README.md`: cut back to an
    overview that links to the guide.
  - Recipes: package `typecheck`, `test`, `lint`, `test-real`, `deploy`,
    `demo-hosted`, and the operator/ingest recipes, plus pnpm scripts
    `deploy`, `demo-hosted`, `test-real`. The root justfile gains one explicit
    delegation, `just evidence-store <recipe>`. No CI cell was added and no
    scope registry was changed.
  - `cli/deploy.ts`: exits 2 when Cloudflare credentials are missing or the
    environment's `database_id` is the placeholder. Otherwise it runs
    `wrangler d1 migrations apply --remote` and then `wrangler deploy`, and
    writes a redacted manifest (resources, migration SHA-256s, Worker version,
    endpoint; no secrets or account ID).
  - `cli/dispatch-demo.ts` plus `.github/workflows/evidence-store-demo.yml`
    (workflow_dispatch only; contents/actions read, id-token write). It
    downloads a named retained artifact, mints an OIDC token for the service
    audience into a 0600 file, ingests as a producer, and uploads the resume
    state for host-side finalization. The dispatcher exits 2 ("external
    prerequisite unavailable") when the workflow is not on `main`, and never
    pushes.
  - `wrangler.jsonc`: `workers_dev: true` for both environments. Before this,
    a deployed Worker would have had no address at all.
  - Drift review of comments: `vitest.config.ts`'s project header (now four
    projects) and the `test-l2` stub text, which pointed at `prove-d1` and now
    points at `test-real`, were corrected; the wrangler header comment now
    mentions the workers.dev choice.
- **Demonstrate service** (NOT done, blocked):
  - Built: the deployed acceptance suite. `tests/acceptance/cases.ts` holds 7
    transport-neutral cases: concurrent mixed-identity rollback; revoked and
    unauthorized identities; expired identity; interrupted upload/finalization
    replay; location withdrawal without revival (including object fetch 404
    after withdrawal); change-feed replay; finite snapshot pagination during
    writes. Each case issues its own credentials through the operator API and
    revokes them in `finally`.
    - `tests/service/acceptance.test.ts` runs every case in process against
      the real authenticator (real `ebt_` tokens, digests in local D1): 7
      passed. Two assertions were broken deliberately and failed as expected,
      then restored.
    - `tests/real/acceptance.test.ts` (vitest project `real`, present only
      under `EVIDENCE_STORE_REAL=1`) runs the same cases over HTTP. With its
      variables missing, every case fails naming them; `just test-real`
      exits 2 before vitest starts. The expiry case is reported as skipped
      deployed, because it needs a movable clock.
  - Not done, and not possible without the credentials: deploying the
    development Worker/D1/R2; the redacted resource manifest; registering the
    real repository; demonstration grants; authentic host and hosted
    ingestion; dispatching the demo workflow (it must first be merged to
    `main`); `just test-real` against the deployment; backup, isolated
    restore, digest-checked retrieval, interrupted-job resume, pre-restore
    cursor rejection; and checking the three Wrangler output assumptions from
    Phase 5. No result was fabricated.
- Requirement → test mapping (new tests):
  - deploy prerequisites, placeholder refusal, argv order, manifest content,
    failing-migration stop, spawned wiring → `tests/node/deploy.test.ts` (8).
  - dispatch token check, workflow-absent exit 2, dispatch body →
    `tests/node/dispatch-demo.test.ts` (5).
  - deployed correctness cases → `tests/service/acceptance.test.ts` (7,
    local) and `tests/real/acceptance.test.ts` (deployed; blocked).
  - Input robustness: no new file-format or configuration loader was added.
    `deploy.ts` reads `wrangler.jsonc` through the existing
    `stripJsonComments` path; it is a tool-owned build input, which the plan
    excludes from the matrix. CLI flags are validated with located exit-2
    errors.
- Gates (2026-11-11, macOS, run after every subagent had finished), in
  `tools/evidence-store`:
  - `just lint` → exit 0.
  - `just test` → 35 files, 901 tests passed, none skipped.
- Scope calculator (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`) → `packages:
  [test-toolkit, darkmatter, repo-deps]`, `change_class: package`. The
  workflow, skill, and docs are read by test-toolkit's workflow contracts,
  `docs/cicd` by darkmatter's shipped-schema corpus, and the root justfile by
  repo-deps' archive test. Run serially:
  - `cargo nextest run -p darkmatter -E 'test(shipped_markdown_inline_schemas_parse_passively) | test(shipped_markdown_named_type_references_resolve)'`
    → 2 passed.
  - `cargo nextest run -p test-toolkit --test ci_workflow_contracts` → 228
    passed. `the_shipped_planner_emits_plans_the_guards_reader_accepts` was
    slow (5.4 s); it is a pre-existing planner test this phase did not change.
  - `cargo nextest run -p repo-deps --bin ci-build -E 'test(the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach)'`
    → 1 passed. `just _tier_filter` and `just _native_path` also run cleanly
    against the edited root justfile.
- Cross-OS: no Rust source changed, so `just cross-check` does not apply.
  The new CLIs spawn Wrangler through Node without a shell. The pnpm
  `test-real` script relies on the justfile to set `EVIDENCE_STORE_REAL`,
  because the `VAR=1 cmd` syntax does not work in Windows shells. Windows
  behavior of the new CLIs is untested.
- **Phase 6 outcome: FAILED (blocked on external prerequisites).** "Publish
  operations" is checked off. "Demonstrate service" stays unchecked, and the
  plan's final checkpoint ("mark implementation complete only after all
  acceptance checks pass") is not met. `implemented` is therefore NOT set.
  To finish: provide `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`, and a
  valid GitHub token; run the one-time provisioning in
  `docs/cicd/evidence-store.md`; merge `.github/workflows/evidence-store-demo.yml`
  to `main`; then run the demonstration steps and `just test-real`. Phase 1's
  feasibility tasks ("Measure capacity", "Prove D1") remain open on the same
  credentials.

### Phase 6 — credentialed run (2026-11-11)

Run on macOS (`bolt`) with a scoped Cloudflare token (D1, Workers Scripts,
and Workers R2 Storage, all Edit) and a GitHub token. Every artifact below
is in this directory and was checked for the Cloudflare token, the account
ID, the GitHub token, the operator token, and `ebt_` prefixes: none appear.

- **Provision and deploy.** New `just evidence-store provision` creates an
  environment's D1 database, private R2 bucket, `database_id`, and
  `CURSOR_HMAC_KEY`, idempotently (a rerun on a provisioned environment
  makes three read-only checks and changes nothing). `development` was
  provisioned and deployed; `deploy-development.json` is the redacted
  manifest (8 migrations by SHA-256, endpoint
  `https://evidence-store-development.inocan.workers.dev`).
- **Registration.** `yankeeinlondon/rusty-biscuit` (1131837348) and a
  private, empty `yankeeinlondon/evidence-store-acceptance` (1454439808)
  are registered and allowlisted, so acceptance writes never enter the real
  repository's history. Only the demo workflow on `main` is OIDC-trusted.
- **Deployed acceptance.** `just test-real` → 6 passed, 1 skipped (expiry
  needs a movable clock), 0 failed (`test-real-development.json`).
- **Prove D1: PASSED** (`prove-d1-development.json`). All 10 D1 checks pass
  on a disposable database, including 20 concurrent conflicting pairs with
  no surviving loser sibling, manifest, member, event, or revision. The
  spike's CPU proxy passed at 256 observations / 1,044,480 bytes. The real
  validator was then sampled on the deployed service: a 256-observation,
  925,964-byte submission was accepted (200, ~750 ms wall), and its replay
  answered `already_present` for all 256. No CPU-limit error occurred, so
  the advertised limits stand. The account's Workers plan (Free or Paid)
  could not be read with this token, so whether Free's 10 ms CPU limit
  applied is not established.
- **Measure capacity: PASSED** (`inventory-2026-11-11.json`, exit 0,
  `feasible: true`). The first credentialed run failed for the two reasons
  Phase 1 recorded: the `cloudflare-account-usage` source was unimplemented,
  and the 12-month no-pruning forecast put R2 at 71.2 GB, 829% of the 8 GiB
  allowance. The owner then ruled "store only the latest passing run per
  cell until retention exists" (see decisions_for_review), and two changes
  followed in `cli/inventory.ts`:
  - `measureAccountUsage` reads whole-account usage, read-only, with the
    scoped token: D1 `file_size` per database, R2 `/usage` per bucket, and
    the GraphQL analytics for D1 rows written (busiest day in 30) and R2
    Class A operations (30-day total). It now reports D1 2.85 MB, R2
    10.8 MB, 12,860 rows written on the busiest day, and 161 Class A
    operations. The forecast adds these to every month, so the store's own
    usage is counted twice on purpose.
  - R2 storage is projected under the spec's retention: twice (211 observed
    cells + 6 unresolved + 0.2/day growth) plus 14 days of twice the daily
    volume, at p95 sizes. The never-pruned projection is still reported as
    `forecast.unpruned_r2`, information only, with the constraint that keeps
    it hypothetical.
  - Result (2,974 observations; daily mean 63, max 387): D1 141.3 MB of
    500 MB (28.3%); D1 rows written 18,933 of 100,000/day (18.9%, of which
    12,860 is current account usage); R2 3.37 GB of 8 GiB (39.2%); R2
    Class A 20,214 of 800,000/month (2.5%). No alert or breach month.
    Never pruned, R2 would reach 70.2 GB (817.5%) in month 12, crossing the
    refusal line in month 1.
- **Host ingestion.** An authentic retained `just ci-seed` collection
  (windows-latest, head `758c3985`, 7 observations, original IDs) was
  ingested into `rusty-biscuit` with a temporary producer grant; all 20
  report, expected-listing, and completion-record files were staged and
  finalized with a separate temporary verifier grant.
- **Hosted ingestion: NOT DONE.** `.github/workflows/evidence-store-demo.yml`
  is not on `main`; dispatching it waits for the merge.
- **Backup and restore** (`recovery-report-restore-test.json`,
  `proof-digest-check.json`). A reader captured a change-feed cursor, then
  backup `a6873e19…` was taken (boundary quiesced 3.1 s; 22 objects;
  `verify-backup` ok). After the boundary, a second authentic collection
  (wsl2-ubuntu, 7 observations, 21 files) was ingested into `development`
  and deliberately killed with 3 uploads staged and 18 pending. The restore
  into a freshly provisioned `restore-test` replayed that state: 0
  recovered, 7 replayed, 0 unrecoverable, 0 missing objects; both
  repositories' generations rotated and a new cursor key was set. On
  `restore-test`, the 21 replayed uploads were finalized, all 14
  observations' 41 files (9,713,679 bytes) were fetched and matched their
  SHA-256 in body and `X-Content-SHA256`, and the pre-restore cursor was
  refused with `400 invalid_cursor`. `recover-jobs` found nothing to do.
  The interrupted job was then resumed and finalized on `development`, where
  the same 41-file digest check passes. Acknowledged loss: known only for
  the replayed state; none was unrecoverable.
- **Cleanup.** The producer, verifier, reader, and capacity-sample grants
  were revoked (each now answers 401). The `restore-test` Worker, D1
  database, and R2 bucket (43 objects) were deleted, and its `database_id`
  returned to the placeholder. `development` and its genuine records remain.

Defects found and fixed during the run (each with a test):

- Completion records from `rig` and `seed` producers were bound by `head`,
  so every authentic seed proof was rejected: a seed run proves each cell
  on its own same-tree commit. `completionBindingProblems` now binds head,
  run, and attempt for `hosted` producers only, as `local_evidence.py` does.
  This departs from the plan's wording ("package, gate, environment, head,
  and target match"); the docs now state the per-producer rule.
- `evidence-finalize` could not recover a rejected or expired upload. It
  now returns such an upload to the producer (state `pending`), keeps going,
  and names `evidence-resume` then `evidence-finalize`; the service already
  opens a fresh upload for the same reference. The service's message for an
  already-rejected upload no longer claims a byte mismatch.
- `prove-d1` treated Cloudflare's HTML 404 for a not-yet-routed workers.dev
  address as "ready", and its `wrangler delete` cleanup failed after the
  Worker was gone (it lists KV, outside the token's scope). Readiness now
  needs the spike's JSON 404, and cleanup deletes through the API.
- `deploy` recorded Wrangler's telemetry URL as the endpoint; it now takes
  only `https://<worker>.<subdomain>.workers.dev`.
- The `operator`, `inventory`, `prove-d1`, `capture-parity`, `install`, and
  `test` recipes passed `{{ args }}`, splitting quoted arguments (a revoke
  `--reason` with spaces failed); they pass `"$@"`.
- `restore-test` had an empty repository allowlist, so a restored copy would
  deny everything; it now lists the same repositories and trusts no
  workflow. `provision` no longer suggests a migrating `deploy` for it, and
  the docs give the code-only `just evidence-store wrangler deploy`.
  Preview URLs are off in both environments.

Gates after the fixes (macOS): `just evidence-store lint` → exit 0;
`just evidence-store test` → 36 files, 916 tests passed.

**Phase 6 outcome: still incomplete.** "Demonstrate service" stays unchecked
until the hosted ingestion runs (needs the merge to `main`). Phase 1's
"Measure capacity" now passes (above) and is checked. `implemented` is not
set.

Capacity follow-up (2026-11-11): `cli/inventory.ts`,
`tests/node/inventory.test.ts` (forecast under retention, the never-pruned
projection staying out of the checks, account-usage offsets, and three
`measureAccountUsage` cases against a fake Cloudflare: summing, missing
credentials, a 403 that names the endpoint but never the token),
`docs/cicd/evidence-store.md` ("Forecast capacity with the inventory", and
the stale "Nothing is deployed yet" bullet now says only development is
deployed), and the ruling recorded on `2026-11-06-rust-evidence-client`.
Gates: `just evidence-store lint` → exit 0; `just evidence-store test` → 36
files, 922 tests passed.
