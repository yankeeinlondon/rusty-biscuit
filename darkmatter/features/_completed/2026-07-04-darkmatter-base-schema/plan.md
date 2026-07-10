---
agent: open_code/zai-coding-plan/glm-5.2
phases: 8
created: 2026-07-04
start_phase: 1
yolo: true
spec: darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/docs/schemas/darkmatter.yaml
docs_updated_during_phase_3:
  - darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/lib.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/cli/tests/layout_style_frontmatter.rs
  - darkmatter/lib/tests/horizontal_rule_integration.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6:
  - darkmatter/docs/schemas/darkmatter-schema.md
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/tests/compose_base_schema.rs
  - darkmatter/cli/tests/compose_refs_and_missing.rs
  - darkmatter/cli/tests/level2_schema_about.rs
  - darkmatter/cli/tests/level2_errors.rs
docs_updated_during_phase_7:
  - darkmatter/features/2026-07-04-darkmatter-base-schema/plan.md
  - darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_8:
  - darkmatter/cli/tests/level2_errors.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/tests/base_schema_end_to_end.rs
docs_updated_during_phase_8:
  - darkmatter/features/2026-07-04-darkmatter-base-schema/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
source_code:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/lib.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/cli/src/args/cli.rs
  - darkmatter/cli/src/args/command.rs
  - darkmatter/cli/src/commands/compose.rs
  - darkmatter/cli/src/commands/mod.rs
  - darkmatter/cli/tests/compose_base_schema.rs
  - darkmatter/cli/tests/compose_refs_and_missing.rs
  - darkmatter/cli/tests/layout_style_frontmatter.rs
  - darkmatter/cli/tests/level2_errors.rs
  - darkmatter/cli/tests/level2_schema_about.rs
  - darkmatter/lib/tests/horizontal_rule_integration.rs
  - darkmatter/lib/tests/base_schema_end_to_end.rs
documentation:
  - darkmatter/features/2026-07-04-darkmatter-base-schema/plan.md
  - darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md
  - darkmatter/docs/schemas/darkmatter-schema.md
packages:
  - darkmatter
---

# Execution Plan: Darkmatter Base Frontmatter Schema

Converts `darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md` into a
phased, dependency-ordered implementation plan.

## Context Snapshot (verified against source)

The draft baseline schema at `darkmatter/docs/schemas/darkmatter.yaml` **already
uses** the two grammar features this feature must deliver — nested YAML mapping
object syntax and the `generated` constraint — but the `SimplifiedSchema`
parser, converter, and descriptor catalog do not yet support either. Concrete
gaps confirmed by reading source:

- `Constraint` enum (`simplified/types.rs:211`) has **no** `Generated` variant.
- String grammar (`simplified/grammar.rs:1027`) does **not** recognize the
  `generated` keyword.
- YAML-shape layer (`simplified/mod.rs:150`) **rejects** mapping property
  values, and (`simplified/mod.rs:120`) **rejects** mapping arms in
  property-level unions.
- Descriptor catalog (`about.rs:120`) has a parity test
  (`constraint_set_matches_descriptor_set`) that **will break** when
  `Generated` is added unless the descriptor set is updated in lockstep.
- Serialize module (`simplified/serialize.rs`) needs `Generated` round-trip.
- Deprecated root `hr` lives in two places: `style/parse.rs:314`
  (`merge_deprecated_top_level_hr`) and `render_tree/entrypoints.rs:132`
  (`hr_defaults_from_frontmatter`).
- `ComposeOptions::with_baseline_schema` exists
  (`compose/context/options.rs:931`) but there is no `darkmatter_base_schema()`
  library accessor.
- `darkmatter/docs/schemas/darkmatter-schema.md` does **not** exist yet.

## Dependency Graph

```
Phase 1 (grammar) ──► Phase 2 (convert) ──► Phase 3 (schema finalize) ──► Phase 4 (lib API) ──► Phase 7 (CLI)
                                  │                     │
                                  └────────────────────►├──► Phase 6 (docs)
                                                        │
Phase 5 (hr removal) ── parallel with 1-4 ─────────────►│
                                                        └──► Phase 8 (e2e validation)
```

