---
$schema: feature-review.yaml
ready: false
findings:
  - "Medium — Shared alias targets are still cloned and decoded per alias"
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T19:36:04-07:00"
spec: 2026-09-15-dasherized-identifiers/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
description: "A **fix** review of `2026-09-15-dasherized-identifiers/spec.md`"
fix: 2026-09-15-dasherized-identifiers/review-5.md
previous: 2026-09-15-dasherized-identifiers/review-4.md
next: 2026-09-15-dasherized-identifiers/review-6.md
---

# Review 5: Dasherized Identifiers

## Verdict

The fix is **not ready for production**. Review 4's finding was implemented in
its stated form: aliases now carry a parser-derived defining-scalar offset,
diagnostic publication computes the Expression-typed value set once, and the
new deterministic work-count regression proves that distinct aliases perform
no prefix searches. Review 4 contained no blocked findings, so none became
newly unblocked.

One bounded performance gap remains. Multiple aliases of the same large scalar
still clone that scalar into every overlay entry and decode it again for every
alias before diagnostic deduplication. That preserves a quadratic worst case
and is not covered by the distinct-small-anchor regression. Human review is
not required; the remaining work is an implementation and regression-test fix
with no unresolved product decision.

## Unblocked Findings

### Resolved — DMLS alias provenance no longer rescans the document prefix

`FmEntry::alias_target_start` records the defining scalar's parser-derived
offset, `expression_values` uses `decode_alias_definition` at that offset, and
the diagnostics entry point shares one computed expression-value set between
schema specialization and expression diagnostics. The regression with 150 and
300 distinct aliases asserts zero alias-search work and exactly one value-set
computation per publication. The full DMLS Level-1 suite passed 689 of 689
tests, including this regression and the in-memory LSP alias-provenance tests.

## Blocked Findings

Review 4 had no blocked findings.

## Findings

### Medium — Shared alias targets are still cloned and decoded per alias

The overlay stores an owned `String` on every alias entry
(`dmls/src/overlay/frontmatter.rs:61-70`) and constructs it by cloning the
anchor definition for each alias (`dmls/src/overlay/frontmatter.rs:419-426`).
Then `expression_values` invokes `decode_alias_definition` independently for
every entry that carries the same definition offset
(`dmls/src/providers/frontmatter.rs:934-959`). Diagnostic deduplication happens
only after that vector has been fully built.

For one scalar of length `L` referenced by `N` Expression-typed aliases, the
overlay therefore owns `N * L` copied bytes and scalar projection performs
`N * L` decoding work. A document where both `L` and `N` grow together remains
quadratic in document size, contrary to the specification's linear DMLS
performance posture. The new test uses `N` distinct short scalars
(`dmls/src/diagnostics/frontmatter.rs:1762-1802`), so its zero prefix-search
counter stays green while this repeated-target case regresses.

Retain each anchor target once in the parsed overlay and let alias entries
refer to that target by stable identity or shared ownership. During one
`expression_values` computation, decode each unique defining offset once and
share the resulting source map across aliases. Extend the work counter with a
fixture containing one increasingly long anchored expression and many aliases;
assert one target decode and one stored target rather than relying on elapsed
time.

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
| 3. DMLS checks every identifier in every operand position | Level 1 provider tests and real in-memory LSP sessions with exact UTF-16 ranges | Correct level and behavior. |
| 4. Unknown roots warn only when absence is unhandled | Level 1 library and CLI tests | Correct level and behavior. |
| 5. Each issue is reported once | Level 1 library, CLI, and LSP diagnostic-identity tests | Correct level and behavior. |
| Permanent shipped-artifact compatibility gate | Level 1 passive corpus and end-to-end shipped-prompt composition | Correct level and behavior. |
| Qualitative performance bounds | Level 1 deterministic work counters plus source inspection | Prefix rescans and duplicate extraction are fixed, but shared targets are still copied and decoded per alias. **Medium gap.** |

## Verification Performed

- Reconciled Review 4: its sole unblocked finding is implemented as specified;
  it had no blocked findings.
- GitNexus reported 9 upstream consumers and LOW risk for
  `expression_values`, spanning the diagnostics and provider modules.
- `cargo nextest run -p dmls --features effects-instrumentation` passed 689 of
  689 Level-1 tests with no skips.
- A selected library run for YAML-scalar, fatal-expression, and shipped-corpus
  contracts could not start because Cargo exhausted the host volume while
  linking the unrelated `render_tree_parity` example. This is an environment
  capacity failure, not evidence against the implementation; the same focused
  contracts are recorded green in the implementation log.
- `git diff --check` passed for the reviewed implementation files.
