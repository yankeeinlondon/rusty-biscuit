---
kind: plan
total_phases: 4
created: 2026-11-08
phase: 5
agent: codex/gpt-6.1-sol
yolo: 'true'
source_files_during_phase_1:
- scripts/ci/storage_sample.py
- scripts/ci/test_storage_sample.py
- scripts/ci/produce-owner.sh
- scripts/ci/affected_scope.py
- scripts/ci/test_affected_scope.py
- scripts/ci-build.rs
- scripts/ci-build-archive.rs
- scripts/ci-build-tests.rs
- scripts/ci-build-archive-tests.rs
- scripts/Cargo.toml
- .github/workflows/ci.yml
- tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_1:
- docs/topics/ci-cd.md
- .github/ci/README.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/rust-devops/ci-cd.md
source_files_during_phase_2:
- scripts/ci/artifacts/produce.cjs
- scripts/ci/artifacts/produce.test.cjs
- scripts/ci/artifacts/publish.cjs
- scripts/ci/artifacts/publish.test.cjs
- scripts/ci/produce-owner.sh
- scripts/ci/build_status.py
- scripts/ci/test_build_status.py
- scripts/ci/runner_loss.py
- scripts/ci/test_runner_loss.py
- scripts/ci/affected_scope.py
- scripts/ci/test_affected_scope.py
- scripts/ci-build-archive-tests.rs
- scripts/ci-build-tests.rs
- scripts/ci-rollup.rs
- scripts/ci-rollup-tests.rs
- scripts/Cargo.toml
- .github/workflows/ci.yml
- .github/workflows/_package-ci.yml
- .github/workflows/_wsl-ci.yml
- tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_2:
- .github/ci/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
- repo-deps
- test-toolkit
source_files_during_phase_3:
- tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_3:
- docs/topics/ci-cd.md
- .github/ci/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/rust-devops/ci-cd.md
- .claude/skills/rust-testing/ci-execution.md
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
- scripts/ci/storage_sample.py
- scripts/ci/test_storage_sample.py
- scripts/ci/produce-owner.sh
- scripts/ci/affected_scope.py
- scripts/ci/test_affected_scope.py
- scripts/ci-build.rs
- scripts/ci-build-archive.rs
- scripts/ci-build-tests.rs
- scripts/ci-build-archive-tests.rs
- scripts/Cargo.toml
- .github/workflows/ci.yml
- tools/test-toolkit/tests/ci_workflow_contracts.rs
- scripts/ci/artifacts/produce.cjs
- scripts/ci/artifacts/produce.test.cjs
- scripts/ci/artifacts/publish.cjs
- scripts/ci/artifacts/publish.test.cjs
- scripts/ci/build_status.py
- scripts/ci/test_build_status.py
- scripts/ci/runner_loss.py
- scripts/ci/test_runner_loss.py
- scripts/ci-rollup.rs
- scripts/ci-rollup-tests.rs
- .github/workflows/_package-ci.yml
- .github/workflows/_wsl-ci.yml
documentation:
- docs/topics/ci-cd.md
- .github/ci/README.md
- .claude/skills/rust-devops/ci-cd.md
- .claude/skills/rust-testing/ci-execution.md
completed_phase: 4
implemented: true
---

# Ubuntu archive storage implementation plan

## Summary and Definition of Done

Instrument the existing Ubuntu archive owner, use one hosted diagnostic to select a bounded storage remedy, and attribute blocked archive consumers before downloading their builds. Preserve shared compilation, archive verification, scheduling, and the existing merge gate. Implementation sessions execute all phases without an intermediate human gate; hosted evidence that cannot be obtained is reported explicitly, never replaced by a local claim.

Completion requires focused producer, publisher, consumer, and workflow checks; current documentation; and an implementation log containing the storage observations and remedy. Hosted acceptance additionally requires one full-workspace Ubuntu owner to publish every planned record within 180 minutes with positive observed free space, and one planned WSL2 consumer to reach test execution using its verified archive. Subsequent WSL2 test failures remain failures and belong to the follow-up fix. Unavailable hosted evidence leaves hosted acceptance **unverified**, and exhausted capacity leaves it **failed**; neither is reported as a completed storage fix.

