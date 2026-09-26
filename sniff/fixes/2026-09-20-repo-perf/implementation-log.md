---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/fixes/2026-09-20-repo-perf/spec.md"
plan: "sniff/fixes/2026-09-20-repo-perf/plan.md"
implemented_by: "claude/default"
started_phase: "1"
source_files_during_phase_1:
    - sniff/lib/src/filesystem/repo/nested.rs
    - sniff/lib/src/filesystem/repo/mod.rs
    - sniff/lib/benches/cases/repo.rs
    - sniff/lib/examples/work_counts.rs
docs_updated_during_phase_1:
    - sniff/lib/benches/README.md
    - sniff/fixes/2026-09-20-repo-perf/plan.md
    - sniff/fixes/2026-09-20-repo-perf/spec.md
docs_created_during_phase_1:
    - sniff/fixes/2026-09-20-repo-perf/implementation-log.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/spike.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/environment.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/baseline/README.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/baseline/summary-table.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/baseline/work_counts-release.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/baseline/work_counts-debug.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/baseline/raw/
skills_files_updated_during_phase_1:
    - .claude/skills/sniff/performance.md
packages:
    - sniff
source_files_during_phase_2:
    - sniff/lib/src/filesystem/repo/nested.rs
docs_updated_during_phase_2:
    - sniff/fixes/2026-09-20-repo-perf/plan.md
    - sniff/fixes/2026-09-20-repo-perf/implementation-log.md
    - sniff/fixes/2026-09-20-repo-perf/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/sniff/performance.md
source_files_during_phase_3:
    - sniff/lib/src/filesystem/repo/nested.rs
docs_updated_during_phase_3:
    - sniff/fixes/2026-09-20-repo-perf/plan.md
    - sniff/fixes/2026-09-20-repo-perf/implementation-log.md
    - sniff/fixes/2026-09-20-repo-perf/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/sniff/performance.md
source_files_during_phase_4: []
docs_updated_during_phase_4:
    - sniff/fixes/2026-09-20-repo-perf/plan.md
    - sniff/fixes/2026-09-20-repo-perf/implementation-log.md
    - sniff/fixes/2026-09-20-repo-perf/spec.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/environment.md
docs_created_during_phase_4:
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/README.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/summary-table.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/worker-diagnostics.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/binaries.sha256
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/bracket.sh.txt
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/summarize.py.txt
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/raw/
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/probes/
    - sniff/fixes/2026-09-20-repo-perf/evidence/counters/
    - sniff/fixes/2026-09-20-repo-perf/evidence/counters/README.md
skills_files_updated_during_phase_4:
    - .claude/skills/sniff/performance.md
source_files_during_phase_5: []
docs_updated_during_phase_5:
    - sniff/fixes/2026-09-20-repo-perf/plan.md
    - sniff/fixes/2026-09-20-repo-perf/implementation-log.md
    - sniff/fixes/2026-09-20-repo-perf/spec.md
docs_created_during_phase_5:
    - sniff/fixes/2026-09-20-repo-perf/evidence/cross-os/
    - sniff/fixes/2026-09-20-repo-perf/evidence/cross-os/cross-check-nested-tests.txt
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - sniff/fixes/2026-09-20-repo-perf/plan.md
    - sniff/fixes/2026-09-20-repo-perf/implementation-log.md
    - sniff/fixes/2026-09-20-repo-perf/spec.md
docs_created_during_phase_6:
    - sniff/fixes/2026-09-20-repo-perf/results.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/detection-output/
    - sniff/fixes/2026-09-20-repo-perf/evidence/detection-output/README.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/detection-output/ac5_dump.rs.txt
    - sniff/fixes/2026-09-20-repo-perf/evidence/detection-output/detect_repo_structure-corpus.json
    - sniff/fixes/2026-09-20-repo-perf/evidence/detection-output/sha256.txt
skills_files_updated_during_phase_6: []
source_code:
    - sniff/lib/src/filesystem/repo/nested.rs
    - sniff/lib/src/filesystem/repo/mod.rs
    - sniff/lib/benches/cases/repo.rs
    - sniff/lib/examples/work_counts.rs
documentation:
    - sniff/lib/benches/README.md
    - sniff/fixes/2026-09-20-repo-perf/spec.md
    - sniff/fixes/2026-09-20-repo-perf/plan.md
    - sniff/fixes/2026-09-20-repo-perf/implementation-log.md
    - sniff/fixes/2026-09-20-repo-perf/results.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/spike.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/environment.md
    - sniff/fixes/2026-09-20-repo-perf/evidence/baseline/
    - sniff/fixes/2026-09-20-repo-perf/evidence/after/
    - sniff/fixes/2026-09-20-repo-perf/evidence/counters/
    - sniff/fixes/2026-09-20-repo-perf/evidence/cross-os/
    - sniff/fixes/2026-09-20-repo-perf/evidence/detection-output/
completed_phase: 6
implemented: true
---

# Implementation Log for 2026-09-20-repo-perf (6 phases)

## Phase 1

### Grounding verification (2026-09-25)

Every grounding fact in the plan was re-checked against the working tree.
- **Confirmed:**
  - `walk_for_nested_markers` is private at `nested.rs:244`. Its only caller
    is `nested.rs:157` (the `evidence.nested_markers == None` arm). The
    counters are at `nested.rs:245-246`.
  - `is_nested_marker_path` is at `nested.rs:279`, and is also used by
    `system_view.rs:294`.
  - `ManifestIndex::build` is at `manifest_index.rs:112` (the
    `build_parallel` + `WorkerCollector` template).
  - `ignore` is locked at 0.4.25. Its default thread policy is
    `available_parallelism().map_or(1, ..).min(12)` (`walk.rs:1435`).
  - The `bench-internals` pattern is at `services/mod.rs:42`.
  - The five named nested tests exist.
  - The darkmatter boundary tests are at `lazy_roots.rs:569/631/668`.
  - The architecture doc mentions `nested_marker_walks` at lines 218/226.
