---
total_phases: 6
created: 2026-09-16
phase: 5
agent: "codex/default"
yolo: "true"
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

# Implementation Plan: Make Expression Defects Visible Before Execution

## Work Summary and Definition of Success

This plan implements the four connected strands in `spec.md`: Darkmatter gains
one source-aware nested-span lint plus a preserved backslash escape; Claudine
rejects invalid single-pass lifecycle expressions before any provider starts
and retains an accurate event-time backstop; DMLS descends through frontmatter
sequences, diagnoses the same defect with source-accurate ranges and safe quick
fixes, and applies the expression severity ladder; and the already-repaired
shipped prompts are verified while documentation and skill snapshots move to
the valid `+` guidance.

Successful completion is observable at each boundary:

- `lint_expression` and the event-time leak guard use
  `ExpressionFinder::find_all_plain` as their one nested-span recognizer, while
  a shared `is_whole_value_span` classifies syntax without claiming that every
  owner is single-pass.
- `\{{` and `\{\{` opt out of expression scanning, odd/even backslash parity is
  correct, and composition preserves every authored backslash.
- Direct compose, inline compose, dry-run, proxy handoff, retry/resume, loop
  refresh, and every statically resolvable sequence document fail in shared
  preparation before a provider process exists; dynamic sequence references
  fail at their own turn before their provider starts.
- DMLS reaches mapping and sequence-nested frontmatter, ranges plain/quoted and
  literal-block diagnostics accurately, declines unsafe folded/tagged edits,
  and applies quick fixes only from typed, versioned diagnostic data.
- Mapping-only DMLS behavior stays byte-identical; array failures range the
  failing item; hover, completion, and navigation work at real list-item
  keys/values but remain silent on synthetic item markers.
- Expression severities match the written policy: certain failures are errors,
  possible failures are warnings, and foreign template syntax in Markdown body
  text never becomes a hard error.
- The pre-fix incident and commit-prompt fixtures prove the regression, the
  already-repaired live prompts remain free of nested interpolation literals,
  and every new test is recorded as failing before its production fix.
- `just test` and `just lint` pass in both `darkmatter/` and `claudine/` on
  macOS; Linux, native Windows, and WSL2 evidence is supplied by CI. No DMLS L2
  editor run is required by this fix.

Planning evidence and risk shape:

- GitNexus was refreshed to commit `3df43cc67f4c4779beff6b4610b1d81ab37bdea0`.
  `prepare_document`, `lower_mapping`, `iter_stack_expression_surfaces`, and
  `run_phase_1c_with_schema` have low, exact upstream risk. The existing
  `expression_values` seam is **HIGH risk** because diagnostics, hover, and
  schema-problem production all consume it; Phase 3 therefore gives its
  projection and regression checks an explicit gate before severity changes.
- `spikes/entry-arena-cost.md` already settles the arena design: use explicit
  item entries, retain linear pointer lookup, add the required per-pass schema
  shape memo, and prohibit a linear lookup inside an `entries()` loop.
- `spikes/parser-agreement.md` already settles parser agreement: DMLS reuses
  Darkmatter's parser. The blocking work is safe scalar-style projection,
  explicit pending-value deferral, and preservation of the validation-report
  guard—not another parser spike.
- The array-rendering companion fix is already complete at `61f085043`.
  Arrays and objects now stringify consistently as compact JSON, so this plan
  deliberately omits `DeclaredExpressionValueKind` and the `_with_types` lint
  APIs from the historical interim design. Array-, object-, scalar-, and
  untyped spans all receive suggestions when the rewrite is otherwise safe.
- The prompt-only repair sequence has also landed (`cd6e036c4`, corrected by
  `3997839bf` and followed by `dd2bccf39`). The implementation must preserve
  those current expressions. The historical regression source is
  `cd6e036c4^`, not the moving `HEAD` named by the older specification text.

## Phase 1 — Lock Contracts and Regression Baselines

### Necessary Rules

- **Single-pass scope only.** Emit the hard nested-span diagnostic only for
  lifecycle communication whole values, lifecycle predicates, stack operands,
  and `proxy … with` values. Body interpolation, mixed frontmatter strings, and
  arbitrary whole-value frontmatter keys remain outside the rule because their
  owners can rescan.
- **Source text is authoritative.** Lint authored `SpannedExpr` string tokens
  and their raw source slices. Never infer this defect from an unspanned
  synthesized `Expr::StringLiteral`; positional action messages must retain
  their current event-time interpolation behavior.
- **One scanner and one syntax classifier.** Static linting and runtime
  guarding share `find_all_plain`; every consumer uses the exported
  `is_whole_value_span` for the exact-span shape. Claudine owns the canonical
  single-pass surface inventory; DMLS mirrors only the lifecycle-key subset
  until schema triggers replace that documented debt.
- **Post-companion suggestion policy.** Because array rendering is already
  unified, do not add schema/catalog type plumbing solely to suppress aggregate
  suggestions. Offer a rewrite for arrays and objects under the same safety
  rules as scalars and untyped values.
- **Passive editor and pre-scan behavior.** DMLS analysis and the sequence
  referenced-document pre-scan may parse and validate authored YAML only. They
  must not compose values, execute shell, fetch remotes, prompt, mutate files,
  or recapture ambient repository/CWD state.
- **Item entries are explicit.** A sequence item is `SequenceItem { index }`,
  has no authored key span, uses bracketed dotted notation, and is never made to
  look like a mapping key. Mapping entries preserve their existing fields and
  behavior.
- **Severity follows certainty.** `ERROR` means the construct cannot work;
  `WARNING` means it might be wrong. The new frontmatter-only defect is an
  error, schema-declared malformed expressions with exact projection are
  errors, body malformed expressions stay warnings, and unknown identifiers
  become warnings.
- **Escapes are observed, not consumed.** An odd backslash run before `{{`
  suppresses the opener; an even run does not. Darkmatter preserves the run and
  leaves CommonMark rendering to remove the visual escape.
- **Fixtures come from the actual pre-fix revision.** Capture both regression
  fixtures from `git show cd6e036c4^:…`, not the now-repaired `HEAD` or current
  working tree. Existing user changes in
  `prompts/_reviews/review-spec-inline.md` and `prompts/plan.md` must be
  preserved rather than overwritten.
- **No new spike is scheduled.** The two completed spikes answer the identified
  risks. Reopen discovery only if the implementation-start indentation check
  disproves that raw literal-block indentation is accepted as expression
  whitespace.

