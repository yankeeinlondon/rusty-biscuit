---
total_phases: 5
created: 2026-10-27
phase: 2
agent: opencode/zai-coding-plan/glm-5.3
yolo: true
source_files_during_phase_1:
- darkmatter/lib/src/markdown/schemas/simplified/types.rs
- darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
- darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
- darkmatter/lib/src/markdown/schemas/simplified/signature.rs
- darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
- darkmatter/lib/src/markdown/schemas/simplified/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- darkmatter
---

# Plan: Declarative Narrowing Metadata in SimplifiedSchema

## Summary and Definition of Done

Today a predicate's narrowing fact — what its boolean result proves about the
original argument — lives in Rust registration metadata
(`RegistrationBuilder::narrows` in
`darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs`,
used by `functions/predicates.rs`, `functions/paths.rs`, the host builder
configure closures, and the facts-artifact reader). This feature makes the
`function(...)` declaration in the catalog the single authority: a new optional
`narrows(...)` clause is parsed by the SimplifiedSchema grammar, validated by
one shared fact compiler, and lowered into the existing `FactKind` /
`FactType` / `NarrowingFact` machinery that `scope/facts.rs`, composition's
call checker, and DMLS already consume. No second narrowing engine is
introduced; the guard algebra (`predicate_fact`, `TypeScope::refine`,
`narrowing_fact_for`) is untouched.

The work falls into these pieces:

- **Schema language (Phase 1):** a `NarrowsClause` on `FunctionType` with
  authored spans; grammar parsing and syntax validation of the clause;
  serialization after `returns(...)`; positioning in subtype/canonical
  comparison.
- **Authority (Phase 2):** one shared fact compiler over parsed declarations
  (all-overload parameter presence, fact agreement, true-iff family
  restrictions); registration builds facts from declarations; the public
  Rust-side `narrows` setter is removed; catalog loading validates every
  function entry; built-ins, the test-host fixture, and test registrations
  migrate to the clause (`file_exists` changes from `string` to `file`).
- **Artifact (Phase 3):** the version-1 facts artifact stays derived output;
  the reader stops calling the (removed) setter and instead compares the
  decoded fact with the catalog-compiled fact, including absence; the
  committed companion fixture is regenerated.
- **Acceptance (Phase 4):** narrowing behavior tests, passivity proofs with
  handler-entry/effect counters, JSON-Schema and meta-type rejection checks,
  the CLI end-to-end guard case, DMLS diagnostics, and the shipped catalog
  corpus.
- **Docs (Phase 5):** the function-signature topic, the expressions
  guard/narrowing reference, the host-functions registration/artifact
  documentation, and the Darkmatter skill topics, including the
  `file_exists` string-only claims.

**Done** means every acceptance criterion in the spec holds, each backed by a
test, and in particular:

- Parse/serialize round trips for `true-implies`, `true-iff`, `false-implies`,
  and presence-only clauses agree on repeated parse→serialize→parse.
- Each invalid clause and overload case is rejected through public entry
  points (`parse_type_definition`, standalone/catalog loading,
  `HostFunctionCatalog::load`, `HostFunctionsBuilder::build`,
  `FunctionRegistration::builder`) with located spans for clause-level errors,
  and no handler executes on any rejection path.
- `file_exists(x)` and `is_file_type(x)` both narrow an `unknown` subject to
  `file` on the true side; `file_exists`'s false side adds no exclusion.
- The migrated `true-iff` primitives (`is_string`, `is_number`, `is_array`,
  `is_object`, `is_null`) and presence predicate (`is_empty`) keep their
  current branch behavior.
- A version-1 artifact with empty, extra, mismatched, malformed, stale-hash,
  or missing fact data behaves per the existing per-function versus
  whole-catalog boundary; a matching artifact agrees with runtime
  registration; a missing artifact grants no narrowing.
- Duplicate binding contracts remain errors despite differing `narrows`
  clauses; equivalent target spellings compare consistently without source
  locations.
- A `narrows`-carrying function in a document schema still fails JSON Schema
  conversion at the property path; `type-definition` meta-type validation and
  DMLS schema authoring use the same grammar checks and located errors.
- `just test`, `just lint`, and `just doctest` pass in `darkmatter/` (library,
  CLI, DMLS, zed-dmls); no L2/L3 tier is touched and no window gains focus.

