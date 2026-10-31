---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/fixes/2026-10-16-unified-logical-or/spec.md"
plan: "darkmatter/fixes/2026-10-16-unified-logical-or/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - simplified-schema
    - darkmatter
    - dmls
    - claudine
    - claudine-cli
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
decisions_for_review:
    - |-
        The plan's Phase 1 baseline task expects Claudine's prompt corpus
        (`just test-prompts` in claudine/) to be green before any code change.
        It is not: 6 of 87 tests fail on the unmodified tree, and none of the
        failures involve `||`. Three `review_fix_rehearsal` tests expect a
        "follow-up pass" that the live review prompts no longer run, and three
        `shipped_prompt_route_drift` tests report that the live
        `prompts/_implement/` has moved ahead of its frozen fixture (new
        `depends_on` checks, a swapped `phase`/`start_phase` fallback order,
        and two new files). I recorded these six failures by name as the
        baseline and did not change the prompts or the fixture. Fixing them is
        prompt-maintenance work outside this fix, and the repository treats
        `prompts/` as draft space that only opt-in tests read. The other
        options were to refresh the fixture from the live prompts, which would
        hide drift the test exists to report, or to fix the rehearsal tests,
        which needs a decision on how the review prompt should behave. If the
        reviewer wants the corpus green, that is a separate change; nothing in
        this fix depends on it. Later phases should treat only failures beyond
        these six as caused by this change.
    - |-
        Phase 2 could not meet the prompt's completion gate of a green
        `just test` and `just lint` in darkmatter/. The plan deliberately
        removes the old `||` API from the simplified-schema library first
        (Phase 2) and migrates Darkmatter, the language server, and Claudine
        afterwards (Phases 3 and 4); its validation rule R16 says downstream
        crates will not compile in between. After this phase darkmatter fails
        to compile only on the renamed `Fallback` node and the deleted
        `ParseMode`, `parse_condition`, `parse_condition_spanned`, and the
        mode argument of `lex_spanned` (25 lib errors, all of those kinds). I
        followed the plan and ran the gates of the crate this phase finished
        (simplified-schema `just test` and `just lint`, both green) instead.
        The alternative, keeping temporary compatibility shims (a deprecated
        `Fallback` alias or a no-op `parse_condition`) so darkmatter compiles
        mid-plan, would add code that Phases 3-4 then delete, and the spec
        says the mode is removed, not left as a no-op. Nothing needs redoing
        if the reviewer accepts this; Phase 3 brings darkmatter back to green.
    - |-
        Phase 3's prompt asks for a green `just test` and `just lint` in
        darkmatter/, but that recipe also builds the language server (dmls),
        which the plan migrates in Phase 4 and which still refers to the
        removed condition-mode API (10 compile errors, all renames or
        deletions listed in the plan's R10). I followed the plan and ran the
        same gates scoped to the two crates this phase finished: the
        darkmatter library (nextest 9720/9720, doctests 209 passed, prompts
        corpus 10/10, clippy clean) and darkmatter-cli (nextest 1460/1460,
        clippy clean). The alternative was to pull the dmls migration forward
        into this phase so the whole area is green, which would empty Phase 4's
        DMLS task and blur the plan's per-phase ownership. Nothing needs
        redoing if the reviewer accepts this; Phase 4 makes the full area
        recipe green.
    - |-
        The plan did not mention `value_dialect_is_condition_parse_superset` in
        `darkmatter/lib/tests/l1/schemas_literal_expression.rs`. With one
        grammar its loop parsed each string twice with the same function, so
        it no longer tested anything. I replaced it with
        `expression_format_accepts_every_value_expression`, which runs the same
        corpus through schema validation of an `expression`-typed property (the
        public path the old test was protecting). Deleting it outright was the
        other option; restoring the old test is not possible because the
        second parser no longer exists.
    - |-
        I renamed in-file unit tests whose names described the removed mode
        split: `interpolation_fallback_versus_condition_or` became
        `logical_or_returns_operand_while_or_function_returns_bool`,
        `interpolation_fallback_uses_fallback_when_falsy` became
        `logical_or_returns_right_operand_when_left_is_falsy` (expression/mod.rs),
        `fires_in_both_parse_modes` became `fires_once_beside_logical_or`,
        `fallback_span_in_condition_dialect_has_no_suggestion` became
        `logical_or_span_is_offered_a_suggestion`, and
        `suggestion_reparses_in_the_original_dialect` became
        `suggestion_beside_logical_or_reparses` (lint.rs). Assertions keep their
        intent except the one the spec reverses (AC6: the nested `||` span is
        now offered a suggestion). Keeping the old names would leave tests
        named after behavior that no longer exists.
    - |-
        Phase 4 removed Claudine's `context --expressions` mode section, as the
        plan's R12 said, but the plan did not list the tests built on it. One
        L1 test asserted the section's intro sentence, and two L2 terminal
        tests used its bullet list, the only list in that report, to prove
        list layout (one blank line on each side, hanging indent, a 1-column
        right margin). I rewrote the L1 test to assert the mode section is gone
        while the precedence table still lists `||`. For L2, I moved the
        blank-line and hanging-indent checks onto the side-effects report's
        constraint list, which already had its own right-margin test, and
        deleted the expressions right-margin test, which would otherwise
        duplicate that one. The alternative was to keep a short `||` list in
        the expressions report just so those tests had something to measure,
        but that is output the spec does not ask for. If the reviewer wants
        list coverage on the expressions report itself, that report needs a
        list again first, and the two tests can then be restored there.
    - |-
        DMLS now flags an undeclared name on the right of `||` in an
        Expression-typed frontmatter value (`f4: known || fm_rhs`), as the
        body `{{ }}` form already did. This follows from the spec's decision
        that one absence rule applies everywhere. The LSP test
        `undeclared_property_fires_in_every_operand_position_at_warning_severity`
        used to expect no warning there (its comment explained the old
        condition-mode rule); it now expects the warning. The hover golden
        `a3_lifecycle_value_hovers.json` was regenerated: a lifecycle guard
        now hovers as `!total_phases || total_phases <= 0` (as written)
        instead of `or(!total_phases, total_phases <= 0)`. Both are the
        intended result; nothing needs redoing unless the absence decision
        itself is reversed.
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
source_files_during_phase_4:
    - claudine/cli/src/commands/context/expressions.rs
    - claudine/cli/src/commands/context/mod.rs
    - claudine/cli/tests/l1/context_command.rs
    - claudine/cli/tests/level2/level2_context_capture.rs
    - claudine/cli/tests/prompts/shipped_prompt_contract.rs
    - claudine/lib/benches/pipeline_work.rs
    - claudine/lib/benches/pipeline_work_alloc.rs
    - claudine/lib/src/composition/error/lifecycle_authoring/tests.rs
    - claudine/lib/src/composition/lifecycle/binding_checks.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/bindings/rename.rs
    - claudine/lib/src/composition/lifecycle/bindings/tests.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/prewarm/classify.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/looping/config.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/sequence/expr.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/dispatch/expression.rs
    - claudine/lib/src/dispatch/expression/tests.rs
    - claudine/lib/src/dispatch/expression/tests/hook_leaf_reads.rs
    - claudine/lib/src/dispatch/matcher.rs
    - claudine/lib/src/dispatch/runner/mod.rs
    - claudine/lib/src/dispatch/template.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/nested_span.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/overlay/event_views.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/expressions/frontmatter_inventory_tests.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/semantic_tokens.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_value_hovers.json
    - darkmatter/dmls/tests/l1/expression_scopes.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
message_to_agent: |-
    Phase 5 added tests only (AC1, AC2, AC5); no production code changed.
    darkmatter `just test` 12884/12884, darkmatter and claudine `just lint`
    clean. The Phase 6 doc and skill list is unchanged from Phase 4's note:
    darkmatter-expressions.md, schemas/definition.md,
    schemas/parsing/{index,grammar,lexing}.md, schema-targeting.md,
    simplified-schema expression-syntax.md, claudine expression-engine.md and
    lifecycle.md, the darkmatter skill's expressions.md (names `Fallback`),
    and the claudine skill's architecture.md line 362, plus the semantics.rs
    operator descriptor and parser.rs precedence entry the spec lists. The
    new tests can serve as doc examples: `length(items || []) > 0` in
    darkmatter/lib/tests/l1/expression_regression.rs.
---

# Implementation Log for 2026-10-16-unified-logical-or (7 phases)

## Phase 1

- Started 2026-10-30. Phase 1 is read-only: a safety search and a green
  baseline. No source, doc, or skill file was changed.

### Safety search re-run (R14)

- Command (repository root, tracked files):
  `git grep -nE '(when|while|until)[=:"'"'"' ]+.*\|\|' -- '*.md' '*.yaml' '*.yml' '*.rs' ':!**/_completed/**' ':!**/*frozen*/**' ':!darkmatter/fixes/2026-10-16-unified-logical-or/**'`
  gave 141 lines. Keeping only lines with a `when=`/`when:`/`while:`/`until:`
  (and `=`) condition left **99 condition lines** (the planning-time run found
  82; the extra lines are new prompt and test-fixture copies, plus doc
  prose).
- Pattern check for a value-consumed `||` (a `||` inside a function call's
  argument list, or a parenthesized `||` group used as a comparison operand):
  **0 hits**.
- A manual read of all 99 lines (grouped into 58 distinct conditions)
  confirmed that every `||` is a truth test. It is either the outermost
  operator, or it sits inside `&&`/`!`/a parenthesized group whose result is
  only tested for truthiness. Edge case checked:
  `env.CLAUDINE_TEST_MISSING || 'default' == 'default'` (a claudine test)
  parses as `env.X || ('default' == 'default')` because `==` binds tighter,
  so it is still a truth test.
- R14 outcome: **0 found, no rewrite needed.** Some doc and comment lines
  still describe the two-mode split (for example the
  `darkmatter/docs/topics/darkmatter-expressions.md` "Logical OR: returns a
  boolean" row, and the "either dialect" prose). Phase 6 rewrites them.

### Green baseline capture

- `cargo nextest run -p darkmatter --features prompt-tests --test prompts`:
  **10/10 passed** (green).
- `just test-prompts` in `claudine/` (the `claudine-cli` `prompts` binary with
  `prompt-tests,test-fixtures`): the default run stopped after 3 failures, so
  I re-ran it with `--no-fail-fast` to get a complete baseline:
  **87 run, 81 passed, 6 failed.** All 6 fail on the unmodified tree; there
  are no working-tree changes under `prompts/` or `claudine/`:
    - `review_fix_rehearsal::a_rerun_on_a_ready_spec_runs_its_open_follow_up_pass`
    - `review_fix_rehearsal::a_ready_review_gets_one_follow_up_pass_and_no_further_review`
    - `review_fix_rehearsal::a_not_ready_review_is_repaired_and_the_ready_one_gets_its_follow_up_pass`
      — these expect a `repair` follow-up pass on a production-ready review
      that the live review prompts do not run (stub log is `[]`, expected
      `["repair"]`).
    - `shipped_prompt_route_drift::fixture_preserves_the_shipped_schema_and_loop_semantics`
    - `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture`
    - `shipped_prompt_route_drift::fixture_body_matches_the_shipped_body`
      — the live `prompts/_implement/` has drifted from
      `claudine/cli/tests/fixtures/shipped_implement_route/_implement/`: new
      `depends_on` checks, a swapped `phase`/`start_phase` fallback order, and
      new `_fm.md`/`implement-feature.md` files. None of the new conditions
      use `||`.
- Neither group is about `||`; both are prompt-maintenance drift. They are
  recorded as the baseline (see `decisions_for_review`) and left unchanged.

### Requirement-to-test mapping

- Phase 1 changes no behavior, so it adds no tests. The two baseline runs
  above are the evidence for AC3's corpus starting state.

### Gates

- Scope calculator, run from the repository root after every Phase 1 edit:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `packages: [darkmatter, darkmatter-cli, dmls, simplified-schema, test-toolkit]`
  and `change_class: package`. The list is not empty because the working tree
  still holds uncommitted source edits from the earlier
  `2026-10-30-reference-navigation-and-hover` work. Phase 1 itself edited only
  this log and the plan. The gates were run regardless:
    - `just test` (darkmatter): **12874 run, 12874 passed, 10 skipped** (69 s).
    - `just lint` (darkmatter): **pass**, including the zed-dmls
      wasm32-wasip2 check.
- No cross-OS check was needed: Phase 1 changed no code.
- No darkmatter skill update was needed: Phase 1 changed no behavior.

## Phase 2

- Test mapping (changed behavior -> test):
    - `||` parses to one `LogicalOr` node, never `or(...)` (AC3 rewrite, not
      deletion): parser.rs `logical_operators::{parses_infix_or_as_logical_or_node,
      and_binds_tighter_than_or_left, and_binds_tighter_than_or_right,
      parenthesized_or_then_and, or_inside_or, chained_or_left_associative}`
      (rewritten from the old `condition_mode_logic` tests that asserted
      `or(...)`); new `or_function_call_still_parses_as_a_call` pins that the
      named `or(...)` function is unaffected; L1 crate test
      `expression_syntax::or_keeps_its_own_node_and_and_lowers_to_a_call`
      (public API: exact `LogicalOr` tree, `&&` -> `and`, token stream
      `Variable, Pipe, Variable, Eof`).
    - `||` lexes as `Token::Pipe` everywhere: lexer.rs
      `logical_operator_tokens::{double_pipe_is_pipe, mixed_operators_tokens}`,
      `spanned::double_pipe_spans_two_bytes`.
    - One bare-`|` error wording: lexer.rs `lexer_errors::error_bare_pipe`
      (exact message and byte position 4; merges the two old mode tests) and
      parser.rs `error_cases::error_bare_pipe` (exact message through `parse`).
    - Rename: ast.rs `display_logical_or` (Display still `a || b`); spanned
      `erasure_matches_parse` corpus now includes the former condition-only
      rows; all former `Fallback` matches in parser tests use `LogicalOr`.
    - Expression format / `default(...)` check now call `parse`: covered by
      the existing format and convert tests plus `expression_syntax`
      tests (all green).
    - Input Robustness Matrix: not applicable (text grammar), per the plan.
- Removed redundant `parse_condition*` duplicate assertions in callback tests
  (they asserted the same grammar twice now that only one exists).
- Gates (simplified-schema): `just test` -> 1288/1288 nextest + 1112/1112
  minimal-feature run; `cargo test --doc -p simplified-schema` -> 28 passed,
  2 ignored; `just lint` -> clean (three clippy configurations).
- Scope calculator: `git status ... | xargs python3 scripts/ci/affected_scope.py`
  -> packages `[simplified-schema, test-toolkit, darkmatter]`, change_class
  `package`; gates cannot be skipped.
- Darkmatter gates: not runnable this phase. `cargo check -p darkmatter
  --all-targets` fails with only the expected rename/deletion errors (R16);
  see `decisions_for_review`. Dependent-package testing is therefore deferred
  to Phases 3-4, which migrate darkmatter, dmls, and claudine.
- No cross-OS check: the change is pure parsing with no paths, processes, or
  OS-specific code.
- No darkmatter skill update in this phase; skills describing the mode split
  are scheduled in Phase 6.

## Phase 3

- Started 2026-10-30. Applied Phase 2's API to `darkmatter/lib` (plan Phase 3
  tasks; done directly rather than through subagents because every edit was
  a mechanical rename across disjoint files).
- Changes: every `Expr::Fallback`/`SpannedExprKind::Fallback` arm renamed to
  `LogicalOr { left, right }` (evaluator, absence, call_check, facts, lint,
  prepared, frontmatter_interpolation, interpolation evaluator, preflight
  collect, remote); every `parse_condition`/`parse_condition_spanned` call
  now `parse`/`parse_spanned`; `lex_spanned` calls take one argument;
  `AuthoredMode::Expression` is a unit variant and `parse_in_mode` is gone;
  `lint_expression(source)` is modeless, `lint_spanned` re-parses once,
  `rewrite` proves its suggestion with `parse_spanned`; `ModeDescriptor`,
  `MODE_DESCRIPTORS`, `mode_descriptors()`, the mode-examples test, and the
  catalog check line are deleted; operator descriptors renamed per R6/R7.
- Drifted comments fixed (code is authoritative): module docs in
  `expression/mod.rs` ("Parser Modes" section -> one sentence),
  `interpolation/mod.rs` (doctest now matches `LogicalOr`), `absence.rs`
  ("fallback primary" -> "left operand of `||`"), `conditions.rs`,
  `frontmatter_shell_expansion.rs`, `lifecycle/{action_shape,parse}.rs`,
  `execution_tests.rs`, `compose/tests/shell.rs`, `semantics.rs` module doc,
  and the ownership guard's module doc (no longer names DMLS's
  `parse_condition`).
