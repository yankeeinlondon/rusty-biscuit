---
kind: plan
created: 2026-10-26
total_phases: 4
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
human_review: false
---

# Shared acyclic definitions implementation plan

## Phase 1 — Establish behavior and measurement evidence

Implement automatic storage of repeatedly referenced structured named types in
Darkmatter's existing definition machinery, extract Claudine's shared document
metadata into one library, and verify every affected consumer. Success means
one reusable body for eligible repeated types, unchanged schema acceptance and
consumer behavior, an eight-file Claudine bundle with three triggers, and all
three effective-schema size cases below a fixed budget calculated from the new
maximum. Frozen inputs remain unchanged; only justified output snapshots move.

All paths below are repository-relative unless stated otherwise. Work stays in
the session's worktree. Implementation records decisions, evidence, departures,
and completed checkpoints in `implementation-log.md` beside this plan. It does not
edit the specification to describe implementation departures or move the feature
to `_completed`.

### Decisions

1. **Automatic eligibility**
   - **Decision:** Share an acyclic target only when it has at least two distinct
     authored value references and resolves to an object, pattern dictionary,
     tuple, array of structured items, or union containing a structured arm.
     Follow aliases without materializing expanded trees. Primitive-only aliases,
     primitive arrays, primitive-only unions, and callable signatures stay out
     of automatic sharing. A list of a union is written through a named union
     and `[]` (`ids: "id[]"` with `id: [string, number]`); classify it by its
     arms, so a list of a primitive-only union stays inlined. A `tuple(...)` is
     structured regardless of its slot types.
   - **Alternatives considered:** Share every named target; use a size heuristic;
     add author-controlled sharing syntax.
   - **Why:** These are the specification's settled eligibility rules and target
     the repeated lifecycle helper bodies without changing small primitive uses.
   - **Downsides:** Tiny structured types may grow, and primitive repetition remains.

2. **Authored site identity**
   - **Decision:** Count unique `(authoring namespace, full source path, target
     DefKey)` sites across root declarations and reachable value bodies, walking
     each body once. Preserve arm and tuple-slot positions, deduplicate root/body
     revisits, and track value reachability separately from signature-only graph
     reachability. References in a body reached only through a callable do not
     contribute unless that body is also value-reachable.
   - **Alternatives considered:** Count expanded occurrences; count only graph
     edges between definitions; pool counts across resolved layers.
   - **Why:** `graph.rs` currently discovers root references but stores edges only
     between bodies. Separate site accounting handles roots without multiplying
     parent uses or changing callable cycle admission.
   - **Downsides:** Descendants repeated only through parent expansion may remain
     inline; independently resolved triggers never pool their counts.

3. **Definition storage and guards**
   - **Decision:** Keep cycle membership and admission separate from the set of
     targets selected for storage. Generalize the existing worklist and two-pass
     summaries for cyclic plus selected acyclic targets. Use the single-pass path
     only when no target needs storage. Enforce the existing 32-level
     non-repeating import-chain limit independently of worklist boundaries,
     including selected chains and fallback paths; revisiting a recursive target
     retains existing cycle behavior rather than consuming an unbounded depth.
   - **Alternatives considered:** Introduce a second lowering implementation;
     use storage membership as recursion membership; let queued bodies reset depth.
   - **Why:** `resolve.rs` already switches passes on `engine.cyclic`, and
     `recursive.rs` already preserves inherited markers and provenance. Storage
     reuse must not weaken import safety or callable cycle rejection.
   - **Downsides:** Acyclic inputs can require two passes, and depth tracking must
     remain independent of the expansion stack.

