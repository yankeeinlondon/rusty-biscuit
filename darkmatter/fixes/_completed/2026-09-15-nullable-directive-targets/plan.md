---
total_phases: 5
created: 2026-09-15
phase: 5
agent: "codex/default"
yolo: "true"
completed_phase: "5"
implemented: true
source_code:
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
  - darkmatter/lib/tests/directive_target_analysis.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/lsp_session.rs
documentation:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/simplified-schemas.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/spec.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_1:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/spec.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_2:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/spec.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/tests/directive_target_analysis.rs
docs_updated_during_phase_3:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_4:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/simplified-schemas.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
human_review: false
human_review_items: []
message_to_agent: >-
  Phase 5 is implementation-complete and ready for review. There is no subsequent
  implementation phase; do not move this fix to `_completed` until the author closes review.
---

# Implementation Plan — Nullable Directive Targets

## Summary and Successful Completion

This fix separates authored directive grammar from evaluated target state. Darkmatter will parse
the authored `::file`, `::code`, and `::url` target once through the shared span-carrying directive
API, preserve typed null and empty-string results during interpolation, and skip only directives
whose whole-value target evaluates to an absence sentinel. Runtime composition remains
condition-aware, shell-approval preflight remains condition-blind, and pending shell-derived
targets continue to fail closed. The same Darkmatter-owned analysis will classify nullable target
expressions and block-condition narrowing for DMLS without executing expressions or duplicating
schema semantics.

Successful completion means all of the following are observable:

- The minimal guarded fixture completes through the real `md compose` preflight lifecycle with an
  unset optional `log`, emits no output for `::file {{log}}`, and does not weaken discovery in
  concrete sibling branches.
- An unguarded evaluated-null or evaluated-empty whole-value target is skipped with one typed
  compose warning, while authored-empty, quoted-empty, mixed, and otherwise malformed targets keep
  their existing grammar/path behavior.
- A pending frontmatter-shell target cannot disappear during approval discovery and later reveal an
  unapproved transcluded command.
- Full-file excerpts and directive line numbers use the same file-relative coordinate space at
  every audited transclusion-parser call site.
- DMLS suppresses `dm.transclusion.broken_path` for interpolated targets and emits exactly one
  `dm.transclusion.nullable_target` warning for a statically nullable, un-narrowed whole-value
  target, ranged on the expression.
- Darkmatter, `darkmatter-cli`, and DMLS tests, lint, build, passive corpus checks, and required
  documentation are green without introducing platform-specific behavior.

## Phase 1 — Rules, Baseline, and Fail-First Evidence

### Necessary Rules

- Preserve the established pipeline split: ordinary composition evaluates page blocks before body
  interpolation/transclusion, while approval preflight scans every branch without evaluating page
  block conditions.
- Treat only a whole-value target expression that evaluates to JSON null or `""` as an evaluated
  absence. A literal missing target and `::file ""` remain authored syntax errors; a mixed target
  such as `"{{dir}}/log.md"` remains an ordinary string/path target.
- Parse authored directives once through `markdown::compose::directives_api` and use its target
  spans. Do not infer provenance by diffing interpolated text or add another directive/block
  recognizer.
- Run pending-shell dependency detection against the authored target before any absence rewrite.
  Unknown or post-approval target shapes fail closed rather than being skipped.
- Keep static analysis passive and Darkmatter-owned. DMLS consumes Darkmatter's expression,
  schema-nullability, context-descriptor, and guard-narrowing results; it does not evaluate an
  expression, resolve a file to classify nullability, or implement a second schema resolver.
- Schema `default(...)` remains metadata and does not establish a runtime value. Unsupported schema
  shapes or expressions classify as `Unknown`, never as safe/non-null by guesswork.
- Recognize narrowing only from the specification's closed AST forms. `file_exists(x)`, truthy
  `x`, `!!x`, successful `x != null` / `x != ''`, parentheses, and `&&` may narrow; `||`, a single
  `!`, and all unrecognized forms do not.
- Do not modify the unrelated prompt defects or the sibling nested-span, optional-parameter,
  silent-context, or array-rendering fixes.

### Risk Spikes

- [x] **Verify Span Contract**
  - Prove with focused tests that `scan_darkmatter_directives` returns the raw target span needed to
    distinguish bare, quoted-empty, whole-value, and mixed targets for `::file`, `::code`, and
    `::url`, including CRLF input and directives inside code regions.
  - Confirm an end-to-start rewrite can remove a full directive line without invalidating other
    spans, and identify the shared compose module that can be called by both terminal interpolation
    and preflight without introducing a second parse.

