---
total_phases: 6
created: 2026-10-12
phase: 3
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/simplified/alphanumeric.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_grammar_proptest.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_union_schema_matrix.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/fixtures/detect/arrays/expected.yaml
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/cli/src/commands/context/format.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/triggers/lint.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_effective_presence.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_any.snap
    - darkmatter/tests/fixtures/schema-triggers/schemas/claudine.trigger.yaml
    - darkmatter/tests/fixtures/schema-triggers/schemas/inline-compose.trigger.yaml
    - darkmatter/tests/fixtures/schema-triggers/schemas/sequence.trigger.yaml
    - darkmatter/docs/schemas/claudine.yaml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate_core_types.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - claudine/lib/src/composition/inline_prompt.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/cli/src/completion/schema_completion/keys.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - darkmatter
    - claudine
    - claudine-cli
    - darkmatter-cli
    - dmls
---

# Core Schema Types Implementation Plan

## Phase 1 — Establish Contracts and Reduce Risk

### Necessary Rules

The work adds `unknown` (canonical spelling of `any`), `null`,
`alphanumeric`, and `shell` to Darkmatter's shared schema system. It changes
SimplifiedSchema presence semantics, migrates affected shipped schemas, and
aligns validation, coercion, serialization, descriptors, CLI, and DMLS.
Although this plan lives in Claudine, most implementation belongs in
Darkmatter. Success means all 14 specification acceptance criteria have
observable tests, shipped artifacts retain their intended selection, and the
specified package gates pass. The terminal state is implementation complete,
ready for review; the author closes the feature.

These rules apply from the beginning:

1. **The specification's decisions are binding.** There are no open product
   questions. Internal representation and helper placement are implementation
   choices. Do not introduce `|` syntax, tuples, approval rules, static
   expression typing, or production lifecycle action schemas from later work.
2. **Build on resolved named types.** `2026-10-13-named-type-presence-and-local-names`
   is already implemented. Preserve its reference, constraint-merging, union
   postfix, and source-origin contracts. Determine whole-value null admission
   after reference resolution and before adding optional-property wrappers.
3. **Use one effective presence calculation.** Scalar `unknown`/`any`, scalar
   `null`, and whole-value unions containing either are optional regardless
   of authored/inherited/hoisted `required`, `eager`, or `generated`. Nullable
   array items and nullable object children do not make their containers
   optional. Preserve raw JSON Schema's authored `required` behavior.
4. **Keep options typed.** Alphanumeric option identity, normalization,
   lowering, serialization, descriptors, subtype comparison, and editor
   consumers share the library's typed representation and canonicalizer.
   Resolve references before comparison; do not recover options from display
   strings or compare keyword sets as a substitute for permitted characters.
5. **Keep validation passive.** Shell validation checks non-whitespace text,
   never syntax, approval, or execution. Preserve literal decoding and
   authored pending-value provenance. Neither new types nor tests may reorder
   composition stages or make inserted `$(...)`/`{{...}}` data executable.
6. **Migrate according to actual domains.** A concrete replacement for a
   trigger's `any(required)` must cover valid fixture inputs, including
   negative `none` conditions. Malformed values intentionally cease matching
   that concrete predicate; test the resulting registry decision. Do not
   infer `steps` from a target schema that does not declare it.
7. **Correct side-effect schema drift, preserve operations.**
   `set_frontmatter.value` keeps null and all other accepted values through an
   explicit null-admitting union. `merge_frontmatter(file, obj)` takes a file
   and object, with no `prop`; use `obj` for the object argument in its schema
   shape, matching the current function signature. This is a schema
   correction, not a lifecycle execution redesign. Audit any additional
   affected fields and record their null meaning individually.
8. **Scope robustness to changed readers.** YAML schema declarations and
   YAML frontmatter values are in scope for the matrices below. Type-expression
   text grammar gets grammar tests, not a configuration matrix. Exported JSON
   Schema is a writer surface; its validation parity tests are separate from
   reader robustness. Existing raw JSON Schema behavior is a regression
   boundary, not a new reader project. If discovery identifies another format
   whose reader must change, add its field-by-field matrix before editing it.
