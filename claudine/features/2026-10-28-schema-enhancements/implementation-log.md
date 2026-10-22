---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/claudine/features/2026-10-28-schema-enhancements/spec.md"
plan: "claudine/features/2026-10-28-schema-enhancements/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/mod.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/detected.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.md-get.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/frontmatter.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.json
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/keywords.schema.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/slash-commands.shipped.yaml
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/timing.shipped.yaml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/main.rs
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.toml
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/src/Cargo.lock
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-28-schema-enhancements/inventory/criteria-map.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/prerequisites.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/function-registry.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/ownership.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/reader-matrix.md
    - claudine/features/2026-10-28-schema-enhancements/inventory/rulings.md
    - claudine/features/2026-10-28-schema-enhancements/behavior-ledger.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/findings.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/default.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/arbitrary-precision.md
    - claudine/features/2026-10-28-schema-enhancements/spikes/numeric-ingress/runs/preserve-order.md
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/PROVENANCE.md
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-gen
    - sniff-cli
human_review: false
source_files_during_phase_2:
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/opencode_config.rs
    - claudine/lib/src/provider/tests.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_grammar_types.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/lifecycle/actions.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/expression/ast.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/replacement.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/numeric.rs
    - darkmatter/lib/src/markdown/numeric/ingress.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/phase.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/signature.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/categories.documented.yaml
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/numeric_admission.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/frontmatter.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_empty_tuple_and_category_annotation.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_fixed_tuple.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_union_operator.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schema_grammar_descriptors__snapshot_variadic_tuple_with_optional_position.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_string_with_pattern_and_array_constraints.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_array_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_any.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolean.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_boolish.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_date.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_datetime.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_email.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_enum_members.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_array_min_items.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_file_match.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number_integer_with_min.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_number.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_numberlike.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_object.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_min_max.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_not_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string_pattern.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_string.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_time.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_url_scheme.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_atom_with_description.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_default_and_required_atoms.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_enum_with_default_and_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_full_document_schema.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_as_union_arm.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_bare.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_constrained_array.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_empty.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_mixed_with_string_fallback.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_multi_line.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_nested.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_inline_object_required_value.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_basic.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_matching_default.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_property_union_hoists_required.snap
    - darkmatter/lib/tests/l1/snapshots/l1__schemas_convert_snapshots__snapshot_root_union_inline_only.snap
    - darkmatter/lib/tests/l1/snapshots/l1__suggest_constraint_phase3__conversion_snapshot_covers_valid_and_invalid_metadata.snap
    - sniff/cli/src/commands/filesystem_query.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/tests/l1/snapshots.rs
docs_updated_during_phase_2:
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/dependencies.md
    - docs/dependencies.md
    - darkmatter/lib/tests/fixtures/schema_enhancements_matrix/PROVENANCE.md
docs_created_during_phase_2:
    - darkmatter/docs/topics/numbers.md
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/schema.md
message_to_agent: |-
  Phase 2 (Wave 2) is implemented; read implementation-log.md "## Phase 2" first. What Phase 3 builds on:
  - Numbers: darkmatter::markdown::numeric is the only way a number enters Darkmatter. Use admit_number_text for text -> number (the one lenient parser), admit_json_number/admit_f64 for values without a spelling, json_number to store and format_number to render. Frontmatter YAML, raw JSON/YAML schemas, schema-file YAML (standalone.rs and resolve.rs both go through numeric::load_admitted_yaml), SimplifiedSchema metadata, and expression literals are admitted AT LOAD, so the conversion core never sees an inexact native number; it still must admit converted text.
  - Grammar AST (schemas/simplified/types.rs): TypeExpr::Tuple(TupleType{elements: TupleElement.., spread}), TypeExpr::Function(Box<FunctionType>) with FunctionParameters/FunctionParameter/CallExample/ExampleOutcome, Constraint::Category. `(A | B)[]` is the existing TypeExpr::Union with is_array; `id: A | B` parses to PropertyDef::Union exactly like the YAML list. Tuples lower to prefixItems/minItems/items; functions are rejected in to_json_schema (the one choke point). Duplicate-overload contracts: simplified/signature.rs.
  - Still on the old engine: schemas/coerce.rs has no tuple (prefixItems) handling, and today's frontmatter coercion turns `true` into "true", so criterion 47's document-level rejection of [true] and every conversion criterion wait for your core. coerce_to_number leaves inexact text unconverted and validate.rs appends the admission reason; replace both with the core.
  - serde_json preserve_order is on for every package linking darkmatter. Alphabetical outputs sort explicitly (md schema validate JSON, md hash non-strict frontmatter hash, Claudine OpenCode/Kilo config, sniff-cli JSON). Do not mechanically refresh snapshots.
  - Hazards in this shared worktree: zsh CDPATH sends a relative `cd claudine` to /Volumes/coding/personal/rusty-biscuit (a different checkout), so always cd with absolute paths and check `pwd -P`; an outside process may commit or briefly roll back files mid-phase (it did in Phase 2). `git apply --3way` stages files; unstage with `git reset -q -- <files>`.
  - Known load-sensitive test: dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back fails only under full-suite load; main fixes it in 43798e829, not yet on feat/pipeline.
---

# Implementation Log for 2026-10-28-schema-enhancements (6 phases)

## Phase 1

Phase 1 is inventory and a one-time numeric-ingress spike. It changes no
production code.

### What was produced

