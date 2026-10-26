---
kind: plan
created: 2026-10-26
total_phases: 5
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
human_review: false
---

# Pipeline work reduction implementation plan

## Phase 1: Establish the implementation baseline

Remove measured duplication while preserving execution, validation, diagnostics,
and file-resolution behavior. Implement C1–C4, C7, D1, D2, D4, D12, D13, and
the P1 duration parity matrix. Refresh C6/D3/D8 and apply their settled
thresholds to produce terminal classifications and any required active specs.
Deliver the X2 completion-walk candidate design without implementing completion
changes. The original spike is complete; this plan's phase numbers organize
the remaining work rather than restart that spike.

Success means each selected performance item has a deterministic regression
for removed work and behavior tests for its contracts; every existing anchor
has comparable before/after evidence or its existing documented limitation;
required area checks pass; and current topic docs explain changed lifetimes
and evaluation boundaries. No item awaits a scope decision. This planning
document does not itself execute the implementation or measurements.

### Decisions

1. **Scope authority**
   - **Decision:** Follow the spec's decided scope and Design Decisions over
     stale verdicts, totals, next steps, and open-question language in
     `spike-report.md`. Preserve historical measurements as historical data;
     add a dated current-outcome section instead of rewriting their history.
   - **Alternatives considered:** Repeat the spike; treat its original table
     as the final implementation list.
   - **Why:** The spec explicitly resolves validator reuse, lazy positions,
     conditional items, X1, X3, and X4 after the report was written.
   - **Downsides:** Readers must distinguish original evidence from refreshed
     evidence; the report needs an explicit current-status pointer.

2. **Dependency boundary**
   - **Decision:** Start production changes only on the combined code from
     `2026-10-26-sequence-quadratic-frontmatter` and
     `2026-10-25-shared-acyclic-definitions`, including their prerequisite
     chains, after refreshed counts and a saved baseline. Verify presence;
     do not merge branches or create another worktree as part of this plan.
   - **Alternatives considered:** Implement against either dependency alone;
     reuse the original loaded-host timings.
   - **Why:** Both dependencies change the costs and schema/runtime surfaces
     this work measures and edits.
   - **Downsides:** An absent dependency is an execution prerequisite; record
     the exact missing code and stop dependent work without inventing results.

3. **Measurement scope**
   - **Decision:** Reuse the existing scenarios, sizes, and anchors on one
     macOS host. Refresh dependency-affected counts/allocations and take one
     brief quiet-host Criterion baseline/comparison pair. No additional spike
     is needed: the existing evidence selects the implementation and X2 design.
   - **Alternatives considered:** New workloads; an OS performance matrix;
     repeat-until-stable timing campaigns.
   - **Why:** The spec forbids widening the completed spike. Counters and bytes
     already establish the selected work; timing uncertainty is reported.
   - **Downsides:** Timing conclusions remain host-specific. A noisy sample
     cannot establish a regression or justify dropping selected work.

4. **Conditional outcomes**
   - **Decision:** Classify C6, D3, and D8 from refreshed evidence using any
     one of: at least 5% anchor time or bytes; excess scaling reaching 10 ms
     or 10 MB at a plausible size no greater than 10× the largest real instance;
     or peak memory growing with an uncapped count. A qualifier gets its own
     dated active Claudine feature spec immediately; otherwise record
     **measured, no change**. Their production optimizations are deliverables
     of those separate specs, as the functional spec requires.
   - **Alternatives considered:** Implement all three here; leave a pending
     recommendation; use estimated shares as measured attribution.
   - **Why:** This applies the author's unchanged measurement rule automatically.
   - **Downsides:** The number of new spec artifacts depends on measurements.
     Unreachable anchors and estimates must remain explicitly identified.

