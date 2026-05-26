---
status: draft
---

# Inline Span Extensions: Source-Rewrite via Strikethrough Overload

## Status

**Draft — brainstorming.** This spec captures a candidate architecture for
darkmatter's inline syntax extensions (`==mark==`, `⌄dim⌄`, and future
atomic / wrap-style inline features). It is not an implementation contract.
Open design questions are surfaced explicitly so the team can push on them
before committing.

The sibling spec
[`../2026-05-26-block-span/spec.md`](../2026-05-26-block-span/spec.md)
covers block-level extensions (HR attributes today, future
admonitions / containers / heading attrs). The two were originally
muddled together inside one span-aware processor for historical reasons;
this spec pair separates them.

Performance context lives in
[`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md).
The 18× span-aware-fold cost recorded against `mark_dim_hr` motivated this
investigation.

## Background

Pulldown-cmark recognizes CommonMark plus GFM extensions (tables, task
lists, strikethrough, autolinks, disallowed raw HTML). Anything outside
that set — including darkmatter's `==mark==`, `⌄dim⌄`, future emoji
shortcodes (`:smile:`), variable interpolation (`{{var}}`), inline math
(`$x^2$`), tooltips — is seen as **literal text** by the parser.

Today's solution is the `SpannedInlineStyleProcessor` in
`darkmatter/lib/src/markdown/render_tree/span.rs`. It:

1. Wraps every pulldown-cmark event in a `SpannedInlineEvent` that carries
   a byte range and a kind tag (Standard / Generated).
2. Scans every Text event for darkmatter delimiters.
3. Maintains an opener stack across text-event boundaries to support
   nesting.
4. Synthesizes typed events (`InlineTag::Mark`, `InlineTag::Dim`) that the
   fold lowers into tree spans.

This costs ≈ 164 µs on the `mark_dim_hr` benchmark fixture — about 18× the
plain `fold_markdown_to_document` (≈ 9 µs). The overhead is structural —
paid once-per-document on the span-aware lane, not once-per-feature — but
it's the dominant per-document cost when any darkmatter inline syntax is
used.

This spec proposes an alternative: **rewrite darkmatter inline syntax to a
GFM-strikethrough envelope at the source-text layer, let pulldown-cmark do
the structural recognition, and dispatch by token in a small fold-side
handler.**

## Proposed Architecture

### Source-layer rewrite

Before parsing, scan the source for darkmatter inline patterns and rewrite
each occurrence to the canonical form:

```
~~|TOKEN|payload|TOKEN|~~
```

Examples (paired forms):

| Source | Rewritten |
|--------|-----------|
| `==highlighted==`   | `~~|mark|highlighted|mark|~~` |
| `⌄dim text⌄`         | `~~|dim|dim text|dim|~~` |

Examples (atomic forms):

| Source | Rewritten |
|--------|-----------|
| `:smile:`           | `~~|emoji|smile|emoji|~~` |
| `{{username}}`      | `~~|var|username|var|~~` |
| `$x^2$`             | `~~|math|x^2|math|~~` (see [Math escape](#open-questions)) |
| `[term]^{def}`      | `~~|tooltip|term||def|tooltip|~~` (multi-field payload) |

### Parsing

Pulldown-cmark runs on the rewritten source with `ENABLE_STRIKETHROUGH`.
It emits `Tag::Strikethrough` events for the envelopes — paid as part of
its native parse pass, **free of darkmatter-specific overhead**.

### Fold-side dispatch

The plain fold (no `SpannedInlineEvent` wrapping) folds normally until it
encounters a `Strikethrough` container. At that point it peeks the first
child Text event:

- **No leading `|token|`** → emit `NodeKind::Delete` (standard
  strikethrough).
- **Leading `|token|`** where `token` is registered → strip the
  surrounding `|token|...|token|` envelope, parse the payload according to
  the token's handler, emit the corresponding tree node(s). Drop the
  outer `Delete` wrapper.
- **Leading `|token|`** where `token` is unknown → emit `NodeKind::Delete`
  with a diagnostic (or treat as literal — see
  [Error recovery](#open-questions)).

### Token registry

Each inline extension registers four small pieces:

1. **Source pattern** — the user-facing syntax the rewriter recognizes
   (e.g. `==X==`, `:NAME:`).
2. **Token name** — short kebab-case identifier (e.g. `mark`, `dim`,
   `emoji`, `var`, `math`).
3. **Fold handler** — receives the payload string, produces tree
   node(s). For wrap-style features, the payload is rendered as Markdown
   inline content (recursive); for atomic features, it's an identifier or
   typed value.
4. **Roundtrip rule** — markdown renderer emits the original source form
   when it encounters a tree node carrying the appropriate provenance hint.

## Goals

- Eliminate the per-event `SpannedInlineEvent` wrapping cost for documents
  that do not use inline extensions (most documents).
- For documents that **do** use inline extensions, replace per-text-event
  scanning with strikethrough-boundary detection (pulldown-cmark's job)
  plus a small per-strikethrough peek (the fold's job).
- Provide a single, uniform fold-side dispatcher so adding a new inline
  feature is "register a token" — not "extend the span-aware processor."
- Keep multi-target lowering intact: tokens still produce typed tree
  nodes that each target lowers in its own vocabulary
  (`<mark>` / SGR 7 / `==` etc.).
- Preserve byte-range provenance for diagnostics.

## Non-Goals

- Block-level darkmatter extensions (HR attributes, future containers).
  Owned by the sibling
  [`../2026-05-26-block-span/spec.md`](../2026-05-26-block-span/spec.md).
- Per-feature rasterization or graphics policy. Owned by
  [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
- Replacing pulldown-cmark or switching parser crates.
- Changing the renderable tree IR (`NodeKind`, `RenderNode`).

## Performance Hypothesis

The current 18× span-aware cost has three plausible components:

1. **Per-event wrapping** — allocating `SpannedInlineEvent { event, range,
   kind }` for every event.
2. **Per-text-event scanning** — looking at every Text event's bytes for
   delimiters.
3. **Opener-stack bookkeeping** — cross-text-event state.

This proposal eliminates (1) and (2) on the plain document and on
non-strikethrough events; (3) collapses into the rewriter (balanced
recognition during source scan) and a small per-strikethrough peek.

**Expected savings on documents with darkmatter inline:** substantial but
unquantified — needs profiling. **Expected savings on documents without
darkmatter inline:** the rewriter exits immediately (no patterns matched),
plain fold runs, savings approach the full 155 µs difference between
span-aware and plain folds.

**Gating decision:** profile the current `SpannedInlineStyleProcessor`
before scheduling this work. If component (1) dominates, this is a big
win. If (3) dominates, this approach barely helps.

## Open Questions

### Token format

`|tokenname|` is one choice. Alternatives:

- ASCII brackets with a sigil: `:{tokenname}:` — but `:` is the emoji
  shortcode opener; would conflict.
- Non-character Unicode codepoints (e.g. `U+FDD0`) — guaranteed not to
  appear in user content, but makes the rewritten source hard to
  inspect in diagnostics / debugger.
- Pipe-wrapped: `|tokenname|` — readable, safe inside strikethrough
  (since pipes don't terminate strikethrough), but conflicts with table
  syntax if a strikethrough ever ends up inside a table cell.

Recommendation TBD.

### Payload escaping

Some payloads contain characters that conflict with the envelope:

- **Math**: `$a~~b$` would rewrite to `~~|math|a~~b|math|~~`, where the
  inner `~~` closes the outer strikethrough early. Mitigations:
  - Escape `~` inside math payloads (`a\~\~b`); the math handler
    un-escapes on the fold side.
  - Use a different envelope for math (e.g. inline code with a token
    prefix: `` `|math|x^2|math|` ``).
  - Decide math is not in scope for this architecture and use a separate
    mechanism.
- **Pipes in payloads**: `==text|with|pipes==` rewrites to
  `~~|mark|text|with|pipes|mark|~~`. The token-pair recognition (leading
  AND trailing `|mark|`) is unambiguous, but a parser that splits on `|`
  naively would misread the payload. The fold handler must search for
  the *trailing* `|mark|` from the end, not split on every `|`.

### Multi-field payloads

Tooltips and similar constructs have multiple fields. One scheme:

```
~~|tooltip|term||def|tooltip|~~
```

A double-pipe `||` separates fields within the payload. The handler
splits on `||` after stripping the envelope. Open: what's the escape
rule for legitimate `||` in user content? Probably `\|\|`, but needs
spec-level decision.

### Provenance translation

The rewriter shifts byte positions. Pulldown-cmark's offsets point into
the rewritten string; diagnostics need original-source positions.

Two approaches:

1. **Translation table** — a sorted list of `(rewritten_offset,
   original_offset, delta)` entries, one per rewrite site. Binary search
   to map any offset. Memory cost: O(rewrites); lookup cost: O(log
   rewrites).
2. **Inline marker bytes** — wrap rewritten payloads with sentinel bytes
   that the fold uses to recover original ranges. Avoids a separate
   table but pollutes the rewritten source.

Recommendation TBD; (1) is the obvious choice unless a profile shows
table lookup is a bottleneck.

### Conflict with legitimate user `~~|...|...|~~`

A user could legitimately write `~~|literal-pipes|inside strike|literal-
pipes|~~` as ordinary struck-through text. Today the proposal would
silently dispatch as if `literal-pipes` were a registered token (probably
emit a diagnostic — see below).

Mitigations:

- The token registry is closed at compile time; unknown tokens fall
  through to standard strikethrough. So the conflict only fires when the
  user's literal text exactly matches a registered token name.
- Escape mechanism: `\|mark|...` opts out of rewriting. The rewriter
  honors the backslash and emits plain strikethrough.
- Documentation: list reserved token names, recommend escaping in any
  prose that legitimately uses `|tokenname|` syntax.

### Markdown roundtrip

When rendering tree → Markdown, the renderer must emit the original
source form, not the rewritten envelope. Strategy:

- Tree nodes carry a hint indicating their darkmatter origin
  (e.g. `attrs.classes = ["mark"]` for `==mark==`).
- The markdown renderer matches on the hint and emits `==…==` (or the
  appropriate source form per token).
- Tree nodes constructed *programmatically* (not from `==` source) with
  the same hint also emit `==…==` — this is correct behavior; the user
  can't tell whether the source was `==` or programmatic.

### Error recovery

What happens on malformed input?

- `~~|mark|unclosed` (no closing token, no closing strikethrough) →
  pulldown-cmark emits literal text; rewriter never wrote that envelope;
  user's source had a literal `~~|mark|unclosed` which renders as plain
  strikethrough opener followed by literal text. No darkmatter
  semantics fire. Probably fine.
- `==unclosed`: rewriter sees one `==` with no pair; emits literal `==`
  unchanged into source. No envelope, no token, no diagnostic.
  Consistent with how today's processor treats unclosed openers.
- `==text==text==` (three `==` in a row): the rewriter pairs greedily —
  first two delimit, third becomes orphan literal. Document this.

### Block-level interaction

This spec is inline-only. HR attributes (`--- { style: ... }`) are
**block-level** and intentionally out of scope here — the sibling
block-span spec owns them. The rewriter for this spec should explicitly
ignore patterns that look like block-level constructs (lines starting
with `---`, fenced blocks, etc.).

### Rewriter performance

The rewriter must scan source bytes for all registered delimiter
patterns. If implemented as N independent passes (one per token), cost
multiplies with token count — same trap as the current span-aware
processor. **Architectural rule:** the rewriter must scan in a single
pass, using a shared DFA or memchr-style multi-pattern scan. New tokens
add patterns to the shared scan, not new passes.

## Decision Sequencing

1. **Profile the current `SpannedInlineStyleProcessor`** (gating
   decision). If wrapping + scanning dominate, this proposal has its
   expected payoff. If opener-stack bookkeeping dominates, it doesn't.
2. **Decide token format and payload escape rules** (above) based on a
   small design pass.
3. **Prototype the rewriter** in isolation with a single token (`mark`).
   Verify behavior on the existing test corpus.
4. **Implement the fold-side dispatcher**. Run against the existing
   `mark_dim_hr` benchmark fixture and `migration_parity` suite. Compare
   numbers against the current span-aware path.
5. **Roll forward** if profiling and prototype confirm the win;
   otherwise file as "considered, deferred" in the perf spec.

## Cross-Bucket Summary

This proposal would graduate to a Bucket-2 framework item in
[`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md)
if profiling supports it. Until then it lives here as a design
investigation.

## Out of Scope

- Block-level darkmatter extensions — owned by
  [`../2026-05-26-block-span/spec.md`](../2026-05-26-block-span/spec.md).
- Graphics / image policy — owned by
  [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
- Replacing pulldown-cmark.
- Changes to the renderable tree IR.

## Related Specs

- [`../2026-05-26-block-span/spec.md`](../2026-05-26-block-span/spec.md) —
  sibling spec for block-level extensions.
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) —
  the perf spec; owns scheduling.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  cross-target graphics policy (unrelated; listed for navigation).
- [`../_completed/2026-05-20-darkmatter-tree/spec.md`](../_completed/2026-05-20-darkmatter-tree/spec.md) —
  the parent migration spec.
- [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md) —
  the recorded `mark_dim_hr` ratios that motivated this investigation.
