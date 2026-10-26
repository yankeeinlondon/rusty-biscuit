---
kind: plan
created: 2026-10-26
total_phases: 4
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
human_review: false
---

# Remove quadratic sequence frontmatter work

## Phase 1 — Attribute the remaining growth

Remove work whose cost per step or leg grows with the number of other sequence
definitions. First attribute the unexplained prompt and shell preflight costs;
then share immutable document state, isolate each leg's writes, and restrict
per-task analysis to the task and its actual dependencies. Preserve all
preflight results, approval inventories and freshness checks, runtime writes,
retry and reread behavior, lexical namespaces, and diagnostic text and timing.

Successful completion means:

- Shell preflight has the same `ExpressionParse` count per step at 10 and 100
  steps with ten holes each.
- A group's private frontmatter views have constant per-leg construction work
  and allocation size at 10 and 100 legs, with fixed nonstructural payload.
- Prompt preflight's measured bytes per task at 100 and 1,000 tasks differ by
  at most 10%; counted L1 work proves the corresponding linear growth.
- A4, A6, and C1 have reproducible before/after measurements, and unchanged
  sequence, group, approval, preflight, and retry suites pass, including
  sibling isolation and atomic group settlement.
- `just test`, `just test-l2`, and `just lint` pass in `claudine/`; every
  test remains within five seconds; longer measurements belong in a bench.

This plan implements `2026-10-26-sequence-quadratic-frontmatter`. It leaves
admission-time leg allocation (C1), C2–C4, C6, C7's independent constant-factor
optimizations, D8's three-parses-per-hole optimization, and post-preflight
prompt launch optimization to `2026-10-25-pipeline-work-reduction`.
Sequence-size-dependent work at those same sites remains in this fix's scope.

### Decisions

1. **Attribute before repairing.**
   - **Decision:** The first task is one bounded release-build attribution
     spike for prompt and shell preflight. Counters confirm each sampled
     candidate before a repair is selected; include every confirmed site
     whose per-step work grows with sequence size.
   - **Alternatives considered:** Repair only provider prediction; assume
     `derived_deferred_keys` explains the entire shell term.
   - **Why:** The specification attributes only 231 MB of 2.33 GB for prompt
     tasks and labels the shell explanation a hypothesis. Current
     `Loader::walk_plan` rebuilds and rescans frontmatter per step, but reading
     code does not attribute the remaining allocations.
   - **Downsides:** Adds a measurement step before repairs. The discovered
     site list determines exact edit locations, not whether required scope
     is delivered.

2. **Share seeds, isolate writes.**
   - **Decision:** Use an immutable, shared group-start frontmatter seed with
     a private delta for each leg. The complete authored `sequence` value
     remains readable through the shared seed; leg construction never deep
     copies it. Apply this ownership rule to per-step preflight state too.
   - **Alternatives considered:** Delete `sequence` from each copied map;
     clone a whole map on its first write; share one mutable map between legs.
   - **Why:** Sharing plus a delta is an explicit mechanism allowed by the
     specification and preserves authored-data lookup, snapshot isolation,
     and write visibility without a quadratic first-write fallback.
   - **Downsides:** Existing concrete-map consumers need a layered lookup or
     borrowed adapter. An explicitly written value still requires owned
     storage; writing a large value is not made free by this fix.

3. **Analyze definitions once.**
   - **Decision:** Keep the resolved task definitions and document-level
     dependency facts shared for one preparation. Per-task work adds only
     that task's definitions, overlays, guards, and dependency closure. A
     changed-file reread creates fresh preparation facts while retaining the
     existing frozen execution membership and approval boundary.
   - **Alternatives considered:** Process-wide memoization; repeatedly scan
     the full frontmatter; hide structural keys from expression lookup.
   - **Why:** `walk_plan`, `step_frontmatter`, and provider prediction
     currently clone or visit document-sized state. Preparation-scoped
     ownership removes that multiplier without changing freshness rules.
   - **Downsides:** Shared dependency summaries must account for overlay
     shadowing and transitive reads. Explicit expressions that read authored
     definitions must retain their existing values and diagnostics.

4. **Preserve evaluation boundaries.**
   - **Decision:** Reuse passive parse/dependency facts without evaluating
     additional values or moving failures earlier or later. Keep caller and
     `with` precedence, runtime-written-key deferral, circular provider
     detection, and late `seq`/`group`/`loop` binding unchanged.
   - **Alternatives considered:** Eagerly evaluate all document values;
     classify every hint as late; bypass approval analysis for untouched tasks.
   - **Why:** Those alternatives alter the preflight verdict or failure
     timing promised by the specification and documented in
     `claudine/docs/topics/flow-control/sequences.md`.
   - **Downsides:** Some constant-factor passes remain. This fix removes the
     sequence-size multiplier rather than combining unrelated pipeline work.