9. **Use existing verification infrastructure.** All added tests are L1,
   portable across macOS, Linux, native Windows, and WSL2, with no terminal or
   browser windows. Use nextest through package recipes, the Test Toolkit,
   runtime manifest paths, and `CliProcessFixture`. Register new modules in
   consolidated L1 roots. Add no CI cells. Never run `cargo fmt`, commit, or
   move the feature to a lifecycle directory as part of implementation.

Waves are numbered globally. A wave is a dependency step: complete its
checkpoint before beginning the next wave. Tasks within a wave may use
concurrent subagents only where explicitly marked and file ownership is
disjoint. Each delegated brief must state that the entire session tree is
non-interactive. One integration owner handles shared AST, parser, resolver,
converter, coercion, descriptor, and L1 registration files; do not assign
competing edits to these files.

### Wave 1 — Contract Inventory

- [x] **Contract inventory**
  - Map acceptance criteria 1–14 to existing tests and necessary additions.
    Inspect `darkmatter/lib/src/markdown/schemas/`: `simplified/{types,
    grammar,mod,source,convert,serialize}.rs`, `resolve.rs`, `phase.rs`,
    `coerce.rs`, `format.rs`, `completion.rs`, `about.rs`, and
    `triggers/{matcher,lint}.rs`, plus optional-property materialization in
    composition and Claudine's schema consumers.
  - Identify every validator builder and every independent presence check;
    record the intended shared seams in `implementation-log.md` beside this
    plan. Inspect DMLS `providers/frontmatter.rs`, schema overlays, and named
    diagnostic projection before defining their library-facing contract.
  - Read the relevant Darkmatter, rust-testing, and just skill guidance.
    Use Sniff/GitNexus when package or caller discovery is needed, confirming
    unresolved graph results with `rg`. Record a focused existing-test
    baseline and unrelated working-tree changes without modifying them.

- [x] **Migration inventory** — parallelizable with Contract inventory;
  owns only the inventory portion of the log, integrated by the coordinator.
  - Search shipped schemas, embedded baselines, inline definitions, tests,
    and docs for `any(required)`, obsolete unknown/null rejection cases,
    undeclared-variable `any` wording, and local definitions named `unknown`,
    `null`, `alphanumeric`, or `shell`. Record each site's required edit or
    reason for retaining an alias example.
  - Enumerate valid and malformed `steps` inputs from actual trigger tests.
    Include all three `darkmatter/tests/fixtures/schema-triggers/schemas/`
    trigger files and `claudine/docs/schemas/partials/side-effect.yaml`.
    Check the current `set_frontmatter` and `merge_frontmatter` operations
    against their schema shapes. The shipped sequence control currently uses
    `steps: []`, and its target only declares `sequence_name`; therefore an
    item-count minimum cannot be inferred from that target. Start the domain
    audit with `unknown[](required)` as the concrete list candidate; narrow
    items only when actual supported inputs justify it.
  - Extend the existing `shipped_schema_corpus` inventory instead of creating
    a second scanner. Distinguish existing exceptions and later-spec grammar
    from regressions introduced here; do not hide newly loadable artifacts
    behind stale exception entries.

Checkpoint: the inventory names affected consumers, exact migration domains,
test ownership, and source-location requirements. Baseline failures are
recorded separately from the intended behavior changes.

### Wave 2 — Regex Feasibility

- [x] **Regex spike**
  - Prerequisite: Wave 1. Run one small, in-memory experiment on the current
    host using the actual validator/regex engine. Prove a whole-string ASCII
    expression can permit periods while forbidding consecutive periods,
    allow empty values, and enforce leading restrictions without lookaround
    or a backtracking engine.
  - Sample `""`, `a.b`, `a..b`, edge newlines, tabs, non-ASCII text,
    leading forbidden characters, and overlapping `pattern`/`not-empty`
    constraints. Check the canonical empty option annotation too.
  - Record the viable regex construction and engine limitations in the log;
    retain useful cases as ordinary tests. This spike runs once, before
    lowering work. It is not a performance benchmark or an OS measurement
    fleet. No other spike is justified by the specification.

