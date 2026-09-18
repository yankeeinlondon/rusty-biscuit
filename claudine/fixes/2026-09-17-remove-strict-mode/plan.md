---
total_phases: 7
created: 2026-09-17
phase: 1
agent: "codex/gpt-5.6-sol"
yolo: "true"
---

# Remove Strict Mode and Centralize Expression Binding — Implementation Plan

## Work Summary and Definition of Success

This plan replaces Darkmatter's split strict/lenient subtree behavior with one
expression contract, then moves lifecycle-global availability, schema feature
restrictions, and editor diagnostics onto shared Darkmatter-owned primitives.
The work spans `darkmatter`, `darkmatter-cli`, `dmls`, `claudine`,
`claudine-cli`, and `claudine-gen`: Darkmatter owns binding classification and
passive validation; Claudine supplies lifecycle policy and retains typed causes;
DMLS consumes the same schema and binding descriptors without depending on
Claudine; and authoritative YAML schemas move to package-owned `schemas/`
directories with deterministic runtime generation.

Successful completion is observable at every boundary:

- `SubtreeStrictness`, `.strict()`, `with_strictness`, strict-root validation,
  and the strictness argument to `compose_subtree` no longer exist. Subtree
  interpolation is always fail-fast for real parser/evaluator failures while
  missing document properties evaluate normally as `null`.
- A Darkmatter binding environment distinguishes a missing document property,
  an available eager or memoized-lazy global (including a real `null` value),
  an unavailable reserved global with a typed reason, and the reserved
  `doc`/`ctx`/`env` namespaces. The same declarations drive passive all-branch
  validation and runtime evaluation without invoking lazy providers during
  validation.
- Bare identifiers resolve only through document state; they never fall
  through to a same-named `ctx` value. Explicit globals shadow document roots,
  unavailable globals never fall through, and `doc.<name>` remains the escape
  hatch for colliding document properties.
- Claudine contains one lifecycle-global catalog and event/scope availability
  matrix. Event-time interpolation, lifecycle shell preflight, sequence shell
  preflight, and sequence task values all use Darkmatter's common API; the old
  undefined-variable, `err`, and surviving-span AST walkers are removed.
- Lifecycle messages, `proxy.with`, and batched `set` remain atomic; shell
  approval still compares the exact resolved bytes; genuine failures stop the
  affected operation before its effect; and Claudine library callers can
  inspect both lifecycle context and the original typed Darkmatter cause.
- `no-shell-expansion` is a first-class SimplifiedSchema constraint. It
  inherits through arrays, key/value containers, imports, merges, and deferred
  evaluation, cannot be relaxed by descendants, and blocks `initialize`
  expansion before execution while the existing runtime shell backstop remains.
- `darkmatter/schemas` and `claudine/schemas` are the sole editable schema
  sources. `claudine-gen` deterministically produces the runtime embedding and
  drift-checks it without depending on `claudine` or `claudine-cli`; DMLS loads
  the same source format dynamically rather than compiling Claudine data.
- DMLS always includes the Darkmatter base schema, supports direct and
  conditional schema inputs, discovers scope-local `schemas/` directories and
  additive `SCHEMA_DIR`, refreshes on create/update/delete and marker changes,
  and diagnoses unavailable globals/feature violations through passive
  Darkmatter validation. An undeclared bare property gets at most one advisory
  stating that it is valid, unknown-typed, and runtime-null unless supplied.
- The literal `prompts/implement.md` regression reaches its authored routing
  error with no provider process or audio publication and without changing the
  prompt to add fallback guards.
- Darkmatter and Claudine Level 1 tests and lint pass; existing applicable L2
  suites pass; schema drift checks are part of existing verification rather
  than a new CI matrix cell; and representative DMLS refresh responsiveness is
  recorded without inventing a numeric requirement.

Planning evidence and risk shape:

