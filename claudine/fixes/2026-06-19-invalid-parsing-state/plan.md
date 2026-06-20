---
agent: codex/
phases: 6
created: 2026-06-20
start_phase: 1
yolo: "true"
---

# Invalid Frontmatter Expansion State Plan

## Success Criteria

- Whole-value frontmatter `{{ ... }}` values fail during Darkmatter composition on parse or evaluation errors, regardless of global `fail_fast`.
- Whole-value frontmatter `$()` values fail when enabled shell expansion cannot parse or expand them, and no enabled whole-value shell candidate can leak into final frontmatter.
- Mixed body prose and mixed frontmatter string leniency remain unchanged when `fail_fast` is false.
- Claudine dry-run output no longer prints the malformed `spec_path` template as successful effective frontmatter.
- Documentation states that frontmatter values that are exactly an expansion form are executable state, not ordinary text.

## Phase 1: Baseline and Localize Existing Behavior

- [ ] Read `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`, `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`, `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`, and `darkmatter/lib/src/markdown/compose/pipeline/mod.rs` to confirm the current ordering and helper contracts.
- [ ] Identify the existing error type and constructor patterns used for `MarkdownError::Transform` in frontmatter interpolation and shell expansion.
- [ ] Confirm how `SourceContext` maps frontmatter keys to source file and line information, including any existing tests that assert key-aware diagnostics.
- [ ] Inspect `claudine/cli/tests/compose_cli.rs` and nearby schema/composition tests to find the lowest-friction CLI regression test harness.
- [ ] Record the exact commands available in the `darkmatter` and `claudine` package-area `justfile`s without running formatting commands.
- [ ] Validation checkpoint: reproduce or confirm the current failure mode with the motivating dry-run command or a reduced local fixture, showing that malformed `spec_path` survives as effective frontmatter before the fix.

Parallelizable:

- [ ] The Claudine CLI test-harness inspection can run in parallel with Darkmatter source inspection because it depends only on locating fixture patterns, not on the implementation decision.

## Phase 2: Enforce Strict Whole-Value `{{ ... }}` Frontmatter Interpolation

- [ ] Add a helper in `frontmatter_interpolation.rs` that detects a string whose trimmed content is exactly one interpolation span by using `ExpressionFinder::find_all_plain(...)`.
- [ ] Route exact whole-value interpolation values through direct parse and evaluation before the existing mixed-string `interpolate_text(...)` fallback.
- [ ] Return a fatal `MarkdownError::Transform` for parse failures even when `ComposeOptions.fail_fast` is false.
- [ ] Return a fatal `MarkdownError::Transform` for evaluation failures even when `ComposeOptions.fail_fast` is false.
- [ ] Preserve current successful whole-value type behavior for booleans, numbers, nulls, and strings.
- [ ] Make an explicit implementation decision for arrays and objects by either preserving typed JSON values or retaining current string-path behavior, then cover that decision with tests or a focused assertion against existing behavior.
- [ ] Ensure error text includes the frontmatter key, the interpolation expression text, and any available source location context.
- [ ] Validation checkpoint: run targeted Darkmatter interpolation tests for the modified module and confirm malformed whole-value interpolation fails while mixed malformed interpolation remains warning-based when `fail_fast` is false.

## Phase 3: Enforce Strict Whole-Value `$()` Frontmatter Shell Expansion

- [ ] Locate the enabled shell-expansion stage in `execute_frontmatter_shell_expansion(...)` and identify where final frontmatter values are available after replacements.
- [ ] Factor or reuse a shell-candidate helper around `parse_shell_value(...)` so post-expansion validation does not duplicate shell grammar or supported suffix rules.
- [ ] Add a post-expansion validation pass over top-level string-valued frontmatter when frontmatter shell expansion is enabled.
- [ ] Fail with `MarkdownError::Transform` if any final top-level value still trims to a whole-value shell expansion candidate after enabled shell expansion has run.
- [ ] Keep existing deferred behavior when frontmatter shell expansion is explicitly disabled in compose options.
- [ ] Preserve mixed literal behavior for values such as `literal $(echo ok)` unless they already match an existing strict parser rule.
- [ ] Ensure shell errors include the frontmatter key, expression text, and source location context where `SourceContext` can locate the key.
- [ ] Validation checkpoint: run targeted Darkmatter shell-expansion tests and confirm valid `$()` expands, malformed `$(` errors, expression-shaped non-command `$()` reports the existing diagnostic, and mixed literals remain outside the new strict whole-value rule.

