---
kind: plan
created: 2026-10-20
total_phases: 5
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Map and Filter Implementation Plan

## Phase 1 — Establish the Implementation Contract

### Necessary Rules

The work adds inline, single-parameter callbacks to the expression grammar,
lexical local bindings to evaluation and passive analysis, and catalog-backed
`map` and `filter`. Success means all eleven specification acceptance criteria
pass through their actual entry points: library composition, `md compose`,
Claudine lifecycle evaluation, shared signature checking, and DMLS. Callbacks
must preserve order, existing truthiness, request context, and typed error
provenance without becoming JSON values or running during passive validation.

The following rulings must be recorded before the affected implementation
starts. Recommendations are planning proposals, not author decisions. Since
this planning session is non-interactive, unresolved decisions remain explicit
implementation gates; `yolo: true` does not resolve them.

| Ruling | Evidence and proposed disposition | Blocks |
| --- | --- | --- |
| Shared signature prerequisite | `2026-10-28-schema-enhancements` is a hard prerequisite despite its later date. Its spec and the proposed `darkmatter/schemas/partials/functions.yaml` are absent from this checkout. Obtain the shared contract and passing prerequisite implementation before Phase 2; do not invent a temporary signature format or implement a broader schema rewrite here. Type-aware evaluation and narrowing are not prerequisites for unknown-typed locals. | Phase 2 |
| Catalog ownership | The current embedded catalog is `darkmatter/docs/schemas/expression-functions.yaml`, loaded by `expression/catalog/mod.rs`. The prerequisite must establish how this becomes or consumes the specified partial. Confirm one authoritative source and its existing consumers; preserve the specified destination rather than silently substituting the current catalog. | Phase 2 |
| Shadow advisory | Acceptance criteria require the advisory, while the open question still requests confirmation. Recommend retaining it only for a parameter hiding a declared document property, with explicit `fm.name` unchanged. Reusing an enclosing parameter alone earns no advisory. Confirm this policy before implementing it. | Phase 2 |
| Diagnostic identities | Review dedicated misplaced-lambda and forbidden-parameter errors, the shadow advisory, and callback-error context under existing diagnostic policies. Decide codes, severity, subjects, deduplication, and span ownership once; retain `dm.expression.undeclared_property` for actual undeclared document reads. Runtime wrapping must retain typed causes. | Phase 2 |
| Operational policy | Confirm expected list sizes and any iteration, nesting, cancellation, or effect limits. Recommend sequential callbacks using the enclosing expression's existing permissions, timeouts, and caches, with no new limit or effect permission introduced implicitly. Effects completed before a later item fails are not automatically rolled back. Any new budget or transactional requirement must be specified before runtime work. | Phase 3 |
| Scope and syntax | Use a callback-only argument representation; no first-class closure or `serde_json::Value` change. Share binding classification with `BindingView`, including host globals, instead of hardcoding Claudine names. Parse and check callbacks on empty/null input without executing their bodies. Preserve existing identifier and condition-mode operator rules. | Phase 2 |

### Wave 1 — Resolve prerequisites

- [x] **Confirm rulings**
  - Resolve the decisions above in `implementation-log.md` beside this plan,
    distinguishing author rulings from recommendations and recording any
    departures from the spec without rewriting its historical decisions.
  - Record the prerequisite revision and public signature APIs, registration
    rules, example parser, and catalog ownership it supplies. Amend only its
    executable-function-parameter restriction as needed for inline callbacks.
  - Completion: prerequisite evidence is available and each gate has an
    explicit disposition. If the shared work is unavailable, report the
    dependency and stop dependent implementation rather than building a shim.

### Wave 2 — Reduce design risk

Depends on Wave 1. This is one bounded design spike, run once before the work
it informs; later phases validate the implementation rather than repeat it.

