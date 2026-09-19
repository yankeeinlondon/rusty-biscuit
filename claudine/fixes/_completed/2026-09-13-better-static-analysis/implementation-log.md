---
created: 2026-09-16
area: claudine
spec: ./spec.md
plan: ./plan.md
implementation_1: "2026-09-16T20:02:17-07:00"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/expression/parser.rs
    - claudine/cli/tests/wrap_compose_validation.rs
    - claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md
    - claudine/cli/tests/fixtures/nested_span_regression/commit.md
docs_updated_during_phase_1:
    - claudine/fixes/2026-09-13-better-static-analysis/spec.md
    - claudine/fixes/2026-09-13-better-static-analysis/plan.md
    - claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/tests/backslash_escape_spans.rs
    - darkmatter/dmls/tests/lsp_session.rs
    - darkmatter/lib/proptest-regressions/markdown/compose/expression/lint.txt
docs_updated_during_phase_2:
    - claudine/fixes/2026-09-13-better-static-analysis/plan.md
    - claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/dmls/src/overlay/frontmatter.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/expressions/frontmatter_inventory_tests.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/graph/substrate.rs
    - darkmatter/dmls/tests/mapping_only_corpus.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_agent-skills.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_docs.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_implement___fm.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_interactive-prompting.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_repo-context.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__observability.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__performance-review.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-implementation.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__suggestion-review.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_writing-clearly.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/brainstorm.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/daily.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/dependency-risk.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/design.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/dream.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/feature.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/lineage.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/sentrux.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/snippets__debug.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/snippets__test-rigor.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/summary-and-suggest.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/validate.md
    - darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md
    - darkmatter/docs/schemas/claudine-types.yaml
docs_updated_during_phase_3:
    - claudine/fixes/2026-09-13-better-static-analysis/plan.md
    - claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/validation.rs
    - claudine/lib/src/composition/lifecycle/tests/diagnostics.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/diagnostics/snapshot/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/tests/wrap_compose_validation.rs
    - darkmatter/dmls/src/diagnostics/nested_span.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
docs_updated_during_phase_4:
    - darkmatter/dmls/docs/diagnostics.md
    - claudine/fixes/2026-09-13-better-static-analysis/plan.md
    - claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - claudine/cli/tests/shipped_prompt_contract.rs
    - prompts/_reviews/review-spec-inline.md
    - prompts/_interactive-prompting.md
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/error/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/nested_span.rs
docs_updated_during_phase_5:
    - claudine/docs/topics/lifecycle.md
    - claudine/docs/topics/composition.md
    - claudine/lib/README.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/dmls/docs/diagnostics.md
    - claudine/fixes/2026-09-13-better-static-analysis/plan.md
    - claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/lifecycle.md
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - claudine/fixes/2026-09-13-better-static-analysis/plan.md
    - claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
    - darkmatter/lib/src/markdown/compose/expression/parser.rs
    - claudine/cli/tests/wrap_compose_validation.rs
    - claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md
    - claudine/cli/tests/fixtures/nested_span_regression/commit.md
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/tests/backslash_escape_spans.rs
    - darkmatter/dmls/tests/lsp_session.rs
    - darkmatter/lib/proptest-regressions/markdown/compose/expression/lint.txt
    - darkmatter/dmls/src/overlay/frontmatter.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/expressions/frontmatter_inventory_tests.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/graph/substrate.rs
    - darkmatter/dmls/tests/mapping_only_corpus.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_agent-skills.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_docs.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_implement___fm.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_interactive-prompting.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_repo-context.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__observability.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__performance-review.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-implementation.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__suggestion-review.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/_writing-clearly.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/brainstorm.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/daily.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/dependency-risk.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/design.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/dream.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/feature.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/lineage.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/sentrux.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/snippets__debug.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/snippets__test-rigor.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/summary-and-suggest.md
    -  darkmatter/dmls/tests/fixtures/mapping_only_corpus/validate.md
    - darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md
    - darkmatter/docs/schemas/claudine-types.yaml
    - claudine/lib/src/composition/lifecycle/source_map.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/src/composition/lifecycle/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/tests/validation.rs
    - claudine/lib/src/composition/lifecycle/tests/diagnostics.rs
    - claudine/lib/src/composition/lifecycle/context/tests.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/diagnostics/snapshot/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - darkmatter/dmls/src/diagnostics/nested_span.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - claudine/cli/tests/shipped_prompt_contract.rs
    - prompts/_reviews/review-spec-inline.md
    - prompts/_interactive-prompting.md
documentation:
    - claudine/fixes/2026-09-13-better-static-analysis/spec.md
    - claudine/fixes/2026-09-13-better-static-analysis/plan.md
    - claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md
    - darkmatter/dmls/docs/diagnostics.md
    - claudine/docs/topics/lifecycle.md
    - claudine/docs/topics/composition.md
    - claudine/lib/README.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/lsp/features.md
    - .claude/skills/claudine/lifecycle.md
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
    - .claude/skills/darkmatter/compose.md
completed_phase: "6"
implemented: true
packages: []
human_review: true
human_review_items:
    - >-
        Acceptance 19 (L1 green on Linux, native Windows, WSL2) is unverified:
        no CI run exists because this session may not push, and all three local
        rigs were unavailable (build-linux stale nightly-reward-spike lock from
        2026-09-14, build-win-native W: at 0 GB free, build-win SSH refused).
        The author must push and confirm CI, or restore a rig, before treating
        the fix as fully accepted; the plan's Cross Platform task is left
        unchecked for this reason.
    - >-
        Rig maintenance needs a human decision: whether the 2026-09-14
        nightly-reward-spike lock on build-linux
        (~/ci-verification/.cross-check.lock, no live process) may be removed,
        and freeing space on build-win-native W:. Agents must not delete another
        effort's lock or data.
    - >-
        spec.md drift for the author to rule on: acceptance 3 still calls the
        prompt repair outstanding; acceptance 8 / D1 describe array suggestion
        suppression made obsolete by 61f085043; D2 says authored text is
        attached to LifecycleExpressionSurface, while the implementation looks
        it up fail-closed from LifecycleSourceMap by property path (while/until
        also spelled in two files); the memo is named ShapeMemo, not
        nested_shape_for_completion.
message_to_agent: >-
    Phase 6 (final) is validation-only: no source, test, doc, or skill file
    changed; only plan.md and implementation-log.md. macOS gates are green
    (darkmatter just test 7922 passed/7 skipped; claudine just test 7193
    passed/9 skipped; both lints pass). Cross-OS evidence (acceptance 19) was
    NOT obtained: pushing is out of scope so no CI run exists; build-linux is
    blocked by a stale-looking 2026-09-14 nightly-reward-spike cross-check lock
    (left in place); build-win-native W: has 0 GB free; build-win (WSL2) refuses
    SSH. The plan's Cross Platform task is intentionally left unchecked. If CI
    goes red only on wsl2-ubuntu, first suspect the five new runtime
    env!("CARGO_MANIFEST_DIR") fixture reads listed in implementation-log.md ->
    Phase 6 -> Cross Platform. spec.md drift (acceptance 3 note, acceptance 8
    array suppression, D2 attach wording, memo name) is recorded for the author,
    not edited.
---

# Implementation Log: Make Expression Defects Visible Before Execution

## Phase 1

Phase 1 is baseline evidence only. It changes no production behavior. The
full evidence is recorded in `plan.md` under **Verification Record → Phase 1**;
this log records what was done and what it means for later phases.

### Fixture capture

- Captured `prompts/_reviews/review-spec-inline.md` and `prompts/commit.md`
  from `cd6e036c4^` (`1fe739f34`) into
  `claudine/cli/tests/fixtures/nested_span_regression/`. `git hash-object`
  matches the pre-fix blobs (`69b690331…`, `dad40813a…`), so the copies are
  byte-identical, including both quote styles, the raw `\n`, and the
  array-valued span.
- The working-tree edits to `prompts/_reviews/review-spec-inline.md` and
  `prompts/plan.md` were left untouched.
- Added the guard test
  `wrap_compose_validation::nested_span_regression_fixtures_preserve_pre_fix_defects`.
  It is pure: it reads the fixtures and spawns nothing. It fails if anyone
  "repairs" a fixture, because that would make later rejection tests vacuous.

### Failing-before proof on unchanged production code

I built `target/debug/claudine` from the worktree and ran it in a hermetic
temp workspace. The `claude` stub wrote an invocation marker, and audio was
set to `PLAYA_DRY_RUN=1` with a private spool.

- `compose --dry-run` exits 0 and lists `success`/`failure` as event-time
  deferred, so prepare accepts the incident.
- A real run writes the marker, which means the provider was spawned. It then
  exits 1 with the original `lifecycle evaluation error`: the surviving
  `{{ctx.repo_name}}` and the misleading "resolve the missing path" hint. This
  reproduces the 2026-09-13 incident exactly.

### Assumption checks

- **Raw literal-block indentation: passed.** I added
  `parser::tests::raw_literal_block_indentation`, 2 tests over all three
  fixture `|-` blocks. The raw and decoded inner text produce equal ASTs in
  both parse modes, and the spanned root covers the trimmed raw text. The
  DMLS raw-coordinate projection design stands, so no spike was reopened.
- **Companion fix:** `61f085043` is an ancestor of HEAD, and no
  aggregate-suggestion suppression or `_with_types` API exists.
- **Event inventory:** the set of seven events is the same in the schema and
  in `LifecycleSignal::ALL`, but the order differs. `when`/`while`/`until` are
  declared in `claudine-types.yaml` as `string`.

### Findings that change later phases

1. `iter_stack_expression_surfaces` yields **stack surfaces only**.
   - Event-level communication fields (`success.say`, the incident surface)
     are raw `Option<String>` on `LifecycleNotification`.
   - `loop.while`/`loop.until` live on `LoopConfig`.
   - Phase 4's "attach to the existing surface, no second walker" instruction
     therefore requires extending that iterator (or its caller) to parse and
     yield whole-value communication fields and loop predicates. Otherwise the
     validator cannot see the incident.
2. `validate_no_undefined_lifecycle_variables` has no production call site
   either, not only `validate_no_interpolation_leaks`. The only production
   lifecycle validator is `validate_no_err_in_no_error_events`
   (`prepare.rs:533`, `:671`).
3. The Phase 3 "Predicate Schema" edit belongs in
   `darkmatter/docs/schemas/claudine-types.yaml`, not `claudine.yaml`.
4. `is_pending_expression_value` does not exist yet. The logic is inline in
   `schemas/format.rs:243–253`.
5. `expression_values` lives in `dmls/src/providers/frontmatter.rs:896`, not
   `overlay/expressions.rs`.
6. No mapping-only DMLS corpus exists. I identified 25 sequence-free
   documents at HEAD `3df43cc67`, listed in the plan. Phase 3 must freeze them
   and capture the baseline before editing `lower_mapping`.
7. `ExpressionFinder` impact is `UNKNOWN` in GitNexus. A text search shows 24
   `find_all_plain` call sites, so treat the Phase 2 scanner change as broad.

### Requirement-to-test mapping (Phase 1)

| Requirement | Test / evidence |
| --- | --- |
| Fixtures come from `cd6e036c4^`, byte-identical | blob-hash comparison (plan record) + `nested_span_regression_fixtures_preserve_pre_fix_defects` |
| Incident is accepted before the fix | manual hermetic dry-run and stub-run probes (plan record) |
| Raw literal-block indentation is expression whitespace | `raw_literal_block_indentation::{review_spec_inline_say_blocks_parse_identically_raw_and_decoded, commit_resides_in_block_parses_identically_raw_and_decoded}` |
| Companion-fix state | `git merge-base --is-ancestor 61f085043 HEAD` + `rg` |

### Gates run