- Condition consumers judged by truthiness: verified that `evaluate_condition`,
  `evaluate_condition_against`, lifecycle `when`, page-block guards, and the
  frontmatter shell ternary all call `is_truthy` on the result, so `||`
  returning its deciding operand cannot change which branch runs. No
  `as_bool()` on a condition result exists in lifecycle, page blocks,
  collection blocks, or conditions.
- Test mapping (changed behavior -> test):
    - `||` evaluates to the deciding operand where conditions used to get a
      boolean: `expression::tests::logical_or_returns_operand_while_or_function_returns_bool`,
      `logical_or_returns_right_operand_when_left_is_falsy`;
      `binding_contract` (condition `label || count > 1` through
      `AuthoredMode::Expression` still evaluates to `true`).
    - AC6 nested-span suggestion for `a == "x {{ y || 'd' }}"`:
      `lint::tests::lint_model::logical_or_span_is_offered_a_suggestion`
      (exact text `a == "x " + (y || 'd')`); `suggestion_beside_logical_or_reparses`;
      `fires_once_beside_logical_or`; the `authority` proptest now runs one
      grammar.
    - Modeless `lex_spanned` re-export yields `Token::Pipe`:
      `simplified_schema_reexports::the_darkmatter_expression_syntax_paths_are_the_library_parser`.
    - `expression` schema format accepts `&&`/`||` and the value corpus through
      the public validation path: `schemas_literal_expression::{expression_accepts_logical_operators,
      expression_format_accepts_every_value_expression}`; parameterized
      `expression(condition)` stays rejected (unchanged test).
    - Shipped-artifact corpus with one grammar: prompts
      `dasherized_identifier_corpus` (10/10).
    - Value parity across surfaces (AC1), the Claudine guard (AC2), and the
      condition absence advisory (AC5) are Phase 5's tests per the plan.