Checkpoint: a linear-time construction is demonstrated and the typed API
contract is settled. If the engine cannot express the specified language,
record the concrete blocker before proceeding; do not weaken the language or
switch engines silently.

### Input Robustness Matrix

The implementation must turn these tables into data-driven public-behavior
tests. For YAML, one matrix driver walks declaration and instance cells from
copied, real shipped artifacts, making exactly one edit per cell. Each field
column has its own positive control. Use the migrated trigger fixtures and
side-effect partial for applicable fields; obtain type-specific controls by
adding one declaration to a copied loadable shipped schema. Do not manufacture
an unrelated hand-written test for every cell.

`Load error` means a public CLI failure and/or schema diagnostic with the
authored location, before validation can report success. Declaration cells
must assert schema usability and witness-instance validation through
`md schema validate`, not merely the parser's return value. For absence of a
definition, a reference/witness proves the declaration was removed rather
than replaced by an empty type. YAML duplicate keys and invalid trailing
content must fail before an intermediate mapping loses that information.

**YAML declaration side:** `types.T` is repeated for each new type and local/
imported reference. `$schema.p` is repeated at nested property positions;
its union column covers native null arms and postfixed named unions.

| Shape | Root `$schema` | `types` in `kind: schema` | `types.T` named definition | `$schema.p` property definition | `$schema.p` union definition |
|---|---|---|---|---|---|
| Control | Required-file root union accepts either valid file | Nonempty named catalog resolves | `T` resolves and its witness validates | Declared type enforces its witness | Concrete-plus-null union accepts both witnesses |
| Absent | Document has no inline declaration; no synthetic empty schema; explicitly requested pure-schema artifact fails classification/use | Load error | A retained `T` reference produces unknown-name error | Property becomes undeclared; open-root witness proves this | Same removal behavior; no synthesized empty union |
| Explicit null | Load error, including empty `$schema:` | Load error | Legal scalar null type; null/omission pass, non-null fails | Same null-type result | Whole null becomes scalar null; a native null element is a legal null arm |
| Wrong type, whole | Numeric/boolean root: load error | Non-mapping: load error | Numeric/boolean definition: load error | Numeric/boolean definition: load error | Numeric/boolean instead of definition: load error |
| Wrong type, one element | Root union with native null/number arm: load error | Sequence instead of mapping: load error | Union with one numeric/boolean arm: load error | Same malformed union: load error | Valid string/object arm plus numeric arm: load error; never discard arm |
| Wrong type, every element | All invalid root arms: load error | Same non-mapping error | All numeric/boolean union arms: load error | Same error | All numeric/boolean arms: load error, never empty union |
| Empty | `{}` remains an explicit open shape; `[]` is a load error | `{}` remains a declared empty catalog; retained reference fails | `{}` is an explicit empty object type; `[]`/empty expression fail | Same object/union/expression outcomes | `[]` fails; `{}` instead is a closed empty object shape |
| Duplicate key | Duplicate `$schema` or shape key: load error | Duplicate `types` or name: load error | Duplicate `T`: load error | Duplicate `p`: load error | Duplicate keys inside an object arm: load error |
| Trailing/invalid content | Invalid YAML or an extra YAML document: load error | Same error | Same error | Same error | Same error |

**YAML instance side:** use completion/final validation with no default. `U`
means resolved `[string(required; min(3)), null]`. `A` means
`alphanumeric(required)`; `S` means `shell(required)`; `N` means
`null(required)`; `X` means `unknown(required)` and its `any` spelling;
`L` means `alphanumeric[](required)`; `O` means a required closed action
object with a required shell key. Repeat `L` for `unknown[]` and `null[]` to
prove item nullability does not change container presence.