| Plan task | Output |
|---|---|
| Contract inventory: criterion → task/test map | `inventory/criteria-map.md`: all 61 criteria (1–59, 33a, 33b) mapped to a plan task, a proposed L1 test identity, and the public result the test observes |
| Contract inventory: prerequisites | `inventory/prerequisites.md`: checked in code, not by spec status. `core-schema-types`, `union-type-constraints`, `fm-namespace` and `type-narrowing` are all **supplied**; the inline `(A \| B)[]` spelling and the declared-type projection are this feature's own work (Phase 2 Grammar, Phase 5 Variable projection) |
| Contract inventory: registry and catalog | `inventory/function-registry.md`, enumerated from the registry source: 113 canonical functions (62 pure, 49 context, 2 lazy: `and`/`or`), 75 aliases; the current catalog matches the registry exactly |
| Contract inventory: behavior ledger | `behavior-ledger.md`: one draft row per canonical function with null family, fixture applicability and preserved/changed status. Reviewed and approved by the author on 2026-10-22 |
| Contract inventory: API/file ownership | `inventory/ownership.md` |
| Contract inventory: reader fields and matrix expansion | `inventory/reader-matrix.md` (with a "Cell coverage" appendix) |
| Contract inventory: rulings | `inventory/rulings.md` (R1–R8, all pending the author) |
| Numeric ingress spike (ran once) | `spikes/numeric-ingress/findings.md`, throwaway crate `spikes/numeric-ingress/src/` (outside the workspace), captured runs in `spikes/numeric-ingress/runs/` |
| Matrix fixtures | `darkmatter/lib/tests/l1/schema_enhancements_matrix/` (5 L1 tests) and `darkmatter/lib/tests/fixtures/schema_enhancements_matrix/` with `PROVENANCE.md` |

### Key findings

- **Examples: 121, not 117.** 74 executable, 47 display-only. The spec's 117
  is the catalog at commit `78540226f` (2026-09-17); `find_files` and
  `try_frontmatter` have since added one executable example each and
  `code_link` two display-only ones. The 72-executable floor is still met.
  Recommendation: carry all 121.
- **`trim` is not a registered function**, but the spec's null table and
  criterion 43 (`trim(null)` → `null`) use it. Criterion 43 cannot pass as
  written without adding `trim` or changing the criterion.
- **`keys`/`values` are not registered yet** (planned by this feature).
- **The draft replacement catalog** (`claudine/docs/schemas/partials/functions.yaml`)
  is missing `find_files`, `code_link`, `try_frontmatter` and lists
  `is_file_type`, which is out of scope and unregistered.
- **Converted-input fixture impossible for 34 functions**: 24 whose only
  parameter is `unknown | null` (7 type predicates, 14 date predicates,
  `file_exists`, `and`, `or`) and 10 taking `any[]` (`first`, `last`, eight
  `as_*`). `and`/`or` have no rejected fixture either. This is ruling R1;
  **criterion 10 stays blocked until the author rules.**
- **Ownership conflict:** `expr/functions/mod.rs` (6,041 lines) holds the
  dispatch, the binding types and 27 context handler bodies, so the Wave 5
  Pure and Context handler tasks would collide. Proposed: before Wave 5, the
  integrator moves the context bodies into their family files (move only).
  Unowned files: `schemas/resolve.rs` (→ Schema adapter) and
  `schemas/mod.rs` (→ integrator). The name `expr/binding.rs` is taken, so
  the Call binder needs another module name. Nine catalog consumers the plan
  does not name exist, including `claudine/gen/src/inputs.rs:292` and
  Claudine's grouping in `claudine/cli/src/commands/context/mod.rs:67`.
- **ConfigState drift (plan rule 8):** the plan's claim is only partly right.
  The `ConfigState` docs (`dmls/src/config/mod.rs:457`) already say the file
  layer is kept, matching `recompute`. The drift is in the test
  `test_semantic_tokens_malformed_value_keeps_previous` (`:1079–1081`), whose
  comment says the last good config survives. Left for the Wave 8 config task,
  which owns that file. Also: one invalid editor value discards *every*
  non-formatting overlay section (existing contract, kept).
- **`workspace/configuration` is never requested by DMLS** (documented). The
  spec's criterion 40 wording is read as the existing `didChangeConfiguration`
  channel (ruling R4).
- **Prerequisite comment drift (not fixed; owned by Phase 5):**
  `PredicateFact::FileExists` (`facts.rs:28`) says the handler rejects
  non-strings; it returns `false`.

### Numeric ingress: selected seam

A **number-spelling sidecar**: each loader keeps building today's
`serde_json::Value` and also records a JSON Pointer → written-number-text map
for numbers stored as `f64`, like the existing `MappingOrders`. One admission
function checks exactness from the spelling. `serde_json/arbitrary_precision`
was rejected (no effect on YAML/JSON5, workspace-wide feature unification
into 67 crates, and it breaks `Value(1.0) == Value(1.00)`). Darkmatter must
declare `serde_json` `raw_value` and `float_roundtrip` itself. Spike
recovered every spelling except YAML `99999999999999999999`, which fails the
whole frontmatter today. Four cell-level decisions for the author are in
`findings.md`.

### Existing defects found (not fixed in Phase 1; each is owned by a later task)