- **Drift (trivial):** `ignore = "0.4"` is at `sniff/lib/Cargo.toml:36`, not
  `:35`.

### Parity spike

See `evidence/spike.md`. The scaffolding was a temporary `#[ignore]`d test
module in `nested.rs`, run and then deleted; none of it remains.
- Plain root: equal. Symlinked root: **differs at the root entry**. This
  checkout, three runs: equal, 12,889 entries, 116 markers, 12 callback
  threads.
- **Real divergence:** with a symlinked starting root, serial `build()`
  yields the depth-0 root as a non-directory. The current production walk
  therefore registers the *link's parent*, which is outside the repo, as a
  candidate when the link's basename is a marker name. Reproduced with a
  link named `package.json`. The parallel walk does not do this.
- A default ruling is proposed in `spike.md`: skip depth-0 in both the
  parallel walk and the serial reference, and pin it with a test. It is
  handed to Phase 2/3 through the spec's `message_to_agent`. I did not
  judge that it needs human sign-off before Phase 2: it only affects a
  marker-named symlinked root, and the plan's ruling mechanism lets the
  author override in review.
- On tiny fixtures, the default policy ran every callback on one thread.

### Harness (no behavior change)

- `nested.rs`: added `serial_reference_paths` (a frozen copy of the
  pre-change serial collect-all loop, with no counters) and
  `pub mod benchmark` (`production_walk`, `serial_reference_walk`,
  `corpus` → `NestedWalkCorpus`, `CandidateFields`). Both are
  `#[cfg(any(test, feature = "bench-internals"))]`, and the module is
  `#[doc(hidden)]`. `walk_for_nested_markers` is untouched.
- `repo/mod.rs`: `pub use nested::benchmark as nested_benchmark` under the
  same cfg. `nested` stays `pub(crate)`.
- `benches/cases/repo.rs`: the `nested_marker_walk_corpus` group
  (`serial_reference`, `production`, `detect_repo_structure`). It is gated
  on `bench-internals` plus `SNIFF_BENCH_NESTED_CORPUS=<dir>`, uses flat
  sampling (20 samples, 3 s warm-up), and prints corpus facts to stderr
  outside timing. It is not added to the CI bench IDs.
- `examples/work_counts.rs`: an optional `repo_structure_corpus` case, gated
  on `SNIFF_WORK_COUNTS_CORPUS`.
- `benches/README.md`: documents the new group and env var.
- New L1 test:
  `filesystem::repo::nested::tests::serial_reference_walk_matches_the_production_walk`.
  It checks that the reference and production walks both equal
  independently spelled-out candidates, and pins the corpus counts (the
  pruned `node_modules` marker is excluded, and a marker-named directory
  is not evidence). It is compiled by the lib unit-test target, carries
  no tier marker, and `cargo nextest list` shows it selected.
- **Deviation:** the plan says "land harness commit". This session is
  forbidden to commit or stage, so the harness sits uncommitted in the
  working tree. Whatever commit captures Phase 1's files becomes the
  harness commit.
- **Deviation (Ruling 3, `git switch` measurement worktree):** I could not
  create commits, so both brackets were measured from this worktree at
  `HEAD` plus the harness. The corpus is this checkout at its fixed path.
  Corpus facts are printed at every bench registration so drift is
  visible: all four brackets agree at 12,893 entries.

### Environment, corpus facts, baseline

- The fingerprint is in `evidence/environment.md`. The host is busy (load
  average 9.8 → 3.7 during the campaign, from other sessions), and the
  corpus volume is case-sensitive APFS.
- Corpus: 12,893 walked entries, 116 markers, 112 candidates (historical:
  11,290 / 92). `detect_repo_structure(<root>)` records
  `nested_marker_walks = 1`, so the fallback is entered.
- Baseline (warm cache, median of the two bracket medians):
  - walk: 168.6 ms debug, 62.9 ms release (historical 172 / 61);
  - `detect_repo_structure`: 240.1 ms debug, 77.1 ms release;
  - other work: ≈71.5 ms debug, ≈14.6 ms release (historical ≈60 of 232
    in debug);
  - bracket drift: ≤1.1% for the walks and ≤3.9% for
    `detect_repo_structure`.
- Everything is in `evidence/baseline/`: the README with commands, the
  summary table, raw `sample.json` files and logs, and `work_counts`
  snapshots.

### Gates

- `just test` (sniff): **2858 passed, 31 skipped**. The skips are
  pre-existing tier/capability skips; none are from this phase.
- `just lint`: clean.
- `cargo clippy -p sniff --all-targets -- -D warnings`: clean.
- `cargo clippy -p sniff-cli --all-targets -- -D warnings`: clean.
- `cargo clippy -p sniff --features bench-internals --all-targets -D warnings`
  and the same with `remote,bench-internals`: clean.
- **Pre-existing, unrelated:** `cargo clippy -p sniff --features network`
  (without `remote`) fails with `-D warnings` on dead code in
  `credentials.rs` and `filesystem/git/remote_observation.rs`. It also
  fails on the lib alone, and neither file is touched.
- No cross-OS runs were made in this phase. The changes are test- and
  bench-only code with no `cfg(windows)` branches. The cross-OS parity legs
  are Phase 5.