## Phase 4: Add Regression Coverage

- [ ] Add Darkmatter unit tests near `frontmatter_interpolation.rs` for malformed whole-value interpolation, valid string interpolation, boolean preservation, number preservation, and mixed malformed interpolation leniency.
- [ ] Add Darkmatter unit tests near `frontmatter_shell_expansion.rs` for enabled valid shell expansion, parse failure, existing no-command diagnostic, post-expansion leakage prevention, and mixed literal behavior.
- [ ] Add a Claudine CLI regression fixture with `spec_path: "{{ dirname(review) + '/spec.md') }}"`.
- [ ] Add a Claudine CLI regression test asserting `claudine compose ... --dry-run` exits non-zero, mentions `spec_path`, mentions an interpolation parse failure or equivalent precise parse diagnostic, and does not print the raw `{{ ... }}` value as a successful effective frontmatter result.
- [ ] Keep fixtures minimal and local to the existing test style so the test does not depend on provider availability or network access.
- [ ] Validation checkpoint: run the new targeted Darkmatter unit tests and the new Claudine CLI regression test directly before broad package test runs.

Parallelizable:

- [ ] Darkmatter unit test additions for interpolation and shell expansion can be drafted in parallel after Phases 2 and 3 define the final helper APIs.
- [ ] The Claudine CLI fixture and assertion structure can be drafted in parallel with Darkmatter tests, but final expected stderr assertions should wait until Darkmatter error text is stable.

## Phase 5: Update Documentation and Skill Notes

- [ ] Update `claudine/docs/topics/composition.md` to state that whole-value frontmatter `{{ ... }}` must parse and evaluate successfully even when mixed/body interpolation remains warning-based.
- [ ] Update `claudine/docs/topics/composition.md` to state that whole-value frontmatter `$()` must parse and expand when frontmatter shell expansion is enabled.
- [ ] Update `.claude/skills/claudine/SKILL.md` composition notes with the same strictness boundary and the key invariant: exact expansion-form frontmatter values are not text and must never leak as raw syntax.
- [ ] Update the relevant Darkmatter compose docs or module docs if they currently describe interpolation warnings, frontmatter interpolation, or `$()` expansion without the strict whole-value exception.
- [ ] Review changed comments and rustdoc around modified symbols; fix or remove drifted comments while preserving the repo rule against unrelated cleanup.
- [ ] Validation checkpoint: verify documentation uses US English and does not claim mixed strings or body interpolation are newly fatal.

Parallelizable:

- [ ] Claudine docs and Darkmatter docs can be updated in parallel after the implementation behavior and exact terms are known.

## Phase 6: Full Validation and Closure

- [ ] Run the narrow Darkmatter test filters for frontmatter interpolation and frontmatter shell expansion.
- [ ] Run the narrow Claudine CLI compose regression test.
- [ ] Run `just test` in the `darkmatter` package area.
- [ ] Run `just test` in the `claudine` package area.
- [ ] Manually validate the original reproduction command: `claudine compose prompts/implement-suggestions.md -y review="features/2026-06-19-review-findings/review-2.md" --claude --dry-run`.
- [ ] Confirm the manual validation fails during compose preparation with a frontmatter interpolation parse error for `spec_path`.
- [ ] Confirm dry-run effective frontmatter does not render the raw malformed `{{ dirname(review) + '/spec.md') }}` value as a successful result.
- [ ] Review `git diff` to ensure changes are surgical and limited to Darkmatter enforcement, Claudine regression coverage, and required docs.
- [ ] Validation checkpoint: map final results back to all seven acceptance criteria in `spec.md` and note any tests not run or platform limitations.

Parallelizable:

- [ ] Narrow Darkmatter and Claudine test commands can run in parallel if they do not share mutable fixtures.
- [ ] Documentation diff review can run in parallel with long package test runs.
