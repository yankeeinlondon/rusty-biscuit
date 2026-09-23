---
$schema: feature-review.yaml
ready: false
findings:
  - "Medium — DMLS alias provenance performs quadratic document rescans"
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T17:11:56-07:00"
spec: 2026-09-15-dasherized-identifiers/spec.md
implemented: true
implemented_by: claude/fable
log: darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
next: 2026-09-15-dasherized-identifiers/review-5.md
description: "A **fix** review of `2026-09-15-dasherized-identifiers/spec.md`"
fix: 2026-09-15-dasherized-identifiers/review-4.md
previous: 2026-09-15-dasherized-identifiers/review-3.md
---

# Review 4: Dasherized Identifiers

## Verdict

The fix is **not ready for production**. Review 3's sole unblocked finding is
functionally implemented: valid tagged and anchored string scalars retain exact
authored provenance, aliases project through a provably unique defining scalar,
and ambiguous aliases fall back to their own token rather than disappearing
from composition or DMLS diagnostics. The new library, spawned-CLI, and real
in-memory LSP-session tests verify the required failure behavior and ranges at
Level 1.

Review 3 contained no blocked findings, so none became newly unblocked. Human
review is not required; the remaining issue is a bounded implementation and
performance-test gap with no unresolved product or design decision.

## Unblocked Findings

### Resolved — Valid tagged expression scalars retain provenance and DMLS diagnostics

The shared scalar decoder now skips tags and anchors while keeping them outside
the projected range. It resolves an alias to its defining scalar only when the
definition is provable, checks every decoded value against parser-owned text,
and otherwise retains the alias token as a typed locus. Composition remains
fatal with no partial output, while DMLS reports warning-severity diagnostics
at exact UTF-16 ranges and withholds code actions when the projection is not
safe to edit.

Focused verification passed 12 library provenance tests, 3 spawned-CLI tests,
and 3 real in-memory LSP-session tests. The protocol fixture covers LF and CRLF,
multibyte text, both tag/anchor orders, unique and redefined anchors, exact
warning severity and source, diagnostic identity, and safe-fix suppression.

## Blocked Findings

Review 3 had no blocked findings.

## Findings

### Medium — DMLS alias provenance performs quadratic document rescans

`decode_alias_target` searches the complete source prefix with `match_indices`
each time it decodes an alias
(`lib/src/markdown/schemas/simplified/yaml_scalar.rs:256-278`). DMLS calls that
decoder from the per-entry loop in `expression_values`
(`dmls/src/providers/frontmatter.rs:924-957`), and one diagnostics publication
currently calls `expression_values` once while specializing schema problems and
again while producing expression diagnostics
(`dmls/src/diagnostics/frontmatter.rs:226-232,622`). A document containing many
Expression-typed aliases therefore performs repeated full-prefix scans on each
editor diagnostics cycle: worst-case quadratic work in document size before
the expression AST's otherwise-linear diagnostic walk begins.

This conflicts with the specification's qualitative linear DMLS performance
posture and is avoidable with data the YAML AST pass already owns.
`lower_mapping` tracks anchors in document order to populate `alias_target`;
extend that state to retain the defining scalar's source start/holder indent (or
an already-decoded source map) and carry it on the alias entry. Then
`expression_values` can project aliases directly without searching the whole
document. Compute the expression-value set once per diagnostics publication and
share it between schema specialization and expression diagnostics. Add a
deterministic work-count regression with many distinct aliases so reinstating a
prefix scan fails without relying on wall-clock timing.

## Requirement Verification Levels

All user-observable behavior in this specification is deterministic parsing,
composition, CLI process behavior, or LSP protocol behavior. Level 1 is the
appropriate tier. No requirement depends on terminal-emulator rendering,
terminal input encoding, OS keyboard events, paste/IME, mouse input, or
scrolling, so Level 2 and Level 3 are not applicable.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Dasherized identifiers preserve subtraction boundaries | Level 1 lexer/parser tables, library composition, spawned CLI, and LSP navigation | Correct level and behavior. |
| 2. Invalid full-document expressions fail with typed provenance and no partial output | Level 1 library and spawned CLI tests, including tagged, anchored, aliased, CRLF, and ambiguous-anchor forms | Correct level and behavior. |
| 3. DMLS checks every identifier in every operand position | Level 1 provider tests and real in-memory LSP sessions, including exact UTF-16 tag/anchor/alias ranges | Correct level and behavior; the eager alias source lookup has the performance finding above. |
| 4. Unknown roots warn only when absence is unhandled | Level 1 library and CLI tests | Correct level and behavior. |
| 5. Each issue is reported once | Level 1 library, CLI, and LSP diagnostic-identity tests | Correct level and behavior. |
| Permanent shipped-artifact compatibility gate | Level 1 passive corpus and end-to-end shipped-prompt composition | Correct level and behavior. |
| Qualitative performance bounds | Level 1 deterministic identity-work regressions plus source inspection | Runtime warning identity is linear, but DMLS alias provenance adds quadratic prefix rescans. **Medium gap.** |

## Verification Performed

- Reconciled Review 3: its one unblocked functional finding is implemented; it
  had no blocked findings.
- Focused tag/anchor/alias verification: 18 passed, 0 failed across library,
  spawned-CLI, and real in-memory LSP-session tests.
- `just test` reached 2,328 passes before 12 unrelated load-sensitive HTTP
  tests hit the 30-second nextest timeout and canceled the remaining tests. All
  12 passed when rerun at `--test-threads 2`; the implementation log records
  the same known full-load behavior and a prior constrained green rerun.
- `just lint` passed for `darkmatter`, `darkmatter-cli`, `dmls`, and
  `zed-dmls-cli`; the `wasm32-wasip2` extension check also passed.
- `git diff --check` passed for the implementation files reviewed.
- GitNexus reported 9 upstream consumers and LOW risk for `expression_values`.
  The newly added `decode_scalar_node` was absent from the current index and
  therefore remained `UNKNOWN`; text search established its production call
  sites in schema source mapping, composition provenance, and DMLS expression
  extraction. No HIGH or CRITICAL graph risk was reported.
