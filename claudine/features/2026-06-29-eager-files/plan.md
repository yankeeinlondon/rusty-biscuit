---
agent: codex/
phases: 6
created: 2026-06-30
start_phase: 1
yolo: true
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
documentation:
  - claudine/features/2026-06-29-eager-files/spec.md
  - claudine/features/2026-06-29-eager-files/plan.md
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

- [ ] Add `Constraint::Eager` in `darkmatter/lib/src/markdown/schemas/simplified/types.rs`, including the display/name arm returning `eager`.
- [ ] Update `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs::parse_one_constraint` so bare `eager` parses as `Constraint::Eager`.
- [ ] Update `darkmatter/lib/src/markdown/schemas/simplified/serialize.rs::write_constraint` so schemas round-trip `file(eager)` back to `eager`.
- [ ] Enforce `eager` as file-only in the type-aware conversion path, returning a fatal schema-preparation error for representative invalid declarations such as `string(eager)` and `number(eager)`.
- [ ] Audit parser error wording for invalid constraints and make sure the offending type and constraint name are visible for `eager` misuse.
- [ ] Validation checkpoint: run the focused SimplifiedSchema parser/serializer tests that cover constraint parsing and round-tripping.

## Phase 2 - Lazy/Eager JSON Schema Lowering and Format Validators

**Goal:** lower bare SimplifiedSchema `file` to a lazy syntax-only format while
preserving raw JSON Schema `darkmatter-file` as eager/existence-checking.

### Tasks

- [ ] Update `darkmatter/lib/src/markdown/schemas/simplified/convert.rs::file_fragment` so bare `file` emits `format: darkmatter-file-reference`.
- [ ] Update `file_fragment` so `file(eager)` emits `format: darkmatter-file`.
- [ ] Remove compiled JSON Schema emission of `x-darkmatter-match` from `file_fragment`; keep `Constraint::Match` only on the SimplifiedSchema data model for suggestions.
- [ ] In `darkmatter/lib/src/markdown/schemas/format.rs`, register `darkmatter-file-reference` as the lazy validator using only `biscuit_file::FileReference::new(value).is_ok()`.
- [ ] Keep `darkmatter-file` wired to the current eager validator path that resolves via document-first then launch-area fallback and fails on missing or unresolvable files.
- [ ] Confirm the lazy validator does not call `resolve()`, `resolve_from()`, `resolve_file_ref_with_fallback()`, git-aware lookup, vault lookup, environment expansion, or path existence checks.
- [ ] Validation checkpoint: add or run a minimal format-validator check proving the same missing syntactically valid path passes `darkmatter-file-reference` and fails `darkmatter-file`.

## Phase 3 - Remove Match Validation and Update Diagnostics/Descriptors

**Goal:** make `match(...)` metadata-only and keep diagnostics accurate without
accidentally resolving lazy file references.

### Tasks

- [ ] Delete `DarkmatterMatchKeyword`, `match_keyword_factory`, and the `x-darkmatter-match` registration from `darkmatter/lib/src/markdown/schemas/format.rs`.
- [ ] Remove any remaining validation-time dependency between match globs and `format: darkmatter-file`.
- [ ] Update `darkmatter/lib/src/markdown/schemas/validate.rs` so targeted invalid-file-reference diagnostics for existence failures apply only to eager `darkmatter-file`.
- [ ] Add a syntax-only diagnostic path for `darkmatter-file-reference` that reports malformed `FileReference` input without resolving the reference.
- [ ] Update `darkmatter/lib/src/markdown/schemas/about.rs` so `file` is described as lazy by default, `eager` is listed as file-only, and `match` is documented as suggestion metadata rather than validation.
- [ ] Audit hard-coded schema fragments in `darkmatter/lib/src/markdown/schemas/coerce.rs`, tests, and docs; change SimplifiedSchema bare-file equivalents to `darkmatter-file-reference` and leave intentional raw eager cases as `darkmatter-file`.
- [ ] Parallelizable: descriptor/catalog text updates can be done while diagnostics are being patched, as long as both use the final format names from Phase 2.
- [ ] Validation checkpoint: run descriptor parity tests such as `constraint_set_matches_descriptor_set` after adding the `eager` descriptor.

## Phase 4 - Darkmatter Test Coverage