## Phase 1 — Establish storage evidence

### Necessary Rules

1. **Keep the existing owner and identity model.** Do not change planner scope, runner labels, timeout, Cargo profiles, schema versions, artifact names, or compatibility checks. `ci.yml::build` has one owner per producer environment; `ci-build-archive.rs::run_produce` sorts records by `(package, key)` and shares the target directory. `ProduceOptions::key` already supports one-record production. There is no need for a planner or public archive-schema change.
2. **The dominant consumer is not known yet.** The specification establishes disk exhaustion but gives no category sizes. Planning read `produce-owner.sh`, `publish.cjs::publish`, and `produce_one`: archives accumulate until publication, inventory extraction is deleted only on the normal path, and the producer restores a target cache. None establishes dominance. `gh run list` located the November 6 run `42246114963`; `gh run view --log-failed` returned HTTP 403 requiring repository admin rights. Do not infer a measured cause from these facts. Run exactly one diagnostic, with the outcomes below already settled.
3. **Use boundary measurements, not a benchmark.** Add `scripts/ci/storage-sample.py`, a Python-stdlib diagnostic utility invoked by the workflow and producer. On Ubuntu collect `os.statvfs` free bytes and inodes for the target and output locations, device identity, recursively counted allocated directory bytes, elapsed monotonic time, cache-hit output, and sample label. Resolve a missing directory to its nearest existing parent for filesystem statistics. Report unavailable measurements explicitly; diagnostic errors must not replace the underlying build exit status. Categories are `target`, retained record archives/copied sidecars, inventory scratch, and schematic guard temporary output; report overlapping categories without adding them into a free-space figure. No new Rust dependency or terminal renderer is required for diagnostic JSONL.
4. **Select remedies by recoverable bytes at the lowest-free-space boundary.** Rank three disjoint removable sets: provably obsolete restored target output; completed record archives and copied sidecars; abandoned producer-owned inventory/packaging directories after their last reader. Count restored target entries as obsolete only when their paths belong to a target triple or Cargo profile absent from every build identity in this owner's plan. Before/after restore inventories establish their origin. Do not use access times, Cargo fingerprint guesses, or classify a whole target tree as stale merely because it is large. Unclassified target bytes remain live/unknown, not recoverable. Select the largest nonzero measured set, breaking ties in the order archives, temporary output, stale cache. Apply the corresponding Phase 2 task. Select additional sets only when the measured recoverable bytes of the first set are smaller than the observed growth between the two latest successful record boundaries; select in the same ranking order. With fewer than two completed boundaries, enable all positively measured sets. Record this calculation, its sampling limits, and the enabled task numbers. This is a deterministic storage heuristic, not a promise that boundary observations captured the peak.
5. **Insufficient or unavailable observations have fixed outcomes.** An unavailable diagnostic leaves storage-remedy tasks disabled and hosted acceptance unverified; still complete consumer attribution, hermetic tests, and documentation. Zero removable bytes or live compiled output exceeding available capacity leaves storage acceptance failed; do not provision larger runners, partition producers, reduce debug information, or request an intermediate ruling. Record that capacity expansion is outside this authorized implementation and finish the independent phases. This follows the specification's explicit prohibition on silently authorizing its capacity options. Neither outcome permits a completed implementation claim.
6. **Publish incrementally only for the archive remedy.** Introduce `scripts/ci/artifacts/produce.cjs` as the hosted coordinator. It invokes the existing `ci-build produce --key` once per sorted planned record, using one unchanged target tree and awaiting completion and publication before the next record. Keep `produce-owner.sh` as the real-Cargo fixture entry point and delegate its hosted behavior to the coordinator through an explicit non-interactive mode; keep its existing local no-upload mode. Do not add a Rust-to-Node callback protocol. Evidence: the Rust key selector already provides the required boundary and the Node artifact transport is already installed by `ci.yml`.
7. **Deletion follows successful archive upload.** Refactor `publish.cjs` into reusable record publication and owner finalization; retain `publish` as the compatibility entry point for existing callers/tests. Retain local manifests, raw statuses, compiler counters, verifier, and a local publication ledger. Delete only the uploaded archive and its record-owned copied-sidecar directory after the awaited archive upload resolves; keep those payloads on upload failure. A failed status upload must not cause re-upload of an immutable archive. The ledger independently records archive and status upload completion; finalization uploads only outstanding artifacts. Preserve individual failures and attempt unrelated records. Never delete the shared target tree. `publish` currently uploads archive before status and already isolates archive-upload exceptions per record.
8. **Preserve measurement meaning during incremental publication.** An early artifact cannot contain a final owner aggregate. Keep each record's compiler-work payload and publish the completed owner aggregate once in a separate diagnostic artifact, derived from retained manifests and `owner-timing.json`. Update `publish.test.cjs` and `ci-build-tests.rs::the_javascript_publisher_reads_a_fixture_generated_from_this_report` to assert the separate final aggregate. Archive/build-status documents retain their current fields and versions; update documentation of the diagnostic payload rather than relabeling a partial aggregate as final.
9. **Check status on every archive consumer.** Add `scripts/ci/build_status.py`, used before `Download this cell's build` in `_package-ci.yml` and `_wsl-ci.yml`. Use `cell_contract.py`'s existing `build_key`, `build_producer`, and `build_artifact` outputs and the exact `build-status-{package}-{producer}-{key}` name. Download status with `continue-on-error` only for that download; the following check must fail on absent, malformed, mismatched, failed, or cancelled status. Failure text includes package, key, producer, stage, and `not run: producer failed`; absent status says `not run: producer outcome unknown` and names the owner. Missing archive after successful status is an artifact-publication failure. No fallback compilation, report fabrication, completion record, or accepted-gap state is added.
10. **Reuse audit attribution without fabricating compilation failures.** `runner_loss.py::synthesize_build_status` already synthesizes build-owner attribution using the plan; `_area-ci.yml` already invokes it before rollup. Keep that path and change its unconditional “no archive was uploaded” assertion to “per-record production/publication outcome unknown”; a missing status does not prove a missing archive. Preserve existing real statuses over synthesized ones. `ci-rollup.rs` already uses a non-success build status to block its consuming cell; update its human-readable reason and drifted historical cutover comments, without inferring compilation failure from an absent status. The area's missing-completion audit and `ci-gate`'s job-result fold remain intact.
11. **Constrain cleanup and portability.** Temporary cleanup owns only `{artifact}-inventory` and explicit producer-created temporary directories; replace the normal-path `remove_dir_all` at `produce_one` with scope cleanup that also runs on errors. Retain scratch through `runtime::observe`. No registry, runner-tool, fixture, or global temporary-directory deletion. Hosted storage sampling is Ubuntu-only; consumer checks and Node orchestration must work on macOS and native Windows as well. Keep Python UTF-8 behavior explicit on Windows. Read the `os` skill's Windows guidance before implementing path containment checks; validate containment using native path components, including symlink escape rejection.