- GitNexus is bound to `better-static-analysis` at
  `4399b3a4b53553ad3c89e1f4bc1dbec1aa02e6e6`, matching `HEAD`. Its current
  `EvaluationLookup` upstream walk is CRITICAL and partially enriched (6 direct
  graph edges, 38 affected indexed processes, and broad transitive fan-out), so
  the partial graph result is not treated as a complete inventory.
- Text inspection currently finds 23 Rust `EvaluationLookup` implementation
  sites including test fixtures, while the specification records 17 direct
  implementations from its earlier graph walk. Phase 1 must reconcile and
  classify every live site before changing the shared seam.
- The four production `SubtreeCompose` consumers are lifecycle event-time
  interpolation, lifecycle shell-command preflight, sequence shell preflight,
  and sequence task value resolution. The migration is one coordinated API
  cutover, not a staged compatibility period.
- Existing DMLS trigger discovery already performs bounded ancestor scanning,
  last-good transactional caching, dependency hashing, and YAML file watching.
  The planned work extends those mechanisms for scoped roots, marker facts,
  direct inputs, and `SCHEMA_DIR`; it does not introduce a second loader.

## Phase 1 — Lock Architecture and Baselines

### Necessary Rules

- **Additive binding seam.** Keep `EvaluationLookup::get` as the lightweight
  value lookup and remove only `is_known_variable_root`. Introduce a composed
  Darkmatter `BindingEnvironment` above the trait with an ordinary-lookup
  adapter where `None` means a valid missing document property. Rich global
  classification is opt-in for consumers that reserve global names; this avoids
  forcing unrelated lookup implementations to duplicate lifecycle semantics.
- **One binding declaration.** Model document properties, reserved namespaces,
  available eager globals, available memoized-lazy globals, and unavailable
  globals explicitly. An unavailable-global record carries stable structured
  reason data; runtime resolution and passive validation consume the same
  declaration object. Validation walks every reference, including inactive
  branches, but never invokes a lazy provider.
- **Shared mechanics, domain policy retained.** Darkmatter owns parsing,
  traversal, precedence, binding resolution, and typed binding errors.
  Claudine owns the names, event/scope availability, values, and capture timing
  of `err`, `timing`, `current`, `current_env`, and sequence-only `group`.
  `current` and `current_env` are independent lazy globals in the final catalog;
  the implementation removes the transitional `current.ctx`/`current.env`
  projection so code, descriptors, and current docs agree.
- **Prepared validation boundary.** Expose a reusable parser-aware binding
  validation operation and reuse the resulting parsed expression where a
  caller immediately evaluates the same source. Do not create a broad prepared
  evaluator across sources with different parse modes, schema locations, or
  snapshot timing; those differences are intentional policy, not duplication.
- **Document namespace rule.** `EffectiveState::get` stops after document
  lookup for bare roots. `ctx.*`, `env.*`, and `doc.*` remain explicit reserved
  namespaces, and each non-EffectiveState lookup is audited so no matching
  context-tail or ambient map fallback recreates the removed behavior.
- **Fail-fast means evaluator failure.** The single subtree API uses the
  existing interpolation machinery in fail-fast form for malformed mixed
  strings, unknown functions, rejected arguments, and file-operation errors.
  Missing document data is successful `null` evaluation and never a permission
  check. `||` remains value selection, not identifier legalization.
- **Literal output is complete data.** Triple braces, `\{{ ... }}` and
  `\{\{ ... }}` retain ordinary one-pass Darkmatter escaping. Claudine must not
  reject a successful result merely because it contains expression-looking
  braces, and must not add another interpolation pass.
- **Feature policy is monotonic.** Represent `no-shell-expansion` in shared
  schema metadata and propagate it downward through arrays, mapping values,
  imported named types, merged baselines/triggers, and deferred values. Once
  inherited, no descendant schema or higher-precedence overlay may relax it.
  `initialize` uses the existing narrow `lifecycle-event` import with this
  constraint; a generic `object` replacement is forbidden.