- **Unrelated working-tree changes seen during the phase**, which I left
  alone: `sniff/.ai/plans/2026-02-16.plan-for-repo-remotes-completion.md`
  (deleted), `sniff/fixes/2026-09-25-recent-commits/` (untracked), and
  `prompts/_implement/implement-plan.md` (modified, and already dirty at
  the start).

### Requirement → test mapping (Phase 1 scope)

| Requirement | Test / evidence |
|---|---|
| Harness introduces no behavior change | `serial_reference_walk_matches_the_production_walk`, plus the five retained nested tests and the `detection.rs` counter/reuse tests (all green) |
| Serial reference equals the pre-change walk | same test (independent expected candidates); corpus `production` ≈ `serial_reference` medians |
| Measured request enters the fallback | `evidence/baseline/work_counts-*.md` (`nested_marker_walks = 1`) |
| Spike parity findings | `evidence/spike.md`; promoted assertions are listed there for Phase 3 |

The sniff skill's `performance.md` gained one trap bullet: serial versus
parallel `ignore` root-entry handling for a symlinked root, and tiny trees
running on one thread.

## Phase 2

### Starting state (2026-09-25)

- The Phase 1 harness is committed (`a734cdfc8` and ancestors); the working
  tree was clean at the start. Grounding facts are unchanged:
  `walk_for_nested_markers` still has one caller, and the counters sit where
  the plan says.

### Rulings applied

- **Depth-0 root entry (spike finding 2): adopted as the default.** The
  parallel callback skips `entry.depth() == 0`. For a symlinked root this is
  redundant with the directory filter on the parallel side, but it holds on
  every platform. `serial_reference_paths` got the same skip, and its doc
  records this as its one deliberate difference from the pre-change loop.
  For a non-symlink root, the root is a directory and was already dropped,
  so corpus counts and Phase 1 baselines are unaffected. The author can
  still override this in review.
- **R1 seam:** `walk_for_nested_markers_with_threads(root, Option<usize>)`
  is a private fn. Production `walk_for_nested_markers(root)` delegates with
  `None`, and `Some(n)` maps to `WalkBuilder::threads(n)`. Nothing is public;
  the bench seam still calls `walk_for_nested_markers`.
- The R2 test-only callback counter is left to Phase 3 (Work-group C), as
  the plan schedules.

### Deviation from the template: collector activation order

The plan says to follow `manifest_index.rs` exactly, with `activate()` as the
first statement of the callback. Reading `ignore` 0.4.25
(`WalkParallel::visit`) shows that the first `builder.build()` visitor lives
on the **calling** thread and receives only root errors (`device_num` /
`DirEntryRaw::from_path` failures, e.g. a missing root).
`WorkerCollector::activate()` on the calling thread clears that thread's
`COUNTER_BUFFER`/`STAGE_BUFFER`, and its drop sets `CURRENT_COLLECTOR` to
`None`. So with activate-first, a missing root would:

1. discard the just-recorded `FS_READ_DIRS` / `REPO_NESTED_MARKER_WALKS`
   and any earlier buffered request work;
2. stop the caller from recording anything for the rest of the request.

Mitigation: activate after the `Ok(entry)` match. Errors record no work, so
nothing is lost. Proof:
- Regression test `a_missing_root_keeps_the_callers_counters`.
- With the activate-first variant temporarily swapped in, it **failed**
  (`FS_READ_DIRS` read 0). With the final code it passes.

`ManifestIndex::build` has the same latent pattern. Its only production call
(`detection.rs:722`) runs after nested outcomes exist, so its root exists and
the hazard is not reachable in practice (only if the root vanishes
mid-request). It is out of scope for this surgical fix and left unchanged;
it is noted for the author.

### Implementation (`nested.rs` only)

- `build()` → `build_parallel().run(..)`, with the builder settings
  unchanged.
- Per-visitor `MarkerWorker { shared, collector, local }`, with a `Drop`
  merge into `Arc<Mutex<Vec<PathBuf>>>`: one lock per visitor lifetime, and
  only when its batch is non-empty.
- The callback does these steps in order:
  - `Err` → continue;
  - activate the collector;
  - skip depth 0, entries whose available file type is a directory (never
    `is_file()`), and non-markers (`is_nested_marker_path` runs before any
    allocation);
  - `entry.into_path()` into the local vec.
  There are no metadata reads, cap, or early exit.
- After `run` returns (workers joined, so all visitors have dropped and
  merged), `mem::take` the shared vec and call
  `candidates_from_marker_paths` once.
- The counters stay at the top of the walk body, once per invocation.
- Docs:
  - The function doc now describes the parallel walk, the default worker
    policy, the scheduling-independent result, and the root-entry rule.
  - The ignored-marker and marker-named-directory notes are reworded as
    long-standing behavior relative to the older probe loop.
  - The ambiguous "see the spec's 'Intentional Behavior Change' section"
    link is removed.
  - Inline comments explain the activation order and the join/merge
    invariant.

### Tests added (lib unit-test target; no tier marker, so L1)

| Requirement | Test |
|---|---|
| One logical walk per invocation, with counters at the chokepoint, for the default, 1-worker, and 4-worker configurations; candidates are correct in each | `filesystem::repo::nested::tests::fallback_walk_records_one_logical_walk_per_invocation` |
| A missing root yields no candidates and keeps 1/1 walk counters; caller work recorded before *and* after the walk survives (regression for the activation-order hazard; failed with activate-first) | `filesystem::repo::nested::tests::a_missing_root_keeps_the_callers_counters` |
| A marker-named symlinked root registers no candidate outside the root; production and serial reference both equal independently spelled candidates, keeping the link spelling | `filesystem::repo::nested::tests::a_marker_named_symlinked_root_registers_no_candidate_outside_the_root` (`cfg(unix)`; Windows symlink coverage is Phase 3 R5) |
| Existing behavior retained | the five pre-existing nested tests, `serial_reference_walk_matches_the_production_walk`, and the `detection.rs` tests are all unmodified and green |