### Wave 1 — Baseline Evidence (parallel)

- [x] **Fixture Capture**
  - Materialize test-only copies of the committed pre-fix
    `prompts/_reviews/review-spec-inline.md` lifecycle values and
    `prompts/commit.md` `resides_in` expression via
    `git show cd6e036c4^:<path>`.
  - Keep the single/double quote forms, the `\n` escape, multiple nested spans,
    and the array-valued span byte-identical so rewrite and projection tests
    exercise the incident rather than the current hand edits.
  - Add a verification-record entry identifying the fixture source commit and
    proving each new test group fails against unchanged production code.

- [x] **Contract Inventory**
  - Record the current callers and behavior of `ExpressionFinder`,
    `prepare_document`, `iter_stack_expression_surfaces`, `expression_values`,
    `lower_mapping`, and `run_phase_1c_with_schema` before edits.
  - Preserve the existing rescanning tests in `interpolation/rewrite.rs`
    unchanged and identify the exact mapping-only DMLS corpus used for
    byte-for-byte regression comparison.
  - Treat the HIGH-risk `expression_values` fan-out as a review gate: no
    severity change may land until hover, diagnostics, and schema-problem
    consumers pass focused tests with the new projection model.

- [x] **Assumption Checks**
  - Run the implementation-start parser check for literal-block expression
    text containing raw YAML indentation.
  - Confirm `61f085043` remains present and no aggregate-suggestion suppression
    exists; if the branch has changed, re-evaluate only the post-companion
    ruling rather than blindly adding the historical interim API.
  - Confirm the authored Claudine schema event inventory and current lifecycle
    iterator order, then establish parity-test fixtures for both Claudine and
    DMLS.

### Phase 1 Checkpoint

- [x] **Baseline Review**
  - The fixture provenance, failing-before evidence, mapping-only corpus, event
    inventory, and companion-fix state are recorded.
  - No implementation starts until the raw-indentation check passes or the
    projection design is explicitly revised.

## Phase 2 — Build Darkmatter Expression Authorities

### Wave 2 — Scanner and Lint Core (parallel with file ownership)

- [x] **Escape Scanner**
  - Update `darkmatter/lib/src/markdown/compose/expression/lexer.rs` so both
    ordinary and plain scanning ignore a `{{` opener preceded by an odd run of
    backslashes and retain it after an even run.
  - Preserve every backslash in output; pin `\{{`, `\{\{`, unescaped spans,
    even/odd runs, unrelated escapes, code-region behavior, literal `{{{ }}}`,
    and unclosed-open cases.
  - Add compose and diagnostic regression coverage proving escaped foreign
    template examples remain literal and warning-free.

- [x] **Lint Model**
  - Add the narrow expression lint module and public `ExpressionLint`,
    `ExpressionLintKind::NestedSpanInStringLiteral`, `lint_expression`,
    `lint_spanned`, and `is_whole_value_span` APIs under
    `darkmatter/lib/src/markdown/compose/expression/`.
  - Walk every authored `SpannedExprKind::StringLiteral`, including object
    keys, slice the raw quoted token, and emit one lint per inner span found by
    `ExpressionFinder::find_all_plain`; return no lint on parse failure.
  - Keep the API surface minimal for the already-landed array unification: do
    not add declared-value classifiers or `_with_types` variants.

- [x] **Rewrite Generator**
  - Generate a complete bare-expression rewrite from raw source slices, reuse
    the authored quote, decode only lifted expression text, parenthesize every
    non-atomic lifted AST, and anchor each span so generated `+` operations
    cannot enter numeric addition.
  - Replace the original literal inside the full expression, suppress the
    suggestion when any lifted span is malformed, and reparse generated output
    in the original dialect as a final safety check.
  - Prove semantic equivalence and evaluated output for ternaries, additive
    spans, adjacent numeric spans, numeric suffixes, a span-only literal,
    escaped quotes, both quote styles, raw `\n`, multiple spans, and aggregate
    JSON values. Re-parsing alone is not acceptable evidence.

### Wave 3 — Shared Schema Helpers and Proof

- [x] **Pending Authority**
  - Export Darkmatter's passive `is_pending_expression_value` predicate from
    the schema-format layer and make schema validation call that same helper.
  - Add focused tests for `{{ … }}` and `$(…)` pending values so DMLS can reuse
    the authority without evaluation or I/O in Phase 4.

- [x] **Authority Properties**
  - Add property tests proving linted literal spans equal the
    `find_all_plain` result for those literal slices and that
    `is_whole_value_span` agrees with the actual `interpolate_value` branch.
  - Keep the two existing rescan tests unchanged and add a negative body case
    showing a nested literal still resolves on a rescanning surface even though
    the low-level lint can describe its syntax.
  - Run focused Darkmatter L1 tests before exposing the new APIs to Claudine or
    DMLS.

### Phase 2 Checkpoint

- [x] **Foundation Gate**
  - Darkmatter has one scanner, one whole-value classifier, one passive pending
    predicate, and one source-aware lint/rewrite implementation.
  - All scanner, semantic-equivalence, escape, and authority property tests pass
    without changing rescanning behavior.

## Phase 3 — Extend the DMLS Frontmatter Model

### Wave 4 — Arena and Schema Shape (parallel)

- [x] **Sequence Arena**
  - Extend `FmEntry` with `FmEntryRole` and optional `key_span`, retain YAML
    `ScalarStyle` plus tag presence, and generalize lowering in
    `darkmatter/dmls/src/overlay/frontmatter.rs` to descend through mappings
    and sequences.
  - Emit one explicit entry for each sequence item plus entries for mapping keys
    beneath it, with correct parent, depth, pointer, value span, and decimal
    index identity; preserve mapping-entry fields byte-for-byte.
  - Re-cut `test_entry_or_ancestor_falls_back_for_array_index` deliberately so
    `/tags/1` is an exact scalar hit, and prove array validation problems now
    range that element rather than the containing sequence.

- [x] **Path Semantics**
  - Introduce typed key/index path segments and one shared dotted formatter:
    sequence indices render as `[0]`, while a mapping key literally named `0`
    remains `.0`.
  - Teach `nested_shape`/context-aware shape traversal to apply union selection,
    consume array indices against the item type, fail closed on mismatched
    segments, and retain merged-arm fallback when no arm is selected.
  - Add round-trip coverage for numeric mapping keys, nested arrays, RFC 6901
    pointers, escaped `/` keys, and `initialize.stack[0].when`.

