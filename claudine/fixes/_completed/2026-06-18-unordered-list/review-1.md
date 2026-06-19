---
ready: true
agent: codex
model: ""
created: 2026-06-19T00:14:55
---

# Review 1 — Tight nested lists through cleanup

## Scope Note

The prompt referenced `@claudine/spec.md`, which does not exist in this
worktree. I treated `claudine/fixes/2026-06-18-unordered-list/spec.md` as the
intended specification because it matches the requested review output path and
the active staged implementation.

## Findings

No blocking findings.

The implementation matches the focused compatibility fix in the spec:

- `darkmatter/lib/src/markdown/cleanup.rs` changes the `Normal` spacing
  predicate to `indent < prev || had_continuation`, so parent-to-child descents
  and same-level siblings stay tight while shallower returns and loose items
  still get separators.
- The adjacent `normalize_list_spacing` documentation now describes shallower
  returns, loose items, and prose-after-list behavior instead of generic
  indentation transitions.
- Darkmatter regression coverage now includes negative assertions for the
  parent-to-child blank-line bug, the closure-shaped incident payload,
  same-level sibling preservation, shallower-return separation, and loose-list
  separation.
- Claudine adds a test-only regression in `composition::prepare` proving direct
  composition preserves the tight nested-list shape that surfaced the incident.

## Verification Level Review

All user-observable requirements here are byte-level Markdown cleanup /
composition output requirements. They do not depend on terminal emulator input
encoding, SGR rendering, glyph width, scrolling, mouse, paste, IME, or OS
keyboard behavior, so Level 1 verification is appropriate.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| `Normal` inserts blanks only for `had_continuation` or `indent < prev` | Level 1 unit tests plus direct CLI behavioral check | Appropriate |
| Tight parent-to-child nested lists stay tight | Level 1 Darkmatter unit tests, Claudine prepare regression, direct `md clean` check | Appropriate |
| Same-level siblings remain tight | Level 1 unit tests | Appropriate |
| Shallower returns still insert a blank | Level 1 unit tests | Appropriate |
| Loose lists still keep separating blanks | Level 1 unit tests | Appropriate |
| Claudine direct composition preserves the incident shape | Level 1 in-process prepare test | Appropriate |

No Level 2 or Level 3 coverage is required for this fix because the observable
contract is the emitted Markdown text, not real-terminal rendering or terminal
keyboard encoding.

## Verification Run

- `cargo nextest run -p darkmatter --lib markdown::cleanup` — 100 passed.
- `cargo nextest run -p claudine --lib composition::prepare` — 42 passed.
- `cargo run -q -p darkmatter-cli -- clean /tmp/tight.md` on
  `- Level 1\n    - Level 2\n        - Level 3\n` emitted the tight nested list
  with no blank lines between levels.

## Residual Risk

I did not run the full workspace suite or `just lint`; the reviewed change is
narrow and the targeted Darkmatter and Claudine suites cover the specified
behavior. The staged worktree also contains unrelated Claudine frontmatter error
rendering changes, which I did not evaluate as part of this review.