- **Schema source and generation boundary.** Move the settled file inventory
  exactly as specified. Add deterministic schema generation to `claudine-gen`,
  which may depend on Darkmatter but never on `claudine` or `claudine-cli`.
  Emit one generated Rust module under `claudine/lib/src/composition/` containing
  normalized source text and parsed descriptor data; both generate and check
  paths cover it, and no second hand-authored runtime schema is retained.
- **Neutral descriptor format.** Add a generic Darkmatter-owned binding-catalog
  envelope referenced by trigger/direct schema metadata. Claudine authors its
  lifecycle descriptors beside `claudine.yaml`; `claudine-gen` consumes that
  file for runtime declarations and DMLS loads it dynamically through
  Darkmatter. DMLS contains no Claudine names, event table, or package
  dependency.
- **Generic activation.** Extend trigger matching with a generic
  workspace-marker predicate and use `.claudine/config.json` or
  `.claudine/config.json5` in Claudine-authored trigger data. Automatic schema
  roots remain document-scope-local; `SCHEMA_DIR` is an additive,
  workspace-wide source whose triggers still must match and whose relative
  imports remain source-relative.
- **Diagnostic rename.** Replace `dm.expression.unknown_identifier` with one
  advisory `dm.expression.undeclared_document_property`; no compatibility alias
  is retained because there are no established external users. Unknown
  functions, unavailable globals, and schema feature violations remain distinct
  hard diagnostics.
- **No prompt workaround.** Preserve the reported ternary in
  `prompts/implement.md`; do not add `|| false`, null placeholders, or schema
  materialization. Audit existing fallbacks and remove only those documented by
  evidence as legality workarounds, retaining semantic defaults.
- **No separate spike.** The specification's risk review is accepted. Phase 1
  records inventories and failing-before fixtures; reopen a spike only if those
  checks disprove the selected binding or schema-generation seams.
- **Cross-platform paths and watching.** All discovery, environment parsing,
  marker matching, generated paths, and tests use `Path`/`PathBuf`, accept
  native Windows separators where input is textual, preserve case-collision
  checks, and avoid symlink-dependent behavior.

### Wave 1 — Contract Evidence (parallel)

- [ ] **Lookup Inventory**
  - Reconcile the specification's 17-site graph inventory with every live
    `EvaluationLookup` implementation found in Darkmatter and Claudine,
    separating production adapters, test fixtures, doctests, and unrelated
    lookup surfaces.
  - For each production implementation, record its document/context/env/global
    precedence, missing-value behavior, resolution-context support, and whether
    it needs only the ordinary adapter or a rich binding environment.
  - Run focused GitNexus context/impact queries for the concrete binding and
    subtree symbols before implementation; treat HIGH/CRITICAL or partial
    results as explicit migration/review gates and confirm unresolved callers
    with `rg`.

- [ ] **Consumer Inventory**
  - Record the four production `SubtreeCompose` sites and all direct `evaluate`
    paths in lifecycle, loop, dispatch, sequence, and Darkmatter composition.
  - Map each caller's parse mode, state snapshot, resolution context, schema
    location, source/property/task identity, atomicity boundary, and current
    error wrapper so the cutover cannot flatten diagnostics or alter capture
    timing.
  - Inventory Claudine AST walkers for undefined roots, lifecycle-global
    availability, shell late-binding, and surviving spans; classify each as
    removable expression mechanics or retained domain policy.

- [ ] **Schema Inventory**
  - Resolve every reference to the files beneath `darkmatter/docs/schemas`, the
    empty `schema-definition.yaml`, and draft/empty files beneath
    `claudine/docs/schemas`; document move, delete, or intentional historical
    dispositions without broad path replacement.
  - Trace Darkmatter base embedding, DMLS configured extensions, trigger
    discovery/cache/watch invalidation, Claudine runtime schema use, and
    `claudine-gen` generate/check application paths.
  - Capture the current trigger precedence, package-scope behavior, named-type
    import origin, and last-good refresh tests as regression baselines.

### Wave 2 — Regression Baselines (parallel)

