---
agent: codex/
phases: 6
created: 2026-06-20
start_phase: 1
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - darkmatter
  - claudine
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/tests.rs
  - claudine/lib/src/composition/prepare.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - darkmatter
source_files_during_phase_4:
  - claudine/cli/tests/compose_cli.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - claudine
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/docs/topics/composition.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/inline/fm-shell-expansion.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/SKILL.md
packages_during_phase_5:
  - claudine
  - darkmatter
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6: []
source_code:
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/tests.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/cli/tests/compose_cli.rs
documentation:
  - claudine/docs/topics/composition.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/inline/fm-shell-expansion.md
  - .claude/skills/claudine/SKILL.md
---

# Invalid Frontmatter Expansion State Plan

## Success Criteria

- Whole-value frontmatter `{{ ... }}` values fail during Darkmatter composition on parse or evaluation errors, regardless of global `fail_fast`.
- Whole-value frontmatter `$()` values fail when enabled shell expansion cannot parse or expand them, and no enabled whole-value shell candidate can leak into final frontmatter.
- Mixed body prose and mixed frontmatter string leniency remain unchanged when `fail_fast` is false.
- Claudine dry-run output no longer prints the malformed `spec_path` template as successful effective frontmatter.
- Documentation states that frontmatter values that are exactly an expansion form are executable state, not ordinary text.

## Phase 1: Baseline and Localize Existing Behavior

- [x] Read `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`, `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`, `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`, and `darkmatter/lib/src/markdown/compose/pipeline/mod.rs` to confirm the current ordering and helper contracts.
- [x] Identify the existing error type and constructor patterns used for `MarkdownError::Transform` in frontmatter interpolation and shell expansion.
- [x] Confirm how `SourceContext` maps frontmatter keys to source file and line information, including any existing tests that assert key-aware diagnostics.
- [x] Inspect `claudine/cli/tests/compose_cli.rs` and nearby schema/composition tests to find the lowest-friction CLI regression test harness.
- [x] Record the exact commands available in the `darkmatter` and `claudine` package-area `justfile`s without running formatting commands.
- [x] Validation checkpoint: reproduce or confirm the current failure mode with the motivating dry-run command or a reduced local fixture, showing that malformed `spec_path` survives as effective frontmatter before the fix.

Parallelizable:

- [x] The Claudine CLI test-harness inspection can run in parallel with Darkmatter source inspection because it depends only on locating fixture patterns, not on the implementation decision.

### Phase 1 Findings (baseline)

**Pipeline ordering** (`pipeline/mod.rs`, `run_compose_pipeline_internal`): frontmatter resolves in a fixed order — Interp pass 1 → Schema Validation → Shell Expansion → Interp pass 2. `interpolate_frontmatter(..)` runs first (line ~156) with `defer_shell_pending = shell_expansion_enabled`; `execute_frontmatter_shell_expansion(..)` runs at line ~226 only when `shell_expansion_enabled`; the second interp pass runs after, with `defer_shell_pending = false`.

**Interpolation contracts**: `frontmatter_interpolation::rewrite_value` → `interpolation::rewrite::interpolate_value(s, evaluator, fail_fast, "frontmatter-interpolation")`. `interpolate_value` first tries `whole_value_scalar(input, evaluator)` (uses `ExpressionFinder::find_all_plain`, requires exactly one span with only-whitespace before/after, then `parse(..).ok()?` and `eval_json`), but it returns `Some` **only** for `Bool | Number | Null` — strings/arrays/objects and any parse/eval failure fall through to `interpolate_text`. `interpolate_text` (rewrite.rs) treats parse failures and non-fatal eval failures as warnings when `fail_fast == false`, leaving the raw `{{ … }}` in place. Only unknown-function eval errors (`is_fatal_eval_error`, `UNKNOWN_FUNCTION_PREFIX`) are fatal regardless of `fail_fast`. This is the leak path for the malformed `spec_path`.