5. **Runtime ownership**
   - **Decision:** Capture an immutable group-start seed after eager validation
     and context capture; construct each mutable branch at admission and release
     it after extracting required results/deltas, before replacement admission.
     Use coherent owned projections under one lock for leaf reads, with no
     lock or borrow held across lifecycle execution.
   - **Alternatives considered:** Snapshot the parent when each worker starts;
     retain completed branches; cache evaluated leaf values.
   - **Why:** Sibling isolation and declaration-order folding depend on the
     group-start view, while the peak-memory goal depends on prompt release.
   - **Downsides:** Results, mutation deltas, and eager metadata still grow
     with declared/completed work; requesting a large leaf still copies it.

6. **Validator lifetime**
   - **Decision:** Use the existing bounded `ValidatorCache`: context-aware
     complete union-arm schemas including reachable shared definitions for D1,
     structural schemas including all alphanumeric constraints for D13. Retain
     arm validators for one rewrite. Judge every value and file existence fresh.
   - **Alternatives considered:** Compile only in a rewrite-local memo; cache
     validation verdicts; move eager-file existence checks out of validation.
   - **Why:** The spec selects compiled-validator reuse and preserves ordinary
     `file(eager)` validation and passive editor behavior.
   - **Downsides:** File-sensitive contexts occupy separate cache entries and
     fresh filesystem judgments remain necessary.

7. **Validation ownership**
   - **Decision:** One library operation returns the report and a preparation
     summary (effective-schema presence and timing metadata); old entry points
     delegate to it. Supply a request-local lazy position provider, initialized
     at most once when a problem needs locations. Keep assignment coercion's
     earlier preparation separate from final validation.
   - **Alternatives considered:** CLI-owned duplicate validation; reuse a
     pre-assignment schema; skip positions only when no schema applies.
   - **Why:** Assignments can select different triggers, and the runtime
     baseline means ordinary valid documents generally do have a schema.
   - **Downsides:** The shared API needs a summary and lazy mapping seam;
     assignment invocations intentionally prepare twice in total.

8. **Named-table freshness**
   - **Decision:** Store the latest successful hash/table per existing resolved
     path identity. Continue reading and hashing on use with `biscuit-hash`;
     errors never serve the previous revision. Active requests keep their owned
     tables. Do not change path identity or introduce an eviction policy.
   - **Alternatives considered:** Whole-cache clearing at 256 entries; retain
     every revision; skip reads; cache failures.
   - **Why:** D12 removes unrelated parse churn while preserving freshness.
   - **Downsides:** Distinct paths remain unbounded and active requests may
     retain superseded tables. The memo is not a fixed byte budget.

9. **Parity ownership**
   - **Decision:** Put the cross-package P1 parity test on the Darkmatter side,
     using Content Policy's public parser and shipped `EDITOR_SCHEMA`. Keep
     the production dependency direction unchanged and use one test roster of
     rules, units, and baseline forms. Preserve intentional lexical/semantic
     differences rather than generate a grammar or strengthen editor regexes.
   - **Alternatives considered:** A production Darkmatter dependency in Content
     Policy; independent duplicated case tables; grammar generation.
   - **Why:** Darkmatter already consumes Content Policy's embedded schema.
   - **Downsides:** Authored patterns remain duplicated; the matrix detects drift.

10. **Completion design**
    - **Decision:** X2 enumerates once per identical resolved scope and traversal
      policy per request, then filters by alternative and merges in authored
      order with existing deduplication. Share candidate canonicalization where
      semantics permit it. Different scopes/policies remain separate.
    - **Alternatives considered:** One walk per alternative; a process-wide
      listing cache; implementing completion code in this feature.
    - **Why:** Existing 1/2/4/8 walk counts establish the duplication; the spec
      asks for a candidate design and fresh observations next request.
    - **Downsides:** Different scopes still walk separately; preserving pruning
      behavior requires explicit treatment in the design.

### Execution rules

Waves have unique indices across the plan. Every task in a wave depends on the
previous wave's checkpoint; tasks in the same wave may run concurrently unless
stated otherwise. Parallel implementation agents own disjoint files and run
only focused tests. The coordinator integrates shared-file changes and runs
full checks sequentially. Keep all work in this session's worktree. The entire
session tree is non-interactive; repeat that restriction in every agent brief.
This plan never authorizes committing, pushing, installing tools, or moving a
feature into `_completed`.

