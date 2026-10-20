---
created: 2026-10-20
total_phases: 6
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - claudine
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/expressions.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/tests/l1/shell_template_guards.rs
    - darkmatter/lib/tests/l1/shell_template_execution.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/lib/tests/l1/main.rs
    - claudine/lib/src/composition/lifecycle/shell_template.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/preflight.rs
docs_updated_during_phase_3:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/expressions.md
---

# Shared type scope implementation plan

## Summary and Success Criteria

Replace the independent static presence and type classifiers with one immutable,
Darkmatter-owned `TypeScope`. Carry that scope through expression branches, body
blocks, lifecycle conditionals, and sequence task guards. Use it for shell
candidate typing, transclusion nullability, absence warnings, and passive DMLS
analysis. Preserve concrete runtime validation and existing shell approvals.

Success means the guarded commit-message example passes discovery with a missing
message after template approval, skips execution when its guard is false, and
passes the live message as one argument when selected. An unguarded missing
message still fails preflight. A selected command whose value becomes null or
changes to an unapproved type fails before any segment launches. Facts never
escape their region or survive an invalidating mutation or execution boundary.

Completion also requires:

- One branch-fact producer and one shared structural subtype relation, with no
  competing static presence classifiers in the audited consumers.
- Evaluator-consistent equality, true/false branch joins, canonical binding
  identity, optional-property nullability, and conservative unknown handling.
- Composition and DMLS agreement at supported sites, with no evaluation or I/O
  in scope construction or passive analysis.
- All 14 specification acceptance criteria mapped to passing L1 evidence;
  future projection and argument-binding capabilities explicitly marked planned.
- Updated current topic documentation and skills, passing affected-area checks,
  and an implementation log recording any departure from the specification.

Affected implementation packages are `darkmatter`, `dmls`, and `claudine`;
`darkmatter-cli` and `claudine-cli` supply invocation regressions. Reuse the
existing schema projection and the implemented `2026-10-23-shell-template-approval`
paths. Do not implement the later lifecycle extraction, argument binder, host
type expansion, or nested-union projection as part of this fix.

Waves are globally numbered. Tasks within a wave may run concurrently on the
stated file boundaries; the next wave waits for the preceding checkpoint.
Parallel implementation does not authorize concurrent test, lint, and tier-list
commands that rebuild shared fixture-enabled binaries. A single implementer can
execute every wave serially with the same dependencies.

## Phase 1 — Establish Contracts and Coverage

### Necessary Rules

1. **Baseline ownership.** The current checkout still has the Claudine lifecycle
   preflight walk. Implement against that owner. If the independently active
   `2026-10-27-darkmatter-lifecycle-ownership` extraction lands first, map the
   same scope contract onto its new owner and record that adjustment; do not
   create a second lifecycle walk or make this fix depend on the extraction.
2. **Projection boundary.** Extract/reuse the current effective-schema lookup
   behind `template/typing.rs::project_hole` and the descriptor authority used
   by directive targets. Preserve their legitimate concrete-value rules through
   the shared layer. Unsupported declarations remain unknown. Existing
   projections must support the specified top-level union examples; richer
   nested shapes and function-result typing remain planned.
3. **Presence versus approval.** Non-null declared types do not excuse actual
   nulls. Only an enclosing live guard's presence evidence, or an existing late
   contract, permits discovery deferral. Record guard evidence by command site
   and expression; never add a document property to `DeferredShellRoots`.
   Keep original declaration, guard refinement, and approval conversion distinct.
4. **Dispatch boundary.** Validate guard-dependent holes at the selected action,
   before its first chain segment, including holes present during discovery.
   Resolve each hole once, check null, original schema, and approved refinement,
   then dispatch that same value. Preserve the task-start barrier for ordinary
   sequence late holes. Exact, prefix, template, and automatic approval paths
   must all retain their applicable validation and closed membership checks.
5. **Equality authority.** Follow the current evaluator's `scalar_string` and
   both-null behavior exactly. Never infer a numeric literal without formatter
   uniqueness, erase constraints, or treat a contradiction as an empty type
   that waives checks. Unknown-minus-null is internal presence evidence, not
   compatibility with a concrete parameter.