- [ ] **Semantic Fixtures**
  - Add failing-before characterization cases for absent bare/document paths,
    schema-declared-but-unset values, bare-vs-`ctx` collision, global shadowing,
    unavailable-vs-null globals, inactive branches, malformed expressions, and
    all three literal escape forms.
  - Preserve whole-value and mixed-string fixtures and the real
    `prompts/implement.md` ternary so tests compare the new behavior with
    ordinary Darkmatter semantics rather than a rewritten prompt.
  - Retain or replace the fatality matrix around genuine parser/evaluator
    failures; do not encode root membership as fatality.

- [ ] **DRY Audit**
  - Record the ownership, inputs, outputs, snapshot timing, and errors for
    effective-state construction, early/late contexts, global declarations,
    parsing, validation, evaluation, interpolation, descriptors, and error
    projection.
  - Select the shared `BindingEnvironment` plus passive validation operation as
    the consolidation seam; list each intentionally retained duplicate where
    Claudine policy, DMLS presentation, parse mode, or capture timing differs.
  - Verify the seam can preserve lazy memoization and typed causes before any
    public API removal begins.

### Phase 1 Checkpoint

- [ ] **Architecture Review**
  - The complete lookup/consumer/schema inventories, failing-before fixtures,
    global availability matrix, DRY audit, and generator artifact layout are
    recorded and internally consistent.
  - No implementation starts while a CRITICAL/HIGH caller is unclassified, a
    schema file has two proposed authorities, or the selected binding model
    collapses absent document data, null globals, and unavailable globals.

## Phase 2 — Build Darkmatter Binding Semantics

### Wave 3 — Binding Foundation

- [ ] **Binding Model**
  - Add the public Darkmatter binding descriptors/environment and typed
    unavailable-global error, with eager and memoized-lazy global providers and
    explicit reserved namespaces.
  - Provide the ordinary `EvaluationLookup` adapter so existing lightweight
    implementations map `None` to a missing document property without an
    invasive trait rewrite; remove `is_known_variable_root` and its docs.
  - Route variable evaluation through the binding environment while preserving
    member/index null propagation, global precedence, read-side function
    resolution contexts, and lazy-provider at-most-once behavior.

### Wave 4 — Binding Consumers (parallel)

- [ ] **Passive Validator**
  - Add parser-aware validation that visits every variable reference in all
    branches, classifies it through the same environment as runtime, and reports
    definitely unavailable globals without reading their values.
  - Represent execution-dependent availability explicitly so passive validation
    defers it and runtime resolution enforces it; prove validation never invokes
    lazy providers, filesystem functions, shell, or other effects.
  - Return structured errors with reference/binding details suitable for
    Claudine context wrapping and DMLS source-range diagnostics.

- [ ] **Namespace Isolation**
  - Remove `EffectiveState`'s bare-name fallback to `get_context_value`; retain
    explicit `ctx.*`, `env.*`, and `doc.*` paths and document-property collision
    access through `doc`.
  - Audit and migrate every production/test `EvaluationLookup` implementation,
    adding parity coverage for conditions, loops, dispatch, frontmatter seeds,
    catalogs, filesystem functions, and non-Claudine fixtures.
  - Add a guard/inventory test that fails when a new implementation introduces
    implicit context-tail fallback or bypasses the ordinary/rich adapter choice.

### Wave 5 — Unified Evaluation

- [ ] **Subtree Unification**
  - Remove `SubtreeStrictness`, builder selectors, convenience arguments,
    `validate_strict_roots`, `collect_variable_roots`, and strictness-specific
    documentation/tests.
  - Make the one subtree path use fail-fast interpolation for genuine failures
    while accepting missing document properties as `null` in whole values,
    empty text in mixed strings, falsy ternaries, and fallback value selection.
  - Preserve triple-brace/backslash escapes and single-pass literal output with
    no surviving-span rejection or additional evaluation pass.

### Phase 2 Checkpoint