| Shape | `p: A` | `p: S` | `p: N` | `p: X` | `p: U` | `p: L` | `p: O` |
|---|---|---|---|---|---|---|---|
| Control | `abc1` passes | `echo ok` passes | Explicit null passes | Native nested object passes unchanged | `abc` passes | `[abc]` passes | `{ shell: echo }` passes |
| Absent | Fails presence | Fails presence | Passes omission | Passes omission | Passes omission | Fails presence for every item type | Fails parent presence |
| Explicit null | Fails | Fails | Passes as supplied null | Passes as supplied null | Passes as supplied null | Fails container type/presence | Fails parent type/presence |
| Wrong type, whole | Number/bool fails without coercion | Number/bool coerces to text in coercing paths; object fails | Any non-null fails | Every native value passes unchanged | Number/bool may pass after existing string coercion; invalid structured value fails | Scalar/object fails | Scalar/array fails |
| Wrong type, one element | Sequence fails | Sequence fails | Sequence fails | Mixed array passes unchanged | Sequence fails | `[abc, 123]` fails; `unknown[]` admits both; `null[]` fails a non-null item | Extra key or malformed shell value fails |
| Wrong type, every element | Sequence fails | Sequence fails | Sequence fails | Array passes unchanged | Sequence fails | `[123]` fails; `unknown[]` admits it; `null[]` rejects it | All unexpected keys fail |
| Empty | `""` passes; `min(1)`/`not-empty` variant fails | Empty/whitespace-only text fails | Empty string/array/object fails | Empty string/array/object passes unchanged | `""` fails `min(3)` | `[]` passes without item-count constraint; `min-items(1)` variant fails | `{}` fails required action key |
| Duplicate key | YAML load error | YAML load error | YAML load error | YAML load error, including nested duplicate | YAML load error | YAML load error, including object-item duplicate | YAML load error, including nested duplicate |
| Trailing/invalid content | YAML load error | YAML load error | YAML load error | YAML load error | YAML load error | YAML load error | YAML load error |

Absence is intentionally legal only where the type contract permits it; tests
must distinguish absent keys from supplied null using unchanged validation
inputs and composition's documented materialization result. Add ordinary
optional concrete-type controls so this change cannot make all optional
properties permanently optional. Exercise preserved raw JSON Schema required
nullable properties alongside these tests. For migrated trigger keys, assert
the selected schema/registry decision for absent, null, wrong-type, mixed,
all-invalid, and empty inputs in both positive and `none` predicates, using
the domains recorded in Wave 1 rather than inventing a universal outcome.

Before closing the matrix, inspect changed paths for load-bearing
`#[serde(default)]`, `Option<T>` null/absence collapse,
`filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` parse loss.
Any matrix review finding records every other cell in its row and column.

**YAML migrated-field side:** instantiate the same matrix driver for these
actual fields. `steps` uses the list candidate above; if the domain inventory
establishes a different concrete declaration, update this column before
implementation. A false `steps` predicate also means a `none` condition no
longer excludes the document; assert the complete registry result, including
whether target-schema validation then fails, rather than calling predicate
failure a successful fallback.

| Shape | Trigger `steps: unknown[](required)` | `set_frontmatter.value` null-admitting union | `merge_frontmatter.obj: object(required)` |
|---|---|---|---|
| Control | Shipped `[]` produces the intended sequence selection | Supplied native value validates | Supplied object validates without `prop` |
| Absent | Predicate false, not an empty list | Allowed by null-admitting presence rule; operation arity is a separate boundary | Validation fails presence |
| Explicit null | Predicate false | Supplied null validates | Validation fails |
| Wrong type, whole | Scalar/object predicate false | Every native value category validates | Scalar/array fails |
| Wrong type, one element | Mixed list matches this unconstrained-item candidate; narrower audited item type must reject invalid items | Mixed array validates unchanged | Array fails; nested object values are unconstrained |
| Wrong type, every element | List still matches this candidate; no item filtering | Array validates unchanged | Array fails |
| Empty | `[]` matches; empty scalar/object does not | Empty text/array/object validates | `{}` validates as an explicit object |
| Duplicate key | YAML load error, never last-wins selection | YAML load error, including nested keys | YAML load error, including nested keys |
| Trailing/invalid content | YAML load error | YAML load error | YAML load error |

Repeat the generic instance columns for each migrated `file`/`prop` argument
using its existing concrete type contract. The removed merge `prop` field is
an unexpected key in a closed argument shape, not an optional retained field.

## Phase 2 — Implement the Shared Type Model

### Wave 3 — Typed Grammar