4. **Explicit use compatibility**
   - **Decision:** Decide reference compatibility explicitly using the unchanged
     stored body, inherited summaries, local postfix, and property/item/union
     context. Inline only an incompatible acyclic use. Reuse existing typed
     conflict checks and inherited-first origins; never recover arbitrary errors
     by retrying inline. Keep selection fixed and emit only the transitive closure
     of definitions reached by surviving references.
   - **Alternatives considered:** Specialize bodies per use; reject valid narrowed
     uses; catch conversion errors and retry.
   - **Why:** The key-exclusion example cannot be represented by a sibling bound
     that undoes the stored bound. Union defaults, suggestions, and optional-file
     sentinel behavior require the same scrutiny as count constraints.
   - **Landing order:** this feature follows
     `2026-10-25-schema-diagnostic-provenance` (its `next_spec` predecessor), so
     `SchemaError::ConstraintConflict` already carries participants (origin plus
     typed constraint). Conflict tests through shared references assert each
     participant's value as well as its inherited-first origin. If that feature
     has unexpectedly not landed when this one starts, assert origins only and
     record the gap in `implementation-log.md`; do not implement participants
     here.
   - **Downsides:** Some eligible uses remain inline; compatibility needs direct
     behavior tests beyond successful JSON Schema compilation.

5. **Identity and consumer parity**
   - **Decision:** Preserve `DefKey` identity, deterministic traversal, opaque JSON
     keys, existing equality reuse and collision rekeying, captured resolution
     contexts, dependency invalidation, and passive validation. Consumers follow
     references along the requested value or cursor path. Preserve diagnostic
     codes, full instance paths, positions, and declaration origins; internal
     JSON keyword paths may change. A reachable callable still errors at its use.
   - **Alternatives considered:** Deduplicate globally by body or name; expand
     every definition for consumers; require identical keyword paths.
   - **Why:** Sharing intentionally changes representation, while callers depend
     on semantics and authored locations. Existing merge machinery owns identity.
   - **Downsides:** Representation-sensitive tests must adapt; equal unrelated
     namespace bodies need not become one definition.

6. **Metadata library and parity**
   - **Decision:** Create `claudine/schemas/document.yaml` containing unchanged
     `prompt`, `hash`, and `last_updated` types. Import them from each metadata
     trigger independently and ship eight bundle files with three triggers.
     Replace the line-subset assertion with comparison of resolved property
     structure and every reachable definition, normalizing generated identifiers
     through a consistent reference mapping rather than comparing bare `$ref`s.
     Use visited definition pairs so comparison terminates for recursive graphs.
   - **Alternatives considered:** Use `linking.yaml`; combine triggers; compare
     authored lines or generated reference names.
   - **Why:** Metadata has separate ownership from linking, and independent
     shadowing and resolution are existing contracts. A separate two-use `hash`
     fixture proves sharing; one import in each trigger does not.
   - **Downsides:** Complete disk bundles gain a library dependency, and the parity
     test must compare reference graphs as well as property fragments.

7. **Behavior before snapshots**
   - **Decision:** Before resolver or shipped-schema edits, inventory frozen and
     shipped inputs and capture positive/negative public results. Regenerate only
     affected output snapshots after the same probes pass. Preserve frozen input
     bytes and byte equality for unaffected entries, including digest entries.
   - **Alternatives considered:** Regenerate all outputs immediately; replace
     frozen inputs with newly extracted metadata schemas.
   - **Why:** `recursive_schema_golden.rs` explicitly promises acyclic byte
     stability today. This feature replaces that output promise, not its behavior
     evidence or historical inputs.
   - **Downsides:** Each changed output needs a recorded explanation and semantic
     evidence; blessing alone cannot establish correctness.

8. **Measurement and budget**
   - **Decision:** Use the existing three effective-schema cases and compact UTF-8
     JSON serialization, including baseline and `$defs`. Record one cold sample
     per case before and after on the same host and build mode, with resolution
     and compilation timed separately inside a fresh process. Set the checked-in
     integer limit to `100_000 * ceil(1.25 * M / 100_000)` using the measured final
     maximum `M`. The result must be below `2_000_000` bytes.
   - **Alternatives considered:** Use the draft estimate as baseline; derive a
     moving limit during tests; add statistical timing or multi-host gates.
   - **Why:** The specification defines this formula and proportional measurement.
     The largest allowable rounded limit is `1_900_000`, so `M <= 1_520_000`
     is necessary. No separate performance spike is needed: the required matched
     observation already answers the cost question without a new design gate.
   - **Downsides:** Samples are observations, not speed claims. If correct,
     in-scope implementation cannot meet the lower budget, preserve the spec's
     explicit stop condition: report before/after sizes and largest remaining
     helpers, leave the phase incomplete, and stop without relaxing the formula,
     widening lifecycle profiles, or claiming completion. This contingent failure
     does not create an unresolved design decision at implementation start.

