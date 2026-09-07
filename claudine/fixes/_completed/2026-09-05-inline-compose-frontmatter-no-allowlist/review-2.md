---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-09-07T14:42:56-07:00
spec: 2026-09-05-inline-compose-frontmatter-no-allowlist/spec.md
implemented: false
description: A **fix** review of `2026-09-05-inline-compose-frontmatter-no-allowlist/spec.md`
fix: 2026-09-05-inline-compose-frontmatter-no-allowlist/review-2.md
previous: 2026-09-05-inline-compose-frontmatter-no-allowlist/review-1.md
---

# Review 2: Inline Compose Frontmatter Without an Allowlist

## Verdict

The fix is **ready for production** under the active inline-compose contract.
No blocking or non-blocking implementation finding remains from Review 1.

Review 1 correctly found that the historical response-block parser sorted new
properties instead of preserving response order. The implementation cycle also
correctly declined to rebuild that parser: the later
`2026-09-05-inline-flow-and-validations` fix formally superseded this fix's
response-block channel and restored the intended file-aware workflow. The
provider now authors requested frontmatter directly in the document, so source
order is retained textually and there is no parsed response-property status
sequence that can be sorted.

The enduring requirement from this fix is the absence of a frontmatter
allowlist. It remains fully implemented: no `response_frontmatter` plan field,
validation, typed error, prompt appendix, response parser, or closure allowlist
exists in active Rust code. The agent may author any top-level property except
the three closure-owned properties (`prompt`, `hash`, and `last_updated`), which
the closure restores or stamps.

## Findings

None.

## Review 1 Resolution

The response-order defect is **superseded, not reproduced**. The active closure
reads the provider's on-disk document and delegates owned-property restoration
to Darkmatter's textual `restore_properties_text` path. Exact-text LF and CRLF
tests prove non-owned property bytes and order survive restoration, while
semantic-delta tests prove additions and replacements are reported in current
document order. The two-run CLI regression proves properties are refreshed in
place without duplication.

Reintroducing an ordered response parser solely to satisfy the retired AC1
mechanism would contradict the active file-aware specification. The intended
outcome—arbitrary prompt-requested frontmatter with stable order and no
allowlist—is achieved more directly by the current design.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: arbitrary existing and new properties survive in authored order; closure-owned properties do not | Level 1 exact-text closure/Darkmatter tests and Level 1 CLI subprocess tests | Appropriate and present under the superseding file-aware channel. The retired response-block mechanism is no longer a production requirement. |
| AC2: a second run refreshes agent-authored properties without duplication | Level 1 CLI subprocess test across two provider-stub runs | Appropriate and present. |
| AC3: the 2026-09-01 shipped guardrails migrate atomically while custom content is preserved | Level 1 real-filesystem tests covering every historical default, one-byte customization, and injected atomic-write failure | Appropriate and present. The migrated destination is the newer file-aware protocol. |
| AC4: no declaration is required; requested properties persist across two runs, authored `prompt` bytes survive, and current guardrails reach the provider | Level 1 Unix CLI subprocess tests plus platform-independent closure tests | Appropriate and present. Windows path construction is separately covered in-process; no terminal behavior is involved. |
| AC5: malformed or ambiguous frontmatter and unchanged/empty bodies fail without an invalid stamp; provider failures roll back | Level 1 parser, closure, and CLI lifecycle tests using real temporary files | Appropriate and present under the active closure contract. Obsolete response-only failure shapes retired with the response channel. |

Levels 2 and 3 are not applicable. These requirements concern prompt bytes,
YAML/document text, filesystem mutation, and subprocess output. They make no
claim about terminal-emulator rendering, glyph width or styling, scrolling, or
physical keyboard encoding.

## Verification Performed

- Focused Claudine Level 1 nextest run: **82 passed, 6,739 skipped, 0 failed**.
  The filter covered closure, guardrails, `wrap_inline_compose`, and
  `inline_completion_lifecycle` tests.
- Static removal audit: active Rust code contains none of
  `response_frontmatter`, `ResponseFrontmatterInvalid`,
  `extract_replacement_parts`, or `rewrite_harvested_frontmatter`.
- GitNexus context inspection confirmed that the active reconciliation path
  calls Darkmatter's textual restore and hash-write primitives; it does not
  parse provider response frontmatter.

The macOS linker emitted its existing compact-unwind size warning during the
test build; it did not affect compilation or execution.

## Production Readiness

The no-allowlist behavior is production ready. Review 1's defect belonged only
to a deliberately retired transport, while the active transport preserves the
same user outcome with stronger textual ordering guarantees and verification at
the appropriate Level 1 tier.