**Error type / constructors**: interpolation uses `MarkdownError::Transform(format!(...))` — e.g. `"Interpolation parse failed for '{expr}': {e}"` and `"Interpolation evaluation failed for '{expr}': {message}"`. Shell expansion uses `ShellExpansionError` built via `frontmatter_parse_error(key, ctx, msg)` (key-tagged); `no_command_diagnostic(..)` is the existing `$()`-with-no-command diagnostic; `parse_shell_value(..)` already errors on `$(` that fails to close or has bad suffixes.

**SourceContext key→line**: `shell_expansion::types::frontmatter_key_line(ctx: &SourceContext, key)` reads `ctx.frontmatter` range + `ctx.content`, scans lines for `key:` / `'key':` / `"key":`, returns 1-indexed file line (`None` when no range/key). Existing test `frontmatter_key_line_resolves_quoted_and_unquoted_keys` (types.rs:1891) asserts this. Directives set `directive.line = frontmatter_key_line(ctx, key)` in `scan_frontmatter`.

**Claudine CLI test harness**: lowest-friction file is `claudine/cli/tests/compose_cli.rs` (uses `assert_cmd::cargo::cargo_bin_cmd!`, `tempfile::tempdir`, `mod common` helpers `augmented_path`/`write_executable`; sets `NO_COLOR`/`HOME`/`PATH`, `current_dir`, runs `compose --<provider>`). A malformed-interpolation regression needs no provider binary because compose preparation fails before any provider spawn. `compose_schema_cli.rs` is the parallel schema-error harness.

**Justfile commands** (no fmt): darkmatter & claudine both `import "../just/devops.just"` etc. Package-local recipes — `just test` (Level-1 nextest, filter `'!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/))'`, runs `-p {lib}` then `-p {cli}`), `just test-l2`, `just test-l3` (claudine), `just lint`, `just check`, `just sanity`, `just all`. Narrow filter example: `cargo nextest run -p darkmatter <substring>`.

**Validation checkpoint (bug reproduced)**: a throwaway lib test over `{ review: "…/review-2.md", spec_path: "{{ dirname(review) + '/spec.md') }}" }` run through `interpolate_frontmatter(.., fail_fast=false, defer=false, None)` confirmed the raw malformed value `"{{ dirname(review) + '/spec.md') }}"` survives untouched in effective frontmatter and is only recorded as a warning (not a hard error). Test passed (asserting the buggy survival), then removed — no production source changed in Phase 1.

## Phase 2: Enforce Strict Whole-Value `{{ ... }}` Frontmatter Interpolation

- [x] Add a helper in `frontmatter_interpolation.rs` that detects a string whose trimmed content is exactly one interpolation span by using `ExpressionFinder::find_all_plain(...)`.
- [x] Route exact whole-value interpolation values through direct parse and evaluation before the existing mixed-string `interpolate_text(...)` fallback.
- [x] Return a fatal `MarkdownError::Transform` for parse failures even when `ComposeOptions.fail_fast` is false.
- [x] Return a fatal `MarkdownError::Transform` for evaluation failures even when `ComposeOptions.fail_fast` is false.
- [x] Preserve current successful whole-value type behavior for booleans, numbers, nulls, and strings.
- [x] Make an explicit implementation decision for arrays and objects by either preserving typed JSON values or retaining current string-path behavior, then cover that decision with tests or a focused assertion against existing behavior.
- [x] Ensure error text includes the frontmatter key, the interpolation expression text, and any available source location context.
- [x] Validation checkpoint: run targeted Darkmatter interpolation tests for the modified module and confirm malformed whole-value interpolation fails while mixed malformed interpolation remains warning-based when `fail_fast` is false.

### Phase 2 Findings (implementation notes)

**Helper location (deviation from literal task wording).** The whole-value detection helper `whole_value_span(input) -> Option<ExpressionLocation>` and the strict parse-and-evaluate routing live in `interpolation/rewrite.rs` (inside `interpolate_value`), not `frontmatter_interpolation.rs`. Rationale: `interpolate_value` is the frontmatter-only interpolation entry point (single caller, `frontmatter_interpolation::rewrite_value`), so co-locating detection + strict eval there avoids a backwards low-level→high-level module dependency. It replaces the old `whole_value_scalar` (which restricted typed results to `Bool|Number|Null` and silently fell through on parse/eval failure). The functional contract the task asked for is met: whole-value spans route through direct `parse` + `eval_json` *before* the lenient `interpolate_text` fallback.