## Commands

- Lib unit tests: `just test` (inside `darkmatter/`)
- Integration tests: `just test-l2`
- Lint: `just lint`
- Doctests: `just doctest`
- Single package from repo root: `just test darkmatter`

---

## Phase 1: SimplifiedSchema Grammar Extensions

**Goal:** Make the parser accept the nested mapping object syntax and the
`generated` constraint so the baseline schema YAML becomes parseable. This is
the foundational phase — Phases 2, 3, 4, 6, and 7 all depend on it.

**Files (primary):** `darkmatter/lib/src/markdown/schemas/simplified/{types.rs,
grammar.rs, mod.rs, serialize.rs}`, `darkmatter/lib/src/markdown/schemas/about.rs`

### Tasks

- [x] Add `Constraint::Generated` variant to the `Constraint` enum in
  `simplified/types.rs`, placed in the "universal" section alongside
  `Required` and `Default`. Add its `keyword()` arm returning `"generated"`.
- [x] Teach the string grammar parser (`simplified/grammar.rs`,
  `parse_one_constraint`) to recognize `generated` as a bare no-argument
  constraint keyword (mirrors `required`/`eager`/`integer`). Reject
  `generated(...)` with an arguments error.
- [x] Extend the YAML-shape layer (`simplified/mod.rs`, `parse_property_def`)
  to accept a `YamlValue::Mapping` at a property position as a nested object
  shape: lower it to `PropertyDef::Single(PropertyAtom::bare_inline_object(shape))`
  by recursively parsing each mapping entry's value through
  `parse_property_def` (so nested mappings, sequences, and string type
  expressions all work). Remove the current "reserved for future" rejection at
  line 150.
- [x] Extend the YAML-shape layer to accept `YamlValue::Mapping` arms inside a
  property-level `Sequence` (union). Each mapping arm lowers to an inline
  object `PropertyAtom`; string arms continue to lower through the grammar.
  Remove the rejection at `simplified/mod.rs:120`.
- [x] Enforce the parser rules from spec §"Nested Object Syntax Requirement":
  (1) a mapping value means "object with these properties," (2) leaf values use
  the existing type-expression grammar including `-> description` and
  `default(...)`, (3) nesting is recursive, (4) sequence values mean property
  unions and a sequence arm may be a type-expression string or a nested
  mapping object shape, (5) a nested mapping without an explicit `type:` key
  is an object shape (not a future long-form descriptor).
- [x] Update `serialize_property_atom` (`simplified/serialize.rs`) to emit
  `generated` in the constraint list so proptest round-trips stay honest.
- [x] Add a `SchemaConstraintDescriptor` entry for `generated` to
  `SCHEMA_CONSTRAINT_DESCRIPTORS` in `about.rs` (`form: "generated"`,
  `target_types: "all types"`, `argument_arity: "0"`,
  `json_schema_effect:` describing the `x-darkmatter-generated` extension and
  static-`required` suppression). This must land in the same change as the
  enum variant or the `constraint_set_matches_descriptor_set` parity test
  fails.
- [x] Update `SCHEMA_SHAPE_DESCRIPTORS` (`about.rs`) with a descriptor for the
  nested mapping object shape so `md schema about` advertises it.

### Validation Checkpoint

- [x] `just test` passes in `darkmatter/` with new unit tests covering: (a)
  `string(generated; required)` parses to two constraints; (b) a nested
  mapping lowers to the same `SchemaShape` as the equivalent quoted inline
  object literal `{ foo: string, bar: number }`; (c) a sequence union with a
  mapping arm lowers correctly; (d) `serialize_property_atom` round-trips
  `generated`; (e) `constraint_set_matches_descriptor_set` passes.
- [x] `just lint` passes.

---

## Phase 2: `generated` Constraint Convert & Validation Semantics