Each implementation task includes its own small L1 regression, behavior tests,
and a pass over affected comments/docs. Long allocation/timing scenarios stay
in benches. Use nextest through area recipes, never consolidated `cargo test`.
Do not run `cargo fmt`.

### Wave 1 — Dependency and harness verification

- [ ] **Verify prerequisites**
  - Confirm both dependency chains are present in the starting checkout and
    record the revision and dirty files in `implementation-log.md` beside this
    plan. Confirm X1 is already delivered by
    `2026-10-26-dmls-frontmatter-typing-allocation`; retain its recorded hover
    allocation comparison points: verb 58,763 and expression 105,025 bytes.
  - Inventory existing counters, allocation targets, timing targets, fixtures,
    and all C1–C8/D1–D13/X1–X4 report rows. Do not recreate completed tooling.
  - Verify counters compile away when disabled, allocator declarations occur
    only in allocation bench binaries, and timing targets use System.
    Check `tools/test-toolkit/README.md` and the `rust-testing` skill's
    performance guidance match the existing tooling; repair omissions only.
  - Run the existing L1/lint gates for the baseline in Darkmatter, Claudine,
    and test-toolkit; record pre-existing failures separately. Test-toolkit
    has no local `test`/`lint` recipes: use root `just test test-toolkit` and
    `just _lint test-toolkit` rather than inventing local commands.

Checkpoint: dependency code, instrumentation, runnable commands, and original
measurement limitations are documented before refreshing evidence.

### Wave 2 — One baseline refresh

- [ ] **Refresh evidence**
  - Re-run existing allocation/counting scenarios affected by dependency merges
    in `pipeline_work_alloc` (both libraries) and DMLS `pipeline_work_a3`;
    retain cold/warm distinctions. Reset peak immediately before each isolated
    allocation scenario, join workers before snapshots, and report scoped peak
    rather than subtract lifetime peaks. Repair harness accounting if needed.
  - Save `pipeline-work-pre` for the two existing library `pipeline_work`
    timing targets on the same quiet macOS host. Record exact commands,
    feature sets, toolchain, profile, revision/dirty state, and Criterion
    uncertainty. The DMLS A3 target uses the counting allocator: its elapsed
    times are instrumented observations, not Criterion/System timings.
  - Update the existing A3/A4/A6 evidence and other dependency-affected rows
    without adding workloads or repeating the full spike. Retain existing
    unreachable-anchor and estimated-share limitations.

- [ ] **Close conditional items**
  - Runs sequentially after **Refresh evidence**, within this wave. Apply
    Decision 4 to C6/D3/D8 and record metric, clause, realistic-size evidence,
    and terminal outcome for each in the report.
  - For every qualifier, create a separate dated active feature directory
    using the execution date and a descriptive operation name, with a concrete
    `spec.md` (candidate, measured evidence, scope, contracts, and acceptance).
    C6 uses literal fast paths/lazy shared state; D3 shares the request trigger
    registry; D8 retains syntax within candidate construction while evaluating
    fresh and preserving error timing/file calls. No `_unscheduled` artifacts.
  - Explicitly retain C5/C8/D5–D7/D9–D11 and X3/X4 as measured, no change;
    X1 remains delivered elsewhere. Confirmed implementation selections remain
    selected regardless of their refreshed share.

Checkpoint: before-change counts/bytes, saved timing baseline, and all three
conditional outcomes exist before Wave 3. There is no additional spike.

Target-specific benchmark commands from the repository root follow. Use the
shared benchmark recipe because Darkmatter's area `bench` hardcodes `--benches`
and forwards arguments after `--`, whereas this run must select Cargo targets.
The shared recipe also supplies the host preflight. Record the actual commands
in the log; replace `--save-baseline` with `--baseline` for the final timing
comparison. Do not use host-generated baseline names in place of the specified
`pipeline-work-pre` name.

