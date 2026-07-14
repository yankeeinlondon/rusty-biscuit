---
ready: true
agent: codex/default
created: 2026-07-10T18:04:01
---

# Review 2: Interpolation Literal

## Verdict

Ready for production. The implementation satisfies the specification, and the
three findings from review 1 are closed at the required Level 1 boundaries.
No requirement depends on terminal rendering or physical input, so Level 2 and
Level 3 verification are not applicable.

## Findings

No blocking or non-blocking implementation findings remain.

## Review 1 Closure

- Required DMLS behavior is now exercised through `ProviderRegistry` with a
  real `DocumentContext`, `SourceMap`, UTF-16 cursor positions, frontmatter/body
  boundaries, provider diagnostics, hover ranges, and Markdown markup
  lowering. The workspace graph test separately asserts that literals create
  neither `NodeKind::Interpolation` nodes nor `uses_variable` edges.
- Literal hover output now selects a backtick fence longer than any run in the
  composed text and uses a fenced block for multiline content. Provider-level
  tests cover embedded backticks, multiline content, Unicode, and composed
  output preservation.
- The frontmatter ordering case now runs through `Markdown::compose_with` with
  an explicitly pre-approved deterministic shell command. It verifies that the
  literal survives the shell-bracketed interpolation passes, remains a string,
  converts exactly once, contributes no interpolation replacement, and emits
  no warning.

## Requirement Verification Matrix

| Requirement | Strongest verification | Assessment |
|---|---:|---|
| Body and inline-code conversion, inert content, zero warnings/replacement count, and `fail_fast` success (AC 1, 2, 8, 14) | L1 rewrite and compose tests | Appropriate and complete. |
| Existing inline interpolation, tight/empty forms, adjacency, four-brace behavior, unclosed fallback, and fenced-code exclusion (AC 3–9) | L1 scanner and rewrite tests | Appropriate and complete. |
| Replacement-introduced literals survive body rescanning (AC 11) | L1 rewrite rescan test | Appropriate and complete. |
| Frontmatter string typing and shell-bracketed two-pass ordering (AC 10) | L1 full compose-pipeline integration test | Appropriate and complete. |
| Context capture and remote-discovery inertness (AC 12) | L1 consumer tests | Appropriate and complete. |
| DMLS diagnostics, UTF-16 hover routing/range/content, and frontmatter/body boundary (AC 2, 13) | L1 provider-registry tests | Appropriate and complete. |
| DMLS graph node and `uses_variable` edge inertness (AC 13) | L1 workspace-graph test | Appropriate and complete. |
| Terminal-dependent behavior | Not specified | L2/L3 are not applicable. |

## Design and Implementation Assessment

The shared `ExpressionFinder` remains the grammar authority and exposes
literals as a distinct scan product without duplicating recognition in its
consumers. Exact-three-brace maximal munch, first-closer behavior, legacy
fallback, code-region exclusion, and byte spans are implemented directly in
the scanner. Conversion is isolated to the final rewrite point for body and
frontmatter surfaces, which preserves literal inertness across rescans and the
frontmatter shell pass. Existing expression-only convenience methods retain
their API and naturally keep passive consumers from evaluating literal
content.

The implementation is suitably small for the feature. Reverse-order span
replacement avoids offset repair, and no additional abstraction or
performance work is warranted for production readiness.

## Validation

`just test` passed from `darkmatter/` using the prescribed nextest workflow:

- 5,295 darkmatter library tests passed; 111 configured non-Level-1 tests were
  skipped.
- 545 darkmatter CLI tests passed; 71 configured non-Level-1 tests were
  skipped.
- 401 DMLS tests passed with no skips.

The full-pipeline frontmatter test and the provider/graph regression tests all
passed in those runs. Level 2 and Level 3 were not run because the specification
contains no real-terminal rendering or OS keyboard-input requirement.