| Defect | Owner task |
|---|---|
| `standalone.rs:127-130` turns a YAML parse error into `Ok(None)`; DMLS then drops it (`named_types.rs:344`, overlay `:231`) | Grammar (Wave 2) |
| Duplicate-key and trailing-content errors in schema files reported as `AmbiguousReferenced` with no detail (`resolve.rs:759`, `:1152`) | Schema adapter (Wave 4) |
| `coerce.rs:597` rounds big integers silently; its comment is wrong | Numeric foundations (Wave 2) |
| `grammar.rs:279`, `:1948` parse type-expression numbers with bare `parse::<f64>()` | Grammar + Numeric (Wave 2) |
| YAML `.inf`/`.nan` and JSON5 `1e400`/`Infinity` become `null`; `1e-400` becomes `0`; YAML integer > u64 fails the whole frontmatter; enum members saturate through `i64`; `min(inf)` accepted | Numeric foundations (Wave 2) |
| `Markdown::from(String)` swallows frontmatter parse errors (`markdown/mod.rs:999-1003`) | not in this feature's scope; reported |
| `catalog.rs:425-438` silently skips a malformed schema-root catalog | not in this feature's scope; reported |
| `md set` stores malformed JSON as a string without warning | only if the numeric seam is routed through it |

### Matrix fixtures

Positive controls come from real producers (shipped `schemas/partials/timing.yaml`
and a pure-envelope shipped schema, `md schema detect --format json` output
and its YAML twin, `md get --yaml` output); keyword fixtures that `detect`
does not emit are hand-written and labeled so in `PROVENANCE.md`.

Tests (all `darkmatter` `l1`, prefix `schema_enhancements_matrix::`):

- `envelope::shipped_tagged_envelope_matrix`
- `envelope::shipped_pure_envelope_matrix`
- `raw_schema::raw_json_schema_keyword_matrix` (48 keywords, `.json` branch)
- `raw_schema::raw_yaml_schema_keyword_matrix` (48 keywords, YAML raw branch)
- `frontmatter::frontmatter_value_matrix`

Only cells current code already decides correctly are asserted. Where the
outcome is right but the error class is mislabeled (`AmbiguousReferenced`),
the test asserts only that loading fails; the fixing task tightens it.
Every other cell is assigned to its task or ruling in
`inventory/reader-matrix.md` → "Cell coverage". DMLS cells wait for the
`expressions` field (Wave 8). Observed contract: an absent optional property
composes as `null`, so absent and explicit null differ only in the source.

`darkmatter/lib/tests/l1/schema_yaml_spellings.rs` was edited only to make
`Edit` and `apply` `pub(crate)` for reuse.

Targeted verification:

- `cargo nextest run -p darkmatter --features effects-instrumentation --test l1 -E 'test(/schema_enhancements_matrix/) | test(/schema_yaml_spellings/) | test(/test_layout/)'` → 30 passed (slowest new test 0.63 s)
- `cd darkmatter && just lint` → clean

### Requirement-to-test mapping (Phase 1)

Phase 1 changes no behavior, so there is no regression test to add. The
behavior Phase 1 pins is the current decided matrix cells above. The plan's
criteria-to-test mapping for later phases is `inventory/criteria-map.md`.

### Gates