9. **Robustness scope**
   - **Decision:** Add no new format loader, configuration field, or deserializer.
     The work changes schema grammar resolution and authored library content, so
     the requested Input Robustness Matrix does not apply. Retain existing loader
     robustness suites and add public semantic probes for metadata and reference
     behavior. In particular, retain optional metadata's existing absent/null
     acceptance and `hash.value: unknown` accepting arbitrary data.
   - **Alternatives considered:** Add a new per-field YAML matrix; treat every
     absent or null metadata value as an invalid configuration shape.
   - **Why:** The user excludes text grammars from that matrix, and this feature
     does not change the loader. Schema-defined metadata semantics remain binding.
   - **Downsides:** This plan supplies no new loader contract; an implementation
     must not introduce a new loader as an incidental shortcut.

### Wave 1 — Capture the baseline

Run these tasks sequentially under one owner. Both must finish before any resolver,
trigger, library, bundle, or snapshot changes. Measurements are implementation
work; this planning pass does not invent baseline values.

- [ ] **Capture semantics**
  - Inventory affected shipped artifacts, all frozen corpus entries, current
    representation assertions, and public consumers of `TypeExpr::Ref` and
    `DefinitionTable`. Read the darkmatter schema/testing guidance and claudine
    skill; use repository test-toolkit conventions for fixtures and public results.
  - Add/reuse public validation probes for passing and failing frozen inputs,
    shipped schemas, and Claudine's deep ordinary/initialization/cancellation and
    sequence/profile corpus. Save expected verdicts and diagnostic tuples before
    edits: code, full instance path, line/column, and declaration origin.
  - Include invalid values at each of two uses, a nested list item, and missing
    required fields; capture defaults, optional/null behavior, coercion, and the
    parser-versus-schema distinctions already tested by Claudine.
  - Record baseline revision, input inventory, focused passing commands, and
    unchanged frozen inputs in `implementation-log.md`. Give every planned acceptance
    family a named existing or new test owner; avoid duplicating existing coverage.

- [ ] **Capture measurements**
  - Reuse `embedded_trigger_payloads_resolve_within_two_megabytes` inputs exactly:
    `claudine.yaml` (`start: []`), `sequence.yaml` (`sequence: []`), and `both`.
    Record compact byte counts and identify largest repeated helpers by authored
    target, including action categories, bundles, loop verbs, and profile variants.
  - Use temporary measurement scaffolding around the same effective-schema path.
    Build once, then run each case in a fresh process, recording cold resolution
    and validator compilation separately without build/startup time or warm
    caches. Reuse existing timing instrumentation where it measures those stages;
    keep necessary probe code out of routine correctness tests.
  - Record host OS, build mode, invocation, stage boundaries, and bytes/times so
    Phase 4 can reproduce the sample. No benchmark project or extra hosts.
  - Inspect the current state of `2026-10-26-dmls-frontmatter-typing-allocation`.
    If its changes are already present, capture pre-change hover and diagnostics
    allocation bytes using `cargo bench -p dmls --bench pipeline_work_a3` now;
    Phase 4 supplies the matched post-change run. Otherwise record that the later
    change owns this paired benchmark obligation.

**Checkpoint:** Baseline semantics, actual byte figures, cold-stage samples, and
the DMLS landing-order obligation are recorded before implementation starts.

## Phase 2 — Implement selection, sharing, and metadata extraction

### Wave 2 — Independent foundation tasks

These two tasks may run concurrently after Wave 1. Give them separate file
ownership. The resolver owner owns graph code and its tests; the metadata owner
owns Claudine schema/bundle/parity changes, excluding the final byte-budget test.