6. **Predicate migration.** Today's strict `file_exists` handler permits a
   success-side present-string fact; passive analysis never calls the handler.
   The author must resolve the original-value/conversion policy before later
   argument conversion registers facts. Recommend presence-only evidence after
   conversion unless metadata proves more. This future ruling does not block
   the present fix; never infer the original type from a converted copy.
7. **Fact lifetime.** Resolve subjects by binding identity, not root spelling.
   Bare/`fm` aliases share facts only for the same document binding. Static
   writes invalidate the destination, ancestors, and descendants; dynamic or
   unknown document writes invalidate all document facts. Preserve possible
   writes through joins, even if an inner branch has ended. Events, iterations,
   retries, resumes, proxy handoffs, and document re-entry start fresh.
8. **Input robustness applicability.** This fix changes analysis of typed schema
   projections and expression/lifecycle regions, not a configuration loader or
   deserializer. The file/configuration Input Robustness Matrix is therefore
   outside the planned scope; text grammar and shell rendering do not trigger
   it. If implementation must change a loader or field validator, first add its
   bounded matrix: default invalid shapes yield a located error, no launch, and
   no publication; state only exceptions, preserve absent/null distinctions,
   share validator cases, and cover each distinct loader's document shapes.
   Use a real fixture control plus one edit per case through the public API;
   use CLI cases only for wiring. Audit `serde(default)`, `Option<T>`, filtered
   elements, `.ok()`, and `unwrap_or_default()` on load-bearing parses. Any
   finding must report the other cells in its validator/loader row and column.

### Spikes

No separate spike is scheduled. The specification already supplies equality
semantics, supported projection limits, and the approval boundary; code inspection
can establish the ownership seams. Evaluator differential cases belong in
permanent correctness tests. No performance spike is required unless a concrete
implementation cost emerges. If one does, record the question first and take one
quick sample on one host before the affected work; broader or OS-specific
measurement requires an explicit author ruling. Run a spike only once.

### Wave 1 — Contract inventory

- [x] **Map decision sites**
  - Inventory `expression/absence.rs`, `compose/directive_targets.rs`,
    `shell_expansion/template/{candidate,typing,fill}.rs`, Claudine
    `composition/preflight.rs`, and `composition/sequence/preflight/`.
  - Trace their public construction paths, runtime dispatch, late-value caches,
    task-start barriers, and DMLS overlay/provider callers. Record concrete
    names for the source guard and a single owner for every scope extension.
  - Confirm the shell-template prerequisite APIs and lifecycle owner before
    editing; preserve unrelated existing worktree changes.
- [x] **Define scope contract**
  - Record the proposed module/API, shared declared projection, binding IDs,
    stable constant-member/index subjects, source provenance, parent sharing,
    polarity, joins, and write invalidation. `TypeScope` has no `Default`;
    unguarded entry points explicitly call `TypeScope::unguarded(..)`.
  - Identify the existing canonical predicate registry and descriptor/binding
    availability APIs. Fix the scope/approval adapter contract before agents
    touch consumer modules. All static checking and shell construction require
    a scope; runtime error owners retain their existing public enums.
- [x] **Map acceptance evidence**
  - Assign specification criteria 1–14 to test owners and public result
    assertions. Define an exhaustive site enum and applicable guard/site cases
    for Phase 5, including explicit unsupported/planned cases.
  - Choose shared fixtures and test-data ownership without a production-only
    abstraction for tests. Freeze the motivating prompt under
    `tests/fixtures/frozen_prompts/`; never read mutable internal prompts in CI.

**Checkpoint:** Every current consumer and runtime boundary has an owner; API
and test seams are recorded in the implementation log. No unresolved decision
affects this fix's correctness; the conversion ruling is explicitly future work.

## Phase 2 — Implement the Shared Analysis Core

### Wave 2 — Independent core foundations (parallel)

- [x] **Extend subtype relation**
  - In `markdown/schemas/simplified/subtype.rs`, add scalar-literal families,
    string-literal membership in enums, enum subsets, and source/destination
    union coverage independent of arm order.
  - Preserve bounded reasoning, alphanumeric identity, always-text rules, and
    existing marker/constraint treatment. Unknown is not proof of a concrete
    type; numeric `literal(4)` is not a subtype of string `enum('4')`.
  - Add focused relation tests, including all four structural examples in
    acceptance criterion 5 and existing shell-rule compatibility regressions.
