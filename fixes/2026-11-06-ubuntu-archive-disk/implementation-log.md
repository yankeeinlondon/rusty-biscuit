---
spec: "/Volumes/coding/rusty-biscuit/fixes/2026-11-06-ubuntu-archive-disk/spec.md"
plan: "fixes/2026-11-06-ubuntu-archive-disk/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
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
decisions_for_review:
    - |-
        Hosted diagnostic replaced by the S4 local measurement. The plan says to
        run one hosted Ubuntu diagnostic and, if it is unavailable, to leave every
        storage remedy disabled. The hosted run is unavailable here: nightlies run
        main, the instrumentation is not merged, and this pipeline may not push.
        I instead applied the plan's ranking rule to the S4 spike, which measured
        the same 66-key plan with CI's own steps: archives and sidecars 6.2 GiB,
        restored cache 0 B (no cache exists), retained scratch 0 B. That enables
        only "Incremental publication"; "Temporary cleanup" and "Cache isolation"
        stay disabled. Alternative: follow the plan literally and disable all
        storage remedies, which ships no storage change this cycle and waits for
        a hosted sample after merge. If the reviewer prefers that, the Phase 2
        incremental-publication work is dropped; nothing else depends on it.
    - |-
        The sampler is scripts/ci/storage_sample.py, not storage-sample.py as the
        plan names it. Every other module in scripts/ci uses underscores, and a
        hyphen would stop the test suite from importing it. Renaming back means
        changing the file, its test import, three workflow lines, the wrapper,
        and the docs.
    - |-
        The producer receives the sampler as `--sample-command '<JSON array>'`.
        That passes an exact argument list with no shell, as the plan asks. A
        repeated `--sample-arg` flag would work equally well. Switching means
        changing the parser, produce-owner.sh, and two parse tests.
    - |-
        Per-record sampling runs once at owner start and once after each record,
        not before and after every record. The "before" of a record is the same
        observation as the "after" of the previous one, and each sample walks the
        whole target tree (about 80 GiB on the hosted owner). Adding a pre-record
        sample is one extra call in run_produce.
    - |-
        Not implemented, flagged for review: S4 measured 6.5 GiB in the schematic
        guard's child target (target/schematic-fresh-output). That output is dead
        after the guard step and is larger than all the archives together.
        Deleting it after the guard would recover about as much as incremental
        publication. The plan's cleanup rule limits deletion to producer-owned
        directories, so I did not add it. If the reviewer wants it, it is a
        one-step workflow change after the guard.
    - |-
        Capacity is probably the real limit, and that decision is unresolved. S4
        puts live compiled output at about 81.5 GiB, against 6.2 GiB of archives.
        If the hosted runner has less than about 82 GiB free, no authorized
        remedy can make the job fit. The spec's open question (larger runner,
        partitioned producers, or less debug information) then needs a human
        decision. Nothing here authorizes any of those.
    - |-
        Coordinator hosted inside a github-script step. The plan said
        produce-owner.sh should hand its hosted behavior to the coordinator. I did
        the reverse: a JavaScript workflow step runs the coordinator, which starts
        produce-owner.sh once per record in an explicit "record" mode. The reason is
        that GitHub gives the artifact upload credentials only to JavaScript
        actions, not to shell steps. The alternative is to copy those credentials
        into the job environment for all later steps, which widens their exposure.
        produce-owner.sh keeps its local mode (one run, no upload), and the
        real-Cargo fixture drives the coordinator's local command line with a fake
        directory upload. Switching to the other direction means exporting the
        token and moving the loop into the shell script.
    - |-
        After a failed upload, the final step retries only what is still missing.
        If a record's archive upload failed, its published status already says
        "failure at upload" and the final step does not retry the archive. If only
        the status upload failed, the final step retries the status and never
        re-uploads the archive, because artifact names cannot be reused. The
        alternative, retrying a failed archive upload in the final step, would need
        the already-published failure status to be replaced, which the artifact
        service does not allow.
    - |-
        Each record's compiler-work file now holds only that record's counts. The
        owner total is uploaded once, by the final step, as a separate artifact named
        compiler-work-owner-{producer}. That artifact name is my choice; the plan
        asked only for "a separate diagnostic artifact". No code reads the old
        per-record owner field; only documentation did, and Phase 3 updates it.
    - |-
        On a Windows owner, the coordinator finds bash by searching PATH in order,
        the same way the existing Rust fixture does. If a runner listed System32
        before Git's bin directory, it would pick the WSL launcher. Hosted Windows
        runners list Git first, and the old `shell: bash` step relied on that too.
        Not tested on Windows from this host.
    - |-
        The repo-deps package does not declare Node as a runner tool, even though
        the shared-dependency fixture now needs node. Its Node publisher suite
        already makes the Ubuntu cell install Node 22, and macOS and Windows images
        ship Node. The WSL2 guest has no Rust toolchain, so this real-Cargo test does
        not run there. If a Node-less environment ever runs the test, it fails with
        an explicit "node not found" message rather than skipping.
    - |-
        The WSL2 build-status steps are not gated on the cell consuming an
        archive. The existing WSL archive download is not gated either, and gating
        only the new check would let such a row skip the check and still reach the
        download. Native cells are gated as the plan says.
    - |-
        The rollup changes its wording but not which cells it blocks. A cell with
        no results reads "not run: producer failed; build ...". A cell that did run
        tests keeps its own result and adds "producer failed; build ..." as context.
        An absent status still blocks nothing in the rollup, which cannot tell a
        missing status apart from a lost consumer report; the "producer outcome
        unknown" wording appears in the consumer's own check instead. Adding that
        note in the rollup would need a new input distinguishing the two cases.
    - |-
        Phase 3 fixed a drifted paragraph in .github/ci/README.md that said the
        artifact-publisher Node suite still needed a suite-registry entry. The
        entry exists in scripts/ci/affected_scope.py, so I took the code as
        correct and rewrote the paragraph. The plan did not ask for this. The
        alternative was to leave it for a separate cleanup; reverting means
        restoring one paragraph and one retired-phrase row in the doc test.
    - |-
        Phase 3 used scoped Clippy (repo-deps, test-toolkit) as the lint gate,
        as the plan's verification task says, instead of the root `just lint`,
        which lints every package area in the monorepo. Running the root recipe
        would cost far more time and check nothing this phase touched.