- Gates:
    - `cargo nextest run -p darkmatter --features effects-instrumentation,work-counters`
      -> 9720 passed, 4 skipped.
    - `cargo test -p darkmatter --doc` -> 209 passed, 8 ignored.
    - `cargo nextest run -p darkmatter --features prompt-tests --test prompts`
      -> 10/10.
    - `cargo nextest run -p darkmatter-cli --features test-fixtures` -> 1460/1460.
    - `just _lint darkmatter`, `just _lint darkmatter-cli` -> clean.
    - Full `just test`/`just lint` in darkmatter/ not runnable: dmls does not
      compile until Phase 4 (10 errors, all R10 renames/deletions). See
      `decisions_for_review`.
- Scope calculator: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` -> packages
  `[darkmatter, simplified-schema, test-toolkit, claudine]`, change_class
  `package`; gates not skippable (run as above).
- Dependent packages: dmls and claudine are the dependents and are migrated
  in Phase 4; they are expected red now (R16), so no dependent test run.
- No cross-OS check: pure parsing/renaming, no paths, processes, or OS code.
- No darkmatter skill change this phase: the skill's expression topic pages
  are Phase 6's documentation task.

## Phase 4

- Did all three edit tasks directly (no subagents). They were mostly
  mechanical renames, and doing them in one pass avoided running parallel
  test suites on one host.
- DMLS (R10): deleted `expressions::parse_condition`, `hover_markdown_condition`,
  the `Dialect` enum and the `StackExpression::dialect` field; `hover_markdown_in_event`
  lost its `parser` parameter and `nested_span_lints` (both forms) lost the mode
  argument; `StackExpression::parse` calls `expressions::parse`. Callers in
  `providers/{frontmatter,dsl,semantic_tokens}.rs` and `diagnostics/{frontmatter,nested_span}.rs`
  use `parse`/`hover_markdown`. `parse`'s doc now states one grammar for every
  surface. Three `Fallback` arms in `overlay/expressions.rs` became `LogicalOr`.
  Fixed doc and test names that still said "dialect" (the `findings` `reparse` doc,
  `condition_dialect_*` tests renamed to `logical_and_*`/`logical_or_*`).
- Claudine (R1/R8/R15): `Parser::with_mode(..)` → `Parser::new(..)` (matcher,
  template, dispatch expression tests, whose `parse_and_evaluate` harness lost
  its mode); `parse_condition` → `parse` at every listed site (the local
  `looping::config::parse_condition` helper is unrelated and kept);
  `AuthoredMode::Expression` unit variant; `lint_expression` modeless;
  `Expr`/`SpannedExprKind::Fallback` → `LogicalOr { left, right }` in matcher,
  prewarm/classify, and prepare/tests (renamed
  `lifecycle_logical_or_span_is_deferred_unresolved`). Updated the
  `dispatch/expression.rs` module doc that called `||` "fallback syntax".
- Claudine CLI (R12): deleted `render_expressions_modes`, its call, and the
  `mode_descriptors`/`render_unordered_list` imports. Tests the plan did not list
  depended on that section (see decisions_for_review):
    - `tests/l1/context_command.rs`: `context_expressions_corrected_interpolation_wording`
      → `context_expressions_reports_no_parse_modes` (mode heading, "two modes",
      and "fallback sugar" absent; the precedence table still lists `||`).
    - `tests/level2/level2_context_capture.rs`: the narrow expressions test
      became `level2_context_expressions_narrow_inline_code_in_tmux` (inverse
      inline code, no literal backticks, width); deleted
      `level2_context_expressions_list_reserves_right_margin_in_tmux` because
      `level2_context_side_effects_list_reserves_right_margin_in_tmux` already
      covers it; moved the hanging-indent and one-blank-line checks onto the
      side-effects list.
- Behavior changes exposed by the DMLS suite (both intended by the spec):
    - `frontmatter_response_contracts::a3_lifecycle_value_hovers_are_unchanged`:
      golden regenerated with `DMLS_UPDATE_GOLDEN=1`; the only diff is the guard
      hover `or(!total_phases, total_phases <= 0)` → `!total_phases || total_phases <= 0`.
    - `lsp_session::undeclared_property_fires_in_every_operand_position_at_warning_severity`:
      now also expects `f4: known || fm_rhs` to flag `fm_rhs` (D3: one absence
      rule, right operand is an ordinary read), matching body `B4`.
- Requirement → test mapping (Phase 4 slice):
    - `||` hovers as authored on frontmatter values: `providers::frontmatter::tests::expression_value_hover_parses_logical_or`
      (asserts `is_empty(title) || is_string(title)` shown and no `or(`);
      `overlay::expressions::tests::hover_shows_logical_or_as_authored`; the a3 golden
      on the shipped `implement-plan.md` bench fixture (end-to-end through the LSP).
    - One grammar in DMLS: `overlay::expressions::tests::one_grammar_keeps_logical_or_and_lowers_logical_and`;
      `hover_resolves_a_kebab_name_at_every_offset_beside_logical_or`.
    - Stack-expression discovery unchanged without a dialect: event_views tests
      keep text and reach assertions (the typed guard asserts its text and value span).
    - Condition absence rule (AC5 in DMLS): the `lsp_session` operand test above.
    - Claudine report has no mode section: `context_expressions_reports_no_parse_modes` (L1, spawns the CLI).
    - AC3 shipped-prompt corpus parses with `parse`: `shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions` passes.
- Gates and runs:
    - `cargo nextest run -p dmls --features effects-instrumentation` → 1664/1666
      before the two updates above, then the two tests alone → pass.
    - `cargo nextest run -p claudine` → 6491/6491; with `--features work-counters` → 6544/6544.
    - `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/context/)'` → 111/111.
    - `just _test_l2 claudine-cli --features terminal-tests` → 288/288 (tmux; includes the reworked context tests).
    - `just test-prompts` (claudine) → 81/87; the 6 failures are exactly the
      Phase 1 baseline (3 `review_fix_rehearsal`, 3 `shipped_prompt_route_drift`).
    - `just test` (darkmatter) → 12873/12873, 10 skipped. `just lint` (darkmatter) → clean.
    - `just lint` (claudine) → clean (only the pre-existing `__eh_frame` linker note).
    - `just check-tier-coverage claudine` / `darkmatter` → 0 stranded.
    - `cargo check -p claudine -p claudine-cli --all-targets --all-features` → clean (benches included).
