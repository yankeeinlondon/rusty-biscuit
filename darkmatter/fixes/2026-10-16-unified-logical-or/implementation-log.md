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
message_to_agent: |-
    Phase 3 finished darkmatter/lib (and darkmatter-cli needed no change): all
    `Fallback` arms are `LogicalOr { left, right }`; `AuthoredMode::Expression`
    is a unit variant; `lint_expression(source)` takes one argument (the
    private helper is `lint_parsed(source, expr, suggest: bool)`);
    `ModeDescriptor`/`MODE_DESCRIPTORS`/`mode_descriptors()` are deleted;
    `compose::expression` no longer re-exports `ParseMode`, `parse_condition`,
    or `parse_condition_spanned`. Green: darkmatter nextest 9720/9720,
    doctests, prompts 10/10, darkmatter-cli 1460/1460, clippy on both.
    Still red, as planned for Phase 4: dmls (10 errors, all R10 items) and
    claudine (`claudine/cli/src/commands/context/expressions.rs` still imports
    `mode_descriptors`, plus the R1/R8/R15 sites). The full darkmatter
    `just test`/`just lint` cannot pass until dmls compiles, so run them
    once Phase 4's DMLS task is done. Phase 1's baseline still applies:
    6 named claudine `test-prompts` failures pre-exist.
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