message_to_agent: |-
    Phase 3 is done: docs and skills now describe incremental publication,
    the ledger, the owner measurement artifact, and the consumer build-status
    check; a doc contract test pins that wording. All gates passed (log,
    Phase 3). For Phase 4: docs/topics/ci-cd.md (subsection "Each archive is
    uploaded, then released, before the next is built") and
    .claude/skills/rust-devops/ci-cd.md (last bullet of "The owner job") say
    hosted confirmation that a full-workspace Ubuntu owner fits its disk is
    "not yet confirmed" / "still outstanding". If Phase 4 obtains a hosted
    result, update both sentences to state it (verified or failed), without
    naming this fix. If hosted evidence is unavailable, leave them. Nothing
    from this work is merged, so a hosted run of the new code needs this
    branch pushed. No BUILD_LINUX/WIN/WSL/MACOS rigs were configured.
---

# Implementation Log for 2026-11-06-ubuntu-archive-disk (4 phases)

## Phase 1

- Started 2026-11-09. Read the plan, the spec, the S4 local disk spike
  (`spikes/S4-archive-disk-per-key.md`), `run_produce`/`produce_one`,
  `produce-owner.sh`, the `ci.yml` build job, and the schematic fresh-output
  guard's recipe and helper (`schematic/gen/tests/common/fresh_output.rs`).

### Wave 1 — storage instrumentation (done)

- **Sampler**: `scripts/ci/storage_sample.py` (Python stdlib). One JSON line per
  boundary, appended to `$RUNNER_TEMP/build-diagnostics/storage.jsonl`:
  `statvfs` free bytes/inodes for the target, output, and temp locations
  (missing path → nearest existing parent, reported as `probed`), device ids
  and pairwise `shared_filesystem`, allocated bytes (`st_blocks`, hard links
  once, symlinks not followed) per category — `target`, `archives` (archive
  files + `*-sidecars`), `inventory` (`*-inventory`), `metadata`, and
  `schematic_guard` (`target/schematic-fresh-output`, `overlaps: ["target"]`)
  — wall and monotonic time, `elapsed_s` from the file's first sample, and the
  cache step's `cache-hit` verbatim. `--inventory` records a two-level target
  inventory; `--restored-since <label>` with `--plan/--producer/--live-path`
  classifies restored entries as `obsolete` (a profile dir no planned identity
  uses), `live`, or `unknown` (Rule 4: never "large therefore stale").
  Unavailable measurements are `{"unavailable": reason}`, never zero. Exit 0
  when a line is written, 1 when none could be, 2 on usage; every caller
  ignores it.
- **Producer hook**: `ci-build produce --sample-command '<json argv>'`
  (`ProduceOptions::sample_command`). `run_produce` runs it once at owner start
  (`--label produce-start`) and after each record's status is written
  (`--label record --record <artifact> --outcome <result>:<stage>`), so success,
  compile failure, and preflight refusal are all sampled. No shell; stdout is
  discarded so `--json` stays one document; a non-zero exit or spawn failure is
  reported on stderr and ignored.
- **Hosted wrapper**: `produce-owner.sh` adds `--sample-command` only when
  `BUILD_DIAGNOSTICS` is set (built with `jq -nc`); local callers and the
  real-Cargo fixture are unchanged.
- **Workflow** (`ci.yml` `build`): the start step exports `BUILD_DIAGNOSTICS`
  on Linux only; six `|| true` sample steps (no `continue-on-error`: existing contracts forbid it on the owner) — before/after
  the cache restore (`id: cache` added to `rust-cache` for `cache-hit`), before
  the guard and after it (`always()` when it ran), before/after publication
  (`always()`) — and an `always()` upload `build-diagnostics-<producer>` (`if-no-files-found: warn`; an upload failure is an owner infrastructure failure like any other upload).
  Runner label, timeout (180), matrix, plan, cache key, and artifact names are
  unchanged. `actionlint` (`~/go/bin/actionlint`) is clean.
- **Schematic temporary paths**, from the recipe and
  `schematic/gen/tests/common/fresh_output.rs`: the child builds share
  `<target>/schematic-fresh-output` (inside `target`, hence the overlap flag),
  and the generated crates are `TempDir`s in `$TMPDIR`/`/tmp` removed per
  test; the `temp` location's free space is sampled for that reason.
- **Docs**: `docs/topics/ci-cd.md` (new "storage samples" paragraph and field
  table), `.github/ci/README.md`, `.claude/skills/rust-devops/ci-cd.md`.

#### Requirement → test mapping

| Requirement | Test |
| ----------- | ---- |
| tiny files counted by allocation; hard link once; symlink not followed | `test_storage_sample.py` `AllocatedBytesTests` (4) |
| missing path → nearest existing parent; missing dir is absent, 0 bytes | `test_a_missing_path_is_measured_at_its_nearest_existing_parent`, `test_a_missing_directory_is_absent_with_zero_bytes`, `test_missing_locations_are_sampled_without_failing` |
| shared filesystem reported | `test_locations_on_one_filesystem_are_reported_shared` |
| failed probe → unavailable, line still written | `test_a_failed_probe_is_reported_unavailable_not_zero`, `test_a_failed_probe_still_writes_its_line` |
| categories distinct, overlap named | `test_categories_stay_apart_and_name_their_overlap` |
| append + elapsed; write failure exit 1; usage exit 2 | `SampleDocumentTests` |
| restored-entry classification (Rule 4) incl. unreadable plan / no earlier inventory | `RestoreClassificationTests` (4) |
| `--sample-command` exact argv, absent by default; malformed argv refused (7 shapes) | `ci-build-tests.rs` `a_sample_command_is_an_exact_argv_and_is_absent_unless_given`, `a_sample_command_that_is_not_a_nonempty_json_argv_is_refused` |
| every record attempt sampled after its status; failing sampler changes no status, exit code, or `--json` output | `ci-build-archive-tests.rs` `every_record_attempt_is_sampled_and_a_failing_sampler_changes_no_outcome` (Unix) |
| sampler that cannot start ignored on every OS | `a_sampler_that_cannot_start_leaves_a_successful_owner_successful` |
| end to end: real `produce-owner.sh` + real sampler | `the_hosted_wrapper_samples_each_record_through_the_shipped_sampler` (Unix) |
| workflow order, Ubuntu-only, `always()`, never decides result, timeout/runner unchanged | `ci_workflow_contracts.rs` `the_ubuntu_build_owner_samples_storage_at_every_boundary_without_deciding_the_result` |

The suite is registered as a `repo-deps` companion (`SUITE_REGISTRY`,
`scripts/Cargo.toml` `companion-suites`, and the ownership table in
`test_affected_scope.py`). Targeted runs: `python3 scripts/ci/suite_runner.py
test_storage_sample.py` (17 passed); `cargo nextest run -p repo-deps --bin
ci-build -E 'test(sample)'` (5 passed); `cargo nextest run -p test-toolkit
--test ci_workflow_contracts -E 'test(samples_storage) | …'` (11 passed);
`python3 scripts/ci/test_affected_scope.py SuiteOwnershipRegistryTests` (13
passed).

### Wave 2 — diagnostic spike (outcome: hosted diagnostic unavailable)

- The nightly runs `main`. Every recent nightly (Nov 6 `42246114963`, Nov 7
  `42383053581`, Nov 8 `42491863852`, Nov 9 `42674352356` in progress) is at
  `a816f1447`, which does not contain this instrumentation, and this pipeline
  may not commit or push. No hosted run can carry the samples until this
  change merges, so the hosted diagnostic is **unavailable** for this phase.
  No run was triggered (Rule 5; spec: no speculative full-scope run).
- Hosted facts that are readable: Nov 8's `build (ubuntu-latest)` ran
  15:12:29–17:20:22 UTC (≈2 h 08 min) and failed with the runner's own
  annotation `System.IO.IOException: No space left on device` (job
  `128771069322`; check-run annotations API). `gh cache list --key
  build-owner` returns nothing: no producer cache exists, so the cache restore
  brings back nothing.
- **Selection.** The spec allows existing evidence to replace the diagnostic
  run, and the S4 spike (2026-11-07) measured the same 66-key Nov 6 plan with
  CI's own steps on `$BUILD_LINUX` (apparent sizes, the ext4 equivalent).
  Rule 4 applied to S4 at its lowest-free-space boundary (end of produce):
  - stale restored target output: **0 B** (no cache exists; `cache-hit` false);
  - completed archives + copied sidecars: **6.2 GiB** (archives 5.63 GiB);
  - abandoned inventory/packaging scratch: **0 B retained** (S4 does not
    separate it; `produce_one` already deletes `{artifact}-inventory` on the
    normal path, and no failed record left any);
  - largest nonzero set: **archives** → Phase 2 **Incremental publication**
    is enabled. Growth between the last two measured boundaries is about
    0.5 GiB per key (6.5 GiB over keys 55–66), smaller than the 6.2 GiB the
    archive set recovers, so no further set is added.
  - **Disabled**: Temporary cleanup (no positively measured retained scratch),
    Cache isolation (0 restored bytes). **Consumer attribution** always runs.
  - Sampling limits: S4's samples were 20 s apart, grouped small keys, and are
    ±0.5 GiB per key; they are not the instantaneous peak and not this
    runner's free space, which no evidence establishes.
- **Capacity warning (unresolved).** S4 puts live compiled output at about
  81.5 GiB of `target/` (66.8 GiB in `deps`, of which 33.2 GiB are rlibs with
  3,243 per-key feature duplicates, plus 6.5 GiB of the guard's
  `schematic-fresh-output`), against 6.2 GiB of archives. If the hosted
  runner's free space is below about 82 GiB, removing archives cannot make the
  job fit, and Rule 5's failed outcome applies: the capacity choice (larger
  runner, partitioned producers, or less debug information) is the spec's
  open question and is not authorized here.