- [x] **Predicate Schema**
  - Update `darkmatter/docs/schemas/claudine.yaml` so lifecycle `when`, `until`,
    and `while` fields are expression-typed and therefore use DMLS's existing
    condition dialect.
  - Add passive schema corpus coverage proving the shipped schema parses and
    these fields resolve through array-item shapes without executing them.

### Wave 5 — Capability and Cost Integration

- [x] **Shape Memo**
  - Add a per-analysis-pass memo for `nested_shape_for_completion`, keyed by
    typed ancestor path and discarded after the pass, so each distinct ancestor
    shape is cloned at most once.
  - Apply it to every per-entry diagnostics/link consumer exposed by sequence
    descent; do not add a pointer-to-index arena map.
  - Add the `index_of` complexity warning and review guard: no production loop
    over `entries()` may call `key_path(entry)`, `entry_by_pointer`, or
    `entry_by_dotted` internally.

- [x] **Capability Boundaries**
  - Adapt every key-specific caller to `MappingProperty`/`Some(key_span)` and
    every value-specific caller to support `SequenceItem`/`value_span`.
  - Add positive hover, completion, and navigation tests at real list-item
    keys/values plus negative tests at synthetic item indices/markers.
  - Compare all existing mapping-only diagnostics byte-for-byte before and
    after descent; explicitly test the newly reachable malformed and unknown
    expression diagnostics inside list items.

- [x] **Interpolation Inventory**
  - Add frontmatter string interpolation inventory to
    `dmls/src/overlay/expressions.rs`, carrying semantic expression text and an
    explicit projection policy rather than matching decoded and raw spans by
    ordinal.
  - Use direct offsets for untagged plain scalars, `DecodedScalar` maps and
    YAML-style fragment encoding for single/double quotes, raw inner-expression
    coordinates for literal blocks, and whole-scalar/no-action fallback for
    folded or tagged scalars; aliases produce no expression diagnostic at the
    alias site.
  - Suppress a quick-fix suggestion when the flagged literal's raw slice spans
    a newline, and gate the existing malformed-expression producer to untagged
    plain/single/double styles with exact projection.

### Phase 3 Checkpoint

- [x] **Overlay Gate**
  - Sequence entries, bracketed paths, array schema traversal, scalar metadata,
    the shape memo, and capability boundaries are covered by focused DMLS L1.
  - `expression_values` consumers pass the HIGH-risk review gate, mapping-only
    output is unchanged, and unsafe scalar styles retain the generic schema
    diagnostic rather than a fabricated expression range.

## Phase 4 — Wire Claudine and DMLS Consumers

### Wave 6 — Independent Consumers (parallel)

- [x] **Lifecycle Validator**
  - Build one `LifecycleSourceMap` from raw frontmatter, keyed by canonical
    lifecycle property path, and attach matching authored scalar text/spans to
    the existing `LifecycleExpressionSurface`; do not add a second lifecycle
    walker.
  - Implement `validate_no_nested_spans_in_literals` across whole-value
    communication fields, all predicates, action operands, and `proxy … with`
    values using the proper parse mode and the shared Darkmatter lint.
  - Treat a parsed configured surface with no source-map record as an internal
    validation error, preserve `LifecycleSignal::ALL` first-error order, wire
    the validator into every shared preparation path, and delete
    `validate_no_interpolation_leaks` after recutting its useful cases.
  - Add `LifecycleNestedSpanInLiteral` with frontmatter highlighting,
    property/literal/nested-span fields, an illustrative bare-expression
    rewrite, and the `{{{ … }}}` intentional-braces hint.

- [x] **Editor Diagnostic**
  - Add `dm.expression.nested_span_in_literal` at `ERROR` and invoke the shared
    lint only for whole-value lifecycle scalars and expression-typed lifecycle
    predicates—not body, mixed, or arbitrary frontmatter surfaces.
  - Maintain the stopgap lifecycle-key inventory and scope `err`, `timing`, and
    `current` as known only beneath those keys; add schema parity and both
    positive/negative root-scope tests.
  - Produce precise inner-span ranges for plain/quoted/literal-block styles and
    whole-scalar ranges without actions for folded/tagged styles.

- [x] **Safe Quick Fix**
  - Add the `Rewrite with + concatenation` quick fix using a versioned typed
    diagnostic payload containing the document version, authored replacement
    range, action discriminator, and already YAML-style-encoded replacement.
  - Never parse the diagnostic message; decline stale snapshots and diagnostics
    without suggestions; preserve scalar quotes, block indicators,
    indentation, and every untouched byte.
  - Verify literal-block, single-quoted, and double-quoted edits produce a
    parseable document with zero nested-span diagnostics; folded/tagged and
    multiline-unsafe cases expose no action.

- [x] **Severity Ladder**
  - Raise safely projected schema-typed `EXPRESSION_MALFORMED` to `ERROR`, keep
    body malformed expressions at `WARNING`, and raise
    `EXPRESSION_UNKNOWN_IDENTIFIER` to `WARNING` in both producers.
  - Call the exported `is_pending_expression_value` before parsing, preserve
    `union_rejected` as the independent “never outrank accepted validation”
    guard, and correct its comment to state that invariant.
  - Prove exact behavior for plain/single/double `1 +`, generic schema fallback
    for literal/folded/tagged/alias values, pending `{{ … }}`/`$(…)`, accepted
    unions, foreign body templates, and the existing malformed code action.

- [x] **Runtime Backstop**
  - Keep `reject_surviving_spans` for dynamically stored template text but make
    its failure reason typed, include the canonical lifecycle property, and
    render the surviving span plus its two possible causes accurately.
  - Select the concatenate/`{{{ … }}}` hint from the typed reason, reserving the
    missing-path hint for actual lookup/evaluation failures; update all
    `LifecycleEvaluationError` construction and rendering sites together.
  - Add an event-time regression proving the new property/reason/hint appears
    and the old generic hint does not.

### Wave 7 — Sequence and End-to-End Enforcement

- [x] **Referenced Prescan**
  - Add an up-front, passive referenced-document lifecycle lint to sequence
    Phase 1c for every literal document executable before the existing compose
    attempts begin.
  - Resolve through `biscuit_file::FileReference` using the captured
    `FileResolutionContext` and the same `for_source` derivation as execution;
    never use ambient `resolve()`, prefix checks, or a second reference grammar.
  - Cover explicit-relative, implicit-relative, `@`, `&`, and `^` forms on all
    OSes. Allow an interpolated reference to defer until its step turn, where
    shared preparation must still reject it before that step's provider starts.

