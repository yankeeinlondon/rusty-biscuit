---
agent: codex/
phases: 6
created: 2026-06-30
start_phase: 1
yolo: true
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_atom_file_bare.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_atom_file_match.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_atom_file_array_min_items.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_full_document_schema.snap
  - darkmatter/lib/tests/fixtures/validate/file_match_nonmatching_still_valid/Cargo.toml
  - darkmatter/lib/tests/fixtures/validate/file_match_nonmatching_still_valid/doc.md
  - darkmatter/lib/tests/fixtures/validate/file_match_nonmatching_still_valid/expected.json
  - claudine/lib/src/composition/schema_validation.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - darkmatter
  - claudine
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - claudine/cli/src/completion/schema_completion.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - darkmatter
  - claudine-cli
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/completion.rs
  - darkmatter/lib/tests/fixtures/validate/file_eager_match_nonmatching_still_valid/doc.md
  - darkmatter/lib/tests/fixtures/validate/file_eager_match_nonmatching_still_valid/expected.json
  - darkmatter/lib/tests/fixtures/validate/file_eager_match_nonmatching_still_valid/Cargo.toml
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - darkmatter
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - claudine/cli/tests/compose_schema_cli.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter
  - claudine-cli
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/topics/schema-definition.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - darkmatter
source_code:
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/coerce.rs
  - darkmatter/lib/src/markdown/schemas/completion.rs
  - claudine/lib/src/composition/schema_validation.rs
  - claudine/cli/src/completion/schema_completion.rs
documentation:
  - claudine/features/2026-06-29-eager-files/spec.md
  - claudine/features/2026-06-29-eager-files/plan.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/topics/schema-definition.md
packages:
  - darkmatter
  - claudine
  - claudine-cli
---

# Execution Plan - Lazy `file` References With Opt-In `eager`

Implements [`spec.md`](spec.md): SimplifiedSchema `file` becomes lazy by
default, `file(eager)` preserves the existing existence check, `match(...)`
becomes suggestion metadata only, and raw JSON Schema `format:
darkmatter-file` keeps its established eager behavior.

## Plan Summary

| Phase | Scope | Parallelizable |
| ----- | ----- | -------------- |
| 1 | Add the `eager` constraint to the SimplifiedSchema model/parser/serializer | No |
| 2 | Split JSON Schema lowering and runtime format validators into lazy/eager paths | No |
| 3 | Remove `match` validation and update diagnostics/descriptors/coercion fragments | Partly |
| 4 | Add and update Darkmatter unit coverage | Partly |
| 5 | Verify Claudine consumption and add motivating E2E coverage | Partly |
| 6 | Run final validation, documentation sweep, and closeout checks | No |

## Dependency Notes

- Phase 1 is the foundation. Later phases need `Constraint::Eager` to exist and
  round-trip.
- Phase 2 depends on Phase 1 because lowering must branch on `Constraint::Eager`.
- Phase 3 depends on Phase 2 for the final format names, but descriptor edits
  can proceed in parallel with diagnostic updates once the lowering behavior is
  settled.
- Phase 4 test updates depend on Phases 1-3. Independent test files can be
  split across implementers.
- Phase 5 depends on the Darkmatter behavior being available to Claudine.
- Phase 6 depends on all implementation and tests.

## Phase 1 - SimplifiedSchema Constraint Plumbing

**Goal:** represent `eager` as a first-class SimplifiedSchema constraint and
reject it anywhere except `file`.

### Tasks

- [x] Add `Constraint::Eager` in `darkmatter/lib/src/markdown/schemas/simplified/types.rs`, including the display/name arm returning `eager`.
- [x] Update `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs::parse_one_constraint` so bare `eager` parses as `Constraint::Eager`.
- [x] Update `darkmatter/lib/src/markdown/schemas/simplified/serialize.rs::write_constraint` so schemas round-trip `file(eager)` back to `eager`.
- [x] Enforce `eager` as file-only in the type-aware conversion path, returning a fatal schema-preparation error for representative invalid declarations such as `string(eager)` and `number(eager)`.
- [x] Audit parser error wording for invalid constraints and make sure the offending type and constraint name are visible for `eager` misuse.
- [x] Validation checkpoint: run the focused SimplifiedSchema parser/serializer tests that cover constraint parsing and round-tripping.