```sh
just _bench darkmatter --features work-counters --bench pipeline_work_alloc
just _bench claudine --features work-counters --bench pipeline_work_alloc
just _bench dmls --bench pipeline_work_a3
just _bench darkmatter --bench pipeline_work -- --save-baseline pipeline-work-pre
just _bench claudine --bench pipeline_work -- --save-baseline pipeline-work-pre
```

## Phase 2: Implement independent foundations

### Wave 3 — Parallel foundations

- [ ] **Project runtime state** — C3
  - Own `claudine/lib/src/composition/runtime_state.rs` and snapshot-read
    callers in library/CLI task, leg, and container execution. Add only the
    narrow owned projections callers need; clone related requested fields
    coherently under one lock and drop the guard before lifecycle actions.
  - Audit mutation reads as well as container events. Keep full snapshots
    where semantically required; preserve retry clearing, pass restoration,
    mutation ordering, sequence binding, and lexical restoration.
  - Assert projection reads do not increment full-snapshot work or copy
    unrelated output history; test concurrent coherent reads and behavior.
    Update the runtime-state lifetime guidance and sequence topic docs.

- [ ] **Resolve ambient leaves** — C2 and C4
  - Own `claudine/lib/src/dispatch/expression.rs` and
    `claudine/lib/src/composition/looping/expression.rs`. Implement C2 and C4
    as separate focused changes, using closed member dispatch before whole-root
    materialization and copying only the requested value.
  - Compare dotted reads against whole-root projections for scalars, nested
    paths, arrays, absent/null values, reserved/unknown members, and large
    requested leaves. Preserve absent `hook.env.*`, namespace boundaries, and
    whole-root representation; demonstrate changed event/pass values are fresh.
  - Assert scalar reads cause zero whole-payload/loop-root materializations
    regardless of large sibling size, while whole-root reads still materialize.
    Update lifecycle/hook and loop lookup documentation.

- [ ] **Retain latest tables** — D12
  - Own the named-table memo section of
    `darkmatter/lib/src/markdown/schemas/resolve.rs`. Replace revision-keyed
    accumulation/256-entry clearing with Decision 8's path entry. Preserve
    existing read/hash observations, errors, shared tables, and import provenance.
  - Prove more than 256 distinct paths do not force unrelated reparses; an
    edited file reparses once while an unchanged file stays reusable. Cover
    malformed content, deletion, recovery, and an old table held by an active
    request. Exercise the input robustness cases below through the public API.
  - Update named-import/cache lifetime documentation; remove comments that
    still describe a bounded revision memo. Consult `biscuit-hash` for hashing
    changes and `os` before changing any path comparison or Windows handling.

- [ ] **Test duration parity** — P1
  - Own a dedicated Darkmatter L1 test module and its registration. Use one
    roster covering every production rule and `d`, `wk`, `mo`, `yr`, in
    default-baseline, property-baseline, and inline-date forms.
  - Exercise Content Policy's public parser, shipped editor pattern fragments,
    and the shipped schema through Darkmatter. Cover whitespace, references,
    malformed spellings, and a positive control for each form; identify lexical
    acceptance versus positive-count/calendar-date semantic rejection explicitly.
  - Keep Content Policy independent of Darkmatter in production. No performance
    counter or grammar generator. Document the tested grammar contract and
    intentional differences in the appropriate current topic pages.

- [ ] **Design shared enumeration** — X2
  - Own `completion-walk-design.md` beside this plan; inspect
    `claudine/cli/src/completion/schema_completion/candidates.rs` and the walker.
    Deliver a candidate design, not production completion changes.
  - Specify the request-owned scope/policy key, enumeration records, shared
    canonical identities, alternative-specific filtering, authored-order merge,
    and deduplication. Explain conservative combined pruning: one alternative
    must not prune files another could accept. Include hidden/gitignored/underscore
    filters, skip directories, symlink rules, overlapping roots, error behavior,
    display spelling, memory lifetime, and fresh enumeration next request.
  - Include an implementation sketch and future acceptance cases: 1/2/4/8
    identical alternatives walk once; distinct scope/policy walks separately;
    results/order match existing behavior. Record the existing quick timing
    sample within the already-required single-host pass, without gating design.
    Load `os` before reasoning about path identity changes.