- [x] **Verify Typed Evaluation**
  - Exercise the existing interpolation parser/evaluator with the request's `EvaluationLookup` and
    confirm it can preserve JSON null, empty string, and concrete string results for a parsed
    whole-value target before stringification.
  - Trace pending frontmatter-shell literals into directive target spans and prove the current
    dynamic-shape policy can reject them before a nullable rewrite. Escalate only if the existing
    dependency data cannot identify the target safely; otherwise record that no design change is
    needed.

### Work-group 1A — Baseline and Impact

These tasks may run concurrently and must finish before production code changes.

- [x] **Capture Baseline**
  - From `darkmatter/`, run `just build`, `just test`, and `just lint`; record any pre-existing
    failures separately from this fix.
  - Capture the authoritative minimal reproduction output and exact source revision/diff. Do not
    attribute live-prompt evidence to `6f5b06251`; omit live-prompt evidence if its nullable working
    state cannot be reconstructed honestly.

- [x] **Record Blast Radius**
  - Re-run GitNexus impact analysis for the exact transclusion parser, interpolation stage,
    `collect_recursive`, shared directive scanners, and DMLS `transclusion_diagnostics` symbols.
  - Record the affected callers/processes and treat the transclusion parser's current HIGH risk as
    a review gate: preserve parser behavior and cover every changed call path rather than widening
    the parser contract casually.

### Work-group 1B — Fail-First Tests

The tasks may be authored concurrently after the span/evaluation spikes, provided overlapping test
files are coordinated. Every new behavior test must be demonstrated failing on the pre-fix code.

- [x] **Pin Runtime Boundaries**
  - Add Darkmatter L1 cases for guarded null success, condition-blind preflight success with no
    null child edge, an unguarded null warning, evaluated-empty-string reason, authored-empty and
    quoted-empty errors, mixed-target preservation, and malformed syntax in a false block having
    different runtime/preflight outcomes.
  - Include a concrete sibling branch containing a discoverable child command so the primary
    regression test proves preflight did not become condition-aware or stop scanning after the
    absent target.

- [x] **Pin Security Boundary**
  - Add library and CLI fail-first cases in which a directive target depends on a pending
    frontmatter-shell value and could reveal a child `::shell`; assert rejection occurs before
    approval and the command never executes.
  - Route the CLI case through `CliProcessFixture` with fixture-owned environment and paths; do not
    hand-build or de-isolate an `md` process.

- [x] **Pin Diagnostic Defects**
  - Add the three-frontmatter-length line-number matrix and assert the reported file line contains
    the malformed directive.
  - Add executed DMLS cases proving the current interpolated target produces the false
    `dm.transclusion.broken_path`, and reserving expectations for nullable/guarded/required/default
    behavior and target-span range.

- [x] **Pin Corpus Coverage**
  - Extend the existing shipped Markdown fixture corpus near the transclusion engine's passive
    corpus test so every checked-in directive target is scanned by `directives_api` without
    composition, file resolution, process execution, or network access.

**Checkpoint:** baseline results and graph risk are recorded; the incident, security boundary,
line-coordinate defect, DMLS false positive, and nullable diagnostic matrix fail for the expected
pre-fix reasons; all unrelated tests remain green.

## Phase 2 — Runtime Target Evaluation and Preflight Safety

### Work-group 2A — Shared Target Rewrite

These tasks are sequential because each consumes the preceding contract.

- [x] **Add Target Model**
  - Introduce focused Darkmatter-owned result types equivalent to concrete versus absent targets,
    with distinct null/empty-string reasons and an optional parsed expression root for diagnostics.
  - Expose only the minimum surface needed by composition and later passive analysis; retain source
    spans and directive kind without changing `ContextValueType`.

- [x] **Evaluate Whole Targets**
  - Build one span-aware helper over `directives_api` that evaluates only whole-value target spans
    for `::file`, `::code`, and `::url` through the existing `Evaluator`/`EvaluationLookup`.
  - Rewrite concrete results, remove absent directive lines from end to start, emit one typed
    warning for an unguarded runtime absence, and leave authored-empty, quoted-empty, mixed, code-
    region, and malformed directives to their existing paths.

- [x] **Wire Compose Walks**
  - Invoke the helper at the start of ordinary body interpolation, after page blocks, using the
    existing resolving lookup and compose report.
  - Invoke the same helper for condition-blind preflight before recursive transclusion parsing,
    while preserving concrete sibling discovery and returning no graph edge for an absent target.
  - Ensure a guard removed by ordinary page-block evaluation suppresses the runtime warning, while
    an unguarded absent directive records exactly one warning.