### Gates

- Focused: `cargo nextest run -p sniff --lib --features remote -E
  'test(/filesystem::repo::(nested|detection)/)'` gave 59 passed.
- `just test` (sniff): **2861 passed, 31 skipped**. That is Phase 1's 2858
  plus the 3 new tests; the skips are the same pre-existing tier/capability
  skips.
- `just lint`: clean.
- `cargo clippy -D warnings --all-targets` is clean for:
  - `-p sniff`;
  - `-p sniff --features remote,bench-internals`;
  - `-p sniff-cli`.
- Cross-OS (`just cross-check sniff --os <os> nested::tests`):
  - native Windows: 8/8 passed (the `cfg(unix)` symlink test is not
    compiled there);
  - Linux (build-linux): 9/9 passed.
  - WSL2 was not run in this phase. There is no `cfg` difference from
    Linux; the full OS legs are Phase 5.
- Production diff: `nested.rs` only. No dependency changes.

## Phase 3

### Starting state (2026-09-25)

- Phase 2 is committed (`f9af74815`); the working tree was clean. The serial
  reference (`serial_reference_paths` with the depth-0 skip) and the R1 seam
  (`walk_for_nested_markers_with_threads`) already existed, so Work-group A's
  "Serial reference" task is satisfied by them plus the new `assert_parity`
  helper. No new serial-walker code was written.

### Production-file change (test-only)

- The one change outside the test module is the R2 hook in the parallel
  callback: the combined skip condition is split into two steps. The first
  skips depth 0 and directories; then `#[cfg(test)] tests::record_admitted_entry()`
  runs; then the marker check. Non-test builds compile to the same logic as
  Phase 2. The hook runs after `activate()`, as the Phase 2 note required.
- `record_admitted_entry` increments `test.nested_walk.admitted_entries`
  (test-defined, not in `counters.rs`) through the thread-buffered
  `increment_counter`. It also records one `test.nested_walk.worker_thread.<id>`
  per thread through `increment_counter_dynamic`, using a thread-local
  once-flag. That flag is sound because `ignore` 0.4.25 spawns fresh scoped
  threads per `run` (`walk.rs:1410`).
- I checked that no in-crate test asserts an exact counter map (all
  `counts.all()` uses are diagnostic messages), so the extra test counters
  cannot disturb other tests.

### Tests added (`filesystem::repo::nested::tests`, lib unit-test target, L1)

Every parity check goes through `assert_parity(root)`. It compares the
parallel walk under `Some(1)`, `None` (production default), and `Some(4)` with
the serial reference, as complete ordered `(root, matched_standards)` values.
The calling test then asserts independently spelled expected candidates, so a
shared projection bug cannot pass.

| Requirement | Test |
|---|---|
| AC2 matrix: all 12 fixed names, `.sln`/`.slnx`, depths/siblings, Gradle pair → one standard, `package.json` → 3 standards, many markers in one dir, look-alike names, root markers excluded | `every_marker_name_registers_its_standards_at_any_depth` |
| Empty, root-only (every marker at root), and missing roots | `empty_root_only_and_missing_roots_register_no_candidates` |
| Wide tree, 20 repeats × 3 worker configs; expected derived from the placement rule | `wide_tree_parity_holds_across_repeats_and_worker_counts` |
| Hidden dirs eligible; prune (12 names, top-level and nested) authoritative; marker-named directories are not evidence, their descendants are | `prune_hidden_and_marker_named_directories_keep_their_semantics` |
| Non-directory symlinks admitted (including dangling); directory links not followed; no canonicalization (the macOS `/var` temp root stays spelled as given) | `non_directory_symlinks_are_admitted_and_directory_links_are_not_followed` |
| Symlinked starting root, serial vs parallel, under the link spelling (R5, cross-platform; Windows skips with a printed reason only without the privilege) | `a_symlinked_starting_root_walks_like_its_target_under_the_link_spelling` |
| Walk-level platform case rules (fixed names fold only on Windows; `.SLN`/`.SLNX` never match) | `walked_marker_names_follow_the_platform_case_rules` |
| Non-Unicode basenames are not markers; a walk that meets one continues | `non_unicode_basenames_are_not_markers` (the walk half skips with a printed reason where the filesystem refuses the name: macOS APFS) |
| Permission denial is best effort (R6) | `an_unreadable_directory_is_skipped_and_the_walk_continues` (`cfg(unix)`; skips if run privileged. Native Windows is not asserted: that would need ACL editing) |
| Git and non-Git roots: untracked marker found; `.gitignore` exclusion and `!` negation; nested `.gitignore`; `.ignore`; `.git/info/exclude`; controlled global excludes; outside Git only `.ignore` applies | `git_ignore_rules_apply_under_an_isolated_git_configuration`, which spawns the `#[ignore]`d `git_ignore_rules_child` with `HOME`/`USERPROFILE` set to a disposable home and `XDG_CONFIG_HOME` removed, and requires a sentinel line proving the child's assertions ran. Run by hand, the child returns without asserting. |
| Chokepoint counters for an empty root, in all worker configs (populated and missing roots are already pinned by Phase 2 tests) | `an_empty_root_records_one_logical_walk` |
| Supplied evidence (`Some(markers)` and `Some(&[])`) starts no fallback walk; populated evidence reaches the same detector outcomes and seeds as the fallback; `Some(&[])` also reads no dirs | `supplied_evidence_starts_no_fallback_walk` |
| R2 worker propagation: the admitted-entry counter equals the serial reference's entry count (522); distinct worker threads printed as diagnostics, exactly 1 for `Some(1)` | `every_visitor_flushes_its_work_into_the_request` |
| Retained tests | All nine pre-existing `nested.rs` tests and the `detection.rs` tests are unmodified and green |