Scope calculator (run from the repository root after all edits):
`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
→ `packages: [darkmatter, test-toolkit, claudine]`, `change_class: package`.
The gates were therefore required, and all were run serially with no
subagent running:

| Gate | Result |
|---|---|
| `claudine`: `just test` | 10,691 passed, 9 skipped, 0 failed (332 s) |
| `claudine`: `just lint` | clean |
| `just check-tier-coverage claudine` | 0 stranded |
| `just check-tier-coverage darkmatter` | 0 stranded |
| `darkmatter`: `cargo nextest run -p darkmatter --features effects-instrumentation --test l1` (the binary this phase changed) | 2,073 passed, 0 failed |
| `darkmatter`: `just lint` | clean |

No failures, no load-sensitive reruns, no pre-existing failures observed.
Dependent packages: Phase 1 changed only test files and fixtures inside the
`darkmatter` `l1` binary, so no dependent package can observe the change.
Cross-OS check skipped: the new tests use in-memory fixtures and temp
directories, with no OS-specific path handling; CI covers the other OSes.
The `claudine` skill needed no update (no Claudine behavior changed).

## Author Decisions (2026-10-22)

Phase 1 paused for the decisions below. The author approved every
recommendation as written, so each one is decided:

- **Category names:** option A. A blank or repeated name in `categories:` is a
  load error at that entry; names compare exactly; list order is kept.
- **Numeric edge cells:** all four recommendations (a located per-field error
  for oversized YAML integers; no `1e3`/`.5` expression literals; JSON
  `1e400`'s load error counts as "not finite"; `json`-typed `1e400` stays
  malformed JSON).
- **`inventory/rulings.md` R1–R7:** each recommendation, as written there.
- **`trim`:** add it as a text transform, so criterion 43 holds as written.
- **Examples:** carry all 121.
- **`behavior-ledger.md`:** each row marked "ruling requested" takes its
  proposed behavior; `has_command` and its analogues take the criterion 44
  signature (`string | null`). `keys(null)` and `values(null)` return `[]` (signature
  `object | null`), so a missing map lists nothing.
- **Wrong kind of value to a yes/no check** (`has_key("hello", "a")`,
  `has_command(["git"])`): a type error, not `false`. `null` still gives
  `false`.
- **Behavior ledger (criterion 13):** the author reviewed and approved the
  whole ledger on 2026-10-22; the review is recorded at the top of
  `behavior-ledger.md`.

The items as they were raised:

- **Can a category name appear twice, or be empty? (needed before Phase 2's grammar work)**

  Phase 2 adds a `categories:` list to schema files, which groups functions for documentation. The spec does not say what happens when two entries have the same name, or when a name is blank. The grammar code written in Phase 2 has to pick one answer, and changing it later means rewriting that code and its tests.

  - **Option A — reject both (recommended).** A blank name, or a name that repeats an earlier one, makes the schema file fail to load with an error pointing at the entry. Names compare exactly ("Math" and "math" are different). List order is kept. *Pro:* matches how the old catalog already rejects blank categories and how the spec rejects duplicate parameter names; nothing is silently dropped. *Con:* a small authoring mistake stops the file loading.
  - **Option B — merge duplicates, first one wins.** *Pro:* forgiving. *Con:* no sensible rule when the two copies have different descriptions; hides mistakes.
  - **Option C — silently drop blank names.** *Con:* this is the kind of silent filtering the project's input rules forbid.

  I recommend **A**. Say if you would rather compare names case-insensitively, so "Math" and "math" count as the same name.
- **Four edge cases in how numbers are read (needed before Phase 2's number work)**

  Phase 1's investigation picked how numbers will be checked for exactness: each file reader keeps today's values and also remembers how each number was written, so `9007199254740993` (too large to store exactly) can be rejected instead of silently rounded. A workspace-wide alternative (`serde_json`'s `arbitrary_precision` switch) was rejected because it would change number behavior in 67 crates and still not help YAML. Details: `spikes/numeric-ingress/findings.md`. Four smaller cases need your answer:

  1. **Huge whole numbers in YAML, such as `99999999999999999999`.** Today they make the *whole* frontmatter fail to parse. *Recommended:* keep it an error, but report it on that one field with its location. *Alternative:* report it as "cannot be represented exactly", like the other big numbers. That is more uniform, but it needs a custom YAML read path.
  2. **Expression literals such as `1e3` or `.5`.** These are not valid in expressions today. *Recommended:* keep it that way; the spec does not add that syntax. *Alternative:* add it (new syntax, more scope).
  3. **JSON `1e400` (too large to store).** The JSON reader already fails with "number out of range". *Recommended:* accept that load error as the spec's "not finite" outcome. *Alternative:* re-word the error to say "not finite" (cosmetic, extra work).
  4. **Text declared as `json` that contains `1e400`.** It is rejected today as malformed JSON. *Recommended:* keep that.
- **Rulings needed later (not blocking Phase 2, but best decided now while you are reviewing)**

  Each has a recommendation in `inventory/rulings.md` or `inventory/function-registry.md`. Replying "approve the recommendations" settles them all.

  - **R1 — some functions cannot have a "converted input" test (blocks acceptance criterion 10, Phase 4).** 34 functions take any value unchanged (for example `is_number`, `file_exists`, `first`) or take no arguments, so a test showing the input being converted is impossible. *Recommended:* mark those cells "impossible", test instead that the input arrives unchanged, and cover conversion with test-only functions. *Alternatives:* weaken those functions' signatures so they convert (changes behavior; not recommended), or drop them from the criterion (forbidden by the plan).
  - **`trim` does not exist.** The spec's criterion 43 expects `trim(null)` to return `null`, but there is no `trim` function. Add `trim` (recommended, a small string function) or change the criterion.
  - **121 examples, not 117.** Three functions added four examples after the spec was written. Carry all 121 (recommended).
  - **R3 — duplicate keys in JSON.** Reject them in raw `.json` schema files and in `json`-typed text (recommended). Keep the editor-settings channel as it is, because an existing test depends on it.
  - **R4 — editor settings channel.** The spec says `workspace/configuration`, but the language server deliberately uses `didChangeConfiguration`. Read the spec as meaning the existing channel (recommended).
  - **R5 — some "should fail" cells in the plan contradict documented behavior.** For example `types.x: null` is documented as the `null` type. Keep the documented behavior for ordinary schema files, and apply the stricter rules only in the new function-catalog loader (recommended).
  - **R7 — two plan cells disagree with the JSON Schema standard (`prefixItems: []`, `enum: []`).** Follow the standard's validator (recommended).
  - **Ambiguous null behavior for about 20 functions** (for example `is_empty(null)`, `starts_with(null, …)`) and **`has_command(true)`'s draft signature**, which contradicts criterion 44. These are listed in `behavior-ledger.md`; the ledger review is a pre-merge requirement anyway.

## Phase 2

### Integrator step 1: `serde_json` features and `preserve_order` triage

`darkmatter/lib/Cargo.toml` now declares `serde_json` with `preserve_order`,
`raw_value`, and `float_roundtrip`. Before this change the darkmatter,
claudine, claudine-cli, darkmatter-cli and dmls build graphs did **not** have
`preserve_order` (only a whole-workspace build got it, through tauri-build and
tree-sitter); every package that depends on `darkmatter` now gets it.

Fallout measured with the full darkmatter (lib, CLI, DMLS) and claudine L1
suites, no fail-fast: 45 darkmatter and 3 claudine failures, every one a key
order change. Classification:

| Consumer | Contract | Resolution |
|---|---|---|
| `md schema validate --format json` | alphabetical keys, pinned byte-for-byte by `schema_validate_legacy_json_output_is_byte_identical` | explicit `sort_all_objects()` before printing |
| non-strict `md hash` frontmatter hash | persisted in documents; nested keys were sorted | nested values sorted before hashing; new unit test `non_strict_frontmatter_hash_ignores_nested_key_order`; benchmark fixture identities unchanged |
| `OPENCODE_CONFIG_CONTENT` / `KILO_CONFIG_CONTENT` merge | byte-identical whatever the insertion order | merged value sorted; test now asserts through `merge_overlay` |
| SimplifiedSchema → JSON Schema (`to_json_schema`) | none (diff-friendly output) | written order is intended; `$schema` now leads the document; 41 snapshots refreshed after a script confirmed each old/new pair is JSON-equal |
| `as_json5`, frontmatter `serde_json` maps, `WithOverlay::iter`, provider `serialized_field_list` | none; order was an artifact of `BTreeMap` | expectations now state written/declaration order; `WithOverlay::iter` doc comment was stale (said order was sorted) and was rewritten |

`dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
failed once and passed on rerun alone and in the next full run: load-sensitive,
not an ordering effect.