- [x] **Spawn Prevention**
  - Add hermetic library/CLI tests for direct compose, inline compose, dry-run,
    proxy, sequence step two, retry/resume/loop re-entry, predicates, action
    operands, and `proxy … with` values.
  - Use `CliProcessFixture` and an invocation-marker stub to prove no provider
    starts; for a statically referenced sequence document, prove step one also
    never starts because the pre-scan covers every resolvable step first.
  - Pin negative controls for synthesized action strings, mixed lifecycle
    strings, ordinary whole-value frontmatter, and all currently valid
    positional/key-value interpolation forms.

### Phase 4 Checkpoint

- [x] **Agreement Gate**
  - Claudine and DMLS agree on lifecycle inventory, offending nested span, and
    bare rewrite bytes for every surface both understand.
  - Direct, proxy, and sequence paths fail before spawn; the runtime backstop
    remains reachable only for dynamic template text; DMLS fixes are safe and
    severity tests pass.

## Phase 5 — Correct Authored Content and Documentation

### Wave 8 — Content and Guidance (parallel)

- [x] **Prompt Verification**
  - Treat the current `success.say`, `failure.say`, and `resides_in` expressions
    as repaired controls: preserve their valid `+` concatenation, `ctx.repo`,
    and `as_csv(…)` behavior unless a production test demonstrates a remaining
    defect.
  - Do not absorb or overwrite the unrelated working-tree edit at the end of
    `prompts/_reviews/review-spec-inline.md`; if no expression repair is needed,
    leave both prompt files untouched.
  - Compose both live prompts through their normal paths and verify emitted
    spoken/written text contains no braces and lifecycle completion is clean;
    record this as fulfillment of the specification's live-content criterion.

- [x] **Reference Docs**
  - Update `claudine/docs/topics/lifecycle.md` with the new validation error,
    before/after form, single-pass literal rule, and deletion of the stale claim
    that `validate_no_interpolation_leaks` runs.
  - Update `darkmatter/docs/inline/interpolation.md` with rescanning versus
    single-pass behavior, `+` rewriting, both preserved backslash escapes, and
    the already-unified aggregate suggestion behavior.
  - Update `darkmatter/docs/lsp/features.md` with frontmatter sequence coverage,
    the new diagnostic/action, projection limits, changed expression
    severities, and the written warning/error policy.
  - Check both READMEs; edit only if they currently enumerate affected
    validation errors or diagnostic codes.

- [x] **Skill Guidance**
  - Update the Claudine lifecycle and Darkmatter expression-authoring skill
    guidance with the inert-quoted-literal rule, valid `+` form, escape syntax,
    and single-pass/rescanning boundary.
  - Refresh every changed skill Markdown hash through `md hash <file>`; do not
    hand-author hash values.

### Wave 9 — Drift Review

- [x] **Comment Audit**
  - Review every changed symbol's `///`, `//!`, and inline comments for behavior
    drift, especially scanner escapes, lifecycle surfaces, `union_rejected`,
    scalar projection, arena roles, and Phase 1c guarantees.
  - Remove or correct stale implementation narration without broad adjacent
    cleanup; ensure no documentation promises precise folded/tagged projection
    or hard body errors.

### Phase 5 Checkpoint

- [x] **Authoring Gate**
  - Both live defects are corrected without losing concurrent user edits, all
    listed docs and skills describe the shipped behavior, and changed skill
    hashes verify cleanly.

## Phase 6 — Validate, Review, and Hand Off

### Wave 10 — Focused Validation

- [x] **Darkmatter L1**
  - Run focused lint/scanner/rewrite/schema/DMLS tests first, then `just test`
    from `darkmatter/`.
  - Confirm `no_side_effects.rs` and `packaging_contract.rs` remain green and
    record the deliberate array-item fallback and severity assertion changes.

- [x] **Claudine L1**
  - Run focused lifecycle preparation, runtime backstop, compose/proxy/sequence,
    and `wrap_compose_validation` tests first, then `just test` from
    `claudine/`.
  - Assert every process test uses the shared fixture, child-local dry-run audio
    policy, and invocation markers; no real provider, network, audio, or
    interactive prompt is allowed.

### Wave 11 — Serial Repository Gates

- [x] **Lint Gates**
  - Run `just lint` in `darkmatter/`, then `just lint` in `claudine/`; do not run
    these Cargo-heavy gates concurrently and do not run `cargo fmt`.
  - Run only the package-area checks needed to diagnose failures; DMLS L2 and
    browser/terminal focus-changing suites are outside this fix's required
    evidence.

- [x] **Graph Review**
  - Run GitNexus `detect_changes` over the complete worktree diff and require a
    non-partial, non-truncated result; investigate all affected processes and
    re-run impact for any implementation-added symbols whose blast radius was
    not covered in Phase 1.
  - Review the final diff for the forbidden quadratic arena shape, duplicate
    lifecycle walkers, hand-rolled file-reference resolution, message-parsed
    code actions, accidental rescanning changes, and edits outside the declared
    scope.

- [ ] **Cross Platform**
  - Submit the existing package-scoped CI plan and collect Linux, native
    Windows, and WSL2 L1 results in addition to local macOS evidence.
  - Treat path/reference fixture failures as portability defects; do not weaken
    explicit/implicit/`@`/`&`/`^` coverage or substitute platform-specific path
    comparisons.

### Final Checkpoint

- [x] **Review Handoff**
  - Map all 19 acceptance criteria to passing tests or recorded review
    evidence, including every failing-before proof.
  - Record commands, outcomes, CI links, deliberate assertion changes, and any
    unavailable environment evidence in this plan's verification record.
  - Run the final `spec.md` comment/documentation drift check and leave the fix
    implementation-complete and ready for author review; do not move it to
    `_completed`, run `just complete`, commit, or push.

## Verification Record

### Phase 1 — Baseline Evidence (2026-09-16, claude/opus)

**Fixture provenance.** `claudine/cli/tests/fixtures/nested_span_regression/`
holds byte-identical copies captured with `git show cd6e036c4^:<path>`
(`cd6e036c4^` = `1fe739f341767d52d3d4e26ae7f8f54916f38524`). `git hash-object`
of each copy equals the pre-fix blob:

| Fixture | Source path at `cd6e036c4^` | Blob |
| --- | --- | --- |
| `review-spec-inline.md` | `prompts/_reviews/review-spec-inline.md` | `69b6903318fb1170fb7efb157f0e847b79f76b38` |
| `commit.md` | `prompts/commit.md` | `dad40813a16a7306f2b80fb637435dd38e21046a` |

These are the canonical fixtures for Darkmatter, DMLS, and Claudine tests;
`darkmatter/lib/tests/schema_phase_validation.rs` already establishes the
cross-area `include_str!` precedent. The guard
`wrap_compose_validation::nested_span_regression_fixtures_preserve_pre_fix_defects`
pins every defect literal (both quote styles, the raw `\n`, all nested spans,
the array-valued span) so a "repaired" fixture cannot make later tests vacuous.

**Failing-before evidence (unchanged production code, `target/debug/claudine`
built from the worktree at `3df43cc67`).** Hermetic temp workspace, fixture
`HOME`, `PATH` = stub `bin` + `/usr/bin:/bin`, `PLAYA_DRY_RUN=1`, private spool,
a `claude` stub that appends `INVOKED` to a marker file:

- `claudine compose review-spec-inline.md spec=… --claude --dry-run` → exit 0;
  the metadata table lists `Deferred: interpolated at event-time: failure,
  start, success`. Prepare accepts the incident.
- The same without `--dry-run` → the marker file contains `INVOKED` (provider
  spawned), then exit 1 with `CompositionError: lifecycle evaluation error`
  whose reason is "unresolved interpolation survived event-time resolution"
  quoting `The review of the draft specification file in the {{ctx.repo_name}} repo has completed`,
  and the old "resolve the missing path or variable" hint. This is the exact
  incident. Every Phase 2–4 test group must record its own failing-before run
  against these fixtures; none exists yet because no rule exists yet.

**Raw-indentation assumption check — PASSED.** New unit tests
`darkmatter::…::expression::parser::tests::raw_literal_block_indentation::{review_spec_inline_say_blocks_parse_identically_raw_and_decoded, commit_resides_in_block_parses_identically_raw_and_decoded}`
parse the raw (indented) and YAML-decoded inner text of all three fixture
`|-` blocks: `parse`, `parse_condition`, and `parse_spanned(..).erase()` agree,
and the spanned root covers the trimmed raw text. Literal-block projection
through raw coordinates is sound; the projection design stands and no spike
is reopened.

**Companion-fix state.** `61f085043` is an ancestor of `HEAD`. No
`DeclaredExpressionValueKind`, `_with_types` lint API, or aggregate-suggestion
suppression exists (`rg` over `darkmatter/` and `claudine/`; the two
`_with_types` hits are unrelated test names). The post-companion ruling holds.

**Contract inventory (pre-edit).** GitNexus upstream impact: `prepare_document`
LOW (8), `iter_stack_expression_surfaces` LOW (6), `expression_values` **HIGH**
(8), `lower_mapping` LOW (5), `run_phase_1c_with_schema` LOW (4),
`reject_surviving_spans` LOW (13), `ExpressionFinder` **UNKNOWN** (struct;
confirmed by text search — `find_all_plain` has 7 Darkmatter and 17 Claudine
call sites, so treat it as broad).

- `ExpressionFinder` (`darkmatter/lib/src/markdown/compose/expression/lexer.rs:95`):
  `find_all`/`scan` skip code regions; `find_all_plain`/`scan_plain` (:261/:271)
  do not. `{{{` is checked first, then brace-depth-aware `scan_legacy_expression`
  (:212). No backslash handling exists in the finder.
- `prepare_document` (`claudine/lib/src/composition/prepare/service.rs:100`)
  dispatches to four prepare variants; callers are `cli/src/commands/compose/mod.rs:401`
  and `cli/src/commands/wrap/harness_orch/prompt.rs:297`. The only lifecycle
  validator wired in production is `validate_no_err_in_no_error_events`
  (`composition/prepare.rs:533` direct, `:671` inline); the comment at
  `:524–532` records that leak/undefined scans were removed deliberately.
- **Neither `validate_no_interpolation_leaks` (validate.rs:1) nor
  `validate_no_undefined_lifecycle_variables` (validate.rs:401) has a production
  call site** — both are test-only exports.
- `iter_stack_expression_surfaces` (validate.rs:107, private) walks
  `LifecycleSignal::ALL` = `[Initialize, Start, Success, Blocked, Failure,
  Finalize, Loop]`, stack items in order, `when` before actions. It yields
  **stack surfaces only**: event-level communication fields
  (`LifecycleNotification.say`, `message`, …, stored as raw `Option<String>`)
  and `loop.while`/`loop.until` (`LoopCondition` in `looping/config.rs:335`)
  are not yielded. `LifecycleExpressionSurface` = `{ property, signal, expr }`.
- `expression_values` is `pub(crate)` at `darkmatter/dmls/src/providers/frontmatter.rs:896`
  (not `overlay/expressions.rs`). Scalar entries with an expression-typed arm
  only. Consumers: hover (`expression_value_at` :926, `expression_hover` :941),
  `schema_problem_diagnostics` suppression (`diagnostics/frontmatter.rs:227`),
  and `expression_diagnostics` (`:618`; MALFORMED `WARNING` :636,
  UNKNOWN_IDENTIFIER `INFORMATION` :653).
- `lower_mapping` (`dmls/src/overlay/frontmatter.rs:379`) recurses only into
  mappings; a sequence is one `FmValueKind::Sequence` entry.
  `test_entry_or_ancestor_falls_back_for_array_index` is at `:565`.
- `run_phase_1c_with_schema` (`cli/src/commands/wrap/sequence/phase1c.rs:62`),
  sole caller `wrap/sequence/mod.rs:558`.
- `reject_surviving_spans` (`lifecycle/executor.rs:1847`, deep variant `:1864`)
  returns `LifecycleExprError::prose(..)` — the hint comes from the generic
  evaluation-error renderer, not from this function.
- Rescan tests `rescans_replacement_text_for_nested_interpolation` (`rewrite.rs:553`)
  and `rescans_false_branch_for_nested_interpolation` (`:571`) exist and stay
  unmodified. `interpolate_value` `:254`, `whole_value_span` `:290`.