### Mutation checks (temporarily applied, then reverted)

- Commenting out `worker.collector.activate()` made
  `every_visitor_flushes_its_work_into_the_request` fail (`left: 0, right: 522`).
- Switching the production builder to `git_exclude(false)` made the Git-ignore
  isolation test fail with a parity divergence.

### Worker-count evidence (wide fixture, 522 entries)

| Host | `Some(1)` | default | `Some(4)` |
|---|---:|---:|---:|
| macOS (M4 Max, 16 logical CPUs) | 1 | 10 | 4 |
| Linux (build-linux) | 1 | 12 | 4 |
| native Windows (build-win-native) | 1 | 12 | 4 |

### Gates

- `just test` (sniff): **2874 passed, 32 skipped**. That is Phase 2's 2861
  plus 13 of the 14 new tests. The 14th is the `#[ignore]`d child fixture,
  which accounts for the extra skip.
- `just lint`: clean.
- `cargo clippy -D warnings --all-targets` is clean for:
  - `-p sniff`;
  - `-p sniff --features bench-internals`;
  - `-p sniff --features remote,bench-internals`;
  - `-p sniff-cli`.
  One `cloned_ref_to_slice_refs` lint in a new test was fixed.
- Cross-OS (`just cross-check sniff --os <os> nested::tests`):
  - native Windows: 20/20 passed (the two `cfg(unix)` tests are not compiled
    there). Uncaptured reruns showed no SKIP lines, so the symlink tests
    really created links (Developer Mode is on), and the non-Unicode walk half
    ran on NTFS.
  - Linux: 22/22 passed. The non-Unicode walk half and the mode-000 denial
    both ran; neither skipped.
  - WSL2 was not run (same `cfg` path as Linux); the OS legs are Phase 5.
- `--run-ignored all` over the nested/detection filter: 73/73 passed, after
  making the child inert when started without its parent.

### Skill update

`.claude/skills/sniff/performance.md` gained two bullets:
- how to prove every parallel visitor flushed, and why
  `increment_counter_dynamic` cannot;
- the trap that `ignore`'s global excludes come from the process's
  `HOME`/`USERPROFILE`, and the child-process isolation pattern.

## Phase 4

### Starting state (2026-09-25)

- Phase 3 is committed. `HEAD` is `2f4eb5264`, and the working tree was clean.
- Commits now exist for both sides, so the A/B could follow ruling R3
  properly:
  - baseline side = harness commit `43a08f94e` (serial walk);
  - after side = `2f4eb5264`, the tested code. Its non-test lib code equals
    implementation commit `f9af74815`.
- `git diff --stat 43a08f94e 2f4eb5264 -- sniff/lib Cargo.lock` touches
  `nested.rs` only.

### Method

- **Measurement worktree.** A fixed-path worktree,
  `/Volumes/coding/wt/rusty-biscuit/repo-perf-measure`, was detached at each
  side in turn. For each side it built the `perf` bench (release and dev),
  `work_counts`, and a disposable probe example (untracked, deleted after
  each build). The binaries were copied to `/tmp` and hashed. The worktree
  was removed afterwards.
- **Deviation from R3.** Both sides were measured against **this** checkout
  as the corpus, instead of each side's switched checkout. The corpus
  therefore stayed byte-identical: 12,908 entries in every bracket. Evidence
  was written to `/tmp` during timing and copied in afterwards, so the
  corpus never changed mid-campaign.
- **Timing.**
  - Three alternated Criterion brackets (r1, r2 reversed, r3), four
    invocations each, with 3 s warm-up and 20 flat samples.
  - Two alternated probe rounds covering tiny-tree latency (20 samples ×
    200 iterations, case order rotated) and concurrent detections at 1 and
    14 threads (nextest concurrency). Resource use came from
    `/usr/bin/time -l`.
- **Host load.** It was heavy and uncontrolled, with a 1-minute load average
  of 11–90 from other sessions. Every log records it.
- **Worker diagnostics.** Two temporary `#[ignore]`d tests ran in the
  measurement worktree only, then were reverted: the corpus worker count
  (R2 counter), and a latency sweep by worker count through the R1 seam.
  The fix branch's sources were not touched in this phase.

### Results (medians of bracket medians; full tables in `evidence/after/README.md`)

| Measure | Baseline | After | Change |
|---|---:|---:|---|
| Walk, release (same-process serial reference → parallel) | 66.53 ms | 22.57 ms | 2.95× |
| Walk, debug | 173.83 ms | 53.00 ms | 3.28× |
| `detect_repo_structure(<repo root>)`, release | 79.40 ms | 35.71 ms | 2.22× |
| `detect_repo_structure(<repo root>)`, debug | 243.66 ms | 106.91 ms | 2.28× |
| Tiny tree `detect_repo_structure`, release | 0.49 ms | 3.36 ms | **+2.9 ms (≈6.8×)** |
| Tiny tree `detect_repo_structure`, debug | 0.80 ms | 4.05 ms | **+3.2 ms (≈5×)** |
| Tiny, 1 thread × 400 requests, throughput | ≈1,940 req/s | ≈300 req/s | **−84%** |
| Tiny, 14 threads, throughput | ≈4,160 req/s | ≈4,075 req/s | −2% (noise) |
| Corpus, 14 threads, throughput | ≈53 req/s | ≈52 req/s | −2% (noise) |
| Corpus, 14 threads, p95 latency / CPU / peak RSS | 285–296 ms / 33–35 s / 150–152 MB | 419–420 ms / 38–40 s / 174–196 MB | +42% / +15% / +14–31% |

