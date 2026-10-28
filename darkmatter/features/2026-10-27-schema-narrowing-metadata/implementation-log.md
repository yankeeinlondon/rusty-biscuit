---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-27-schema-narrowing-metadata/spec.md"
plan: "darkmatter/features/2026-10-27-schema-narrowing-metadata/plan.md"
implemented_by: "claude/sonnet"
started_phase: 1
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
decisions_for_review:
  - |-
    Narrowing target of a grouped-union array: the plan says a target must be
    one non-reference arm. I reject only named references (`Name`,
    `Name@file`) and multi-arm unions; a grouped union followed by `[]`
    (`(a | b)[]`) parses as one array atom and is accepted. Rejecting it too
    would be a one-line change in `narrows_target_problem` (types.rs).
message_to_agent: |-
  Phase 1 added `FunctionType.narrows: Option<NarrowsClause>` (types.rs) with
  `NarrowsKind`, span-free equality, and shared `pub(crate)` helpers
  `returns_exact_boolean` and `narrows_target_problem` in types.rs for the
  Phase 2 compiler to reuse. Grammar already rejects non-boolean returns,
  undeclared/rest parameter, union and named targets. `just lint` currently
  fails in darkmatter-cli (`cli/src/commands/render.rs:22` too_many_arguments),
  pre-existing and unrelated; lib and dmls clippy are clean.
---

# Implementation Log for 2026-10-27-schema-narrowing-metadata (5 phases)

## Phase 1

- Added `NarrowsClause`/`NarrowsKind` and `FunctionType.narrows` (types.rs);
  `parse_function` parses `narrows(name[: type]; kind)` in any clause order,
  with post-loop checks (duplicate, parameter declared/not rest, boolean-only
  returns, single non-reference target, kind keyword). EBNF and the
  unknown-constraint message updated.
- `write_function` serializes the clause right after `returns(...)`;
  `canonical_atoms` clears it; subtype deliberately ignores it (comment added).
- Tests (in-process via `parse_type_definition`/`parse_type_expr`):
  `grammar::narrows_clause_tests` (control, absence, all kinds, order,
  duplicate, shape errors, parameter, returns matrix),
  `serialize::narrows_serialization_tests` (repeated round trip, placement),
  `signature::narrows_contract_tests` (duplicates despite differing clauses),
  `subtype::narrows_subtype_tests`.
- Tier: all L1 unit tests in the lib crate; no spawn, no repository capture.
- Gates: `just test` 12921 passed. `just lint`: darkmatter-cli fails with a
  pre-existing `clippy::too_many_arguments` at `cli/src/commands/render.rs:22`
  (file untouched this phase); `cargo clippy -p darkmatter -p dmls
  --all-targets -D warnings` is clean. Serde-smell grep not applicable (no
  serde on the new fields).
- No skill change needed this phase (grammar surfaces documented in Phase 5).