Checkpoint: focused tests pass for each foundation; integration confirms
runtime projections are ready for C1 and schema memo changes for D13. Resolve
shared documentation edits through the coordinator, not concurrent rewrites.

## Phase 3: Integrate scheduler and schema reductions

### Wave 4 — Parallel integration

- [ ] **Admit private branches** — C1; requires C3
  - Own library `composition/sequence/task/group.rs` and CLI group scheduling
    in `commands/wrap/sequence/`, including `task_run.rs`. Move mutable runtime
    and live-frontmatter creation to admission from the immutable start seed.
    Preserve eager leg validation/context capture, worker cancellation/prep
    inheritance, sibling isolation, and declaration-order result/mutation fold.
  - Extract required result/deltas and release private branches before admitting
    replacements. Instrument deterministic live/peak branch counts if existing
    construction counters cannot express the lifetime contract; disabled
    instrumentation remains empty.
  - Cover capped/uncapped and nested groups, retries, interruption, fail-fast,
    never-admitted cancellation, atomic/tolerant outcomes, and delayed legs
    still seeing the group-start state in library and CLI paths. Assert peak
    heavyweight branches ≤ active cap and one construction per admitted leg
    per attempt; never assert constant total constructions. Use synchronization,
    not sleeps, to expose admission/release boundaries.

- [ ] **Classify provider hints** — C7
  - Own `composition/sequence/preflight/providers.rs` and its preflight callers.
    Classify effective agent/model hints before constructing expression state
    or making the caller's redundant frontmatter copy.
  - Preserve document/task `with:`/caller-setter precedence, runtime-written
    key and transitive dependency analysis, late values, normal expression
    failures, and launch-time authority. Text-valued referenced/runtime-written
    keys must not be mistaken for dependency-free literals.
  - Compare literal and equivalent-expression selection; assert zero provider
    expression-state constructions/copies on the safe literal path and fresh
    state on the expression path. Update provider-selection/preflight docs.

- [ ] **Reuse arm validators** — D1
  - Own schema `rewrite.rs`, its compose caller, and necessary `validate.rs`
    cache integration. Compile complete eligible arms with reachable shared
    definitions through the context-aware cache; retain handles for the rewrite
    rather than hashing/looking up per list element.
  - Preserve the exactly-one-accepting-arm rule; zero/multiple matches leave
    input unchanged. Test cold batch size independence, warm reuse, changed
    schemas, shared definitions, document/caller origins, and file create/remove
    between uses. Assert compilation work is per distinct arm/context rather
    than per item; every value/file judgment still runs fresh.
  - Keep validation-only read-only, ordinary eager-file existence checks,
    compose-only normalization, and existing diagnostic provenance unchanged.

- [ ] **Return preparation summary** — D2
  - Own `darkmatter/lib/src/markdown/schemas/mod.rs` validation orchestration
    and `darkmatter/cli/src/commands/schema/validate.rs`. Introduce the shared
    report/summary operation and delegate existing public validation entry
    points to it; expose only the metadata the CLI needs.
  - Remove the CLI's display-only second preparation. Retain the separate
    pre-assignment coercion preparation and recompute after assignments.
  - Assert one final preparation without assignments, two total with them,
    including a setter changing trigger selection. Cover baseline-disabled
    no-schema, baseline-only, explicit/trigger schemas, errors, pending values,
    every existing output format, quiet/verbose behavior, and exit statuses.
    Update schema-validation API/CLI docs without changing rendered output.

Checkpoint: integration passes focused C1/C7/D1/D2 behavior and work regressions;
no conflicts remain in runtime or schema files before the next wave.

### Wave 5 — Parallel schema refinements