- **Corpus worker count:** all 12 default workers visited entries on 5 of 5
  runs, in debug and release. The R2 counter matched the serial reference
  exactly (12,908).
- **Counters:** byte-identical for baseline against after, in both
  profiles. They also match the Phase 1 snapshot
  (`evidence/counters/README.md`). The fallback is entered in every measured
  request (`nested_marker_walks = 1`).
- **Attribution (after side):**
  - Release "other work" is unchanged (≈13.6 → ≈13.1 ms).
  - The debug subtraction is noisy (±11 ms), because the after-side isolated
    debug walk ranged from 42 to 65 ms under load.
- **Tiny-tree floor mechanism:** confirmed in the `ignore` 0.4.25 source.
  - Idle workers poll with `thread::sleep(1 ms)` (`walk.rs:1848`), the walk
    ends only once every worker is idle, and every `run` spawns fresh
    threads (`walk.rs:1410`).
  - Floor by worker count on the tiny tree: 1 worker 0.27 ms, 2 workers
    1.58 ms, 4 workers 1.70 ms, 8 workers 2.18 ms, 12 workers 3.01 ms.
  - On the corpus, 4 workers give 23.1 ms against 19.3 ms for 12.

### Decision rule

- **Gain versus variation:** the gain far exceeds run-to-run variation. Even
  the worst after-side bracket beats the best baseline bracket by more than
  40 ms (release) and more than 100 ms (debug). The walk and command
  speedups are reported separately. No compose claim is made.
- **R7 (material regression): triggered.**
  - Tiny-tree single-request latency is about 5–7× worse (+3 ms per call).
  - Sequential small-request throughput is down 84%.
  - Concurrent large requests show worse tail latency and resource use.
- **Scope of the regression:** every structure-only caller hits the fallback
  and pays the floor. Those callers are `RepoRequest::structure()`, the
  `sniff repo` commands, claudine completion and composition, darkmatter's
  compose snapshot, and sniff recent-commits. Only `repo_full` requests
  reuse the shared walk's evidence.
- **Escalation:**
  - Per the spec and R7, this phase stops and escalates: `human_review` is
    set on the spec, with options.
  - The implementation is unchanged. No worker cap, adaptive scheduling, or
    pool was added.
  - Phase 5, which assumes a frozen implementation, should wait for the
    author's decision.

### Gates

- `just test` (sniff): **2874 passed, 32 skipped**. Identical to Phase 3; no
  source changed in this phase.
- `just lint`: clean.
- `cargo clippy -p sniff --features remote,bench-internals --all-targets -- -D warnings`:
  clean.
- No tests were added or renamed: this phase is measurement only. The
  temporary diagnostic tests lived only in the removed measurement worktree.
- No cross-OS runs were made. Timing is a single-host (macOS) campaign by
  design, and the cross-OS parity legs are Phase 5.

### Requirement → evidence mapping

| Plan task | Evidence |
|---|---|
| Isolated walk A/B (alternated, debug + release, ≥3 warmups, 20 samples, worker count) | `evidence/after/README.md`, `summary-table.md`, `raw/*.sample.json` + logs, `worker-diagnostics.md` |
| End-to-end A/B (both commits, same root spelling, fallback verified) | same files; `probes/counters-mode.txt`; `evidence/counters/` |
| Re-attribute composition | `evidence/after/README.md` § Re-attributed composition |
| Compare counters | `evidence/counters/README.md` + four `work_counts-*.md` |
| Probe small and concurrent | `evidence/after/probes/` (raw, probe source, script); README § Tiny tree and concurrent detections |
| Apply the decision rule | README § Decision rule and escalation; spec `human_review_items` |
| Preserve the evidence | `evidence/after/` (commands in `bracket.sh.txt`, `probes/probes.sh.txt`, `summarize.py.txt`; `binaries.sha256`), `evidence/counters/`, `evidence/environment.md` Phase 4 re-check |

### Skill update

`.claude/skills/sniff/performance.md` gained one trap bullet: the fixed
per-walk cost of `ignore`'s `build_parallel()`, with the measured floor by
worker count.

### Re-verification (2026-09-25, later session)

- A later session was asked to carry out Phase 4 and found it already done:
  all plan tasks checked, evidence under `evidence/after/` and
  `evidence/counters/`, and the escalation recorded in the spec.
- No work was repeated. `git diff HEAD -- sniff/lib Cargo.lock` is empty.
- Gates re-run: `just test` 2874 passed, 32 skipped; `just lint` exit 0 with
  no warnings.
- The R7 escalation stands. Phase 5 waits on the author's decision.

## Phase 5

### Starting state and the pending decision (2026-09-25)

- HEAD `b064c9b57` with a clean tree. `git diff HEAD -- sniff/lib Cargo.lock`
  is empty, so Phase 5 validates the Phase 3 implementation exactly as Phase 4
  measured it.
