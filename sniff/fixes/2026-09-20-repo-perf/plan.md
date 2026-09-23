---
total_phases: 6
created: 2026-09-20
phase: 1
agent: "opencode/zai-coding-plan/glm-5.3"
yolo: "true"
fix: 2026-09-20-repo-perf
spec: sniff/fixes/2026-09-20-repo-perf/spec.md
packages:
    - sniff
---

# Parallelize the nested-marker walk — implementation plan

Implements `sniff/fixes/2026-09-20-repo-perf/spec.md`: replace the serial
fallback walker in `walk_for_nested_markers` with `ignore`'s `build_parallel()`
while preserving candidates, ordering, evidence reuse, work accounting, and
cross-platform behavior.

## Work Summary and Success Definition

### Summary of required work

Production change is confined to `sniff/lib/src/filesystem/repo/nested.rs`
(plus a measurement-only exposure seam):

- Switch `walk_for_nested_markers` from `WalkBuilder::build()` (serial,
  collect-all-paths) to `build_parallel()`, preserving every builder setting
  (`hidden(false)`, `git_ignore(true)`, `git_global(true)`, `git_exclude(true)`,
  the `should_skip_directory_name` `filter_entry` prune, and all remaining
  defaults including `.ignore`/parent-ignore handling and symlink behavior).
- In each visitor callback: continue past walk errors, keep the existing
  non-directory condition (never `is_file()`), apply `is_nested_marker_path`
  **before** allocating an owned `PathBuf`, and append to a per-worker local
  `Vec<PathBuf>` merged once on visitor drop into a shared
  `Arc<Mutex<Vec<_>>>` — the `ManifestIndex::build` worker pattern
  (`manifest_index.rs:112-211`), not a per-entry mutex.
- Propagate `performance::WorkerCollector::inherit()` per visitor, activate in
  the first callback, drop on the executing thread; the root/error-only visitor
  must also finish cleanly. Join all workers and merges before calling
  `candidates_from_marker_paths` exactly once.
- Keep `FS_READ_DIRS` and `REPO_NESTED_MARKER_WALKS` at the caller chokepoint,
  once per invocation (including empty/missing root). No new counters.
- Locked `ignore` default worker policy only (no public knob, no pool).

Test work: a test-only serial reference walker, a fixture-matrix parity and
semantics suite, counter/worker-propagation assertions, and retention of the
existing nested-marker tests.

Evidence work: before/after timing (isolated walk and public
`detect_repo_structure`, debug and release) with alternation, a separate work
counter comparison, tiny-tree and concurrent-detection probes, and focused
parity runs on macOS, Linux, native Windows, and WSL2 through existing
workflows. `darkmatter/` runs as a downstream regression gate.

### What successful completion looks like

- Parity suite green: for every fixture, the parallel walk and the test-only
  serial reference produce equal ordered `(root, matched_standards)` candidate
  lists, and independently expected candidates are asserted so a shared
  projection bug cannot make both sides pass.
- Semantics preserved and pinned by tests: root-marker exclusion, Git
  ignore/negation/`.ignore`/exclude-file behavior, hidden/pruned/marker-named
  directories, missing roots, platform marker-name case rules, non-directory
  symlink admission, no canonicalization of collected paths.
- Counters: direct fallback invocation under a fresh collector reports exactly
  one `FS_READ_DIRS` and one `REPO_NESTED_MARKER_WALKS`; supplied evidence
  (including `Some(&[])`) reports zero fallback walks; worker-thread work is
  visible in the collector (no disappearing counts).