- Scope calculator: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` → packages
  `[claudine, claudine-cli, dmls, test-toolkit, darkmatter]`, change_class `package`;
  gates not skippable (run as above).
- No cross-OS check: renames and deletions only, no path, process, or OS code.
- No darkmatter skill change: the parse-mode mentions in docs and skills
  (`.claude/skills/claudine/architecture.md:362`, darkmatter and claudine
  expression docs) are Phase 6's task.

## Phase 5

- Wrote the three test tasks directly (no subagents): the files were
  disjoint, small, and running them in one pass kept the host unloaded.
- AC1 (cross-surface value parity), `darkmatter/lib/tests/l1/expression_regression.rs`:
    - Evaluator rows via public `parse` + `evaluate` against a `HashMap`
      lookup, module `logical_or_value_parity` (all seven spec rows):
      `a_present_left_operand_is_the_value` (`items || []`, `[1,2]`),
      `an_absent_left_operand_yields_the_right_operand` (`[]`),
      `a_defaulted_collection_feeds_length_in_a_condition` (`true` / `false`),
      `a_truthy_left_operand_is_returned_not_coerced_to_a_boolean` (`"x"`),
      `two_falsy_operands_yield_the_last_operand` (`0`),
      `a_truthy_left_operand_short_circuits_the_right_operand`
      (`Ok(true)`, with a `false || unknown_fn()` control proving the call
      would be an `UnknownFunction` error).
    - Surface rows through `Markdown::compose_with`:
      `regression_logical_or_defaults_a_collection_in_a_page_block_gate`
      (`::block when="length(items || []) > 0"` renders with `items: [q]`,
      dropped when absent), `regression_logical_or_gate_takes_the_truth_of_the_deciding_operand`
      (`a || b` gate dropped for `false`/`0`, shown for `"x"`/`true`),
      `regression_logical_or_interpolates_the_deciding_operand`
      (`{{ a || b }}` → `x`, `{{ c || d }}` → `0`, the two `length(..) > 0`
      rows, and `{{ true || unknown_fn() }}` → `true`).
- AC2 + AC1's third surface, `claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs`:
    - `when_logical_or_defaulted_open_questions_fires_when_listed` /
      `_skips_when_absent`: the exact spec guard
      `length(frontmatter(plan, 'open_questions') || []) > 0` over a real
      `plan.md` in the document's directory (`base_dir`, relative `plan`
      value, no cwd change); the `message` action runs only when one open
      question is listed; the outcome is the default (no error) both times.
    - `when_logical_or_takes_the_truth_of_the_deciding_operand` (added beyond
      the plan so the lifecycle `when:` surface covers every AC1 row, not only
      the AC2 guard): one stack whose seven guards are the table's rows; only
      the four truthy rows emit, in order, and the event does not fail.
- AC5, `darkmatter/lib/tests/l1/unknown_identifier_warning.rs`:
    - `a_page_block_gate_logical_or_handles_only_its_left_operand`:
      `when="spce || plan"` warns for `plan` only (line 4); `when="plan || spce"`
      warns for `spce` only; the block is dropped in both.
    - `a_page_block_gate_logical_or_over_declared_roots_is_silent`: both
      orders with `$schema` declaring `spce` and `plan` raise no advisory.
- AC3 is covered by the rewritten AST tests of Phases 2–4 and the corpora;
  AC4 is Phase 7's workspace build; AC6 by the Phase 3 lint rewrite
  (`a_body_keeps_the_nested_span_the_lint_describes_literal` and siblings),
  all of which are in the darkmatter `just test` run below.
- Tier/placement: all new tests are L1 (no tier markers), in files already
  declared by `darkmatter/lib/tests/l1/main.rs` and Claudine's in-crate test
  module; nextest selected and ran each one (filters below).
- Targeted runs:
    - `cargo nextest run -p darkmatter -E 'binary(l1) & test(/logical_or/)'` → 9/9.
    - `cargo nextest run -p darkmatter -E 'binary(l1) & test(/page_block_gate_logical_or/)'` → 2/2.
    - `cargo nextest run -p claudine -E 'test(/when_logical_or/) | test(/when_unknown_function_fails_closed/)'` → 4/4.
    - `cargo nextest run -p claudine -E 'test(/lifecycle::executor::tests::conditions_control/)'` → 21/21.
- Gates (run once, after all edits, nothing else running):
    - darkmatter `just test` → 12884/12884 passed, 10 skipped (includes dmls; +11 new tests over Phase 4's 12873).
    - darkmatter `just lint` → clean. claudine `just lint` → clean.
    - `just check-tier-coverage darkmatter` / `claudine` → 0 stranded.
- Not run: the full claudine lib suite (only a test module changed; the
  module's 21 tests were run) and `just test-prompts` (no prompt or corpus
  input changed). No cross-OS check: test-only changes; the one file read
  lives in a temp dir and is reached through a relative `plan.md` value
  against `base_dir`, with no OS-specific path.
- No darkmatter skill change: Phase 5 only adds tests.
- Scope calculator (whole uncommitted tree, which still includes Phase 4's
  uncommitted edits): packages `[claudine, claudine-cli, darkmatter, dmls,
  test-toolkit]`, change_class `package`; gates not skippable (run above).
