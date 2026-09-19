---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T20:47:49-07:00"
spec: 2026-09-15-dasherized-identifiers/spec.md
implemented: false
description: "A **fix** review of `2026-09-15-dasherized-identifiers/spec.md`"
fix: 2026-09-15-dasherized-identifiers/review-6.md
previous: 2026-09-15-dasherized-identifiers/review-5.md
---

# Review 6: Dasherized Identifiers

## Verdict

The fix is **ready for production**. Review 5's sole unblocked finding is
implemented: aliases of one scalar now share one stored target, each defining
scalar is decoded once per expression-value computation, and each authored
expression is parsed once per diagnostic publication. The malformed-expression
cache preserves an error accepted by a mixed union until an alias under a
rejecting Expression property can report it, so the optimization does not
change diagnostic disposition.

Review 5 contained no blocked findings, so none became newly unblocked. No
human review is required; the implementation follows the specification's
ratified behavior and leaves no product or design decision unresolved.

## Unblocked Findings

### Resolved — Shared alias targets are stored, decoded, and parsed once

`FmEntry::alias_target` and `AnchorDefinition::text` use shared `Arc<str>`
ownership, so every alias of one definition retains only a constant-size clone.
`expression_values` caches `Arc<DecodedScalar>` by the parser-derived defining
offset for the duration of one computation. `expression_diagnostics` then
settles each authored expression span after one parse; a malformed parse that a
mixed union accepts remains cached until a rejecting alias reports it.

The deterministic regressions exercise 200 aliases at two expression lengths.
They assert pointer-identical target storage, one target decode, one expression
parse, one diagnostic for a validly parsed expression, and correct delayed
reporting for a malformed expression. The non-instrumented behavior remains
covered through the complete DMLS Level-1 suite and in-memory LSP sessions.

## Blocked Findings

Review 5 had no blocked findings.

## Findings

None.

## Requirement Verification Levels

All user-observable behavior in this specification is deterministic parsing,
composition, CLI process behavior, or LSP protocol behavior. Level 1 is the
appropriate tier. No requirement depends on terminal rendering, terminal input
encoding, OS keyboard events, paste/IME, mouse input, or scrolling, so Levels 2
and 3 are not applicable.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Dasherized identifiers preserve subtraction boundaries | Level 1 lexer/parser tables, library composition, spawned CLI, and LSP navigation | Correct level and behavior. |
| 2. Invalid full-document expressions fail with typed provenance and no partial output | Level 1 library and spawned-CLI tests, including tagged, anchored, aliased, CRLF, and ambiguous-anchor forms | Correct level and behavior. |
| 3. DMLS checks every identifier in every operand position | Level 1 provider tests and in-memory LSP sessions with exact UTF-16 ranges | Correct level and behavior. |
| 4. Unknown roots warn only when absence is unhandled | Level 1 library and CLI tests | Correct level and behavior. |
| 5. Each issue is reported once | Level 1 library, CLI, and LSP diagnostic-identity tests | Correct level and behavior. |
| Permanent shipped-artifact compatibility gate | Level 1 passive corpus and end-to-end shipped-prompt composition | Correct level and behavior. |
| Qualitative performance bounds | Level 1 deterministic work counters, pointer-identity assertions, and source inspection | Correct level: shared targets incur one allocation, decode, and parse per authored expression during publication. |

## Verification Performed

- Reconciled Review 5: its sole unblocked finding is implemented; it had no
  blocked findings.
- GitNexus reported LOW upstream risk for `FmEntry` (4 impacted symbols) and
  `expression_values` (9 impacted symbols). The diagnostics function has 23
  upstream impacts at LOW risk.
- `cargo nextest run -p dmls --features effects-instrumentation` passed 691 of
  691 Level-1 tests with no skips.
- Focused library runs covering the shipped corpus, composition, fatal
  expression failures, and expression regressions passed 93 of 93 tests.
- Spawned-CLI dasherized-identifier and expression-failure tests passed 11 of
  11 tests.
- `md schema validate` accepted this review against `feature-review.yaml` with
  no problems.
- `just lint` passed for the Darkmatter library, CLI, DMLS, Zed CLI, and the
  `wasm32-wasip2` Zed extension check.
- `git diff --check` passed for the reviewed implementation files.