**Goal:** Make the JSON Schema converter and the validation/required logic
honor `generated` semantics: static authored documents are not faulted for a
missing `generated` property, but the `required` type/nullability semantics
are preserved for runtime/effective schemas.

**Depends on:** Phase 1 (needs `Constraint::Generated` in the AST).

**Files (primary):** `darkmatter/lib/src/markdown/schemas/simplified/convert.rs`,
`darkmatter/lib/src/markdown/schemas/{resolve.rs, validate.rs, rewrite.rs}`

### Tasks

- [x] In `convert.rs`, when an atom carries `Constraint::Generated`, emit an
  `x-darkmatter-generated: true` annotation on the property's schema object so
  downstream tooling (LSP, completion, runtime validators) can discover the
  ownership/supply semantics.
- [x] Define and implement the static-`required` suppression rule: a property
  whose atom carries `Generated` is **not** added to the parent object's
  `required` array during conversion, even when `Required` is also present.
  This makes an absent `generated` property pass static authored-document
  validation (spec semantics point 1).
- [x] Preserve `required` type/nullability: a `string(generated; required)`
  atom still lowers to a non-nullable `string` type (no `null` arm added), so
  a host-supplied runtime value is type-checked. Verify the existing
  optional-`null`-arm logic keys off `Required` presence, not `required`-array
  membership.
- [x] Audit `resolve.rs` baseline-merge (`merge_baseline_into_document` ~line
  339) so a baseline `generated` property does not get force-added to the
  document-level `required` list during merge. The merge copies baseline
  `required` entries (line 342); confirm `generated` baseline properties are
  excluded there too, or that convert already suppressed them.
- [x] Verify the compose-time schema validation stage
  (`compose/schema_validation.rs`) and `rewrite.rs` do not regress when a
  `generated` property is absent from authored frontmatter.

### Validation Checkpoint

- [x] Snapshot/unit tests in `convert.rs` prove: (1) a `generated` property is
  absent from the emitted `required` array; (2) the same property carries
  `x-darkmatter-generated: true`; (3) the type remains non-nullable when
  `required` is also present; (4) `generated` + non-required still emits a
  nullable type as before.
- [x] A focused integration test shows an authored document omitting a
  baseline `ctx.*` `generated; required` property validates cleanly, while the
  same document with a wrongly-typed `ctx.today` value fails.
- [x] `just test` passes.

---

## Phase 3: Baseline Schema Scope Review & Finalization

**Goal:** Confirm the baseline property list is complete and correct against
every Darkmatter surface, resolve the four spec open questions, and freeze
`darkmatter/docs/schemas/darkmatter.yaml`.

**Depends on:** Phase 1 + Phase 2 (the file must be parseable and convertible
before it can be validated end-to-end). The **investigative audit** portion is
parallelizable and can start during Phase 1.

**Files (primary):** `darkmatter/docs/schemas/darkmatter.yaml`

### Tasks

- [x] **Audit (parallelizable during Phase 1):** Compare the baseline property
  list (`$schema`, `title`, `description`, `tags`, `draft`, `metadata`,
  `last_updated`, `hash`, `style`, `change`, `replace`, `ctx`, `prologue`,
  `epilogue`, `ignore_invalid`, `interpolate_code_blocks`) against each
  surface that reads frontmatter: compose pipeline, render tree, `style::parse`,
  hash (`hash/`), delta/change (`delta/`), and docs. Record any property
  Darkmatter defines/interprets/mutates that is missing from the list, or any
  listed property that is not actually a Darkmatter-owned frontmatter
  contract.
- [x] **Resolve Open Question 2** (`style` shape): decide whether `style`
  remains `object` or whether stable top-level style buckets get inline-object
  modeling. The draft already models `style` with nested mappings; confirm
  this matches the `ACTIVE_STYLE_WIRING_SUB_SPEC = 9` surface in
  `style/descriptor.rs` and adjust if drift is found.
- [x] **Resolve Open Question 3** (`change` validation shape): confirm `change`
  stays `any` in v1 (per Non-Goal 2) or narrow it if the delta surface
  documents a stable contract. Default: keep `any`.