- [x] **Type representation**
  - Prerequisite: Phase 1 checkpoint. Extend the shared AST/keyword catalog
    for unknown/any, null, alphanumeric options, and shell. Keep internal
    `Any` naming or rename it consistently; expose `unknown` canonically.
  - Introduce typed alphanumeric options and one canonicalizer in the
    library: table order, redundant blocked characters, and restrictions
    implied by `leading-alpha`. Preserve authored option spans independently
    of normalized identity.
  - Parse native YAML null only in property definitions, union arms, and
    named definitions. Keep root null, null root arms, and `literal(null)`
    invalid. Route schema meta-types through the same rules.

- [x] **Option grammar** — sequential after Type representation, same
  integration owner because grammar/AST files overlap.
  - Implement option-before-`;` syntax, constraint-only shorthand, and
    rejection of unknown, repeated, post-separator, or mixed unseparated
    options. Keep existing constraint syntax and applicability intact.
  - Extend named-reference postfix resolution to compare canonical option
    identities and merge only constraints. Equivalent spellings succeed;
    different option sets fail with definition/use-site related locations.
  - Switch colliding shipped local references to explicit `Name@this`.
    Fix obsolete grammar-rejection expectations; serialize canonical
    `unknown`, explicit string `"null"`, and canonical alphanumeric options.

Checkpoint: parsing, reference resolution, meta-type recognition, diagnostics,
and parse/serialize/parse meaning agree. Cover native null spellings, local
and imported null types, option order, aliases, and conflicts before exposing
the model to parallel consumers.

## Phase 3 — Integrate Validation and Type Relations

### Wave 4 — Effective Presence

- [x] **Presence integration**
  - Prerequisite: Wave 3. Implement resolved whole-value null admission and
    make lowering, trigger matching/vacuity lint, optional materialization,
    completion metadata, and launch/completion projection consume it.
  - Cover written/inherited/hoisted markers, nullable postfixed unions,
    required nullable-item arrays, and required objects with nullable
    children. Preserve non-nullable generated/eager behavior and raw JSON
    Schema presence. Update behavior-changing comments at every edited seam.
  - Verify inclusive root unions remain inclusive and open where already
    open; prove the null-based XOR idiom without changing closest-arm
    reporting, dictionary restrictions, or raw closed objects.

Checkpoint: criteria 1–4 and 12 pass through public validation paths;
nullable union constraints still constrain non-null values. No migration
fixture may be left relying on the former presence meaning at final closure.

### Wave 5 — Value Semantics

- [x] **Value validation**
  - Prerequisite: Wave 4. Own converter, format, coercion, resolver default
    checks, and associated registrations. Lower null to `type: null`,
    unknown/any to `{}`, and shell to `format: darkmatter-shell`.
  - Use the spike's alphanumeric regex and preserve its base pattern, every
    user pattern, and `not-empty` with conjunctions as needed. Emit
    `x-darkmatter-type` and canonical `x-darkmatter-options`, including `[]`.
  - Reject invalid null defaults; validate alphanumeric defaults and
    suggestions against its full applicable type constraints without
    changing other types' advisory suggestions. Mirror expression's shell
    constraint rules, scalar default conversion, and structured-default
    rejection. Register shell format enforcement in every builder.
  - Preserve unknown native values recursively. Reject number/bool coercion
    for alphanumeric in scalars, items, and union candidates while allowing
    other union arms their existing conversions. Mirror expression's shell
    coercion, literal decoding, and authored pending-value deferral.

- [x] **Subtype relation** — parallelizable with Value validation; owns a
  separate library module and tests. The integration owner exports it.
  - Implement the passive relation over resolved typed atoms. Identical
    types with identical canonical identity return true, including identical
    non-string types and arrays; distinct arrays/containers are not promoted
    to string. Ignore ordinary constraints and optional wrappers.
  - Compare alphanumeric allowed-character and allowed-first-character
    sets plus the double-period guarantee. Include implied restrictions,
    option reordering, and redundant options. Check every always-text type
    against string; unknown, numbers, arrays, and unlisted pairs return false.
  - Do not attempt regex inclusion, named-reference I/O, runtime matching,
    scalar conversion, or approval-policy implementation.