- [ ] **Binding Gate**
  - Run focused Darkmatter nextest targets for expression, subtree,
    interpolation, condition, and lookup-adapter contracts.
  - Confirm absent and schema-declared-but-unset properties, unavailable and
    null globals, inactive-branch validation, literal parity, genuine failures,
    and non-Claudine lookup behavior match the Phase 1 matrix before consumers
    migrate.

## Phase 3 — Enforce Schema Feature Policy

### Wave 6 — Constraint Grammar

- [ ] **Constraint Syntax**
  - Add `no-shell-expansion` to `Constraint`, postfix grammar, native mapping
    `$constraints`, serializer, source spans, descriptors/completion, semantic
    compatibility checks, and documentation.
  - Preserve constrained named-import syntax such as
    `lifecycle-event(no-shell-expansion)@./claudine-types.yaml`; correct current
    docs that claim native mapping schema forms are unsupported.
  - Reject invalid placements or attempts to negate/relax the flag with typed
    schema diagnostics.

### Wave 7 — Policy Metadata

- [ ] **Policy Metadata**
  - Carry the restriction in effective-schema metadata independently of JSON
    value validation so composition and DMLS can inspect the applicable policy
    at an exact schema location.
  - Define monotonic merge/import rules and origin attribution for direct
    baselines, triggers, document schemas, named types, unions, arrays, and
    key/value containers.
  - Keep generic feature vocabulary in Darkmatter; no lifecycle event name or
    Claudine-specific rule enters the grammar.

### Wave 8 — Deferred Enforcement

- [ ] **Expansion Guard**
  - Enforce the location policy during preparation before shell expansion and
    carry it with deferred values for evaluation-time enforcement when content
    was unavailable earlier.
  - Ensure a failed required-schema, binding, or feature-policy validation
    blocks the affected operation rather than continuing as an advisory, while
    preserving existing recovery routing and completed prior effects.
  - Prove restricted arrays/mappings cover every descendant and cannot be
    relaxed; generated, overlaid, proxied, mutated, and moved values retain the
    applicable destination/source policy defined by the effective schema.

### Wave 9 — Initialize Enforcement

- [ ] **Initialize Schema**
  - Apply `no-shell-expansion` to the narrow imported lifecycle-event type for
    `initialize`; retain its explicit communication/stack key structure.
  - Keep Claudine's runtime preflight-boundary shell backstop and bootstrap
    frontmatter prohibition as defense in depth, including dead branches,
    failure/catch/finalize routes, approvals, whitelists, caches, and
    `no_error` attempts.
  - Add Darkmatter tests for blocked expansion and Claudine-facing fixtures for
    all preflight-shell regressions.

### Phase 3 Checkpoint

- [ ] **Policy Gate**
  - Run focused SimplifiedSchema grammar, import, merge, source-map,
    composition, and DMLS metadata tests.
  - Confirm validation is passive, blocking, location-specific, inherited,
    non-relaxable, and preserved through deferred evaluation before lifecycle
    schemas adopt it.

## Phase 4 — Centralize Claudine Lifecycle Bindings

### Wave 10 — Catalog and Errors (parallel)

- [ ] **Global Catalog**
  - Replace omission-based `lifecycle_injected_globals` with one Claudine
    catalog declaring `err`, `timing`, `current`, `current_env`, and `group`,
    including types, eager/lazy capture, event/scope availability, conditional
    runtime availability, and structured unavailable reasons.
  - Reserve every catalog name even when unavailable so bare access cannot fall
    through; preserve explicit `doc.<name>` access and global shadowing.
  - Exercise every global against every lifecycle event/scope through
    Darkmatter validation and runtime APIs, including sequence groups and
    conditional `finalize` error state.

- [ ] **Typed Context**
  - Extend lifecycle composition errors/wrappers so event, action, property,
    task, and source-path context surrounds—rather than stringifies—the original
    typed Darkmatter error.
  - Remove prose-only undefined-variable construction and the
    `LifecycleUndefinedVariable` variant/rendering/snapshots; map unavailable
    global and feature-policy errors to stable Claudine diagnostics without
    losing their typed source.
  - Add library-only downcast/source-chain tests independent of CLI rendering.