- [x] **Prototype callback plumbing**
  - Inspect `expression/{ast,parser,mod,prepared,binding,absence,lint,error}.rs`,
    `expression/scope/`, registration/dispatch, and DMLS expression consumers.
    The current AST has spanned and erased forms; ordinary dispatch eagerly
    evaluates value arguments. Demonstrate how callback arguments bypass that
    eager path while continuing to use the authoritative registry.
  - Sketch or minimally prototype one nested local read and one nested
    file-reference failure. Establish how local dotted/bracket reads, source
    spans, typed cause accessors, and observer state survive evaluation and
    how runtime context requirements are collected from callback bodies.
  - Produce the callback-node, lexical-scope, and error-wrapper contracts in
    the implementation log. Identify exhaustive AST consumers and assign
    ownership for subsequent waves. Remove disposable prototype code or
    explicitly incorporate it into Phase 2.
  - Do not run a performance spike by default: the spec supplies neither a
    target size nor a known cost requiring measurement. If the operational
    ruling calls for measurement, define one quick sample on one host here,
    with a concrete question and decision threshold; wider benchmarking
    requires a separate author ruling.

Checkpoint: implementation can proceed without choosing unresolved public
behavior. The grammar, runtime, and editor owners agree on the same contracts.

## Phase 2 — Build Callback Syntax and Passive Scope

### Reconciliation with the implemented prerequisite (2026-10-23)

`2026-10-28-schema-enhancements` is now implemented, so the Phase 1 headline
("the hard prerequisite is not implemented") no longer holds. Phase 1's
records stay as written; Phases 2–5 below are updated to the delivered code.
The spec's Decisions section records the new decisions (callback bodies
checked with the parameter typed `unknown`, the binder's type error for a
non-array iterable, no lifecycle verbs for `map`/`filter`).

- The catalog is `darkmatter/schemas/partials/functions.yaml`, loaded by
  `compose/expression/catalog/mod.rs`. `darkmatter/docs/schemas/expression-functions.yaml`
  no longer exists, so the log's Option C has nothing to target.
- Calls bind through `compose/expression/call_binding/`. Handlers receive only
  a `&BoundCall`. `EvaluationMode` has `Eager` and `Lazy` (`LazyCall`), and an
  executable function-typed parameter is rejected by
  `RegistrationError::FunctionValued`. `ExpressionError::ArgType` and `Arity`
  no longer exist; binding failures are `ExpressionError::Call(CallBindingError)`.
  The Phase 1 dispatch contract's `ArgType` for a non-array iterable is
  therefore superseded by the binder's type error.
- Design-time checking is `compose/expression/call_check.rs`, run by
  composition (`compose/call_precheck.rs`) and DMLS
  (`dmls/src/diagnostics/call_check.rs`). DMLS reports arity, ambiguity, and
  category 5 as `dm.expression.incompatible_call` and category 3 as
  `dm.expression.uncertain_call` (2 and 4 under `expressions.strict`).
- The lifecycle verb inventory derives from the catalog
  (`darkmatter/lib/src/lifecycle/signatures.rs`; Claudine's
  `claudine/lib/tests/l1/claudine_schemas.rs` `generated_verbs()`).
- The log's human-review ordering item described a missing prerequisite.
  The remaining policy items (shadow advisory, diagnostic identities,
  operational limits) still need the author's answer before Phase 2, as the
  log says.

### Wave 3 — Extend shared expression infrastructure

Depends on Phase 1. These tasks are sequential because they change shared AST
and analysis contracts; delegate them as one owned slice if using subagents.

- [ ] **Parse inline callbacks**
  - Add `=>` tokenization and a spanned, callback-only argument form to the
    shared parser and AST. Preserve the `parse_spanned(...).erase()` contract,
    formatting, and parameter/body source spans.
  - Let the catalog's function-parameter positions govern lambda placement.
    Give invalid placements their dedicated diagnostic: top-level, collection
    elements, operator operands, return positions, and value-only arguments.
    Reject named callbacks and zero/multiple parameters consistently.
  - Cover ternary bodies, nested calls, comma boundaries, malformed arrows,
    dasherized identifiers, subtraction, and both expression/condition modes.
    Lambda bodies include every existing operator; the arrow has the lowest
    precedence. Define incomplete editor input recovery without inventing a
    second grammar.