- Performance evidence recorded with full provenance: the isolated-walk gain
  exceeds observed run-to-run variation on this checkout; no unreported
  material regression on a tiny tree or concurrent detections (any such
  regression triggers the spec's escalation rule, not silent scope widening).
- `just test` and `just lint` pass in `sniff/`; focused parity tests exercised
  on macOS, Linux, native Windows, and WSL2 (cross-compilation alone is not
  runtime parity); `just test` passes in `darkmatter/` including the three
  observation-boundary tests; no CI matrix cells or timing gates added.
- Docs and comments pass a drift review; the fix directory contains the raw
  evidence bundle and a results summary. The author (never the agent) moves
  the fix to `_completed`.

### Grounding facts (verified on this branch, 2026-09-20)

Checked against the working tree, not assumed. A phase that contradicts one of
them should stop and re-derive rather than proceed.

- `walk_for_nested_markers` is a private fn at
  `sniff/lib/src/filesystem/repo/nested.rs:244`; its only caller is
  `discover_nested_workspace_outcomes` (`nested.rs:157`) when
  `evidence.nested_markers` is `None`. Counter increments sit at
  `nested.rs:245-246`, before the walk.
- `ManifestIndex::build` (`sniff/lib/src/filesystem/repo/manifest_index.rs:112`)
  is the in-repo template for `build_parallel` + per-worker local vec +
  merge-on-`Drop` + `WorkerCollector::inherit()/activate()` — proven in
  production, so no new pattern needs to be invented.
- `ignore` is locked at exactly **0.4.25** in the workspace `Cargo.lock` and is
  already a direct dependency (`sniff/lib/Cargo.toml:35`). No dependency or
  feature change is needed. The spec's "available parallelism capped at 12,
  falling back to 1" default-worker-policy statement matches this version.
- The `nested` module is `pub(crate)`; the established bench-exposure pattern
  is `#[cfg(any(test, feature = "bench-internals"))]` (cf.
  `sniff/lib/src/services/mod.rs:42`). The Criterion harness lives at
  `sniff/lib/benches/` (`cases/repo.rs`, env-gated rows like
  `SNIFF_BENCH_DEEP_REPO`) and `sniff/lib/examples/work_counts.rs` produces
  scoped work-counter snapshots via public APIs.
- `is_nested_marker_path` (`nested.rs:279`) is the exact predicate the shared
  observation walk records with (`system_view.rs:294`); reusing it in the
  parallel callback keeps the fallback and evidence paths byte-identical by
  construction. There are 12 fixed marker names plus the `.sln`/`.slnx`
  suffixes; `package.json` maps to three standards;
  `settings.gradle`/`settings.gradle.kts` both map to Gradle.
- Existing tests to retain: `root_marker_does_not_register_a_candidate`,
  `supplied_evidence_and_the_fallback_walk_agree`,
  `root_marker_from_supplied_evidence_registers_no_candidate`,
  `nested_marker_predicate_matches_fixed_names_and_solution_suffixes`,
  `marker_name_matches_is_exact_on_unix_and_case_insensitive_on_windows`
  (`nested.rs` test module), plus the shared-observation counter/reuse and
  gitignore/prune/inventory-cap tests in `detection.rs`.
- The darkmatter boundary tests are at
  `darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs:569,631,668`.
- `sniff/docs/sniff-library-architecture.md` mentions
  `nested_marker_walks` — include it in the closure drift sweep.

### Guardrails (explicitly out of scope)

- No public API, CLI, serialized-output, request-tier, or observation-timing
  change; callers and the shared observation walker are untouched.
- `RepoEvidence::nested_markers` semantics unchanged (`Some(markers)` —
  including `Some(&[])` — reuses evidence, starts no walk).
- No new counters, no inventory cap, no early exit or truncation, no
  canonicalization, no following of nested directory links.
- No Git-index fast path, no `git ls-files` oracle, no adaptive/shared-pool
  scheduling (escalate with evidence instead).
- No CI matrix cells, timing gates, or compose-suite speedup claims.

## Phase 1 — Rulings, grounding verification, harness, and baseline

### Necessary Rules

Rulings are implementation-mechanism decisions the spec leaves open. Each has
a default the implementation proceeds on; the author may override before or
during review.

1. **Test-only worker-count injection.** The one-worker/multi-worker exercise
   uses a crate-private seam — e.g.
   `walk_for_nested_markers_with_threads(root, threads: Option<usize>)` with
   the production `walk_for_nested_markers(root)` delegating with `None`
   (ignore's locked default). `#[cfg(test)]`-gated or private-fn visibility
   only; no public option, no builder knob.
2. **Worker-propagation verification mechanism.** A counter name defined in
   test code (not added to `performance/counters.rs`), incremented through
   the normal `performance::increment_counter` path inside the callback
   behind `#[cfg(test)]`, with the expected total derived from the fixture's
   entry count (equal to the serial reference's entry count). A fresh-collector
   reading equal to that total proves every visitor's `WorkerCollector`
   flushed; a lower reading proves a lost flush. No production per-entry
   counter is added; an absent counter still means zero.
3. **Baseline comparability.** The official "before" for the isolated walk is
   the same-process test-only serial reference (it retains the pre-change
   builder settings), measured in the same build/profile/host as the parallel
   walk, alternated within the harness. The official "before/after" for
   `detect_repo_structure` is a fixed-path measurement worktree alternating
   `git switch` between two commits (see R4), so root spelling and checkout
   contents stay identical. Historical 172/61/30/18 ms figures are context,
   never gates.
4. **Harness-first commit.** Measurement scaffolding (bench-internals
   exposure of the walk seam, corpus bench row, counter-snapshot wiring)
   lands first as a **no-behavior-change commit**. The baseline side is
   that harness commit; the after side is the same harness plus the
   implementation, so both measurement sides share one harness. The
   corpus/checkout benchmark row is env-gated (like `SNIFF_BENCH_DEEP_REPO`)
   manual evidence — never a portable CI assertion or timing gate.
5. **Symlinked starting root.** Include in the serial/parallel comparison on
   platforms where creating the symlink needs no elevation (macOS, Linux,
   WSL2, and native Windows where developer mode grants it). Where elevated
   privilege would be required, record an explicit skip with rationale in the
   evidence; never fail silently and never escalate privileges.
6. **Permission-error tests.** Assert best-effort continue only where an
   unprivileged read denial is constructible on the target platform;
   otherwise record the privilege-dependent skip. `chmod`-based denial is not
   assumed identical on macOS/Linux/Windows.
7. **Regression escalation.** If tiny-tree latency or concurrent-detection
   throughput regresses materially versus baseline, stop and bring the
   evidence back for review (spec "Alternatives and open questions"). Do not
   widen scope to a global pool or adaptive scheduler inside this fix.

### Spikes

- **Parallel-walker parity probe** (time-boxed, disposable): before the full
  rewrite, a scratch test or temporary branch verifies on (a) a small fixture
  and (b) this monorepo checkout that a `build_parallel` walk with the exact
  preserved settings yields the same entry set as the serial walk — including
  a symlinked starting root and the `filter_entry` prune — and that worker
  callbacks on this host actually run on multiple threads under the default
  policy. Expected outcome: either confirmation (proceed) or a concrete
  parity divergence (e.g. root-entry handling) that Phase 2 must handle
  deliberately. Promote the probe's assertions into the Phase 3 suite; discard
  the scaffolding.

Tasks:

- [ ] **Verify grounding**
    - Re-confirm every grounding fact above on the working branch (ignore
      lockfile version, direct dependency, caller census, counter sites,
      existing tests, darkmatter test locations) and note any drift in the
      fix's log before proceeding.
- [ ] **Run parity probe**
    - Execute the time-boxed spike; record findings (entry-set equality,
      observed worker count on the corpus, symlinked-root behavior) in
      `evidence/spike.md`; promote or discard its assertions per the outcome.
- [ ] **Land harness commit**
    - Add the no-behavior-change measurement seam:
      `#[cfg(any(test, feature = "bench-internals"))]` exposure of the walk,
      an env-gated corpus bench row in `sniff/lib/benches/cases/repo.rs`
      (fixture creation and compilation outside timed regions), and any
      `work_counts` extension needed for fallback-path counter snapshots.
    - Verify `just lint` and the existing nested/detection tests stay green;
      this commit must not alter `walk_for_nested_markers` behavior.
- [ ] **Record environment**
    - Capture the performance fingerprint once into
      `evidence/environment.md`: host, OS, cores, available parallelism,
      `rustc -vV`, commit SHAs (baseline and after sides) plus uncommitted
      state, lockfile state, feature set, build profiles, ignore
      configuration, cache treatment (warmed, labeled as such), test-runner
      concurrency, and the worker policy in force.
- [ ] **Capture corpus facts**
    - On this checkout, re-derive the historical counts: total walked paths,
      marker-file count (92 / 11,290 are the historical figures), and the
      root spelling used; verify the measured request actually enters the
      fallback (fresh-collector `REPO_NESTED_MARKER_WALKS ≥ 1`, no supplied
      evidence); store under `evidence/baseline/`.
- [ ] **Capture baseline tranche**
    - On the fixed-path worktree at the harness commit, measure the isolated
      serial walk and public `detect_repo_structure(<repo root>)` in debug
      and release: ≥3 warmups, ≥20 measured samples per case, median and
      range; snapshot work counters alongside
      (`work_counts` example or equivalent); record the
      `detect_repo_structure` time attribution breakdown the spec asks to
      remeasure (~60 ms of 232 ms debug is historical). Preserve exact
      commands and raw samples under `evidence/baseline/`.
    - Measurement runs are serialized on one host: no concurrent builds,
      tests, or sibling measurements during timed regions.

**Validation checkpoint 1** — grounding verified; spike outcome recorded;
harness commit landed with no behavior change (lint + existing tests green);
environment, corpus facts, and the baseline tranche (commands + raw samples +
counter snapshots) stored under `sniff/fixes/2026-09-20-repo-perf/evidence/`.

## Phase 2 — Parallel walker implementation

Depends on Phase 1 (harness commit and spike findings). Production change,
kept surgical.

- [ ] **Rewrite the walk**
    - Replace `build()` with `build_parallel()` in `walk_for_nested_markers`
      preserving: `hidden(false)`, `git_ignore(true)`, `git_global(true)`,
      `git_exclude(true)`, the existing `filter_entry` directory prune, and
      every remaining default (`.ignore`, parent ignores, Git-repository
      requirement for Git rules, symlink traversal defaults).
    - Visitor callback: on `Err` return `WalkState::Continue` (best-effort,
      same as the serial `.filter_map(ok)`); skip entries whose available
      file type is a directory using the existing
      `!entry.file_type().is_some_and(|ft| ft.is_dir())` condition — never
      `is_file()`; apply `is_nested_marker_path(entry.path())` before
      `to_path_buf()`; push to the per-worker local vec. No per-entry
      metadata or content reads, no cap, no early exit.
    - Per-worker state struct with `Drop` merge into
      `Arc<Mutex<Vec<PathBuf>>>` (one lock acquisition per visitor lifetime,
      zero during visitation), modeled on `ManifestWorker`
      (`manifest_index.rs:123-140`).
    - Complexity: the visitor that only sees the root entry or an error must
      still drop cleanly (flush collector, merge its possibly-empty batch) —
      the `Drop`-based pattern provides this; keep the struct owned by the
      boxed closure so it drops on the executing thread.
- [ ] **Propagate collectors**
    - Follow `manifest_index.rs` exactly: `WorkerCollector::inherit()` when
      building each visitor, `activate()` as the first statement of the
      callback, flush via drop on the worker thread. After `run` returns
      (all workers joined, all merges finished), lock the shared vec once,
      drain, and call `candidates_from_marker_paths` exactly once.
    - Keep `FS_READ_DIRS` + `REPO_NESTED_MARKER_WALKS` increments exactly
      where they are (before the walk), including for empty and missing
      roots; add no counters.
- [ ] **Add worker-count seam**
    - Implement the R1 private seam (`Option<usize>` threads parameter
      defaulting to `None` = locked ignore default). Production path passes
      `None`; nothing public is exposed and no tuning knob is documented.
- [ ] **Drift-proof the docs**
    - Update the module/function docs for the parallel walk: per-worker
      buffers, merge-once, collector propagation, default worker policy.
      Fix the ambiguous "see the spec's 'Intentional Behavior Change' section"
      link; keep the ignored-marker and marker-named-directory deltas
      documented as established behavior relative to the older probe loop —
      not as changes introduced by this fix (spec compatibility section).
    - Per AGENTS.md comment discipline: no HOW-narration; keep the
      contract/invariant comments that earn their length.
- [ ] **Smoke the result**
    - Existing `nested.rs` and `detection.rs` tests pass unchanged;
      `performance::testing::measure` shows exactly one
      `FS_READ_DIRS` + one `REPO_NESTED_MARKER_WALKS` for a direct fallback
      invocation; `just lint` clean.

**Validation checkpoint 2** — production diff confined to `nested.rs` (+ the
Phase 1 seam); all pre-existing sniff tests green with no test modifications;
counter smoke asserts 1/1; docs reviewed for drift; no new dependencies.

## Phase 3 — Parity, semantics, and counter test suite

Depends on Phase 2. Tasks are authored concurrently in three work-groups and
land as focused commits; they share the `nested.rs` test module and fixture
helpers, so coordinate final assembly serially.

**Work-group A — parity core** (concurrent with B and C):

- [ ] **Serial reference**
    - Add the test-only serial reference retaining the pre-change walker
      settings and collect-all-non-directory behavior, feeding the unchanged
      `candidates_from_marker_paths` projection (AC1). It is a parity oracle
      and the same-process timing baseline — clearly documented as such.
- [ ] **Parity assertions**
    - For each fixture: assert the parallel and serial-reference candidate
      lists are equal as complete ordered `(root, matched_standards)` values
      (not byte-identical `Vec<Candidate>`), and independently assert the
      expected candidates for at least the rich fixtures so a shared
      projection bug cannot make both sides pass.
- [ ] **Repeat and vary workers**
    - Repeat the wide-tree comparison at least 20× (scheduling variance) and
      run it through the R1 seam in both a one-worker (`Some(1)`) and the
      default multi-worker configuration; both must pass all fixtures.

**Work-group B — fixture semantics** (concurrent with A and C):

- [ ] **Marker fixture matrix**
    - Disposable fixtures covering: multiple sibling directories and depths;
      empty and root-only trees; every one of the 12 fixed marker names; both
      `.sln` and `.slnx` suffixes; multiple markers mapping to one standard
      (both Gradle settings files); one marker mapping to multiple standards
      (`package.json` → npm/yarn/bun); multiple markers in one directory.
      Fixtures are immutable during assertion; no racy deletion.
- [ ] **Ignore-rule fixtures**
    - Git and non-Git roots; untracked-but-unignored marker (discoverable);
      `.gitignore` exclusion and negation; `.ignore`; controlled global
      excludes and `.git/info/exclude`. Isolate host Git/ignore configuration
      using the repository test utilities (git2-based init and fixture Git
      plumbing already used in `detection.rs`); never mutate process
      environment concurrently.
- [ ] **Traversal-edge fixtures**
    - Hidden directories stay eligible; the named-directory prune stays
      authoritative (`node_modules`, `target`, `dist`, `build`, …); a
      marker-named directory is not evidence but its permitted descendants
      can contain markers; missing root yields no candidates; non-directory
      symlinks to markers are admitted without following nested directory
      links or canonicalizing collected paths; a symlinked starting root is
      compared serial-vs-parallel where supported (R5).
- [ ] **Platform case rules**
    - Fixed marker names ASCII case-insensitive on native Windows and
      byte-exact on Unix (macOS, WSL2); `.sln`/`.slnx` case-sensitive
      everywhere; non-Unicode basenames are not markers. Reuse the existing
      predicate/matcher tests rather than duplicating platform logic; add
      fixture-level assertions only where the walk (not the matcher) is
      under test.

**Work-group C — counters and propagation** (concurrent with A and B):

- [ ] **Chokepoint counters**
    - Under a fresh collector (`performance::testing::measure`): direct
      fallback invocation on a populated fixture, an empty root, and a
      missing root each report exactly one `FS_READ_DIRS` and one
      `REPO_NESTED_MARKER_WALKS`; supplied-evidence discovery — including
      `Some(&[])` — reports zero fallback walks; legitimate detector work is
      accounted separately, never asserted to be zero.
- [ ] **Worker propagation**
    - Implement the R2 test-only callback counter; expected total equals the
      fixture entry count (cross-checked against the serial reference);
      assert the collector's post-walk reading matches. Include a diagnostic
      capture of the distinct worker threads observed on the wide fixture
      (feeds the "actual worker count" evidence gap the spec calls out).
- [ ] **Retain existing tests**
    - Keep `root_marker_does_not_register_a_candidate`,
      `supplied_evidence_and_the_fallback_walk_agree`, and the other existing
      tests listed in the grounding facts passing without weakening them.

**Validation checkpoint 3** — full `just test` and `just lint` pass in
`sniff/` (recipe keeps `remote` feature coverage); every fixture in AC2/AC3
covered; parity green in both worker configurations; counter assertions green
on the local host; no test relies on racy deletion, concurrent env mutation,
or unrecorded privilege assumptions.

## Phase 4 — Performance evidence campaign

Depends on Phase 3 (so the measured code is the tested code). All timing tasks
run serialized on the Phase 1 host; no concurrent workloads during timed
regions. Alternation is mandatory to expose drift.

- [ ] **Isolated walk A/B**
    - Same-process alternation of the serial reference and the parallel walk
      (identical build, profile, host, fixture): debug and release, ≥3
      warmups, ≥20 samples per case, median and range, warmed-cache labeled.
      Corpus = the fixed-path worktree checkout of this monorepo. Record the
      observed worker count and worker policy alongside.
- [ ] **End-to-end A/B**
    - In the fixed-path worktree, alternate `git switch` between the harness
      commit (baseline side) and the implementation commit (after side),
      rebuilding between switches; measure public
      `detect_repo_structure(<repo root>)` with the same sampling protocol
      and identical root spelling. Verify each measured request enters the
      fallback (counter check), never supplied evidence.
- [ ] **Re-attribute composition**
    - Remeasure the attribution of `detect_repo_structure` elapsed time
      (walk vs other work) on the after side; compare against the historical
      ~60 ms-of-232 ms debug split rather than assuming it.
- [ ] **Compare counters**
    - Separately from timing, compare stable work counters before/after
      (fallback walks, manifest reads/parses, metadata probes): unchanged is
      expected — the mechanism is concurrent traversal plus retaining fewer
      paths, not fewer logical walks. Disappearing worker counts are a
      defect, not an optimization.
- [ ] **Probe small and concurrent**
    - Measure a tiny tree's single-request latency and concurrent detections
      at the normal local test-runner concurrency: throughput, resource use,
      and latency versus baseline. Report regressions and uncertainty
      explicitly; never hide them behind the large-checkout average.
- [ ] **Apply the decision rule**
    - The claimed gain must exceed observed run-to-run variation on this
      checkout; compare against the historical 172→30 ms debug / 61→18 ms
      release walk-only figures as context only. Keep the walk's speedup,
      the command's speedup, and any compose claims strictly separate (no
      compose-suite claim without measuring that suite — out of scope here).
      If R7's material-regression condition triggers, stop and escalate.
- [ ] **Preserve the evidence**
    - Store commands, raw samples, provenance (commits, environment, cache
      treatment, alternation order) under `evidence/after/` and
      `evidence/counters/`.

**Validation checkpoint 4** — both A/B campaigns complete with alternation
and full provenance; counters compared; tiny-tree and concurrency probes
recorded; decision rule applied and the verdict (including any escalation)
written into the evidence bundle.

## Phase 5 — Cross-OS validation and downstream suites

Depends on Phase 4 (frozen implementation). The four OS legs are a work-group
across hosts; consult the `os` skill for which host produces which evidence
and how to reach it. Reuse qualifying passing evidence per the repo's
evidence-reuse policy; a request to avoid duplicate passing tests is not an
environment ban.

**Work-group — OS legs** (macOS, Linux, native Windows, WSL2 run concurrently
on their respective hosts):

- [ ] **macOS leg**
    - Confirm and record the full `just test` + `just lint` pass from Phase 3
      on this host as the macOS evidence cell.
- [ ] **Linux leg**
    - Run the focused parity/semantics/counter tests and `just lint` on a
      Linux host through existing workflows; record byte-exact marker-name
      assertions passing.
- [ ] **Native Windows leg**
    - Same focused set on native Windows: case-insensitive fixed-marker
      assertions are meaningful here; record any R5/R6 privilege-dependent
      skips (symlink elevation, permission denial) with rationale.
- [ ] **WSL2 leg**
    - Run the focused set inside WSL2 following the established reproduction
      path from the `os` skill; results join the Linux-path evidence.
- [ ] **Evidence bookkeeping**
    - Record the `{package, environment, tier}` cells each run or reused
      evidence satisfies; cross-compilation alone is not runtime parity; no
      new CI matrix cells or timing gates are introduced — confirm the CI
      plan output is unchanged for this branch.

**Work-group — downstream** (concurrent with the OS legs):

- [ ] **Darkmatter regression**
    - `just test` in `darkmatter/` passes, explicitly including
      `the_observation_is_fixed_at_request_creation`,
      `a_child_reads_the_observation_fixed_at_request_creation`, and
      `an_older_constructor_request_is_fixed_at_the_root_entry_not_by_the_child`.
      Report the actual selected and passed test counts; do not weaken those
      tests or change observation timing to meet a performance goal.

**Validation checkpoint 5** — every OS leg has green focused parity evidence
(fresh or qualifying reuse) with skips recorded; darkmatter suite green with
the three boundary tests confirmed and counts reported; CI scope unchanged.

## Phase 6 — Closure and review readiness

Depends on all prior phases. No new production or test behavior.

- [ ] **Consolidate results**
    - Write `sniff/fixes/2026-09-20-repo-perf/results.md`: outcome vs the
      spec's acceptance criteria (each criterion mapped to its evidence
      file), the performance verdict with medians/ranges, counter
      comparison, deviations from this plan, and remaining risks.
- [ ] **Drift sweep**
    - Re-run the doc/comment drift pass over `nested.rs` and check every
      document that mentions the walk or its counters — including
      `sniff/docs/sniff-library-architecture.md` and
      `docs/dependencies.md` (no crate changes expected; verify none
      happened). Update the `sniff` skill only if architecture or workflow
      actually changed (it should not have).
- [ ] **Final gates**
    - One last `just test` + `just lint` in `sniff/` on this host; verify the
      production diff is confined to `nested.rs` plus the measurement seam;
      run the repo's pre-commit graph change analysis before handing off.
- [ ] **Hand off for review**
    - Terminal state is "implementation complete, ready for review": summarize
      evidence locations for the reviewer. Moving the fix to `_completed` is
      the author's action, never the agent's.

**Validation checkpoint 6** — results.md complete and consistent with the
evidence bundle; drift sweep clean; final gates green; fix directory contains
spec, plan, log, results, and evidence; review hand-off summary delivered.