- [x] **Resolve Open Question 4** (generated property tables in docs): decide
  whether the docs page emits generated property tables in addition to the
  transcluded YAML. Default for v1: transclusion only (keep docs and schema
  non-drifting with one source).
- [x] Freeze `darkmatter/docs/schemas/darkmatter.yaml`: remove the
  "intentionally uses the proposed nested object syntax" comment now that the
  syntax is shipped, and confirm every property carries a `-> description`
  where it is public/user-facing.
- [x] Confirm the deprecated root-level `hr` is **not** present in the schema
  (Non-Goal 6) and that `style.hr.*` is the only horizontal-rule surface.

### Validation Checkpoint

- [x] `darkmatter/docs/schemas/darkmatter.yaml` parses cleanly via
  `parse_yaml_schema` (Phase 1 grammar) — verified by a test that reads the
  file with `include_str!` and parses it.
- [x] The parsed schema converts to a baseline-compatible Draft 2020-12 JSON
  Schema via `to_json_schema` without errors.
- [x] Audit findings are recorded (a short note in the feature directory or a
  resolved open-questions section in the spec) and any missing properties are
  added.

---

## Phase 4: Library API Integration

**Goal:** Expose the baseline schema as a first-class library surface for
compose callers, validation callers, and downstream packages (Claudine).

**Depends on:** Phase 3 (frozen schema file).

**Files (primary):** `darkmatter/lib/src/markdown/schemas/mod.rs`,
`darkmatter/lib/src/lib.rs`, `darkmatter/lib/src/markdown/compose/context/options.rs`

### Tasks

- [x] Add a public `darkmatter_base_schema() -> SimplifiedSchema` function
  (namespace it under `markdown::schemas` or a dedicated module per the
  candidate API in spec §"Library Integration"). Load the authored YAML via
  `include_str!("../../../docs/schemas/darkmatter.yaml")` and parse it through
  `parse_yaml_schema` at call time (or lazily cache via `std::sync::OnceLock`).
- [x] Add a companion accessor that returns the compiled Draft 2020-12 JSON
  Schema value (e.g. `darkmatter_base_json_schema() -> serde_json::Value` or a
  precomputed constant), so validation callers do not each re-pay the convert
  cost.
- [x] Add a `ComposeOptions` convenience method (e.g.
  `with_darkmatter_baseline_schema()`) that wires `darkmatter_base_schema()`
  into the existing `with_baseline_schema(...)` slot, so library callers can
  opt in with one call.
- [x] If a checked-in generated Rust artifact is preferred over `include_str!`
  + runtime parse, add a small deterministic generator invoked from a `just`
  recipe (not `build.rs`) and check in the output. Default: prefer
  `include_str!` to avoid a generator in v1 unless compile-time cost
  justifies it.
- [x] Re-export the new accessors from `darkmatter/lib/src/lib.rs` so they are
  reachable at `darkmatter::darkmatter_base_schema` (or the chosen namespace).

### Validation Checkpoint

- [x] Unit test: `darkmatter_base_schema()` returns a non-empty
  `SimplifiedSchema` whose `$schema`, `title`, `ctx` properties are present.
- [x] Unit test: the compiled JSON Schema validates a known-good frontmatter
  sample and rejects a known-bad `title` (wrong type).
- [x] Unit test: `ComposeOptions::new().with_darkmatter_baseline_schema()`
  injects the baseline and still allows unknown user keys (Non-Goal 1) and
  preserves document `$schema` precedence (Non-Goal 5).
- [x] `just test` + `just doctest` pass; the new public symbols have rustdoc
  following the repo convention (no `# H1`, `## H2` sections).

---

## Phase 5: Deprecated Root `hr` Removal

**Goal:** Delete the two remaining runtime compatibility paths that read
root-level `hr` so `style.hr.*` is the only horizontal-rule frontmatter
surface.

**Parallelizable:** This phase is **independent of the schema grammar work**
and can be developed concurrently with Phases 1–4. It only touches
`style/parse.rs`, `render_tree/entrypoints.rs`, and their tests.