- No `is_pending_expression_value` exists. The pending logic is inline in
  `darkmatter/lib/src/markdown/schemas/format.rs:243–253` (contains `$(` or
  `{{` → accepted) plus `schemas/mod.rs:1479` `scan_pending_values`.
- Severity assertions today: only
  `unknown_expression_root_is_informational_frontmatter_diagnostic`
  (`dmls/src/diagnostics/frontmatter.rs:1195`) asserts a severity. Malformed
  tests (`:1102`, `:1357–1445`) assert count/source only.

**Mapping-only DMLS corpus.** No existing corpus or snapshot suite exists. The
regression corpus is the 25 documents whose frontmatter contains no YAML
sequence, taken from `HEAD` `3df43cc67` (read via `git show`, not the working
tree, because two prompts carry uncommitted user edits):
`prompts/{_agent-skills, _docs, _implement/_fm, _interactive-prompting,
_repo-context, _reviews/observability, _reviews/performance-review,
_reviews/review-implementation, _reviews/review-spec-inline,
_reviews/suggestion-review, _writing-clearly, brainstorm, daily,
dependency-risk, design, dream, feature, lineage, sentrux, snippets/debug,
snippets/test-rigor, summary-and-suggest, validate}.md` and
`darkmatter/dmls/tests/fixtures/suggest_constraint/{inline, raw-consumer}.md`.
Phase 3 materializes them as frozen fixtures and captures the baseline
diagnostics **before** editing `lower_mapping`.

**Lifecycle event inventory (parity fixture).** Schema
(`darkmatter/docs/schemas/claudine.yaml:18,23–28`): `initialize`, `start`,
`blocked`, `success`, `failure`, `finalize` → `lifecycle-event`; `loop` →
`loop-event`. Claudine: `LifecycleSignal::ALL` (same seven, different order —
parity must compare sets). Communication fields (both event types): `say`,
`say_first`, `effect`, `message`, `stderr`, `stdout`, `info`, `warn`,
`success`, `notify`, `stack`. Predicates: `lifecycle-stack-item.when`,
`loop-event.while`, `loop-event.until` — all declared in
**`darkmatter/docs/schemas/claudine-types.yaml`** (`:11`, `:39`, `:40`) as
`string`, not in `claudine.yaml`.

**HIGH-risk gate.** `expression_values` stays a review gate: no severity
change lands until hover, `schema_problem_diagnostics`, and
`expression_diagnostics` pass focused tests under the new projection model.

### Phase 2 — Darkmatter Expression Authorities (2026-09-16, claude/opus)

Details and per-test failing-before evidence are in
`implementation-log.md` → `## Phase 2`.

**Foundation Gate — satisfied.**

- **One scanner.** `ExpressionFinder::scan` honors the odd-run backslash
  escape for `{{` and `{{{`, and every `find_all`/`find_all_plain`/`scan_plain`
  consumer inherits it. Depth counting inside an open span is unchanged.
- **One whole-value classifier.** `whole_value_span` now lives in
  `expression/lint.rs`. `interpolate_value` and the public
  `is_whole_value_span` both call it, and a property test pins agreement with
  `interpolate_value`'s actual branch.
- **One pending predicate.** `schemas::format::is_pending_expression_value` is
  called by the `darkmatter-expression` format validator.
- **One lint/rewrite.** `expression::lint` provides `lint_expression`,
  `lint_spanned`, `ExpressionLint`, and
  `ExpressionLintKind::NestedSpanInStringLiteral`. Following the
  post-companion ruling, there is no type plumbing and arrays and objects are
  offered rewrites.
- **Rescanning behavior unchanged.** Both `rewrite.rs` rescan tests are
  unmodified and green, and a body composition of a nested literal still
  resolves.

**Implementation decisions that refine the spec (no ruling needed):**

1. **Shared suggestion.** A suggestion rewrites the whole expression and
   repairs every flagged literal, so every lint from one expression carries
   the same text.
2. **Relaxed anchor rule.** An anchor is relaxed only after an earlier piece
   holds a character no `f64` string can contain. "Piece cannot parse as a
   number" is insufficient: `"1" + "e5"` is `"1e5"`.
3. **Self-check is AST equivalence.** The rewrite must re-parse to the
   intended tree in every candidate dialect and must leave no nested-span lint.
   Otherwise the suggestion is `None`, which also covers object keys, doubly
   nested literals, `{{{ }}}` pieces, and a lifted `||` under the condition
   dialect.
4. **Dialect recovery.** `lint_spanned` recovers the dialect by re-parsing the
   source.
5. **Known raw-vs-decoded parity edge.** Detection runs on the raw slice, as
   specified. A literal authored `'\\{{ x }}'` is flagged even though the
   runtime text is escaped.

**Gates (macOS):**

| Gate | Result |
| --- | --- |
| Focused Darkmatter (`expression::`, `interpolation::`, `schemas::format`) | 979/979 passed |
| `just test` in `darkmatter/` (lib, CLI, DMLS, zed CLI) | 7871 passed, 7 skipped |
| `just test --no-fail-fast` in `claudine/` | 7166 passed, **2 failed**, 9 skipped |
| `just lint` in `darkmatter/` | passed |
| `just lint` in `claudine/` | passed |

Both Claudine failures come from another session's concurrent, uncommitted
edits in files this fix does not touch:

- `dispatch_inventory_matches_committed_file`: sites `1611 → 1613`, from the
  `cli/src/commands/wrap/*` and `provider_overlay` edits.
- `repository_test_placement`: `lib/src/mcp/inject.rs` has 349 inline test
  lines, over the 300 limit.

Every test in `wrap_compose_validation` passes, including the fixture guard.

**Graph:** GitNexus upstream impact for `ExpressionFinder.scan` is `UNKNOWN`.
That is resolved by Phase 1's text search (24 `find_all_plain` sites) and by
the full Darkmatter and Claudine L1 runs. `interpolate_value` is MEDIUM (15),
and its logic is unchanged because the helper was only moved.

**Cross-OS:** not run. The changes are byte-level string scanning plus
`include_str!` of LF-normalized fixtures, with no path or process surface. CI
supplies the Linux, native Windows, and WSL2 L1 results in Phase 6.

### Phase 3 — DMLS Frontmatter Model (2026-09-16, claude/opus)

Details, per-test failing-before evidence, and the requirement-to-test map are
in `implementation-log.md` → `## Phase 3`.

**Overlay Gate — satisfied.**