- [ ] **Discover shared targets**
  - Extend `darkmatter/lib/src/markdown/schemas/resolve/graph.rs` and the resolver's
    discovery result to expose value-site counts and structured classification
    alongside unchanged cycle membership/admission. Keep discovery passive and
    reuse existing namespace/import resolution rather than adding reads.
  - Follow aliases with memoized shape classification and explicit handling of
    cyclic targets. Preserve deterministic first-reached order and root-union
    namespace ownership. Include root sites and distinguish callable-only reach.
  - Prove eligibility and counts through resolved schema output: two root sites,
    two sites in one body, alias sites, repeated-parent nonmultiplication,
    root/body deduplication, unused declarations, callable-only references,
    canonical namespace identity, and same names in separate namespaces.
  - Cover objects, pattern dictionaries, tuples, object arrays, structured unions,
    primitive aliases, primitive arrays, primitive-union arrays, and primitive-only
    unions. Cyclic admission tests remain unchanged and passing.

- [ ] **Extract document metadata**
  - Add `claudine/schemas/document.yaml` as a passive `kind: schema` library with
    the three exact declarations, descriptions, defaults, and hash forms. Replace
    only the corresponding payload declarations in `claudine.yaml` and
    `inline-compose.yaml` with relative named imports; preserve match conditions,
    property order, openness, optional presence, and independent shadowing.
  - Update `lib/src/composition/schema/bundle.rs`, `shipped_bundle`/`SchemaRepo`
    disk fixtures, embedded inventory, byte-equality checks, and export probes.
    Assert eight trigger/library files and exactly three trigger files; do not
    add separately embedded host catalogs to that count.
  - Replace the textual subset assertion in `lib/tests/l1/claudine_schemas.rs`
    with resolved graph parity for all three properties. Retain behavioral parity,
    test each trigger independently and the effective merge, and retain shadowing
    and embedded-without-disk tests. This task must pass against the old resolver
    as well as the new one; final sharing evidence belongs to Phase 3.

### Wave 3 — Generalize definition expansion

Depends on **Discover shared targets**. One owner edits the coupled resolver,
summary, and lowering code; do not split these files among simultaneous agents.

- [ ] **Expand selected definitions**
  - Update `resolve.rs`, `resolve/recursive.rs`, and affected
    `simplified/definitions.rs`/`simplified/convert.rs` paths to use separate
    cyclic and storage sets, generalized worklists, inherited use summaries, and
    the existing two-pass expansion path. Preserve the no-storage fast path.
  - Expand each selected body once per pass, move only existing use markers and
    annotations to uses, and retain child declarations and value constraints in
    bodies. Preserve lexical contexts, example-file origins, dependencies,
    searched locations, cache invalidation, and embedded/reserved import parity.
  - Preserve the 32-level non-repeating import limit independently of storage.
    Test the existing accepted/rejected boundary with both inlined and selected
    intermediate targets, aliases, and unchanged descending/non-descending cycles.
  - Implement explicit per-use compatibility, including changed versus unchanged
    key exclusions, inherited/local bounds, union-arm markers, default conflicts,
    suggestion limits, array/item context, and optional-file empty strings.
    Unsupported valid uses inline; invalid schemas retain their typed failures.
  - Prune JSON definitions by reachability from surviving references, transitively
    through definition bodies. Test mixed shared/fallback uses and all-fallback
    output with no orphan definitions. Preserve error reporting for reachable
    callables at the use's property path.
  - Review comments/docs on every changed symbol, including claims that a
    definition is necessarily cyclic. Keep unrelated formatting untouched.

**Checkpoint:** Focused resolver and metadata tests pass together. Tests prove
one body/two references, single-use inlining, per-use independence and fallback,
no unused definitions, unchanged cycle/depth guards, and eight-file bundle parity.

## Phase 3 — Verify consumers and end-to-end behavior

### Wave 4 — Independent consumer lanes

Depends on Phase 2. Three tasks may run concurrently with separate ownership:
Darkmatter library consumers/tests, DMLS providers/tests, and Claudine integration
tests. Coordinate shared fixture edits through one owner. Each lane runs its
focused tests; the complete area suites run once after integration in Phase 4.