5. **Use deterministic complexity evidence.**
   - **Decision:** Use feature-gated work counters for L1 and the existing
     allocation harness for bytes. Compare identical fixture shapes and
     measurement boundaries; report setup separately. Retain the fixed
     per-preparation intercept when explaining counts, and require the
     specification's equal per-step shell count for its exact fixture.
   - **Alternatives considered:** Wall-clock thresholds; exact allocated-byte
     assertions; only measuring small sequences.
   - **Why:** The original measurements were on a loaded host. Counts expose
     repeated work, while allocation benches capture costs a copy count alone
     cannot reveal.
   - **Downsides:** Counter-only tests cannot prove allocation savings; the
     final allocation measurements are also required.

6. **Bound profiling and validation.**
   - **Decision:** Profile on this macOS host only, using the existing large
     fixtures and one short CPU sample per unexplained path. No new spike
     repeats the already-proven group-copy diagnosis. Use nextest through
     package recipes, including for the ignored release measurement tests.
   - **Alternatives considered:** A cross-OS performance campaign; repeating
     all Track 5 experiments; copying the spec's `cargo test` command.
   - **Why:** No OS-specific cost is named. Existing C1 measurements already
     establish the group's cause, and repository rules require nextest.
   - **Downsides:** Performance evidence is host-specific; implementation and
     tests must still be portable across macOS, Linux, native Windows, and WSL2.

This is an ownership and analysis refactor, with no file-format, loader,
configuration grammar, or deserialization changes. The Input Robustness Matrix
does not apply. Preserve existing input validation and malformed-input tests;
do not introduce a new loader to implement shared state.

### Wave 1 — Attribution spike

- [ ] **Attribute preflight**
  - Run the existing large prompt and A6 shell scenarios in release mode.
    Sample each preflight entry point with `sample <pid> 15`, demangle with
    `rustfilt`, and inspect its call tree. If a scenario is too short, repeat
    that scenario within the same bounded profiling invocation; do not enlarge
    its input domain or launch real providers or shells.
  - Confirm candidate copying, traversal, state-construction, and expression
    analysis sites with counters behind `work-counters`. Record counts at 10
    and 100 steps for every discovered site, including sites outside the
    initially named files. Attribute allocation-heavy callees as well as CPU
    hotspots; CPU percentages alone cannot account for prompt bytes.
  - Inspect `preflight/mod.rs` (`walk_plan`, `step_frontmatter`, state and hint
    preparation), `preflight/providers.rs`, `preflight/reapproval.rs`, and
    called Darkmatter analysis/state adapters. Follow measured evidence to
    additional sites rather than limiting the repair to these candidates.
  - Create `implementation-log.md` beside this plan. Record scenario commands,
    feature flags, measurement boundaries, site/counter tables, baseline
    allocation rows, and a source-to-repair map.
  - Select repairs by result: repeated deep copies become shared-seed/delta
    access; repeated whole-document analysis becomes once-per-preparation
    facts plus task-local analysis; both findings require both repairs.
    Constant-size costs remain unchanged. Record concrete interface choices
    under this phase's `### Design Decisions`, with alternatives, why, and
    downsides; no approval or further spike is needed.
  - **Checkpoint:** Every unexplained path has confirmed sequence-size
    multipliers mapped to a repair, and the baseline reproduces the reported
    growth. If a measured contributor remains unexplained, continue attribution
    within this task before production edits.

## Phase 2 — Establish shared frontmatter ownership

### Wave 2 — Shared foundation

Depends on Wave 1. Keep this wave serial because it establishes interfaces
used by all three repair tasks.

- [ ] **Introduce shared views**
  - Introduce the smallest internal view needed to hold an immutable shared
    seed and owned overrides. Avoid a generic persistence framework or a new
    dependency. Share nested immutable values; an `Arc` around a map followed
    by deep clone-on-write of that whole map does not meet the contract.
  - Preserve presence, explicit null, whole-value shadowing, stored literal
    token interpretation, and authored structural values. A write to one
    scalar key must not copy the unrelated `sequence` subtree.
  - Adapt scope-stack/live-frontmatter lookup and write boundaries, and the
    measured evaluation-state adapters. Do not materialize a full owned map
    per step, leg, lookup, or write. Whole-document serialization remains
    permitted only at an existing boundary that actually requires it.
  - Retain one request's context and resolution authorities. If evidence
    requires Darkmatter changes, use its existing lookup abstraction where
    possible and preserve other consumers; expand validation to the changed
    Darkmatter surface.

