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
packages:
    - repo-deps
    - test-toolkit
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
message_to_agent: |-
    Phase 1 is done. The hosted diagnostic was unavailable because the
    instrumentation is not merged and nightlies run main. I applied Rule 4 to the
    S4 spike's local measurement instead (see the log's Wave 2 section and
    decisions_for_review). Phase 2 selection: ENABLE "Incremental publication";
    DISABLE "Temporary cleanup" (no retained scratch was measured) and "Cache
    isolation" (0 restored bytes; `gh cache list --key build-owner` is empty).
    Mark the disabled tasks with those reasons. "Consumer attribution" always
    runs. The new sampler is scripts/ci/storage_sample.py (underscore). The
    producer's hook is ProduceOptions::sample_command / `--sample-command`, and
    run_produce samples once at start and once after each record's status is
    written. Keep that hook working when produce.cjs drives `ci-build produce
    --key` once per record: pass the same --sample-command through, or the
    per-record samples disappear. produce-owner.sh builds that argument only
    when BUILD_DIAGNOSTICS is set. The working tree also has uncommitted changes
    from the unrelated publish feature (distribution.py, tap.py,
    distribute-binaries.yml, and parts of ci_workflow_contracts.rs); they are
    not part of this fix. No BUILD_LINUX/WIN/WSL/MACOS rigs were configured.
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