- **Focused tests:** `cargo nextest run -p darkmatter --lib raw_literal_block_indentation` passed 2/2. `cargo nextest run -p claudine-cli --test wrap_compose_validation nested_span_regression` passed 1/1.
- **`just test` in `claudine/` (`--no-fail-fast`):** 7161 run, 7160 passed, 1 failed, 9 skipped.
  - The failure is `claudine-cli::dispatch_inventory::dispatch_inventory_matches_committed_file`. It is caused by **uncommitted, concurrent shadow-home edits** to `claudine/lib/src/provider_overlay/*`, which another session made during this phase. Its new test code adds a `Codex` reference that the committed inventory lacks. Regenerating the inventory confirmed that this is the only drift. I restored the committed file because that work owns the update.
  - In the first run, the `provider_overlay::tests::lease::{a_sweep_never_reclaims_a_root_marked_retained, a_failed_write_back_retains_the_root_with_a_marker_the_sweep_honors}` tests failed while those files were being edited. They passed 8/8 in isolation and in the complete rerun.
  - Every test touched by this phase passes.
- **Lint:** `just lint` in `claudine/` passed, and `just lint` in `darkmatter/` passed. Both run clippy with `--all-targets -D warnings`.
- **Cross-OS:** not run. Phase 1 adds only `include_str!` with `/` separators and `.lines()` parsing over LF-normalized fixtures (`.gitattributes` `eol=lf`), so there is no OS-specific surface. CI covers it.

## Phase 2

### Escape Scanner (D8)

- `ExpressionFinder::scan` now skips a `{{` or `{{{` opener that follows an
  odd-length run of backslashes, advancing past the whole brace run so
  `\{{{ x }}}` is neither a literal nor an expression. An even run leaves the
  opener active. Nothing is consumed. Both `scan` (Markdown-aware) and
  `scan_plain` go through the same loop, so all 24 `find_all_plain` callers and
  DMLS body scanning inherit the rule.
- **Deliberate non-change:** the brace-depth counter inside an open span
  (`scan_legacy_expression`) still counts an escaped inner `{{`. Making it
  honor the escape would split `{{ 'a \{{ b }}' }}` at the inner `}}`. Pinned by
  `escaped_inner_opener_does_not_change_depth_counting`.
- Tests:
  - `darkmatter/lib/src/markdown/compose/expression/lexer.rs` → `tests::backslash_escape` (11 tests). **Failing-before:** 4 failed against the unchanged scanner (`single_backslash_suppresses_opener_in_both_scan_modes`, `escaped_literal_opener_is_neither_literal_nor_expression`, `escaped_span_does_not_hide_a_following_span`, `backslash_run_parity_decides_the_opener`). The other 7 pin unchanged behavior: `\{\{`, unescaped spans, unrelated backslashes, even-run literal, unclosed opener, fenced code, and inner depth.
  - `darkmatter/lib/tests/backslash_escape_spans.rs` (6 compose tests through `Markdown::compose`: output bytes plus `report.warnings`). **Failing-before**, measured by disabling `is_escaped_opener`: 3 failed (the escaped opener, parity, and the escaped single-pipe examples). 3 pass either way: `\{\{`, the unescaped span, and unrelated backslashes.
  - `darkmatter/dmls/tests/lsp_session.rs` → `backslash_escaped_foreign_template_examples_have_no_expression_diagnostics`. This is the real LSP session over `didOpen`. The unescaped control case yields `dm.expression.malformed`. **Failing-before:** with the check disabled, the escaped document produced two `dm.expression.malformed` diagnostics.

### Lint Model and Rewrite Generator (D1)

- Added `darkmatter/lib/src/markdown/compose/expression/lint.rs`, re-exported
  from `expression`: `ExpressionLint`,
  `ExpressionLintKind::NestedSpanInStringLiteral { literal, nested }`,
  `lint_expression(source, mode)`, `lint_spanned(source, expr)`, and
  `is_whole_value_span(text)`. As the plan requires post-companion, it has no
  `DeclaredExpressionValueKind` and no `_with_types` variants; arrays and
  objects get suggestions.
- **One whole-value classifier.** `whole_value_span` moved from
  `interpolation/rewrite.rs` into `lint.rs` as `pub(crate)`.
  `interpolate_value` now calls it there, and the public
  `is_whole_value_span` delegates to the same function.
- **Walk.** Every authored `StringLiteral` is visited, including quoted object
  keys; bare identifier keys carry no quotes and are skipped.
  `find_all_plain` runs over the raw slice between the quotes, producing one
  lint per nested span. `lint.span` is the nested `{{ … }}` byte range in the
  expression source. A parse failure yields `[]`.
- **Suggestion shape (decision).** A lint's suggestion is one rewrite of the
  *whole expression* that repairs every flagged literal, so all lints from one
  expression carry the same text. A single edit then fixes the value, and a
  partially repaired rewrite is never offered.
- **Rewrite rules as implemented:**
  1. Literal pieces are re-emitted raw in the original quote. The lifted span
     is decoded by running the lexer itself over `quote + text + quote`, so
     there is no second copy of the escape table.
  2. A non-atomic lifted AST is parenthesized.
  3. **Per-span anchor:** `("" + span)`, using the author's quote. It is relaxed
     to a bare span only once some earlier piece's decoded text contains a
     character that no `f64` string can contain (anything outside
     digits/`.+-`/`e i n f t y a` in either case). The spec's "the piece cannot
     parse as a number" is not sufficient, because `"1" + "e5"` is `"1e5"`. The
     adjacent-span test `"1e{{a}}{{b}}"` pins this.
  4. A multi-part chain is parenthesized when its parent binds tighter than
     `+`: unary, arithmetic `Binary`, or an `Index`/`MemberAccess` base.
- **Self-validation is AST equivalence, not re-parse.** The generator builds
  the intended `Expr` alongside the text: the host tree with each literal
  replaced by its `+` chain, and lifted spans parsed with the *interpolation*
  grammar a rescanning pass uses. The suggestion is offered only if the
  rewrite re-parses to exactly that tree in every candidate dialect, **and**
  the rewrite has no remaining nested-span lint. `lint_spanned` recovers the
  candidate dialects by re-parsing the source in both modes and keeping the
  modes that reproduce `expr`.
- **Suggestion is `None` for:**
  - a malformed lifted span;
  - a quoted object key, since a key cannot be a `+` chain;
  - a lifted `||` inside a condition-dialect host, where fallback would become
    boolean OR;
  - a doubly nested literal;
  - a literal that also holds a `{{{ … }}}` escape, which only rescanning
    converts;
  - an undecodable split escape.
- **Known edge (not fixed; recorded):** nested-span detection runs on the raw
  slice, as specified. A literal authored as `'\\{{ x }}'` decodes to
  `\{{ x }}`, which the runtime scanner treats as escaped, but the raw slice
  shows an even run and is flagged. Only `\\` changes parity between raw and
  decoded text; a real document has no reason to write this.
- **Tests:** `expression::lint::tests` (26).
  - `lint_model` (7): both quote styles, span/literal byte ranges, both parse
    modes, ternary/function-argument/array/object key and value positions, the
    quiet cases (`"a " + b + " c"`, `{{{ }}}`, unclosed, escaped, no literal),
    parse failure, `lint_spanned` == `lint_expression`, and the
    `is_whole_value_span` shape.
  - `rewrite_generator` (19):
    - the incident success branch rewrites exactly to the spec string (via the
      `include_str!` fixture and real frontmatter parse), and its single-pass
      output equals the rescanned output with no braces;
    - the failure branch lifts function calls;
    - `commit.md` keeps single quotes and raw `\n`, and the array span gets a
      suggestion;
    - explicit tree equality for the ternary;
    - evaluated-output equivalence (single-pass rewrite vs `interpolate_text`
      rescan) for ternary, additive, adjacent (`{{a}}{{b}}`, `{{a}}5`, `5{{a}}`,
      `1e{{a}}{{b}}`), span-only (typed `"7"` string), escaped quotes, raw `\n`,
      multiple literals, and array/object JSON;
    - the tight parent;
    - all six `None` cases;
    - condition-dialect reparse.
- **Failing-before:** the API did not exist, so the tests could not compile
  against unchanged code. Mutation evidence that the tests hold each rule: with
  parenthesization disabled, 4 tests fail (ternary, tree equality, additive,
  condition fallback); with anchoring disabled, 2 fail (span-only,
  adjacent-numeric).

### Pending Authority

- Added `pub fn is_pending_expression_value(value: &str) -> bool` in
  `darkmatter/lib/src/markdown/schemas/format.rs`, reachable as
  `darkmatter::markdown::schemas::format::is_pending_expression_value`. It is
  lexical only: the value contains `$(` or `{{`. The `darkmatter-expression`
  format validator now calls it instead of its inline check, so behavior is
  byte-identical. `schemas/mod.rs`'s `scan_pending_values`/`pending_reason`
  classify a whole `Value` with shell-over-template precedence for
  compose-time deferral reporting. That is a different question, so it was
  left alone.
- Tests in `schemas::format::schema_plus_content_formats`:
  - `pending_expression_values_are_classified_lexically` covers `{{`, `$(`,
    malformed-but-pending values, and near-misses (`$ (x)`, `{ {x} }`,
    `{x: 1}`).
  - `expression_validation_defers_exactly_on_pending_values` goes through the
    real convert → coerce → validate path for `when: expression`. Pending
    malformed values are accepted, a malformed final value (`a ((`) is
    rejected, and the predicate agrees with each case.
- Failing-before: the predicate did not exist, so the tests fail to compile.
  Validation behavior is intentionally unchanged, which is a refactor to a
  shared authority.

### Authority Properties