- [x] **Build scope model**
  - Add the Darkmatter-owned immutable scope and shared declared-type layer.
    Reuse existing presence calculation, reserved-root descriptors, and binding
    resolution; do not rebuild effective schemas per hole.
  - Represent non-null unknown facts without adding a schema type. Retain
    constraints and source/owner identity; unavailable globals stay unavailable.
  - Test optional versus required declarations, root shadowing, bare/`fm`
    identity, constant paths/indices, unknown projections, and shared parents.
  - This task owns new scope/projection files; the subtype task owns the
    structural relation. Integrate through one agreed adapter after the wave.

### Wave 3 — Branch algebra (depends on Wave 2)

- [x] **Implement guard facts**
  - Implement `assume(&Expr, Polarity)` as the sole fact producer for truthiness,
    negation, parentheses, current predicates, equality, and representable
    discriminants. Resolve aliases and case through the canonical registry.
  - Give `&&`, `||`, and ternaries the specified operand scopes and route joins.
    Use existing expression dialects; do not change truthiness, fallback return
    values, or equality evaluation.
  - Differentially test scalar coercive equality, null/empty string, both-null,
    JSON-text container comparisons, reversed operands, both-side unions,
    numeric formatting, conservative exclusions, and contradictions against
    the real evaluator. Keep unsupported parent projections unchanged.
- [x] **Implement fact invalidation**
  - Add persistent invalidation/join operations and write-set propagation for
    static destinations, aliases, ancestors/descendants, and dynamic writes.
    Determine which effects can replace document bindings; require write sets
    or invalidate all document facts.
  - Keep mutation analysis passive. Test nested joins and restoration of only
    unaffected incoming facts; never propagate newly learned facts past a
    region or model nonreturning controls to waive analysis.
  - Execute after guard-fact edits if the two tasks touch the same core files;
    Wave 3 is a serial wave, not a proposed parallel split.

**Checkpoint:** The shared core passes its focused L1 tests; subtype checks
explicitly account for scope nullability at their adapter. `file_exists` analysis
uses metadata only, and no core path evaluates expressions or performs I/O.

## Phase 3 — Preserve Shell Authorization Boundaries

### Wave 4 — Shell construction and filling (parallel by module)

- [x] **Scope shell candidates**
  - Make candidate construction and `typing.rs` projection require a scope.
    Read narrowed types there; apply broad-string approval fallback afterward,
    retaining original validators and guard refinement provenance.
  - Add site/expression-specific conditional deferral metadata. Missing guarded
    holes retain authored expressions and require templates; missing unrelated
    or unguarded holes and actual nulls under declarations still fail discovery.
  - Preserve saved string-like rule syntax, non-string broad-string approval,
    unavailable-binding errors, grammar refusal, variable-program refusal,
    blacklist precedence, and offered/closed approval membership.
- [x] **Validate live holes**
  - Extend `fill.rs` and runtime validation to reject null and validate both
    declaration and approved refinement before conversion/dispatch. Use the
    once-resolved checked value for every argument position it fills.
  - Keep conditional holes out of unconditional `resolve_late_values` sweeps
    and cached step-entry values. Preserve ordinary deferred-hole snapshots
    and existing task-start validation behavior.
  - Test present-at-discovery values changing type, invalid original constraints,
    null after selection, whole-chain refusal, and exact/prefix/template/automatic
    paths. Missing remains `MissingValue`; incompatible values retain typed
    template-value refusal and cannot be suppressed by recovery or `no_error`.
  - Candidate and filling tasks coordinate shared timing/placeholder definitions
    through the Wave 1 contract; assign those definitions to one owner.

**Checkpoint:** Shared template API tests prove conditional deferral is local,
invalid live values launch no segment, saved approvals retain their grammar,
and ordinary late values still use the original barrier. Both API changes are
integrated before consumer work begins.

## Phase 4 — Integrate Every Current Consumer

### Wave 5 — Consumer tracks (parallel by package/module)