### Gates (Phase 1)

- Scope calculator (`git status … | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [repo-deps, test-toolkit, darkmatter]`, `change_class: package`,
  so the gates were run, not skipped. `darkmatter` is selected because its
  repository-wide document tests read `docs/topics/ci-cd.md` and
  `.claude/skills/rust-devops/ci-cd.md`. The tree also holds uncommitted
  changes from the unrelated publish feature, which the calculator counts too.
- `just test repo-deps`: 512 passed, 1 skipped.
- `just test test-toolkit`: the first run failed 2 existing contracts
  (`only_recovery_and_diagnostic_steps_ignore_errors`,
  `the_owner_job_reaches_the_gate_as_infrastructure_and_owns_no_cell`): the
  build owner may carry no `continue-on-error`. I removed it from the seven
  new steps. Sample steps rely on `|| true`, and the diagnostics upload is an
  ordinary owner upload. Rerun: 525 passed, 4 skipped.
- `python3 scripts/ci/suite_runner.py test_affected_scope.py`: 492 OK (747 s).
  `test_storage_sample.py`: 17 OK.
- Dependents: `cargo nextest run -p darkmatter --test l1 -E
  'test(/shipped_schema_corpus|trigger_spelling_guard|current_root_documentation_contract|current_root_migration_guard|schema_roots|schema_phase_validation/)'`:
  60 passed.
- Lint: `cargo clippy -p repo-deps -p test-toolkit --all-targets
  --all-features -- -D warnings` is clean. Root `just lint` fans out to every
  area, so I stopped it and used the scoped Clippy the plan names.
  `actionlint .github/workflows/ci.yml` is clean.
- No pre-existing failures and no load-sensitive reruns. No OS rigs:
  `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, and `BUILD_MACOS` are unset, so all
  testing ran on macOS. The Unix-only recorder and wrapper tests run on Linux
  CI; the cannot-start test runs everywhere.

## Phase 2

- Started 2026-11-09. Applied the Phase 1 Wave 2 selection: **Incremental
  publication** and **Consumer attribution** run; **Temporary cleanup** (0 B
  retained scratch measured) and **Cache isolation** (0 restored bytes; no
  producer cache exists) are disabled and marked so in the plan.
- The two enabled tasks have disjoint primary files and ran as two parallel
  subagents, each verifying only its own slice; the area gates ran once
  afterward.

### Wave 3 — Incremental publication (enabled)

- New `scripts/ci/artifacts/produce.cjs` coordinator. `produceOwner` sorts the
  owner's records by `(package, key)` in byte order, matching Rust. It runs one
  `produce-owner.sh --key <key>` child per record, with `PRODUCE_MODE=record`,
  all sharing one target tree, and publishes each record before starting the
  next. A nonzero child or failed upload is remembered and later records are
  still attempted. `hostedOptions` builds the shared hosted arguments. A local
  CLI (`--upload-dir`, a fake directory transport) is used by the real-Cargo
  fixture.
- `publish.cjs` is split into `publishRecord` and `finalizeOwner`; `publish`
  remains as an alias of `finalizeOwner`. The publication ledger
  (`<out>/publication-ledger.json`, written atomically by temp file + rename)
  records archive and status uploads separately. The archive and its
  `<artifact>-sidecars` directory are deleted only after the awaited upload
  resolves and the ledger is persisted. Containment is checked first (inside
  `out`, no symlink, no realpath escape). Manifests, statuses, the verifier
  copy, counters, and the target tree are never deleted. A failed status
  upload retries only the status. The finalizer synthesizes statuses for
  records never reached and uploads nothing twice. Each record's
  compiler-work file now holds only `{record}`. The owner aggregate is
  uploaded once as `compiler-work-owner-{producer}`
  (`owner-compiler-work.json`), and only when a record was measured.
- `produce-owner.sh`: `PRODUCE_MODE` unset behaves as before (local, no
  upload, writes `owner-timing.json`). `record` leaves owner timing to the
  coordinator. Any other value exits 2. `CI_BUILD_OUT` overrides the output
  directory. `--sample-command` is still added to every per-key call, so
  per-record storage samples survive, now with a `produce-start` sample per
  record.
- `ci.yml` `build`: "Produce and publish the archives" is a github-script step
  (`id: produce`, not `always()`, so a failed schematic guard still skips
  production and fails the job). It is a JavaScript action because only
  those receive the artifact runtime token. The `always()` "Publish package
  builds and statuses" step now calls `finalizeOwner`. Runner, timeout (180),
  matrix, cache, guard, artifact names, and sample labels are unchanged, and
  no `continue-on-error` was added.
- `.github/ci/README.md`: corrected two paragraphs that this change made false
  (the owner running "once, without `--key`" and the per-artifact owner
  aggregate).

### Wave 3 — Consumer attribution (always runs)

- New `scripts/ci/build_status.py`: `check(path, Cell) -> Verdict` and
  `StatusError(file, field, problem)`. It uses `schema.loads_strict` and reads
  bytes as explicit UTF-8, then applies the plan's field rules. Failure or
  cancelled gives `not run: producer failed` with package, key, producer,
  stage, and detail. Absent gives `not run: producer outcome unknown` and
  names the owner job. The CLI exits 0 only for a valid matching `success`
  status.
- `_package-ci.yml` (gated on `consumes_archive`) and `_wsl-ci.yml` (on the
  Windows host, before any guest provisioning): "Download this cell's build
  status" (`continue-on-error` on that step only; the allowlist in
  `only_recovery_and_diagnostic_steps_ignore_errors` was narrowed to it), then
  "Check this cell's build status", then the existing archive download. The
  always-run status and report uploads are kept. WSL's "Record producer
  status" names a refused check rather than reporting a guest provisioning
  failure.
- `runner_loss.py`: the synthesized detail says the per-record
  production/publication outcome is unknown. Real statuses still win.
- `ci-rollup.rs`: blocked-cell reasons now read `not run: producer failed;
  build …`, or `producer failed; build …` for a cell that did run tests. An
  absent status still never blocks or infers compilation failure. The
  drifted "not cut over / compile in place" comments were rewritten.
- `test_build_status.py` is registered as a `repo-deps` companion (the
  `SUITE_REGISTRY`, `companion-suites`, and `EXPECTED_COMPANION_OWNERS`
  entries).

#### Requirement → test mapping (Phase 2)

| Requirement | Test |
| ----------- | ---- |
| records in Rust byte order; each published before the next child starts | `produce.test.cjs` "records are produced in ci-build's byte order, each published before the next starts", "the comparator matches Rust byte order on the key once packages tie" |
| upload-before-delete; pending upload keeps the archive; ledger written first | "an archive is deleted only after its upload resolves and the ledger records it" |
| successful sibling beside a failed record and a nonzero child | "a successful record beside a failed one and a silent child is still published" |
| never-reached records get synthesized statuses; owner aggregate published once | "records the owner never reached get synthesized statuses, and the owner measurement is published once" |
| failed status upload: archive never re-uploaded, status retried alone | `publish.test.cjs` "a failed status upload is retried alone: the immutable archive is never uploaded twice" |
| finalizer replay uploads nothing; verifier/manifests/statuses retained | "replaying the finalizer after a complete publication uploads nothing" |
| archive upload failure keeps payload, publishes failure, sibling succeeds | "a failed archive upload keeps its payload and publishes an upload failure beside a successful sibling" |
| containment: escape, symlink, parent escape not deleted | "an uploaded payload outside the output directory or behind a symbolic link is not deleted" |
| separate owner aggregate (Rule 8), degenerate measurements still refused | updated aggregate and degenerate-fixture tests; "an owner with no measured record publishes no owner measurement" |
| real Cargo through the coordinator: compile-once kept, archives released locally, statuses published | `ci-build-archive-tests.rs` `one_owner_tree_shares_a_dependency_compile_without_unifying_features` |
| wrapper modes (record / local / unknown refused) | `the_owner_wrapper_leaves_owner_timing_to_the_coordinator_in_record_mode` |
| build-owner workflow: per-key coordinator, `always()` finalizer, guard failure still fails, sample order, no drift | `ci_workflow_contracts.rs` `the_build_owner_expands_one_leg_per_producer`, `every_build_publishes_a_keyed_artifact_and_a_status_for_any_outcome`, `the_ubuntu_build_owner_samples_storage_at_every_boundary_without_deciding_the_result`, `the_build_owner_runs_the_schematic_fresh_output_guard_before_archiving`, `an_archive_miss_never_enters_a_fallback_build` |
| status reader: control, shared required-string validator (missing/null/type/empty), each wired field | `test_build_status.py` `ControlTests` (3), `RequiredStringTests` (2) |
| `schema_version` exact integer 1 | `SchemaVersionTests` |
| result enumeration, producer-failed wording, unknown fails closed, stage never authorizes | `ResultTests` (3) |
| identity must match the cell, per field | `IdentityTests` |
| optional `detail` exceptions | `DetailTests` (3) |
| loader: duplicate key, trailing garbage, invalid JSON, NaN, Infinity, second document, non-object | `LoaderTests` (2) |
| absent status → outcome unknown naming the owner; UTF-8 | `FileTests` (3) |
| CLI wiring: control, malformed JSON, mismatched key (location + exit) | `CliTests` (3) |
| consumer workflows: status download → check → archive download, `continue-on-error` only on the status download, `consumes_archive` gate, no WSL guest step after a refusal | `an_archive_consumer_checks_its_build_status_before_downloading`, `only_recovery_and_diagnostic_steps_ignore_errors` |
| runner-loss wording | `test_runner_loss.py` `test_a_lost_build_owner_is_attributed_to_its_build_record` |
| rollup reasons | five updated `ci-rollup-tests.rs` cases incl. `a_cell_whose_build_published_no_status_is_not_inferred_to_be_blocked` |

### Gates (Phase 2)

- Scope calculator (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [repo-deps, test-toolkit]`, `change_class: package`, so the gates
  ran.
- `just test repo-deps`: 513 passed, 1 skipped. Four pre-existing archive
  fixtures (`a_consumer_reports_its_extraction_apart_from_its_identity_checks`,
  `a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed`,
  `the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach`,
  `the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it`)
  took 5.3–8.6 s. None were changed in this phase. A trailing cargo
  global-cache cleanup warning (permission denied in `~/.cargo/registry`) is
  host noise and did not affect the result.
- `just test test-toolkit`: 526 passed, 4 skipped. Three slow tests are
  pre-existing and not touched by this phase.
- Companion suites: `node --test 'scripts/ci/artifacts/*.test.cjs'` 14 passed;
  `test_build_status.py` 21 OK; `test_runner_loss.py` 60 OK;
  `test_storage_sample.py` OK; full `test_affected_scope.py` 492 OK (708 s).
- Lint: `cargo clippy -p repo-deps -p test-toolkit --all-targets
  --all-features -- -D warnings` clean; `actionlint` on `ci.yml`,
  `_package-ci.yml`, `_wsl-ci.yml` clean.
- Dependents: the calculator selects no other package, and no docs read by
  other packages' tests were changed.
- No OS rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` are unset;
  everything ran on macOS. Hosted behavior (the real artifact client inside
  github-script, per-key production on the Ubuntu runner) is **unverified**
  until a hosted run; Phase 4 owns that.
- Docs left for the Phase 3 "Current documentation" task:
  `docs/topics/ci-cd.md` and `.claude/skills/rust-devops/ci-cd.md` still
  describe end-of-job publication and the per-record owner aggregate. They,
  `.github/ci/README.md:925`, and
  `.claude/skills/rust-testing/ci-execution.md:38` still say `MISSING —
  blocked by build <key>`.

## Phase 3

- Started and finished 2026-11-09, single agent (the work was documentation
  plus one contract-test extension; no subagents were used).

### Wave 4 — Current documentation

- `docs/topics/ci-cd.md` ("One native owner compiles, every tier consumes"):
  new subsection "Each archive is uploaded, then released, before the next is
  built", with a Mermaid flow (produce → upload → ledger → delete → status →
  next record; upload failure keeps the payload; finalizer at the end). It
  documents the github-script coordinator and why it is JavaScript,
  `PRODUCE_MODE=record` / unset / refused, `CI_BUILD_OUT`,
  `<out>/publication-ledger.json` (separate archive/status entries, atomic
  write, no re-upload), the containment-checked deletion, the `always()`
  finalizer (synthesized `failure`/`cancelled` at stage `produce`), the
  enabled remedy and why it was chosen, that hosted confirmation is still
  outstanding, and the separate `compiler-work-owner-<producer>` artifact.
  The storage-sample text became its own "Storage samples" subsection. The
  `MISSING — blocked by build <key>` paragraph was replaced by "A consumer
  checks its build's status before downloading the archive", with a table of
  status → outcome using the exact `not run: producer failed` /
  `not run: producer outcome unknown` wording, the rollup wording, and the
  runner-loss synthesis.
- `.claude/skills/rust-devops/ci-cd.md`: "The owner job" now describes the
  per-record coordinator (Mermaid diagram), wrapper modes, ledger, deletion
  rules, finalizer, the enabled/disabled remedies, and that
  `schematic-fresh-output` is not deleted; the per-key `produce-start`
  sample; the separate owner measurement artifact; the consumer
  status check and the rollup's new reasons.
- `.github/ci/README.md`: replaced `MISSING — blocked by build <key>` with the
  consumer check and rollup wording. **Drift found and fixed**: the paragraph
  after the suite-selection table said the `artifact-publisher` Node suite
  "still needs a `SUITE_REGISTRY` entry"; `affected_scope.py` has had that
  entry. Following the code, the paragraph now says it is registered and
  names `test_build_status.py` as a `repo-deps` companion.
- `.claude/skills/rust-testing/ci-execution.md`: the blocked-cell sentence
  now uses the two `not run:` forms.
- No docs page names this fix. Code comments in the touched Phase 2 files were
  re-checked: no `cut over`, `compile in place`, or "no archive was uploaded"
  text remains in `ci-rollup.rs`, `runner_loss.py`, `build_status.py`,
  `produce-owner.sh`, or `scripts/ci/artifacts/`.
- Regression guard: `ci_workflow_contracts.rs::
  the_ci_documentation_states_the_implemented_behavior` gained six retired
  phrases (`blocked by build` in all four files, `invokes the producer once
  without`, the stale "still needs a SUITE_REGISTRY entry") and eight required
  phrases (`publication-ledger.json`, `PRODUCE_MODE=record`,
  `compiler-work-owner-<producer>`, both `not run:` forms). The committed
  (HEAD) versions of all four files contain `blocked by build` and the topic
  lacks `publication-ledger.json`, so the new assertions fail on the old docs.

### Wave 4 — Focused verification

- Scope calculator (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [repo-deps, test-toolkit, darkmatter]`, `change_class: package`.
  `darkmatter` is selected because its shipped-Markdown schema corpus tests
  scan `docs/topics` and `.claude/skills`. The gates ran.
- `just test repo-deps test-toolkit` (root): 1039 passed, 5 skipped, 9 slow.
  The slow tests (archive fixtures, archive path guard, planner/guard
  contract, kache contracts; 5.2–9.2 s) are pre-existing and untouched by
  this phase.
- Companions: `node --test 'scripts/ci/artifacts/*.test.cjs'` 14 passed
  (the `refusing to delete …` stderr line is the containment test's expected
  output); `suite_runner.py test_build_status.py` 21 OK;
  `test_runner_loss.py` 60 OK; `test_storage_sample.py` 17 OK.
  `test_affected_scope.py` (708 s) was not rerun: neither it nor
  `affected_scope.py` changed in this phase, and it passed in Phase 2.
- Lint: `cargo clippy -p repo-deps -p test-toolkit --all-targets
  --all-features -- -D warnings` clean (the plan's scoped Clippy; the root
  `just lint` orchestrates every area in the monorepo, so it was not run).
  `actionlint` (`~/go/bin/actionlint`) on `ci.yml`, `_package-ci.yml`,
  `_wsl-ci.yml` clean.
- Dependents: `darkmatter`'s two corpus tests
  (`shipped_markdown_inline_schemas_parse_passively`,
  `shipped_markdown_named_type_references_resolve`) passed. No other package
  is selected.
- The contract tests the plan names (`the_build_owner_expands_one_leg_per_producer`,
  `every_build_publishes_a_keyed_artifact_and_a_status_for_any_outcome`,
  the storage-sampling and compiler-work contracts,
  `an_archive_consumer_checks_its_build_status_before_downloading`) were
  written in Phase 2 and pass; behavior through fake transports is covered by
  the Node suite. No further workflow or code change was needed.
- No OS rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` are unset;
  everything ran on macOS.

#### Requirement → test mapping (Phase 3)

| Requirement | Test |
| ----------- | ---- |
| docs no longer say `blocked by build`; owner no longer "once without `--key`"; publisher suite not called unregistered | `the_ci_documentation_states_the_implemented_behavior` (RETIRED rows) |
| docs state the ledger, record mode, owner measurement artifact, and both `not run:` forms | same test (REQUIRED rows) |
| doc Markdown still parses for schema scanning | darkmatter `shipped_schema_corpus` (2 tests) |
