---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-02T15:22:15+01:00
spec: 2026-09-01-file-param-anchoring/spec.md
implemented: false
description: A **fix** review of `2026-09-01-file-param-anchoring/spec.md`
fix: 2026-09-01-file-param-anchoring/review-2.md
previous: 2026-09-01-file-param-anchoring/review-1.md
---

# Review 2: File Parameter Anchoring

## Verdict

The fix is **ready for production**. All three findings from Review 1 are
resolved: the shipped `md compose` route now supplies launch-anchored caller
provenance, property and root unions preserve exactly-one-applicable-arm
selection, and the missing Level 1 verification rows now cover the shipped
planning prompt, post-shell stability, schema layering, idempotence, and
single trigger discovery.

No blocking functionality, correctness, ergonomics, performance, or test-rigor
gaps were found in this iteration.

## Findings

No findings.

## Review 1 Closure

### Public `md compose` anchoring

`run_compose` captures the launch directory once, builds a request-scoped
`FileResolutionContext`, and supplies that context plus the launch fallback to
both reference validation and final composition. Resolved documents outside
the launch repository receive a document context derived from the retained
snapshot, with source-repository and package-area discovery used only to shape
the document side of the context. Caller overrides continue to anchor at the
captured launch base.

At the library boundary, a supplied `FileResolutionContext` is now sufficient
to synthesize caller input records from raw overrides, so callers cannot
silently retain the former split-value behavior merely because they omit the
compatibility fallback field.

Level 1 process tests invoke the shipped `prompts/plan.md` through the real
`md compose` binary from both the repository root and `claudine/` package area.
Both assert the complete plan path beside the same specification. Claudine's
own process regression now uses the same shipped prompt and real fix document.

### Applicable union selection and stability

Root-union selection now distinguishes exact matches from arms whose failures
are deferred solely because values remain composition-pending. It prefers one
exact arm, accepts one pending arm only when there is no exact match, and
declines to project for zero or ambiguous candidates. Property unions retain
the same exactly-one-applicable-arm behavior.

Level 1 tests cover an eager-file/plain-string property union, active eager and
non-eager discriminated root arms, ambiguous and zero-match root unions, and
bidirectional eager-mode drift after interpolation. A post-shell discriminant
change also fails with the typed `CallerFileClassificationChanged` diagnostic
when trigger schemas are disabled, closing the static-schema path that Review 1
identified.

### Remaining specification evidence

The added Level 1 coverage proves that post-shell expansion and frontmatter
pass 2 retain the caller projection's native semantic value, portable body
presentation, and derived path. A test-only counter verifies one trigger
registry discovery across the pre-shell and post-shell passes. Separate tests
prove projection installation is idempotent and that baseline and document
schemas layer correctly while an absent optional eager caller property remains
unprojected.

## Requirement Verification Levels

| User-facing requirement | Strongest verification present | Assessment |
|---|---|---|
| Eager caller values are projected before frontmatter pass 1 from repository-root and package-area launches | Level 1 Darkmatter library, `md compose` process, and Claudine process tests | Appropriate and present on every shipped route. |
| The shipped planning workflow derives one plan beside the input specification | Level 1 process tests using `prompts/plan.md` from both launch directories | Appropriate and present. |
| Frontmatter semantic state and body presentation identify the same file | Level 1 native-frontmatter and portable-body assertions, including post-shell/pass-2 coverage | Appropriate and present. |
| Caller resolution uses the captured launch context without prompt or ambient-CWD fallback | Level 1 context-only, process-launch, and post-capture CWD-mutation tests | Appropriate and present. |
| Lazy, ordinary-string, excluded, absent optional, and document-owned values retain their established behavior | Level 1 focused integration tests | Appropriate and present. |
| Scalar, array, property-union, and root-union shapes use only one applicable file declaration | Level 1 scalar, array, mixed property-union, discriminated root-union, ambiguous, and zero-match tests | Appropriate and present. |
| Dynamic eager typing fails closed after interpolation and after shell/pass 2 | Level 1 typed-error tests for trigger and root-union drift | Appropriate and present. |
| Malformed and missing eager caller values retain typed file diagnostics and launch provenance | Level 1 structured diagnostic tests | Appropriate and present. |
| Projection is idempotent and trigger discovery occurs once | Level 1 direct idempotence test and test-only discovery counter | Appropriate and present. |
| Native semantic and portable presentation paths preserve identity on macOS, Linux, and Windows | Platform-neutral Level 1 assertions plus an enabled Windows-specific test | Appropriate; macOS ran here and Windows remains assigned to Windows CI. |

Levels 2 and 3 are not applicable. This fix changes composition semantics,
filesystem anchoring, and structured diagnostics. It does not claim terminal
rendering, glyph layout, styling, scrolling, keybindings, paste/IME/mouse
behavior, or OS keyboard encoding.

## Verification Performed

- `darkmatter/just test`: **7,589 passed; 50 higher-tier tests skipped**.
- `darkmatter/just lint`: **passed** for `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- `claudine/just test`: **6,685 passed; 11 higher-tier tests skipped**.
- `claudine/just lint`: **passed** for all five package-area crates and the
  diagnostic guards.
- `git diff --check` and `git diff --cached --check`: **passed** after the
  review artifacts were written.
- GitNexus upstream impact: **low risk** for the reviewed CLI and shared
  projection symbols; one shared compose/transclusion execution flow is
  affected.
- GitNexus staged change detection: **medium aggregate risk**, with 30 changed
  symbols across 11 files and the expected compose pipeline as the sole
  affected indexed execution flow.

The executed gates establish macOS-host confidence. Cross-platform path
behavior remains covered by platform-neutral tests and a Windows-gated Level 1
test suitable for Windows CI.

## Production Readiness

Every acceptance criterion has implementation and appropriately leveled
verification. Caller eager-file values now have one launch-resolved semantic
identity across frontmatter expressions, schema/path operations, shell-pass
revalidation, and body presentation, while document-owned and lazy-file
contracts remain unchanged. No further changes are required for production
readiness.