Note: the `Cargo.toml` edit was reverted once by something outside this
session while the first test run was in flight (other sessions share this
worktree); it was reapplied and re-verified.

### Integrator step 2: numeric admission API (frozen for Wave 2)

New module `darkmatter::markdown::numeric` (`darkmatter/lib/src/markdown/numeric.rs`):
`admit_number_spelling` (loader/lexer spelling), `admit_number_text` (the one
lenient text parser), `admit_f64`, `admit_json_number`, `admit_integer`,
`parse_boolean_word`, `format_number`, `json_number`, and the error types
`NumericAdmissionError` / `NumberTextError`. Whole-number exactness is
decided by comparing the written digits with the exact decimal expansion of the
nearest `f64`, so no big-integer dependency is needed.

### Wave 2 execution

The two Wave 2 tasks ran in parallel as foreground subagents, each in its own
git worktree seeded with a patch of the integrator's uncommitted base (steps 1
and 2), so neither could break the other's build. Each returned a patch
against that base; both applied to this worktree with no conflicts.

- **Grammar descriptors** (`/tmp/phase2-grammar.patch`): tuples, functions,
  `|` and grouping, `category`, `categories:`, inline examples, duplicate
  overload detection (`simplified/signature.rs`), function rejection in
  document schemas, numeric admission of schema metadata, the standalone
  YAML-error defect, serialization round trips, and docs.
- **Numeric foundations** (`/tmp/phase2-numeric.patch`): load-time admission
  for frontmatter YAML, raw JSON/YAML schemas, and expression literals
  (`numeric/ingress.rs`); `f64`-only operators with non-finite errors; one
  number formatter; JSON5 non-finite errors; the `coerce.rs` silent-rounding
  defect; `darkmatter/docs/topics/numbers.md`.

Integration fixes after the merge:

- The grammar task made a standalone schema file that is not valid YAML a
  `SchemaDocument` "not valid YAML" error. That path ran before the numeric
  task's oversized-integer rescan, so a raw YAML schema with
  `99999999999999999999` lost its admission reason
  (`raw_yaml_schema_numeric_token_matrix` failed). `standalone.rs` now loads
  through `numeric::load_admitted_yaml`, so every schema-file loader admits
  numbers the same way and reports the admission reason.
- The grammar task narrowed `LARGE_INT_CASES` in `schemas_literal_expression.rs`
  to exact values; the numeric task's coercion test iterated it expecting
  inexact values. The test now names its inexact inputs explicitly.

Concurrency hazards met (recorded so the next phase avoids them):

- An outside process committed this phase's integrator work in this shared
  worktree mid-phase (`58e2a5790 feat(darkmatter): preserve authored order and
  validate numeric ingress`, `b3e647ae8` for the log and plan). While it did,
  working files were briefly rolled back, and one claudine test build picked
  up an old test name. The rerun on the settled tree passed.
- zsh `CDPATH` resolves a relative `cd claudine` from a subdirectory to
  `/Volumes/coding/personal/rusty-biscuit/claudine`, a different checkout. One
  claudine `just lint` and both `check-tier-coverage` runs went there; all three
  were rerun from absolute paths in this worktree. Use absolute paths.
- `git apply --3way` stages what it applies; the merged files were unstaged
  again with `git reset -q -- <files>` (index only).

### Design Decisions

Integrator decisions:

1. **`preserve_order` everywhere, sort only where a published contract needs it.**
   - Decision: declare `serde_json/preserve_order` in `darkmatter/lib/Cargo.toml`
     and sort explicitly at outputs whose alphabetical order is a contract.
   - Alternatives: sort every serialized map back to alphabetical (hides the
     feature the spec asks for); refresh every changed snapshot (silently
     changes byte-stable outputs and persisted hashes).
   - Why: the spec rules `preserve_order` in (decision 75); persisted hashes and
     byte-pinned JSON reports must not drift.
   - Downsides: four sort sites to keep; a new alphabetical-output contract must
     sort explicitly.
2. **`sniff-cli` sorts its JSON keys.**
   - Decision: `sniff-cli` sorts at its three JSON writers
     (`output::sort_json_keys`), and its snapshot tests compare with
     `sort_maps`; a new test pins sorted keys on real output.
   - Alternatives: accept the new key order and refresh the five snapshots.
   - Why: `sniff-cli` links `darkmatter`, so without sorting its output would
     depend on which packages a build unifies.
   - Downsides: struct-declaration order is not used for `sniff` JSON.