### Wave 1

- [x] **Storage instrumentation**
  - Implement Rule 3 and add samples before/after cache restore, selected schematic guard execution, every record attempt including handled failure, and publication. Hook per-record samples into `run_produce`; use a diagnostic command supplied by the hosted wrapper, absent for ordinary local callers, with arguments passed directly rather than through a shell.
  - Keep samples in `$RUNNER_TEMP/build-diagnostics`; add an always-run upload in `ci.yml::build`. Preserve earlier samples after failure. Track schematic temporary paths from its actual recipe and subprocess environment rather than attributing their bytes to archives.
  - Validate the sampler with tiny files, missing paths, shared filesystems, and a failed probe; no deliberately full disk.

### Wave 2

- [x] **Diagnostic spike** _(hosted diagnostic unavailable in this pipeline; selection made from the S4 local measurement — see `implementation-log.md`, Phase 1)_
  - Run once on the Ubuntu hosted owner with a full-workspace resolved plan, preferably the next already-required nightly. Do not trigger an all-OS/full-test run merely to diagnose storage. Record run, attempt, plan head, build keys, cache state, duration, and every available sample in `implementation-log.md` beside this plan.
  - Apply Rules 4–5 mechanically to select Phase 2 storage tasks. Positive archive, temporary, and stale-cache measurements enable their named tasks; ties and combined remedies follow Rule 4. No measurement enables a larger runner or profile change.
  - Lack of credentials, a missing available hosted run, or a lost runner without adequate samples is the unavailable-evidence outcome. Continue remaining independent work; do not schedule another diagnostic spike or wait for human action.