- [ ] **Defer source positions** — D4; requires D2
  - Own validation orchestration in `schemas/mod.rs`, position consumers in
    `validate.rs`/`positions.rs`, and affected compose/DMLS adapters. Thread a
    request-local lazy mapping provider through report production while keeping
    existing eager-position entry points usable.
  - Assert valid baseline-only and no-schema validations build zero position
    maps; multiple problems requiring locations share at most one. Preserve
    problem codes, instance paths, spans, aliases/shared-definition origins,
    and original/composed source mapping. Include late preparation failures
    and pending-value behavior; do not delay diagnosis to avoid a map.
  - Exercise normal CLI and DMLS diagnostic paths, not just the provider
    helper. Update validation/diagnostic lifetime docs and relevant skill guidance.

- [ ] **Cache annotation checks** — D13; requires D12 and D1
  - Own alphanumeric default/suggestion validation in `schemas/resolve.rs` and
    focused tests. Use `ValidatorCache::structural_validator_for` with complete
    constraints; reuse its public API without editing the cache implementation
    concurrently with D4.
  - Assert one compilation for equal constraints across repeated resolutions
    and distinct compilation/behavior for different options. Verify defaults,
    suggestions, acceptance, and diagnostic origins; cache no value verdicts.
    Update alphanumeric/cache documentation where lifetime is described.

Checkpoint: every selected item has its deterministic L1 proof and contract
coverage, and all retained public validation entry points behave consistently.

## Phase 4: Verify behavior and input robustness

### Input robustness contract

This work changes the named-type file reader's memo lifetime (D12), so retain
loader/field-shape coverage there. It does not introduce a new configuration
format. Hook payloads and runtime values use C2/C4 behavior cases; expression
and duration text grammars are outside this configuration matrix.

Default: every invalid named-import document/field shape returns a located
public schema-resolution error, executes no commands, publishes no runtime
result, and never substitutes a cached old table. Cache-internal retention of
an old successful entry is allowed only if it cannot satisfy the failed read.
Preserve existing advisory suggestion-lint behavior as the exception: invalid
suggestion metadata remains located lint data on an otherwise usable schema.

Use a real frozen named-type fixture with one edit per case and an unedited
successful public-resolution control. Test the shared standalone envelope
validator once: tagged `types` absent, explicit null (including YAML `types:`),
wrong scalar/list type, and empty mapping. An empty mapping is valid and
declares no names; importing a missing name must report unknown type rather
than reinterpret the empty map as absence. Test mixed/all-invalid mapping
entries through the shared property-definition validator, with a valid entry
as control; no invalid entry may be silently discarded. Pure `$schema` and
tagged `types` envelopes each need a control and one invalid wiring case;
pure root unions and whole-file references cannot supply named imports.

The named-table reader passes text through the YAML loader and standalone
classifier, regardless of file extension. Cover that loader once for duplicate
keys, malformed/trailing content, a second YAML document, invalid UTF-8, and
valid format syntax (including JSON-compatible YAML); do not multiply the
matrix by extensions. Keep field validation outcomes separate from document
decoding outcomes. Reuse existing public coverage where it already proves a
cell; add only missing cases. Check both cold and previously successful warm
reads for malformed/deleted files, then repaired content recovery.

At the CLI boundary use `CliProcessFixture` for one positive case and one or
two invalid cases per loader, checking rendered source location and no launched
command. All remaining cells run in-process through public schema resolution
or validation. Before closing coverage, inspect changed loader paths for
`#[serde(default)]`, `Option<T>` collapsing null/absence, `filter_map(..as_str())`,
`unwrap_or_default()`, and `.ok()` swallowing load-bearing parse errors.
Any matrix finding records the state of every other shape for that validator
and every field wired to the affected shape, rather than fixing one cell only.

### Wave 6 — Integrated verification

The coordinator performs these two tasks sequentially: complete the coverage
audit, then run the consolidated required checks. Do not run area-wide checks
concurrently with focused agents or each other.