- [x] **Enforce Pending Rejection**
  - Extend authored-target dynamic-shape detection so a pending shell-derived target is rejected
    before the rewrite and before child resolution.
  - Verify graph reuse cannot convert an approved absent/concrete target into a later unapproved
    child command.

### Work-group 2B — Independent Corrections

These tasks may run concurrently with Work-group 2A after Phase 1 because they do not depend on the
new nullability classifier.

- [x] **Correct Line Coordinates**
  - Audit every transclusion-parser call that combines body-relative directive lines with either
    `source_context_for_errors()` or `full_source_context_for_errors()`.
  - Add `frontmatter_line_count()` exactly once for full-file contexts, preserve already-consistent
    body-only callers, and make the parameterized line assertions pass.

- [x] **Suppress False Path**
  - In DMLS `transclusion_diagnostics`, skip `broken_path` classification for any target containing
    an interpolation span because its local path cannot be known statically.
  - Preserve `broken_path` for concrete missing files and confirm the executed before/after
    diagnostic evidence.

**Checkpoint:** the guarded and unguarded compose/preflight matrices pass; pending targets fail
closed; authored syntax boundaries remain errors; all corrected excerpts point to the actual
directive; concrete DMLS missing paths still warn while interpolated paths do not.

## Phase 3 — Passive Nullability and Guard Narrowing

### Work-group 3A — Static Classifiers

The two classifier tasks may run concurrently behind shared expression-path conventions, then join
for integration.

- [x] **Classify Nullability**
  - Add a passive Darkmatter API that accepts a parsed whole-value expression, assembled
    `EffectiveSchema`, and static frontmatter, returning non-nullable, nullable with an
    `ExpressionPath`, or unknown.
  - For document parameters, use `EffectiveSchema.simplified` plus `origins`; classify only
    top-level document-owned or referenced SimplifiedSchema properties, honor `required`, do not
    apply `default(...)`, and let a concrete non-null frontmatter scalar suppress nullability.
  - For cataloged `ctx.*` paths, reuse `context_variable_descriptors().required` and `.default`;
    leave raw JSON Schema, baseline-only values, trigger payloads, root unions, unsupported nested
    properties, and complex expressions unknown.

- [x] **Analyze Guard Narrowing**
  - Walk the existing parsed condition AST and return narrowed `ExpressionPath` values for only the
    closed forms in Necessary Rules; compose nested blocks by unioning enclosing narrowed sets.
  - Add truth-table tests that compare every recognized form to real evaluator behavior, including
    parentheses, `a && b`, true/false inequality outcomes, nested blocks, and negative controls for
    `||`, single `!`, and unsupported expressions.

### Work-group 3B — Classifier Integration

- [x] **Integrate Target Analysis**
  - Combine target recognition, static nullability, and enclosing-block narrowing in one
    Darkmatter-owned passive analysis surface that retains the directive kind, expression root,
    and source span for consumers.
  - Make the classification matrix pass for optional `file`, optional `string(default(...))`,
    required `file`, explicit null, concrete non-null frontmatter, supported `ctx.*`, guarded
    targets, and unknown schema/expression cases.

- [x] **Audit Passive Behavior**
  - Prove the analysis performs no filesystem resolution, network access, expression execution,
    shell execution, or document mutation and degrades to `Unknown` when authority is missing.
  - Review all new public docs and nearby comments for accurate invariants, deleting any
    implementation narration that would drift.

**Checkpoint:** Darkmatter's passive API owns one tested nullability and narrowing rule, agrees
with evaluator truth tables, produces no false non-null classifications, and can be consumed by
DMLS without duplicating schema or expression logic.

## Phase 4 — DMLS Diagnostic and Documentation

### Work-group 4A — DMLS Integration

- [x] **Add Diagnostic Code**
  - Add `dm.transclusion.nullable_target` to the stable diagnostic code catalog with source
    `darkmatter.compose` and `WARNING` severity.
  - Feed the DMLS document's `overlay.bundle()` effective schema and static frontmatter into the
    Phase 3 analyzer; do not reassemble a schema in the DSL provider.

- [x] **Publish Nullable Warning**
  - Emit one warning only for a nullable, un-narrowed whole-value directive target and range it on
    the target expression span.
  - Use the specified message naming the expression root and explaining guard, binding, and
    `required` remedies; emit nothing for non-nullable, narrowed, mixed, or unknown targets.

