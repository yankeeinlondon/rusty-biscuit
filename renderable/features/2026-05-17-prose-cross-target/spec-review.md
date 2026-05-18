# Spec Review: Prose Cross-Target Rendering

**Reviewer:** Claude
**Date:** 2026-05-17
**Reviewed file:** `spec.md` (Draft)
**Verified against:** `renderable/src/markdown.rs`, `biscuit-terminal/lib/src/components/prose/` (`mod.rs`, `markdown.rs`, `styles.rs`, `tokens.rs`, `render.rs`, `prose.rs`)

## Summary

The spec is well-scoped and the core decision — keep `Prose` off the render
tree and give it its own target-neutral IR — is sound and well-justified. The
problem statement, non-goals, and parity-via-terminal-oracle strategy are
strong.

The main weakness is that the **IR sketch cannot represent the one feature the
spec explicitly says it must** (atomic reset semantics), and several
target-emitter contracts (text decoding/escaping, link href storage, the
`Literal` node) are left implicit. These should be nailed down before
implementation, because they determine the parser's output shape.

## Post-Review Decision — Atomic Syntax Removal

After this review, a follow-up call-site analysis was run and a decision was
taken: **drop the atomic-token grammar (`{{token}}`) from `Prose`**, keeping
only bracketed tags (`<bold>…</bold>`) and the Markdown subset.

Rationale and consequences:

- Atomic tokens are the **sole source of C1**. They are unscoped and can
  produce overlapping (non-nestable) style ranges. Bracketed tags and the
  Markdown subset are well-nested by construction.
- With atomics removed, the IR collapses to a **pure tree** — no flat
  `StyleOp` / `Reset` variants, no flat→nested reconstruction. **C1 is
  resolved**, not solved (see updated C1 below).
- The Markdown subset already lowers to bracketed tags in the pre-processor,
  so it costs nothing in IR complexity and is retained.
- Call-site analysis: ~218 atomic-token matches across 12 non-test files,
  concentrated in Claudine hook/action UI code. This is a **breaking grammar
  change** and must be sequenced as a prerequisite migration phase — see the
  updated `spec.md` (Goal, Non-Goals, and the new migration phase).

This decision has been folded into `spec.md`. The remaining critical issues
below (C2, C3) are independent of the atomic/bracketed question and still
stand.

## Strengths

- Clear separation of "what" (cross-target IR) from "what not" (no render-tree
  re-pointing, no grammar changes).
- Terminal renderer as behavioral oracle is the right call, and the explicit
  "existing tests pass unchanged" requirement is good discipline.
- Non-Goals section is unusually thorough and prevents scope creep.
- The per-target mapping tables (Markdown / MarkdownPlus) are concrete enough
  to test against.

## Critical Issues

### C1 — The IR cannot express atomic reset semantics — RESOLVED by dropping atomic syntax

**Original finding.** The spec said atomic reset tokens "should remain explicit
operations in the IR rather than being approximated" — but the proposed
`ProseNode` enum had **no variant for a bare style operation**. Every styling
path in the sketch is a scoped `Span { style, children }`.

Atomic tokens (`{{bold}}…{{reset}}`) are a *flat sequence of style mutations*
with no inherent nesting and can even escape an enclosing block tag's scope
(`<b>x {{red}}y</b> z` leaves `z` red but not bold). Browser and Markdown need
strict open/close nesting, so an emitter would have had to reconstruct a tree
from overlapping flat ranges — the highest-risk part of the feature, with no
design.

**Resolution.** The atomic grammar is being removed (see Post-Review Decision
above). With only bracketed tags and the Markdown subset, every styled region
is well-nested by construction. The IR becomes a pure tree (`Span` only, no
flat ops), and the flat→nested reconstruction problem **ceases to exist**.

No reconstruction algorithm is needed. The terminal emitter keeps its existing
layer-restoration logic for *nested* bracketed tags; it no longer has to cope
with atomics outliving a parent scope.

### C2 — `Text` vs `Literal` distinction is undefined

The IR has both `Text(String)` and `Literal(String)`. The spec never says how
they differ. "Unknown tags/tokens preserved as literal text" (lines 63, 124,
241) implies `Literal` = verbatim — but **verbatim is wrong for Browser**
(must still HTML-escape `<`, `&`) and for Markdown (must still escape sigils),
or FR-4/FR-5 are violated.

Recommendation: drop `Literal` and route unknown tags/tokens to `Text`. Every
target then escapes per its own rules. If `Literal` must stay, define it
precisely as "content already in the target's escaped form" and explain how a
target-neutral IR can hold target-specific text (it can't cleanly — another
argument for dropping it).

### C3 — Text node decoding/escaping contract is unstated

The current parser supports backslash escapes (`\_`, `\*`, …) and a flanking
rule. The spec must state explicitly:

- `ProseNode::Text` holds **fully decoded** content (escapes resolved, e.g.
  `\_` stored as `_`; flanking-suppressed sigils stored literally).