- `expression::lint::tests::authority`:
  - `flagged_literals_equal_scanner_hits` (proptest, both dialects). The oracle
    is independent of the lint's tree walk: it takes the **lexer's**
    `StringLiteral` token spans (`lex_spanned`) and runs `find_all_plain` over
    each raw slice. Literals are built from fragments: `{{ x }}`, lone
    `{{`/`}}`/`{`, `\`, `\\`, and `{{{ x }}}`. They are placed in shapes
    covering the ternary, call, array, object key/value, `+`, index,
    unary-minus, and `!`/`||` positions.
  - `whole_value_classifier_matches_interpolate_value_branch` (proptest).
    `interpolate_value`'s typed branch is observed as a non-string value or a
    hard error (a malformed whole value is fatal even with
    `fail_fast = false`); its text branch returns a string. Inputs combine
    `{{ 1 }}`, `{{ ( }}`, `{{{ 1 }}}`, whitespace, `\`, and lone braces.
  - `rescanning_body_still_resolves_what_the_lint_describes` covers the
    negative case: `{{ pkg ? 'in {{pkg}}' : 'x' }}` in a body composes to
    `in darkmatter` with no warnings, while `lint_expression` still describes
    one nested span.
- **Mutation evidence** that the properties bite:
  - skipping object-key literals fails `flagged_literals_equal_scanner_hits`.
    This happened only after the generator was changed to fragment-based
    literals; the first random-character generator missed it.
  - skipping index-position literals fails the same test;
  - a classifier that also accepted `{{{ 1 }}}` fails the whole-value
    property.
- `rescans_replacement_text_for_nested_interpolation` and
  `rescans_false_branch_for_nested_interpolation` are unmodified. The only
  `rewrite.rs` diff is the moved `whole_value_span` and its import. They pass.
- Focused run: `cargo nextest run -p darkmatter --lib -E 'test(/expression::/) | test(/interpolation::/) | test(/schemas::format/)'` passed 979/979.

### Graph analysis

- GitNexus upstream impact:
  - `ExpressionFinder.scan` → `UNKNOWN` (unresolvable callers). Confirmed by
    Phase 1's text search: 7 Darkmatter and 17 Claudine `find_all_plain`
    sites. The escape is therefore treated as broad, and both packages' full
    L1 suites were run.
  - `interpolate_value` → MEDIUM (15 impacted, exact). The logic is unchanged
    because `whole_value_span` was only moved.
  - `whole_value_span` → LOW.

### Requirement-to-test mapping (Phase 2)

| Requirement | Test(s) |
| --- | --- |
| `\{{` / `\{\{` opt out; odd/even parity; both scan modes | `lexer::tests::backslash_escape::*` |
| Backslashes preserved through compose; unescaped still interpolates; unrelated backslashes untouched | `tests/backslash_escape_spans.rs` (6) |
| Escaped foreign template examples warning-free (compose + DMLS diagnostics) | `escaped_foreign_template_examples_produce_no_expression_warnings`, `lsp_session::backslash_escaped_foreign_template_examples_have_no_expression_diagnostics` |
| Lint fires once per nested span, all positions, both quotes/modes; quiet cases; parse failure → none | `lint::tests::lint_model::*` |
| Incident suggestion exact; `commit.md` quotes/`\n`/array rewrite | `incident_success_branch_rewrites_exactly`, `incident_failure_branch_lifts_function_calls`, `commit_fixture_keeps_single_quotes_raw_newline_and_offers_array_rewrite` |
| Semantic equivalence on evaluated output (ternary, additive, adjacent, suffix, span-only, escaped quotes, raw `\n`, multiple, aggregates) | `lint::tests::rewrite_generator::*` |
| AST equivalence, not re-parse | `rewrite_tree_equals_the_intended_tree` + internal equivalence gate |
| Malformed span → lint without suggestion | `malformed_nested_span_has_no_suggestion` (+ key/condition/doubly-nested/literal-escape `None` cases) |
| Shared pending predicate | `pending_expression_values_are_classified_lexically`, `expression_validation_defers_exactly_on_pending_values` |
| Lint == `find_all_plain` over literal slices | proptest `flagged_literals_equal_scanner_hits` |
| `is_whole_value_span` == `interpolate_value` branch | proptest `whole_value_classifier_matches_interpolate_value_branch` |
| Rescanning surfaces unchanged | `rescans_*` (unmodified), `rescanning_body_still_resolves_what_the_lint_describes` |

### Gates run (Phase 2)

- Focused: `cargo nextest run -p darkmatter --lib -E 'test(/expression::/) | test(/interpolation::/) | test(/schemas::format/)'` → 979/979.
- `just test` in `darkmatter/` → 7871 passed, 7 skipped, exit 0.
- `just test` in `claudine/` stopped at the first failure (fail-fast). The
  rerun with `just test --no-fail-fast` gave **7168 run: 7166 passed, 2
  failed, 9 skipped.** Both failures are pre-existing or concurrent and out of
  scope:
  - `claudine-cli::dispatch_inventory::dispatch_inventory_matches_committed_file`:
    sites `1611 → 1613`. The same failure was recorded in Phase 1. It is
    caused by another session's uncommitted `claudine/cli/src/commands/wrap/*`
    and `provider_overlay` edits.
  - `claudine-cli::test_placement::repository_test_placement`:
    `lib/src/mcp/inject.rs` has 349 inline test lines, over the 300 limit.
    That file is also modified by the concurrent session. It is new since
    Phase 1.
  - This phase edits no Claudine file. The Claudine suites that consume the
    scanner (`wrap_compose_validation`, lifecycle, compose) pass.
- `just lint` in `darkmatter/` → pass. `just lint` in `claudine/` → pass. Run
  serially.
- Cross-OS: not run (string-only change); see the plan record.
- Skills: no Claudine or Darkmatter skill change. Phase 2 adds library APIs
  only, and the authoring-guidance update (escape syntax, `+` form) is
  scheduled in Phase 5 → Skill Guidance.
- `darkmatter/lib/proptest-regressions/markdown/compose/expression/lint.txt`
  was generated during the mutation runs. It holds three shrunk seeds: a
  quoted object key, an index-position literal, and a lone `{{{ 1 }}}`. They
  pass against the real implementation and are kept as regression seeds, as
  proptest recommends.

## Phase 3

### Mapping-only corpus baseline (captured before any overlay edit)

- Froze the 23 sequence-free `prompts/` documents named in the Phase 1
  record from `git show 3df43cc67:prompts/<path>.md` into
  `darkmatter/dmls/tests/fixtures/mapping_only_corpus/` (`/` → `__`), plus the
  existing `suggest_constraint/{inline,raw-consumer}.md` fixtures (25 total).
- `darkmatter/dmls/tests/mapping_only_corpus.rs` runs the full provider
  registry's diagnostics with the shipped Claudine extension activated and
  compares normalized JSON to `baseline.json`. The baseline was blessed on
  the **unchanged** overlay (`DMLS_BLESS_MAPPING_CORPUS=1`; a bless run still
  fails) and the test passed on the unchanged code. A companion test asserts
  no corpus document contains a sequence, so descent cannot reach them.

### Graph analysis (pre-edit)

GitNexus upstream impact (index at `3df43cc67`): `FmEntry` LOW (4),
`lower_mapping` LOW (5), `nested_shape_for_completion` LOW (20),
`nested_shape` LOW (7), `expression_values` LOW (8; Phase 1 recorded HIGH
under an earlier index — treated as HIGH per the plan gate regardless),
`def_at_path_ctx` **MEDIUM** (27). `FmEntry.key_span` field reads are not
resolved by the graph, so every consumer was found by text search (25 sites
across `overlay`, `providers`, `diagnostics`, `graph/substrate.rs`) and
adapted.

### Sequence Arena (D6)

- `overlay/frontmatter.rs`: `FmEntryRole { MappingProperty, SequenceItem {
  index } }`, `key_span: Option<SourceSpan>` (`Some` exactly for properties),
  `scalar_style: Option<FmScalarStyle>` (Plain/SingleQuoted/DoubleQuoted/
  Literal/Folded) and `tagged: bool`. Lowering is `lower_mapping` →
  `lower_entry`, which recurses through mappings **and** sequences, emitting
  one entry per item (`key` = decimal index, pointer `/tags/1`, dotted
  `tags[1]`) before that item's children.
- **Finding:** `rlsp-yaml-parser` resolves a Core-schema tag onto every
  untagged node, so `Node::Scalar.tag.is_some()` is always true. Tag
  authorship is `NodeMeta.tag_loc.is_some()`. A tagged scalar's value span
  excludes the tag token (pinned by test).
- Key-specific consumers now require `Some(key_span)`: folding (item
  collections do not fold; properties nested in items do), document symbols,
  `ctx`/schema/type-definition hovers, style-key diagnostics, the
  migrate-style-key action, DSL interpolation definition, substrate key facts,
  `key_entry_on_line`, `still_placed` (an item defers to its nearest keyed
  ancestor), and `container_at_offset` (an item sequence has no key line).
- Pre-descent parity kept deliberately where a consumer's contract is about
  mapping properties: `frontmatter_authoring` (semantic schema values) and
  `nested_property_value_span` skip `is_in_sequence` entries, because the
  semantic parsers already locate sequence interiors through
  `locate_schema_value`; `passive_namespace_names` and
  `inline_schema_file_keys` accept only mapping properties.
- **Deliberate assertion change:**
  `test_entry_or_ancestor_falls_back_for_array_index` was re-cut and renamed
  `test_entry_or_ancestor_is_exact_for_array_item` — `/tags/1` is an exact
  scalar hit; an absent `/tags/5` still falls back to `/tags`.

### Path Semantics

- `FmPathSegment::{Key(&str), Index(usize)}`, `FrontmatterAst::path_at` /
  `path_of` / `is_in_sequence`, and the single dotted formatter
  `format_dotted` (lowering uses the same `push_dotted`), so an index renders
  `[0]` and a key named `0` stays `.0`. `key_path_at` still returns decimal
  indices as strings (pointer identity); its docs now direct schema
  resolution to `path_at`.
- `providers/frontmatter.rs` has one traversal:
  - `nested_shape(root, &[FmPathSegment])` (context-free): a key followed by
    one index descends an array arm's item shape; a bare key descends a
    non-array arm; a leading index, an index into a non-array, a key into an
    array-only property, or a nested index fail closed.
  - `ShapeMemo` + `def_at_path_ctx(ctx, root, path, memo)` (context-aware):
    discriminant selection over array-ness-admissible arms (inadmissible arms
    project to `{}` so selector indices stay aligned), merged-arm fallback
    retained, `navigate_json` indexes arrays, and a trailing index resolves
    the array arms' item definition.
  - `ArrayCrossing::Explicit` for addressed entries; `Transparent` for
    completion. **Design decision:** completion's authoring ancestry stays
    key-only (`enclosing_path` drops indices) because it must agree with the
    indentation fallback, which cannot know an item index, and because a
    `- ` line authors an item of the array above it. `Transparent` is exactly
    the pre-descent walk, so completion ancestry is unchanged.

### Predicate Schema

`darkmatter/docs/schemas/claudine-types.yaml`: `lifecycle-stack-item.when`,
`loop-event.while`, `loop-event.until` are now `expression`. Verified the
Claudine runtime never loads these files (only DMLS and Darkmatter tests do),
so this is editor-scoped. The shipped-schema classification corpus
(`overlay::tests::shipped_schema_corpus_uses_content_classification`) stays
`ready`, and `lib/tests/base_schema_end_to_end.rs` stays green.

### Shape Memo

`ShapeMemo` memoizes per typed ancestor path (prefixes included) for one pass
and is dropped with it; the walk borrows `root` instead of cloning it.
`expression_values` and `nav_targets` share one memo per pass. `index_of`
carries the quadratic-shape warning. Review of every production
`entries()` loop (`overlay/schema.rs:219`, `diagnostics/frontmatter.rs:472`,
`graph/substrate.rs:526`, `providers/frontmatter.rs:503/911/1408/1436`,
`providers/dsl.rs:934`): none calls `key_path`, `path_of`, `index_of`,
`entry_by_pointer`, or `entry_by_dotted` internally.

### Capability Boundaries

- Hover: a list-item key hovers its schema definition (key-ranged); a
  list-item expression value gets the expression hover; a scalar item hovers
  its item definition labeled with its dotted path (`tags[0]`) and ranged on
  the value. **Deliberate behavior change:** a cursor on a sequence's `- `
  markers, or on an item that is itself a collection, is now silent (it
  previously showed the enclosing sequence property's hover).
- Completion: keys inside item mappings and list-item predicate values
  (the old lexical fallback read the key as `- when`) now complete; no
  synthetic index is ever offered.
- Navigation: `file[]` items are link/definition targets; the marker and key
  are not.

### Interpolation Inventory

- `overlay/expressions.rs`: `ScalarProjection` (Plain / Literal / Quoted /
  Whole) with `project`, `range`, `analyzed_offset`, `is_exact`, and
  `encode_fragment` (single: `'`→`''`; double: `\\`, `\"`, `\n`, `\t`, control
  chars refused; plain: refuses `: `, ` #`, trailing `:`, line breaks, tabs,
  and flow indicators; literal: verbatim). Exactness additionally requires
  the decoded text to equal the parser's value and the raw slice to be a
  single line, so folding can never slip through `DecodedScalar`.
- `frontmatter_interpolations(document, ast)` → `FrontmatterInterpolation {
  entry, text, whole_value, … }` with `outer_span`, `inner_span`, `project`,
  and `nested_span_lints(document, mode)` → `ProjectedNestedSpan { range,
  nested, replacement }`. Literal blocks scan the raw slice after the header
  line (a header comment is never scanned); folded/tagged/multi-line/mismatch
  scalars analyze the decoded value and range the whole scalar. The rewrite is
  suppressed when the flagged literal's authored slice contains a line break.
- `ExpressionValue` now carries this projection. The malformed-expression
  producer fires only for exact values, and `schema_problem_diagnostics`
  suppresses the generic problem only for exact values, so block/tagged/
  multi-line values keep the schema error rather than a fabricated
  expression range. Expression hover requires an exact value.

### Failing-before evidence (Phase 3)

Each production change was neutralized in place and the new tests re-run
(toggle script restores the file afterwards; the full suite was green again
after every toggle):

| Neutralized change | Tests that failed |
| --- | --- |
| Sequence descent (`items…take(0)`) | 17: all 6 new arena tests, 2 inventory (`aliases_items…`, `commit_fixture…`), `array_item_schema_problem_ranges_the_failing_item`, 8 `sequence_tests` (hover key/value + scalar item, navigation, discriminated arm, predicates reach values, list-item diagnostics, memo, completion) — the mapping-only corpus stayed green |
| Malformed style gate + exact-only suppression | `malformed_expression_producer_is_gated_to_exactly_projected_styles` |
| Synthetic-position hover rule | `hover_is_silent_on_synthetic_item_positions` |
| Memo lookup | `expression_pass_materializes_each_distinct_ancestor_shape_once` |
| Array-ness arm admission | `nested_shape_fails_closed_on_mismatched_segments`, `union_arms_are_chosen_by_array_crossing`, `transparent_crossing_serves_key_only_authoring_ancestry` |
| Predicate typing reverted to `string` | 7 `sequence_tests` (index traversal, predicates typed, predicates reach values, list-item diagnostics, list-item hover, memo, completion) |
| Single-quote encoding + newline suppression + single-line exactness | `quoted_rewrites_are_encoded_for_their_scalar_style`, `literal_spanning_a_line_break…`; the multi-line case stayed green because the decoded-equals-value check independently rejects folding (defense in depth) |

`completion_never_offers_synthetic_item_indices` is a negative guard and
passes both before and after by design.

### Requirement-to-test mapping (Phase 3)

| Requirement | Test(s) |
| --- | --- |
| Item entries: role, `key_span == None`, index key, pointer, dotted, value span, parent, depth; mappings keep role and exact key spans | `overlay::frontmatter::tests::{test_sequence_items_are_explicit_entries, test_mapping_items_descend_to_their_keys, test_top_level_entries_and_spans}` |
| `/tags/1` exact hit (deliberate re-cut) | `test_entry_or_ancestor_is_exact_for_array_item` |
| Array problems range the failing element | `diagnostics::frontmatter::tests::array_item_schema_problem_ranges_the_failing_item` |
| Typed segments, `[0]` vs `.0`, nested arrays, RFC 6901 escapes, `initialize.stack[0].when` | `test_index_and_numeric_key_paths_stay_distinct`, `test_mapping_items_descend_to_their_keys`, `sequence_tests::nested_shape_consumes_array_index_against_item_type` |
| Fail closed; union selection; merged fallback; array-ness arm choice | `sequence_tests::{nested_shape_fails_closed_on_mismatched_segments, union_arms_are_chosen_by_array_crossing, context_aware_walk_selects_discriminated_arm_inside_array_items, transparent_crossing_serves_key_only_authoring_ancestry}` |
| Scalar style + tag retained (all block indicators, tag, alias) | `test_scalar_style_and_tag_are_retained` |
| Predicates expression-typed in the shipped schema; reached passively through array items | `sequence_tests::{shipped_claudine_predicates_are_expression_typed, predicates_inside_stack_items_reach_expression_values_without_executing}` (a `$(touch …)` predicate is reached, never run) |
| Memo: ≤ one materialization per distinct ancestor path on `implement-plan.md` | `sequence_tests::expression_pass_materializes_each_distinct_ancestor_shape_once` (fixture frozen from `3df43cc67`) |
| No quadratic lookup in entry loops | review, recorded above |
| Hover / completion / navigation positive at list items, negative at synthetic positions | `sequence_tests::{hover_works_at_list_item_key_and_value, hover_on_scalar_list_item_describes_the_item_type, hover_is_silent_on_synthetic_item_positions, completion_works_inside_list_items, completion_never_offers_synthetic_item_indices, navigation_works_at_list_item_file_values_only}` |
| Mapping-only diagnostics byte-identical | `tests/mapping_only_corpus.rs` (25 docs, baseline captured pre-edit) |
| Newly reachable malformed/unknown diagnostics inside list items | `sequence_tests::expression_diagnostics_newly_reach_list_items` |
| Inventory projection: plain/single/double exact, literal raw coordinates, folded/tagged/multi-line whole scalar, aliases silent, escapes honored | `overlay::expressions::frontmatter_inventory_tests::*` (9) |
| Incident + commit fixtures: nested spans ranged in authored bytes; one rewrite per expression; `|-` and indentation preserved; re-lint is clean; idempotent | `incident_literal_blocks_range_each_nested_span_in_authored_coordinates`, `incident_rewrite_preserves_block_indicator_indentation_and_untouched_bytes`, `commit_fixture_literal_with_escaped_newline_is_single_line_and_rewritable` |
| YAML-style fragment encoding (read/write/read round trip) | `quoted_rewrites_are_encoded_for_their_scalar_style` |
| Newline suppression | `literal_spanning_a_line_break_keeps_its_range_but_offers_no_rewrite` |
| Malformed producer gated to exact styles; schema problem kept for `|`, `|-`, `|+`, `|2-`, `>`, `>-`, tagged, multi-line; alias silent | `diagnostics::frontmatter::tests::malformed_expression_producer_is_gated_to_exactly_projected_styles` |

### Gates run (Phase 3, macOS)

- Focused: `cargo nextest run -p dmls` → 696/696.
- `just test` in `darkmatter/` → **7904 passed, 7 skipped** (includes
  `no_side_effects`, `packaging_contract`, `lsp_session`,
  `base_schema_end_to_end`).
- `just lint` in `darkmatter/` → pass after fixing one `needless_borrow` in
  `providers/dsl.rs` (the DMLS suite was re-run green afterwards). The kache
  EXDEV line is an advisory copy fallback, not a failure.
- `just test --no-fail-fast` in `claudine/` → **7168 passed, 9 skipped, 0
  failed.** The two Phase 2 concurrent-session failures no longer reproduce.
- `just lint` in `claudine/` → pass.
- Cross-OS: not run on the rigs. Portability was reviewed instead: the tests
  use virtual `/w` roots only where no file URI is produced (navigation
  asserts resolved `Path` components because `/w` has no Windows file URI),
  the discriminated-arm test uses a real temp directory and a relative
  `@./types.yaml` import, the corpus test normalizes the manifest directory
  out of messages, and fixtures are LF by `.gitattributes`. CI supplies
  Linux, native Windows, and WSL2.
- Skills: no skill change. The DMLS arena and projection are internal; the
  authoring guidance (`+` form, escapes) is scheduled for Phase 5.

## Phase 4

Phase 4 was split by crate. The orchestrator (claude/opus) implemented the
Claudine consumers. A delegated sub-agent implemented the DMLS editor
diagnostic, quick fix, and severity ladder; its record is under
"DMLS consumers" below.

### Graph analysis (pre-edit)

GitNexus upstream impact:

- `prepare_direct_with_prompt` and `prepare_inline` are **HIGH** (18 and 17
  impacted). Every compose route prepares through them. This is expected:
  the new validator only adds a rejection for defective documents, and the
  full Claudine L1 suite is the regression gate.
- `iter_stack_expression_surfaces`, `reject_surviving_spans`, and
  `run_phase_1c_with_schema` are LOW.
- `validate_no_interpolation_leaks` is **UNKNOWN** (no resolved callers). A
  text search confirmed its only references were the `lifecycle/mod.rs`
  re-export and tests, so it was deleted as Resolved Implementation
  Determination 1 directs.

### Lifecycle Validator (D2)

- **`LifecycleSourceMap`** (`lifecycle/source_map.rs`): authored lifecycle
  scalars keyed by the exact property paths `iter_stack_expression_surfaces`
  emits.
  - It is built from the frontmatter object `parse_lifecycle_config` read.
    Lifecycle keys are excluded from compose, so their values are authored
    text.
  - Positional and key/value actions are named the way `action_shape.rs`
    stores them. Examples: `error`→`reason`, `proxy`→`target`,
    `retry`→`max_attempts`, `resume`→`message`, `defer`→`delay`,
    `shell`→`command`, communication verbs → `message`
    (`message`/`text`/`sound`), and signature verbs → `arg[k]`. For signature
    verbs, `k` counts only the parameters actually present.
  - `proxy … with` trees use the same `with.key[i]` spelling.
  - **Design finding:** the parsed stack keeps only `Expr` trees, so the
    authored text cannot be read back from a surface, and the mapping from
    surface to YAML depends on the verb. Rather than add a second walker, the
    validator looks up each surface the one canonical iterator yields. A miss
    is `LifecycleInvalid` with an "internal error" message (Determination 2),
    never a silent skip.
- `LifecycleExpressionSurface` gained `predicate: bool`. It is set only for
  `stack[i].when`, so a `with.when` overlay key is not mistaken for a
  predicate.
- **`validate_no_nested_spans_in_literals(frontmatter, lifecycle, path)`**
  walks `LifecycleSignal::ALL` in order. For each event it checks:
  1. every `LIFECYCLE_COMM_FIELDS` value, only when `is_whole_value_span`
     holds, linting the inner expression with `ParseMode::Interpolation`;
  2. `loop.while` / `loop.until`, linted whole with `ParseMode::Condition`;
  3. every stack surface: predicates in condition mode, everything else only
     when it is a whole-value span.

  The first Darkmatter `lint_expression` lint aborts. `literal` and `nested`
  are the raw source slices, and `suggestion` is Darkmatter's bytes,
  unchanged.
- It is wired into both `prepare_direct_with_prompt` and `prepare_inline`,
  immediately after `parse_lifecycle_config` and before shell preflight. Every
  entry reason reaches it through `prepare_document`: direct, proxy target,
  retry, resume, and the loop seed. The stale prepare-time comment claiming
  that no leak scan runs was corrected.
- **`CompositionError::LifecycleNestedSpanInLiteral`** `{ source_path,
  property, literal, nested, suggestion }` replaces the
  `LifecycleInterpolationLeak` variant, which only the deleted function
  produced.
  - It is registered in the lifecycle render family, the
    `composition.lifecycle_invalid` code, the detail projection, and the
    `FrontmatterHighlight::Property` excerpt.
  - It renders the header `nested interpolation inside a string literal`, the
    property, the nested span, the literal, and the illustrative rewrite
    ("expression shown without its `{{ }}` wrapper or YAML quoting"), with the
    `{{{ … }}}` hint.
  - The spec's `line: Option<usize>` was not added. The existing property
    highlight already lands the excerpt on the right line, as it does for
    every other lifecycle-property error.
  - **Bug found while testing:** `escape_prose_path` renders `"` as a visible
    `\"`. User text in this block uses `Prose::escape_text` instead, and a
    test pins that quotes render unescaped.
- **Deleted:** `validate_no_interpolation_leaks`,
  `find_matching_warning_reason`, the orphan doc block for it at the end of
  `action_shape.rs`, and the dead-entry-point tests:
  - `tests/validation.rs::validation_is_the_dispatch_gate_for_leaked_lifecycle`
  - `tests/diagnostics.rs`'s five "stack leak scan" tests

  Their useful coverage (every comm field, every event, stack surfaces) is
  recut through the new validator. `error/tests.rs` and
  `cli/src/output/error_walker/tests.rs` switched their excerpt-machinery
  example from the deleted variant to the new one.

### Runtime Backstop (D4)

- `LifecycleExprError::SurvivingSpan { span }` replaces the
  `Prose("unresolved interpolation survived …")` raise in
  `reject_surviving_spans`. Its `Display` is the spec's reason text.
- `CompositionError::LifecycleEvaluationReason { Expression, SurvivingSpan {
  span } }` is a new public enum.
  - `LifecycleErrorInfo` gained `property: Option<String>` and `reason`.
  - `from_error_or_action` recovers the reason by downcasting the typed error,
    so no message text is matched. Neither field is projected into `err.*`.
- The executor attaches the property with `LifecycleErrorInfo::at_property`
  at each site:
  - top-level fields: `{event}.{field}` (`say`/`say_first`/`effect` resolved
    per audio phase);
  - `when` guards: `{event}.stack[i].when`;
  - stack evaluation errors: `{event}.stack[i].action[j]`.
- `CompositionError::LifecycleEvaluationError` gained `property` and `reason`,
  filled by `lifecycle_evaluation`. The renderer names the property when
  present. It selects the concatenate/`{{{ … }}}` hint for `SurvivingSpan`,
  keeping the "resolve the missing path" hint only for `Expression`, and it
  renders the full typed reason rather than the length-clamped `msg`.
- **Observation:** a *stack action operand* whose value resolves to template
  text is re-expanded by `render_message` at event time. It fails there as an
  unknown-root `Expression` error, not a `SurvivingSpan`. The spec's "not
  re-expanded at event time" is exactly true only for top-level fields. The
  reason text was kept as specified. The existing
  `post_dm2_surviving_span_fails_before_dispatch` test asserts only the
  property for that reason.

### Referenced Prescan

- **Design finding:** Sequence Plus static preflight
  (`build_preflight_graph_with_invocation`) already resolves every referenced
  `prompt:` document through `resolve_sequence_reference_in_context` with each
  origin's `derive_source` `FileResolutionContext`. That is the snapshot
  execution reuses via `PromptTaskRequest.path`. It retains the parsed
  document in `PreflightGraph::prompt_documents`.
- The pre-scan is therefore
  `PreflightGraph::validate_prompt_lifecycle_literals()`. It parses each
  retained document's authored lifecycle and runs the same validator: no
  compose, shell, fetch, prompt, or new reference code.
  - A document whose lifecycle block does not parse is left to its own turn,
    where prepare reports that error.
  - The sequence document itself is covered by Phase 1c's per-step
    `compose_step` → prepare.
- **Placement deviation:** the call runs in `execute_sequence` immediately
  after the graph is built. That is before `approve_preflight_graph` and
  before `run_phase_1c_with_schema`, not inside Phase 1c. A defect is
  reported before any shell-approval prompt, and `--dry-run` performs the
  same walk.
- **Interpolated references:** the graph resolves `{{ENV}}`-style reference
  spans through the file-reference grammar (covered). A reference that needs
  frontmatter or runtime values is not resolvable by preflight today, so no
  such step reaches execution unvalidated. Invariant 4's first exception
  (catch at the step's turn) still holds through prepare.

### Spawn Prevention and failing-before evidence

The CLI process tests all use `CliProcessFixture::command()` (child-local
`PLAYA_DRY_RUN`, private spool, minimal `PATH`). A `goose` stub (plus a
`.cmd` on Windows) appends to `$PROVIDER_MARKER`, so "never started" is
asserted from the marker, not assumed. Each test also asserts the audio spool
stays empty.

- **Loop finding:** a document loop parses its lifecycle once in the seed
  preparation (`build_loop_seed_with_lifecycle` → `prepare_direct`) and
  reuses that stamped config for every iteration. A lifecycle defect written
  mid-loop therefore never executes. A first draft of a mid-loop re-entry case
  ran all three iterations for this reason, and it was recut as
  `loop_document_with_a_nested_span_starts_no_iteration`.
- Resume needs a captured session, so its re-entry case uses a Claude stub
  that emits a `system/init` stream line before failing.

Failing-before evidence (production change neutralized in place, then
restored):

| Neutralized change | Tests that failed |
| --- | --- |
| Validator call disabled in both prepare paths | 7 CLI tests: `compose_rejects_the_pre_fix_incident…`, `inline_compose_rejects…`, `proxy_to_the_incident…`, `sequence_document_with_a_nested_span…`, `predicates_operands_and_proxy_with_values…`, `loop_document_with_a_nested_span…`, `reentry_refuses_a_defect…` (the referenced-document sequence test stayed green because the pre-scan alone covers it) |
| Sequence pre-scan call disabled | `sequence_referencing_the_incident_in_step_two_starts_no_step` |
| `reject_surviving_spans` reverted to the untyped prose raise | `surviving_span_in_top_level_field_renders_property_reason_and_specific_hint` |
| New library APIs (`validate_no_nested_spans_in_literals`, `LifecycleEvaluationReason`, `validate_prompt_lifecycle_literals`) | did not exist, so every library test using them failed to compile |

### DMLS consumers (delegated sub-agent)

A `rust-developer` sub-agent implemented **Editor Diagnostic**, **Safe Quick
Fix**, and **Severity Ladder**. The orchestrator re-ran the dmls suite
(714/714) after it finished.

- **New files:**
  - `darkmatter/dmls/src/diagnostics/nested_span.rs`: the stopgap lifecycle
    inventory, the whole-value and predicate producers, and the
    `NestedSpanRewrite` payload.
  - `diagnostics/nested_span/nested_span_tests.rs`
  - `diagnostics/frontmatter/severity_tests.rs`
- **Lifecycle inventory** (`nested_span::lifecycle`):
  - Events: `initialize, start, success, blocked, failure, finalize, loop`.
  - Communication fields mirror `LIFECYCLE_COMM_FIELDS` (`effect`/`stack`
    excluded).
  - Whole-value surfaces: `[event, comm field]` and everything under
    `[event, stack, i, …]` except the item's own `when`/`no_error`. That
    covers operands, `target`, and `with` values at any depth.
  - Predicates are `stack[i].when` and `loop.while`/`loop.until`, and must be
    expression-typed in the schema.
  - `err`/`timing`/`current` are known roots only beneath an event key. The
    scoping lives in the frontmatter producer, not `is_unknown_root`, which
    GitNexus rates HIGH.
  - A parity test reads both authored schema files.
- **One diagnostic per expression**, on its first nested span. The incident
  yields exactly two, one per `say`. Every lint from one expression carries
  the same whole-expression rewrite, so one quick fix repairs every span.
  Claudine likewise reports an expression's first lint, so both name
  `{{ctx.area}}` for `success.say`.
- **Quick fix** `Rewrite with + concatenation`:
  - It reads only typed `data`: `{action: "dm.expression.rewrite_concatenation",
    version: 1, documentVersion, range, newText}`.
  - It declines a version mismatch, a foreign action or version, and missing
    `data`.
  - Folded, tagged, and line-spanning literals carry no data. The diagnostic
    is skipped when the overlay holds a stale last-good tree.
- **Severity ladder:**
  - schema-typed, exactly projected `EXPRESSION_MALFORMED` → `ERROR`;
  - body malformed stays `WARNING`;
  - `EXPRESSION_UNKNOWN_IDENTIFIER` → `WARNING` in both producers;
  - `is_pending_expression_value` is called before parsing;
  - the `union_rejected` comment now states the real invariant.

  The ladder is written in `diagnostics/codes.rs` and
  `darkmatter/dmls/docs/diagnostics.md`.
- **Deliberate assertion changes:**
  - `unknown_expression_root_is_informational_frontmatter_diagnostic` →
    `…_is_a_warning_frontmatter_diagnostic`.
  - `malformed_expression_producer_is_gated_to_exactly_projected_styles` now
    also asserts `ERROR`.
  - `sequence_tests::expression_diagnostics_newly_reach_list_items` asserts
    `ERROR`/`WARNING`.
- **Corpus re-bless** (`mapping_only_corpus/baseline.json`): the only diff is
  eight body `unknown_identifier` entries going from severity 3 to 2. The
  baseline held no `err`/`timing`/`current` entries and gained no nested-span
  diagnostic.
- **Failing-before:** 14 in-place mutations (A–K in the sub-agent report).
  Examples: producer disabled (7 tests), scoping always true, version and
  discriminator checks off, ladder reverted per producer, pending check off,
  encoding off, event list drift, rewrite altered, action gated on severity.
  Each failed its targeted tests and was restored green.
- **Note:** the existing "Wrap in interpolation literal" action only ever
  applied to body diagnostics, which stay `WARNING`. The test proves it keys
  on the code, not the severity.

### Requirement-to-test mapping (Phase 4)

| Requirement | Test(s) |
| --- | --- |
| Incident rejected at `success.say` with literal, nested span, Darkmatter rewrite bytes; then `failure.say` | `lifecycle::tests::nested_span::{incident_is_rejected_at_success_say_with_the_shared_rewrite, incident_failure_say_is_reported_once_success_say_is_repaired}` |
| Every comm field × event, `loop` comm, predicates (`when`, `loop.while/until`) in condition mode | `nested_span::{every_communication_field_on_every_event_is_checked, predicates_are_linted_as_condition_text}` |
| Operands in positional/key-value/array/signature forms; `proxy … with` paths | `nested_span::{action_operands_are_checked_in_every_authoring_form, proxy_with_values_are_checked_at_their_overlay_path}` |
| `LifecycleSignal::ALL` first-error order | `nested_span::events_are_reported_in_lifecycle_signal_order` |
| Negative controls: synthesized bodies, key/value params, `set_frontmatter` whole value, mixed strings, `{{{ }}}`, `+` form, non-lifecycle key (commit fixture) | `nested_span::{synthesized_mixed_and_non_lifecycle_surfaces_are_not_rejected, commit_fixture_nested_span_lives_on_a_non_lifecycle_key}` + CLI `synthesized_mixed_and_ordinary_frontmatter_values_still_launch` |
| Missing source record is an internal error | `nested_span::a_surface_without_an_authored_source_is_an_internal_error` |
| Error rendering (header, property, literal, unescaped rewrite, `{{{ … }}}` hint) | `nested_span::nested_span_error_renders_property_literal_rewrite_and_escape_hint` |
| Claudine inventory parity with authored schema | `nested_span::single_pass_inventory_matches_the_authored_claudine_schema` |
| Direct compose + `--dry-run`: exit non-zero, names property/literal/rewrite, no provider | CLI `compose_rejects_the_pre_fix_incident_before_any_provider_starts` |
| Inline compose, document untouched | CLI `inline_compose_rejects_a_nested_span_before_any_provider_starts` |
| Proxy handoff | CLI `proxy_to_the_incident_is_rejected_before_any_provider_starts` |
| Sequence: referenced step two (+ dry-run) and self-defect, no step starts | CLI `sequence_referencing_the_incident_in_step_two_starts_no_step`, `sequence_document_with_a_nested_span_starts_no_step` |
| Pre-scan: explicit/implicit/`@`/`&`/`^`/env-interpolated references; unparseable left to turn | `sequence::preflight::tests::lifecycle_literals::*` |
| Predicates / operands / `with` / `loop.while` via CLI | CLI `predicates_operands_and_proxy_with_values_are_rejected_before_launch` |
| Retry / resume re-entry; loop seed | CLI `reentry_refuses_a_defect_introduced_by_the_previous_attempt`, `loop_document_with_a_nested_span_starts_no_iteration` |
| Runtime backstop: property, typed reason, specific hint, old hint absent | `executor::tests::event_time_interpolation::{surviving_span_in_top_level_field_renders_property_reason_and_specific_hint, expression_raise_keeps_the_missing_path_hint, post_dm2_surviving_span_fails_before_dispatch}` + CLI `surviving_span_at_event_time_names_the_property_and_specific_hint` |
| DMLS diagnostic, ranges, scope, quick fix, ladder, pending, agreement bytes | see "DMLS consumers" |

### Gates run (Phase 4, macOS)

- `cargo nextest run -p dmls` → 714/714.
- `just test` in `darkmatter/` → 7922 passed, 7 skipped (sub-agent run,
  including `no_side_effects` and `packaging_contract`).
- `just lint` in `darkmatter/` → pass (sub-agent run).
- `just test --no-fail-fast` in `claudine/`:
  - The first run gave 7188 passed and 1 failed. The failure was
    `error_walker::tests::withholds_frontmatter_yaml_block_when_not_tty`: my
    recut example error body contained the frontmatter key the test asserts
    is absent. I fixed the example.
  - The final run: **7189 passed, 9 skipped, 0 failed**.
- `just lint` in `claudine/`:
  - It first failed with `clippy::result_large_err`, because the new
    `LifecycleEvaluationError` fields grew `CompositionError`.
  - `reason` is now `Box<LifecycleEvaluationReason>` on the variant. After
    that change: pass.
- **Cross-OS:**
  - `just cross-check claudine-cli --os windows --test wrap_compose_validation`
    could not run: `build-win-native` reported "No space left on device"
    while applying the patch (environment issue, not a test result).
  - Linux: the run waited on the `build-linux` host lock, held since
    2026-09-14 by an unrelated `nightly-reward-spike` job. It was stopped
    rather than contending, so Linux, native Windows, and WSL2 evidence is
    left to CI.
- **Not done this phase:** the stale `LifecycleInterpolationLeak` /
  `validate_no_interpolation_leaks` text in `claudine/docs/topics/lifecycle.md`
  and `.claude/skills/claudine/lifecycle.md`. Phase 5 (Reference Docs / Skill
  Guidance) owns those edits.

## Phase 5

Phase 5 corrects authored content and documentation. Entries are appended as
each task lands.

### Prompt Verification

- **Expressions:** both live prompts already hold the valid `+` form
  (`success.say`/`failure.say` in `prompts/_reviews/review-spec-inline.md`,
  `resides_in` with `as_csv(…)` in `prompts/commit.md`). No expression was
  changed.
- **Finding: the live review prompt could not compose at all.** Composing it
  through the normal path failed before launch with `TransclusionError` twice.
  - `::file _writing_clearly.md`: `dd2bccf39` added the file as
    `prompts/_writing-clearly.md` (hyphen, one directory up). No underscore
    spelling has ever existed.
  - `::file ./_set_spec_schema.md`: that file lives in `prompts/`, not
    `prompts/_reviews/`.

  Both references now read `../_writing-clearly.md` and
  `../_set_spec_schema.md`, the same pattern `feature-review.md` uses for
  `../_senior-reviewer.md`. `prompts/_interactive-prompting.md` had the same
  underscore typo (`./_writing_clearly.md` → `./_writing-clearly.md`). These
  three `::file` lines are the only prompt edits. The author's unrelated
  uncommitted edit at the end of `review-spec-inline.md` (removal of a
  dangling `- set the \`` line) was left intact.
- **Pre-existing, out of scope, recorded only:**
  - Four shipped prompts have lifecycle blocks that `parse_lifecycle_config`
    rejects outright:
    - `prompts/fix.md` (`initialize` is a list)
    - `prompts/merge-conflicts.md` (`initialize.stack` is a map)
    - `prompts/format.md` (`start.stack[0]` has no `action`)
    - `prompts/_implement/implement-feature.md` (`initialize` is a list)

    Preparation refuses them with their own error. The nested-span corpus
    test skips a lifecycle that does not parse rather than reporting it.
  - `prompts/commit.md` composes `model` correctly
    (`--model minimax/MiniMax-M3` reaches the provider), but the model-catalog
    warning quotes the raw uncomposed `{{ … }}` frontmatter value.
- **Tests added** (`claudine/cli/tests/shipped_prompt_contract.rs`):
  - `shipped_prompt_lifecycles_have_no_nested_spans_in_literals`: a passive
    corpus test. It runs the public `parse_lifecycle_config` +
    `validate_no_nested_spans_in_literals` (the same validator
    `prepare_direct`/`prepare_inline` call) over every shipped prompt, with
    the pre-fix incident as a negative control that must be refused at
    `success.say`.
  - `shipped_review_spec_inline_fires_success_cleanly` /
    `…_fires_failure_cleanly`: end-to-end runs through
    `CliProcessFixture::command()`, `compose -y --codex`, and a copied corpus.
    A `codex` stub captures its stdin, so a missing capture proves a
    pre-launch failure rather than a lifecycle outcome.
    - Success asserts exit 0, the composed prompt reached the provider, the
      `success.info` line is written with its link resolved, no `{{`
      anywhere in the output, no lifecycle validation/evaluation error, and
      an empty audio spool.
    - Failure asserts a non-zero exit with no lifecycle error and no `{{`.
    - `say` text is not observable under the child-local `PLAYA_DRY_RUN`,
      but it resolves through `resolve_emit` → `reject_surviving_spans`,
      which fails the event closed on any surviving span. A clean lifecycle
      is therefore the spoken-text proof. Both branches of each ternary are
      also covered statically by the corpus test.
  - `shipped_commit_prompt_composes_resides_in_and_fires_success_cleanly`: a
    staged file, an `opencode` stub capturing argv and stdin, and a `just`
    stub. It asserts `resides_in` is composed into the delivered prompt with
    no `{{`, `success.stack`'s `just gitnexus` ran exactly once, and there is
    no lifecycle error and no audio.
- **Failing-before evidence.** Both live prompts were backed up, overwritten
  with the `cd6e036c4^` regression fixtures, tested, and restored (checksums
  compared):
  - The corpus test and both review end-to-end tests **failed**.
  - The commit end-to-end test **passed** on the pre-fix bytes, as expected:
    `resides_in` is a non-lifecycle whole value consumed only by the
    rescanning body (`{{resides_in}}`), so its nested spans resolved there
    even before the repair (see Phase 1
    `commit_fixture_nested_span_lives_on_a_non_lifecycle_key`). It is a
    positive control for acceptance 3, not a regression test.
  - Before the three `::file` corrections, both review end-to-end tests
    failed with "the provider must have started" (`TransclusionError`).

### Reference Docs

- **`claudine/docs/topics/lifecycle.md`** (and its skill snapshot
  `.claude/skills/claudine/lifecycle.md`, kept in step by applying the same
  two blocks):
  - **Action Forms:** new `#### Braces inside a quoted literal are inert`,
    covering the single-pass surfaces, the `+` form, rescanning surfaces as
    the contrast, and `{{{ … }}}` for literal braces.
  - **Validation:** the stale `LifecycleInterpolationLeak` section (which
    claimed `validate_no_interpolation_leaks` "still runs") was replaced by
    two sections:
    - `LifecycleNestedSpanInLiteral`: what is checked, the prepare placement
      that covers direct/inline/dry-run/proxy/retry/resume/loop seed, the
      loop-seed stamping fact, and the sequence pre-scan placement (after
      graph build, before shell approval), plus the incident before/after;
    - `LifecycleEvaluationError`: surviving span, covering the property, the
      typed reason, the hint selection, and the Phase 4 observation that
      stack operands re-expand and fail as ordinary expression errors.
- **`darkmatter/docs/inline/interpolation.md`:**
  - **`## Escaping an Opener with a Backslash`:** both spellings, odd/even
    parity, backslash preserved by compose, CommonMark rendering, and YAML
    double-quote decoding.
  - **`## Braces Inside String Literals`:** rescanning vs single-pass
    surfaces, the `+` form, the rewrite guarantees, and the unified aggregate
    (array/object) suggestion behavior, plus when a suggestion is withheld.
  - **Implementation section drift fixed:** it said "single-pass rewrite",
    but `interpolate_text` is a fixpoint loop. Two bullets had been mangled
    to `{{{ ... }}}` where `{{ ... }}` was meant, including a no-op
    "`{{{ ... }}}` → `{{{ ... }}}`" conversion.
  - **Verified against this worktree's `md` build** (the installed `md` is
    stale and predates the Phase 2 scanner):
    - `A \{{ name }} B \\{{ name }}` composes to
      `A \{{ name }} B \\bob`;
    - YAML `q: "\\{{ x }}"` → `\{{ x }}`;
    - mixed `q: "a {{ 'b {{name}}' }}"` → `a b bob`;
    - whole-value `r: "{{ 'b {{name}}' }}"` → `b {{name}}`, which a body
      `{{r}}` then resolves on rescan.
- **`darkmatter/docs/lsp/features.md`:**
  - **§4.1:** frontmatter coverage notes covering sequence descent (list
    items, element-ranged array problems, silent synthetic marker), string
    scalar scanning scoped to owned surfaces, and projection limits (plain,
    quoted, and literal-block exact; folded, tagged, and alias fall back and
    never get a fabricated range or edit).
  - **§4.3:** a row for `dm.expression.nested_span_in_literal` and its
    quick-fix limits, plus the written severity policy and ladder table,
    pointing at `dmls/docs/diagnostics.md#severity` as the authoritative
    per-code list, so the two agree rather than diverge.
- **`darkmatter/dmls/docs/diagnostics.md`** (reconciliation): the
  nested-span row now also states that no rewrite is offered for a literal
  that spans lines, matching `ProjectedNestedSpan::replacement`.
- **READMEs.**
  - `darkmatter/README.md` and `claudine/README.md` enumerate no validation
    errors or diagnostic codes, so they were not edited.
  - `claudine/lib/README.md` has a "Compatibility Notes" section listing
    public `CompositionError` variant changes. It gained a 2026-09 entry for
    the removed `validate_no_interpolation_leaks` /
    `LifecycleInterpolationLeak`, the new validator and variant, and the new
    `LifecycleEvaluationError` fields.

### Skill Guidance

- **`.claude/skills/claudine/lifecycle.md`:** the snapshot received the same
  Action Forms and Validation blocks as the topic doc. That covers the
  inert-quoted-literal rule, the valid `+` form, and the single-pass versus
  rescanning boundary.
- **`.claude/skills/claudine/SKILL.md`:** the Lifecycle stacks row now reads
  "nested-span-in-literal, surviving-span & err-placement guards", replacing
  "leak & err-placement guards".
- **`.claude/skills/claudine/timeline.md`:** new `2026-09-16 —
  better-static-analysis` entry, with the loop-seed stamping trap.
- **`.claude/skills/darkmatter/compose.md`:** new `### Braces Inside String
  Literals` and `### Escaping an Opener` paragraphs under Interpolation.
- **`claudine/docs/topics/composition.md`** and its skill snapshot
  `.claude/skills/claudine/composition.md`: the list of lifecycle guards that
  receive the frontmatter excerpt named the deleted "interpolation leak". It
  now says "nested span in a literal"; `LifecycleNestedSpanInLiteral` is
  registered with `FrontmatterHighlight::Property`.
- **Hashes:** `md hash --save` was run on every edited file that carries a
  `hash:` property:
  - `.claude/skills/claudine/timeline.md`
  - `.claude/skills/claudine/composition.md`
  - `.claude/skills/darkmatter/compose.md`
  - `claudine/docs/topics/composition.md`

  Each re-hash verified (`md hash` equals the stored value).
  `lifecycle.md` (both copies), `SKILL.md`, and the Darkmatter docs carry no
  `hash:` property.
- **Observation (not changed):** `timeline.md`'s committed hash at `HEAD`
  already did not match its body. `md hash --save` corrected it as part of
  this edit.

### Comment Audit

A read-only sub-agent reviewed the `///`, `//!`, and `//` comments on every
`.rs` change from Phases 1–4. It found six drifts and one borderline case. Each
was confirmed against the code, then fixed comment-only:

- **`darkmatter/lib/src/markdown/compose/expression/lexer.rs`** (module doc and
  `ExpressionFinder` doc): claimed code spans are ignored. Only fenced and
  indented blocks are skipped; inline code spans are scanned. This predates
  the fix but sits beside the new escape comments.
- **`claudine/lib/src/composition/lifecycle/validate.rs`:**
  - `validate_no_nested_spans_in_literals`: communication fields are read
    from the parsed config (already authored strings). Only stack surfaces and
    loop predicates use `LifecycleSourceMap`.
  - `visit_string_literals`: still described its purpose as a leak scan. Its
    only caller is now the `err`-availability scan.
- **`claudine/lib/src/composition/error/mod.rs`**
  (`LifecycleNestedSpanInLiteral`): now also names the sequence pre-scan as a
  raise site.
- **`claudine/lib/src/composition/lifecycle/context.rs`**
  (`LifecycleErrorInfo::reason`): recovered by downcasting the reported error
  itself; no source-chain walk.
- **`darkmatter/dmls/src/diagnostics/frontmatter.rs`**
  (`expression_diagnostics`): ranges are exact only for untagged single-line
  plain or quoted values, and otherwise cover the whole scalar.
- **`darkmatter/dmls/src/diagnostics/nested_span.rs`** (borderline): the parity
  test reads `claudine-types.yaml` as well as `claudine.yaml`.

Clean areas:

- backslash-escape comments;
- validation placement comments (none put the pre-scan inside Phase 1c);
- `union_rejected`;
- scalar projection and arena roles;
- severity comments.

A repo-wide Rust search finds no remaining `validate_no_interpolation_leaks`,
`LifecycleInterpolationLeak`, `find_matching_warning_reason`, or leak-scan
wording.

### Requirement-to-test mapping (Phase 5)

| Requirement | Test / evidence |
| --- | --- |
| Acceptance 3: live prompts carry no nested span on single-pass lifecycle surfaces (passive corpus, all shipped artifacts) | `shipped_prompt_contract::shipped_prompt_lifecycles_have_no_nested_spans_in_literals` (incident fixture as negative control) |
| Acceptance 3: `review-spec-inline.md` composes and fires `success` cleanly through the normal path, and no braces are written | `shipped_prompt_contract::shipped_review_spec_inline_fires_success_cleanly` |
| Acceptance 3: `failure` fires cleanly; the failure report is not replaced by a lifecycle crash | `shipped_prompt_contract::shipped_review_spec_inline_fires_failure_cleanly` |
| Acceptance 3: `commit.md` `resides_in` composes into the delivered prompt with no raw span; `success` stack runs | `shipped_prompt_contract::shipped_commit_prompt_composes_resides_in_and_fires_success_cleanly` (positive control) |
| Acceptance 18: D5 docs updated, skill hashes refreshed | `md hash` equals the stored `hash:` for all four re-stamped files (checked) |
| Documented behavior matches the binary | worktree `md compose` runs recorded under Reference Docs |

### Gates run (Phase 5, macOS)

- **`just test --no-fail-fast` in `claudine/`**, first run: 7192 passed and 1
  failed. The failure was
  `spawn_site_guard::migrated_l1_tests_keep_the_isolation_the_builder_gave_them`,
  because the new commit test used `.current_dir` on a `git add` helper. It was
  switched to `git -C <dir>`, the `compose_repository_context.rs` pattern.
- **`just test --no-fail-fast` in `claudine/`**, final run (after the comment
  fixes): **7193 passed, 9 skipped, 0 failed**.
- **`just lint` in `claudine/`:** pass.
- **`just lint` in `darkmatter/`:** pass. It was run because Phase 5 made
  comment-only edits to Darkmatter and DMLS sources.
- **Not run:**
  - `just test` in `darkmatter/`, because the source changes there are
    comment-only;
  - cross-OS checks: the new end-to-end tests are `#[cfg(unix)]`, matching
    their siblings, and the corpus test and path fixes are separator-neutral.
    CI supplies Linux, Windows, and WSL2.
- **Pre-existing, unrelated:** kache prints `Cross-device link (os error 18)`
  staging warnings during builds; they are harmless.

## Phase 6

Phase 6 validates, reviews, and hands off. Entries are appended as each task
lands.

### Darkmatter L1

- `just test` in `darkmatter/` (lib, CLI, DMLS, zed CLI): **7922 passed, 7
  skipped, 0 failed**. The same count as the Phase 4 final run, so Phase 5's
  comment-only Darkmatter/DMLS edits changed nothing.
- The focused groups ran inside that run and are all green: `expression::lint`,
  the lexer escape tests, `backslash_escape_spans`, `mapping_only_corpus`,
  `nested_span_tests`, the frontmatter severity tests, and the sequence-descent
  tests.
- `dmls::no_side_effects::dsl_requests_spawn_no_processes_and_open_no_sockets`
  and `dmls::packaging_contract::dist_recipe_and_zed_extension_agree_on_archive_names`
  both **passed**.
- Deliberate assertion changes, carried from earlier phases and not new here:
  - array-item fallback: `test_entry_or_ancestor_falls_back_for_array_index`
    was re-cut as `test_entry_or_ancestor_is_exact_for_array_item` (Phase 3);
  - severity ladder: three DMLS severity assertions re-cut, and the
    mapping-only corpus re-blessed with exactly 8 body unknown-identifier
    lines, 3 → 2 (Phase 4).

### Claudine L1

- `just test --no-fail-fast` in `claudine/`: **7193 passed (1 slow), 9
  skipped, 0 failed**.
- Process-test audit. Every process test this fix added,
  `wrap_compose_validation.rs` (the incident, dry-run, inline, proxy, sequence,
  `when`/operand/`with`/`loop.while`, retry/resume, and loop-seed cases) and
  `shipped_prompt_contract.rs`, spawns through `CliProcessFixture::command()`:
  - That gives the fixture home, the minimal `PATH`, and child-local
    `PLAYA_DRY_RUN=1` with a private spool. The tests assert the spool is
    empty or absent.
  - Providers are stubs. `goose` and `codex` write invocation markers or
    captures, and `opencode` and `just` are stubbed too, so no real provider,
    network, or audio is reached.
  - `shipped_prompt_contract` passes `-y`. Everything else fails in preparation
    or runs a stub that exits at once, and `std::process::Command::output`
    gives the child a null stdin, so no test can block on an interactive
    prompt.
  - `spawn_site_guard` (green) enforces that no raw spawn site exists.

### Lint Gates

- Serial, never concurrent:
  - `just lint` in `darkmatter/`: **pass**, including the wasm32-wasip2
    `zed-dmls` compile;
  - `just lint` in `claudine/`: **pass**.
- `cargo fmt` was not run.

### Graph Review

- **GitNexus `detect_changes`, scope `all`,** run through MCP on the
  worktree:
  - **Complete.** `changed_count` 365 equals the listed array length, and
    neither `partial` nor `truncated` is set.
  - **Scope.** 90 files, 13 affected processes, risk `high`.
  - **The diff mixes two efforts.** The worktree also holds the concurrent,
    uncommitted shadow-home / provider-overlay work: `provider_overlay/*`,
    `mcp/inject*`, `wrap/{launch_plan*,wrapper_mcp.rs,composition/pipeline.rs}`,
    `opencode_config.rs`, and `provider/kilo/behavior.rs`.
- **Affected processes, by effort:**
  - `Execute_loop_with_lifecycle → {Early_binding_context, Injected_globals}`
    through `resolve_emit` / `emit_top_level` is this fix: the event-time
    backstop now raises a typed `SurvivingSpan`. It is covered by the executor
    `event_time_interpolation` tests and the shipped-prompt end-to-end tests.
  - `Execute_sequence → *` (5 flows) is this fix: the referenced-document
    pre-scan call in `wrap/sequence/mod.rs`. It is covered by the
    `wrap_compose_validation` sequence cases.
  - `Construct_argv_and_system_prompt → *` (6 flows) is shadow-home
    (`pipeline.rs` overlay plumbing), not this fix.
  - All blast radii were assessed in Phases 1–4 (`prepare_document`,
    `iter_stack_expression_surfaces`, `expression_values` HIGH, `lower_mapping`,
    `run_phase_1c_with_schema`, `reject_surviving_spans`,
    `ExpressionFinder.scan`, `interpolate_value`). No implementation-added
    public symbol needed a new impact run: they are new leaves whose callers
    are the ones listed.
- **Forbidden-shape diff review** (read-only sub-agent; each finding
  re-checked by hand):
  - **Quadratic arena: clean.** Every production loop over `entries()` uses
    parent-chain helpers (`path_at`, `key_path_at`, `is_in_sequence`), which
    are O(depth).
    - The per-pass schema-shape memo exists as `ShapeMemo`
      (`dmls/src/providers/frontmatter.rs`); the plan calls it
      `nested_shape_for_completion`.
    - **Note, not fixed:** the loop at `diagnostics/frontmatter.rs` over
      `expression_values` calls `expression_root_is_unknown`. That does a
      linear `entry_by_dotted` plus a `known_shape` rebuild per value. The
      code predates this fix, but sequence descent now feeds stack `when`
      values through it. That is linear per value, not a scan of `entries()`,
      so acceptance 12 holds, but it is a cost hotspot to watch.
  - **Duplicate lifecycle walkers: design deviation recorded, accepted.**
    `LifecycleSourceMap::from_frontmatter` walks the raw frontmatter lifecycle
    blocks to key authored scalars by canonical property path. The validator
    then looks up each surface that the one canonical
    `iter_stack_expression_surfaces` yields, and a miss fails closed as
    `LifecycleInvalid` (internal error).
    - This meets Resolved Implementation Determination 2's intent: one
      canonical surface iterator, authored text from one source map, no silent
      skip.
    - It differs in shape. The authored text is looked up by property string
      rather than attached as a field on `LifecycleExpressionSurface`, and the
      source map mirrors `action_shape.rs`'s verb-to-operand naming.
    - The `["while", "until"]` pair is spelled in both `validate.rs` and
      `source_map.rs`.
    - Drift between the two cannot pass silently: the fail-closed miss and the
      Claudine inventory parity test catch it. Not refactored in the
      validation phase.
  - **Hand-rolled file-reference resolution: clean.** The sequence pre-scan
    reuses the preflight graph's resolved `prompt_documents`. The added
    `.join(` calls are all in tests.
  - **Message-parsed code actions: clean.**
    `code_actions::rewrite_with_concatenation` reads only the typed,
    versioned `NestedSpanRewrite` payload, gated on `document_version`.
  - **Accidental rescanning changes: clean.** `rewrite.rs` only moved
    `whole_value_span` to `expression/lint.rs`. Both rescan tests are outside
    the diff.
  - **Event-time hint: clean.** It is chosen by matching the typed
    `LifecycleEvaluationReason`, recovered by downcast.
  - **Edits outside scope: none from this fix.** Every changed file this fix
    owns is in the plan's Phase 1–5 lists. The other changed files belong to
    shadow-home, to the author's own `prompts/plan.md` edit, or to the tail of
    `prompts/_reviews/review-spec-inline.md`.
    - Shadow-home files: `.claude/skills/os/windows.md`,
      `claudine/docs/providers/dispatch-inventory.json`,
      `claudine/docs/pipeline.md`, `claudine/cli/README.md`, the overlay and
      Kilo topic docs and skill snapshots, and `target_launch/tests.rs`.
    - `SKILL.md`, `timeline.md`, and `claudine/lib/README.md` carry lines from
      both efforts.

### Cross Platform

**Not obtained. This task stays unchecked.**

- **CI:** the plan's route is to submit the package-scoped CI plan. That
  requires a push, and this session may not commit or push, so no CI run
  exists and there are no CI links.
- **Local rigs** (`just cross-check`), all three unavailable:
  - **build-linux:** the per-host lock `~/ci-verification/.cross-check.lock`
    is still held by `{"purpose": "nightly-reward-spike", "owner":
    "reward-20260914-c3e60d0", "branch": "feat-nightly-perf", "started":
    "2026-09-14T18:25:30Z"}`. No cargo, nextest, or rustc process is running
    on the host, so the lock looks stale. It belongs to another effort and was
    not removed.
    - My `darkmatter` waiter was stopped after about 12 minutes, and its
      uploaded `.sh`/`.patch` files were deleted from `~/ci-verification`.
    - The owner's lock and `nightly-reward-20260914/` were left untouched.
  - **build-win-native:** `W:` reports **0 GB free**, the same "No space
    left on device" state as Phase 4. Nothing was run and nothing was deleted.
  - **build-win (WSL2):** SSH was refused on two attempts ("Connection reset
    by peer", then "Connection closed").
- **Portability review instead of evidence:**
  - Every new fixture is `eol=lf` through `.gitattributes`
    (`git check-attr` checked on the regression, corpus, and sequence-descent
    fixtures).
  - `mapping_only_corpus/baseline.json` holds no absolute paths or URIs.
  - The `wrap_compose_validation` marker provider has a `goose.cmd` Windows
    arm.
  - The process tests that need `sh` are `#[cfg(unix)]`, as their siblings
    are.
  - **Residual WSL-archive risk to watch in CI:** five new test sites read
    fixtures at run time through `env!("CARGO_MANIFEST_DIR")`:
    - `dmls/tests/mapping_only_corpus.rs`
    - `dmls/src/providers/frontmatter/sequence_tests.rs`
    - `dmls/src/diagnostics/nested_span/nested_span_tests.rs`
    - `claudine/lib/src/composition/lifecycle/tests/nested_span.rs`, which
      reaches the sibling area `darkmatter/docs/schemas`
    - `claudine/cli/tests/shipped_prompt_contract.rs`

    The `os` skill lists compile-time manifest-dir fixtures as a
    `wsl2-ubuntu` archive hazard. The same pattern already appears in 33
    places in `darkmatter/dmls` and `claudine/cli/tests` at `HEAD`, so the
    repository's WSL leg evidently resolves it. If one of these tests turns
    red only on WSL, that is the first place to look.

### Review Handoff — acceptance criteria

| # | Criterion (short) | Evidence |
| --- | --- | --- |
| 1 | Pre-fix incident rejected at prepare, names `success.say`, literal, `+` rewrite, no spawn | `lifecycle::tests::nested_span::incident_is_rejected_at_success_say_with_the_shared_rewrite`; CLI `wrap_compose_validation::compose_rejects_the_pre_fix_incident_before_any_provider_starts` (also `--dry-run`). Failing-before: Phase 1 manual run (provider `INVOKED`, event-time crash) |
| 2 | Proxy and sequence steps rejected before any spawn; dynamic reference at its turn | CLI `proxy_to_the_incident_is_rejected_before_any_provider_starts`, `sequence_referencing_the_incident_in_step_two_starts_no_step`, `sequence_document_with_a_nested_span_starts_no_step`; `sequence::preflight::tests::lifecycle_literals::*` (explicit/implicit/`@`/`&`/`^`/env-interpolated, unparseable left to turn) |
| 3 | Live prompts corrected, compose, fire cleanly, no braces | `shipped_prompt_contract::{shipped_prompt_lifecycles_have_no_nested_spans_in_literals, shipped_review_spec_inline_fires_success_cleanly, shipped_review_spec_inline_fires_failure_cleanly, shipped_commit_prompt_composes_resides_in_and_fires_success_cleanly}`. Failing-before recorded in Phase 5 |
| 4 | Positional/key-value interpolation and rescanning unchanged; rescan tests unmodified | `nested_span::synthesized_mixed_and_non_lifecycle_surfaces_are_not_rejected`, CLI `synthesized_mixed_and_ordinary_frontmatter_values_still_launch`, `rewrite.rs` `rescans_*` (outside the diff, re-verified in Graph Review), `rescanning_body_still_resolves_what_the_lint_describes`; full claudine L1 green |
| 5 | One recognizer; shared classifier; both parity tests; both property tests | proptests `flagged_literals_equal_scanner_hits`, `whole_value_classifier_matches_interpolate_value_branch`; parity `nested_span::single_pass_inventory_matches_the_authored_claudine_schema`, `nested_span_tests::lifecycle_inventory_matches_the_authored_claudine_schema` |
| 6 | DMLS diagnostic scope, style ranges, quick fix leaves zero diagnostics | `nested_span_tests::{incident_yields_one_error_per_say_on_the_inner_span, the_same_defect_in_a_body_produces_no_diagnostic, mixed_strings_and_non_lifecycle_whole_values_are_not_flagged, ranges_are_inner_for_exact_styles_and_whole_for_folded_and_tagged, quick_fix_rewrites_the_incident_literal_blocks_and_nothing_else, folded_tagged_and_line_spanning_literals_offer_no_action}` |
| 7 | Event-time message names key and typed reason; old hint absent | `executor::tests::event_time_interpolation::{surviving_span_in_top_level_field_renders_property_reason_and_specific_hint, expression_raise_keeps_the_missing_path_hint}`; CLI `surviving_span_at_event_time_names_the_property_and_specific_hint` |
| 8 | Array span: no suggestion; object/scalar/untyped: suggestion | **Superseded by the plan's post-companion ruling.** `61f085043` unified array rendering, so arrays are also offered a rewrite and no type plumbing exists. Evidence: `commit_fixture_keeps_single_quotes_raw_newline_and_offers_array_rewrite`, `aggregate_values_render_as_json`, `lint::tests::rewrite_generator::*`. The criterion's text was not amended in `spec.md` (see drift check) |
| 9 | `SequenceItem` entries, `nested_shape` crosses sequences, rule reaches predicates/operands/`with` in the editor | `overlay::frontmatter::tests::test_sequence_items_are_explicit_entries`, `sequence_tests::nested_shape_consumes_array_index_against_item_type`, `nested_span_tests::stack_operands_proxy_with_values_and_predicates_are_diagnosed` |
| 10 | Mapping-only output identical; hover/completion/navigation positive and negative | `tests/mapping_only_corpus.rs` (baseline captured pre-edit; the only re-bless is acceptance 13's severity); `sequence_tests::{hover_works_at_list_item_key_and_value, hover_is_silent_on_synthetic_item_positions, completion_works_inside_list_items, completion_never_offers_synthetic_item_indices, navigation_works_at_list_item_file_values_only}` |
| 11 | Array schema problem ranges the element | `diagnostics::frontmatter::tests::array_item_schema_problem_ranges_the_failing_item` |
| 12 | Memo in place; no linear scan inside an `entries()` loop | `sequence_tests::expression_pass_materializes_each_distinct_ancestor_shape_once` (memo type `ShapeMemo`); Graph Review diff audit clean |
| 13 | Severity ladder values, written down | `severity_tests::{expression_severities, ladder_separates_schema_typed_frontmatter_from_body_inference, foreign_template_syntax_in_a_body_is_at_most_a_warning}`, `incident_yields_one_error_per_say_on_the_inner_span`; written in `dmls/src/diagnostics/codes.rs`, `dmls/docs/diagnostics.md`, `docs/lsp/features.md` §4.3 |
| 14 | Style gate for the malformed producer | `diagnostics::frontmatter::tests::malformed_expression_producer_is_gated_to_exactly_projected_styles` |
| 15 | `union_rejected` invariant plus comment and test; one exported pending predicate | `union_rejected_paths` doc states the invariant; pinned by `valid_expression_under_mixed_union_is_clean` and the mixed/expression-arm union tests; `is_pending_expression_value` with `pending_expression_values_are_classified_lexically`, `expression_validation_defers_exactly_on_pending_values`, `severity_tests::pending_values_are_deferred_before_parsing` |
| 16 | Backslash escape | `lexer::tests::backslash_escape::*`, `tests/backslash_escape_spans.rs` (6) |
| 17 | `err`/`timing`/`current` scoped to lifecycle keys | `nested_span_tests::{late_binding_roots_are_known_beneath_lifecycle_keys, late_binding_roots_are_unknown_outside_lifecycle_keys}` |
| 18 | D5 docs and skill hashes | Phase 5 Reference Docs / Skill Guidance; `md hash` verified on the four re-stamped files |
| 19 | L1 green on macOS and in CI on Linux/Windows/WSL2 | **macOS: satisfied** (darkmatter 7922/7 skipped, claudine 7193/9 skipped, both lints). **Linux/Windows/WSL2: not obtained**; see Cross Platform. It remains open until the branch is pushed and CI reports |

### Final drift check (`spec.md` and docs)

- **Docs:** there are no stale `validate_no_interpolation_leaks` /
  `LifecycleInterpolationLeak` references outside `fixes/`/`features/`. The
  two remaining mentions, in `claudine/lib/README.md` compatibility notes and
  the `timeline.md` entry, deliberately record the removal. No
  `DeclaredExpressionValueKind` or `_with_types` appears in docs or skills.
- **`spec.md` drift, recorded for the author and not edited.** The spec is the
  ratified design record, and rewriting acceptance text is the author's call.
  - Acceptance 3 still says "This is outstanding work: the current
    `review-spec-inline.md` is malformed". It is now done: the expressions were
    repaired upstream and the `::file` paths in Phase 5.
  - Acceptance 8 and the D1 interim design describe array-span suggestion
    suppression, which the landed companion fix `61f085043` made obsolete.
    The plan's post-companion ruling governs.
  - D2 / Determination 2 say the authored scalar is *attached* to
    `LifecycleExpressionSurface`. The implementation looks it up from
    `LifecycleSourceMap` by the surface's canonical property, fail-closed (see
    Graph Review).
  - The spec and plan name the memo `nested_shape_for_completion`. The code
    type is `ShapeMemo`.
- **Source comments:** Phase 5's audit covered every Phase 1–4 `.rs` change,
  and Phase 6 changed no source. Nothing new to audit.

### Phase 6 summary

- **No source, test, doc, or skill file changed** in this phase. Only
  `plan.md` and this log changed.
- **Skills:** no notable change for the `claudine` skill. The `os` skill was
  not edited either: the rig outages are transient host state, not durable
  OS facts.
- **Human review is requested** for acceptance 19, the unavailable cross-OS
  evidence.

## Implementation of Review Findings #1

> **started at:** 2026-09-16T20:02:17-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/claudine/fixes/2026-09-13-better-static-analysis/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Expression-literal decoding reverses escaped-opener parity after the hard lint' at 20:03:29
        - confirmed the shared `flag_literal` boundary has LOW upstream graph risk: one direct caller (`collect_flagged`), five total impacted symbols, and no indexed execution process; text and graph searches identified Claudine prepare-time validation and DMLS diagnostics as the downstream consumers of `lint_expression`
        - found that the lint scans authored literal bytes while the lexer classifies the decoded `StringLiteral`; runs of two or three authored backslashes therefore both shrink during decoding but produce opposite escaped-opener parity
        - implemented decoded-value escape classification with a byte-boundary map back to the authored literal, preserving authored diagnostic ranges and rewrite slices
        - added Level 1 odd/even regression matrices at the shared Darkmatter lint, Claudine prepare-time validator, and DMLS diagnostic boundaries
        - the first targeted Darkmatter recipe could not write the shared `target/` artifacts; verification was moved to a fresh writable `CARGO_TARGET_DIR` as in the review evidence
        - the full Darkmatter gate exposed a stale property-test oracle that still compared the lint to raw-literal scanning; it now derives its authority set from lexer-decoded token values and retains the generated two-backslash regression seed
        - the corrected full Darkmatter Level 1 gate passed all 7,924 selected tests with 7 tier-filtered skips
        - the first full Claudine Level 1 run reached 5,713 tests before seven unrelated spawn tests rejected the session wrapper's inherited `HOME=/Users/ken/.claudine`; the gate is being rerun with the native user home so provider-overlay isolation is evaluated under its intended host invariant
        - the corrected full Claudine Level 1 gate passed all 7,200 selected tests with 9 tier-filtered skips when run with the native user home
        - `just lint` passed in both directly affected package areas; Darkmatter also completed its Zed WASM check, and Claudine completed its guard tests plus all crate-specific Clippy phases
        - audited the changed module documentation and comments so the decoding and authored-range contract is explicit without changing public policy documentation
        - final GitNexus change detection reported LOW risk across the shared worktree, 5 changed files, 13 changed symbols, no affected execution processes, and no partial or truncated result
- work completed for 'Expression-literal decoding reverses escaped-opener parity after the hard lint' at 20:31:33
- starting the work on 'Ambiguous proxy overlay paths can bypass prepare-time validation' at 20:32:24
        - refreshed the GitNexus index to the current worktree commit; the canonical lifecycle surface iterator has LOW upstream risk with four direct consumers and no indexed execution process
        - GitNexus could not resolve callers for `LifecycleSourceMap` or `validate_no_nested_spans_in_literals`, so text search confirmed the map is private to the validator and the validator is reached by both shared preparation paths, sequence preflight, shipped-prompt coverage, and lifecycle unit tests
        - confirmed that dotted display strings are serving as internal identity in both independent walks, allowing arbitrary `proxy.with` map keys to collide with structural fields and array indices
        - selected a typed lifecycle surface path whose field, map-key, and array-index segments compare structurally while retaining a separate human-readable diagnostic rendering
        - implemented the typed path in both the raw source map and canonical expression-surface iterator; ordinary diagnostic paths remain unchanged while punctuation-bearing overlay keys render explicitly, such as `with["a.b"]`
        - added six Level 1 library regressions covering dotted keys, bracket-shaped keys, and a nested mapping/array path with the defect on each side of every collision
        - added a CLI process regression for the previously bypassed dotted-key case and asserted that the provider marker is never written
        - the focused compile check, all six library collision cases, and the CLI no-provider-start case passed in the isolated target directory
        - the full Claudine Level 1 gate passed all 7,204 selected tests with 9 tier-filtered skips when run with the native user home
        - `just lint` passed the Claudine guard tests and every crate-specific Clippy phase
        - audited the changed module and symbol documentation so the typed identity and diagnostic-rendering boundary are explicit; no user-facing lifecycle policy changed
        - final GitNexus change detection reported LOW risk across the shared worktree, 10 changed files, 40 changed symbols, no affected execution processes, and no partial or truncated result
- work completed for 'Ambiguous proxy overlay paths can bypass prepare-time validation' at 20:50:36

### Successful Completion

- The implementation of review cycle 1 has completed successfully in 50 minutes 3 seconds. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):
        - fixed: expression-literal escape parity is now classified after decoding while diagnostic ranges remain mapped to authored bytes
        - fixed: lifecycle source association now uses structural field, map-key, and array-index path segments that cannot collide
        - deferred: none
- The files changed for these findings were:
        - `darkmatter/lib/src/markdown/compose/expression/lint.rs`
        - `darkmatter/lib/proptest-regressions/markdown/compose/expression/lint.txt`
        - `darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs`
        - `claudine/lib/src/composition/lifecycle/source_map.rs`
        - `claudine/lib/src/composition/lifecycle/validate.rs`
        - `claudine/lib/src/composition/lifecycle/tests/nested_span.rs`
        - `claudine/cli/tests/wrap_compose_validation.rs`
        - `claudine/fixes/2026-09-13-better-static-analysis/review-1.md`
        - `claudine/fixes/2026-09-13-better-static-analysis/implementation-log.md`