**Arrays/objects decision: preserve typed JSON.** A successful whole-value `{{ … }}` now yields its evaluated `serde_json::Value` for *all* types — bool, number, null, string, array, and object — generalizing the prior scalar-only preservation. Covered by `whole_value_array_reference_preserves_typed_value` (a whole-value reference to a seed array resolves to the typed array, not a stringified form).

**Strict scope.** Only genuine parse failures and genuine evaluation errors are fatal. Undefined variables still resolve to `null` (`Expr::Variable` → `Ok(Null)`), so `{{ missing }}` stays lenient; string concatenation with a null operand coerces (not an error). The fatal set therefore covers malformed syntax, unknown functions, type errors, and read-side errors (e.g. remote URL in the local-only frontmatter surface).

**Key in error.** `rewrite_value` errors are wrapped at the two `interpolate_frontmatter` call sites by `key_scoped_error(key, err)`, prepending `frontmatter key '<key>': ` to the `Transform` message (which already carries the expression text). No `SourceContext` is threaded into `interpolate_frontmatter`, so file/line context is "where available" — the claudine render boundary maps the named key back to a frontmatter line for its highlighted excerpt.

**Downstream test updates required by the stricter behavior** (kept the suite green, all aligned with success criterion #1):
- `claudine` `implement_suggestions_prompt_rejects_malformed_spec_path` (was `…composes_without_lifecycle_leak`): the shipped prompt's malformed `spec_path` now aborts compose with a parse error naming `spec_path` instead of leaking — this is the motivating bug; the prompt is intentionally left malformed as the Phase 6 reproduction specimen.
- `claudine` lifecycle leak-guard tests (`malformed_lifecycle_interpolation_fails_preparation`, `lifecycle_leak_reported_for_first_field_in_deterministic_order`): fixtures changed from whole-value to *mixed* malformed strings, since whole-value malformed lifecycle spans are now caught earlier by Darkmatter; the claudine leak guard still covers the mixed-string case.
- `darkmatter` Decision-B remote-URL tests (`frontmatter_value_remote_url_fails_loudly`, `frontmatter_file_exists_remote_url_fails_loudly`): a whole-value read-side error on the local-only frontmatter surface now aborts composition (a stronger "fail loudly") rather than warning + leaving the value unsubstituted.

**Pre-existing, unrelated failure:** `darkmatter` `schema_validation::tests::inline_object_uncoercible_value_left_alone` fails on the unmodified baseline too (confirmed via `git stash`); not caused by this phase.

## Phase 3: Enforce Strict Whole-Value `$()` Frontmatter Shell Expansion

- [x] Locate the enabled shell-expansion stage in `execute_frontmatter_shell_expansion(...)` and identify where final frontmatter values are available after replacements.
- [x] Factor or reuse a shell-candidate helper around `parse_shell_value(...)` so post-expansion validation does not duplicate shell grammar or supported suffix rules.
- [x] Add a post-expansion validation pass over top-level string-valued frontmatter when frontmatter shell expansion is enabled.
- [x] Fail with `MarkdownError::Transform` if any final top-level value still trims to a whole-value shell expansion candidate after enabled shell expansion has run.
- [x] Keep existing deferred behavior when frontmatter shell expansion is explicitly disabled in compose options.
- [x] Preserve mixed literal behavior for values such as `literal $(echo ok)` unless they already match an existing strict parser rule.
- [x] Ensure shell errors include the frontmatter key, expression text, and source location context where `SourceContext` can locate the key.
- [x] Validation checkpoint: run targeted Darkmatter shell-expansion tests and confirm valid `$()` expands, malformed `$(` errors, expression-shaped non-command `$()` reports the existing diagnostic, and mixed literals remain outside the new strict whole-value rule.

### Phase 3 Findings (implementation notes)

**Most parse-time strictness already existed.** The four behaviors the validation checkpoint asks for — valid `$()` expands, malformed `$(` errors (`find_unquoted_closing_paren`), expression-shaped non-command `$()` reports `no_command_diagnostic`, and mixed literals stay outside the rule (`scan_frontmatter` requires a leading `$(`) — were all already implemented in `frontmatter_shell_expansion.rs` and confirmed green. The genuinely new code is a **post-expansion leak guard**.

**New guard: `validate_no_whole_value_shell_leak`.** Added to `frontmatter_shell_expansion.rs` and called from `execute_frontmatter_shell_expansion` after the replacement loop (and in the early no-op return). It walks every top-level string value and rejects any that is still a whole-value `$(...)` candidate. This catches the residual leaks the strict-start scan cannot: command output that *reproduces* `$( … )` (e.g. `$(echo '$(date)')`), and a whole-value `$(...)` behind leading whitespace (`"  $(echo hi)"`) that `scan_frontmatter`'s `starts_with("$(")` skips. Guarding inside `execute_frontmatter_shell_expansion` means the **disabled** path (where the pipeline never calls it) keeps `$(...)` deferred unchanged, satisfying the "explicitly disabled" task.

**Candidate helper: `is_whole_value_shell_candidate`.** Trims the value and, only when it opens `$(`, delegates to `parse_shell_value(trimmed, key, None, ctx)`. A value counts as a leak **only** when the shared parser returns `Ok(Some(_))` — a clean whole-value directive. This reuses the `$( … )` grammar plus the `::timeout` / `::no-cache` suffix rules in one place (no duplication) and, crucially, leaves mixed/trailing forms alone: `literal $(echo ok)` (no leading `$(`), `$(echo ok) trailing` (`Err`: trailing content), and malformed `$(foo` (`Err`) all pass the guard untouched.

**Error type deviation (`ParseDirective`, not `Transform`).** The task wording says "Fail with `MarkdownError::Transform`", but the guard uses the module's existing `frontmatter_parse_error(key, ctx, msg)` → `ShellExpansionError::ParseDirective` → `MarkdownError::ShellExpansion`. Rationale: `ShellExpansion` is the key-tagged, line-aware error that claudine's `map_compose_error` already routes to `CompositionError::ShellExpansionFailed`, which the CLI renders with the syntax-highlighted frontmatter excerpt. A bare `Transform(String)` would lose the frontmatter key/line tagging and the rich render. The message carries the offending expression text, and the error origin carries the key + resolved line, satisfying the key/expression/source-location task. No claudine-side change was needed — the existing `ShellExpansion` mapping handles the new diagnostic.

**Tests.** Added four focused unit tests in the module: `leak_guard_rejects_surviving_whole_value_candidate`, `leak_guard_trims_before_classifying`, `leak_guard_ignores_plain_and_mixed_values`, and `is_whole_value_shell_candidate_recognizes_forms`. The 114-test `frontmatter_shell_expansion` filter passes. (Comprehensive interpolation/shell/CLI regression coverage is Phase 4.)

**Pre-existing, unrelated failures.** `darkmatter`'s `just test` shows 6 failures in `layout::page::tests` (color-mode / code-block padding); they reproduce identically on the baseline with the Phase 3 change stashed and are terminal-color-mode-dependent, not caused by this phase.

## Phase 4: Add Regression Coverage

- [x] Add Darkmatter unit tests near `frontmatter_interpolation.rs` for malformed whole-value interpolation, valid string interpolation, boolean preservation, number preservation, and mixed malformed interpolation leniency.
- [x] Add Darkmatter unit tests near `frontmatter_shell_expansion.rs` for enabled valid shell expansion, parse failure, existing no-command diagnostic, post-expansion leakage prevention, and mixed literal behavior.
- [x] Add a Claudine CLI regression fixture with `spec_path: "{{ dirname(review) + '/spec.md') }}"`.
- [x] Add a Claudine CLI regression test asserting `claudine compose ... --dry-run` exits non-zero, mentions `spec_path`, mentions an interpolation parse failure or equivalent precise parse diagnostic, and does not print the raw `{{ ... }}` value as a successful effective frontmatter result.
- [x] Keep fixtures minimal and local to the existing test style so the test does not depend on provider availability or network access.
- [x] Validation checkpoint: run the new targeted Darkmatter unit tests and the new Claudine CLI regression test directly before broad package test runs.

Parallelizable:

- [x] Darkmatter unit test additions for interpolation and shell expansion can be drafted in parallel after Phases 2 and 3 define the final helper APIs.
- [x] The Claudine CLI fixture and assertion structure can be drafted in parallel with Darkmatter tests, but final expected stderr assertions should wait until Darkmatter error text is stable.

### Phase 4 Findings (implementation notes)

**Darkmatter coverage was front-loaded into Phases 2 and 3.** Every interpolation and shell-expansion case this phase asks for already landed alongside the enforcement code (each phase kept its suite green by adding the matching regression test), so no new Darkmatter tests were authored — adding duplicates would violate the repo's surgical-change rule. The existing tests that satisfy each Phase 4 item:

- *Interpolation* (`frontmatter_interpolation.rs`): malformed whole-value → `whole_value_parse_failure_is_fatal_without_fail_fast`; valid string / boolean / number preservation → `whole_value_interpolation_preserves_scalar_type` (covers `'x'`, `false`, `1 + 1`, `null`, and a mixed-string case in one assertion); mixed malformed leniency → `mixed_malformed_interpolation_stays_warning_without_fail_fast`. The narrow `frontmatter_interpolation` filter runs 53 green.
- *Shell expansion* (`frontmatter_shell_expansion.rs`): enabled valid expansion → `execution_tests::execute_replaces_frontmatter_value_with_output`; parse failure → `scan_errors_on_malformed_shell_expression` / `ignores_partial_match_no_closing`; existing no-command diagnostic → `non_ternary_all_expression_value_errors_with_brace_suggestion` and `ternary_with_no_command_branch_errors_with_brace_suggestion`; post-expansion leakage prevention → `leak_guard_rejects_surviving_whole_value_candidate` / `leak_guard_trims_before_classifying`; mixed literal behavior → `leak_guard_ignores_plain_and_mixed_values` / `ignores_embedded_expression`. The narrow `frontmatter_shell_expansion` filter runs 114 green.

**New work: the Claudine CLI regression test.** `compose_dry_run_malformed_whole_value_spec_path_aborts_without_leaking` in `claudine/cli/tests/compose_cli.rs` drives the compiled binary end-to-end. The fixture is an inline local `plan.md` carrying `spec_path: "{{ dirname(review) + '/spec.md') }}"` (the shipped malformed shape) plus a seed `review` value, in the established schema-test style (no separate fixtures file, no network). It runs `compose --goose <file> --dry-run`, asserts a non-zero exit, that stderr names `spec_path` and contains `Interpolation parse failed`, that stdout never contains the raw `dirname(review)` template (the original leak), and that the staged `goose` stub is never invoked (preparation aborts before the dry-run provider seam). Used `--goose` with a recording stub rather than the motivating `--claude` so the test stays hermetic and provider-agnostic — the parse failure is provider-independent (Phase 6 runs the literal `--claude` reproduction manually).

## Phase 5: Update Documentation and Skill Notes

- [x] Update `claudine/docs/topics/composition.md` to state that whole-value frontmatter `{{ ... }}` must parse and evaluate successfully even when mixed/body interpolation remains warning-based.
- [x] Update `claudine/docs/topics/composition.md` to state that whole-value frontmatter `$()` must parse and expand when frontmatter shell expansion is enabled.
- [x] Update `.claude/skills/claudine/SKILL.md` composition notes with the same strictness boundary and the key invariant: exact expansion-form frontmatter values are not text and must never leak as raw syntax.
- [x] Update the relevant Darkmatter compose docs or module docs if they currently describe interpolation warnings, frontmatter interpolation, or `$()` expansion without the strict whole-value exception.
- [x] Review changed comments and rustdoc around modified symbols; fix or remove drifted comments while preserving the repo rule against unrelated cleanup.
- [x] Validation checkpoint: verify documentation uses US English and does not claim mixed strings or body interpolation are newly fatal.

Parallelizable:

- [x] Claudine docs and Darkmatter docs can be updated in parallel after the implementation behavior and exact terms are known.

### Phase 5 Findings (documentation notes)

**Claudine composition doc.** Added a new `## Whole-Value Frontmatter Expansion Is Executable State` section to `claudine/docs/topics/composition.md` (between the inline-compose/sequence-mismatch frontmatter section and the YAML-blocks-in-errors section). It states both strictness rules — whole-value `{{ ... }}` must parse/evaluate even when `fail_fast` is off, whole-value `$(...)` must parse/expand when frontmatter shell expansion is enabled — and explicitly scopes the strictness to whole-value spans only, noting mixed strings and body interpolation keep their lenient warning behavior. Links out to the two Darkmatter docs.

**Darkmatter compose docs.** `fm-interpolation.md`'s "Missing Variables And Errors" section previously said only "When `fail_fast` is disabled, the original string is preserved and a warning is recorded" — which is now inaccurate for whole-value spans. Added a "Whole-Value Exception (Strict)" subsection documenting the strict parse-and-evaluate contract, typed-result preservation, and the undefined-variable-stays-lenient carve-out, while reaffirming mixed/body interpolation are not newly fatal. `fm-shell-expansion.md` already documented parse-time strictness; added a "Post-Expansion Leak Guard" subsection under Error Handling describing the enabled-only guard, the residual leaks it catches, and the disabled/deferred path.

**Skill notes.** Added a "Whole-value frontmatter expansion strictness" bullet to the composition notes in `.claude/skills/claudine/SKILL.md`, leading with the key invariant (exact expansion-form values are executable state, never text, must never leak as raw syntax), and regenerated the body `hash:` frontmatter via `md hash`. Updated the repo-scoped skill copy only (the one the session loaded).

**Comment/rustdoc review.** The inline `//` comments and `///` rustdoc around the modified symbols (`interpolate_value`, `whole_value_span`, `key_scoped_error`, `validate_no_whole_value_shell_leak`, `is_whole_value_shell_candidate`, and the claudine `prepare.rs` lifecycle-guard comments) were authored alongside the Phase 2/3 behavior and already describe the strict whole-value contract accurately — no drift to fix.

**Validation.** US English throughout (e.g. "behavior", "recognized"); no claim that mixed strings or body interpolation are newly fatal — every doc explicitly states the opposite. `just lint` and `just test` in the claudine area pass (1667 tests, 0 failures). No Rust source changed in this phase.

## Phase 6: Full Validation and Closure

- [x] Run the narrow Darkmatter test filters for frontmatter interpolation and frontmatter shell expansion.
- [x] Run the narrow Claudine CLI compose regression test.
- [x] Run `just test` in the `darkmatter` package area.
- [x] Run `just test` in the `claudine` package area.
- [x] Manually validate the original reproduction command: `claudine compose prompts/implement-suggestions.md -y review="features/2026-06-19-review-findings/review-2.md" --claude --dry-run`.
- [x] Confirm the manual validation fails during compose preparation with a frontmatter interpolation parse error for `spec_path`.
- [x] Confirm dry-run effective frontmatter does not render the raw malformed `{{ dirname(review) + '/spec.md') }}` value as a successful result.
- [x] Review `git diff` to ensure changes are surgical and limited to Darkmatter enforcement, Claudine regression coverage, and required docs.
- [x] Validation checkpoint: map final results back to all seven acceptance criteria in `spec.md` and note any tests not run or platform limitations.

Parallelizable:

- [x] Narrow Darkmatter and Claudine test commands can run in parallel if they do not share mutable fixtures.
- [x] Documentation diff review can run in parallel with long package test runs.

### Phase 6 Findings (validation and closure)

**Narrow filters.** `cargo nextest run -p darkmatter frontmatter_interpolation` → 53 passed. `cargo nextest run -p darkmatter frontmatter_shell_expansion` → 114 passed. `cargo nextest run -p claudine-cli compose_dry_run_malformed_whole_value_spec_path_aborts_without_leaking` → 1 passed.

**Package suites.** `claudine` `just test` → 1668 passed, 0 failed. `darkmatter` `just test` → 6 failures, all in `layout::page::tests` (color-mode / code-block panel inversion: `color_mode_unknown_*`, `dark_terminal_inverts_to_light_panel_via_captured_terminal`, `code_panel_inverts_against_terminal_not_option_in_transparent_default`). These reproduce **identically with the Phase 2/3 compose changes stashed out** (verified via `git stash push -- darkmatter/lib/src/markdown/compose/` + targeted re-run, 3/3 still failing), are terminal-color-mode-dependent (assert absence of truecolor `48;2;` background sequences), and touch no file this fix modified. Pre-existing and unrelated — consistent with the Phase 3 finding.

**Lints.** `just lint` clean in both `darkmatter` and `claudine` areas.

**Manual reproduction.** The original command first aborts on `review` schema validation because `features/2026-06-19-review-findings/review-2.md` does not exist in this worktree. Pointing `review` at an existing file (`-y review="/tmp/.../review-2.md"`) drives compose past schema validation and confirms the fix: exit code 1, `MarkdownError: transform failed`, message `frontmatter key 'spec_path': Interpolation parse failed for 'dirname(review) + '/spec.md')': Expected end of expression, found ')' at position 6`. The raw `{{ dirname(review) + '/spec.md') }}` template is **not** rendered as effective frontmatter — composition aborts during preparation, before any dry-run effective-frontmatter dump.

**Diff review.** `git diff --stat` shows changes limited to: Darkmatter enforcement (`interpolation/rewrite.rs`, `frontmatter_interpolation.rs`, `frontmatter_shell_expansion.rs`, `compose/tests.rs`), Claudine regression coverage (`cli/tests/compose_cli.rs`, `lib/src/composition/prepare.rs`), required docs (`composition.md`, `fm-interpolation.md`, `fm-shell-expansion.md`), the skill note (`SKILL.md`), and this `plan.md`. No stray files; surgical.

**Acceptance-criteria map (spec.md):**
1. Whole-value `{{ ... }}` parse failures fatal regardless of `fail_fast` — ✅ `whole_value_parse_failure_is_fatal_without_fail_fast` + manual repro.
2. Whole-value `{{ ... }}` eval failures fatal regardless of `fail_fast` — ✅ Phase 2 strict-eval routing; covered by interpolation suite (unknown function / type / read-side error cases).
3. Whole-value `$()` parse/expansion failures fatal when shell expansion enabled — ✅ `leak_guard_*` + parse-time scan tests (114 green).
4. Malformed `spec_path` reproduction fails instead of appearing in effective frontmatter — ✅ manual repro + `compose_dry_run_malformed_whole_value_spec_path_aborts_without_leaking`.
5. Mixed-string / body interpolation leniency unchanged — ✅ `mixed_malformed_interpolation_stays_warning_without_fail_fast`, `leak_guard_ignores_plain_and_mixed_values`.
6. Composition docs state the strict contract and distinguish mixed/body warnings — ✅ Phase 5 docs.
7. `just test` passes in `darkmatter` and `claudine` areas — ✅ `claudine` fully green; ⚠️ `darkmatter` green except 6 pre-existing, terminal-color-mode-dependent `layout::page` failures unrelated to this fix (proven by stash re-run).

**Platform limitations.** Validated on macOS (Darwin) only; no Windows/Linux run available on this host. The fix is pure-Rust composition logic with no platform-specific code paths. The 6 darkmatter `layout::page` failures are environment-dependent (headless/non-real-terminal color detection), not portability defects in this change.
