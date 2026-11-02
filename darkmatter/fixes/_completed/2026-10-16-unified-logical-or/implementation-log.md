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
    - |-
        The plan and spec place the `expression` schema format's "two
        dialects" paragraph and its reserved `expression(condition)` form in
        `darkmatter/docs/topics/schemas/definition.md`. That page no longer
        holds them: the format's reference moved to the SimplifiedSchema
        language docs (`simplified-schema/docs/topics/language/types.md` and
        `language/index.md`). I rewrote the text where it lives now (one
        grammar; a parameterized `expression(...)` is rejected because there
        is nothing to select) and left definition.md unchanged. Likewise
        `schema-targeting.md` had no parse-mode wording, so it is unchanged.
        The alternative, adding the text back to definition.md, would
        duplicate the language reference. Nothing needs redoing unless the
        reviewer wants the format documented in Darkmatter as well.
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
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/state-management/expression-engine.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/schemas/parsing/index.md
    - darkmatter/docs/topics/schemas/parsing/lexing.md
    - simplified-schema/docs/topics/expression-syntax.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/language/types.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/claudine/architecture.md
    - .claude/skills/darkmatter/expressions.md
source_files_during_phase_7: []
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_code:
    - simplified-schema/lib/src/expression/ast.rs
    - simplified-schema/lib/src/expression/lexer.rs
    - simplified-schema/lib/src/expression/parser.rs
    - simplified-schema/lib/src/expression/mod.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/tests/l1/expression_syntax.rs
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
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
documentation:
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/state-management/expression-engine.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/docs/topics/schemas/parsing/index.md
    - darkmatter/docs/topics/schemas/parsing/lexing.md
    - simplified-schema/docs/topics/expression-syntax.md
    - simplified-schema/docs/topics/language/index.md
    - simplified-schema/docs/topics/language/types.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/darkmatter/expressions.md
completed_phase: 7
implemented: true
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

## Phase 6

- Did the work directly, without subagents: eleven small Markdown edits in
  separate files, with nothing gained from running them in parallel.
- Code-side wording the spec's Documentation list names (the `semantics.rs`
  operator descriptor, the `parser.rs` precedence entry, and both `mod.rs`
  module docs) was already updated in Phases 2–3. A grep for
  `Fallback|mode-dependent|ParseMode|parse_condition|dialect` over those
  files returns nothing, so no source file changed in this phase.