- [ ] **Preserve library semantics**
  - Audit consumers that infer recursion from definition presence. Change only
    affected assumptions in phase projection, validation/origin projection,
    defaults, coercion, eager-file handling, deferred lifecycle operands, operand
    validation, completion lookup, and callable rejection.
  - Re-run Phase 1 expectations against sharing. Assert unchanged launch and
    completion outcomes, required/null behavior, property versus list-item
    nullability, default application, file normalization, and deferred operands.
    Validation-only paths remain read-only and execute nothing.
  - At both uses, compare diagnostic codes, full instance paths, positions, and
    declaration origins against captured expectations, including nested items,
    missing required fields, and inherited-first constraint-conflict origins.
  - Test equal-body reuse and different-body rekeying through baseline, trigger,
    document, and root-union merges, including references between definitions.
    Assert all references resolve to their intended bodies and resolution bytes
    are deterministic. Retain imported-file dependency invalidation coverage.
  - Exercise disk, embedded, and reserved imports through public resolution APIs;
    extend existing passive shipped-artifact coverage rather than making a new
    terminal suite for resolver behavior.

- [ ] **Preserve editor behavior**
  - Extend `darkmatter/dmls/tests/l1/recursive_schema_editor.rs` and appropriate
    provider/diagnostic tests with a repeated acyclic structured target at two
    uses, including a nested list item and an alias/imported declaration.
  - Through the normal DMLS request path, assert diagnostics and source ranges,
    completion, hover, and definition navigation at each use and the intended
    declaration. Preserve incomplete-object completion and imported dependency
    refresh. Add focused `md schema validate` wiring for a valid and an invalid
    shared use with the correct location and full instance path.
  - Keep traversal proportional to the requested cursor/value path: do not rebuild
    fully copied trees to serve editor queries. Use existing fixtures and
    `CliProcessFixture`; no focus-dependent terminal or browser tests are needed.

- [ ] **Prove Claudine integration**
  - Through passive validation and `pre_validate_schema`, cover independent main
    and inline triggers, their normal merge, absent/null metadata, prompt/date
    validity, shorthand hash strings, longhand hashes and defaults, arbitrary
    `hash.value` data, invalid metadata, and trigger shadowing.
  - Add a small same-resolution fixture importing `hash@./document.yaml` into two
    properties: assert one shared definition, references at both properties, and
    acceptance of both stored hash forms. Do not infer sharing from independent
    trigger imports.
  - Reuse Claudine's deep lifecycle/profile corpus and generated operand tests to
    prove unchanged action permissions, initialization/cancellation restrictions,
    launch verdicts, deferred templates, and schema/parser distinctions. Keep
    metadata extraction separate from lifecycle grammar changes.

**Checkpoint:** Every acceptance family has semantic evidence through its owning
public API or normal CLI/DMLS request path. Existing frozen inputs and expected
behavior still pass before any output snapshots are regenerated.

## Phase 4 — Lock representation, document behavior, and validate

### Wave 5 — Finalize evidence and documentation

Depends on Phase 3. The three tasks below may run concurrently: assign frozen
snapshots, Claudine budget/measurement, and docs/skills to separate owners.

- [ ] **Refresh affected snapshots**
  - Review the frozen corpus entry by entry against Phase 1. Use the existing
    `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1` mechanism only for affected outputs after
    semantic probes pass. Review JSON and digest-backed changes, record the cause
    for each, and retain byte equality for unaffected entries.
  - Do not modify frozen schema inputs or rewrite them to import the new metadata
    library. Current shipped-artifact tests separately cover that library.
  - Update `recursive_schema_golden.rs` comments and the corpus README to describe
    frozen-input semantic preservation and intentional shared-definition output,
    removing the superseded universal acyclic byte-stability claim.