Checkpoint: criteria 5–7, 13, and 14 have focused passing tests, including
validator-compiled regexes, edge newlines, tabs, Unicode whitespace,
constraints that overlap, shell whitespace, and repeated serialization
round trips. Literal and inserted-data passivity regressions remain green.

## Phase 4 — Migrate Artifacts and Align Consumers

### Wave 6 — Parallel Consumer Work

- [ ] **Artifact migration** — parallelizable; owns shipped YAML, migration
  fixture expectations, and the inventory results in the log.
  - Prerequisite: Phase 3. Replace all presence-dependent `any(required)`
    uses with the narrowest valid-domain concrete expression and required
    marker. Update the three trigger fixtures and definition examples;
    preserve positive selection, negative exclusions, and vacuous-arm lint.
  - Correct the side-effect partial per Phase 1 rules. Test explicit null
    and every native value category for `set_frontmatter.value`; test an
    object argument without `prop` for merge and rejection of non-objects.
    Record each additional field's null decision and the intentional
    malformed-trigger difference.
  - Re-run the inventory search: retained alias examples must not claim
    `any(required)` requires presence. Update the conversion test that
    previously rejected omission while retaining explicit-null acceptance.

- [ ] **Catalog alignment** — parallelizable; owns `schemas/about.rs`, CLI
  about presentation and tests. Shared library registrations go through the
  integration owner.
  - Add authoritative descriptors and option metadata; show unknown with
    alias any and accurately describe null, alphanumeric, and passive shell.
    Maintain type/constraint descriptor parity tests and canonical output.
  - Ensure `md schema about` consumes the shared catalog. Use existing
    `TerminalRenderable` components for any changed terminal rendering.
    Publish stable catalog metadata early enough for the next wave.

- [ ] **Runtime parity** — parallelizable; owns dedicated library/Claudine
  consumer tests, with new test registrations integrated centrally.
  - Assert ordinary validation, trigger matching, composition, and
    launch/completion agree on presence. Keep existing initialization parser
    and runtime shell bans green, including shell-valued `set` and bootstrap
    expansion. No production action-union rollout is included here.
  - Use a closed lifecycle-shaped fixture whose initialize action union
    excludes shell and whose start union includes it. Reject shell-only,
    mixed allowed-action-plus-shell, and empty action objects under initialize.
  - Prove raw JSON Schema nullable-required controls, union candidate
    coercion, and origin-aware pending/data behavior through existing APIs.

Checkpoint: migrated artifacts load and select correctly; catalog parity
passes; runtime consumers use the shared contracts without duplicated
interpretation. Integrate all three tasks before editor work.

### Wave 7 — Editor Integration

- [ ] **Editor support**
  - Prerequisite: Wave 6. Update DMLS keyword completion, alphanumeric option
    completion before `;`, and hover using parsed library options/catalog
    metadata. Hover reports effective canonical character sets and
    unknown's any alias; local built-in collisions retain explicit references.
  - Project native-null spans and option errors, including named definition
    conflicts with related locations, through existing shared source maps.
    Do not add a DMLS-local parser, canonicalizer, or presence algorithm.
  - Add L1 LSP tests for valid and invalid unsaved schema buffers,
    completion cursor positions, hover, meta-types, and initialize/start
    action diagnostics with exact ranges. No editor windows are launched.

Checkpoint: criteria 9–10 and editor portions of 7/14 pass; DMLS and `md`
agree on valid declarations, effective types, and authored diagnostic sites.

## Phase 5 — Prove Public Behavior and Update Documentation

### Wave 8 — Parallel Acceptance Work

- [ ] **Acceptance matrix** — parallelizable; owns CLI/L1 matrix tests.
  - Prerequisite: Phase 4. Implement the Phase 1 robustness matrix driver,
    including positive controls, duplicate keys, extra documents, and public
    outcomes. Finish the changed-reader smell audit and record the results.
  - Exercise the specification's full Phase A table through
    `md schema validate`: file presence, unknown/any, null, inclusive and XOR
    roots, nullable named unions, and required containers. Extend existing
    schema CLI/phase suites rather than bypassing normal invocation.
  - Cover all alphanumeric options with positive/negative values, canonical
    annotations, native scalar failures, defaults/suggestions, overlapping
    patterns, and serializer meaning. For redundant options, assert semantic
    equivalence rather than inventing an impossible negative witness.
  - Use isolated fixture homes/CWD/PATH and scoped Test Toolkit helpers;
    declare repository reads in CI-indexable forms. Ensure each new module
    is reachable from its existing L1 target.