3. **`to_json_schema` puts `$schema` first.**
   - Alternatives: leave it last (where insertion order puts it).
   - Why: readers expect the dialect first; the 41 snapshot changes were checked
     to be order-only by comparing each old/new pair as JSON.
   - Downsides: none found.
4. **Numeric admission at ingress (load time).**
   - Decision: frontmatter YAML, raw JSON/YAML schemas, schema-file YAML,
     SimplifiedSchema metadata, and expression literals admit every number when
     loaded, from its written spelling; a failure is a located load error
     whatever the property's declared type. The Phase 1 spelling sidecar is
     used only inside the loaders (the guided second pass) rather than stored
     beside the value.
   - Alternatives: store a JSON Pointer → spelling sidecar on `Frontmatter` and
     check at schema validation (Wave 4).
   - Why: the spec applies the check "wherever a value becomes a number",
     including native YAML numbers; a value that no `f64` holds cannot be a
     Darkmatter value under any type; checking at load closes the silent
     `.inf` → `null` and `1e-400` → `0` defects now, needs no new public
     state, and criteria 30/59's native rows ("fails validation", "rejected")
     are met as a located load error.
   - Downsides: an untyped or `string` property holding such a number must
     quote it; whole numbers in exponent form that no `f64` holds (`1e308`,
     `1e300`) are rejected, as the spec's whole-number rule requires.
5. **Criterion 46's `1e308 * 10`** is exercised with `big` supplied through
   frontmatter, written as the full digits of `f64::MAX` (an exactly held whole
   number), because exponent literals are not expression syntax (author
   decision, numeric cell 2) and `1e308` itself is an inexact whole number.

Grammar descriptors decisions (subagent):

1. A trailing `-> description` on a `|` union describes the last option, as the
   list form does. Alternatives: reject it; copy it to every option. Downside:
   per-option descriptions need the YAML list.
2. An option inside a YAML-list union cannot itself be a `|` union. Alternative:
   flatten it. Why: one spelling per union; flattening breaks per-item source
   projection.
3. Grouping: a group without `[]` joins the enclosing union; `(A)[]` is `A[]`;
   lists of lists and a group with its own constraint list are rejected. Why:
   one representation (`TypeExpr::Union` with `is_array`).
4. `?` on a union tuple element follows TypeScript: `(A | B)?` is required;
   `A | B?` and `A? | B` are rejected.
5. `tuple` and `function` are reserved built-in keywords; a local type with
   either name is reached as `tuple@this`. Downside: a bare reference to such a
   local type now fails.
6. A spread must be written as an array type (`...T` naming an array type is
   rejected) and takes no list constraints. Why: the simplest rule matching
   "must name an array type".
7. Tuple constraints are limited to `required`, `default`, `generated`,
   file `example`, and `category`, checked at parse time, so types files that
   are never converted are still checked.
8. Functions take no `required`/`default` but may take `[]`; a trailing `;`
   and trailing commas in element/parameter lists are accepted.
9. Tuple elements, parameter types, and return types take no `-> description`.
10. `|` ends a `Name@file` reference everywhere; inside a group, tuple, or
    signature, `)`, `]`, `;`, and `?` also end it. Downside: a referenced path
    cannot contain `|`.
11. Binding contracts for duplicate overloads compare canonical parameter types
    and constraints (order-insensitive), optional and rest positions; they
    ignore names, descriptions, examples, categories, return types, and
    `fallible`. Checked structurally at parse time and again with local named
    types resolved (depth 16) when a types file loads. Downside: references
    into other files compare by name.
12. `to_json_schema` is the one place that rejects a function in a document
    schema, naming the path; it works because the resolver inlines references
    first.
13. `tuple([])` lowers to `minItems: 0, items: false` with no `prefixItems`
    (the 2020-12 meta-schema rejects an empty `prefixItems`, ruling R7).
14. `categories:` is accepted only in the tagged envelope; the global catalog
    loader is unchanged. An entry splits on its first `->`; `Name ->` with no
    text is an error; at most one `category(...)` per type.
15. The strict schema-document source locator now follows `|` and `>` block
    scalars, since `function` signatures are written as `|-` blocks. This moves
    a boundary documented as "frozen v1"; the pinning tests and docs were
    updated.
16. A schema file that is not valid YAML is a `SchemaDocument` error instead of
    a silent decline or the mislabeled `AmbiguousReferenced`. DMLS
    `standalone_lints` still returns `None` for unparseable text, so ordinary
    YAML with syntax errors does not show as a broken schema.
17. DMLS surfaces the YAML error where it was dropped: the overlay's named-type
    catalog records it and hover on a reference into that file shows it;
    `diagnostics/named_types.rs` attaches it as related information.
18. Schema numbers: `inf`/`nan` report "not finite"; quoted defaults stay text;
    word enum members are admitted only when spelled like numbers;
    `suggest(...)` candidates are unchanged.

Numeric foundations decisions (subagent):

1. Equality stays exact (integer-to-integer `==` does not round). Alternative:
   compare everything as `f64` (broke two pinned `expression_strict_equality`
   tests). Why: every admitted number is held exactly, so exact `==` agrees with
   ordering for every value a document, schema, or literal supplies. Downside:
   an unadmitted external integer (for example `--set` with
   `9007199254740993`) is unequal to `…992` while neither `<` nor `>` holds;
   arithmetic rejects it.