- [x] **Complete DMLS Matrix**
  - Make executed tests pass for unguarded optional, direct and outer-nested guards, required,
    default-metadata, concrete missing path, and interpolated-path suppression cases.
  - Assert exact code, source, severity, multiplicity, message root, and target range so the new
    warning cannot coexist with the false `broken_path` warning.

### Work-group 4B — Documentation

This work-group may run concurrently with 4A once Phase 3's public contracts are stable.

- [x] **Document Runtime Semantics**
  - Update `docs/inline/interpolation.md` with evaluated-absence skip behavior and guard-based
    warning suppression.
  - Update `docs/topics/darkmatter-expressions.md` and
    `docs/topics/simplified-schemas.md` to state that schema defaults are metadata, optional uses
    remain nullable when unbound, and `required`/concrete binding are the non-null mechanisms.

- [x] **Document Diagnostics**
  - Update `dmls/docs/diagnostics.md` with the new stable code, warning severity, target range,
    narrowing behavior, and the `broken_path` exclusion for interpolated targets.
  - Update `darkmatter/README.md` only if its current diagnostic inventory requires the new code.

- [x] **Update Skill Contract**
  - Update `.claude/skills/darkmatter/compose.md` with the condition-aware runtime versus
    condition-blind approval invariant, the nullable-target behavior, the pending-target fail-closed
    rule, and the recommended `file_exists(...)` guard idiom.
  - Run `md hash .claude/skills/darkmatter/compose.md` after the content is final and verify the
    Markdown-aware frontmatter/body hashes are current.

**Checkpoint:** DMLS publishes only the correct nullable-target warning with exact range and
severity; guarded and unknown cases stay quiet; all specified user, diagnostic, and skill docs
match the implemented behavior.

## Phase 5 — Integration Verification and Evidence

### Work-group 5A — Focused Verification

These tasks may run concurrently before the full package-area gate.

- [x] **Verify Library Matrix**
  - Run the focused Darkmatter tests for directive scanning, target evaluation, interpolation,
    transclusion parsing, preflight recursion/graph reuse, pending command shape, nullability,
    narrowing, line coordinates, and the passive shipped-artifact corpus.
  - Confirm every new regression test was observed failing before its implementation and now
    passes for the intended reason.

- [x] **Verify CLI Lifecycle**
  - Run the minimal guarded and pending-target fixtures through the normal `md compose` entry point
    via `CliProcessFixture`, with shell preflight enabled and no host focus/input side effects.
  - Capture the minimal probe's before/after stdout, stderr, and exit status verbatim.

- [x] **Verify DMLS Output**
  - Execute the DMLS diagnostic path and capture before/after diagnostic listings for
    `::file {{log}}`, proving `broken_path` is gone, the unguarded nullable warning exists, and the
    guarded form is clean.
  - Re-run the existing transclusion diagnostic suite to prove concrete path and cycle behavior is
    unchanged.

### Work-group 5B — Final Gates

- [x] **Run Area Gates**
  - From `darkmatter/`, run `just build`, `just test`, and `just lint` for the library, CLI, and
    language server. Do not run `cargo fmt`.
  - Run `just test-l2` only if implementation moves behavior into a real-terminal integration
    surface; no browser gate is required for this non-rendering fix.

- [x] **Review Platform Safety**
  - Confirm target/span handling is byte- and line-ending-safe for LF/CRLF and introduces no
    macOS-only path comparison, shell, or filesystem behavior; preserve `FileReference` and the
    captured compose resolution context for all concrete targets.
  - Review qualifying macOS, Linux, and native Windows/WSL2 evidence through the repository's
    normal CI evidence policy rather than adding speculative matrix cells.

- [x] **Analyze Final Changes**
  - Run GitNexus `detect-changes --scope all` against this exact worktree, re-running if partial or
    truncated, and review every affected process plus any HIGH/CRITICAL risk before handoff.
  - Inspect `git diff` for scope drift, stale comments/docs, accidental changes to the sibling
    fixes, and unintended expectation changes. Record each deliberately re-cut line-number
    assertion.

- [x] **Record Acceptance**
  - Map the final test/evidence record to all nine specification acceptance criteria, including the
    security-complete pending-target case and the evaluated-empty versus authored-empty boundary.
  - Leave the fix implementation-complete and ready for review; do not move it to `_completed`, run
    `just complete`, or commit unless separately requested.

**Checkpoint:** every acceptance criterion has named passing evidence; package-area gates and
final graph analysis are clean; the diff contains only the nullable-target fix, its tests,
documentation, skill update, and plan evidence.