- [ ] **Verify view semantics**
  - Add focused tests for shared seed reads, private delta writes, null
    overrides, nested lookup and whole-value shadowing, and readable authored
    `sequence` data. Demonstrate that two views cannot observe each other's
    mutations and that a small write does not copy structural definitions.
  - Compile the affected library and CLI consumers. Review changed symbols'
    docs and comments in the same edits; remove claims that a leg requires a
    full frontmatter copy.
  - **Checkpoint:** Consumers can use the new view without losing data or
    introducing hidden full-map materializations. Record the stable interfaces
    and file ownership for Wave 3 in the implementation log.

## Phase 3 — Remove every measured multiplier

### Wave 3 — Parallel repairs

Depends on Wave 2. These tasks can run concurrently using subagents after
assigning disjoint files. The shell task owns shared loader orchestration in
`preflight/mod.rs`; the provider task owns `providers.rs` and supplies any
caller integration as a handoff to that owner. Each task owns its focused
tests. Any newly discovered shared adapter is changed by one designated owner
before dependent tasks resume. Every subagent receives the session's
non-interactive restrictions and runs only its targeted checks.

- [ ] **Isolate group views**
  - Replace `run_parallel`'s `Mutex::new(seed.clone())` live cells in
    `sequence/task/group.rs` with private deltas over one group-start seed.
    Update the other measured container/scope paths that duplicate definitions.
  - Keep eager leg admission bookkeeping, runtime branches, declaration-order
    context capture, scheduling, and settlement as they are; admission-time
    allocation and independent runtime-history copies belong to Track 5.
  - Preserve private in-memory writes, member `set_frontmatter` disk effects,
    next-step rereads, nested-step filesystem visibility, retries, and atomic
    failure publication. Do not make every member reread disk at admission.
  - Keep `LiveFrontmatterCopy` meaningful: count actual copies, not a renamed
    shared-view creation. Add a separate view counter if required so zero
    deep copies cannot accidentally mean that no legs executed.
  - Validate 10/100-leg work counts and focused sibling-isolation,
    declaration-order merge, retry, and atomic-group tests.

- [ ] **Linearize shell analysis**
  - Repair every shell-path multiplier confirmed in Wave 1, starting with
    loader per-step state and repeated deferred-root/type/guard analysis.
    Prepare source facts once and overlay each step's own facts without
    walking other task definitions.
  - Preserve transitive dependencies, authored definition reads, task-local
    guard facts, lexical context, source locations, first-position propagation,
    exact/template approvals, closed membership, and reread freshness.
  - Leave the independent constant-factor placeholder parsing optimization
    alone. Add a `work-counters` L1 regression at 10 and 100 steps, ten holes
    per step, asserting equal `ExpressionParse` count per step and actual
    graph/approval results so omitted analysis cannot pass the test.

- [ ] **Linearize prompt preflight**
  - Repair all attributed prompt-path multipliers, including caller-side
    frontmatter copying and provider state construction where their size
    grows with the enclosing sequence. Use shared state and dependency facts;
    avoid an unrelated literal-hint fast path or launch optimization.
  - Preserve document → `with` → caller precedence and runtime-write
    deferral. Cover static, absent, null, circular, failing, shell-derived,
    transitive, and late hints through existing public preflight results.
  - Add small 10/100-task counted-work regressions for each repaired
    multiplier. Assert predictions and discovered tasks as well as work
    counts; leave the 100/1,000 allocation comparison to the bench.
  - **Wave checkpoint:** Integrate all repairs, run targeted sequence and
    preflight checks, and confirm every Wave 1 site has been addressed.
    Follow unexpected remaining sequence-size growth to its cause in this
    fix; do not file the required repair as follow-up work.

## Phase 4 — Prove compatibility and scaling

### Wave 4 — Behavioral verification

Depends on integrated Wave 3. Tests and documentation can proceed concurrently
in disjoint files; keep allocation measurements out of this wave to avoid
measurement contention.