**Files (primary):** `darkmatter/lib/src/style/parse.rs`,
`darkmatter/lib/src/markdown/render_tree/entrypoints.rs`

### Tasks

- [x] Remove `merge_deprecated_top_level_hr` and its caller in
  `style/parse.rs::from_frontmatter` (lines ~314–316 and the whole
  `merge_deprecated_top_level_hr` function body ~332–419). Root-level `hr`
  is no longer merged into `style.hr`.
- [x] Remove the root-`hr` read in `render_tree/entrypoints.rs`:
  `hr_defaults_from_frontmatter` (line 132) currently reads
  `md.frontmatter().as_map().get("hr")`. Either delete the function (if
  `style.hr` is the sole source after the `style::parse` change) or redirect
  it to read from the parsed `style.hr` only. Audit the caller at line ~571
  to confirm the HTML `:root` CSS-variable path still resolves from
  `style.hr`.
- [x] Update or delete the tests in `style/parse.rs` that exercise
  `merge_deprecated_top_level_hr` (search for `top-level`, `hr.style`,
  `hr.color` test fixtures ~lines 722–728, 888–910). These tests assert
  deprecated-alias behavior that is now removed.
- [x] Update render-tree tests in `entrypoints.rs` that feed root `hr` to
  `hr_defaults_from_frontmatter`.
- [x] Update any docs/skill text that still advertises root `hr` as a
  deprecated alias: `darkmatter/.claude/skills/darkmatter/SKILL.md` (the
  "Horizontal rules" note) and any topic docs. Point authors to `style.hr`.
- [x] If a migration diagnostic is desired (optional), add it as a one-time
  warning outside the base schema contract, clearly naming `style.hr` as the
  replacement. Default: skip in v1 since the alias was already `Deprecated`.

### Validation Checkpoint

- [x] A document with root-level `hr: { color: red-500 }` no longer affects
  rendering; only `style: { hr: { color: red-500 } }` does.
- [x] `just test` + `just test-l2` pass in `darkmatter/` with no remaining
  references to root `hr` in production code (only historical/feature docs).
- [x] `rg "\"hr\"" darkmatter/lib/src/style/parse.rs darkmatter/lib/src/markdown/render_tree/entrypoints.rs`
  returns no production-path matches (test-only matches OK if scoped to
  asserting the removal).

---

## Phase 6: Documentation Contract

**Goal:** Add the documentation file next to the schema using a transclusion so
docs and validation source cannot drift.

**Depends on:** Phase 3 (frozen schema file). The transclusion target must be
stable.

**Files (primary):** `darkmatter/docs/schemas/darkmatter-schema.md`

### Tasks

- [x] Create `darkmatter/docs/schemas/darkmatter-schema.md` with the intro
  prose from spec §"Documentation Contract" and a `::code ./darkmatter.yaml`
  transclusion directive pointing at the same schema file the library
  `include_str!`s.
- [x] Add prose documenting: (1) the base schema is added by default by the
  darkmatter library; (2) document `$schema` declarations override baseline
  properties on conflict (Non-Goal 5); (3) unknown frontmatter keys remain
  allowed (Non-Goal 1); (4) `generated` properties are host-supplied and not
  authored in static frontmatter.
- [x] If Open Question 4 resolved in favor of generated property tables, add
  them here as generated content (otherwise omit — transclusion only).
- [x] Verify the transclusion resolves: compose the doc file (or run the docs
  build) and confirm the schema YAML appears inline.

### Validation Checkpoint

- [x] The transclusion path `./darkmatter.yaml` resolves to the exact file
  used by `darkmatter_base_schema()` (spec testing requirement 7).
- [x] `just lint` passes; the new doc follows repo Markdown conventions.

---

## Phase 7: CLI Integration

**Goal:** Decide whether `md compose` auto-injects the base schema and implement
the decision without breaking existing contracts.

**Depends on:** Phase 4 (library API). Also depends on **resolving Open Question
1**.

**Files (primary):** `darkmatter/cli/src/commands/compose.rs`,
`darkmatter/cli/src/commands/validate.rs`

