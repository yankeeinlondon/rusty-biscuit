---
total_phases: 7
created: 2026-10-30
phase: 4
agent: opencode/zai-coding-plan/glm-5.3
yolo: 'true'
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- simplified-schema
- darkmatter
source_files_during_phase_2:
- simplified-schema/lib/src/expression/ast.rs
- simplified-schema/lib/src/expression/lexer.rs
- simplified-schema/lib/src/expression/parser.rs
- simplified-schema/lib/src/expression/mod.rs
- simplified-schema/lib/src/format.rs
- simplified-schema/lib/src/simplified/convert.rs
- simplified-schema/lib/tests/l1/expression_syntax.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- darkmatter/lib/src/catalog/mod.rs
- darkmatter/lib/src/lifecycle/action_shape.rs
- darkmatter/lib/src/lifecycle/mod.rs
- darkmatter/lib/src/lifecycle/parse.rs
- darkmatter/lib/src/lifecycle/validate.rs
- darkmatter/lib/src/markdown/compose/call_precheck.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/parser.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/scopes.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/expression/absence.rs
- darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/lint.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/prepared.rs
- darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
- darkmatter/lib/src/markdown/compose/expression/semantics.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
- darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
- darkmatter/lib/src/markdown/compose/interpolation/mod.rs
- darkmatter/lib/src/markdown/compose/page_blocks/guards.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/compose/remote.rs
- darkmatter/lib/src/markdown/compose/tests/shell.rs
- darkmatter/lib/tests/l1/binding_contract.rs
- darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
- darkmatter/lib/tests/l1/schema_language_ownership_guard.rs
- darkmatter/lib/tests/l1/schemas_literal_expression.rs
- darkmatter/lib/tests/l1/simplified_schema_reexports.rs
- darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
---

# One Meaning for `||` — Implementation Plan

Implements `darkmatter/fixes/2026-10-16-unified-logical-or/spec.md`: `||`
becomes logical OR returning the deciding operand on every surface, the parse
mode is deleted, and the AST node is renamed `LogicalOr`.

Scope verified by reading the code: only `simplified-schema` defines the
grammar (`cargo metadata` shows `darkmatter` is its only dependent); `dmls`
and `claudine` reach it through Darkmatter's re-exports. No other workspace
crate references `ParseMode`, `parse_condition`, `Token::OrOr`, or the
`Fallback` expression node (grep hits in `sniff`, `worktree`, `biscuit-speaks`,
`model-citizen`, `schematic`, `unchained-ai` are unrelated `with_model`
builders; `zed-dmls` (excluded from the workspace) references none of them).

## Summary and Definition of Done

The work renames `Expr::Fallback`/`SpannedExprKind::Fallback` to `LogicalOr`
(fields `left`/`right`), deletes the `ParseMode` machinery (`ParseMode`,
`Token::OrOr`, `Lexer::with_mode`, `Lexer::mode`, `Parser::with_mode`,
`parse_condition`, `parse_condition_spanned`, the mode parameter of
`lex_spanned` and `lint_expression`, the `AuthoredMode::Expression(ParseMode)`
payload, and the DMLS `Dialect`/`parse_condition`/`hover_markdown_condition`
wrappers), so `||` parses to one node with one meaning everywhere, and updates
every caller in four crates plus their documentation and skills.

Done means:

- Acceptance criteria 1–6 of the spec hold, each covered by a test named in
  Phase 5 (value parity across `{{ }}`, `::block when=`, and Claudine
  lifecycle `when:`; the `open_questions` guard fires; AST-shape tests rewritten
  to `LogicalOr`, never deleted; the mode symbols are gone;
  left-operand absence handling in conditions; the nested-span suggestion is
  offered for `a == "x {{ y || 'd' }}"`).