- [ ] **Verify runtime parity**
  - Run existing sequence, nested-container, group, preflight, approval, and
    retry suites unchanged. Add only missing regression cases: sibling reads
    after another leg's private write; small live-frontmatter writes with a
    large inline definition; disk write followed by the next step's reread;
    nested-step file visibility; failed group publication; and preparation
    after a changed-file reread retaining frozen membership.
  - Assert public graph/prediction outcomes and execution results. Use fake
    providers/shells and temporary fixtures for in-process tests; use
    `CliProcessFixture` only for missing CLI wiring coverage. Preserve error
    text, source location, and the stage at which failure occurs.
  - Run the counter regressions through
    `just test-library --features work-counters <filter>` in `claudine/`.
    Use per-test before/after counter deltas; never reset shared counters
    while worker threads are active. Keep each L1 case under five seconds.

- [ ] **Document state ownership**
  - Update the private-view and static-preflight explanations in
    `claudine/docs/topics/flow-control/sequences.md`, with a compact example
    of a private write and a later step's disk reread. Distinguish private
    in-memory state from shared filesystem effects, including nested steps.
  - Update the Claudine skill where the ownership/workflow description
    changes, and any other topic page that describes copied leg state.
    Explain how the view is formed without promising a new public behavior.
  - Keep topic pages independent of spec paths/names. No README change is
    needed unless implementation changes a public behavior, which this fix
    is required to preserve.

### Wave 5 — Allocation evidence

Depends on Wave 4. Run measurement scenarios sequentially on one host. This
is acceptance remeasurement of the original spike, not a second attribution
spike or an expanded performance study.

- [ ] **Measure allocation scaling**
  - Update `pipeline_work_alloc.rs` and the existing crate-private measurement
    fixtures so they measure the actual new production view construction.
    Replace C1's obsolete standalone `fixture.frontmatter.clone()` probe;
    retaining that probe would measure code the executor no longer uses.
  - Run the A4 preflight and group-step rows, A6 at 10/100 steps, the complete
    C1 table at 10/100/1,000 legs with the original payload variants and cap
    four, and prompt preflight at 100/1,000 tasks. Preserve original fixture
    semantics and setup boundaries. Report remaining runtime-branch costs
    separately rather than requiring Track 5's C1 saving here.
  - Use `cargo bench -p claudine --features work-counters --bench
    pipeline_work_alloc -- <scenario>` for the allocation bench. For ignored
    crate-private rows, use the package's nextest-backed
    `just test-library --release --features work-counters --run-ignored only
    <exact-filter>`; select only the required measurement tests. Do not use
    the spec's historical `cargo test` reproduction command.
    Split existing multi-size measurement tests into bounded cases if needed;
    move a measurement that cannot fit five seconds into a bench while
    retaining access to its existing crate-private fixture. Do not relax the
    test timeout or reduce the required measurement sizes.
  - Report per-view bytes at 10/100 legs without exact-byte L1 assertions.
    Require no leg-count-dependent growth; also report counters, total bytes,
    and peak bytes. Require
    `abs((bytes_1000 / 1000) / (bytes_100 / 100) - 1) <= 0.10` for prompt
    preflight, using comparable full-preflight rows and no subtraction of
    unexplained costs.
  - Add before/after results and reproducible commands to this fix's
    `implementation-log.md` and to
    `claudine/features/2026-10-25-pipeline-work-reduction/spike-report.md`.
    Preserve historical baselines and identify this fix's measurements by
    `2026-10-26-sequence-quadratic-frontmatter`; keep raw evidence alongside
    the report when needed. Report time as contextual evidence only.
  - **Checkpoint:** All three scaling criteria pass and every attributed
    multiplier is removed. Repair failures within the owning task and rerun
    only affected checks and acceptance rows before final gates.

### Wave 6 — Final gates

Depends on Wave 5 and completed documentation.

- [ ] **Run final gates**
  - Run `just test`, `just test-l2`, and `just lint` in `claudine/` once over
    the integrated change. L2 must use the recipe's harness without raising
    terminal/browser focus. Do not invoke `cargo fmt`; correct any formatting
    findings manually. Run affected Darkmatter checks if its code changed.
  - Confirm all tests meet the five-second limit, both feature-on
    and ordinary builds compile, and no production path depends on counters
    or macOS sampling tools. Keep implementation platform-neutral; load the
    `os` skill before changing platform-specific or path-comparison code.
  - Record commands, results, changed files, completed decision records, and
    acceptance evidence in the implementation log. Check off tasks only when
    their outputs and checkpoints are satisfied.
  - Leave the spec snapshot unchanged and the active fix in place. Terminal
    state is implementation complete, ready for review; do not commit or move
    the directory to `_completed`.