### Tasks

- [x] **Resolve Open Question 1:** Decide whether `md compose` injects the base
  schema by default. Recommended v1 stance: inject for `md compose` to match
  library defaults, but preserve a flag/env escape hatch
  (`--no-baseline-schema` or `DARKMATTER_NO_BASELINE_SCHEMA=1`) for users who
  want raw behavior. Document the decision in the spec's Open Questions
  section once made.
- [x] If auto-injecting: wire `darkmatter_base_schema()` into the CLI compose
  path's `ComposeOptions` baseline slot by default, behind the escape hatch.
  Ensure document-level `$schema` still wins on conflict (Non-Goal 5) and
  unknown keys remain allowed (Non-Goal 1).
- [x] Do **not** change the existing `md schema validate` baseline contract
  (explicit `--baseline-schema` file and `BASELINE_SCHEMA` env var stay as-is
  per spec §"CLI Integration"). Optionally, when no explicit baseline is
  supplied, default to the darkmatter base schema with a notice.
- [x] Add a `--baseline-schema`/`--no-baseline-schema` toggle to `md compose`
  if auto-injection is the default.

### Validation Checkpoint

- [x] CLI integration test: `md compose` on a doc using `ctx.today` (a
  `generated` property) succeeds without the author declaring `ctx`.
- [x] CLI integration test: a doc with `$schema: { title: number }` still
  fails validation (document `$schema` precedence preserved).
- [x] CLI integration test: a doc with an unknown `custom_key: 42` composes
  without error (unknown keys allowed).
- [x] `just test` + `just test-l2` pass.

---

## Phase 8: End-to-End Validation & Closure

**Goal:** Satisfy all 10 spec testing requirements and confirm cross-platform
correctness.

**Depends on:** All prior phases.

### Tasks

- [x] Spec test 1: `darkmatter/docs/schemas/darkmatter.yaml` parses as
  referenced `SimplifiedSchema`.
- [x] Spec test 2: the parsed schema converts to a baseline-compatible JSON
  Schema.
- [x] Spec test 3: known valid frontmatter examples pass validation.
- [x] Spec test 4: invalid known-property values fail validation (e.g.
  `title: 42`, `draft: "maybe"`).
- [x] Spec test 5: unknown user-defined frontmatter keys remain accepted.
- [x] Spec test 6: document `$schema` definitions override baseline properties
  on conflict.
- [x] Spec test 7: the documentation transclusion path points at the same
  schema file used by source integration.
- [x] Spec test 8: nested mapping object syntax lowers to the same JSON Schema
  as the existing quoted object-literal syntax (equivalence test).
- [x] Spec test 9: sequence union arms accept nested mapping object shapes.
- [x] Spec test 10: `generated` properties are omitted from static-document
  required checks for authored frontmatter, but retain `required`
  type/nullability semantics in runtime/effective schemas and
  LSP/completion metadata.
- [x] Cross-platform build check: confirm `cargo build` (or `just build`)
  succeeds. Note any platform-specific concerns — the `include_str!` path and
  YAML parse are OS-agnostic; flag if a generator is added in Phase 4 that
  needs to run identically on macOS/Windows/Linux.
- [x] `just lint` clean across `darkmatter/` (lib + cli).
- [x] `just doctest` clean — new public symbols have runnable doc examples.
- [x] Update `.claude/skills/darkmatter/SKILL.md` "Schema Validation" section
  to mention the base schema is now a first-class library surface (drift
  maintenance per AGENTS.md).
- [x] Final review: confirm Non-Goals 1–6 are honored (no unknown-key
  rejection, no full nested-DSL modeling, no long-form grammar, no build-time
  source rewriting, `$schema` precedence unchanged, root `hr` gone).

### Validation Checkpoint

- [x] All 10 spec tests above pass as automated tests in the repo.
- [x] `just test`, `just test-l2`, `just lint`, `just doctest` all green in
  `darkmatter/`.
- [x] From repo root: `just test darkmatter` green.
