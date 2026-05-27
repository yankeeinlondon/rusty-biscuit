---
ready: false
agent: open_code
model: ""
---

# Review: Comment Quality — Iteration 4

## Summary

The prior review cycles addressed all three major findings: rendering behavior changes were reverted, the checker baseline is now clean, and fixture tests cover all four heuristic categories. The rubric docs, checker script, and test harness are well-structured. Two items remain before this feature meets its own acceptance criteria.

## Findings

### High: `stream/reporting.rs` still contains the canonical HOW-narration anti-pattern

The docblock on `summary_to_event_meta` at `claudine/lib/src/stream/reporting.rs:13-20` reads:

```rust
/// Convert a `StreamExecutionSummary` into an `EventMeta` suitable for JSONL logging.
///
/// The resulting `EventMeta` has:
/// - `event = SessionEnd`
/// - `extra.synthetic = true`
/// - `extra.synthetic_kind = "stream_wrapper_summary"`
/// - `extra.stream_protocol` set to the protocol name
/// - Token usage, cost, duration, and other fields mapped into `extra`
```

This is **the literal "Before" example** in `docs/comment-quality.md` anti-pattern 1. The bullet list restates what the function body does field-by-field. The comment-quality cleanup removed the section-marker `//` comments from this file (anti-pattern 6) but left this HOW-narration block intact.

Similarly, `semantic_event_to_event_meta` at lines 162-182 has a 20-line docblock that narrates the function's field-by-field mapping. Some bullets carry Criterion B rationale ("so the row doesn't collide with the canonical `SessionEnd` summary row"), which should be condensed and kept; the rest is HOW-narration.

The spec acceptance criterion says: "All `.rs` files under `claudine/lib/src/` and `claudine/cli/src/` have been reviewed against the rubric and updated where applicable." Leaving the exact example cited in the rubric doc un-cleaned is a gap.

**Severity: High.** The file is in the required cleanup scope, the anti-pattern is explicitly named in the rubric, and the docs use this file as the canonical bad example.

### Medium: 13 unique broken intra-doc link warnings in `cargo doc -p claudine`

The spec acceptance criterion requires: "`cargo doc -p claudine` and `cargo doc -p claudine-cli` produce no warnings about broken intra-doc links."

`cargo doc -p claudine --no-deps` currently emits 13 unique unresolved-link warnings. A representative sample:

- `warning: unresolved link to \`crate::protect::protect\``
- `warning: unresolved link to \`EventMeta\``
- `warning: unresolved link to \`HookDecision::Allow\``
- `warning: unresolved link to \`ClaudineConfig\``
- `warning: unresolved link to \`SemanticEvent::ToolResult\``

These may be pre-existing (not introduced by this feature). However, the acceptance criterion is unconditional — it says "produce no warnings about broken intra-doc links." If these are pre-existing, the criterion should be narrowed in the spec or the links should be fixed as part of this pass.

**Severity: Medium.** The spec's acceptance criterion is explicit, but this is likely a pre-existing condition rather than a regression from the comment-quality work.

### Low: `docs/comment-quality.md` anti-pattern 7 uses a sketch, not a real codebase example

The acceptance criteria require "one expanded before/after pair per anti-pattern [...] each citing a real file path in the codebase." Anti-pattern 7 (heavy-setup doc examples) uses a sketch marked "(sketch)" in the "Before" and references a hypothetical `claudine/lib/tests/merge_tests.rs` in the "After" that may not exist as a real path. All other anti-patterns and positive criteria cite real file paths.

**Severity: Low.** The anti-pattern is well-explained and the sketch is clearly labeled. This is a minor spec-compliance gap.

## Coverage Assessment

| User-facing requirement | Verification level | Assessment |
|---|---|---|
| `just check-comments` produces parseable findings for the 4 specified patterns | Level 1 (11 fixture tests in `check-comments-tests.sh`) | Adequate |
| `just check-comments` exits 0 even with findings (warn-only) | Level 1 (explicit test case) | Adequate |
| Checker handles single-line, multi-line signature, and multi-line body function shapes | Level 1 (separate fixture per shape) | Adequate |
| `cargo test -p claudine` and `cargo test -p claudine-cli` pass | Level 1 | Verified (2302 + 978 tests passing) |
| All `.rs` files under `claudine/lib/src/` and `claudine/cli/src/` reviewed against rubric | Manual review | Gap: `stream/reporting.rs` still has HOW-narration |
| `cargo doc` produces no broken intra-doc link warnings | Build verification | Gap: 13 pre-existing warnings |

No Level 2 or Level 3 verification is required for this feature — its user-facing behavior is a warn-only heuristic script, not terminal UX.

## Verification

- `just check-comments claudine/lib/src claudine/cli/src` — exits 0, no findings.
- `./scripts/check-comments-tests.sh` — all 11 Level 1 fixture cases pass.
- `cargo test -p claudine --lib --color=never` — 2302 passed, 0 failed.
- `cargo test -p claudine-cli --bins --color=never` — 978 passed, 0 failed.
- `cargo doc -p claudine --no-deps --color=never` — 13 unique unresolved-link warnings (pre-existing).
- `AGENTS.md` Comment Quality section — 29 lines (within ≤ 30 target).
- All 5 explicitly named modules (`composition`, `mcp`, `protect`, `system_prompt`, `stream`) have `//!` links to their topic docs.

## Verdict

Not ready for production. The rubric, checker, tests, and documentation are materially aligned with the spec. Two items remain:

1. The HOW-narration on `summary_to_event_meta` and `semantic_event_to_event_meta` in `stream/reporting.rs` — the exact anti-pattern the feature exists to remove — must be cleaned up.
2. The 13 broken intra-doc link warnings need to be resolved or the acceptance criterion needs to be scoped to "no new warnings introduced by this feature."