## Phase 2 - Lazy/Eager JSON Schema Lowering and Format Validators

**Goal:** lower bare SimplifiedSchema `file` to a lazy syntax-only format while
preserving raw JSON Schema `darkmatter-file` as eager/existence-checking.

### Tasks

- [x] Update `darkmatter/lib/src/markdown/schemas/simplified/convert.rs::file_fragment` so bare `file` emits `format: darkmatter-file-reference`.
- [x] Update `file_fragment` so `file(eager)` emits `format: darkmatter-file`.
- [x] Confirm the array forms lower **per item**: `file(eager)[]` produces `items` carrying `format: darkmatter-file` and `file[]` produces `items` carrying `format: darkmatter-file-reference`. `eager` is routed as an item-level constraint into `file_fragment`; array constraints (`min`, `unique`, …) stay on the array wrapper and must not reach `file_fragment` (they would trip `invalid_constraint`).
- [x] Remove compiled JSON Schema emission of `x-darkmatter-match` from `file_fragment`; keep `Constraint::Match` only on the SimplifiedSchema data model for suggestions.
- [x] In `darkmatter/lib/src/markdown/schemas/format.rs`, register `darkmatter-file-reference` as the lazy validator using only `biscuit_file::FileReference::new(value).is_ok()`.
- [x] Keep `darkmatter-file` wired to the current eager validator path that resolves via document-first then launch-area fallback and fails on missing or unresolvable files.
- [x] Confirm the lazy validator does not call `resolve()`, `resolve_from()`, `resolve_file_ref_with_fallback()`, git-aware lookup, vault lookup, environment expansion, or path existence checks.
- [x] Verify the precondition the lazy contract rests on: `biscuit_file::FileReference::new()` is itself construction-only (no filesystem/git/env/vault IO). If it performs any IO, the lazy validator must use a purely-syntactic check instead so that `{{MISSING_ENV}}/out.md`, `vault:future.md`, `%@future.md`, and `./future.md` all pass lazy validation per spec.
- [x] Validation checkpoint: add or run a minimal format-validator check proving the same missing syntactically valid path passes `darkmatter-file-reference` and fails `darkmatter-file`.

## Phase 3 - Remove Match Validation and Update Diagnostics/Descriptors

**Goal:** make `match(...)` metadata-only and keep diagnostics accurate without
accidentally resolving lazy file references.

### Tasks

- [x] Delete `DarkmatterMatchKeyword`, `match_keyword_factory`, and the `x-darkmatter-match` registration from `darkmatter/lib/src/markdown/schemas/format.rs`.
- [x] Remove any remaining validation-time dependency between match globs and `format: darkmatter-file`.
- [x] Update `darkmatter/lib/src/markdown/schemas/validate.rs` so targeted invalid-file-reference diagnostics for existence failures apply only to eager `darkmatter-file`.
- [x] Add a syntax-only diagnostic path for `darkmatter-file-reference` that reports malformed `FileReference` input without resolving the reference.
- [x] Update `darkmatter/lib/src/markdown/schemas/about.rs` so `file` is described as lazy by default, `eager` is listed as file-only, and `match` is documented as suggestion metadata rather than validation.
- [x] Audit hard-coded schema fragments in `darkmatter/lib/src/markdown/schemas/coerce.rs`, tests, and docs; change SimplifiedSchema bare-file equivalents to `darkmatter-file-reference` and leave intentional raw eager cases as `darkmatter-file`.
- [x] Parallelizable: descriptor/catalog text updates can be done while diagnostics are being patched, as long as both use the final format names from Phase 2.
- [x] Validation checkpoint: run descriptor parity tests such as `constraint_set_matches_descriptor_set` after adding the `eager` descriptor.

## Phase 4 - Darkmatter Test Coverage

**Goal:** replace eager-by-default assumptions with explicit lazy/eager matrix
coverage and protect the raw JSON Schema compatibility contract.

### Tasks

