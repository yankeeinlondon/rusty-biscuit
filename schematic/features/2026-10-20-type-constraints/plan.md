---
total_phases: 8
created: 2026-10-21
phase: 1
agent: "claude/sonnet"
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - schematic/features/2026-10-20-type-constraints/spikes/s1-lossless-numbers.md
    - schematic/features/2026-10-20-type-constraints/spikes/s2-schemars-output.md
    - schematic/features/2026-10-20-type-constraints/spikes/s3-body-encode-reuse.md
skills_files_updated_during_phase_1: []
packages:
    - schematic-runtime
    - schematic-gen
    - schematic-schema
    - schematic-define
    - schematic-definitions
source_files_during_phase_2:
    - Cargo.lock
    - schematic/gen/src/lib.rs
    - schematic/runtime/Cargo.toml
    - schematic/runtime/src/lib.rs
    - schematic/runtime/src/validate/mod.rs
    - schematic/runtime/src/validate/decimal.rs
    - schematic/runtime/src/validate/eval.rs
    - schematic/runtime/src/validate/pattern.rs
    - schematic/runtime/src/validate/plan.rs
    - schematic/runtime/src/validate/violation.rs
    - schematic/runtime/tests/l1/main.rs
    - schematic/runtime/tests/l1/validate_decimal.rs
    - schematic/runtime/tests/l1/validate_eval.rs
    - schematic/runtime/tests/l1/validate_report.rs
    - schematic/runtime/tests/fixtures/compile_fail/path_segment_holds_only_static_names.rs
    - schematic/runtime/tests/fixtures/compile_fail/path_segment_holds_only_static_names.stderr
    - schematic/runtime/tests/fixtures/compile_fail/violation_holds_no_submitted_value.rs
    - schematic/runtime/tests/fixtures/compile_fail/violation_holds_no_submitted_value.stderr
    - schematic/schema/Cargo.lock
    - schematic/schema/src/lib.rs
docs_updated_during_phase_2:
    - docs/dependencies.md
    - schematic/docs/dependencies.md
    - schematic/docs/runtime/README.md
    - schematic/runtime/README.md
docs_created_during_phase_2:
    - schematic/docs/runtime/validation.md
skills_files_updated_during_phase_2:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/SKILL.md
    - .claude/skills/schematic-define/schematic/gen.md
    - .claude/skills/schematic-define/schematic/responses.md
source_files_during_phase_3:
    - Cargo.lock
    - schematic/schema/Cargo.lock
    - schematic/define/Cargo.toml
    - schematic/define/src/builders.rs
    - schematic/define/src/bundle.rs
    - schematic/define/src/constraints/check.rs
    - schematic/define/src/constraints/mod.rs
    - schematic/define/src/constraints/number.rs
    - schematic/define/src/constraints/pattern.rs
    - schematic/define/src/constraints/registered.rs
    - schematic/define/src/lib.rs
    - schematic/define/src/openapi/export/mod.rs
    - schematic/define/src/openapi/export/parameters.rs
    - schematic/define/src/openapi/import/mappings/mod.rs
    - schematic/define/src/openapi/import/mappings/responses.rs
    - schematic/define/src/pagination.rs
    - schematic/define/src/pagination/lower.rs
    - schematic/define/src/pagination/validate.rs
    - schematic/define/src/params/mod.rs
    - schematic/define/src/params/param_def.rs
    - schematic/define/src/registration.rs
    - schematic/define/src/request.rs
    - schematic/define/src/types.rs
    - schematic/define/tests/builders.rs
    - schematic/define/tests/definition_serde.rs
    - schematic/define/tests/endpoint_literal_surface.rs
    - schematic/define/tests/pagination_constraints.rs
    - schematic/define/tests/registered_model_rules.rs
    - schematic/define/tests/value_constraints.rs
    - schematic/definitions/src/openai/mod.rs
    - schematic/gen/src/codegen/api_struct/tests.rs
    - schematic/gen/src/codegen/endpoint_methods/tests.rs
    - schematic/gen/src/codegen/pagination/tests.rs
    - schematic/gen/src/codegen/request_structs/tests.rs
    - schematic/gen/src/definition_gen.rs
    - schematic/gen/src/diagnostics.rs
    - schematic/gen/src/validation.rs
    - schematic/gen/src/ws_codegen/plan.rs
    - schematic/gen/tests/l1/endpoint_auth_output.rs
    - schematic/gen/tests/l1/fresh_output_compile.rs
    - schematic/gen/tests/l1/output_validation.rs
    - schematic/gen/tests/l1/postman_runnable.rs
    - schematic/gen/tests/l1/query_params_codegen.rs
    - schematic/gen/tests/l1/typed_path_output.rs