- [ ] **Corpus coverage** — parallelizable; owns the existing
  `shipped_schema_corpus` test and artifact invocation coverage.
  - Extend its passive inventory for the changed built-ins, native null,
    trigger migration, and side-effect partial. Preserve instrumentation
    proving no execution/network attempts or document mutation.
  - Add/extend a normal `md schema validate` invocation against actual
    migrated shipped artifacts. Audit obsolete unknown/null rejection
    snapshots and corpus exceptions; keep later-feature unsupported syntax
    explicit rather than widening scope.

- [ ] **Current documentation** — parallelizable; owns README, topic docs,
  and skill references, coordinating any shared definition-example edits
  with Artifact migration.
  - Reconcile `darkmatter/docs/topics/schemas/definition.md` optionality,
    universal constraints, type table, and phase table. Add compact examples
    for all four types, option syntax/canonicalization, inclusive roots/XOR,
    whole-value versus item nullability, and the subtype relation.
  - Update `schema-types.md`, `schema-constraints.md`, `schema-validation.md`,
    `schema-triggers.md`, and `dmls-schema-support.md` where their current
    behavior descriptions change, plus the Darkmatter README and skill's
    schema reference. Describe shell custom-format enforcement for exported
    schemas and keep authoring validation distinct from runtime shell bans.
  - Rename undeclared-expression-variable type wording to unknown without
    promising static checks. Correct relevant Claudine side-effect docs if
    they reflect the schema drift. Remove obsolete planned-nullable wording.
  - Docs stand alone for a new developer and never name/link feature specs.
    Use a small Mermaid flow where it clarifies resolution → null admission
    → presence → validation; avoid unrelated documentation cleanup.

Checkpoint: each of criteria 1–14 maps to a named passing test or gate;
robustness cells assert public outcomes; docs and descriptor/editor behavior
agree. Record departures from the specification only in the implementation
log; do not rewrite the finalized spec.

## Phase 6 — Validate and Prepare Review

### Wave 9 — Final Gates

- [ ] **Package verification**
  - Prerequisite: Phase 5. Run `just test` and `just lint` in `darkmatter/`
    (library, CLI, DMLS, and the recipe's remaining targets), then `just test`
    in `claudine/` for the partial and dependent behavior. Run any additional
    focused check only if actual downstream API edits require it.
  - Preserve nextest isolation. Record exact commands, outcomes, and any
    toolchain/environment blockers, including the existing Zed WASM target
    prerequisite of Darkmatter lint; do not claim a blocked gate passed.
  - Keep portable assertions free of shell syntax assumptions and host path
    spelling. Use qualifying existing evidence where available; otherwise
    follow existing OS test routes/policy and load the os skill before
    claiming an environment cannot be tested. No new matrix cells, remote
    performance fleet, terminal tests, or browser tests are required.

- [ ] **Review readiness**
  - Review the final diff against the migration inventory and all 14
    acceptance criteria. Confirm no unaccounted presence checks, validator
    builders, display-string option parsing, stale comments, or shipped
    presence-dependent `any(required)` uses remain.
  - Confirm raw JSON Schema behavior, composition stage order, passive
    validation, initialization bans, and out-of-scope later features remain
    consistent with their contracts. Repeat only checks invalidated by a
    subsequent fix.
  - Complete `implementation-log.md` with affected files, semantic migration
    decisions, test evidence, and any remaining concrete limitations. Check
    off this plan only for completed tasks and report implementation complete,
    ready for review. Leave feature lifecycle closure to the author.

Checkpoint: required gates pass or an explicit unresolved blocker is reported;
the final review has a complete acceptance/evidence map and no silently
weakened requirement.