- Darkmatter docs:
    - `darkmatter-expressions.md`: the API bullet now names `parse` instead
      of `parse_condition`. The parsing summary ("Mode changes exactly one
      thing", which also wrongly said `||` lowers to `or(…)`) is replaced by
      "One grammar, one meaning". Precedence row 7 is now "Logical OR". The
      truthiness paragraph names the `x || default` idiom and links to the
      new section. "Interpolation vs. Condition Mode" is replaced by
      "Logical OR", which carries the spec's three-row table, a
      `::block when="length(items || []) > 0"` example (taken from the
      Phase 5 regression tests), the absence rule for the left operand, and
      the D4 explanation of why `&&`, `and(…)`, and `or(…)` return booleans.
    - `schemas/parsing/index.md`: the two-mode table is replaced by an
      "One grammar" section, and the condition API sentence drops
      `parse_condition`.
    - `schemas/parsing/grammar.md`: the ladder now has a single
      `logical_or` rule, so the `fallback` rule and the mode branches are
      gone. "Parse modes" is folded into one-grammar text plus the `&&`
      precedence note. "Lowering" now has a `||` vs `&&` table and explains
      why `||` keeps its own node. The stray `Fallback` note is deleted, and
      the DMLS paragraph names `parse_spanned` only.
    - `schemas/parsing/lexing.md`: `||` now maps to `Pipe` only, and the
      lone-`|` error carries its single wording. The "Mode affects exactly
      one token" section is deleted, and `lex_spanned(input)` no longer
      shows a mode argument.
    - `schemas/schema-targeting.md` and `schemas/definition.md`: neither page
      contains mode or dialect wording any more, so both are unchanged (see
      decisions_for_review).
- simplified-schema docs:
    - `expression-syntax.md`: the intro example uses `parse`. The entry-point
      table loses the `parse_condition*` rows and `ParseMode`. "The two
      dialects" is replaced by "Logical OR", with an AST/value table and a
      `LogicalOr` example.
    - `language/types.md` and `language/index.md`: these are the actual home
      of the `expression` format's "two dialects" paragraph and its
      `expression(condition)` reservation. They now say there is one grammar
      and that a parameterized `expression(…)` is rejected.
- Claudine docs:
    - `expression-engine.md`: the "Interpolation vs. condition mode" section
      is replaced by "Logical OR", with an interpolation example and two
      `when:` examples, one of them the `length(x || []) > 0` idiom.
      Precedence row 7 is now "Logical OR: `||`". The evaluation steps say
      that one `parse()` serves every surface.
    - `lifecycle.md`: "(in the condition dialect)" is removed.
- Skills:
    - `.claude/skills/darkmatter/expressions.md`: the `TypeScope::assume`
      bullet now says `&&` lowers to an `and(...)` call and `||` is the
      `LogicalOr` node, treated as `or` in a truth test. No other mode-split
      phrasing remains in the darkmatter skill, and `compose.md` "Fallback
      Expressions" is left as the plan says.
    - `.claude/skills/claudine/architecture.md`: the hook diagram line now
      reads `parse(expr)`.
- Sweep:
    - `rg "condition mode|condition dialect|Interpolation vs\. Condition|Logical OR / Fallback|parse_condition|ParseMode|#parse-modes"`
      over darkmatter/docs, claudine/docs, simplified-schema/docs, and
      .claude/skills returns zero hits.
    - The only remaining hits are in `darkmatter/dmls/design/research/`,
      historical research left as written. No inbound links pointed at the
      removed `#interpolation-vs-condition-mode` or `#parse-modes` anchors,
      apart from the two in-tree links that were rewritten.
- Test mapping: this phase changes no behavior, so it adds no tests. The doc
  examples restate behavior already pinned by the Phase 5 tests
  (`logical_or_value_parity`, `regression_logical_or_*`,
  `a_page_block_gate_logical_or_*`).
- Scope calculator:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → packages `[claudine-cli, darkmatter, dmls, test-toolkit]`, change_class
  `package`. The changed docs are test inputs, so the gates are not
  skippable.
- Gates:
    - darkmatter `just test`: 12884/12884 passed, 10 skipped.
    - darkmatter `just lint`: clean.
    - Narrowed dependent cells:
        - `cargo nextest run -p claudine-cli --features test-fixtures -E 'binary(l1) & test(/loop_gate_ambient/)'`
          → 6/6. This test reads `lifecycle.md` through `include_str!`.
        - `cargo nextest run -p test-toolkit -E 'test(active_ci_authority_matches_the_retirement_contract)'`
          → 1/1. This test reads `.claude/skills/claudine/architecture.md`.
- No cross-OS check: Markdown only.
- No darkmatter skill change beyond `expressions.md`.

## Phase 7

- Phase 7 is validation only. No source, doc, or skill file changed; the only
  edits are this log, the plan, and the spec frontmatter. I did the work
  directly, without subagents, and ran each gate serially.

### Every-feature workspace build (AC4)

- `cargo check --workspace --all-features --all-targets` → exit 0, no
  errors or warnings (2m 58s). A leftover `ParseMode`, `Token::OrOr`,
  `parse_condition`, or `parse_condition_spanned` anywhere in the workspace
  would have been a compile error.

### Leftover sweep

- `rg -n "ParseMode|parse_condition_spanned|parse_condition\b|Token::OrOr|Expr::Fallback|SpannedExprKind::Fallback|Lexer::with_mode|Parser::with_mode|MODE_DESCRIPTORS|hover_markdown_condition" simplified-schema/ darkmatter/ claudine/ --type rust`
  → 2 hits, both the permitted R15 exclusion: claudine's loop-map helper
  `fn parse_condition(` (`composition/looping/config.rs:520`) and its one
  call (line 295). No defects.
- `rg -n "condition mode|condition dialect|Interpolation vs\. Condition|Logical OR / Fallback" darkmatter/docs claudine/docs simplified-schema/docs .claude/skills`
  → 0 hits. The plan names `.opencode/skill`, which is a symlink to
  `.claude/skills` (rg does not follow it), so I searched the target.

### R14 safety search (final run before landing)

- Same command as Phase 1 (`git grep -nE '(when|while|until)[=:"' ]+.*\|\|'`
  over `*.md`/`*.yaml`/`*.yml`/`*.rs`, excluding `_completed`, frozen
  fixtures, and this fix's directory) → 152 lines, of which 102 are
  `when=`/`when:`/`while:`/`until:` lines (Phase 1: 141 / 99; the new
  lines are this fix's Phase 5 tests and Phase 6 doc examples).
- Value-consumed `||` checks: a `||` inside a function argument list, or a
  parenthesized `||` group used as a comparison operand. Hits:
    - `length(items || []) > 0` / `length(frontmatter(plan, 'open_questions') || []) > 0`
      in `conditions_control.rs`, `expression_regression.rs`, and
      `darkmatter-expressions.md`. This fix added these on purpose to pin the
      new value semantics.
    - `(a || b) && c` (`compose/tests/shell.rs`) and
      `(draft || preview) && user.is_admin` (docs). These are tested only for
      truth, so they are unaffected.
    - `prompts/_reviews/feature-review.md:79`. The match comes from the prose
      word "_while_", and the `||` is inside a `{{ }}` interpolation, which
      already had value semantics. Unaffected.
- R14 outcome: **0 existing conditions consume `||` as a value; no rewrite
  needed.**

### Area suites

- simplified-schema `just test` → 1112/1112 passed; `just lint` → clean.
- darkmatter `just test` → 12884/12884 passed, 10 skipped (lib, cli, dmls,
  zed-dmls-cli; same count as Phases 5–6); `just lint` → clean, including
  the zed-dmls wasm32-wasip2 build.
- `cargo nextest run -p darkmatter --features prompt-tests --test prompts`
  → 10/10.
- claudine `just test` → 11810/11810 passed, 15 skipped; `just lint` → clean.
- claudine `just test-prompts` → the fail-fast run stopped after 3 failures.
  So I re-ran the same binary to completion
  (`cargo nextest run -p claudine-cli --features prompt-tests,test-fixtures --test prompts --no-fail-fast`):
  87 run, 81 passed, 6 failed. These are exactly the six Phase 1 baseline
  failures (three `review_fix_rehearsal::*`, three
  `shipped_prompt_route_drift::*`; see decisions_for_review). There are no
  new failures, and `shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions`
  (AC3 corpus) passes.
- `just check-tier-coverage darkmatter` / `claudine` → 0 stranded.
- Narrowed dependent cells for the changed docs (still uncommitted in this
  tree from Phase 6):
  `cargo nextest run -p claudine-cli --features test-fixtures -E 'binary(l1) & test(/loop_gate_ambient/)'`
  → 6/6; `cargo nextest run -p test-toolkit -E 'test(active_ci_authority_matches_the_retirement_contract)'`
  → 1/1. Other dependents are covered at compile level by the all-features
  workspace check; this phase changed no symbol.
- No load-sensitive or flaky tests were seen.

### Acceptance-criteria roll-up

Every test below passed in this phase's runs:

- **AC1 — `||` returns the same value on `{{ }}`, `::block when=`, and
  lifecycle `when:`.**
    - Evaluator rows, `darkmatter/lib/tests/l1/expression_regression.rs`
      `logical_or_value_parity::*`:
      `a_present_left_operand_is_the_value`,
      `an_absent_left_operand_yields_the_right_operand`,
      `a_defaulted_collection_feeds_length_in_a_condition`,
      `a_truthy_left_operand_is_returned_not_coerced_to_a_boolean`,
      `two_falsy_operands_yield_the_last_operand`,
      `a_truthy_left_operand_short_circuits_the_right_operand`.
    - Body and page-block surfaces:
      `regression_logical_or_interpolates_the_deciding_operand`,
      `regression_logical_or_defaults_a_collection_in_a_page_block_gate`,
      `regression_logical_or_gate_takes_the_truth_of_the_deciding_operand`.
    - Lifecycle surface: claudine
      `conditions_control::when_logical_or_takes_the_truth_of_the_deciding_operand`.
- **AC2 — the `open_questions` guard fires.** claudine
  `conditions_control::when_logical_or_defaulted_open_questions_fires_when_listed`
  and `..._skips_when_absent`.
- **AC3 — AST-shape tests rewritten to `LogicalOr`, not deleted.**
    - simplified-schema `expression::parser::tests::logical_operators::*` (10/10,
      including `parses_infix_or_as_logical_or_node`,
      `chained_or_left_associative`, `and_binds_tighter_than_or_{left,right}`,
      `parenthesized_or_then_and`, `or_inside_or`).
    - The corpora: darkmatter prompts 10/10 and claudine
      `shipped_prompts_have_parseable_schemas_and_expressions`. These check
      syntax only.
    - Shipped passive call-check verdicts (added in review-fix iteration 1,
      see `log.md`): darkmatter prompts
      `shipped_conditions_keep_their_call_check_verdict` call-checks every
      shipped condition (224, 22 with `||`) as parsed and with each
      `LogicalOr` projected back to the former `or(…)` call; no verdict
      differs. Controls: `call_check_distinguishes_a_rejected_condition_call`,
      `call_verdict_comparison_reports_a_changed_verdict`,
      `projection_marks_only_synthesized_or_calls`.
- **AC4 — the mode symbols are gone.** The every-feature workspace check
  above plus the zero-defect leftover sweep.
- **AC5 — absence handling for the left operand in conditions.**
    - darkmatter `unknown_identifier_warning::a_page_block_gate_logical_or_handles_only_its_left_operand`
      and `..._over_declared_roots_is_silent`.
    - DMLS `lsp_session::undeclared_property_fires_in_every_operand_position_at_warning_severity`.
- **AC6 — the nested-span suggestion is offered for `a == "x {{ y || 'd' }}"`.**
  darkmatter `lint::tests::lint_model::logical_or_span_is_offered_a_suggestion`
  (exact text `a == "x " + (y || 'd')`), `suggestion_beside_logical_or_reparses`,
  `fires_once_beside_logical_or`.

### Spec departures and drift

- No new departures in this phase. The departures from earlier phases are in
  decisions_for_review: the `expression` format docs live in
  simplified-schema, the `context --expressions` tests were rewritten, the
  DMLS right-operand warning, and the renamed mode-split tests.
- No comment or doc drift found. The sweeps above are clean.
- No darkmatter skill change: Phase 7 changed no behavior, and Phase 6 already
  brought `.claude/skills/darkmatter/expressions.md` up to date.
- No cross-OS check: no code changed in this phase. Earlier phases changed
  grammar and AST code with no path, process, or OS-specific surface.

### Scope calculator

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → packages `[claudine-cli, darkmatter, dmls, test-toolkit]`, change_class
  `package`. This includes Phase 6's uncommitted doc edits, so the gates
  were not skippable and were run (above).

### State

- Implementation complete, ready for review. Nothing committed, no
  `just complete`, no lifecycle move.