docs_updated_during_phase_3:
    - docs/dependencies.md
    - schematic/docs/dependencies.md
    - schematic/docs/definition-builders.md
    - schematic/docs/generation/validation.md
    - schematic/docs/io/definition-bundles.md
    - schematic/docs/pagination.md
    - schematic/docs/typed-registration.md
    - schematic/docs/websocket-definitions.md
docs_created_during_phase_3:
    - schematic/docs/value-constraints.md
skills_files_updated_during_phase_3:
    - .claude/skills/schematic-define/SKILL.md
    - .claude/skills/schematic-define/schematic/define.md
    - .claude/skills/schematic-define/schematic/pagination.md
---

# Type constraints implementation plan

## Work summary and success criteria

Preserve value rules (`minLength`, `maximum`, `multipleOf`, `uniqueItems`, `enum`,
and the rest of the spec's keyword table) from handwritten and imported
definitions through OpenAPI import, serialized definitions, emitted Rust
definitions, typed registration, generation, and OpenAPI export. Enforce the
supported **request-side** rules before any network or credential work, through
one shared runtime checker, with errors that never contain submitted values.
Keep the pager's page-size safety, keep fluent setters, and add no response
validation.

The work spans four packages: `schematic-define` (rule metadata, importer,
exporter), `schematic-gen` (plan compilation, `validate()`, `into_parts`
integration), `schematic-runtime` (the `validate` module, exact numbers,
patterns, pager reuse, `ABI` bump), and `schematic-definitions` (regenerated
artifacts only).

Success looks like all of the following, each with evidence in
`implementation-log.md`:

- Every supported keyword round-trips by **meaning** through direct OpenAPI
  import/export **and** through `import --definitions-out` → assembly →
  registration. A large integer bound and a decimal `multipleOf` survive
  unnarrowed.
- A violating request yields the existing `InvalidArgument` error shape and
  **zero** transport sends through `.build()`, ordinary await, `.full()`,
  `client.send(request)`, and `send_raw`. A valid request sends unchanged data.
- `request.validate()` returns every violation in deterministic order with safe
  paths (`query.limit`, `body.items[2].displayName`, redacted map keys) and no
  submitted values anywhere in `Display`, `Debug`, or sources.
- GitHub's `.page_size(500).fetch_page().await` still refuses before sending,
  now through the shared helpers; invalid sizes changed mid-traversal are still
  caught.
- A structurally decodable response outside a declared bound still decodes.
- `just test`, `just lint`, `just doctest`, and (when committed clients change)
  `just test-schema` pass from `schematic/`; the current documentation, skills,
  and dependency docs describe the new behavior.

The terminal state is **implementation complete, ready for review**. The agent
never commits, moves the feature to `_completed`, or runs `just complete`.

## How to read this plan

- A **wave** is an execution step; wave numbers are unique across the plan and
  waves run in order. Tasks in one wave may run concurrently **only** when their
  write ownership is disjoint (stated per task). A single-task wave is
  sequential.
- An **integrator** owns shared manifests (`Cargo.toml`, `Cargo.lock`), justfiles,
  test roots, regeneration, and committed artifacts. Parallel subagents propose
  edits to those rather than racing writes.
- **Every subagent brief must state that the entire session tree is
  non-interactive** and must name its write ownership.
- Every code phase updates the `///`/`//!` docs and inline comments of each
  symbol whose behavior it changes, plus the relevant `docs/` page, README
  overview, and skill, in the same phase (Phase 8 audits, it does not first
  write them). Comments and docs describe behavior without linking to this
  feature.
- Every phase checkpoint runs `just test` and `just lint` from `schematic/`.
  Regeneration goes through the existing recipes, never hand edits. Do not run
  `cargo fmt`, raw `cargo test`, or commit.
- Load the `schematic-define`, `rust-testing`, and `os` skills before the phases
  that touch their areas; load `os` before path-comparison or `#[cfg(windows)]`
  code. Use the test toolkit and nextest conventions.

## Phase 1: Contracts, rulings, spikes, and baselines

### Necessary Rules

Classification follows the unattended-run convention. **No rule is blocking.**
Adopted defaults are recorded in `implementation-log.md` as
"adopted default: {choice} (source: {section})" and are reviewed by the author
afterward.

| # | Kind | Rule |
| - | ---- | ---- |
| 1 | **decided** | **JSON bodies are validated on the serialized representation, and that representation is the one sent** (source: Open Questions → bodies, option 1, ruled 2026-10-21). Generated `into_parts` serializes the body to a `serde_json::Value` once, evaluates the plan, then encodes that same `Value`; there is no serialize-check-reserialize gap. When a body's plan has **no** rules, the existing direct serialization path is kept unchanged (no temporary tree, no byte change). Standalone `validate()` serializes separately; `.build()` and await remain separate preparations, as today. Consequence to document: for constrained bodies, object key order on the wire follows `Value` ordering and may differ from struct order (JSON-equivalent). |
| 2 | **decided** | **Formats are annotation-only** (source: Open Questions → formats, option 1, ruled 2026-10-21). `format` is stored, exported, and labeled "annotation only" in generation diagnostics and rustdoc; nothing rejects a request on `format`. No `chrono` or format-checker dependency is added; the format table in the spec stays a proposal. |
| 3 | **decided** | **Exact numeric rules use one exact decimal helper** (source: Open Questions → numerics, option 1, ruled 2026-10-21). Range, `multipleOf`, `enum`/`const` equality, and `uniqueItems` all compare through a runtime `Decimal` (sign, big-integer mantissa, decimal exponent). Dependency: reuse a big-integer crate already in `Cargo.lock` (`num-bigint` preferred) if one exists; otherwise add `num-bigint`. No floating point and no tolerance. Exponents beyond a documented bound (default ±10 000) are a located definition error, not an allocation hazard. The final dependency choice is made in Wave 3 from spike S1's evidence and logged. |
| 4 | **decided** | **Registered models' stored component schemas stay authoritative** (source: One source for model rules). No competing editable constraint list on registered fields; no change to component names, Rust paths, collision handling, or registration order; an endpoint-specific rule never mutates a shared schema. New metadata exists only for parameters, inline body roots, form fields, and imported declarations needed to rebuild schemas. |
| 5 | **decided** | **Page-size bounds keep two sites** (source: Pagination integration). Declared min/max lower into the shared runtime helpers *and* the pager engine keeps calling them after each effective-size choice. The engine's validation site is not removed. Conflicting parameter/strategy bounds fail definition validation. |
| 6 | **decided** | **No response validation**, no opt-in mode. WebSocket schemas, binary/file bodies, and OpenAPI 3.1 import are out of scope (source: Non-goals). |
| 7 | **decided** | **`ABI` increments** by one from its value at implementation time (`4` when this plan was written; earlier features in the queue may raise it) in `runtime/src/lib.rs` in the same change that makes generated code require `schematic_runtime::validate` (source: Shared helpers; the constant's own doc comment). |
| 8 | **decided** | **Default failure outcome.** Every invalid constraint shape produces a *located* definition error (API, endpoint when applicable, schema path, keyword, reason), generates nothing, and sends nothing. Only the exceptions in the Input Robustness section behave otherwise. |
| 9 | **defaulted** | **Public names** are fixed here so later waves agree (source: "specify the final method names in the implementation plan"). Runtime: `schematic_runtime::validate::{Violation, Violations, Keyword, PathSegment, Plan, Pattern, Decimal}`; request method `fn validate(&self) -> Result<(), Violations>`. Define: `Constraints` struct with fluent methods `min_length`, `max_length`, `pattern`, `format`, `minimum`, `maximum`, `exclusive_minimum`, `exclusive_maximum`, `multiple_of`, `min_items`, `max_items`, `unique_items`, `allowed_values`, `constant`, and `items(Constraints)` for element rules; attached via `ParamDef::constraints(..)` and the body/form builders' `.constraints(..)`. Numeric inputs to the builders accept `i64`, `u64`, `f64`, and a decimal `&str` (the string form is the lossless route). Names may change only if a doctest proves a collision, and the change is logged. |
| 10 | **defaulted** | **Pattern admission subset** (source: Shared helpers → patterns). Enforce a pattern only when a `regex-syntax` AST walk proves ECMAScript-equivalent meaning: literals, anchors, groups, alternation, quantifiers, and explicit character classes; `\d \w` compiled with Unicode off. `.`, `\s`, `\b`, `\p{..}`, inline flags, and anything Rust rejects (look-around, back-references) are *not* enforced. Authored patterns outside the subset fail definition validation; imported valid patterns outside it stay unenforced with a schema-path warning; syntactically invalid imported patterns get an "invalid pattern" diagnostic, not an "unsupported" one. The admitted/rejected table is a Phase 3 deliverable and is documented. |
| 11 | **defaulted** | **Violation type shape** (source: "concrete result type is fixed in the plan"). `Violation { path, keyword, reason }` where `path` is a `Vec<PathSegment>` (name, index, or redacted-key), `keyword` is a `Keyword` enum, `reason` is a static-text-plus-numbers `String`. `Violations` is an ordered collection implementing `Display`/`Debug`/`Error` with aggregate text. It converts into the existing `InvalidArgument` reason; no public error enum changes. |
| 12 | **decided** | **Authorizations.** Starting this run authorizes everything the spec asks for: regenerating affected imported definitions and client/OpenAPI artifacts through existing recipes, adding `regex` and an exact-number dependency to the packages that need them, and documentation/skill updates. Not authorized: commits, pushes, CI changes, live provider calls, lifecycle moves, hand-edited generated output, and replacing the OpenAPI parser. Scope the spec does not mention is logged and left undone. |
| 13 | **defaulted** | **No performance spike.** The spec states none is required. Do not add one; if the author wants the temporary-tree cost measured, it is one host and a quick sample, listed as a review-agenda item. |

### Spikes

Each spike runs once, in Wave 2, before the work it informs; none is repeated in
later phases. Results are recorded in `implementation-log.md` and, where they
change a default above, as an adaptation.

- **S1 – Lossless number path.** On this host, push three values (`9223372036854775808`,
  `1e30`, `0.1`) through: `serde_json::Value` round trip, the importer's text
  pre-pass (`clamp_numeric_bounds` in `define/src/openapi/import/normalize.rs`),
  `openapiv3` parsing, and `openapiv3` serialization in the exporter
  (`define/src/openapi/export/conversion.rs`). Record where precision is lost.
  Decide between (a) scoped `serde_json/arbitrary_precision` (rejected by default:
  it is workspace-wide feature unification and alters untagged/flatten
  deserialization) and (b) **default**: carry exact decimal text out-of-band
  (decimal strings in the new constraint metadata; exporter patches rendered
  schema text from that sidecar rather than trusting `openapiv3` numerics).
- **S2 – Schemars output fixture.** Inspect the real Schemars 1 output for
  `Option<T>`, nested/inner element rules, and each of `range`, `length`,
  `pattern`, `inner`, and `extend(..)` for `multipleOf` and exclusive bounds.
  Record exactly which keywords attributes can express and what nullable shapes
  (`type: [T, "null"]`, `anyOf` with a null branch) look like, so traversal does not
  assume every `anyOf` is nullable.
- **S3 – Body encode reuse.** Read `runtime/src/encode.rs` and the generated
  `into_parts` for body handling and confirm a prebuilt `serde_json::Value` can
  be encoded without reserializing the typed model. If not, record the minimal
  runtime addition (no public API change beyond `validate`).

### Wave 1: Dependency and contract check

- [x] **Confirm prerequisites**
  - Read `2026-10-14-schematic-definition-api` and `2026-10-12-schematic-endpoint-methods`
    implementation state. Verify the spec's "current contracts" against code:
    `FieldDef`, `ParamDef`, `SchemaComponent`, `EndpointSpec::into_parts`,
    `Call::build`, the pager's page-size check, and the importer/exporter files it
    cites. Check `git status` for unrelated user changes before writing anything
    (the spec itself is modified; leave it alone).
  - **Queue position (ruled 2026-10-21).** This feature runs last in the
    Schematic series, after `2026-10-18-schematic-fidelity-polish`, following
    the series' date order. This plan was written against the code before
    `2026-10-15-schematic-definition-vocabulary`,
    `2026-10-16-schematic-response-model-graphql`,
    `2026-10-17-schematic-websocket`, and fidelity-polish landed. Refresh every
    cited file, symbol, and count against the landed code and log each
    adaptation. In particular, typed path parameters (vocabulary) are
    `ParamDef`s and take constraints like any other parameter; the version-1
    JSON definition bundle (vocabulary) must carry the new constraint metadata,
    and its strict loader's input matrix gains the constraint cells; and
    fidelity-polish's generated Postman examples and `x-schematic` metadata must
    stay consistent with the exported constraints.
  - Record discrepancies in the log. If typed registration or the pager contract
    is genuinely absent, that is an evidence-based blocker per the convention;
    otherwise proceed against the code as it exists.
  - Create `implementation-log.md` in this directory: spec/plan identifiers,
    implementer, packages touched, per-phase files, adopted defaults, deviations,
    review agenda.
- [x] **Inventory touchpoints**
  - Locate (do not edit): where `import --definitions-out` emits Rust definitions
    and Schemars attributes; the parameter/body/form builders; where generated
    code is asserted against `ABI`; `just _doctest`, `test-schema`, and
    `generate*` recipes; the existing generation guard test harness to reuse.
  - Record the dependent-package list for `just test` scope and which committed
    clients/OpenAPI artifacts regeneration will touch.

### Wave 2: Spikes (parallel, read-only except scratch fixtures)

- [x] **Spike S1**
  - Ownership: scratch crate/fixture directory only, outside package sources.
  - Output: a table of loss points and the chosen carry strategy.
- [x] **Spike S2**
  - Ownership: scratch fixture only. Output: keyword/attribute expressibility
    table and a nullable-shape catalogue that becomes a Phase 5 fixture.
- [x] **Spike S3**
  - Ownership: read-only. Output: one-paragraph body-reuse design for Phase 5.

### Wave 3: Baselines and decisions

- [x] **Capture baselines**
  - Run `just test` and `just lint` to record the starting state; confirm the
    GitHub `.page_size(500).fetch_page().await` refusal exists today and keep its
    test as the regression anchor.
  - Count existing constraint keywords present across committed OpenAPI
    artifacts and imported definitions (a derived number, not hard-coded) to size
    the preservation fixture and regeneration diff.
- [x] **Freeze decisions**
  - Apply S1–S3 to rules 1, 3, and 9; fix the exact-number dependency; amend the
    log with any adaptation. Integrator adds the dependency stanzas later, in the
    phase that first needs them.

**Checkpoint 1:** log exists with rulings, spike results, baselines, and touchpoint
inventory; the rule table has no unresolved entries; `just test`/`just lint`
baseline recorded.

## Input robustness approach (applies to Phases 3 and 4)

These phases add loaders and validators for constraint metadata, so the matrix
below governs their tests. **Default outcome:** every invalid shape produces a
located definition error or diagnostic, generates nothing, and sends nothing. Only
the exceptions below differ.

**Loaders (document shapes), tested once per distinct loader.** Read the code to
confirm which formats share a loader before writing cells:

| Loader | Cells (control + invalid) |
| ------ | ------------------------- |
| Serialized definitions/snapshots (JSON/YAML/TOML via the existing conversion route) | control with new metadata; duplicate key; trailing garbage; format-specific syntax (YAML `k:` null, second YAML document); old snapshot without the metadata reads as absent |
| OpenAPI 3.0 document (JSON/YAML) | control; duplicate keyword; bounds as Boolean-flag exclusives; a bound too large for `i64`/`f64` (must fail with its path or be captured exactly, never clamped) |

Spawn the CLI only to prove wiring: per loader, the control plus one or two
invalid shapes, asserting the error location in rendered output and that nothing
launched/generated. All other cells run in-process at the crate's public API.

**Field shapes, tested once per field validator** (the validators are the
numeric-bound, length/count, pattern, `enum`/`const`, `format`, and `items`
checks; fields sharing a validator share its matrix, each other field needs a
control plus one invalid shape proving it is wired in):

- *Exceptions to the default:* absent → no constraint (and the key is omitted on
  write); `pattern: ""` is valid and matches everything; `format` of an unknown
  name is preserved as an annotation; `enum`/`const` apply to whole JSON values
  and an explicit JSON `null` member is data, not absence; `[]` for `enum` is a
  definition error; `uniqueItems: false` is the same as absent.
- *Never conflate:* absent vs explicit `null` (`"maxLength": null` is malformed,
  not absent); wrong type for the whole field (`"maxLength": "5"`); wrong element
  inside `enum`-typed constraint lists; empty vs absent. Only the shapes the
  field's type allows apply.
- *Semantic definition errors:* negative length/count, nonpositive `multipleOf`,
  empty `enum`, `minimum > maximum`, `minItems > maxItems`, equal exclusive
  endpoints, incompatible authored type (e.g. `min_length` on an integer),
  constraint attached to a binary/file body.

**Smell grep before declaring the matrix done** (record results in the log):
`#[serde(default)]` on a load-bearing constraint field; `Option<T>` where absent
and null must differ (use a `present` deserializer or `Option<Option<T>>`);
`filter_map(.. as_str())`, `unwrap_or_default()`, or `.ok()` on a load-bearing
parse in the new code. A review finding about one cell must list the state of
every other cell in the same row and column of that validator or loader.

## Phase 2: Runtime validation core

Build `schematic_runtime::validate` as a self-contained, well-tested library
before anything depends on it. Nothing is wired into generation yet.

### Wave 4: Foundations (parallel; disjoint files)

- [x] **Decimal type**
  - New `runtime/src/validate/decimal.rs`: parse from JSON number text,
    compare, test divisibility (`multipleOf` with positive divisor), equality
    (`1 == 1.0`), exponent-bound guard. Integrator adds the Phase 1-chosen
    dependency.
  - Tests: boundary integers beyond `u64`, `0.1` divisor against `0.3`/`0.35`,
    `1` vs `1.0` vs `1e0`, negative values, exponent over the bound.
- [x] **Violation types**
  - New `runtime/src/validate/violation.rs`: `Violation`, `Violations`, `Keyword`,
    `PathSegment`, deterministic ordering helpers, `Display`/`Debug` that carry
    only path, keyword, and numeric limits.
  - Tests: ordering (declaration order → object-key order → index → keyword);
    redacted map-key placeholder; compile-fail-style test that no constructor
    accepts a submitted value.
- [x] **Pattern cell**
  - New `runtime/src/validate/pattern.rs`: const-constructible `Pattern` with lazy
    `OnceLock<Regex>`, compile once, never panic, never compile per request (a
    runtime compile failure becomes an unenforced-skip with no panic, since
    generation already proved validity). Integrator adds `regex`.
  - Tests: unanchored search semantics, compile-once (counter in test build),
    Unicode input.

### Wave 5: Evaluator

- [x] **Plan and evaluator**
  - New `runtime/src/validate/plan.rs` and `eval.rs`: a data-only `Plan`
    (string/number/array/enum/const rules, object properties, array items, map
    values, named references for recursion, nullable wrapper) and an evaluator over
    `serde_json::Value`. Type-specific rules apply only to matching JSON kinds;
    string length counts code points; `uniqueItems` uses deep equality with
    object member order ignored and `Decimal` number equality; `enum`/`const`
    whole-value equality. Recursion follows the finite value, referencing named
    plans.
  - Typed (non-JSON) entry points for parameters: `check_str`, `check_number`,
    `check_array_len` helpers that run **before** joining/exploding.
  - Tests per validator (the field-shape matrix above): length with astral
    characters; inclusive/exclusive endpoints; decimal divisibility; absent vs
    empty vs `null`; unique arrays containing equal objects with differently
    ordered members; multiple violations; recursion over a finite nested value.

### Wave 6: Integration surface

- [x] **Export and ABI**
  - Wire `validate` into `runtime/src/lib.rs`, increment `ABI` by one (rule 7), update its doc
    comment expectations, and add `docs/runtime` page content for the module.
  - Add the `Violations` → `InvalidArgument` conversion (reason text only; no
    change to public error enums).

**Checkpoint 2:** runtime unit tests green via `just test`; `just lint` clean;
doctest for each public helper compiles (`just doctest`); no generated crate
changed yet, so no regeneration.

## Phase 3: Definition model, authoring, and validation

### Wave 7: Data model (sequential; one owner for the shared types)

- [x] **Constraint types**
  - New `define/src/constraints.rs`: `Constraints` with the fluent builders from
    rule 9, `ConstraintNumber` stored as decimal text (lossless), item rules,
    serde with fields omitted when absent and reads-as-absent for old snapshots.
    Load-bearing fields use a `present` deserializer so `null` is distinguished
    from absent (no bare `#[serde(default)]`).
  - Attach to `ParamDef`, inline body roots, and form fields via builders;
    keep struct-literal construction unnecessary for authors.
  - Compiling doctests for every builder (`just doctest`).

### Wave 8: Validation and pagination lowering (parallel; disjoint files)

- [x] **Definition checks**
  - Validate at API build and generator entry, including deserialized
    definitions: all semantic definition errors from the robustness section,
    located with API/endpoint/schema path/keyword/reason, using the existing
    diagnostic conventions.
  - Pattern admission (rule 10): `regex-syntax` AST walk; produce the
    admitted/rejected table as a test fixture and as docs. Integrator adds `regex`.
  - Tests per validator plus the two loader wiring checks and the control rows.
- [x] **Pagination lowering**
  - Lower declared page-size min/max (in `define/src/pagination*`) into
    `Constraints` for the adopted parameter so OpenAPI/rustdoc show them; fail
    definition validation when parameter and strategy bounds conflict or
    disagree.
  - Tests: agreeing bounds pass; conflicting min, conflicting max, and one-sided
    declaration each fail with a located error.
- [x] **Registered-model rules**
  - Document and test Schemars attributes on handwritten fields as the single
    source (fixture from S2), including root and nested types and unsupported
    composition detection (`oneOf`/`anyOf`/`allOf`/`not` or unresolved reference
    over a constrained request value → classification consumed in Phase 5).
  - Tests only plus a classification helper; no change to component names/order.

**Checkpoint 3:** `just test` and `just lint` green; doctests compile; matrix
smell-grep run and logged; docs for definition builders updated
(`docs/definition-builders.md`, `docs/typed-registration.md`).

## Phase 4: OpenAPI import, definitions-out, and export

### Wave 9: Lossless capture (sequential; touches shared normalization)

- [ ] **Replace clamping**
  - Replace constraint-value clamping in `normalize.rs` with lossless capture
    per S1 (exact decimal text sidecar); a source number that cannot be
    represented fails with its path. Never clamp, round, or narrow. Keep any
    non-constraint normalization the importer still needs, and update the
    diagnostics that referred to clamping.
  - Normalize OpenAPI 3.0 Boolean `exclusiveMinimum`/`exclusiveMaximum` flags
    with their bounds into internal numeric-bound meaning; lower `const` support
    to singleton `enum` on export.

### Wave 10: Import and export (parallel; disjoint files)

- [ ] **Importer**
  - In `define/src/openapi/import/mappings/` retain constraints on parameters,
    inline bodies, form fields, array items, aliases, nested-only components,
    renamed fields, and a constrained reference used by several endpoints,
    without mutating the shared component for one endpoint.
  - Emit rebuilt Schemars attributes where expressible (S2 table) and explicit
    schema metadata where not, so `import --definitions-out` followed by
    assembly keeps every rule. Produce the unsupported-rule diagnostics
    (path, keyword, reason; no warnings for `title`/`description`).
  - Tests: direct import; definitions-out → assembly; response-only constraints
    preserved without generating checks.
- [ ] **Exporter**
  - In `define/src/openapi/export/{conversion,parameters,request_body,components}.rs`
    emit constraints for parameters, bodies, forms, and components with the
    reversed exclusive-bound mapping, `const` as singleton `enum`, nullable
    schemas keeping their value rules, and lossless numerics (sidecar patching
    or an error; never the `openapiv3` numeric types for constraint values).
  - Tests: meaning-level round trips (not byte spelling), renamed references,
    large integer bound, decimal `multipleOf`.

### Wave 11: Compact preservation fixture

- [ ] **Fixture and acceptance**
  - One compact fixture covering every supported keyword: inline roots, aliases,
    renamed properties, arrays and items, local recursive references, exclusive
    bounds, `const` lowering, a large integer bound, and a decimal `multipleOf`.
  - Two separate acceptance tests: direct import/export, and emitted typed
    definitions rebuilt through registration. Both compare meaning.

**Checkpoint 4:** `just test`, `just lint`, `just doctest` green; OpenAPI
loader wiring cells (CLI) pass; no byte-for-byte golden drift except
explained exclusive-bound/`const` spelling changes, which are listed in the log.

## Phase 5: Generation and send integration

### Wave 12: Plan compilation (sequential; one owner of `gen` plan code)

- [ ] **Compile plans**
  - In `schematic-gen`, compile each endpoint's parameters, body, and forms into
    runtime `Plan` data (reusing named plans so recursion terminates), resolving
    local component references from the stored schemas. Compile every admitted
    pattern during generation so invalid ones surface before emission.
  - Generation **fails with an actionable error** for a supported request rule
    under unsupported composition or an unresolved reference (path, keyword,
    reason); unenforced rules (formats, non-admitted patterns, unknown keywords)
    become rustdoc labels and diagnostics, not errors, except where the spec
    requires failure.

### Wave 13: Emission (parallel; disjoint generated-code modules)

- [ ] **`validate()` method**
  - Emit `validate(&self) -> Result<(), Violations>` on each request with
    supported constraints in `request_structs/`, returning all violations in
    deterministic order (parameter declaration → body traversal), with
    location-prefixed names (`query.limit`) and serialized body paths.
- [ ] **`into_parts` integration**
  - In `gen/src/codegen/request_structs/into_parts.rs` run the same checks
    before returning parts so endpoint calls, `client.send`, `.full()`, and
    `send_raw` are all covered. Parameters are checked on typed values before
    joining/exploding (array length first). JSON bodies follow rule 1 (serialize
    once, check, encode that `Value`); text bodies and textual/JSON form fields
    are checked before URL encoding or multipart assembly. Binary/file bodies
    with attached rules are rejected at definition validation (Phase 3), never
    silently skipped.
  - Serialization failure remains the existing preparation error.
- [ ] **Docs and diagnostics**
  - Generated field rustdoc distinguishes enforced rules from preserved
    annotations and unenforced patterns/formats. Update module docs.

### Wave 14: Generation tests

- [ ] **Generation tests**
  - Reuse the existing generation guard: fresh generated code compiles and uses
    a mock transport. Cover build, ordinary await, `.full()`, direct `send`, and
    `send_raw` for violating (error shape + zero transport sends) and valid
    (sent unchanged) requests; direct field mutation after an early `validate()`;
    a custom-`Serialize` body (renamed, skipped, flattened fields); secret
    values in bodies and map keys never appearing in `Display`/`Debug`/sources;
    unsupported composition producing the generation error.

**Checkpoint 5:** generation tests green via `just test`; `just lint` and
`just doctest` clean; `just generate-dry-run` shows an expected, reviewed diff.

## Phase 6: Pager integration

### Wave 15: Engine reuse

- [ ] **Shared helper in engine**
  - In `runtime/src/pager/engine.rs` keep the page-size checks after each effective
    size selection (initial knobs, sent defaults, mid-traversal changes) but
    route them through the shared `validate` helpers so one implementation
    supports both sites. Preserve provider next-link size handling unless the
    caller overrides it. Keep the error shape for pager planning problems.
  - Do not teach `.per_page(..)` on endpoints where the pager adopted that
    parameter in any docs.
- [ ] **Pager tests**
  - Initial invalid size; invalid size changed during traversal; adopted sent
    defaults; conflicting parameter/strategy bounds (definition-time, from Phase
    3); GitHub max remains 100 and its controls remain; the `.page_size(500)`
    regression asserting zero sends.

**Checkpoint 6:** pager tests green; `just test`/`just lint` clean; pagination
docs (`docs/pagination.md`) updated.

## Phase 7: Regeneration and end-to-end evidence

### Wave 16: Regenerate (sequential; integrator only)

- [ ] **Regenerate artifacts**
  - Through the existing recipes only (`just generate`, `generate-openapi`,
    `generate-postman` as applicable, plus `import-*` for imported definitions):
    regenerate affected definitions, clients, OpenAPI documents, and any
    drift-checked outputs. Review the diff against the Phase 1 baseline; explain
    every change (new `validate()` methods, constraint metadata, rustdoc
    labels). Never hand-edit generated files.
  - Run `just test-schema` for committed client changes and `just check-drift`.

### Wave 17: Cross-cutting acceptance (parallel; read-only checks and test files)

- [ ] **No response enforcement**
  - Test: a structurally decodable reply outside a declared value bound still
    decodes as before.
- [ ] **Cross-OS review**
  - Load the `os` skill. Review new code for path/regex/locale assumptions;
    confirm the pure-Rust paths compile on macOS, Linux, native Windows, and
    WSL2. Evidence is the pushing host's local run; do not claim other
    environments passed without evidence, and record what was unexercised.
- [ ] **Matrix audit**
  - Run the smell grep and confirm every validator row/column has a control and
    an invalid-shape cell; list any exception in the log.

**Checkpoint 7:** `just test`, `just test-l2` (if the area has L2 for touched code),
`just lint`, `just doctest`, `just test-schema`, `just check-drift` all green;
diff explained.

## Phase 8: Documentation, skills, and closure

### Wave 18: Docs and skills (parallel; disjoint files)

- [ ] **Topic docs**
  - Update `docs/typed-registration.md`, `docs/definition-builders.md`,
    `docs/authoring-examples.md`, `docs/pagination.md`, the generation and
    runtime pages, and OpenAPI conversion docs. Written for a developer new to
    the repository: lead with what the reader can do, a compact example per
    rule, a Mermaid flow for "where a request is checked" (build vs await vs
    pager) and for the import → definitions-out → registration round trip.
    Mark nothing "planned" that now exists; do not link or name this feature.
  - Document the enforced subset honestly: formats annotation-only, pattern
    subset table, exact-number behavior, JSON body key-order note, and that
    responses are not validated.
- [ ] **Skills and dependency docs**
  - Update the Schematic skills (`schematic-define` and any generation/runtime
    skill) with the new builders and error behavior; update
    `docs/dependencies.md` for `regex`, the exact-number dependency, and any
    `regex-syntax` use. Keep READMEs overview-only.

### Wave 19: Closing audit

- [ ] **Audit comments and drift**
  - For every behavior-changing symbol, confirm `///`/`//!` and inline comments
    match the code; code wins on drift, and each drift found is recorded with how
    it was resolved. Grep for dead references to clamping behavior.
- [ ] **Final evidence**
  - Re-run only checks whose inputs changed since Checkpoint 7. Complete
    `implementation-log.md` with commands, environment, results, changed files,
    adopted defaults, deviations, unexercised environments, and the review
    agenda (including the optional performance sample from rule 13).
  - Set the spec's implementation fields only as the repository workflow
    allows; do not move the feature to `_completed` or run `just complete`.

**Checkpoint 8 (final):** every success criterion at the top has evidence in the
log; terminal state **implementation complete, ready for review**.