- Each emitter **re-escapes** decoded text for its own target (ANSI: none;
  Browser: HTML-escape; Markdown: escape `_ * [ ] ( ) ` # ~` etc.).

Parser test "Backslash escapes remain literal" (line 256) is ambiguous about
whether the backslash survives into the IR. Make it concrete: "after parsing,
`\_` yields `Text(\"_\")`".

Note: `prose/markdown.rs` already contains an `html_escape` helper — the
Browser emitter can reuse it rather than re-implementing.

## Recommendations by Section

### Proposed Design / Parser Boundary

- **R1** — Acknowledge the existing module layout. `prose/` is already a
  multi-file module: `markdown.rs` (the Markdown pre-processor:
  `convert_links`, `convert_bold`, `convert_italics`, flanking helpers),
  `tokens.rs`, `styles.rs`, `render.rs`. The spec says "refactor, not replace"
  but never names these files. State where the IR and parser will live
  (e.g. new `prose/ir.rs` + `prose/parse.rs`) and which existing functions are
  reused vs. moved.
- **R2** — `Sequence(Vec<ProseNode>)` looks redundant with the child vectors
  already on `ProseDocument` and `Span`. Unless it serves a distinct purpose,
  drop it. (The spec says names aren't settled — flagging structure, not
  names.)
- **R3** — State explicitly that `Link { href }` stores the **raw, unresolved**
  href. `styles.rs` resolves relative paths against package/git root
  (`find_package_root`, `find_git_relative_base`) for terminal OSC8. That
  resolution is a terminal-emitter concern (consistent with FR-8); Browser and
  Markdown should emit the author-supplied href, not a local filesystem path.

### Browser Target

- **R4** — `BrowserRenderable` requires `Debug + Any` and an `as_any` method
  (verified in `renderable::browser`). The spec's Browser section should note
  `Prose` must add `as_any`, and that output is a `BrowserFragment<Ready>`
  built via the fragment/`ComposableNode` API — not a raw string.
- **R5** — Resolve the block-vs-inline tension. Prose owns a `Layout`
  (margins, alignment, word-wrap) and handles **fenced code blocks**, which are
  block-level. The spec hedges ("`<span>` or `<div>` depending on layout
  semantics", line 156). Pick one: recommend a block wrapper (`<div>`), since
  code blocks cannot live inside a `<span>` and Layout maps to block CSS.

### Markdown / MarkdownPlus

- **R6** — Make explicit that `MarkdownRenderable` has **two required methods
  with no defaults**: `render_markdown` and `render_markdown_plus` (verified in
  `renderable/src/markdown.rs`). FR-2 should name both so the implementer
  knows MarkdownPlus is mandatory, not optional.
- **R7** — Resolve the strikethrough open question by aligning with the repo:
  the tree Markdown renderer (`renderable/src/tree/render/markdown.rs`) already
  handles a `Delete` node for both dialects. Have Prose emit whatever that
  renderer emits for `Delete` in plain Markdown, for consistency, instead of
  leaving it open.
- **R8** — `render_markdown`'s doc comment says output "may include YAML
  frontmatter". Note that `Prose` (inline content) must **never** emit
  frontmatter.

### Requirements

- **R9** — FR-9 and Open Question #4 conflict: FR-9 *requires* mapping
  "existing Prose layout fields that have clear CSS equivalents" while the open
  question asks how much layout to map "in the first pass". Pick a concrete
  first-pass scope (recommend: left/right margin only → CSS `margin`; defer
  alignment and word-wrap) and make FR-9 match it.

### Testing

- **R10** — The parity strategy needs an oracle that survives the refactor. If
  the old terminal renderer is replaced in-place, "old output vs new IR-backed
  output" (line 270) has nothing to compare against. Specify one of:
  (a) capture golden snapshots of current output **before** the change and
  diff against them, or (b) keep the old renderer behind a test-only path
  during migration. Point at the concrete corpus — the existing Prose unit
  tests in `prose.rs` and any Prose snapshots under `biscuit-terminal/lib/tests/`.
- **R11** — Acceptance criteria covers "unknown tag"; also require an unknown
  **token** (`{{frobnicate}}`) case, and verify both across all three
  non-terminal targets (escaped, not verbatim — ties to C2).

## Open Questions — Suggested Resolutions

| # | Question | Suggestion |
|---|----------|------------|
| 1 | IR visibility | Start `pub(crate)`. No external consumer in this feature; promote later if `TreeRenderable for Prose` lands. |
| 2 | Inline styles vs scoped CSS classes | Inline styles first — simpler, no `ComponentStylesheet` plumbing, matches "minimum code" (Rule 2). Revisit if Prose output volume makes it heavy. |
| 3 | GFM `~~` in plain Markdown | Decide now; align with the tree Markdown renderer's `Delete` behavior (R7). |
| 4 | Layout → CSS coverage | Margins only in pass one (R9). |
| 5 | Cache parsed doc vs parse-per-render | Keep parse-per-render for now (current behavior). Caching is a perf concern — cross-reference the existing perf-boost spec rather than deciding here. |

## Nits

- Lines 63/124/241 use "literal text" loosely — once C2 is resolved, make the
  wording consistent (escaped `Text`, not a verbatim node).
- Current `Prose` struct fields are `content: String` + `layout: Layout` — the
  "Current Behavior" section could note this since FR-9 depends on `Layout`.
- "fenced code blocks" appear in both inline-grammar discussion and the IR;
  consider one sentence clarifying Prose is "inline-plus-code-block", since a
  purely inline component wouldn't carry `CodeBlock`.