### Wave 11 — Consumer Cutover (parallel)

- [ ] **Event Evaluation**
  - Remove `first_undefined_stack_variable` calls from `when_matches`,
    `render_message`, and `resolve_typed_value`, plus the helper/root walkers and
    closed-world preparation validator when no producer remains.
  - Replace `validate_no_err_in_no_error_events` traversal with a thin adapter
    that builds the event binding environment, invokes Darkmatter passive
    validation, and adds lifecycle location context; remove the adapter entirely
    if normal preparation can supply that context directly.
  - Remove shallow/deep surviving-span guards while retaining atomic full-value
    resolution, typed whole values, ordinary mixed interpolation, and genuine
    evaluation failure before dispatch/mutation.

- [ ] **Preflight Cutover**
  - Migrate lifecycle shell preflight and sequence shell preflight from custom
    root walkers to explicit unavailable-global declarations plus Darkmatter
    validation/evaluation.
  - Preserve source/property/task diagnostics, resolution contexts, no-effect
    preflight behavior, exact resolved command bytes, and approval byte parity.
  - Keep target-identity and other genuinely domain-specific sequence checks
    only where they are not expression-binding mechanics; document that
    ownership in the DRY audit.

- [ ] **Task Cutover**
  - Migrate sequence task value resolution and all production subtree callers
    to the single API in the same coordinated change; remove every `.strict()`
    and old convenience signature.
  - Verify `proxy.with`, mapping-based `set`, communication batches, and group
    task values resolve completely before mutation or dispatch and treat missing
    properties as successful `null` values.
  - Update interpolation conformance tests to use shared Darkmatter/lifecycle
    fixtures for escapes, whole values, mixed strings, missing values, and
    genuine failures.

### Phase 4 Checkpoint

- [ ] **Lifecycle Gate**
  - Run focused Claudine library tests for every former undefined-variable call
    shape, the complete global matrix, shell restrictions, atomic mutation,
    typed causes, literal parity, and recovery routes.
  - Confirm no Claudine production code parses or walks expression ASTs solely
    to classify variable roots, reproduce short-circuit behavior, or reject
    successful expression-looking output.

## Phase 5 — Establish Authoritative Schema Distribution

### Wave 12 — Source Migration

- [ ] **Schema Move**
  - Move `darkmatter.yaml` and `expression-functions.yaml` to
    `darkmatter/schemas`; move populated `claudine.yaml`,
    `claudine-types.yaml`, `err.yaml`, and `env.yaml` to `claudine/schemas` while
    preserving `claudine/schemas/review.yaml`.
  - Keep `darkmatter-schema.md` as documentation with corrected transclusion;
    remove or retire the empty `schema-definition.yaml` reference after proving
    no functional consumer; prevent empty/draft `claudine/docs/schemas` files
    from becoming authorities.
  - Update imports, fixtures, loaders, documentation links, tests, and skill
    paths individually, preserving source-relative named imports and avoiding
    unrelated historical records.

### Wave 13 — Distribution Inputs (parallel)

- [ ] **Trigger Data**
  - Author generic Claudine trigger data and the lifecycle binding descriptor
    catalog beside the Claudine schemas, using workspace-marker facts and
    ordinary frontmatter/path predicates rather than DMLS code.
  - Ensure automatic activation remains scoped to documents beneath the schema
    source root; cover wider intentional use through `SCHEMA_DIR` or a root
    schema installation, never sibling leakage.
  - Validate the descriptor catalog and trigger envelopes through Darkmatter's
    standalone schema APIs before either runtime generation or editor loading.

- [ ] **Base Embedding**
  - Repoint Darkmatter's base and expression-function embedding at
    `darkmatter/schemas`, update transclusion/build paths, and retain one parsed
    cache rather than adding generation where the specification does not
    require it.
  - Add source-location and line-ending portability tests for macOS, Linux, and
    Windows compilation, using repository-relative `/` in `include_str!` paths
    and normalized generated output.
  - Add an inventory guard preventing reintroduction of competing live schema
    copies under documentation directories.

