---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-16T16:34:14-07:00
spec: 2026-09-15-nullable-directive-targets/spec.md
implemented: false
description: A **fix** review of `2026-09-15-nullable-directive-targets/spec.md`
fix: 2026-09-15-nullable-directive-targets/review-1.md
---

# Review 1: Nullable Directive Targets

## Verdict

The fix is **ready for production**. The implementation preserves the intended
condition-aware runtime and condition-blind approval split, represents evaluated
null and empty-string targets without routing them through directive grammar,
keeps authored-empty targets invalid, corrects full-file error coordinates, and
fails closed when a target depends on pending shell expansion.

Darkmatter owns both typed target evaluation and passive nullability/narrowing
analysis. DMLS consumes that shared analysis, suppresses the misleading
`broken_path` warning for interpolated targets, and emits the new warning only
for nullable, non-narrowed whole-value targets. I found no correctness,
security, performance, ergonomics, or verification defect that should block
release.

## Findings

### Low — Runtime warning metadata is not asserted as completely as the contract describes

The unguarded runtime tests verify one warning, the expression root, and the
typed null versus empty-string reason, but they do not assert the warning's
`line_number` or that its message names the directive kind
(`darkmatter/lib/src/markdown/compose/preflight/collect.rs:942` and
`darkmatter/lib/src/markdown/compose/preflight/collect.rs:960`). They also run
the runtime skip path only through `::file`, while the contract applies equally
to `::code` and `::url`.

The implementation is shared across all three kinds, records the adjusted file
line, and renders the kind from the parsed directive
(`darkmatter/lib/src/markdown/compose/directive_targets.rs:356` and
`darkmatter/lib/src/markdown/compose/directive_targets.rs:398`). Scanner tests
cover all three kinds and the passive integration test covers `::file`,
`::code`, and `::url`, so this is a non-blocking test-precision gap rather than
evidence of incorrect behavior. A compact table-driven Level-1 runtime test
should pin kind, line, root, and absence reason for all three directive kinds.

## Requirement Verification Levels

This fix changes deterministic parsing, composition, approval-graph discovery,
diagnostic production, and process execution policy. Level 1 is the appropriate
verification boundary for every user-observable requirement. Nothing in the
spec depends on terminal-emulator rendering, terminal input encoding, keyboard
events, paste/IME behavior, mouse behavior, or scrolling, so Level 2 and Level
3 are not applicable.

| Acceptance criterion | Strongest verification present | Assessment |
| --- | --- | --- |
| Guarded unset target succeeds through normal `md compose` preflight and transcludes nothing | Level 1 spawned CLI via `CliProcessFixture`: `compose_guarded_nullable_target_through_real_preflight_lifecycle` | Pass; correct level |
| Malformed directive lines use file-relative coordinates across at least three frontmatter lengths | Level 1 in-process preflight: `preflight_parse_errors_use_file_relative_lines` | Pass; checks both the numeric line and selected source text |
| Interpolated targets do not emit `dm.transclusion.broken_path` | Level 1 real LSP session: `interpolated_transclusion_target_does_not_report_broken_path` and `nullable_transclusion_diagnostic_matrix` | Pass; correct level |
| Unguarded nullable target emits one warning; supported direct and nested guards suppress it | Level 1 real LSP session plus evaluator-backed passive-analysis tests | Pass; exact code, source, severity, range, message, and roots are asserted |
| Runtime remains condition-aware and preflight condition-blind for malformed syntax | Level 1 in-process compose/preflight pair: `malformed_directive_in_false_block_has_distinct_runtime_and_preflight_outcomes` | Pass; correct level |
| Existing transclusion and DMLS behavior remains valid | Level 1 complete package-area suite plus passive shipped-fixture corpus and normal CLI shipped-fixture compose | Pass |
| Pending shell-derived targets cannot introduce unapproved child commands | Level 1 library test plus spawned CLI sentinel test: `pending_shell_target_is_rejected_before_child_command_approval` and `compose_rejects_pending_target_before_child_command_can_execute` | Pass; fail-closed behavior and non-execution are both observed |
| Schema `default(...)` remains metadata rather than a runtime fallback | Level 1 passive classifier and real LSP diagnostic matrix | Pass; the defaulted optional property remains nullable |
| Evaluated empty string is skipped while authored empty and quoted-empty targets remain errors | Level 1 in-process compose/parser tests | Pass; typed empty-string reason and authored boundaries are distinct |

## Implementation Assessment

- The target rewrite runs before ordinary body interpolation, preserving typed
  null and empty-string values until the directive decision is made.
- Preflight checks authored targets against pending shell values before its
  condition-blind interpolation pass, so an absent edge cannot conceal a child
  command whose target becomes concrete only after approval.
- The line-offset parser is used only where body-relative lines are projected
  into a full-file source context; ordinary body-only callers keep their
  existing coordinate convention.
- Passive nullability classification is conservative for unsupported schema or
  expression shapes and uses the existing effective schema, origins, static
  frontmatter, context descriptors, and parsed condition AST.
- The scanner change has a HIGH upstream blast radius in GitNexus (58 symbols
  across Compose, DMLS, Graph, and Overlay), but the full area suite passed and
  aggregate change detection reported LOW risk with no affected execution
  process.
- The implementation adds no target-specific branches or native path
  comparisons. LF and CRLF span behavior is covered, while concrete targets
  continue through the existing cross-platform file-resolution path.

## Verification

- Focused Level-1 matrix: 31 passed, 0 failed.
- Complete `just test`: 7,842 passed, 0 failed, 7 intentional higher-tier
  skips.
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, `dmls`,
  `zed-dmls-cli`, and the `zed-dmls` `wasm32-wasip2` compile check.
- `git diff --check`: passed.

The local commands used an isolated writable Cargo target plus the host's
Homebrew clang and Command Line Tools SDK because the shared target directory
contains read-only artifacts. The initial shared-target attempt failed before
compiling product code and is not counted as a product failure.