- [ ] **Bind passive locals**
  - Extend shared binding and static traversal APIs with lexical callback
    scopes. Resolve the nearest local before shorthand document reads;
    explicit namespaces remain explicit. Reject namespace/global parameter
    names using the site's actual binding view, including unavailable globals.
  - Extend `validate_prepared`, static variable reads, lint, branch/type scope,
    and context-requirement discovery to visit callback bodies without
    evaluating them. Locals have unknown type; do not infer item or result
    types. Keep local facts isolated from identically named document paths
    and enclosing locals. This includes `DeclaredTypes::subject` /
    `TypeScope::type_of_expr` (`compose/expression/scope/`), so that the call
    check's `ArgumentType::of` sees a local as `unknown` and never as a
    same-named declared property.
  - Implement the confirmed shadow advisory through normal warning identity
    and deduplication APIs. Never report a local or its member root as an
    undeclared document property; still report genuine missing properties
    inside callbacks.

- [ ] **Check callback signatures**
  - Extend the prerequisite's shared checker and executable registration
    validation to accept inline callback arguments: amend
    `RegistrationError::FunctionValued` (and the grammar translator's
    function-where-value-expected error in `call_binding/translate.rs`) for a
    callback parameter only, keeping function-valued returns and document
    schemas rejected. Teach `check_call`/`classify_argument` to accept a
    callback argument for a `function` parameter and reject it elsewhere, and
    make `check_expression_calls` walk callback bodies under a scope binding
    the parameter as `unknown`. Enforce exactly two call arguments and exactly
    one callback parameter for these functions.
  - Use `unknown[] | null` for the iterable, one `unknown | null` callback
    parameter and `unknown | null` callback return, and `unknown[]` result.
    Validate callback form even when runtime iteration would be empty.
  - Test missing/extra arguments, non-function callbacks, invalid parameter
    counts, misplaced callbacks, forbidden names, and valid nested callbacks
    through public preparation/validation results, including DMLS
    `dm.expression.incompatible_call`. Test that a category 5 call inside a
    body (`map([], x => min(true, 5))`) rejects the document at composition,
    and that `filter(tags, tags => min(tags, 5))` with
    `tags: boolean(required)` is not category 5. Use counters or failing
    provider fakes to prove validation performs no callback I/O or execution.

Checkpoint: parser and passive-analysis tests pass; every AST consumer handles
the new form; existing expressions and catalog artifacts remain accepted.
Freeze these shared APIs before Wave 4 so its owners can work independently.

## Phase 3 — Implement Runtime and Editor Behavior

### Wave 4 — Parallel implementation

Depends on Phase 2 and the operational ruling. Tasks in this wave may run
concurrently with separate file ownership. The runtime owner controls evaluator
and error modules; the editor owner controls DMLS; the catalog owner controls
catalog artifacts and examples. Shared API changes return to the integrator
before other owners adopt them. Do not run a full suite per subagent.

- [ ] **Evaluate collection callbacks**
  - Add registered callback dispatch for `map` and `filter`, separate from
    eager value-argument evaluation: an `EvaluationMode::Callback` beside the
    delivered `Eager` and `Lazy`, following the `LazyCall` pattern so the
    evaluator, not a `&BoundCall` handler, runs the body. Evaluate the iterable
    once in the enclosing scope and bind it through the shared binder against
    `unknown[] | null`, then evaluate the body once per item in order.
  - Return `[]` for absent/null and empty arrays without callback execution.
    Reject string, numeric, boolean, and object iterables with the binder's
    type error (`ExpressionError::Call`), which names the function and the
    received type. Preserve null items and null map results; filter
    preserves original items and applies the existing truthiness function.
  - Support nested locals, inner-name reuse, member/index reads, and restoration
    after callback exit or failure. Forward the enclosing lookup's request
    context, host binding availability, absence observer, remote policy, and
    approved effect behavior; never recapture process state.
  - Wrap a body failure with function and zero-based item index, stop before
    later items, and preserve every nested wrapper and underlying typed cause,
    location, file-resolution failure class, and authoring-fatal status
    (for `ExpressionError::Call`, only an implementation-contract binding
    error is authoring-fatal; `ExpressionError::CallCheck` rejections stay
    fatal).
  - Verify public evaluation results with ordered/counted fakes: one iterable
    evaluation, all filter items visited, no empty/null callback execution,
    first-error stop, nested failures, and state restoration. Use actual
    `find_files`/`frontmatter` behavior in a controlled file fixture for the
    provenance case.