### Wave 14 — Runtime Generation

- [ ] **Schema Generator**
  - Add dedicated `claudine-gen schemas generate` and `schemas check` paths for
    deterministic schema parsing/normalization and emission of the Claudine
    runtime schema/binding module without adding a dependency on `claudine` or
    `claudine-cli`.
  - Integrate the schema artifact into generator reporting, unit fixtures, and
    the existing package verification recipes; byte-compare committed output so
    source/artifact drift fails existing gates without coupling it to a
    provider-only slug selection.
  - Make the Claudine library consume only the generated module and prove the
    embedded schema/descriptors are semantically equivalent to the YAML inputs.

### Phase 5 Checkpoint

- [ ] **Authority Gate**
  - Run `claudine-gen` check/generation tests and Darkmatter shipped-schema/meta
    schema tests from a clean generated state.
  - Confirm editing authoritative YAML changes both generated runtime drift and
    DMLS-loaded semantics, while DMLS itself contains no embedded Claudine
    schema or lifecycle catalog.

## Phase 6 — Complete DMLS Static Analysis and Discovery

### Wave 15 — Editor and Discovery Foundations (parallel)

- [ ] **Binding Diagnostics**
  - Feed Darkmatter binding descriptors and passive validation results into
    expression diagnostics, hover, and completion for body and schema-typed
    frontmatter surfaces.
  - Rename the advisory code/message to undeclared document property, emit at
    most one warning per reference, use schema type information for declared
    but unset properties, and keep bare identifiers classified as document
    properties rather than `ctx` tails.
  - Emit hard, source-ranged diagnostics for unavailable lifecycle globals and
    schema feature violations—including inactive branches—while deferring
    execution-dependent availability and keeping unknown functions distinct.

- [ ] **Scoped Roots**
  - Extend the existing trigger registry to compose repository/open-tree,
    package-area, and package `schemas/` roots with explicit scope ownership;
    nearest definitions retain deterministic precedence without affecting
    sibling scopes.
  - Add generic direct always-on schema inputs alongside conditional triggers,
    always layering them over the Darkmatter base through the same effective
    schema engine.
  - Detect generic workspace marker facts at the applicable root and feed them
    to trigger matching without a Claudine-specific filename in DMLS.

- [ ] **Environment Source**
  - Read a valid `SCHEMA_DIR` as an additive workspace-wide schema source,
    preserve trigger conditions and source-relative imports, and allow paths
    outside the workspace or inside another package.
  - Define process-start/config-reload environment timing and diagnose invalid
    values without replacing automatic discovery.
  - Prove a normally scope-local source can apply elsewhere only when selected
    through `SCHEMA_DIR` and its trigger matches.

### Wave 16 — Editor and Refresh Proof (parallel)

- [ ] **Passive Proof**
  - Test editor validation with effect-counting lazy providers and filesystem/
    shell sentinels to prove it executes no action, expansion, read-side
    operation, or provider.
  - Compare DMLS results directly with Darkmatter preparation validation for the
    same schemas, binding catalogs, inactive branches, and feature policies.
  - Migrate diagnostic documentation, suppression fixtures, severity tests, and
    mapping-only corpora atomically with the code rename.

- [ ] **Refresh Pipeline**
  - Extend existing watcher globs, server-rescan fallback, dependency hashes,
    and cache keys for marker files, direct inputs, `SCHEMA_DIR`, nested roots,
    and newly created/deleted `schemas/` directories.
  - Preserve last-good transactional behavior for malformed edits and refresh
    startup/create/update/delete state for open documents without requiring an
    editor window to gain focus.
  - Add deterministic macOS tests using temporary workspaces plus path/case
    fixtures that exercise native Windows/Linux semantics; record representative
    startup and refresh observations without a numeric pass/fail budget.

### Phase 6 Checkpoint

