---
$schema: feature-review.yaml
ready: false
findings:
  - "High — Valid tagged expression scalars still lose provenance and DMLS diagnostics"
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T15:49:45-07:00"
spec: 2026-09-15-dasherized-identifiers/spec.md
implemented: true
description: "A **fix** review of `2026-09-15-dasherized-identifiers/spec.md`"
fix: 2026-09-15-dasherized-identifiers/review-3.md
previous: 2026-09-15-dasherized-identifiers/review-2.md
next: 2026-09-15-dasherized-identifiers/review-4.md
---

# Review 3: Dasherized Identifiers

## Verdict

The fix is **not ready for production**. Review 2's sole unblocked finding was
implemented for literal and folded block scalars and for multiline plain,
single-quoted, and double-quoted scalars. Composition now retains the authored
expression span through those YAML presentations, and DMLS parses the same
decoded text and projects its diagnostics back to exact document ranges.

Review 2 contained no blocked findings, so there was nothing to re-evaluate as
newly unblocked. Human review is not required; the remaining finding is an
implementation and Level-1 verification gap with no unresolved design choice.

## Findings

### High — Valid tagged expression scalars still lose provenance and DMLS diagnostics

Requirement 2 requires every fatal mixed-frontmatter expression error to carry
the original source path, line, and expression span, and Requirement 3 requires
DMLS to inspect every identifier in an Expression-typed frontmatter value.
The new shared decoder deliberately rejects a value beginning with `!`
(`lib/src/markdown/schemas/simplified/yaml_scalar.rs:198-205`), even though an
explicitly tagged string such as `!!str "{{ bad( }}"` is valid YAML and the
frontmatter parser supplies it to composition as an ordinary string. The
composition projector consequently falls back from `OnDiskSpan` to an
unpositioned error (`lib/src/markdown/compose/frontmatter_interpolation.rs:391-439`).

A direct file-backed `md compose` reproduction with
`title: !!str "prefix {{ upper( }}"` exited 1 and emitted no partial document,
but the diagnostic contained only the key-level YAML excerpt. It omitted the
`Expression at line: ..., column: ...` locus that the same expression receives
when authored as a plain, quoted, or block scalar. This is still a failure of
Requirement 2's typed provenance contract, not merely a presentation
improvement.

DMLS uses the same decoder and drops the value when decoding returns `None`
(`dmls/src/providers/frontmatter.rs:882-918`). An Expression-typed `!!str`
value therefore receives no parse or unknown-identifier diagnostics at all,
which leaves Requirement 3 incomplete on a valid scalar presentation. The
implementation log records tags and aliases as residual limitations, but the
spec does not exempt valid YAML string forms from either requirement.

Teach the shared source map to unwrap explicit scalar tags while retaining the
tag's authored prefix outside projected expression ranges. Also handle aliases
through their defining scalar when provenance is unambiguous, or reject them
at the feature boundary with a typed diagnostic rather than silently skipping
DMLS analysis. Add Level-1 library, CLI, and real in-memory LSP-session tests
for tagged Expression values, asserting fatality, exact authored byte/line/
column ranges, warning severity, and exact UTF-16 identifier ranges. Include an
alias case that pins the chosen provenance behavior.

## Requirement Verification Levels

All user-observable behavior in this specification is deterministic parsing,
composition, CLI process behavior, or LSP protocol behavior. Level 1 is the
appropriate tier. No requirement depends on terminal-emulator rendering,
terminal input encoding, OS keyboard events, paste/IME, mouse input, or
scrolling, so Level 2 and Level 3 are not applicable.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Dasherized identifiers preserve subtraction boundaries | Level 1 lexer/parser tables, library composition, spawned CLI, and LSP navigation | Correct level and behavior. |
| 2. Invalid full-document expressions fail with typed provenance and no partial output | Level 1 library and spawned CLI tests | Block and multiline scalar coverage now passes, but valid tagged strings still lose the expression span. **High gap.** |
| 3. DMLS checks every identifier in every operand position | Level 1 provider tests and real in-memory LSP sessions | Block and multiline Expression values now have exact ranges; valid tagged Expression values are skipped. **High gap.** |
| 4. Unknown roots warn only when absence is unhandled | Level 1 library and CLI tests | Correct level and behavior for analyzed scalar forms. |
| 5. Each issue is reported once | Level 1 library and CLI identity tests | Correct level and behavior. |
| Permanent shipped-artifact compatibility gate | Level 1 passive corpus and end-to-end shipped-prompt composition | Correct level and behavior. |
| Qualitative performance bounds | Level 1 deterministic identity-work regressions plus source inspection | The scalar map is linear in authored/decoded bytes and adds no cross-document lock. |

## Verification Performed

- Reconciled Review 2: its one unblocked finding is implemented; it had no
  blocked findings.
- Focused library failure-contract suite: 24 passed, 0 failed. This includes
  literal/folded block scalars, chomping and indentation indicators, CRLF,
  multibyte prefixes, and multiline plain/double-quoted values under both
  `fail_fast` settings.
- Source inspection confirmed that composition and DMLS share
  `decode_scalar_node`, compare decoded text with their parser-owned value, and
  project through the same byte map.
- The implementation's recorded post-fix Darkmatter run completed with 8,221
  passed, 0 failed, 14 skipped; its 80 load-related HTTP timeouts passed in
  constrained reruns. The recorded package-area lint gate passed.
- A direct file-backed tagged-string CLI reproduction confirmed the remaining
  gap: exit 1 and no partial output, but no expression-specific line/column
  locus.
- GitNexus was bound to the `feat-dark-fixes` worktree index (indexed after the
  Review 2 implementation). It resolved the DMLS `expression_values` callers
  but exposed no execution-flow process for this projection seam, so the
  behavior and unresolved decoder cases were confirmed by source inspection
  and the focused tests above.