- [ ] **Expose editor locals**
  - Consume the shared parser, binding, and signature APIs in
    `dmls/src/overlay/expressions.rs` and hover, completion, diagnostics, and
    semantic-token providers. Highlight arrow and parameter; parameter hover
    identifies its owning function, including nested `map`/`filter` calls.
  - Offer innermost locals ahead of document properties only inside their
    callback body. Restore outer bindings after inner callbacks and remove
    locals after callback exit. Keep iterable-argument scope outside the new
    binding and explicit `fm.*` access tied to frontmatter.
  - Assert diagnostics and precise projected ranges on body and frontmatter
    surfaces, including a multiline/escaped scalar. Test nested reuse,
    incomplete input, completion ordering, hover ownership, and no provider
    execution through normal DMLS provider/protocol fixtures.

- [ ] **Publish catalog entries**
  - Add both functions to `darkmatter/schemas/partials/functions.yaml` using
    the prerequisite's catalog wiring, signature grammar, and inline
    `example(call => result)` mechanism, with one declared category (for
    example the existing `Collection`). Do not maintain a second catalog copy.
  - Exclude callback-taking functions from the catalog-derived lifecycle verb
    inventory (`darkmatter/lib/src/lifecycle/signatures.rs`) and from
    Claudine's `generated_verbs()` parity, and leave the verb lists in
    `darkmatter/schemas/partials/{lifecycle,md-lifecycle}.yaml` and
    `claudine/schemas/{action,lifecycle}.yaml` unchanged. Test the
    exclusion.
  - Include ordinary, null/empty, and nested examples. Prove registration and
    catalog signatures agree through the shared parity checks.
  - Add the cross-feature regression for
    `example(filter([1, 2, 3], n => n > 1) => [2, 3])`: only the arrow outside
    the call separates example input/output. Execute the example after the
    runtime slice is integrated; catalog parsing can be checked independently.

Checkpoint: integrate all three slices, run their focused L1 tests, and verify
the same signatures and diagnostics across runtime preparation and DMLS.

## Phase 4 — Prove the User-Facing Paths

### Wave 5 — Parallel acceptance coverage

Depends on integrated Phase 3. Darkmatter and Claudine tasks can run
concurrently in separate test files. All cases here are L1: hermetic
filesystem/subprocess tests do not require a terminal or browser.

- [ ] **Verify compose surfaces**
  - Exercise both functions through frontmatter interpolation, body
    interpolation, and `::block when`, including nested lexical access and
    the agreed warning behavior. Assert typed arrays remain typed in
    whole-value interpolation and produced strings remain data, without a
    second expression scan.
  - Through `CliProcessFixture`, prove `md compose` success and representative
    misplaced-lambda and callback failures. Assert rendered diagnostic code
    where exposed, file/line/column, function/index context, and no emitted
    composed result on failure. Test detailed semantics in-process rather
    than spawning the CLI for every case.
  - Add a passive shipped-artifact corpus check through the existing catalog
    path. Run the existing opt-in grammar corpus with `just test-prompts`;
    behavior fixtures use frozen copies and never depend on live prompts.

- [ ] **Verify lifecycle integration**
  - Use Claudine's normal lifecycle preparation, binding, and execution path
    with fixture-owned dependency specs. Run the motivating `filter` rule
    through `when:` and `error:`: one unimplemented dependency rejects and
    identifies it; all implemented dependencies accept; an empty dependency
    list does not trigger the error. Include a `map` lifecycle condition case.
  - Keep the fixture independent of repository spec lifecycle state and
    production prompt policy. Confirm host globals remain readable in bodies
    and forbidden as parameter names through the Claudine binding view.
  - Use public lifecycle results and existing test-toolkit fakes; launch no
    agent provider or real shell for the dependency gate. Any CLI wiring test
    uses Claudine's `CliProcessFixture`. Read the Claudine skill before edits.