No spike is scheduled: every open question is answerable from code, and the
spec explicitly rules out performance work ("No performance spike or new
benchmark campaign is required").

## Phase 1: The `narrows` Clause in the Schema Language

Add the declaration to `FunctionType`, the grammar, and the serializer, with
no consumer changed yet. `FunctionType.narrows` defaults to `None`
everywhere, so the workspace stays compile-green and behavior-neutral at the
end of this phase.

### Necessary Rules

The rules below settle every choice the spec leaves open; each names the
evidence it follows from.

1. **Clause grammar.** `narrows` is a function constraint of the shape
   `narrows(name)` or `narrows(name: type)` followed by `; kind` inside the
   clause's own parentheses, where `kind` is exactly one of `true-implies`,
   `true-iff`, `false-implies`. Operand order is fixed (parameter, optional
   type, implication); neither operand is a list. It may appear in any order
   among the other clauses, at most once, and the existing trailing `;`
   before the function's closing `)` admits it. Evidence: spec "Proposed
   Grammar" and "Inside `narrows`, exactly one parameter identifier, an
   optional `: type`, and one `; implication` are required"; the clause loop
   in `grammar.rs::parse_function` (lines 1073–1145) already treats clauses
   uniformly.
2. **Representation.** `FunctionType` (types.rs:359) gains
   `pub narrows: Option<NarrowsClause>`. `NarrowsClause` lives beside it in
   `types.rs` and holds `parameter: String`, `parameter_span`,
   `target: Option<PropertyAtom>`, `target_span`, `kind: NarrowsKind`,
   `kind_span`, and `clause_span` — all spans `Range<usize>` byte offsets
   into the type-expression string, the same convention as
   `CallExample::call_span`/`expected_span` (types.rs:410–423). `NarrowsKind`
   is a new schema-side enum `{ TrueImplies, TrueIff, FalseImplies }`
   (defined in `types.rs`; the grammar layer must not import from
   `compose::expression`, which already imports `schemas::simplified`).
   `PartialEq` for `NarrowsClause` ignores every span field, following
   `CallExample`'s custom `PartialEq` (types.rs:425–431).
3. **Grammar-level checks, all span-located, in `parse_function`.** After the
   clause loop (where `parameters` is known regardless of clause order, as
   the required-clause checks at grammar.rs:1147–1155 already assume):
   - duplicate `narrows` → error at the second clause's keyword span whose
     message names the first occurrence ("duplicate `narrows` constraint on
     `function`", joining the existing duplicate arm at grammar.rs:1085–1094;
     the message reports the first clause's byte offset as the related
     location — message-level naming, the established pattern for
     `duplicate category` in `standalone.rs::parse_categories`);
   - the named parameter must be declared in the same signature's
     `parameters(...)` and must not be the rest parameter → error at the
     parameter span ("`narrows` names parameter `x`, which this function does
     not declare" / "…which is a rest parameter");
   - the type operand parses as exactly one union arm (two arms → "`must be a
     single type, not a union`", matching the existing message in
     registration.rs:468) and must not be a named reference
     (`TypeExpr::Imported`, bare `Name` or `Name@file`) → "a named type
     cannot be a narrowing target; write a self-contained type" (this is new
     syntax with no existing declarations, so rejecting references narrows
     nothing; it implements the spec's chosen Open-Questions option 1);
   - the implication keyword must be one of the three (unknown or missing →
     error at the kind span);
   - extra operands (a second identifier, a second kind, a comma list) →
     error;
   - `narrows` requires the same declaration's `returns(...)` to be exactly
     the single primitive `boolean`: `PropertyDef::Single` atom, `is_array ==
     false`, `TypeExpr::Primitive(SimplifiedType::Boolean)`. `boolean | null`,
     `unknown`/`any`, `boolish`, unions, and arrays are rejected with the
     error at the `narrows` clause span. Evidence: spec Validation "Every
     overload must return a non-null boolean on success; `boolean | null`,
     `unknown`, and truthy non-boolean results do not qualify" — `boolish` is
     a coerced truthiness type (translate.rs:1022–1025 compiles both to
     `Shape::Boolean`), and its result is exactly a "truthy non-boolean"
     case, so it is rejected.
4. **Shared validation helpers, one implementation.** The per-declaration
   predicates (`returns_exact_boolean(&PropertyDef)`, the target checks) are
   `pub(crate)` functions in `schemas/simplified` (beside `NarrowsClause`) so
   that both the grammar (early, span-located) and the Phase-2 fact compiler
   (authoritative, also guarding programmatically built `FunctionType`s that
   never passed through the grammar) call the same code. No validity rule
   exists in two implementations. Evidence: spec "without … a second
   implementation of validity rules".
5. **Serializer.** `write_function` (serialize.rs:310) emits the clause
   immediately after `returns(...)`, before `fallible`/`example`/constraints,
   with the spellings `narrows(value: file; true-implies)` and
   `narrows(x; false-implies)`. Evidence: spec "Serialize it after
   `returns(...)`"; the spec's own examples place `narrows` before
   `category`/`fallible`.
6. **Subtype, canonical, and equality positioning.**
   - Function subtype (`subtype.rs::same_definition`, lines 570–581) ignores
     `narrows` exactly as it ignores examples and categories: subtyping is
     about accepted inputs and outputs; `narrows` is analysis metadata.
   - `signature.rs::canonical_atoms`'s `TypeExpr::Function` branch (lines
     182–193) clears `narrows` alongside `examples`, so duplicate-overload
     contracts and nested-function canonicalization never see the clause.
     `binding_contract` (signature.rs:64) and registration's `contract()`
     (registration.rs:547) already compare parameters only, so "overloads
     with identical binding contracts remain duplicates even when their
     narrowing clauses differ" holds by construction and gets a test.
   - Derived `PartialEq` on `FunctionType` includes `narrows` (span-free via
     rule 2), which is correct for descriptor/clone comparisons.
7. **Where each check runs.** Grammar-level checks (rule 3) run everywhere
   the grammar runs: standalone documents, catalogs, inline `$schema`,
   `type-definition` meta-type validation (`format.rs::
   parse_type_definition_instance`), and DMLS-authored schemas, all through
   the same `SchemaError::Grammar` spans and the existing
   source-projection/located-failure machinery (source.rs). Cross-overload
   checks (Phase 2) run only in the shared fact compiler — catalog loading
   and registration — because that is where an overload set exists; a
   document schema whose `types:` union mixes `narrows` facts parses
   (description-only, never registered), while `duplicate_overload` continues
   to apply to it. Evidence: spec "Parsing a function catalog does not
   register or execute its functions"; `check_named_types`
   (standalone.rs:330) is the existing whole-`types` check site.
8. **Open-Questions ruling.** Option 1: this feature keeps targets
   self-contained (primitives with constraints, `file`, `enum(...)`,
   `unknown[]`, inline objects, tuples — whatever parses as one non-reference
   arm); named/imported and recursive targets are rejected per rule 3. Every
   existing built-in fact migrates without named targets, and the spec states
   "The implementation scope above assumes that option."
9. **Preserved targets.** One-way targets keep today's acceptance
   (`file`, `string(min(3))`, `enum(a, b)`, `unknown[]`, inline objects —
   whatever `parse_type_definition` yields as a single non-reference atom;
   see `compile_fact` registration.rs:462–479 and `Shape::suitable_for`
   scope/shape.rs:409). `true-iff` keeps requiring `null` or a bare exact
   family (`string`, `number`, `boolean`, `object`, `unknown[]`) via
   `is_exact_fact_type` (scope/shape.rs:598), checked in the Phase-2 compiler
   (it needs the shape layer, which `schemas` must not import upward).
10. **Default outcome for invalid input (matrix base).** Every invalid clause
    shape produces a located `SchemaError::Grammar` naming the offending
    token; nothing is registered, no handler runs, nothing publishes. The
    only non-error exception is **absence**: a function without the clause
    has no fact (`None`), and absence is meaningful (it is not an empty
    collection, a default, or an error).

### Tasks

Wave 1 — data model and grammar (single task; the serializer depends on it):

- [x] **Narrows clause parsing**
    - Add `NarrowsClause` and `NarrowsKind` to
      `darkmatter/lib/src/markdown/schemas/simplified/types.rs` per rule 2
      (span fields excluded from `PartialEq`), and `narrows: Option<…>` to
      `FunctionType`; update every `FunctionType { … }` construction site
      (grammar.rs, tests) to `None`/parsed value.
    - Extend `parse_function` in
      `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs`: accept the
      `narrows` keyword with arguments, parse the operand per rule 1, and
      enforce every rule-3 check with spans; extend the unknown-constraint
      message ("a function takes `parameters`, `returns`, `narrows`,
      `category`, `fallible`, and `example`") and the EBNF comment at the top
      of the file.
    - Add the shared per-declaration helpers per rule 4 in
      `schemas/simplified` and call them from the grammar.
    - Pre-requisite: none. Risk: clause order freedom means parameter
      existence must be checked after the loop, not inline.

Wave 2 — serializer and positioning (parallel, file-disjoint):

- [x] **Serialize the clause**
    - Extend `write_function` in
      `darkmatter/lib/src/markdown/schemas/simplified/serialize.rs` per rule
      5; a `None` clause serializes to nothing.
    - Add round-trip tests: all three implication kinds and presence-only,
      parse → serialize → parse yields equal `FunctionType`s (spans excluded),
      and serialize places the clause after `returns(...)`.
- [x] **Comparison positioning**
    - Clear `narrows` in `signature.rs::canonical_atoms`'s function branch
      (rule 6) and leave `subtype.rs::same_definition` untouched, with a
      comment noting `narrows` is deliberately excluded from subtype.
    - Add unit tests in `signature.rs` (same parameters, different `narrows`
      → duplicate overload still detected) and `subtype.rs` (functions
      differing only in `narrows` remain mutual subtypes).
- [x] **Grammar rejection tests**
    - In `grammar.rs`'s test module, one cell per rule-3 shape from a valid
      control (`function(parameters(x: unknown | null); returns(boolean);
      narrows(x: string; true-iff))`): duplicate clause (second located,
      first named), unknown kind, missing kind, unknown parameter, rest
      parameter, union target, reference target (`Name`, `Name@this`),
      malformed type text, extra operands, clause on `returns(boolean |
      null)` / `returns(any)` / `returns(boolish)` / non-boolean.
    - Keep a control row proving the unedited fixture parses (matrix rule).
    - These are in-process through `parse_type_definition`, the crate's
      public parser (matrix rule: test at the public API, not a spawned
      binary).

Validation checkpoint: `just check` in `darkmatter/`, then
`cargo nextest run -p darkmatter --features effects-instrumentation
schemas::simplified` (grammar/serialize/signature/subtype units) — all green
with no behavior change (`just test` filter for
`lib/tests/l1/shipped_schema_corpus` still passes: no shipped catalog entry
uses the clause yet).

## Phase 2: Shared Fact Compiler, Registration Authority, and Built-in Migration

One compile-atomic change set: the compiler reads declarations, registration
and catalog loading enforce them, the Rust-side setter and binding facts are
removed, and every in-tree caller migrates to the clause. The tree may not
compile between this phase's waves; it must at the phase checkpoint.

### Tasks

Wave 3 — compiler and authority (interlocking; land together):

- [ ] **Shared fact compiler**
    - In `call_binding/registration.rs`, replace `compile_fact` (which takes
      a builder-supplied `(parameter, kind, ty)` and re-parses type text) with
      `compile_declared_fact(function: &str, overloads: &[FunctionType]) ->
      Result<Option<NarrowingFact>, RegistrationError::InvalidFact>` that:
      returns `Ok(None)` when no overload declares the clause; runs the
      shared per-declaration helpers on every declaring overload (rule 4 —
      programmatic `FunctionType`s bypassed the grammar); requires the named
      parameter in **every** overload at the **same** index, never rest —
      replacing today's `filter_map` (registration.rs:450) which silently
      skips overloads that omit the parameter, with a regression test where
      only the later overload omits it; requires all overloads to agree
      (all-absent, or same parameter name and position, same `NarrowsKind`,
      canonically equal target or both presence-only — canonical equality via
      serialized target with descriptions stripped and constraints sorted,
      never raw text or spans), rejecting mixed or contradictory facts with a
      message naming both overloads; and for `true-iff` requires
      `is_exact_fact_type` on the parsed target (rule 9). A presence-only
      clause compiles to `FactType::NonNull` — never a concrete type. The
      compiled `NarrowingFact` keeps its shape; `FactType::Type` text becomes
      the canonical serialized target.
    - Lower `NarrowsKind` → `FactKind` here; delete nothing else from the
      public `NarrowingFact`/`FactKind`/`FactType` surface.
- [ ] **Registration reads declarations**
    - `build_with` calls `compile_declared_fact` on its `FunctionType`
      overloads (both `Source::Grammar` and `Source::Types`) and stores the
      result; delete `RegistrationBuilder::narrows`, the builder's `fact`
      field, and the old `compile_fact` parameter-lookup path (its
      boolean-return check is superseded by the shared helpers). Evidence:
      spec Ownership step 4 ("Remove the Rust-side narrowing setter … after
      migrating all callers"), and this repo has no install base ("no
      deprecation period").
    - Update the `build_with` doc comment and `RegistrationError` messages
      accordingly, plus every doc comment that links the removed setter
      (`host/mod.rs::HostFunctionsBuilder::function`'s configure list names
      `RegistrationBuilder::narrows` today).
- [ ] **Catalog loading validates**
    - In `compose/expression/catalog/mod.rs::load_catalog`, after projecting
      a function's overload atoms, run `compile_declared_fact` and fail the
      load with `CatalogError::Invalid` naming `` `types.{name}` `` on error
      (the whole-file failure mode of the existing strict loader, e.g. mixed
      categories at catalog/mod.rs:310–314 and duplicate overloads in
      `check_named_types`). Passive editor loading
      (`HostFunctionCatalog::load` → `host_function_set` →
      `schemas/catalog.rs`) reports this under the existing
      `dm.schema.host_catalog` behavior; the compiler is a pure function, so
      no path initializes the runtime registry (rule 7's cycle constraint).
    - Note: `build_with` re-validates the same declarations through the same
      function — that is deliberate reuse, not a second implementation.
- [ ] **Built-in catalog migration** (text-only, parallel-safe)
    - `darkmatter/schemas/partials/functions.yaml`: add the clause to the
      eight predicates — `is_null`: `narrows(x: null; true-iff)`; `is_string`:
      `narrows(x: string; true-iff)`; `is_number`: `narrows(x: number;
      true-iff)`; `is_array`: `narrows(x: unknown[]; true-iff)`; `is_object`:
      `narrows(x: object; true-iff)`; `is_empty`: `narrows(x;
      false-implies)`; `is_file_type`: `narrows(value: file; true-implies)`;
      `file_exists`: `narrows(value: file; true-implies)` — the last a
      semantic change from the current Rust-side `string` fact
      (paths.rs:10), per spec Ownership step 3. Parameter names must match
      each catalog declaration exactly (`x` for predicates, `value` for the
      file pair). No other entry changes; `is_positive`/`is_negative`/
      `is_integer` gain nothing.
    - `darkmatter/lib/tests/fixtures/host_functions/project.yaml`:
      `is_project_file` gains `narrows(value: file; true-implies)`.
    - Risk: the catalog hash changes, invalidating `project.facts.json`
      until Phase 3 blesses it; the freshness test will fail between Phase 2
      and Phase 3 — acceptable, sequenced in the same implementation run.

Wave 4 — migrate callers and remove the Rust-side declarations (parallel,
file-disjoint; must merge with Wave 3 before the checkpoint):

- [ ] **Remove binding facts**
    - `functions/predicates.rs`: drop all seven `.narrowing(...)` calls;
      `functions/paths.rs`: drop `file_exists`'s `.narrowing("value",
      TrueImplies, Some("string"))`, keeping `.false_for_malformed_reference`
      (that rule stays Rust-side per spec Ownership: "Rust registration still
      owns unrelated operational metadata"); `functions/mod.rs`: delete
      `FunctionBinding::fact` and the `narrowing` const fn, and remove the
      fact plumbing from `compile_binding` (facts now arrive inside the
      catalog overloads it already passes through `builder_from_types`).
- [ ] **Migrate test functions and host tests**
    - `lib/tests/l1/flow_sensitive_narrowing.rs`: the `predicate` helper and
      `registration_rejects_invalid_facts` move from
      `builder.narrows(...)` to signature text carrying the clause (e.g.
      `function(parameters(value: unknown | null); returns(boolean);
      narrows(value: string; true-iff))`); invalid-fact cases become
      grammar/compiler rejections through `builder(...).build()`.
    - `lib/tests/l1/host_functions.rs`: `test_host`'s `is_project_file`
      configure closure becomes `|registration| registration` (the fixture
      catalog now declares the fact); the two setter-based rejection cells
      (unknown parameter on `other`; non-null fact on `project_label`) become
      in-memory catalog edits carrying invalid clauses, failing
      `HostFunctionCatalog::load` — stronger than before (rejected before
      any builder runs) and consistent with rule 7.
    - `lib/tests/l1/host_functions.rs::a_host_catalog_outlives_the_strings_it_was_loaded_from`:
      the runtime-builder half drops its `.narrows` call; the artifact
      assertion is unchanged (Phase 3 blesses the bytes).
- [ ] **Catalog and registration tests**
    - `compose/expression/catalog/tests.rs`: a catalog whose overloads
      disagree on facts (mixed presence, different kinds, different targets)
      fails `load_catalog`; two overloads with identical parameters and
      different clauses still fail as duplicates; a clause naming a
      parameter only the later overload declares fails (the `filter_map`
      regression, exercised through the public loader).
    - `scope/tests.rs` / registry parity: no change expected — the registry
      build now compiles facts from the same descriptors; run the existing
      parity tests.

Validation checkpoint: `just check`; then
`cargo nextest run -p darkmatter --features effects-instrumentation` for
`flow_sensitive_narrowing`, `host_functions` (registration section),
`function_dispatch`, `directive_target_analysis`, `binding_contract`,
`compose_file_predicates` (the guard cases must keep passing — `file`-typed
subjects stay `file` under both the old string and new file targets because
`suitable_for` keeps sub-type arms, scope/shape.rs:409–446), the
`expression_function_catalog` unit tests, and dmls's
`expression_scopes`/`host_globals` (Acceptance 13 and the Claudine-bundle
rows). The artifact freshness test is expected red until Phase 3.

## Phase 3: Host Artifact Comparison and Fixture Regeneration

The artifact stays `format_version: 1` with its required `facts` array as
derived output; the reader's authority flips from "supply the fact" to
"compare with the catalog's compiled fact".

### Tasks

Wave 5 — reader semantics:

- [ ] **Compare, never supply**
    - In `host/artifact.rs::rebuild`: delete the `builder.narrows(...)` call
      (line 371); build the registration from the catalog (facts come from
      the declarations via `build_with`); then compare the **entire** decoded
      artifact fact — `parameter`, `position`, `kind`, and `type` spelling
      including `<non-null>` — against `registration.narrowing_fact()`,
      including absence in both directions: `facts: []` while the catalog
      declares → per-function incomplete ("records no fact, but the catalog
      declares …"); an artifact fact while the catalog declares none →
      per-function incomplete; kind/parameter/position/type-spelling
      mismatch → per-function incomplete. Keep the existing >1-entry
      rejection and the `required`/`nullable` strictness (`facts: null`
      stays a per-function error, absent stays a per-function error).
      Preserve the per-function versus whole-file boundary: whole-file cells
      (absent/unreadable/malformed/duplicate-key/trailing/unknown-field/
      version/hash) behave exactly as today.
    - The writer `facts_artifact()` is unchanged in algorithm — it already
      derives from the registration, which now carries the catalog-compiled
      fact; determinism and sorted keys stay.
    - Evidence: spec Ownership step 5 ("The artifact must never call a
      setter to supply or override catalog narrowing", "compare the entire
      decoded fact with the catalog's compiled fact, including absence").
- [ ] **Reader matrix extension**
    - In `lib/tests/l1/host_functions.rs::
      the_facts_artifact_reader_walks_the_input_robustness_matrix`, add
      fact-comparison cells edited from the committed artifact one field per
      cell: `facts: []` on `is_project_file` (a declared fact recorded as
      absent); an artifact-only fact on `project_label` (a fact entry where
      the catalog declares none); wrong `kind`; wrong `parameter`; wrong
      `position` (already covered, now via comparison); wrong `type` spelling
      (`file` → `string`, and `<non-null>` ↔ type text). Expect `Expect::Incomplete(&["is_project_file"])` /
      `project_label` per cell with reasons, control row proving the
      unedited artifact keeps both functions complete and the guard
      narrowing. Per the matrix rules: one edit per cell, control case
      retained, asserted through the public `analysis_only` result, and no
      handler entries (the counters stay at zero).
- [ ] **DMLS wiring cases**
    - `dmls/tests/l1/host_function_diagnostics.rs`: add rows for a fact
      mismatch artifact (edited `facts: []` / wrong kind) — calls to that
      function report `dm.expression.incomplete_analysis`, the guarded
      `frontmatter` call loses its narrowing-backed compatibility, and the
      whole-file failure rows keep today's behavior. A missing artifact
      still yields incomplete for every function (existing coverage; keep).

Wave 6 — regenerate the companion (depends on Wave 5 + Phase 2's catalog
edit):

- [ ] **Bless and prove freshness**
    - Run the freshness test with `DARKMATTER_BLESS_HOST_FACTS=1` to rewrite
      `lib/tests/fixtures/host_functions/project.facts.json` from the
      migrated fixture (the `catalog_hash` changes because `project.yaml`
      changed; the `facts` entry for `is_project_file` keeps its content —
      same kind/parameter/position/type — proving the derived encoding is
      unchanged); re-run without the variable to prove byte-for-byte
      freshness and determinism.
    - `dmls` lifecycle host-profile tests and `dmls/tests/l1/
      host_function_diagnostics.rs` use the same fixture; all green.

Validation checkpoint: `cargo nextest run -p darkmatter --features
effects-instrumentation host_functions` and `-p dmls --features
effects-instrumentation host_function_diagnostics lifecycle_host_profile`
green; `just check` green.

## Phase 4: Behavior, Passivity, and End-to-End Acceptance

The battery that backs the spec's acceptance criteria. All tasks are
file-disjoint and run in one wave.

### Tasks

Wave 7 — acceptance battery (parallel):

- [ ] **Narrowing behavior tests**
    - `lib/tests/l1/flow_sensitive_narrowing.rs`: with `source: unknown`
      declared, `is_file_type(source)` and `file_exists(source)` true sides
      read `file` (the latter newly so), and `file_exists`'s false side adds
      no exclusion (a valid-but-missing path remains a possible file value);
      the migrated `true-iff` primitives keep both branch shapes
      (`is_string` true → `string`, false → `string` removed from a known
      union; `is_null` true → `null`); `is_empty` false side excludes only
      `null`; aliases (`fileExists`, `isString`) narrow identically; an
      omitted optional argument adds no fact (a predicate whose fact
      parameter is optional, called without it, narrows nothing — via
      `narrowing_fact_for`'s count filter).
    - Subject identity (already covered by existing rows; keep passing):
      explicit and implicit local reads share facts, a `frontmatter(...)`
      evaluation is a separate subject from a like-named local, and repeated
      matching call text creates no identity.
- [ ] **Original-value implication tests**
    - `lib/tests/l1/context_function_contracts.rs`: for each migrated
      predicate, representative inputs prove the declared branch is about the
      original value, including convertible ones — `is_number("4")` is
      `false`, `is_file_type(42)` is `false`, `is_empty` on `0`/`false`/
      `"0"` is `false`, `is_null(null)` true only for null — so the runtime
      never honors the clause by converting first. These are behavior tests
      of the handlers that the declarations describe, not inference from
      samples.
- [ ] **Passivity proofs**
    - Handler-entry counters (`IS_PROJECT_FILE_ENTRIES`, …) stay zero through
      `HostFunctionCatalog::load`, `HostFunctionsBuilder::build`,
      `analysis_only`, and `check_expression_calls` on guarded expressions
      (extend the existing counter assertions in `host_functions.rs`).
    - `lib/tests/l1/shipped_schema_corpus.rs`: the walk now covers
      `functions.yaml` with clauses (already in the corpus — assert it stays
      loadable and the effect counters do not move); add one narrow check
      that every catalog entry carrying `narrows` compiles a fact through the
      public registry (eight entries, `file_exists`/`is_file_type` →
      `TrueImplies`/`file`), proving the shipped catalog corpus is valid
      under the new syntax without executing anything.
- [ ] **Boundary tests**
    - JSON Schema: a document schema whose property reaches a
      `narrows`-carrying `function(...)` still fails conversion at that
      property path with the existing message (convert.rs:1088) — one test
      through `parse_yaml_schema` + conversion.
    - `type-definition` meta-type validation: a string value containing a
      valid `narrows` clause validates; an invalid clause (unknown kind)
      fails with the located grammar error — through
      `format.rs`'s public keyword path.
    - DMLS schema authoring: an authored schema file with an invalid
      `narrows` clause produces the located `dm.schema.*` diagnostic at the
      clause, by extending `dmls/tests/l1/schema_yaml_spellings.rs` (that
      suite already ranges definition-grammar errors inside YAML scalars
      through the editor), proving the provenance path through YAML block
      scalars into DMLS.
- [ ] **CLI end-to-end**
    - `cli/tests/l1/compose_file_predicates.rs`: extend `DOCUMENT` with a
      fourth body line `{{ file_exists(source) ? frontmatter(source,
      'title') : 'none' }}` — an `unknown`-typed `source` accepted at
      `frontmatter`'s file parameter after a `file_exists` guard, the spec's
      "normal `md compose` case" — and update the asserted stdout to include
      the rendered `Target` (the fixture's `target.md` exists). Use the
      ternary form, not `&&`: infix `a && b` lowers to the boolean
      `and(a, b)` (parser.rs::parse_logical_and) and would render `true`,
      while the ternary renders the guarded read's value — the direct
      before/after demonstration of `file_exists`'s string→file migration
      (a plain `string` is not suitable for `frontmatter`'s file parameter,
      so this line previously warned as an incompatible call). The
      always-false warning accounting for malformed literals stays exactly
      one. Keep the fixture's existing `is_file_type` ternary case.

Validation checkpoint: `just test` in `darkmatter/` (library, CLI, DMLS,
zed-dmls — L1 only; no terminal/browser focus), then `just lint`.

## Phase 5: Documentation and Drift Maintenance

Update the record of behavior in the same change, per the repo's drift rules.
All tasks are file-disjoint and run in one wave.

### Tasks

Wave 8 — documentation (parallel):

- [ ] **Function-signature topic**
    - `darkmatter/docs/topics/schemas/definition.md` (Function Signatures):
      add `narrows(name: T; kind)` / `narrows(name; kind)` to the constraint
      table with a compact example, the ordering rule, the
      boolean-returns requirement, the target rules (self-contained single
      types; named references rejected; `true-iff` limited to `null` and
      bare families), the overload-agreement rule, and the
      original-argument implication with the limits of negative narrowing.
      State that the clause is description-only and changes no binding.
- [ ] **Expressions guard reference**
    - `darkmatter/docs/topics/darkmatter-expressions.md`: replace the
      `file_exists(x)` "present string" row (~line 331) and the "a string"
      annotation (~line 372) with the `file` suitability fact; explain that
      `file_exists` is `true-implies` (existence is not `true-iff` — a valid
      path to a missing file remains a possible file value), and link the
      schema-side declaration as the authority.
- [ ] **Host-functions topic**
    - `darkmatter/docs/topics/host-functions.md`: the registration example
      (~line 65) drops `registration.narrows(...)` for a catalog declaring
      the clause; the behavior-facts table row for narrowing moves to the
      catalog column; the artifact section states `facts` is derived and
      compared (mismatch → incomplete), keeping `format_version: 1`.
- [ ] **Skill topics**
    - `.opencode/skill/darkmatter/expressions.md` (the
      `RegistrationBuilder` fact mentions), `schema.md` (the
      `function(...)` grammar surfaces), `library-surfaces.md` (the removed
      public setter), and `compose.md` where `file_exists` narrowing is
      described: same corrections as the docs topics.
- [ ] **Cross-checks**
    - Grep the repo for `\.narrows\(` and `narrows(` in docs/skills to catch
      stragglers; no `docs/dependencies.md` change (no crates added or
      removed); no AGENTS.md change (no workspace layout or convention
      change).

Validation checkpoint: `just doctest` (the `host/mod.rs` module example and
any doc example touched still compile), `just test` and `just lint` one final
time in `darkmatter/`, and a final scan of the plan's checkboxes.

## Input Robustness Matrix

This feature extends a text grammar (the `narrows` clause) and changes a
loader's field handling (the facts-artifact reader's `facts` field). The
default outcome, stated once: **every invalid shape produces a located error
(clause grammar) or a typed load/registration error, runs no handler, and
publishes nothing** (Phase 1 rule 10). The exceptions, and where each
validator's matrix is walked once:

- **Clause fields** (shared validator: `parse_function`'s `narrows` branch,
  tested in-process through `parse_type_definition`, Phase 1 Wave 2):
  absent → no fact (the one non-error exception); every other shape —
  explicit malformed operand, wrong operand type (union/reference target),
  unknown kind, duplicate clause, unknown/rest parameter, extra operands,
  non-boolean returns — is a located error. Presence-only is a first-class
  valid shape, not an absence stand-in. No `serde` touches these fields, so
  the `#[serde(default)]`/`Option<T>` null-conflation smells cannot arise;
  the code-smell grep still runs over `artifact.rs` (unchanged strict
  helpers).
- **Artifact `facts` field** (one loader: the strict JSON reader in
  `artifact.rs`, matrix already walked in `host_functions.rs`, extended in
  Phase 3 Wave 5): `[]` is a valid explicit absence that must match a
  non-declaring catalog; `null` and absent are errors (existing
  `required`/`array` strictness); one entry is decoded and compared whole;
  more than one is an error. Document shapes (duplicate keys, trailing
  content, unknown fields) are unchanged cells of the same loader.
- **Catalog YAML** is one loader (`load_catalog`) already matrix-covered by
  the claudine `host_catalog.rs` and `host_functions.rs` suites; new
  narrows-bearing catalog cells (invalid clause, disagreeing overloads) are
  added in Phase 2 Wave 4 through the public loader.
- A review finding about any cell lists the state of every other cell in the
  same row and column (same validator or loader) before it is accepted.