- [ ] **Editor Gate**
  - Run DMLS Level 1 unit/integration tests for diagnostics, hover, completion,
    base/direct/conditional layering, root scoping, marker changes,
    `SCHEMA_DIR`, imports, and watcher/rescan invalidation.
  - Confirm the editor is responsive in representative fixtures, produces no
    side effects, never embeds Claudine data, and agrees with runtime
    preparation on every shared validation case.

## Phase 7 — Regression, Documentation, and Handoff

### Wave 17 — End-to-End Proof (parallel)

- [ ] **Router Regression**
  - Add a literal shipped-prompt CLI test for the implementation router with
    only `spec` supplied and no pending review, preserving the authored
    `plan`/`review` ternaries.
  - Use `CliProcessFixture` with its hermetic CWD/home/PATH,
    `PLAYA_DRY_RUN=1`, and private spool; assert the intended identified-spec
    line and routing error, absence of unknown-root/undefined/fallback advice,
    no provider spawn, and no audio publication.
  - Keep the test at Level 1 and avoid terminal/browser focus.

- [ ] **Fallback Audit**
  - Search shipped prompts for fallbacks or synthetic null keys added solely to
    satisfy closed-world validation; remove only evidence-backed legality
    workarounds and retain real authored defaults.
  - Add prompt guards proving the reported expression remains unmodified and no
    obsolete strict-mode guidance reappears.

- [ ] **Reference Update**
  - Update Darkmatter expression/interpolation/schema documentation, Claudine
    lifecycle/composition topics, READMEs, timeline, DMLS diagnostics/editor
    docs, and all mirrored `.claude/skills/claudine` and Darkmatter skill
    references.
  - Describe missing properties as valid runtime `null`, distinguish
    unavailable globals and genuine evaluator failures, document explicit
    `ctx.*`, literal escapes, feature restrictions, schema ownership,
    generation, dynamic discovery, and `SCHEMA_DIR`.
  - Review every behavior-changing symbol's `///`/`//!` and inline comments;
    remove stale strict/unknown-root/surviving-span narration without rewriting
    unrelated historical specifications or logs.

### Wave 18 — Darkmatter Gate

- [ ] **Darkmatter Gates**
  - From `darkmatter/`, run `just test` and `just lint` with nextest-backed
    recipes; run existing relevant `just test-l2` coverage only if it exercises
    the changed DMLS/schema boundary.
  - Record focused test commands, outcomes, and representative DMLS refresh
    observations; do not run `cargo fmt` unless separately requested.

### Wave 19 — Claudine Gate

- [ ] **Claudine Gates**
  - From `claudine/`, run `just test`, `just lint`, and generator drift checks;
    run `just test-l2` only for an existing suite covering the changed
    composition boundary or an added L2 regression.
  - Verify the CLI regression remains hermetic/silent and no new CI matrix cell
    was introduced.

### Wave 20 — Graph Review

- [ ] **Graph Review**
  - Refresh GitNexus with `just gitnexus` if the index no longer matches HEAD,
    then run complete `detect-changes --scope all`; re-run any partial or
    truncated result and review every HIGH/CRITICAL affected symbol/process.
  - Re-run text inventories for strictness APIs, root walkers, old diagnostic
    terminology, old schema paths, competing schema copies, and every
    `EvaluationLookup` implementation.

### Wave 21 — Acceptance Review

- [ ] **Acceptance Review**
  - Trace each specification acceptance criterion to code, tests,
    documentation, and recorded evidence; confirm all seven phase checkpoints
    are satisfied and all plan tasks are checked.
  - Report implementation complete and ready for review without moving the fix
    to `_completed`, running `just complete`, committing, or altering unrelated
    working-tree changes.

### Final Checkpoint

- [ ] **Review Handoff**
  - Darkmatter, DMLS, Claudine library/CLI, schema generation, shipped prompt,
    docs, and skill snapshots all express the same binding and feature-policy
    contract.
  - Required package gates and applicable existing L2 suites are green, graph
    review is complete, cross-platform risks are covered by portable design and
    CI evidence, and the change is implementation-complete for author review.