Checkpoint: record an acceptance map in the implementation log: spec criteria
1–2 → surface/lifecycle cases; 3–5 → parser/scope/diagnostic cases; 6–8 → runtime
order, null, nested scope, and provenance cases; 9 → catalog/docs; 10 → DMLS
tokens/hover/completion; 11 → shared signature and passive-validation cases.
Each entry names the actual test and public observable result.

Input robustness matrix scope: expression/lambda syntax is a text grammar, so
the requested file/configuration matrix does not apply to these new argument
forms. Catalog entries use the prerequisite's existing loader and validators;
adding entries alone does not justify retesting that loader's shape matrix.
If implementation changes a catalog field validator or loader, add focused
coverage for that changed decision point: invalid shapes default to located
errors with no execution/publication, explicitly record only exceptions, keep
a real positive fixture with one edit per cell, share matrices for shared
validators, and test document shapes once per distinct loader. Prove wiring
through the crate's public result and use minimal CLI cases only where it
actually exposes that loader. Audit changed load-bearing parsing for
`#[serde(default)]`, absent/null conflation via `Option<T>`, silent `filter_map`
element removal, `unwrap_or_default()`, and discarded `.ok()` failures. Any
matrix review finding accounts for the other cells in its row and column.

## Phase 5 — Document and Validate the Integrated Change

### Wave 6 — Update current documentation

Depends on Phase 4. Documentation owners may work concurrently only when
editing separate files; final validation follows their integration.

- [ ] **Document expression behavior**
  - Update `darkmatter/docs/topics/darkmatter-expressions.md` with callback
    placement, precedence, lexical shadowing, explicit `fm`, reserved/host
    names, null behavior, truthiness, error chains, and compact examples of
    both functions and nesting. Add a Mermaid scope/evaluation flow where it
    clarifies the lifecycle of an item binding.
  - Update `darkmatter/docs/topics/expression-engine.md`,
    `darkmatter/docs/inline/call-check.md` (callback bodies are checked with
    the parameter typed `unknown`), DMLS topic pages including
    `darkmatter/dmls/docs/diagnostics.md`, and relevant darkmatter skill
    references for shared callback APIs and passive guarantees. Review the README for a concise public-feature
    mention/link; keep syntax and output references in topic docs.
  - Review docs/comments on changed symbols for drift, especially claims that
    all non-boolean functions evaluate arguments eagerly or that namespaces
    always precede every binding. Record resolved drift in the implementation
    log. Current docs describe behavior without naming feature directories.

### Wave 7 — Validate and hand off

Depends on Wave 6; execute the integration gates sequentially.

- [ ] **Run scoped verification**
  - Run `just test` and `just lint` from `darkmatter/`, then the affected
    Claudine package-area L1 and lint recipes. Use nextest-backed recipes,
    never `cargo test` or an unsolicited `cargo fmt`. Do not run lint
    concurrently with tests. Run `just test-prompts` after the grammar change
    and record its outcome separately from the normal L1 evidence.
  - Keep new tests cross-platform with fixture-owned paths, portable native
    helpers, runtime manifest/binary discovery, and no Unix shell dependency.
    Use the `os` skill before Windows-specific or path-comparison changes.
    Record environments actually tested and reuse qualifying evidence; do
    not claim unrun OS checks or add speculative CI cells.
  - No L2/browser tests are required unless an implementation change introduces
    behavior that actually needs those resources. If needed, use `just
    test-l2` or `just test-browser` with the shared harness; never activate
    terminal/browser windows or send OS input.

- [ ] **Close acceptance evidence**
  - Complete the eleven-criterion test map, record command results and
    environment coverage, and confirm all Phase 1 gates were resolved and
    implemented. Report any remaining failure as incomplete implementation.
  - Review the final diff for first-class callback leakage, parallel checking
    paths, provider execution during validation, recaptured context, dropped
    error causes, and unauthorized production prompt changes.
  - Leave the feature in its active directory as implementation complete,
    ready for review. Do not move it to `_completed`, run `just complete`, or
    commit unless separately instructed.