- [ ] **Integrate document scopes**
  - In Darkmatter compose, thread true-side body-block scopes through body
    shells, shell blocks, and transclusion targets; nest scopes lexically.
    Route standalone frontmatter shell values through an explicit unguarded
    scope. A later read of a frontmatter property grants it no earlier guard.
  - Replace `narrowed_expression_paths` and competing static target classifiers
    with scope results; preserve existing descriptor/concrete-value behavior
    and typed unavailable-binding failures.
  - Replace `AbsenceScope`'s private branch facts with shared facts while keeping
    a separate intentional-absence warning policy for fallbacks/predicate
    arguments. Preserve warning identity and actual evaluation reachability.
    Sibling paths and false ternary branches must not inherit true-side facts.
- [ ] **Integrate host scopes**
  - In Claudine preflight, replace the flattened lifecycle action walk where
    necessary with a scope-carrying traversal: explicit `then`, `else`, inline
    guarded actions, five conditional levels, and ordered `set` invalidation.
    Check each `set` RHS under the incoming scope before invalidating writes.
  - Apply scopes to lifecycle shell commands and whole-value shell `set` values,
    including secondary command sites discovered by the existing audit. Discover
    both branches without evaluating guards, predicates, or mutations.
  - Apply task `when` scopes to the sequence-owned shell and `with` expressions
    using pre-`with` bindings. Caller guards never narrow a task document's body.
    Remove independent static decisions in both preflight modules.
  - Thread conditional metadata through lifecycle templates, sequence prepared
    sites, document late-value probes, and reached-action dispatch. False
    branches skip conditional checks; ordinary late-value barriers remain.
  - Start fresh at each event/re-entry boundary and keep prior action effects
    when a later selected action fails. Preserve authored command identity
    through rereads, retries, and closed approval settlement.
- [ ] **Integrate editor scopes**
  - In DMLS expression overlays and hover/completion/diagnostic providers, build
    lexical scopes from passive source regions and call the same library.
    Carry spans through frontmatter scalar projection and nested expressions.
  - Support the current body and frontmatter/lifecycle/task sites identified in
    Phase 1; record exact capability limits instead of inventing host or nested
    union projections. No duplicate predicate registry or branch walker that
    independently decides facts.
  - Compare supported sites with library types/diagnostics. Completion must
    respect narrowed alternatives; hover shows the local type. Analyze both
    branches statically without claiming runtime warnings for unread branches.

**Checkpoint:** All current static consumers require the scope and compile
against the integrated APIs. Focused public composition/preflight and passive
DMLS tests pass. New scope construction, joins, and editor analysis remain
effect-free. Each track reviews comments for behavior drift in its own edits.

## Phase 5 — Prove Conformance and Prevent Bypasses

### Wave 6 — Test and enforcement tracks (parallel authorship)

- [ ] **Complete conformance matrix**
  - Implement the exhaustive site enum and a compact reusable case table:
    body `::shell`, `::shell-block`, transclusion target, frontmatter `$()`,
    lifecycle shell, inline lifecycle guard, whole-value lifecycle `set` shell,
    sequence task shell, and sequence `with` values. Each site has explicit
    applicable/inapplicable entries; frontmatter does not acquire lifecycle
    guards. Mark future projections planned rather than passing them vacuously.
  - Cover true/false/negated guards, nested guards, logical operand scopes and
    joins, aliases, predicate aliases, optional declarations, unknown values,
    contradictory guards, and mutation invalidation. Reuse the core algebra
    tests; site cases prove wiring and public outcomes rather than duplicating
    every internal case at every site.
  - Run shared applicable cases through public composition/preflight APIs and
    DMLS analysis, asserting type, location, diagnostics, and availability.
    Assert passive `file_exists` analysis invokes no filesystem/network handler.
- [ ] **Guard analysis ownership**
  - Add a source guard auditing the named static decision functions and scope
    construction sites inventoried in Phase 1, including both Claudine
    preflight modules. Reject competing static classifiers and unreviewed
    `TypeScope::unguarded` construction.
  - Require a reason for each allowlisted live site and reject stale entries.
    Permit runtime `is_null`, predicate implementations, and warning-policy
    absence handling. Prove the guard rejects representative forbidden snippets
    and permits legitimate runtime validation without editing production files.