- [ ] **Audit behavior coverage**
  - Build an item-to-test map for C1/C2/C3/C4/C7/D1/D2/D4/D12/D13/P1 and
    the public loader matrix. Verify tests are registered in consolidated
    binaries and counter features are explicitly enabled through area recipes.
  - Use isolated nextest processes for global counter deltas, explicit
    cold/warm cache state, and fixtures under five seconds per L1 test.
    Scope repository reads in CI-indexable forms; use frozen prompt fixtures.
  - Check unchanged public output, diagnostics, approval membership,
    file resolution, retries, and lifecycle ordering through existing public
    integration tests. Preserve entry-point parity across library, CLI, and
    affected DMLS consumers. Repair any attributable regression within scope.

- [ ] **Run required checks**
  - Sequentially run `just test` and `just lint` in `darkmatter/` and
    `claudine/`; include the counter-enabled L1 cases using their supported
    package-qualified feature arguments. Run `just test-l2` where affected
    terminal coverage exists and headless browser checks only if affected.
    No terminal/browser test may activate a window or take focus.
  - Run root test-toolkit tests/lint as above for the required tooling check;
    run Content Policy area checks if its code/tests changed. Include DMLS
    in Darkmatter checks because its validation consumers and A3 are affected.
  - In a separate Cargo invocation build `darkmatter`, `darkmatter-cli`,
    `claudine`, and `claudine-cli` with default features, without test/counter
    feature unification; prove counter state is disabled. Check affected bench
    targets separately with their declared required features.
  - Preserve portability across macOS, Linux, native Windows, and WSL2.
    Use ordinary deterministic tests without byte/time budgets; reuse normal
    CI environment scheduling rather than adding performance cells. Load the
    `os` skill before Windows/path edits or investigating a single-OS failure.
    Record which environments actually ran; do not claim unexecuted evidence.

Checkpoint: required checks pass, matrix outcomes are explicit, default builds
remain uninstrumented, and each selected item is behaviorally complete.

## Phase 5: Compare evidence and prepare review

### Wave 7 — Final evidence and documentation

- [ ] **Compare anchor results**
  - On the baseline host/toolchain/profile/features/workloads, run the same
    target-specific timing commands with `--baseline pipeline-work-pre` and
    the existing allocation/counting targets. Run measurements sequentially
    without concurrent builds, tests, or other measurement agents.
  - Record each existing A1–A6 anchor's before/after time, bytes, direct work,
    and Criterion uncertainty in `implementation-log.md`; distinguish
    instrumented A3 elapsed observations from System/Criterion results. Retain
    existing unreachability/attribution limits rather than inventing percentages.
  - Resolve any reproducible regression attributable to the changes using
    direct work evidence. Do not add hosts, workloads, repetitions, portable
    byte assertions, or statistical gates. Repairs rerun only affected checks;
    repeated complete measurement campaigns are not part of this plan.

- [ ] **Reconcile current docs**
  - May run concurrently with measurement only as documentation work with no
    builds. Reconcile touched canonical pages in `claudine/docs/topics/` and
    `darkmatter/docs/topics/schemas/`, plus relevant skills. Explain behavior
    and lifetimes with compact examples; use Mermaid for admission/release
    flow where useful. Docs never name or link to feature/fix snapshots.
  - Update READMEs only if their overview claims changed; update dependency
    docs only if an unavoidable crate change occurred (none is planned).
    Confirm counting-allocator guidance stays correct in test-toolkit and the
    `rust-testing` skill. Keep the completion candidate's proposed behavior
    explicitly planned wherever described in current docs.

### Wave 8 — Closure audit

- [ ] **Prepare review record**
  - Reconcile the spec acceptance list against the item-to-test map, check
    results, anchor comparison, refreshed report, conditional active specs,
    and `completion-walk-design.md`. Record departures from the snapshot spec
    in the implementation log; do not rewrite its decided requirements.
  - Confirm no stale item awaits a ruling, no X1 cache was duplicated, no
    excluded optimization was implemented, and no unrelated production change
    or formatting churn entered the diff. After any repair, run only the
    relevant validation again unless its blast radius requires more.
  - Mark completed plan tasks and report **implementation complete, ready for
    review**, with exact checks and any evidence limitations. Leave the feature
    active and make no commit or lifecycle-directory move.