- **Sequence entries.** `FmEntryRole`, `Option` key spans, `FmScalarStyle`,
  and explicit-tag detection (`NodeMeta.tag_loc`; the loader resolves a tag
  onto every untagged node) are covered by focused overlay L1.
- **Bracketed paths and array schema traversal.** `FmPathSegment`,
  `format_dotted`, `path_at`, a fail-closed typed `nested_shape`, and a
  memoized context-aware `def_at_path_ctx` are tested against the shipped
  Claudine schema and a discriminated array-item union.
- **Shape memo.** On `implement-plan.md` a pass materializes at most one
  shape per distinct ancestor path.
- **HIGH-risk `expression_values` gate.** Hover, `schema_problem_diagnostics`,
  and `expression_diagnostics` pass focused tests under the projection model.
- **Mapping-only output unchanged.** The 25-document corpus baseline was
  captured before `lower_mapping` changed and is byte-identical after.
- **Unsafe scalar styles.** `|`, `|-`, `|+`, `|2-`, `>`, `>-`, tagged, and
  multi-line values keep the generic schema diagnostic and receive no
  `dm.expression.malformed`.

**Deliberate behavior and assertion changes (not regressions):**

1. `test_entry_or_ancestor_falls_back_for_array_index` was re-cut as
   `test_entry_or_ancestor_is_exact_for_array_item`. This is the
   item-entries design switch.
2. Array schema problems now range the failing element.
3. Hover on a sequence's `- ` markers, or on a collection-valued item, is
   silent. It previously showed the enclosing sequence property.
4. Expression hover and the malformed producer require an exactly projected
   value. Block and tagged expression values fall through to the schema hover
   and the schema error.
5. Lifecycle `when` / `while` / `until` are `expression`-typed in
   `claudine-types.yaml`. This is editor-scoped: the Claudine runtime does not
   load the file.

**Design decision:** completion ancestry stays key-only
(`ArrayCrossing::Transparent`), matching the indentation fallback. Addressed
entries use `ArrayCrossing::Explicit`.

**Gates (macOS):**

| Gate | Result |
| --- | --- |
| `cargo nextest run -p dmls` | 696/696 |
| `just test` in `darkmatter/` | 7904 passed, 7 skipped |
| `just lint` in `darkmatter/` | passed |
| `just test --no-fail-fast` in `claudine/` | 7168 passed, 9 skipped, 0 failed |
| `just lint` in `claudine/` | passed |

**Cross-OS:** not run on rigs. Test paths were reviewed for Windows. CI
supplies Linux, native Windows, and WSL2.

### Phase 4 — Claudine and DMLS Consumers (2026-09-16, claude/opus)

Details, the full requirement-to-test map, and per-change failing-before
evidence are in `implementation-log.md` → `## Phase 4`.

**Agreement Gate — satisfied.**

- **Inventory.** Claudine
  (`nested_span::single_pass_inventory_matches_the_authored_claudine_schema`)
  and DMLS (`nested_span_tests::lifecycle_inventory_matches_the_authored_claudine_schema`)
  each test their single-pass inventory against `claudine.yaml` and
  `claudine-types.yaml`.
- **Offending span.** Both report an expression's first nested span. For the
  incident, both name `{{ctx.area}}` in `success.say`.
- **Rewrite bytes.** Both assert byte equality with Darkmatter's
  `lint_expression` suggestion
  (`incident_is_rejected_at_success_say_with_the_shared_rewrite`,
  `incident_rewrite_is_the_darkmatter_lint_suggestion_verbatim`).
- **Fail before spawn.** A provider invocation marker proves no provider
  starts for:
  - direct compose, `--dry-run`, and inline-compose;
  - a proxy handoff;
  - a sequence whose step two references the incident, and a sequence with a
    defect in its own lifecycle (no step starts);
  - `when` predicates, action operands, `proxy … with` values, and
    `loop.while`;
  - a `retry` / `resume` re-entry;
  - a loop seed.
- **Runtime backstop.** It is reachable only for template text that resolves
  at event time. It names the property and selects its hint from a typed
  reason.
- **DMLS.** Quick fixes produce parseable documents with zero remaining
  diagnostics, and the severity ladder tests pass.

**Deliberate behavior and assertion changes (not regressions):**

1. `validate_no_interpolation_leaks` and `CompositionError::LifecycleInterpolationLeak`
   are deleted. The dead-entry-point tests in `lifecycle/tests/{validation,diagnostics}.rs`
   were removed and their coverage recut through the new validator.
2. `LifecycleEvaluationError` gained `property` and `reason`. The
   missing-path hint no longer appears for a surviving span.
3. DMLS severities changed:
   - `unknown_identifier` is now `WARNING`;
   - schema-typed, exactly projected `malformed` is now `ERROR`;
   - three DMLS assertions were re-cut;
   - the mapping-only corpus was re-blessed, with a diff of exactly 8 body
     unknown-identifier severity lines (3 → 2).
4. The sequence referenced-document pre-scan runs right after the static
   preflight graph is built, which is before shell approval and Phase 1c. It
   reuses the graph's resolution, so no new file-reference code was needed.

**Gates (macOS):**

| Gate | Result |
| --- | --- |
| `cargo nextest run -p dmls` | 714/714 |
| `just test` in `darkmatter/` | 7922 passed, 7 skipped |
| `just lint` in `darkmatter/` | passed |
| `just test --no-fail-fast` in `claudine/` | 7189 passed, 9 skipped, 0 failed |
| `just lint` in `claudine/` | passed |

**Cross-OS:** not obtained.

- `build-win-native` failed the patch upload with "No space left on device".
- `build-linux` was locked by an unrelated job running since 2026-09-14.

CI supplies Linux, native Windows, and WSL2.

### Phase 5 — Authored Content and Documentation (2026-09-16, claude/opus)

Details are in `implementation-log.md` → `## Phase 5`.

**Authoring Gate — satisfied.**

- **Live prompts.** Both live defects were already repaired with `+`; no
  expression changed.
  - **Finding:** the repaired `prompts/_reviews/review-spec-inline.md` still
    could not compose, because of two broken `::file` paths introduced by
    `dd2bccf39`. `_writing_clearly.md` became `../_writing-clearly.md`, and
    `./_set_spec_schema.md` became `../_set_spec_schema.md`.
    `prompts/_interactive-prompting.md` had the same underscore typo.
  - Only those three lines were edited. The author's concurrent working-tree
    edit is preserved.