## Phase 2 — Implement bounded storage lifetimes

### Wave 3

The following tasks have disjoint primary files and may run concurrently after Wave 2. Execute only storage tasks enabled by its recorded result; mark disabled tasks with the measured reason. Consumer attribution always executes.

- [x] **Incremental publication**
  - Implement Rules 6–8 in `produce.cjs`, `publish.cjs`, `produce-owner.sh`, and the build job. Aggregate nonzero child/upload outcomes while attempting later records. Preserve the schematic guard failure as a job failure; do not erase it with a successful finalizer.
  - Keep the always-run final publication step for unfinished status uploads and records never reached; avoid duplicate immutable artifact names. Persist ledger updates atomically after successful uploads and before cleanup. A process lost between remote upload and ledger persistence remains infrastructure failure; do not overwrite a remote artifact to manufacture success.
  - Add hermetic Node tests for upload-before-delete ordering, pending upload retention, archive failure retention, failed status upload, finalizer replay, successful sibling beside failure, record ordering, and retained verifier/manifests. Update the shared-dependency fixture to exercise the new sequential hosted coordinator with a fake upload transport and retain its existing compile-count assertions.

- [x] **Temporary cleanup** _(disabled by the Wave 2 selection: no retained inventory/packaging scratch was measured, 0 B; see `implementation-log.md`, Phase 1)_
  - Implement Rule 11 in `ci-build-archive.rs::produce_one`; make cleanup run after inventory failure and runtime observation failure as well as success, preserving the original error. Remove partial producer-owned packaging outputs only after the failed record is finished and no publication reader exists.
  - Extend `ci-build-archive-tests.rs` with small fake-nextest failures demonstrating scratch release and retained successful payloads; preserve the existing portability/inventory checks.

- [x] **Cache isolation** _(disabled by the Wave 2 selection: 0 restored bytes, no producer cache exists (`gh cache list --key build-owner` is empty); see `implementation-log.md`, Phase 1)_
  - For the stale-cache outcome, disable target-cache restore/save for the Ubuntu producer only; retain current caching for other producers and shared target reuse during the current run. This is narrower and safer than deleting entries guessed stale from Cargo fingerprints. Keep the existing lockfile/toolchain cache identity elsewhere.
  - Record diagnostic restore bytes and the resulting cold-build tradeoff. Update workflow contract tests to pin the Ubuntu exclusion and unchanged current-run sharing. Do not clear Cargo's registry or change `RUSTC_WRAPPER` policy.

- [x] **Consumer attribution**
  - Implement Rules 9–10 in `build_status.py`, both consumer workflows, and rollup reason rendering. Gate native download/check steps on `consumes_archive`; WSL resolves status on the Windows host before guest provisioning.
  - Keep always-run status/report uploads; blocked cells produce an ordinary failure status with no test completion. Preserve runner-loss attribution in `runner_loss.py`, and add regression cases to its existing `test_runner_loss.py` only where changed wording affects it.
  - Add in-process tests of the public status-check result and narrow CLI wiring tests described below. No GitHub network request is needed in these tests.

### Input robustness matrix

The new build-status reader is the only new configuration/document reader. Reuse `schema.py::loads_strict`; do not introduce another JSON parser. Default: every invalid document or load-bearing field produces a located error naming its file and field, exits nonzero, launches no archive/test, and publishes no success or completion.