- `cargo check --workspace --all-features --all-targets` passes (AC4).
- `just test` and `just lint` pass in `simplified-schema`, `darkmatter`
  (includes dmls + zed-dmls-cli), and `claudine`; the prompt corpora pass
  (`just test-prompts` in claudine, and darkmatter's prompts test target).
- The repository safety search (no condition whose `||` result is consumed as
  a value) has been re-run immediately before landing and its outcome recorded
  in the implementation log.
- Docs and skills listed in Phase 6 describe the unified behavior; no doc or
  skill still describes a mode split.

Input Robustness Matrix: skipped — this changes a text grammar embedded in
Markdown (the matrix's explicit exclusion), and the accepted language is
unchanged: the condition grammar was a superset of the value grammar that
differed only in how `||` lowered, and `||` parses in both today, so every
string rejected before stays rejected with a located error, and none newly
accepted.

## Phase 1 — Baseline and Decisions

Phase 1 takes no code changes. It re-establishes the spec's safety claim on
the current tree and fixes every implementation decision so later phases
execute without choices.

### Necessary Rules

- **R1 — Node rename.** `Expr::Fallback { primary, fallback }` becomes
  `Expr::LogicalOr { left, right }`, and `SpannedExprKind::Fallback` becomes
  `SpannedExprKind::LogicalOr { left, right }`, in
  `simplified-schema/lib/src/expression/ast.rs` (definitions at lines 139–146
  and 320–325, `Display` at 222 keeps rendering `a || b`, `erase` at 409,
  module docs at 13/23–25, test at 651). Spec D1 names the node and fields.
  Rename sites, all verified by grep
  (`Expr::Fallback|SpannedExprKind::Fallback`): darkmatter
  `markdown/compose/expression/{mod.rs:662,1170,1189, absence.rs:327,421,483,533,
  call_check.rs:196,1197,1303, lint.rs:190,487, prepared.rs:651,
  scope/facts.rs:28, frontmatter_interpolation.rs:1424,1559,
  interpolation/evaluator.rs:461, preflight/collect.rs:2003, remote.rs:457}`;
  dmls `overlay/expressions.rs:502,559,1721`; claudine
  `composition/lifecycle/prewarm/classify.rs:252,338`,
  `composition/prepare/tests.rs:579`, `dispatch/matcher.rs:137`; plus tests in
  `simplified-schema/lib/src/expression/parser.rs`,
  `simplified-schema/lib/tests/l1/expression_syntax.rs`, and
  `darkmatter/lib/tests/l1/expression_regression.rs`.
- **R2 — One token.** `||` lexes as the existing `Token::Pipe` on every
  surface; `Token::OrOr` is deleted. `Token::Pipe`'s `Display` stays `"|"`
  (asserted by tests today; the spec does not change it); its doc comment
  becomes "The `||` logical OR operator." Evidence: spec D2 deletes only
  `OrOr`; lexer.rs:32–33, 106–108, 296–313.
- **R3 — One bare-`|` error.** The lexer's lone-`|` error is worded once:
  `Unexpected '|'. Use '||'.` (today two wordings at lexer.rs:306–312). Spec
  D2 fixes the wording; the two tests asserting the old wordings
  (`error_bare_pipe_in_interpolation`, `error_bare_pipe_in_condition`) merge
  into one.
- **R4 — Deleted API.** In `simplified-schema/lib/src/expression/`:
  `ParseMode`, `Lexer::with_mode`, `Lexer::mode`, `Parser::with_mode`,
  `parse_condition`, `parse_condition_spanned`, and `Token::OrOr` are deleted;
  `lex_spanned(input)` loses its mode parameter (it is a public signature that
  takes a mode — same rule as `lint_expression`, spec D2); `Lexer::new` and
  `Parser::new` remain the constructors. Re-exports in
  `simplified-schema/lib/src/expression/mod.rs:23–26` drop `ParseMode`,
  `parse_condition`, `parse_condition_spanned`. Evidence: spec D2; lexer.rs
  114–132, 246–277, 682–697; parser.rs 158–180, 883–925.
- **R5 — Grammar shape.** `parse_ternary_branch` is deleted;
  `parse_ternary` calls the renamed rule directly. Today's `parse_fallback`
  (parser.rs:465–482) is renamed `parse_logical_or` and builds
  `SpannedExprKind::LogicalOr { left, right }`; today's `or(...)`-lowering
  `parse_logical_or` (parser.rs:417–434) is deleted. `parse_logical_and`
  (parser.rs:442–459) is unchanged: `&&` still lowers to `and(a, b)` (spec D4).
  Module grammar docs (parser.rs:6–24) keep one `logical_or` rule.
- **R6 — Precedence tables.** `PRECEDENCE_TABLE`
  (simplified-schema/lib/src/expression/parser.rs:934–943) rows become
  `("Logical AND", "`&&`")` and `("Logical OR", "`||`")`, and darkmatter's
  `OPERATOR_DESCRIPTORS` names match pairwise — the test
  `operator_precedence_matches_parser`
  (darkmatter/lib/src/markdown/compose/expression/semantics.rs:1049–1069)
  asserts equal length and name equality by position, so both files change in
  the same change. Spec Documentation names both entries.
- **R7 — Semantics descriptors.** In
  `darkmatter/lib/src/markdown/compose/expression/semantics.rs`: the operator
  entry (lines 113–123) becomes name `"Logical OR"`, operators `` `||` ``,
  description `"Returns the operand that decided the result: the first truthy
  operand, otherwise the last."` (spec wording, no mode clause); `"Logical
  AND"`'s description (line 105) drops the mode clause. `ModeDescriptor`,
  `MODE_DESCRIPTORS` (262–319), `mode_descriptors()` (775–780), the
  `mode_examples_evaluate_correctly` test (916–932), and the catalog check
  line `darkmatter/lib/src/catalog/mod.rs:465` are deleted — grep shows the
  only other consumer is the Claudine CLI renderer handled by R12.
- **R8 — `AuthoredMode::Expression` loses its payload.** The variant becomes
  a unit variant (prepared.rs:55–65); `parse_in_mode` is deleted and the
  `Expression` arm calls `parse_spanned`. Callers:
  `darkmatter/lib/src/lifecycle/validate.rs:180`,
  `claudine/lib/src/composition/lifecycle/binding_checks.rs:58`,
  `claudine/lib/src/composition/sequence/expr.rs:190`, and tests
  (`prepared.rs:981`, `darkmatter/lib/tests/l1/binding_contract.rs:599–642`).
  Evidence: spec D2 — public signatures that take a mode lose it.
- **R9 — Lint.** `lint_expression(source)` (spec D2 names this signature);
  `lint_spanned(source, expr)` keeps its guard that the re-parse reproduces
  `expr` but re-parses once with `parse_spanned`; `rewrite` proves the
  suggestion with `parse_spanned`; `lint_with_modes`/`rewrite` lose their
  `modes` parameters. The test
  `fallback_span_in_condition_dialect_has_no_suggestion` (lint.rs:869–876) is
  rewritten to assert `a == "x {{ y || 'd' }}"` **is** offered a suggestion
  (spec D2 and AC6), and `suggestion_reparses_in_the_original_dialect`
  (lint.rs:892–901) reparses via `parse_spanned`. Evidence: lint.rs:59–86,
  123–151, 305–343.
- **R10 — DMLS wrappers deleted.** In `darkmatter/dmls/`: the public
  `expressions::parse_condition` (overlay/expressions.rs:445–457) is deleted —
  its callers (`diagnostics/frontmatter.rs:776`,
  `providers/frontmatter.rs`, `providers/dsl.rs:1088`,
  `providers/semantic_tokens.rs:623`, `overlay/event_views.rs:212`) use
  `expressions::parse`; `hover_markdown_condition`
  (overlay/expressions.rs:1184–1205) is deleted — its callers use
  `hover_markdown`; `hover_markdown_in_event` (1226–1244) loses its `parser`
  parameter (every caller passed a dialect-selected parser:
  providers/frontmatter.rs:1261, 1289; diagnostics/frontmatter.rs:983); the
  `Dialect` enum and `dialect` field (overlay/event_views.rs:198–215, 228) are
  deleted and `StackExpression::parse` calls `expressions::parse` — grep shows
  `.dialect` is consumed only through `.parser()` at those two sites plus
  tests; `nested_span_lints(document, text, offset, mode)` (258–287) and
  `FrontmatterInterpolation::nested_span_lints(document, mode)` (353–357) lose
  the `mode` parameter. Spec D2: the mode is removed rather than left as a
  no-op; "condition" stays a word for where an expression is used.
- **R11 — Schema format.** `DARKMATTER_EXPRESSION_FORMAT` validation
  (simplified-schema/lib/src/format.rs:287–298) and the `default(...)`
  expression check (simplified/convert.rs:2228) call `parse`. The
  `expression(condition)` form stays rejected as an unknown constraint (no
  grammar change; test `expression_parameterized_rejected_in_v1` in
  `darkmatter/lib/tests/l1/schemas_literal_expression.rs:446–458` keeps its
  assertion); only the "reserved for a future backward-compatible opt-in"
  explanations in docs are dropped, per spec D2.
- **R12 — Claudine CLI mode report removed.** `render_expressions_modes`
  (claudine/cli/src/commands/context/expressions.rs:233–283) and its call
  (claudine/cli/src/commands/context/mod.rs:271) are deleted with
  `MODE_DESCRIPTORS` (R7); no other CLI surface renders it (grep over
  `darkmatter/cli` and `claudine/cli`). The removal follows from D2's "prose
  that says 'condition mode' is rewritten" — a report of a mode split that no
  longer exists is not rewritten but removed.
- **R13 — D3 needs no new absence code.** Conditions get interpolation's
  absence rule (left operand is an absence check; right operand inherits
  false-side facts) automatically once they parse to `LogicalOr`, because the
  evaluator arm (expression/mod.rs:662–675), `walk_static`
  (absence.rs:533–537), and `branch_scope` (absence.rs:421–424) already apply
  it to the node; `or(a, b)` call operands keep ordinary reads. AC5 is covered
  by a new test in `darkmatter/lib/tests/l1/unknown_identifier_warning.rs`
  (the file that owns the advisory today).
- **R14 — Safety search outcomes.** Phase 1 re-runs the spec's search for a
  condition (`when`/`while`/`until`, Markdown/YAML/Rust, excluding
  `_completed/` and frozen fixtures) whose `||` result is consumed as a value
  (a function argument or comparison operand). If none is found: proceed. If
  one is found: rewrite that condition in the same change so `||` is a truth
  test again (the 2026-10-16 precedent rewrote `length(x || []) > 0` to
  `!is_empty(x)`), record the edit in the implementation log, and proceed.
  Never stop and never leave the condition unchanged. A planning-time run on
  2026-10-30 found zero value-consumed `||` in conditions (82 condition lines
  containing `||`; the three Rust hits are test fixtures using `||` as the
  outermost truth test).
- **R15 — Unchanged neighbors.** `evaluate_condition` and `ConditionError`
  keep their names (spec D2: "condition" remains a word for where an
  expression is used); `or(...)`/`and(...)` keep returning booleans; `&&`
  stays boolean and lowered to `and(...)` (spec D4); helper type leniency is
  untouched (spec D5); Claudine hook-action `when`
  (`claudine/lib/src/dispatch/runner/mod.rs:111`) only swaps
  `parse_condition` → `parse` — its skip-with-warning behavior is unchanged
  (spec Out of Scope); claudine's local `parse_condition`/
  `parse_condition_expression` helpers in
  `claudine/lib/src/composition/looping/{config.rs:520, dsl.rs:225}` parse
  loop maps, not expressions, and are not renamed.
- **R16 — Validation commands.** Per area: `just test` and `just lint` in
  `simplified-schema/`, `darkmatter/`, `claudine/`. Package-scoped mid-plan
  checks use the area recipe's own package lines:
  `cargo nextest run -p simplified-schema --features work-counters,terminal`,
  `cargo nextest run -p darkmatter --features effects-instrumentation,work-counters`,
  `cargo nextest run -p dmls --features effects-instrumentation`,
  `cargo nextest run -p claudine`. Prompt corpora:
  `just test-prompts` in `claudine/` and
  `cargo nextest run -p darkmatter --features prompt-tests --test prompts`.
  AC4's every-feature build is `cargo check --workspace --all-features
  --all-targets`. Mid-plan, downstream crates are expected not to compile
  until their phase completes; each phase's validation command is scoped to
  crates that phase has finished.

Tasks:

- [x] Safety search re-run
    - Re-run the R14 search over the repository (Markdown, YAML, Rust;
      exclude `_completed/`, `frozen`, and this fix's own directory) for
      `when`/`while`/`until` conditions containing `||`, and inspect each hit
      for a `||` whose result is consumed as a value (e.g.
      `length(x || [])`, `(a || b) == true`).
    - Apply the R14 outcome actions and record the result (found-count and
      any rewrite) in `darkmatter/fixes/2026-10-16-unified-logical-or/` as an
      implementation-log entry.
- [x] Green baseline capture
    - Run, before any code change: `cargo nextest run -p darkmatter --features
      prompt-tests --test prompts` and `just test-prompts` in `claudine/`
      (the passive shipped-artifact corpus named in AC3), confirming both are
      green so any later failure is attributable to this change.

**Wave 1** — the two tasks above run concurrently (read-only; no file edits).

## Phase 2 — simplified-schema: One Grammar, One Node

Everything in this phase is inside `simplified-schema/lib`. After Wave 2 the
crate itself is green; `darkmatter` (and therefore dmls/claudine) will not
compile until Phases 3–4 — expected, per R16.

Tasks:

- [x] Core grammar unification
    - Files: `simplified-schema/lib/src/expression/{ast.rs, lexer.rs,
      parser.rs, mod.rs}` (one owner; their tests are in-file).
    - ast.rs: rename per R1; update module docs and the in-file `Display`
      test; `Display` keeps `a || b`.
    - lexer.rs: delete `ParseMode`, `Token::OrOr`, `Lexer::with_mode`,
      `Lexer::mode`, and the mode field; `||` always emits `Token::Pipe`; the
      bare-`|` error is worded once per R3; `lex_spanned(input)` per R4;
      rewrite in-file tests (merge the two bare-pipe tests; make the
      condition-mode lexer tests modeless; drop the mode argument from
      `lex_spanned` call sites in tests).
    - parser.rs: delete `Parser::with_mode`, `parse_condition`,
      `parse_condition_spanned`, and the `mode` field; reshape the grammar per
      R5; update the grammar doc block (one `logical_or` rule, no mode
      annotations), the precedence doc list ("Logical OR — `||`"), the
      doctests (`parse(r#"foo || "default""#)` matches
      `Expr::LogicalOr { .. }`), `PRECEDENCE_TABLE` per R6, and every in-file
      test: `Fallback` matches become `LogicalOr { left, right }`; tests that
      parsed via `parse_condition`/`parse_condition_spanned` use
      `parse`/`parse_spanned`; a test that asserted infix `||` lowers to
      `FunctionCall { name: "or" }` is rewritten to assert the `LogicalOr`
      node (AC3: rewritten, never deleted).
    - mod.rs: module docs drop the `ParseMode` phrase; re-export lists drop
      `ParseMode`, `parse_condition`, `parse_condition_spanned` (R4).
- [x] Schema format and crate tests
    - Files: `simplified-schema/lib/src/format.rs`,
      `simplified-schema/lib/src/simplified/convert.rs`,
      `simplified-schema/lib/tests/l1/expression_syntax.rs`.
    - format.rs and convert.rs per R11; drop the "condition mode accepts a
      parse superset" sentence from `DARKMATTER_EXPRESSION_FORMAT`'s doc
      (format.rs:130–136) — state that the value must parse under the
      Darkmatter expression grammar, full stop.
    - expression_syntax.rs: drop the `ParseMode`/`parse_condition` imports;
      the `lex_spanned("a || b")` two-mode comparison collapses to one
      assertion (tokens are `Variable, Pipe, Variable`); replace the
      two-mode table test with one asserting `parse("a || b")` yields
      `Expr::LogicalOr` and `parse("a && b")` lowers to `and(a, b)` (D4).
- [x] Phase 2 validation
    - `cargo nextest run -p simplified-schema --features work-counters,terminal`
      and `just lint` in `simplified-schema/` (R16). Do not run workspace-wide
      checks yet.

**Wave 2** — the two edit tasks run concurrently (disjoint files); the
validation task runs after both complete.

## Phase 3 — Darkmatter Library Migration

All tasks are mechanical applications of Phase 2's API to
`darkmatter/lib`. File ownership is disjoint per task.

Tasks:

- [x] Evaluator and analysis arms
    - Files (under `darkmatter/lib/src/markdown/compose/`):
      `expression/mod.rs`, `expression/absence.rs`, `expression/call_check.rs`,
      `expression/scope/facts.rs`, `expression/lexer.rs`,
      `frontmatter_interpolation.rs`, `interpolation/evaluator.rs`,
      `interpolation/mod.rs`, `preflight/collect.rs`, `remote.rs`.
    - Rename every `Fallback` pattern to `LogicalOr { left, right }` at the
      sites listed in R1; keep the evaluator arm's chain-handling comment
      (mod.rs:666–670) and `facts.rs`'s "truthy exactly when `or(a, b)` is"
      comment (both remain true).
    - `expression/mod.rs`: rewrite the "Parser Modes" module-doc section
      (lines 41–45) into one sentence — `||` is logical OR returning the
      deciding operand on every surface; the operator list entry "Logical OR /
      Fallback" becomes "Logical OR" (line 37); the `pub use lexer::{...}` and
      `pub use parser::{...}` re-export lists (158–166) drop `ParseMode`,
      `parse_condition`, `parse_condition_spanned`; `expression/lexer.rs`
      re-export (line 36) drops `ParseMode`; `interpolation/mod.rs` imports
      (78–79) drop `ParseMode` and `parse_condition`.
- [x] Semantics catalog
    - Files: `markdown/compose/expression/semantics.rs`,
      `darkmatter/lib/src/catalog/mod.rs`.
    - Apply R6 (descriptor names) and R7 (delete `ModeDescriptor`,
      `MODE_DESCRIPTORS`, `mode_descriptors()`, the mode-examples test, and
      the `catalog/mod.rs:465` check line).
- [x] Lint rework
    - File: `markdown/compose/expression/lint.rs`.
    - Apply R9: modeless `lint_expression`, single-reparse `lint_spanned`,
      modeless proof in `rewrite`; update the doctest; rewrite the tests named
      in R9 plus `fires_in_both_parse_modes` (becomes a single-mode count
      assertion) and the `authority` proptest (drop the
      `condition: any::<bool>()` mode parameterization and the
      `ParseMode::Condition` branches at 942–946, 974).
- [x] Prepared values
    - File: `markdown/compose/expression/prepared.rs`.
    - Apply R8: `AuthoredMode::Expression` unit variant; delete
      `parse_in_mode`; the `Expression` arm and `parse_occurrences` (line 252)
      call `parse_spanned`; update doc examples and the in-file test at 981.
- [x] Condition-surface call swaps
    - Files: `markdown/compose/{conditions.rs, call_precheck.rs,
      page_blocks/guards.rs, collection_blocks/parser.rs,
      collection_blocks/scopes.rs, frontmatter_shell_expansion.rs,
      expression/call_binding/translate.rs}`,
      `lifecycle/{parse.rs, validate.rs, mod.rs, action_shape.rs}`,
      `frontmatter_shell_expansion/tests/execution_tests.rs`.
    - Swap `parse_condition(` → `parse(` and `parse_condition_spanned(` →
      `parse_spanned(` at: conditions.rs:19,160,199,294 (+ in-file tests
      445–1074); call_precheck.rs:609; guards.rs:154; collection_blocks
      parser.rs:258 and scopes.rs:198; frontmatter_shell_expansion.rs:1812;
      call_binding/translate.rs:1072; lifecycle parse.rs:874, mod.rs:63.
    - `collection_blocks/parser.rs:244`: `lex_spanned(token,
      ParseMode::Interpolation)` → `lex_spanned(token)`; drop the
      `ParseMode` import (line 31).
    - lifecycle/validate.rs: collapse the mode selection (lines 98–108) to
      `lint_expression(&source)`; `AuthoredMode::Expression(ParseMode::
      Condition)` → `AuthoredMode::Expression` (line 180); drop `ParseMode`
      from the test imports (616–617).
    - Fix the comment at execution_tests.rs:814 and the `parse_condition`
      mention in action_shape.rs:375 to say `parse`.
- [x] Library test files
    - Files: `darkmatter/lib/tests/l1/{binding_contract.rs,
      compose_expression_failure_contract.rs, expression_regression.rs,
      simplified_schema_reexports.rs, schemas_literal_expression.rs}`,
      `darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs`.
    - binding_contract.rs: `AuthoredMode::Expression(...)` →
      `AuthoredMode::Expression` (599–642); drop the `ParseMode` import.
    - compose_expression_failure_contract.rs: `parse_condition` → `parse`
      (16, 507–508); assertions unchanged (failures stay failures).
    - expression_regression.rs: `Fallback` matches → `LogicalOr`.
    - simplified_schema_reexports.rs: line 57 uses `parse`; delete the
      `ParseMode` re-export assertion (62).
    - schemas_literal_expression.rs: `parse_condition` → `parse` (6, 605);
      `expression_accepts_either_dialect` (474–479) is renamed to
      `expression_accepts_logical_operators` — its assertion is unchanged
      (`&&`/`||` values parse); the parameterized-form test keeps asserting
      rejection (R11).
    - dasherized_identifier_corpus.rs: the `Surface::mode()` mapping (61–71)
      is deleted; `dashed_variables` calls `lex_spanned(text)` (313–314); the
      parse dispatch (326–327) collapses to `parse(&expression.text)`;
      mode-parameterized test rows (415) drop the mode.
- [x] Phase 3 validation
    - `cargo nextest run -p darkmatter --features
      effects-instrumentation,work-counters`, then
      `cargo nextest run -p darkmatter --features prompt-tests --test prompts`
      (R16). dmls and claudine are still red — do not build them.

**Wave 3** — the six edit tasks run concurrently (disjoint file ownership);
the validation task runs after all six complete.

## Phase 4 — DMLS and Claudine Migration

Tasks:

- [ ] DMLS migration
    - Files: `dmls/src/overlay/expressions.rs` (+ its
      `frontmatter_inventory_tests.rs`), `dmls/src/overlay/event_views.rs`,
      `dmls/src/diagnostics/{frontmatter.rs, nested_span.rs}` (+
      `nested_span/nested_span_tests.rs`), `dmls/src/providers/{frontmatter.rs,
      dsl.rs, semantic_tokens.rs}`, `dmls/tests/l1/expression_scopes.rs`.
    - Apply R10: delete `expressions::parse_condition`,
      `hover_markdown_condition`, the `Dialect` enum and `dialect` field, and
      the mode parameters of `hover_markdown_in_event`/`nested_span_lints`;
      callers use `expressions::parse` / `hover_markdown`.
    - Rewrite `parse`'s doc (overlay/expressions.rs:431–443) to drop the
      dialect comparison; `function_call_at`'s comment about synthetic
      `and`/`or` calls (482–485) stays (`&&` still lowers).
    - Tests: `parse_condition("a || b")` display assertions (2634–2642)
      become `parse("a || b")` rendering `a || b`; the hover test pair
      (2651–2659, 2726–2742) collapses to `hover_markdown`; the
      frontmatter-provider test `condition_dialect_expression_value_hover_
      uses_condition_grammar` (3319–3331) is renamed
      `expression_value_hover_parses_logical_or` and asserts the hover shows
      `a || b` (not `or(a, b)`); event_views dialect assertions (404, 449,
      474) drop the `Dialect` and keep the discovered-text assertions;
      `expression_scopes.rs` (276, 676) swaps `parse_condition` → `parse`;
      `nested_span.rs:180` and the inventory tests drop the mode argument;
      comment fixes at diagnostics/frontmatter.rs:1902 and
      providers/frontmatter.rs:3320.
- [ ] Claudine dispatch, benches, CLI
    - Files: `claudine/lib/src/dispatch/{matcher.rs, template.rs,
      runner/mod.rs, expression/tests.rs, expression/tests/hook_leaf_reads.rs}`,
      `claudine/lib/benches/{pipeline_work.rs, pipeline_work_alloc.rs}`,
      `claudine/lib/src/composition/error/lifecycle_authoring/tests.rs`,
      `claudine/cli/tests/prompts/shipped_prompt_contract.rs`,
      `claudine/cli/src/commands/context/{expressions.rs, mod.rs}`.
    - `Parser::with_mode(trimmed, ParseMode::Condition/Interpolation)` →
      `Parser::new(trimmed)` (matcher.rs:67, template.rs:477);
      `parse_condition` → `parse` (runner/mod.rs:111; benches at
      pipeline_work.rs:179,317 and pipeline_work_alloc.rs:265,385;
      lifecycle_authoring/tests.rs:14,315; shipped_prompt_contract.rs:70,112,
      144 — the AC3 corpus test).
    - dispatch/expression/tests.rs: the `parse_and_evaluate(source, mode, ..)`
      harness (17–24) drops the mode; all call sites (247–686) drop the
      `ParseMode::*` argument; hook_leaf_reads.rs likewise (150–177).
    - Apply R12: delete `render_expressions_modes` and its call, and the
      `mode_descriptors` import.
    - matcher.rs:137's `Expr::Fallback` arm → `LogicalOr`.
- [ ] Claudine composition
    - Files: `claudine/lib/src/composition/lifecycle/{bindings.rs,
      bindings/rename.rs, bindings/tests.rs, binding_checks.rs, executor.rs,
      tests/nested_span.rs, prewarm/classify.rs}`,
      `composition/looping/{config.rs, expression.rs}`,
      `composition/sequence/{grammar.rs, expr.rs, preflight/mod.rs}`,
      `composition/prepare/tests.rs`.
    - `parse_condition` → `parse`: bindings.rs:667; rename.rs:138,147;
      bindings/tests.rs:393,430; executor.rs:289; looping/config.rs:423
      (fully-qualified form); looping/expression.rs:306;
      sequence/grammar.rs:486; sequence/preflight/mod.rs:2643.
    - binding_checks.rs: `AuthoredMode::Expression(ParseMode::Condition)` →
      `AuthoredMode::Expression` (58); drop the `ParseMode` import (15).
    - sequence/expr.rs: `AuthoredMode::Expression(ParseMode::Interpolation)`
      → `AuthoredMode::Expression` (190); drop the `ParseMode` import (24).
    - tests/nested_span.rs: `lint_expression(&inner_expression(say),
      ParseMode::Interpolation)` → `lint_expression(&inner_expression(say))`
      (64); drop the import.
    - prewarm/classify.rs: `SpannedExprKind::Fallback` arms (252, 338) →
      `LogicalOr { left, right }`; prepare/tests.rs:579 likewise.
- [ ] Phase 4 validation
    - `cargo nextest run -p dmls --features effects-instrumentation`,
      `cargo nextest run -p claudine`, then `just test-prompts` in `claudine/`
      (the AC3 corpus, green again once this wave's edits complete).

**Wave 4** — the three edit tasks run concurrently (dmls, claudine
dispatch/benches/CLI, claudine composition are file-disjoint); the validation
task runs after all three complete.

## Phase 5 — Acceptance Tests and Corpora

New tests only; every production change has landed.

Tasks:

- [ ] Cross-surface value parity (AC1)
    - File: `darkmatter/lib/tests/l1/expression_regression.rs` (extend).
    - Evaluator rows (via `evaluate(&parse(..), ..)` against a fixture state,
      following the file's existing helpers): the seven spec rows —
      `items || []` with `items: [1,2]` and absent; `length(items || []) > 0`
      with `items: ["q"]` and absent; `a || b` with `a:"x", b:true` → `"x"`;
      with `a:false, b:0` → `0`; and `true || unknown_fn()` → `true` with
      `unknown_fn` never called (an evaluation would be an
      `UnknownFunction` error, so assert `Ok(true)`).
    - Surface rows in the same file using `Markdown::compose_with` (the
      pattern from lint.rs's `a_body_keeps_the_nested_span_the_lint_describes_
      literal`): one `::block when="length(items || []) > 0"` document whose
      body renders with `items: ["q"]` and is dropped when `items` is absent;
      one interpolation `{{ a || b }}` rendering `x`. Claudine lifecycle `when:`
      truth-value rows are covered by the AC2 test below (the third surface).
- [ ] Claudine lifecycle guard (AC2)
    - File: `claudine/lib/src/composition/lifecycle/executor/tests/
      conditions_control.rs` (extend; the `when_guarded_fallback_*` tests are
      the pattern).
    - A lifecycle `success` stack with
      `when: "length(frontmatter(plan, 'open_questions') || []) > 0"` and a
      `message` action: with a plan file whose frontmatter lists one open
      question the action runs; with the property absent it does not; no
      error in either case.
- [ ] Absence advisory in conditions (AC5)
    - File: `darkmatter/lib/tests/l1/unknown_identifier_warning.rs` (extend).
    - Compose a document with a `::block when="spce || plan"` (both roots
      undeclared): no `dm.expression.undeclared_property` advisory is raised
      for the left operand; a companion `when="plan || spce"` still raises it
      for the right operand. Schema-declared names stay validated (assert a
      declared-root document stays advisory-free in both positions).
- [ ] Phase 5 validation
    - Run the three new/extended test files via the Phase 3/4 package
      commands (R16) and confirm every AC1–AC6 assertion passes: AC1/AC5 in
      darkmatter, AC2 in claudine, AC3 across the rewritten tests of Phases
      2–4 plus the corpora, AC4 deferred to Phase 7, AC6 in the Phase 3 lint
      rewrite.

**Wave 5** — the three test tasks run concurrently (disjoint files); the
validation task runs after they complete.

## Phase 6 — Documentation and Skills

Describe the result, not the change (spec: Documentation). Every page below
was read at planning time and its mode-dependent lines are identified.

Tasks:

- [ ] Darkmatter docs
    - `darkmatter/docs/topics/darkmatter-expressions.md`: replace the
      "Interpolation vs. Condition Mode" section (824–841) with a short
      "Logical OR" statement carrying the spec's Goal table and one `x || []`
      condition example; fix the API bullet (63) to drop `parse_condition`;
      rewrite the parsing-summary bullet (96–98, "Mode changes exactly one
      thing" — also correct its claim that interpolation lowers `||` into
      `or(…)`); operator list entry (219) becomes "**Logical OR** — `||`";
      the truthiness paragraph (248–249) keeps `a || b` semantics but stops
      calling it "a true fallback operator" in mode terms (it may keep
      "fallback" as the idiom's name); state that `or(…)`/`and(…)` and `&&`
      return booleans and why `&&` differs (D4).
    - `darkmatter/docs/topics/schemas/parsing/index.md`: remove the two-mode
      table (56–62) and the mode grammar note; keep the `ConditionError`
      sentence (82) reworded to "the public condition API".
    - `darkmatter/docs/topics/schemas/parsing/grammar.md`: remove the mode
      branches (23–27, 54–66, 70, 142); rename the `fallback` rule to
      `logical_or`; the AST note (80) becomes "infix `||` builds the
      `LogicalOr` node; `&&` lowers to `and(…)`".
    - `darkmatter/docs/topics/schemas/parsing/lexing.md`: `||` row (143)
      becomes `Pipe` unconditionally; the lone-`|` error (151) carries the one
      wording; delete the `ParseMode` paragraph (163–164).
    - `darkmatter/docs/topics/schemas/schema-targeting.md`: remove parse-mode
      grammar references (found by grep for "mode"/"dialect" in the page).
    - `darkmatter/docs/topics/schemas/definition.md`: rewrite the
      "two dialects" semantics paragraph (717) to one sentence — a bare
      `expression` validates when the string parses under the (single)
      Darkmatter expression grammar; drop the "reserved" phrasing from the
      limitations (713 and 4472), keeping that parameterized forms are
      rejected.
- [ ] simplified-schema docs
    - `simplified-schema/docs/topics/expression-syntax.md`: remove the
      `parse_condition`/`parse_condition_spanned` rows and the `ParseMode`
      note (43–46, 55–63); replace the two-mode comparison table with the
      Logical OR statement; `lex_spanned` row loses "(either (`ParseMode`))";
      drop the "condition mode accepts a superset" explanation (62–63).
- [ ] Claudine docs
    - `claudine/docs/topics/state-management/expression-engine.md`: replace
      "Interpolation vs. condition mode" (114–125) with the unified Logical OR
      statement (both `{{ }}` and `when` examples return the deciding
      operand); precedence row 7 (152) becomes "Logical OR: `||`"; the
      implementation reference (222–223) drops `parse_condition()` and says
      one parser serves every surface.
    - `claudine/docs/topics/flow-control/lifecycle.md`: drop "(in the
      condition dialect)" from the nested-span validation sentence (1710).
- [ ] Skills
    - `.opencode/skill/darkmatter/expressions.md`: line 196–197's "`and`/
      `or`/`||` (lowered to `FunctionCall`/`Fallback`, matched through the
      registry)" becomes "`&&` lowers to an `and(...)` `FunctionCall`; `||`
      is the `LogicalOr` node; both matched through the registry"; sweep the
      file for any remaining mode-split phrasing.
    - `.opencode/skill/claudine/architecture.md`: the diagram line (362)
      `parse_condition(expr)` → `parse(expr)`.
    - `.opencode/skill/darkmatter/compose.md` needs no change: its
      "Fallback Expressions" section (722) shows interpolation-only examples
      whose description stays true on every surface.

**Wave 6** — the four tasks run concurrently (disjoint files).

## Phase 7 — Workspace Validation and Review Package

Tasks (sequential):

- [ ] Every-feature workspace build (AC4)
    - `cargo check --workspace --all-features --all-targets` — proves
      `ParseMode`, `Token::OrOr`, `parse_condition`, and
      `parse_condition_spanned` no longer exist anywhere (a stray reference is
      a compile error).
- [ ] Leftover sweep
    - `rg -n "ParseMode|parse_condition_spanned|parse_condition\b|Token::OrOr|Expr::Fallback|SpannedExprKind::Fallback|Lexer::with_mode|Parser::with_mode|MODE_DESCRIPTORS|hover_markdown_condition" simplified-schema/ darkmatter/ claudine/ --type rust`
      — expect zero hits, with exactly one permitted exclusion: the
      definition and calls of claudine's local loop-map helper
      `fn parse_condition(` in `composition/looping/config.rs:520` (R15);
      every other hit is a defect to fix before Phase 7 ends.
    - `rg -n "condition mode|condition dialect|Interpolation vs\. Condition|Logical OR / Fallback" darkmatter/docs claudine/docs simplified-schema/docs .opencode/skill`
      — expect zero hits.
- [ ] Area suites
    - `just test` and `just lint` in `simplified-schema/`, `darkmatter/`
      (covers lib, cli, dmls, zed-dmls-cli), and `claudine/`; then
      `just test-prompts` in `claudine/` and
      `cargo nextest run -p darkmatter --features prompt-tests --test prompts`
      one final time.
- [ ] Acceptance-criteria roll-up and review package
    - Verify each spec acceptance criterion 1–6 against its named test from
      Phases 2–5 and record the mapping in the implementation log.
    - Re-run the R14 safety search a final time (the spec requires it
      "before it lands"); apply R14 outcomes.
    - Write the implementation log (spec departures: none expected; any
      discovered drift is fixed in the same change per repo policy), update
      this plan's checkboxes, and leave the tree at "implementation complete,
      ready for review" — no commits, no `just complete`, no lifecycle moves
      (repo policy: the author closes a fix).