- **Acceptance 3 tests** (`claudine/cli/tests/shipped_prompt_contract.rs`):
  - a passive corpus test with the shared validator over every shipped
    prompt, using the incident as a negative control;
  - end-to-end `success` and `failure` runs of the live review prompt;
  - an end-to-end run of the live commit prompt.

  Failing-before: with the `cd6e036c4^` bytes swapped into the live paths,
  the corpus test and both review end-to-end tests failed. The commit
  end-to-end test passes either way, because `resides_in` is consumed only by
  the rescanning body; it is a positive control.
- **Docs and skills** describe the shipped behavior:
  - `claudine/docs/topics/lifecycle.md` and its skill snapshot, where the
    stale `LifecycleInterpolationLeak` / `validate_no_interpolation_leaks`
    claim was removed;
  - `darkmatter/docs/inline/interpolation.md`;
  - `darkmatter/docs/lsp/features.md` §4.1/§4.3, including the severity
    ladder;
  - `darkmatter/dmls/docs/diagnostics.md` (reconciled);
  - `claudine/lib/README.md` compatibility note;
  - `claudine/docs/topics/composition.md` and its skill snapshot;
  - Claudine `SKILL.md` and `timeline.md`;
  - Darkmatter `compose.md` skill.

  Every edited file carrying `hash:` was re-stamped with `md hash --save` and
  verified.
- **Comment audit.** A read-only sub-agent audited all Phase 1–4 `.rs`
  changes and found six real drifts (plus one borderline case). All were
  confirmed against the code and fixed comment-only:
  - `lexer.rs` module and `ExpressionFinder` docs (inline code spans are
    scanned);
  - `validate_no_nested_spans_in_literals` source of authored text;
  - `visit_string_literals` stale "leak" wording;
  - `LifecycleNestedSpanInLiteral` raise sites (the sequence pre-scan too);
  - `LifecycleErrorInfo::reason` (no chain walk);
  - DMLS `expression_diagnostics` range claim;
  - `nested_span.rs` parity sources.

  No stale `validate_no_interpolation_leaks` /
  `LifecycleInterpolationLeak` reference remains in Rust sources. No doc
  promises folded/tagged projection or hard body errors.

**Gates (macOS):**

| Gate | Result |
| --- | --- |
| `just test --no-fail-fast` in `claudine/` (final) | 7193 passed, 9 skipped |
| `just lint` in `claudine/` | passed |
| `just lint` in `darkmatter/` (comment-only source edits) | passed |

The first `claudine/` run failed exactly one test,
`spawn_site_guard::migrated_l1_tests_keep_the_isolation_the_builder_gave_them`:
the new commit test staged its file with `.current_dir`. It now uses
`git -C`.

`just test` in `darkmatter/` was not re-run: Phase 5's Darkmatter/DMLS
source edits are comment-only, and lint compiled them.

**Cross-OS:** not run.

- The new end-to-end tests are `#[cfg(unix)]`, like their sibling shipped
  prompt tests.
- The corpus test and the prompt path fixes are separator-neutral.

CI supplies Linux, native Windows, and WSL2.

### Phase 6 — Validation, Review, and Handoff (2026-09-16, claude/opus)

Details, including the full 19-criterion acceptance map, are in
`implementation-log.md` → `## Phase 6`.

**Gates (macOS, this phase):**

| Gate | Result |
| --- | --- |
| `just test` in `darkmatter/` | 7922 passed, 7 skipped, 0 failed (incl. `no_side_effects`, `packaging_contract`) |
| `just test --no-fail-fast` in `claudine/` | 7193 passed, 9 skipped, 0 failed |
| `just lint` in `darkmatter/` (serial) | passed |
| `just lint` in `claudine/` (serial) | passed |
| GitNexus `detect_changes` (scope `all`) | complete: 365/365 symbols, not partial or truncated; 90 files, 13 flows, risk high; the mix includes the concurrent shadow-home diff |

**Deliberate assertion changes (carried, none new):**

- the array-item fallback re-cut (Phase 3);
- three DMLS severity assertions and the 8-line corpus severity re-bless
  (Phase 4);
- the dead-entry-point leak tests recut through the new validator (Phase 4).

**Graph review:** the new Claudine flows are `resolve_emit` /
`emit_top_level` (the typed backstop) and `execute_sequence` (the pre-scan),
and both are covered. The `construct_argv_and_system_prompt` flows belong to
shadow-home. The forbidden-shape audit is clean for:

- the quadratic arena (the memo is `ShapeMemo`);
- hand-rolled file references;
- message-parsed code actions;
- rescanning;
- hint selection;
- out-of-scope edits.

Two items are recorded and accepted:

- **Lifecycle source map.** `LifecycleSourceMap` looks up authored text for
  each surface the canonical iterator yields, by its property path, and fails
  closed on a miss. It is not attached to the surface record as D2 describes.
  The `while`/`until` pair is spelled in two files.
- **Unknown-root check.** `expression_root_is_unknown` does a linear
  `entry_by_dotted` for each expression value. That code predates this fix,
  but sequence descent now reaches it more often.

**Cross platform — not obtained; task left unchecked:**

- No CI run exists, because pushing is outside this session.
- `build-linux` is blocked by a stale-looking `nightly-reward-spike` lock from
  2026-09-14, which was left in place.
- `build-win-native` has `W:` at 0 GB free.
- `build-win` (WSL2) refuses SSH.
- Portability was reviewed instead: LF fixtures, no absolute paths in the
  baseline, a Windows marker stub, and `#[cfg(unix)]` on `sh`-dependent tests.
- **Residual WSL-archive watch item:** five new runtime
  `env!("CARGO_MANIFEST_DIR")` fixture reads. There are 33 precedents at
  `HEAD`.

**Acceptance:**

- Criteria 1–18 map to passing tests or recorded review evidence.
- Criterion 8 is satisfied in its post-companion form, where arrays are also
  offered a rewrite.
- Criterion 19 is satisfied on macOS only, and stays open until CI reports
  Linux, native Windows, and WSL2.

**`spec.md` drift recorded for the author (not edited):**

- acceptance 3's "outstanding work" note;
- acceptance 8 and D1's interim array suppression;
- D2's "attach to the surface" wording;
- the `nested_shape_for_completion` name.

The fix is implementation-complete and ready for author review. It was not
moved to `_completed`, and nothing was committed or pushed.