- Required scalar identity fields: `schema_version` (exact integer 1, excluding booleans), `package`, `key`, `producer`, `artifact`, `result`, and `stage` (nonempty strings). Missing, null, wrong type, and empty values fail. Identity values must match the resolved cell; result accepts only `success`, `failure`, and `cancelled`. Unknown results fail closed; accept nonempty stage strings as diagnostic detail, never to authorize success independently of result.
- `detail` is optional diagnostic text: absence or null means no detail; a string, including empty, is preserved; other types fail. `digest`, timings, consumers, and other fields are not interpreted by this pre-download check; authoritative archive verification still checks archive identity and contents afterward.
- Test scalar shapes once through the shared required-string validator; each wired field gets a valid control and one invalid shape. Test schema version and result enumeration separately. Cover optional detail's exceptions separately. No collection-field matrix applies because this reader makes no decisions from collection fields.
- At the shared strict loader, cover duplicate keys, trailing garbage, invalid JSON, nonfinite constants, and a second document. JSON-only syntax applies. Use a real serialized build-status fixture as the unchanged positive control and mutate one field per case. Assert the public check result and no launch through the workflow adapter.
- Spawn the Python CLI only for the control, malformed JSON, and mismatched key, asserting file/field location and nonzero exit. Add workflow contracts proving status validation precedes archive download and no guest/test setup follows a refused check.
- Review new parsing for `Option`/default conflation, `.get(..., default)`, filtering wrong types, swallowed exceptions, and empty fallbacks. Existing permissive publisher fixtures must not become evidence for a permissive consumer. Do not broaden this fix into rewriting unrelated plan/manifest loaders.

## Phase 3 — Verify contracts and document behavior

### Wave 4

- [x] **Focused verification**
  - Run the canonical root `just test repo-deps test-toolkit` once after Wave 3 integration. Run the registered `artifact-publisher` companion through its existing command, `node --test 'scripts/ci/artifacts/*.test.cjs'`, and register new Python status tests as a `repo-deps` companion using the existing suite registry/metadata pattern.
  - Update `tools/test-toolkit/tests/ci_workflow_contracts.rs` around `every_build_publishes_a_keyed_artifact_and_a_status_for_any_outcome`, compiler-work timing contracts, and archive consumer contracts. Assert no scope/matrix/timeout/gate drift, always-run diagnostics/finalization, and success beside failure. Test behavior through fake transports rather than only source-string assertions.
  - Run `actionlint` on the edited workflows and scoped Clippy for `repo-deps` and `test-toolkit`; run new Python tests through their registered companion command. Do not run `cargo fmt`, `cargo test`, a workspace-wide test gate, or terminal/browser tests for this change.
  - Fix failures in the touched paths and rerun only affected checks. Record commands, outcomes, and material limitations in the implementation log.

- [x] **Current documentation**
  - Update `docs/topics/ci-cd.md` and `.claude/skills/rust-devops/ci-cd.md` with the enabled remedy, publication/cleanup lifetimes, blocked-consumer wording, diagnostic locations, final owner measurement artifact, and unverified hosted acceptance where applicable. Include a compact production → upload → delete flow diagram for incremental publication.
  - Inspect edited symbol docs and comments; remove the stale cutover explanation in rollup when updating attribution. No README expansion is needed because no user-facing command is added. Do not link current topic documentation to this dated fix or move its lifecycle directory.

## Phase 4 — Record hosted acceptance

### Wave 5

- [x] **Hosted acceptance**
  - Inspect one available full-workspace Ubuntu producer execution with the enabled remedy. Prefer an already-required run; this is acceptance of accumulated record storage, not a second diagnostic or a reason to rerun every test. Record plan/run/attempt identity, cache state, duration, required versus published keys, and actual minimum observed free bytes and inodes for both locations. Samples must not be described as instantaneous peaks.
  - Inspect one planned WSL2 consumer from that execution: record successful archive verification, test command start, and actual reports. Record ensuing test failures separately without changing their outcomes or fixing them here.
  - Classify each acceptance requirement as verified, failed, or unverified. Missing run access is unverified; exhausted space, missing required publication, or timeout is failed. Report the exact unresolved requirement and evidence without inventing a passing observation, scheduling repeated full-workspace attempts, or adding an intermediate human gate. An insufficient-capacity result completes the investigation and independent attribution work but leaves the storage objective explicitly unachieved.
  - Finish `implementation-log.md` with enabled/disabled tasks, measured cause, remedy tradeoffs, test results, and acceptance state. Leave the fix in its active directory, ready for the final review cycle; do not commit, push, or mark the specification completed.