- [x] Update `schemas/format.rs::file_format_rejects_missing_file` and its existing-file sibling into separate lazy and eager cases.
- [x] Add the four-cell matrix for `file`, `file(required)`, `file(eager)`, and `file(eager; required)` covering absent/null and present values.
- [x] Add malformed-reference coverage proving syntax errors are fatal for both lazy and eager `file` declarations.
- [x] Add array coverage proving `file[]` accepts a missing syntactically valid item and `file(eager)[]` rejects a missing item.
- [x] Update `schemas/validate.rs::darkmatter_file_match_missing_file_produces_one_file_reference_diagnostic` so lazy missing files produce zero existence diagnostics and an eager variant still produces one targeted diagnostic.
- [x] Remove tests that assert `match(...)` rejects an existing non-matching file.
- [x] Add a test proving an existing file that does not match the configured globs still validates, because `match(...)` is metadata only.
- [x] Add completion/schema-shape coverage proving `Constraint::Match` patterns still reach `CompletionKind::File` after `x-darkmatter-match` is removed from compiled JSON Schema.
- [x] Add representative fatal schema-preparation tests for `eager` on non-file types, including at least `string(eager)` and `number(eager)`.
- [x] Add raw JSON Schema compatibility tests proving `format: darkmatter-file` remains eager and `format: darkmatter-file-reference` is lazy syntax-only.
- [x] Parallelizable: format, validate, completion, and descriptor tests can be implemented in separate work streams after Phase 3 lands.
- [x] Validation checkpoint: run `just test darkmatter` from the repo root, or the nearest package-area `just test` if working inside `darkmatter`.

## Phase 5 - Claudine Consumption and Motivating E2E Coverage

**Goal:** verify Claudine continues to trust Darkmatter validation and prove the
reported prompt shape now works.

### Tasks

- [x] Inspect `claudine/lib/src/composition/schema_validation.rs` to confirm it does not add independent file-existence checks and that `required` categorization remains presence-only.
- [x] Confirm `InteractiveShape::File` still receives match patterns from the simplified schema and does not need eager/lazy state for candidate suggestions.
- [x] Regression guard for claudine-cli completion: confirm `claudine/cli/src/completion/schema_completion.rs` `file(match(...))` setter-value completion still surfaces candidates after `x-darkmatter-match` is removed. It reads patterns via `dm_completion::for_property` → `CompletionKind::File` (simplified schema), so removing the compiled keyword should be a no-op — assert it, do not change the completer. (This path was fixed in the prior session; keep it green.)
- [x] Add a Darkmatter `md compose` test mirroring the motivating schema: `review: file(eager; required; match(**/*review*.md))`, `plan: file`, and `plan` pointing at a not-yet-existing output path.
- [x] Add a Claudine `claudine compose` or wrapper-level E2E test for the same prompt shape, proving lazy `plan` composes while missing eager `review` still fails.
- [x] Verify existing faceted Claudine error behavior for eager failures still reports `composition.invalid_file_reference` with the current required/optional categorization.
- [x] Parallelizable: the Darkmatter motivating test and Claudine E2E test can be authored separately once Phase 4 behavior is available.
- [x] Validation checkpoint: run `just test claudine` and, if CLI tests changed, the package-area Claudine L2 command (`just test-l2`) for the affected CLI coverage.

## Phase 6 - Final Validation, Documentation Sweep, and Closeout

**Goal:** make the change ready for implementation review with behavior,
diagnostics, and documentation aligned to the spec.

### Tasks

- [x] Run `cargo check -p darkmatter -p claudine -p claudine-cli` or the repo's equivalent package checks if package names differ.
- [x] Run `just test darkmatter` and `just test claudine` from the repo root.
- [x] Run `just lint` in each touched package area if available and time permits; do not run `cargo fmt` unless explicitly requested.
- [x] Review all touched comments and rustdoc near changed behavior; update or delete drifted comments, assuming code behavior is authoritative.
- [x] Update any public docs or schema-language reports that still imply bare `file` validates existence or that `match(...)` rejects values.
- [x] Confirm no prompt-audit sweep was included; per D3, owner-managed prompt migrations are out of scope except for tests needed to prove this feature.
- [x] Check `git diff --stat` and `git diff` to verify the change is limited to Darkmatter schema behavior, Claudine consumption tests, and directly related docs.
- [x] Validation checkpoint: final review must show bare SimplifiedSchema `file` is syntax-only, `file(eager)` is existence-checking, raw `darkmatter-file` remains eager, `darkmatter-file-reference` is lazy, and `match(...)` is metadata only.