- The Phase 4 R7 escalation (`human_review: true` on the spec) had **no
  recorded author answer** when this phase was requested. Phase 5 ran anyway,
  on explicit instruction, because it is validation only and changes no
  production code. Its evidence holds under option 1 ("accept as
  implemented", the recommendation). Under option 2 or 3 the production code
  changes, and Phase 5 must be rerun on the new code. The escalation stays
  open on the spec.

### OS legs

All four legs ran the same tree, `e13cb10e` (the local HEAD tree). The remote
legs ran through `./scripts/cross-check.sh sniff --os all nested::tests
--no-capture` (cross-check revision `cbddb64fe`, archive mode). The trimmed
output is in `evidence/cross-os/cross-check-nested-tests.txt`.

| Environment | Host | Focused `nested::tests` | Lint |
|---|---|---|---|
| macOS (aarch64) | this host | 22/22 inside full `just test` | `just lint` clean; strict clippy clean |
| Linux | `build-linux` | 22/22 | `just lint` clean (throwaway worktree at `cbddb64fe`, same tree) |
| Native Windows | `build-win-native` | 20/20 | not run (not required by the plan) |
| WSL2 | `build-win` guest | 22/22 | — |

- **macOS:** `just test` 2874 passed, 32 skipped (same as Phases 3 and 4).
  `just lint` exit 0. `cargo clippy -p sniff --all-targets --features
  remote,bench-internals -- -D warnings` and `cargo clippy -p sniff-cli
  --all-targets -- -D warnings` are clean.
- **Linux and WSL2:** `walked_marker_names_follow_the_platform_case_rules`
  passed with the Unix branch in force, so byte-exact fixed marker names and
  case-sensitive `.sln`/`.slnx` held under the real walk. WSL2 ran CI's
  archive mode (built on the Ubuntu target, builder target directory hidden),
  so it follows the Linux path; it is not evidence for native Windows.
- **Native Windows:** the same test passed with the `cfg!(windows)` branch in
  force, so ASCII case-insensitive fixed marker names held.
  - The symlink tests (`a_symlinked_starting_root_walks_like_its_target_under_the_link_spelling`,
    `non_directory_symlinks_are_admitted_and_directory_links_are_not_followed`)
    ran without printing a `SKIP` line, so the host granted symlink creation
    (Developer Mode). No R5 skip happened on this run.
  - Two tests are `#[cfg(unix)]` and do not compile on Windows, which is why
    it runs 20 tests instead of 22. This is by design and recorded here as the
    R5/R6 platform skips:
    - `an_unreadable_directory_is_skipped_and_the_walk_continues` (R6): a
      mode-000 directory is the unprivileged read denial on Unix. Windows would
      need ACL editing.
    - `a_marker_named_symlinked_root_registers_no_candidate_outside_the_root`
      (R5): its fixture is Unix-specific.
- No `SKIP` line appeared on any leg, so no run was privileged or on a
  filesystem that refused non-Unicode names.

### Evidence bookkeeping

- Behavioral runtime evidence (not cross-compilation) for the focused set now
  exists for `{sniff, macos-latest, L1}` (full suite),
  `{sniff, ubuntu-latest, L1}`, `{sniff, windows-latest, L1}`, and
  `{sniff, wsl2-ubuntu, L1}`, in all three cases focused only.
- **No CI receipt was published.** `cross-check` refuses to publish a filtered
  run ("a filtered run does not cover the cell it would be published as"), and
  this phase did not run the full remote L1 suites. Those cells stay
  `execute ci pending` in `just ci-local --plan`, which is correct: the focused
  runs are this fix's parity evidence, not a substitute for the package cells.
- CI scope is unchanged by this fix. `git diff --name-only
  origin/main...HEAD -- .github scripts/ci sniff/**/Cargo.toml .config` is
  empty. The sniff cells in `just ci-local --plan` are the package's standard
  tier declarations. No matrix cell or timing gate was added.

### Darkmatter regression

`just test` in `darkmatter/` was run with `--no-fail-fast` to get complete
counts, because the default run cancelled about 2,100 tests after the first
failure.

- **8498 run: 8496 passed (4 slow), 2 failed, 12 skipped.**
- The three observation-boundary tests passed:
  - `the_observation_is_fixed_at_request_creation`
  - `a_child_reads_the_observation_fixed_at_request_creation`
  - `an_older_constructor_request_is_fixed_at_the_root_entry_not_by_the_child`
- **The 2 failures are not caused by this fix and existed before this phase:**
  - `feature_review_incident::the_shipped_feature_review_composes_and_names_its_spec`
    and `feature_review_incident::the_typo_is_one_identifier_under_the_current_grammar`
    both fail with `File not found: ../_writing-clearly.md`.
  - Commit `6c682a7fd` on this branch (a prompts-only change) added
    `::file ../_writing-clearly.md` to `prompts/_reviews/feature-review.md`.
    The test fixture copies only that prompt, so the include cannot resolve.
    No sniff code is involved.
  - It is already fixed on `origin/fix/wt-ux` by `fe209ae0f`
    ("test(darkmatter): stage _writing-clearly.md for the feature-review
    incident tests"). It was not cherry-picked here, to keep this fix's diff
    confined to sniff.
  - Darkmatter L1 is in this branch's CI scope, so those two tests will be red
    in CI until `fe209ae0f` (or an equivalent) reaches this branch.
- The boundary tests were not weakened, and observation timing is untouched.

### Requirement → test mapping

No tests were added or renamed in this phase. The phase re-ran existing tests
on each OS.

| Requirement | Tests / gate | Result |
|---|---|---|
| AC6 macOS: `just test` + `just lint` | sniff `just test`, `just lint`, strict clippy | 2874 passed / 32 skipped; clean |
| AC6 Linux runtime parity | 22 `nested::tests` on `build-linux` + `just lint` | 22/22; clean |
| AC6 native Windows runtime parity, case-insensitive markers | 20 `nested::tests` on `build-win-native` | 20/20; 2 Unix-only tests by design |
| AC6 WSL2 runtime parity | 22 `nested::tests` in archive mode on `build-win` | 22/22 |
| AC6 no new CI cells or timing gates | `just ci-local --plan`; CI-config diff | unchanged |
| AC7 darkmatter suite + 3 boundary tests | darkmatter `just test --no-fail-fast` | 8496/8498; boundary 3/3; 2 unrelated failures |

### Skill update

None. No architecture or workflow changed. The cross-check facts used here
(filtered runs publish no receipt; use plain substring filters) are already in
the `os` skill.

## Phase 6

### Starting state and the pending decision (2026-09-25)

- HEAD `9d2d39c6d` with a clean tree. `git diff HEAD -- sniff/lib Cargo.lock`
  is empty, so Phase 6 closes out the implementation that Phases 3–5 tested,
  measured, and validated.
- The Phase 4 R7 escalation still has **no recorded author answer**. Phase 6
  changes no production or test code, so it proceeded. `results.md` reports
  the verdict as "pending the author's decision", not "accepted", as the
  Phase 5 message required. `human_review` stays `true` on the spec.

### Consolidated results

- Wrote `results.md`. It covers:
  - each acceptance criterion (AC1–AC7) mapped to its tests or evidence file;
  - the performance table (median of bracket medians, bracket-median range,
    and sample min–max, cross-checked against `evidence/after/summary-table.md`);
  - the counter comparison;
  - the five deviations from the plan;
  - remaining risks.

### AC5 gap closed: full detection output on the checkout

- Earlier phases recorded candidate parity on the checkout (spike), identical
  counters, and fixture-level outcome parity. None recorded a before/after
  comparison of the complete `detect_repo_structure` output on this checkout,
  which AC5 asks for.
- **Method.**
  - A temporary detached worktree built a disposable, untracked release
    example at `43a08f94e` (baseline) and at `9d2d39c6d` (after).
  - The example prints `RepoInfo` as JSON under a fresh `PerformanceCollector`.
  - Both sides read this checkout with the Phase 4 root spelling, alternating
    three times.
  - The worktree was removed afterwards; nothing was added to the fix branch's
    sources.
- **Result: byte-identical on every run** (one SHA-256 across 7 outputs):
  77 packages, 2 standards, 2 layers, and `nested_marker_walks = 1` on every
  run.
- Evidence is in `evidence/detection-output/` (README, probe source, output
  JSON, hashes).

### Drift sweep

- `nested.rs` module, function, and inline docs match the code:
  - builder settings;
  - default worker policy (available parallelism capped at 12);
  - root-entry rule;
  - activation order;
  - join/merge invariant;
  - the serial reference's documented single difference.
  No edits were needed.
- `sniff/docs/sniff-library-architecture.md` (counter still 1 per
  structure-only request), `sniff/lib/benches/README.md`, the
  `benches/cases/repo.rs` and `examples/work_counts.rs` docs, and the
  `tests/fixtures.rs` root-marker comment are all accurate.
- **Dependencies:** the fix's commits (`43a08f94e`, `f9af74815`, `2d886cd98`)
  touch no `Cargo.lock` or `Cargo.toml`, so `docs/dependencies.md` is
  unaffected. The branch-wide `Cargo.lock` and `docs/dependencies.md` diff
  against `main` comes from other merged work on this branch.
- **Cross-area drift, not edited:**
  `claudine/features/2026-08-01-faster-compose/plan.md:125` still calls the
  walk a "serial `WalkBuilder`". It sits in a dated source review of another
  area's active plan. It is recorded in `results.md` for that plan's next
  revision.
- **Sniff skill:** not changed. The architecture and workflow did not change,
  and `performance.md` already carries the parallel-walk facts from
  Phases 1–4.

### Final gates

- `just test` (sniff): **2874 passed, 32 skipped**, identical to Phases 3–5.
- `just lint` (sniff): exit 0, no warnings.
- **Production diff is confined as planned:**
  - `nested.rs`;
  - the `cfg(any(test, feature = "bench-internals"))` `doc(hidden)`
    re-export in `repo/mod.rs`;
  - the env-gated bench row in `benches/cases/repo.rs`;
  - the `work_counts` example.
- **Graph change analysis:** GitNexus `detect_changes` gave no usable signal
  for this fix:
  - the compare range included about 4,500 files of merged branch history;
  - none of this fix's `nested.rs` symbols appeared, so the index looks stale
    for them.
  Following the repo guidance for an unresolved result, a text search
  confirmed the blast radius instead: `walk_for_nested_markers`,
  `serial_reference_paths`, and `nested_benchmark` are referenced only from
  `nested.rs`, `repo/mod.rs`, and `benches/cases/repo.rs`. No caller outside
  sniff exists.

### Requirement → test mapping

This phase added and renamed no tests; it re-ran existing gates and added one
evidence comparison.

| Requirement | Test / gate / evidence | Result |
|---|---|---|
| Final `just test` + `just lint` in `sniff/` | both recipes | 2874 passed / 32 skipped; lint clean |
| AC5 complete output on the checkout | `evidence/detection-output/` | byte-identical before/after |
| Production diff confined | per-commit `git show --stat` plus text caller census | confined |
| Results consistent with evidence | `results.md` numbers checked against `evidence/after/summary-table.md` and the log | consistent (two transcription errors fixed before hand-off) |

### Hand-off

The implementation is complete and ready for review. The one open item is the
author's R7 decision (spec `human_review_items`). The fix directory holds the
spec, plan, log, `results.md`, and `evidence/`:
- `baseline/`, `after/`, and `counters/` for measurements and counters;
- `cross-os/` for the four OS legs;
- `detection-output/` for the AC5 checkout comparison;
- `spike.md` and `environment.md`.

The fix directory stays where it is; moving it is the author's action.