- [ ] **Enforce measured headroom**
  - Repeat the three compact-size measurements and quick cold samples with exactly
    the Phase 1 inputs, host, build mode, and stage boundaries. Record before/after
    resolution/compilation times, bytes, remaining repeated helpers, maximum `M`,
    and the integer calculation of the rounded budget.
  - Check in the fixed lower integer constant in
    `claudine/lib/tests/l1/claudine_schemas_embedded.rs`; rename the two-megabyte
    test and comments to match the actual limit. Keep all three cases, baseline
    inclusion, existing recursive-family assertions, and embedded-only resolution.
    The test compares against the constant; it never derives its own allowance.
  - If the result cannot yield a constant below `2_000_000`, apply Decision 8's
    stop condition and report evidence without declaring this phase complete.
    In-scope defects are fixed before concluding the target is unattainable.
  - If this feature lands after the DMLS allocation change, run
    `cargo bench -p dmls --bench pipeline_work_a3` for the matched post-change
    sample and log hover/diagnostics allocation bytes beside the Phase 1 values.
    Investigate and fix a rise caused by rebuilding expanded trees; do not add a
    timing threshold or repeat the spike as a later routine gate. If this feature
    lands first, record the benchmark obligation for the later change.
  - Remove temporary measurement scaffolding after preserving reproducible
    invocation details and results in the implementation log.

- [ ] **Document shared behavior**
  - Update all acyclic-inlining promises in
    `darkmatter/docs/topics/schemas/definition.md`, including named imports, cycle
    flow, consumer lookup, and limitations. Add a compact two-property object
    example beside a narrowed primitive alias, plus the key-exclusion fallback
    example and a Mermaid flow showing selection and per-use compatibility.
  - Update `local-type-references.md` and `schema-triggers.md` in the same docs
    directory for local sharing, namespace identity, independent layer counts,
    equal-body reuse, and conflicting-body rekeying. Explain behavior to a reader
    new to the repository, without linking or referring to feature snapshots.
  - Update `claudine/docs/topics/schemas.md` with the metadata library and complete
    eight-file disk-bundle instructions, including shadowed embedded triggers.
    Keep `claudine/schemas/README.md` an overview linking to those instructions.
  - Update affected darkmatter and claudine skill pages for lowering and bundle
    inventory, and remove stale cyclic-only/inlining comments in touched code.
    No dependency changes are planned; update dependency docs if implementation
    nevertheless changes a crate dependency.

### Wave 6 — Run final gates

Depends on all Wave 5 tasks. One integration owner runs the required commands
after all edits are combined; do not run competing full suites in subagents.

- [ ] **Run acceptance gates**
  - In each of `darkmatter/` and `claudine/`, run `just test`, `just test-l2`, and
    `just lint`. Use nextest through these recipes, keep each new L1 test below
    five seconds, and use the repository terminal harness without activating
    windows. The darkmatter L1 recipe already includes DMLS and its effects
    instrumentation; retain that coverage without a duplicate full DMLS run.
  - Treat lint as a check. Do not run `cargo fmt` or apply repository-wide
    formatting. Fix only formatting introduced by this implementation manually.
  - Keep changes portable to macOS, Linux, native Windows, and WSL2, with hermetic
    contexts and archive-safe test paths. Load the `os` skill before changing
    platform conditionals/path comparison or asserting a host cannot be tested.
    Do not add CI environments or substitute timing for correctness evidence.
  - Review the diff for unexplained snapshot movement, changed frozen inputs,
    altered lifecycle profiles, new syntax/flags, hidden callable failures,
    fallback-by-error-catching, unused definitions, and stale docs. Resolve each
    defect in scope and rerun only affected checks, followed by any gate whose
    result the fix invalidated.

- [ ] **Record implementation outcome**
  - Map the specification's seven acceptance groups to concrete tests and passing
    results in `implementation-log.md`. Include before/after measurements, chosen
    budget, DMLS benchmark responsibility/results, representation changes,
    decisions made during implementation, and any documented departures.
  - Confirm every task and checkpoint is satisfied before checking off the plan.
    Report implementation complete and ready for review. Do not commit, push, run
    `just complete`, or move the feature directory as part of this plan.

**Checkpoint:** Both areas' required gates pass; consumer behavior, metadata
parity, frozen-input preservation, deterministic representation, and the lower
fixed budget are proved, with measurements and current docs recorded.
