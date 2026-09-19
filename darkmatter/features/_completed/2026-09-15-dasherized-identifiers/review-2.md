---
$schema: feature-review.yaml
ready: false
findings:
  - "High — Block-scalar frontmatter expressions lose required provenance and DMLS analysis"
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-18T11:08:06-07:00"
spec: 2026-09-15-dasherized-identifiers/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
next: 2026-09-15-dasherized-identifiers/review-3.md
description: "A **fix** review of `2026-09-15-dasherized-identifiers/spec.md`"
fix: 2026-09-15-dasherized-identifiers/review-2.md
previous: 2026-09-15-dasherized-identifiers/review-1.md
---

# Review 2: Dasherized Identifiers

## Verdict

The fix is **not ready for production**. All three unblocked findings from
Review 1 were implemented: interpolation errors now retain exact authored
ranges through body rewrites and ordinary YAML quoting/escaping, unknown-root
and warning deduplication now use hash-backed indexes with deterministic
complexity regressions, and the permanent corpus gate discovers arbitrary
Expression-typed properties through a shared passive schema classifier.

Review 1 contained no blocked findings, so there was nothing to re-evaluate as
newly unblocked. Human review is not required; the remaining finding has a
direct Level-1 implementation and verification path.

## Findings

### High — Block-scalar frontmatter expressions lose required provenance and DMLS analysis

Requirement 2 requires every fatal mixed-frontmatter expression error to carry
its original source path, line, and expression span
(`spec.md:202-244`). YAML literal (`|`) and folded (`>`) block scalars are
ordinary string values and are processed by frontmatter interpolation, but the
new source projector explicitly returns no range for a block or multiline
scalar (`lib/src/markdown/compose/frontmatter_interpolation.rs:386-394`). The
failure is therefore only upgraded to `SourceRef::OnDisk`, not
`SourceRef::OnDiskSpan`; composition fails, but the typed error and rendered
diagnostic omit the required expression-specific line, column, range, and
highlight. The renderer can still show a key-level YAML excerpt, but it cannot
identify the failing construct inside that value.
The new failure-contract tests cover plain, double-quoted, escaped, and nested
values, but no block scalar
(`lib/tests/compose_expression_failure_contract.rs:164-201`).

The same representation gap breaks Requirement 3 for Expression-typed
frontmatter values. DMLS obtains the parser's decoded scalar in
`FmEntry::scalar`, but `expression_values` ignores it and reparses the raw
`value_span` through `decode_scalar`
(`dmls/src/providers/frontmatter.rs:883-910`). That decoder models plain,
single-quoted, and double-quoted scalars only
(`lib/src/markdown/schemas/simplified/yaml_scalar.rs:66-89`); it does not apply
YAML block chomping/folding or multiline scalar rules. DMLS consequently does
not parse the same expression text that composition evaluates and cannot emit
the required per-identifier warning ranges inside such values. The LSP
operand-position regression uses only one-line scalar values.

Extend the source-aware scalar representation to decode and project literal,
folded, and multiline YAML strings according to the same value semantics used
by frontmatter parsing. Both compose error anchoring and DMLS should consume
that single decoded-text-to-authored-byte map; DMLS should not independently
reconstruct the value from raw text. Add Level-1 tests for:

- parse and evaluation failures inside `|` and `>` mixed-frontmatter values,
  asserting the exact `OnDiskSpan` range, line, and character column under both
  `fail_fast` settings;
- folding, chomping indicators, indentation, CRLF, escapes where applicable,
  and multibyte text before the expression;
- a real LSP session with an Expression-typed block scalar containing unknown
  identifiers in multiple operand positions, asserting warning severity and
  exact identifier ranges.

## Requirement Verification Levels

All user-observable behavior in this specification is deterministic parsing,
composition, diagnostics, CLI process behavior, or LSP protocol behavior.
Level 1 is the appropriate verification tier. Nothing in the feature depends
on terminal-emulator rendering, terminal input encoding, OS keyboard events,
paste/IME, mouse input, or scrolling, so Level 2 and Level 3 are not applicable.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Dasherized identifiers preserve subtraction boundaries | Level 1 lexer/parser tables, library compose, spawned CLI, and LSP navigation | Correct level and behavior. |
| 2. Invalid full-document expressions fail with typed provenance and no partial output | Level 1 library and spawned CLI tests | Fatality and ordinary-scalar provenance pass; block and multiline frontmatter scalars lack the required span. **High gap.** |
| 3. DMLS checks every identifier in every operand position | Level 1 provider tests and real in-memory LSP sessions | Correct for ordinary scalar and body expressions; block/multiline Expression-typed values are not decoded and projected with compose-equivalent semantics. **High gap.** |
| 4. Unknown roots warn only when absence is unhandled | Level 1 library and CLI tests | Correct level and behavior. |
| 5. Each issue is reported once | Level 1 library and CLI identity tests | Correct level and behavior. |
| Permanent shipped-artifact compatibility gate | Level 1 passive corpus and end-to-end shipped-prompt compose | The Review 1 gap is closed: arbitrary schema-typed properties are discovered and a malformed fixture fails the gate. |
| Qualitative performance bounds | Level 1 deterministic identity-work regressions plus source inspection | The Review 1 quadratic paths are now hash-backed and preserve first-read ordering. |

## Verification Performed

- Focused library/corpus tests: 31 passed, 0 failed.
- CLI expression-failure tests: 4 passed, 0 failed.
- A file-backed `md compose` reproduction with a failing interpolation inside
  a `|` scalar exited 1 with empty stdout, but rendered only a key-level YAML
  excerpt and no `Expression at line: ..., column: ...` locus.
- DMLS real in-memory LSP session tests: 104 passed, 0 failed.
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, `dmls`,
  `zed-dmls-cli`, and the `zed-dmls` `wasm32-wasip2` compile check.
- `just test`: 2,346 tests passed before three unrelated remote-network/cache
  tests hit the shared 30-second timeout and fail-fast canceled the remainder.
  All three timed-out tests passed when rerun serially (8.1s, 10.6s, and
  12.1s). No failure occurred in a dasherized-identifier change path.
- GitNexus reports LOW upstream impact for `anchor_authored_failure` and
  CRITICAL impact for `add_unknown_root_candidates` (26 symbols across four
  execution flows). The new schema extractor was unresolved in the current
  index (`UNKNOWN`), so its consumers and behavior were confirmed by source
  search and focused tests.
