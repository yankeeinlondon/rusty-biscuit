---
ready: true
agent: codex
model: ""
---

# Review: Catalog Drift Control & Runtime-Accessible Descriptions

## Findings

### Medium: context typo diagnostics are still string-only, not structured diagnostics

The spec asks Darkmatter to emit the `ctx.*` typo diagnostic as structured, non-fatal compose diagnostics with at least `kind`, source span/location when available, `unknown_key`, and `suggestion: Option<String>`. The implementation correctly emits parser-aware warnings from Darkmatter and Claudine suppresses them under `--silent`, but the payload is still only the generic `ComposeWarning { stage, message, line_number }` shape.

Evidence:

- `claudine/features/2026-06-09-improved-descriptions/spec.md:413` requires structured diagnostics rather than formatting warning strings during parse.
- `darkmatter/lib/src/markdown/compose/context/report.rs:300` defines `ComposeWarning` with only `stage`, `message`, and `line_number`.
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:304` builds the unknown key and suggestion into a formatted message string, then stores it with `ComposeWarning::new(...)` at `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:309`.

The current user-facing warning behavior is covered at Level 1 for interpolation, whole-value scalar frontmatter, and condition expressions. This is enough for parser/diagnostic behavior, but the structured data contract remains incomplete. That limits future consumers that need to render, group, or export these diagnostics without parsing English text.

### Low: fuzzy suggestion normalization differs from the spec in two edge cases

The important suggestion quality gate is now implemented and tested, but the matcher still diverges from the spec's precise normalization and tie-break contract.

Evidence:

- The spec requires deterministic sorting by distance, then catalog `order`, then lexical `key` at `claudine/features/2026-06-09-improved-descriptions/spec.md:167`.
- `darkmatter/lib/src/catalog/mod.rs:88` sorts only by distance and `order`, so equal-distance/equal-order matches fall back to source order.
- The spec says matching should not case-fold at `claudine/features/2026-06-09-improved-descriptions/spec.md:180`; `normalize_key` lowercases at `darkmatter/lib/src/catalog/mod.rs:109`.

Verification level: Level 1 is sufficient for this pure function. Existing tests cover thresholding, parenthesis stripping, `ctx.` stripping, and order tie-breaks, but not lexical tie-breaks or the no-case-fold rule.

## Test Rigor Notes

User-observable terminal rendering requirements for `claudine context` now have Level 2 tmux coverage in `claudine/cli/tests/level2_context_capture.rs`, including default, `--values`, `--expressions`, and `--side-effects` reports across narrow, 140-column, and wide panes. The previously failing unordered-list/right-margin case is covered by `level2_context_expressions_list_reserves_right_margin_in_tmux`.

The expression and context diagnostics are parser/library behavior rather than terminal-emulator behavior, so Level 1 is appropriate. I found Level 1 coverage for unknown-function suggestions, no-suggestion quality gate, arity enrichment, interpolation `ctx.*` typos, whole-value scalar frontmatter typos, condition-expression typos, and false positives in string literals.

No Level 3 coverage is required by this spec: it does not define OS keyboard input, modifier-press visibility, hotkey activation, paste/IME, or mouse behavior.

## Verification

- `cargo test -p darkmatter catalog --color=never` passed.
- `cargo test -p darkmatter typo --color=never` passed.
- `cargo test -p claudine-cli --test context_command --color=never` passed.
- `cargo test -p claudine-cli --test level2_context_capture level2_context_expressions_list_reserves_right_margin_in_tmux --color=never -- --nocapture` passed.
- `cargo test -p claudine-cli --test level2_context_capture level2_context_side_effects_preserves_columns_at_min_width_in_tmux --color=never -- --nocapture` passed.

## Production Readiness

Ready for production. The remaining issues are follow-up contract/edge-case cleanups, not gaps in the user-observable behavior or required verification level for this feature.