2. Oversized YAML integers: a failed first parse triggers a rescan that reports
   an inadmissible integer beyond `u64` as a located admission error. Downside:
   an exactly held integer beyond `u64` (2^64) keeps the old parse error
   (pinned by a unit test).
3. `coerce_to_number` keeps its spelling rule and leaves inexact text
   unconverted; `validate.rs` appends the admission reason to the type error.
   Alternative: an error channel in `coerce.rs` (a public struct Wave 4
   replaces). Behavior change: `"9223372036854775807"` and
   `"9007199254740993.0"` no longer convert.
4. The JSON5 check stays in Darkmatter (`Json5::deserialize_raw` plus a
   non-finite scan); `biscuit-file` is unchanged. `--state`/`--set` reject a
   non-finite number; transclusion `set.n=Infinity` is an error (quoted stays
   text); a `name=Infinity` shorthand setter stays text, matching its existing
   fallback.
5. Numeric text in arithmetic goes through `admit_number_text`; inadmissible
   numeric text is an error rather than a concatenation. Spellings whose result
   changed: `" 4 "` and `"1_000"` now convert; `"inf"`, `"NaN"`, `"5."`,
   `"1e400"`, `"1e-400"`, and inexact whole text no longer convert.
6. Raw schema admission failures reuse `SchemaError::SchemaDocument` (a new
   variant would ripple through exhaustive matches); JSON `1e400` keeps its
   existing load error (author cell 3).
7. `format_number` writes every digit of a large whole number (Rust's shortest
   form printed `2^60` as `1152921504606847000`, which reads back as a
   different number).

### Requirement-to-test mapping

| Requirement | Tests (darkmatter `l1` unless noted) |
|---|---|
| Criterion 1 (grammar forms and rejections) | `schema_grammar_descriptors::{tuple_forms_parse, function_type_with_every_constraint_parses, union_operator_parses_everywhere_a_type_is_written, categories_list_parses_in_order_with_optional_multi_line_descriptions, invalid_tuples_and_repeated_categories_are_rejected_at_the_offending_text, invalid_function_types_are_rejected_at_the_offending_text, a_union_inside_a_yaml_list_arm_and_a_question_mark_on_a_property_are_rejected}` |
| Criterion 2 (precedence) | `schema_grammar_descriptors::array_suffix_binds_tighter_than_the_union_operator` |
| Criterion 3 (JSON Schema lowering) | snapshots `schema_grammar_descriptors::{snapshot_fixed_tuple, snapshot_variadic_tuple_with_optional_position, snapshot_union_operator, snapshot_empty_tuple_and_category_annotation}`; `union_operator_and_yaml_list_are_the_same_definition` |
| Criterion 4 / 56 (functions in document schemas, indirect forms) | `schema_grammar_descriptors::{a_document_schema_that_reaches_a_function_fails_to_load_naming_the_path, tuples_validate_positions_omission_and_length}`; CLI `schema_grammar_types::schema_validate_rejects_a_function_typed_property_as_a_schema_error` |
| Criterion 5 (categories) | `schema_grammar_descriptors::{a_declared_categories_list_limits_category_names, a_category_on_a_data_type_is_an_annotation}`; `schema_enhancements_matrix::envelope::documented_categories_matrix` |
| Criterion 6 (duplicate overloads) | `schema_grammar_descriptors::overloads_with_one_binding_contract_fail_to_load`; unit `simplified::signature::tests` |
| Criterion 47 (`(A \| B)[]`) | `schema_grammar_descriptors::grouped_union_list_matches_a_named_union_list` (compiled JSON Schema rejects `[true]`; document-level rejection waits for Phase 3 coercion, which today turns `true` into `"true"`) |
| Round trip of new forms | `schema_grammar_descriptors::every_new_form_round_trips_through_serialization` |
| End to end through `md compose` | CLI `schema_grammar_types::{compose_accepts_a_document_matching_a_tuple_and_a_union, compose_rejects_a_document_whose_tuple_has_too_many_positions}` |
| Standalone YAML-error defect | `schema_grammar_descriptors::a_types_file_that_is_not_yaml_reports_its_yaml_error`; DMLS `named_type_references::a_reference_into_a_file_that_is_not_yaml_reports_the_yaml_error` (fails before the fix) |
| Schema metadata numbers | `schema_grammar_descriptors::numeric_metadata_rejects_what_an_f64_cannot_hold`; `schemas_literal_expression::literal_inexact_large_integer_is_a_schema_load_error` |
| Criterion 21 (literal part) | `numeric_admission::{an_inexact_expression_literal_is_rejected_at_the_literal, an_exact_expression_literal_evaluates_exactly}`; `expression_type_scope::literal_numbers_narrow_to_the_value_the_evaluator_compares` |
| Criteria 30/59 (native YAML rows, spellings) | `numeric_admission::{native_frontmatter_numbers_no_f64_holds_fail_to_load_with_location, admissible_frontmatter_numbers_load_exactly}`; lib unit `frontmatter::tests::{inadmissible_frontmatter_numbers_are_located_load_errors, inadmissible_numbers_are_not_hidden_by_expression_fallbacks, admissible_frontmatter_numbers_load_exactly}`; `numeric::tests::*`; `numeric::ingress::tests::yaml_*` |
| Quoted inexact text vs `number` (coerce defect) | `numeric_admission::quoted_inexact_text_against_a_number_fails_validation_with_the_reason`; lib unit `coerce::tests::large_integral_strings_are_admitted_before_coercion`; `schemas_literal_expression::literal_large_integer_coercion_admits_before_converting` |
| Criterion 46 (operators, presentation) | `numeric_admission::{non_finite_results_are_errors, equality_and_ordering_agree_on_represented_values, whole_numbers_never_pass_through_an_integer_cast, the_largest_finite_number_loads_and_renders_every_digit, numeric_text_converts_through_the_canonical_parser, an_out_of_range_index_is_none_rather_than_saturated}`; lib unit `ast::tests::display_number_literal_beyond_i64_never_saturates`, `numeric::tests::presentation_never_narrows` |
| Raw schema numeric tokens | `numeric_admission::{raw_schema_numbers_are_admitted_at_load, ordinary_raw_schema_bounds_still_load_and_validate}`; `schema_enhancements_matrix::raw_schema::{raw_json_schema_numeric_token_matrix, raw_yaml_schema_numeric_token_matrix}` |
| Frontmatter numeric matrix cells | `schema_enhancements_matrix::frontmatter::frontmatter_value_matrix` (18 numeric cells) |
| JSON5 non-finite | lib unit `numeric::ingress::tests::json5_non_finite_numbers_are_rejected`, `transclusion::parser::tests::set_property_non_finite_number_is_an_error_not_null`; CLI `compose_state_set::test_compose_non_finite_json5_number_is_an_error_not_null`, CLI unit `commands::compose::tests::parse_shorthand_value_keeps_a_non_finite_number_as_text_not_null` |
| Criterion 58 (map order) | lib unit `markdown::hash::tests::non_strict_frontmatter_hash_ignores_nested_key_order`, `frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`, `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`; `benchmark_fixtures::benchmark_manifest_matches_recorded_identities`; CLI `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`; claudine `opencode_config::tests::yolo_permission_block_serializes_independent_of_insertion_order`, `action_shape_control::proxy_with_iteration_follows_authored_order_and_is_complete`; sniff-cli `snapshots::json_output_keys_are_sorted_at_every_depth` |