- [ ] **Prove invocation behavior**
  - Through hermetic L1 command builders, run the frozen motivating prompt with
    a recording fake executable. Prove absent-message template preflight,
    false-guard no launch, and selected-message exact single-argument delivery.
    Never create a real commit or launch a real provider.
  - Prove unguarded/mismatched-property missing values still fail, declarations
    cannot hide actual nulls, selected invalid chains launch nothing, and prior
    actions retain effects. Cover null/type-changing `set`, aliases and nested
    writes, plus skipped conditional versus ordinary task-start late barriers.
  - Reuse existing exact/prefix/template, blacklist, argument-boundary, recovery,
    initialization-shell, and approval-membership regressions. Add cases only
    for newly crossed boundaries. Use child-local silent audio and isolated
    homes/PATHs; use portable fake executables and archive-safe fixture paths.

**Checkpoint:** The acceptance-to-evidence map has no uncovered current case.
The source guard catches bypasses, DMLS agrees at supported sites, and command
recordings prove no launch on refusal or skipped guards. New test files are
registered in their consolidated L1 targets. Matrix findings include the state
of sibling cases for the affected guard/site boundary.

## Phase 6 — Document and Validate Delivery

### Wave 7 — Documentation tracks (parallel)

- [ ] **Document Darkmatter behavior**
  - Update expression and shell-approval topic pages plus applicable DMLS docs
    and Darkmatter skills. Describe both guard sides, equality caveats,
    declaration/refinement/fallback separation, live checks, and passive
    analysis. Include compact guarded/unguarded examples and a scope-to-dispatch
    Mermaid flow; label future argument-binding/projection behavior planned.
  - Keep README changes limited to public overview drift. Do not add spec paths
    or feature/fix names to current docs.
- [ ] **Document Claudine behavior**
  - Update composition/preflight, lifecycle, and sequence topic pages and
    Claudine skills with conditional validation, task-guard bindings, mutation
    invalidation, and re-entry lifetimes. Explain why an unselected branch skips
    its conditional checks while ordinary sequence late values retain a barrier.
  - State the current strict-handler `file_exists` fact and future conversion
    limitation without implying filesystem guarantees. Update implementation
    evidence and departures in the implementation log; leave the spec snapshot
    and unrelated files unchanged.

### Wave 8 — Final verification (serial)

- [ ] **Run affected checks**
  - Run the canonical `just test` in `darkmatter/` and `claudine/` after all
    branches integrate; these recipes cover the relevant library, CLI, and DMLS
    L1 targets. Run each area's `just lint` separately afterward. Do not run
    `cargo fmt`, `lint-fix`, or `cargo test`; report pre-existing failures
    separately from regressions.
  - Run `just check-tier-coverage darkmatter` and
    `just check-tier-coverage claudine` from the repository root, sequentially
    after tests/lint so feature changes cannot overwrite a running test binary.
    Scope any necessary follow-up reruns to changed or failing tests.
  - Require portable behavior on macOS, Linux, native Windows, and WSL2 through
    the repository's existing environment/evidence policy. No added CI cell,
    terminal/browser tier, live provider test, or all-OS run is required by this
    static-analysis fix. Load the `os` skill before platform-specific changes
    or environment-specific failure handling; do not claim unrun environments
    were locally verified.
- [ ] **Audit delivery evidence**
  - Review the diff for residual private fact producers, unreviewed unguarded
    construction, broad root deferrals, cached discovery empties, branch leaks,
    unconditional conditional-hole barriers, and unsupported behavior claims.
    Check docs/comments against behavior and log any resolved drift.
  - Record acceptance criteria 1–14 with test identities/results, the planned
    future cases, final command outcomes, and any remaining author ruling.
    Report implementation complete, ready for review only when the checkpoints
    pass. Leave lifecycle directories in place; do not run `just complete` or
    commit unless separately instructed.

**Checkpoint:** Current acceptance cases and affected checks pass, documentation
describes implemented behavior, and review evidence clearly separates this fix
from future pipeline work. The author's review closes the implementation cycle.
