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
