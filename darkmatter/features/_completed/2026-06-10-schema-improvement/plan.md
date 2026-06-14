---
agent: open_code
phases: 4
created: "2026-06-10"
start_phase: 1
hash: ""
source_code:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/detect.rs
  - darkmatter/lib/src/markdown/schemas/completion.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/src/commands/schema/mod.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/src/commands/schema/assignment.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
  - darkmatter/lib/tests/schemas_convert_snapshots.rs
  - darkmatter/cli/tests/schema_about.rs
documentation:
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/lib/src/markdown/schemas/about.rs
packages:
  - darkmatter
  - darkmatter-cli
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/detect.rs
  - darkmatter/lib/src/markdown/schemas/completion.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/cli/src/commands/schema/assignment.rs
  - darkmatter/lib/tests/schemas_grammar_proptest.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/tests/schemas_convert_snapshots.rs
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_valid/doc.md
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_valid/expected.json
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_missing_required/doc.md
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_missing_required/expected.json
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_wrong_type/doc.md
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_wrong_type/expected.json
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_extra_property_rejected/doc.md
  - darkmatter/lib/tests/fixtures/validate/inline_object_array_extra_property_rejected/expected.json
  - darkmatter/lib/tests/fixtures/validate/opaque_object_array_accepts_extra_property/doc.md
  - darkmatter/lib/tests/fixtures/validate/opaque_object_array_accepts_extra_property/expected.json
  - darkmatter/lib/tests/fixtures/validate/property_union_invalid/doc.md
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_bare.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_array.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_required_value.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_constrained_array.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_nested.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_multi_line.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_as_union_arm.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_mixed_with_string_fallback.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_inline_object_empty.snap
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/src/commands/schema/mod.rs
  - darkmatter/cli/src/commands/schema/about.rs
  - darkmatter/cli/tests/schema_about.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
docs_updated_during_phase_4:
  - darkmatter/docs/topics/schema-definition.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
---

# Execution Plan: Inline Nested Object Schemas in SimplifiedSchema

## Overview

Add inline object literal syntax to the SimplifiedSchema grammar, compile it to Draft 2020-12 JSON Schema, extend compose-time coercion to recurse into inline objects, and expose a descriptor-backed `md schema about` command for discoverability.

---

## Phase 1 — Core Data Model & Grammar Parser

**Goal:** Extend the AST and string-layer grammar parser so inline object literals parse correctly into `TypeExpr::InlineObject(SchemaShape)`.

**Parallelizable:** No — each task depends on the previous data model.

- [x] Add `TypeExpr` enum and update `PropertyAtom.ty` field in `types.rs`
- [x] Implement recursive `inline_object` parser in `grammar.rs` with depth limit enforcement
- [x] Handle whitespace stripping inside `{ ... }` per Decision #2
- [x] Parse property definitions with all four syntax forms inside inline objects
- [x] Handle `->` description termination at comma or closing brace at current nesting level
- [x] Enforce hard max nesting depth of 32 levels with `SchemaError::Grammar`
- [x] Handle `[]` array suffix and postfix constraints on inline objects
- [x] Add grammar parser unit tests covering all parser test cases from the spec
- [x] **Validation checkpoint:** Run parser unit tests — all must pass

---

## Phase 2 — JSON Schema Conversion & Compose Coercion

**Goal:** Compile inline objects to JSON Schema and extend compose-time coercion to recurse into nested inline object fields.

**Parallelizable:** No — coercion depends on conversion, which depends on the AST.

- [x] Implement `TypeExpr::InlineObject` branch in `convert::type_fragment`
- [x] Emit `additionalProperties: false` for every inline object fragment
- [x] Hoist `required` constraints to parent `required` arrays correctly
- [x] Handle inline object arrays: wrap in `{ type: array, items: <object fragment> }` with array constraints
- [x] Handle inline object as union arm in `anyOf` compilation
- [x] Extend compose coercion to recurse into inline object fields at nested paths
- [x] Implement per-arm coercion for property-level unions with inline object arms
- [x] Handle zero-match and ambiguous union coercion (leave original uncoerced)
- [x] Add conversion snapshot tests and validation integration tests
- [x] Add compose coercion integration tests
- [x] Add backward-compatibility tests ensuring existing v1 schemas parse identically
- [x] **Validation checkpoint:** Run conversion snapshot tests, validation integration tests, and coercion integration tests — all must pass

---

## Phase 3 — Schema About Command & Descriptor Catalog

**Goal:** Build a typed descriptor catalog for the schema language and expose it via `md schema about` and a public library API.

**Parallelizable:** No — CLI depends on the descriptor catalog library API.

- [x] Define schema-language descriptor types in `about.rs`
- [x] Implement descriptor catalog covering: schema shapes, type vocabulary, constraints, inline object rules, coercion rules
- [x] Ensure every `SimplifiedType` has exactly one descriptor and every constraint has descriptor coverage
- [x] Add `md schema about` CLI command in `args.rs` and `commands/schema/`
- [x] Render human-readable report from descriptor catalog (not hand-maintained prose)
- [x] Ensure `md schema about` performs no document parsing, context capture, effect-engine construction, file resolution, or network access
- [x] Expose public schema-language descriptor API for library callers
- [x] Add schema about tests: rendering, descriptor parity, deterministic order, uniqueness, no side effects
- [x] **Validation checkpoint:** Run schema about tests — all must pass

---

## Phase 4 — Documentation & Final Integration

**Goal:** Update public documentation and run full integration to ensure end-to-end correctness.

**Parallelizable:** Documentation update can happen in parallel with final integration testing if Phase 3 is complete.

- [x] Update `darkmatter/docs/topics/schema-definition.md` with inline object syntax, postfix constraints, identifier rules, description comma limits, nested coercion, and 32-level nesting limit
- [x] Document `md schema about` as the implementation-bound CLI reference
- [x] Update YAML-shape layer error wording in `mod.rs` for mapping values
- [x] Run the full darkmatter test suite (unit + integration)
- [x] Run `md schema validate` and `md compose` against example schemas from the spec to verify end-to-end behavior
- [x] **Validation checkpoint:** Full test suite passes; no regressions in existing v1 schemas

---

## Risk Mitigation Checkpoints

- [x] **Grammar ambiguity** — Verify `{` is rejected inside constraint argument lists (existing behavior)
- [x] **Parser recursion depth** — Verify depth 33 returns `SchemaError::Grammar` (test in Phase 1)
- [x] **`additionalProperties: false` surprise** — Verify opaque `object` still compiles without `additionalProperties: false` (backward-compat test in Phase 2)
- [x] **Property-level union hoisting** — Verify inner `required` stays inside inline object fragment, not hoisted to property level (conversion test in Phase 2)

---

## Task Summary

| Phase | Tasks | Parallelizable |
|-------|-------|---------------|
| Phase 1 | 8 tasks | No |
| Phase 2 | 11 tasks | No |
| Phase 3 | 9 tasks | No |
| Phase 4 | 5 tasks | Partial (docs + tests in parallel) |

**Total phases:** 4  
**Starting phase:** 1