Existing tests whose expectations changed (each pinned behavior this phase
changes on purpose): the 41 conversion snapshots (order only);
`as_json5_uses_the_formatter_object_idiom`,
`parsed_frontmatter_retains_nested_mapping_order_as_metadata`,
`serialized_field_list_matches_catalog`,
`proxy_with_iteration_is_canonical_and_complete` (renamed), the OpenCode
insertion-order test (order); `coerce::tests::large_integral_strings_*`,
`schemas_literal_expression::literal_large_integer_*`,
`expression_type_scope::literal_numbers_narrow_to_the_value_the_evaluator_compares`
(numeric admission); `schema_yaml_spellings` (library and DMLS),
`meta_schema_phase6`, `schemas_union_postfix`, `shipped_schema_corpus`, and
`convert.rs` unit tests (grammar: block-scalar locator, YAML errors, new
keywords); sniff-cli JSON snapshots (now compared with `sort_maps`).

### Draft catalog under the new grammar

`claudine/docs/schemas/partials/functions.yaml` is still classed Draft. Its
only grammar failures are two uses of `min-length(...)`, plus its `behavior:`
envelope key. With those fixed, all 111 signatures parse and the file passes
the categories and overload checks. `mutations.yaml` signatures parse; it is
rejected for its `name:` key. Neither draft was edited (Phase 4 migrates them).

### Gates

Scope calculator, run from the repository root after all edits:
`packages: [claudine, claudine-gen, darkmatter, darkmatter-cli, dmls, sniff-cli, test-toolkit]`,
`change_class: package`, so every gate was required. All ran serially with no
subagent running:

| Gate | Result |
|---|---|
| `darkmatter`: `just test` (`--no-fail-fast` rerun) | 11,342 passed, 10 skipped |
| `darkmatter`: `just lint` | clean |
| `claudine`: `just test` | 10,691 passed, 9 skipped |
| `claudine`: `just lint` | clean |
| `just check-tier-coverage claudine` / `darkmatter` | 0 stranded / 0 stranded |
| Dependents, `cargo nextest run -p <pkg>` one at a time | messenger 226, messenger-cli 133, research 606, sniff-cli 976, playa-cli 25, biscuit-icon-cli 98, biscuit-speaks-cli 108, claudine-gen + claudine-contract 298: all passed |
| `sniff`: `just lint` | clean |

Load-sensitive: `dmls::l1 searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`
failed under full-suite load in three runs (including one before any Phase 2
code, with only `preserve_order`) and passed alone every time. `main` already
relaxes this assertion (`43798e829 test(dmls): accept an intermediate watch
registration under load`), which `feat/pipeline` does not have yet.

Cross-OS: no OS-specific path, process, or console code changed (numeric
parsing, YAML/JSON loaders, grammar, key ordering), so no `just cross-check`
run was made; CI covers the other environments.

The `claudine` skill needed no change: Claudine's only visible effects are the
authored-order `with:` iteration (documented in Darkmatter) and the sorted
OpenCode/Kilo inline config (unchanged output). The `darkmatter` skill
(`expressions.md`, `frontmatter.md`, `schema.md`) was updated.