**Goal:** replace eager-by-default assumptions with explicit lazy/eager matrix
coverage and protect the raw JSON Schema compatibility contract.

### Tasks

- [ ] Update `schemas/format.rs::file_format_rejects_missing_file` and its existing-file sibling into separate lazy and eager cases.
- [ ] Add the four-cell matrix for `file`, `file(required)`, `file(eager)`, and `file(eager; required)` covering absent/null and present values.
- [ ] Add malformed-reference coverage proving syntax errors are fatal for both lazy and eager `file` declarations.
- [ ] Add array coverage proving `file[]` accepts a missing syntactically valid item and `file(eager)[]` rejects a missing item.
- [ ] Update `schemas/validate.rs::darkmatter_file_match_missing_file_produces_one_file_reference_diagnostic` so lazy missing files produce zero existence diagnostics and an eager variant still produces one targeted diagnostic.
- [ ] Remove tests that assert `match(...)` rejects an existing non-matching file.
- [ ] Add a test proving an existing file that does not match the configured globs still validates, because `match(...)` is metadata only.
- [ ] Add completion/schema-shape coverage proving `Constraint::Match` patterns still reach `CompletionKind::File` after `x-darkmatter-match` is removed from compiled JSON Schema.
- [ ] Add representative fatal schema-preparation tests for `eager` on non-file types, including at least `string(eager)` and `number(eager)`.
- [ ] Add raw JSON Schema compatibility tests proving `format: darkmatter-file` remains eager and `format: darkmatter-file-reference` is lazy syntax-only.
- [ ] Parallelizable: format, validate, completion, and descriptor tests can be implemented in separate work streams after Phase 3 lands.
- [ ] Validation checkpoint: run `just test darkmatter` from the repo root, or the nearest package-area `just test` if working inside `darkmatter`.

## Phase 5 - Claudine Consumption and Motivating E2E Coverage

**Goal:** verify Claudine continues to trust Darkmatter validation and prove the
reported prompt shape now works.

### Tasks

- [ ] Inspect `claudine/lib/src/composition/schema_validation.rs` to confirm it does not add independent file-existence checks and that `required` categorization remains presence-only.
- [ ] Confirm `InteractiveShape::File` still receives match patterns from the simplified schema and does not need eager/lazy state for candidate suggestions.
- [ ] Add a Darkmatter `md compose` test mirroring the motivating schema: `review: file(eager; required; match(**/*review*.md))`, `plan: file`, and `plan` pointing at a not-yet-existing output path.
- [ ] Add a Claudine `claudine compose` or wrapper-level E2E test for the same prompt shape, proving lazy `plan` composes while missing eager `review` still fails.
- [ ] Verify existing faceted Claudine error behavior for eager failures still reports `composition.invalid_file_reference` with the current required/optional categorization.
- [ ] Parallelizable: the Darkmatter motivating test and Claudine E2E test can be authored separately once Phase 4 behavior is available.
- [ ] Validation checkpoint: run `just test claudine` and, if CLI tests changed, the package-area Claudine L2 command (`just test-l2`) for the affected CLI coverage.

## Phase 6 - Final Validation, Documentation Sweep, and Closeout

**Goal:** make the change ready for implementation review with behavior,
diagnostics, and documentation aligned to the spec.

### Tasks

- [ ] Run `cargo check -p darkmatter -p claudine -p claudine-cli` or the repo's equivalent package checks if package names differ.
- [ ] Run `just test darkmatter` and `just test claudine` from the repo root.
- [ ] Run `just lint` in each touched package area if available and time permits; do not run `cargo fmt` unless explicitly requested.
- [ ] Review all touched comments and rustdoc near changed behavior; update or delete drifted comments, assuming code behavior is authoritative.
- [ ] Update any public docs or schema-language reports that still imply bare `file` validates existence or that `match(...)` rejects values.
- [ ] Confirm no prompt-audit sweep was included; per D3, owner-managed prompt migrations are out of scope except for tests needed to prove this feature.
- [ ] Check `git diff --stat` and `git diff` to verify the change is limited to Darkmatter schema behavior, Claudine consumption tests, and directly related docs.
- [ ] Validation checkpoint: final review must show bare SimplifiedSchema `file` is syntax-only, `file(eager)` is existence-checking, raw `darkmatter-file` remains eager, `darkmatter-file-reference` is lazy, and `match(...)` is metadata only.
