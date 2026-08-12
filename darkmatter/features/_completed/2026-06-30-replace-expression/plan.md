---
agent: codex/
phases: 5
created: 2026-06-30
start_phase: 1
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
docs_updated_during_phase_2:
  - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
docs_updated_during_phase_4:
  - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
documentation:
  - darkmatter/docs/topics/darkmatter-expressions.md
packages: [darkmatter]
---

# Replace Expression Functions Plan

## Success Criteria

- [x] `replace(x, find, replacement)`, `replace_first(x, find, replacement)`, and `replace_last(x, find, replacement)` are available as pure Darkmatter expression functions.
- [x] Each function accepts exactly three string arguments, propagates `null`, errors on non-string arguments, treats an empty `find` as a no-op, and performs literal case-sensitive substring replacement.
- [x] `replace_first` and `replace_last` expose underscore-free aliases `replacefirst` and `replacelast`; `replace` has no alias.
- [x] Runtime function registration, descriptor catalog entries, and the generated docs table stay in parity.
- [x] Focused unit tests, catalog parity tests, docs-sync tests, and the Darkmatter package test recipe pass.

## Phase 1: Orientation And Baseline

- [x] Read `darkmatter/features/2026-06-30-replace-expression/spec.md` and confirm the accepted contracts: subject-first signature, empty-`find` no-op, no-match no-op, literal matching, case sensitivity, null propagation, type errors, and arity errors.
- [x] Inspect `darkmatter/lib/src/markdown/compose/expression/functions.rs` for existing string mutation handler style, especially `string_mutation`, `without_date`, `ensure_leading`, `ensure_trailing`, `PURE_FUNCTIONS`, `dispatch`, and the `fn_string_mutations` / `fn_phase3` test modules.
- [x] Inspect `darkmatter/lib/src/markdown/compose/expression/catalog.rs` for the `String Mutations` descriptor order, example conventions, `generate_expression_function_table`, runtime/catalog parity tests, and docs-sync tests.
- [x] Inspect `darkmatter/docs/topics/darkmatter-expressions.md` around `<!-- BEGIN GENERATED FUNCTION TABLE -->` and the prose list of string mutations to identify all documentation updates.
- [x] Validation checkpoint: record the exact files expected to change before implementation: `functions.rs`, `catalog.rs`, and `darkmatter-expressions.md`; avoid unrelated refactors or formatting churn.

## Phase 2: Runtime Tests

- [x] Add focused unit tests in `functions.rs` under the existing string mutation tests for core replacement behavior:
  `replace("a.b.c", ".", "/")`, `replace_first("a.b.c", ".", "/")`, `replace_last("a.b.c", ".", "/")`, `replace("aaa", "a", "bb")`, `replace_first("foofoo", "foo", "bar")`, and `replace_last("foofoo", "foo", "bar")`.
- [x] Add unit tests for empty `find` no-op behavior for all three functions.
- [x] Add unit tests for no-match no-op behavior for all three functions.
- [x] Add a case-sensitive literal matching test, including punctuation that would be special in regex syntax.
- [x] Add null propagation tests covering all three argument positions for all three functions.
- [x] Add type error tests covering non-string subject, non-string `find`, and non-string `replacement` for all three functions.
- [x] Add arity tests proving each function rejects fewer or more than three arguments via the existing arity helper.
- [x] Add dispatch tests for canonical names and aliases: `replace`, `replace_first`, `replacefirst`, `replace_last`, and `replacelast`.
- [x] Validation checkpoint: run the narrow expression function tests with nextest or the package's `just test` filter if available; expect compile failures until the handlers are implemented.

## Phase 3: Runtime Implementation

- [x] Add `replace(args: &[Value]) -> Result<Value, String>` in `functions.rs` near the existing string mutations, using `require_args("replace", args, 3)`, `any_null(args)`, and `require_string("replace", ...)`.
- [x] Implement `replace` so `find == ""` returns the original subject string unchanged, otherwise returns `x.replace(find, replacement)`.
- [x] Add `replace_first(args: &[Value]) -> Result<Value, String>` using the same arity, null propagation, and string requirements under the function name `replace_first`.
- [x] Implement `replace_first` so `find == ""` returns the original subject string unchanged, otherwise uses `find`/slicing or `replacen(..., 1)` to replace only the leftmost match.
- [x] Add `replace_last(args: &[Value]) -> Result<Value, String>` using the same arity, null propagation, and string requirements under the function name `replace_last`.
- [x] Implement `replace_last` so `find == ""` returns the original subject string unchanged, otherwise uses `rfind` and UTF-8-safe slicing at the returned byte boundary to replace only the rightmost match.
- [x] Register all three handlers in `PURE_FUNCTIONS` in the `String mutations` block after the existing `ensure_*` entries or another locally consistent location:
  `replace` with `aliases: &[]`, `replace_first` with `aliases: &["replacefirst"]`, and `replace_last` with `aliases: &["replacelast"]`.
- [x] Validation checkpoint: run the focused runtime tests and confirm all new behavior, null propagation, type errors, arity errors, canonical dispatch, and alias dispatch pass.

## Phase 4: Catalog And Documentation

- [x] Add `ExpressionFunctionDescriptor` entries for `replace(x, find, replacement)`, `replace_first(x, find, replacement)`, and `replace_last(x, find, replacement)` in the `String Mutations` category in `catalog.rs`.
- [x] Continue the existing `String Mutations` `order` sequence without renumbering unrelated categories; use concise descriptions that state all/first/last literal occurrence replacement.
- [x] Add executable examples in the descriptor entries that match the accepted behavior, such as `replace("a.b.c", ".", "/")` => `a/b/c`, `replace_first("a.b.c", ".", "/")` => `a/b.c`, and `replace_last("a.b.c", ".", "/")` => `a.b/c`.
- [x] Update `darkmatter/docs/topics/darkmatter-expressions.md` generated function table rows for the three new descriptors, preserving exact output expected by `generate_expression_function_table`.
- [x] Update any nearby hand-maintained string mutation summary list in `darkmatter-expressions.md` so it includes `replace`, `replace_first`, and `replace_last`.
- [x] Parallelizable after descriptor signatures are finalized: one implementer can update docs while another finishes runtime tests, as long as the final docs table is regenerated or manually matched to `generate_expression_function_table`.
- [x] Validation checkpoint: run the catalog tests that compare runtime signatures to descriptors and compare `darkmatter-expressions.md` to generated catalog output.

## Phase 5: Final Verification And Handoff

- [x] Run `just test` from the `darkmatter` package area to execute the package unit test suite through nextest.
- [x] Run any focused test commands used during development one final time if `just test` output is too broad to clearly identify the new coverage.
- [x] Run `just lint` from the `darkmatter` package area if the implementation touched public API docs or exposed warnings; do not run `cargo fmt` unless explicitly requested.
- [x] Review `git diff -- darkmatter/lib/src/markdown/compose/expression/functions.rs darkmatter/lib/src/markdown/compose/expression/catalog.rs darkmatter/docs/topics/darkmatter-expressions.md` for unrelated formatting churn, stale comments, or behavior/doc drift.
- [x] Validation checkpoint: confirm every acceptance criterion from the spec has a passing test or an enforced parity/docs check.
- [x] Handoff note: summarize changed files, tests run, any skipped validation, and the duplicate frontmatter `agent` instruction resolution if relevant to the implementation record.
