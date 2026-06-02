---
status: draft
---

# Inline Span Extensions: Source-Rewrite via Strikethrough Overload

## Status

**Draft — architecture approved.** The core shape (source rewrite into a
GFM-strikethrough envelope, plain fold, fold-side dispatcher) is locked.
A small number of implementation-level sub-decisions remain — see
[Open Questions](#open-questions) — but they don't change the
architecture and can be resolved during the prototype step.

Decision lineage (recording the brainstorm so the choices aren't
re-litigated):

| Question | Decision | Notes |
|---|---|---|
| Profile-gated or architecture-driven? | **Architecture-driven.** | Perf upside is verified post-implementation, not a precondition. |
| Token marker format | **`<<NAME>>U+FDD0`** | Readable ASCII tag plus a Unicode non-character sentinel for collision-proofing. |
| Math (`$x^2$`) in scope? | **No.** | If/when math arrives it will likely use a different envelope (inline code) — verbatim payload semantics suit math better. |
| Tree IR shape | **Hybrid `NodeKind::Extended { token, children, payload }`** | One new variant; renderers dispatch by token name. |
| Migration strategy | **Big-bang replace.** | Single in-flight branch; finite call sites; coexistence costs more than it saves. |
| Behavior parity vs fidelity recovery | **Recover `<mark>` element fidelity.** | This is the right moment to fix the legacy-vs-tree `<mark>` → `<span class="mark">` regression. |

The sibling spec
[`../2026-05-26-block-extension/spec.md`](../2026-05-26-block-extension/spec.md)
covers the narrow lift of HR attributes out of the inline span-aware
processor; it deliberately does not design a general block-extension
architecture (that would be premature from one data point).
Performance context lives in
[`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md).

## Background

Pulldown-cmark recognizes CommonMark plus GFM extensions (tables, task
lists, strikethrough, autolinks, disallowed raw HTML). Anything outside
that set — including darkmatter's `==mark==`, `⌄dim⌄`, future emoji
shortcodes (`:smile:`), variable interpolation (`{{var}}`), tooltips — is
seen as **literal text** by the parser.

Today's solution is the `SpannedInlineStyleProcessor` in
`darkmatter/lib/src/markdown/render_tree/span.rs`. It wraps every
pulldown-cmark event in a `SpannedInlineEvent`, scans every Text event for
darkmatter delimiters, maintains an opener stack across text-event
boundaries, and synthesizes typed events that the fold lowers into tree
spans. This costs ≈ 164 µs on the `mark_dim_hr` benchmark fixture — about
18× the plain `fold_markdown_to_document` (≈ 9 µs). The overhead is
structural, paid once-per-document on the span-aware lane.

This spec replaces that architecture with: **rewrite darkmatter inline
syntax to a GFM-strikethrough envelope at the source-text layer, let
pulldown-cmark do the structural recognition, and dispatch by token in a
small fold-side handler.**

## Proposed Architecture

### Source-layer rewrite

Before parsing, scan the source for darkmatter inline patterns and rewrite
each occurrence to the canonical envelope:

```
~~<<TOKEN>>U+FDD0 payload <<TOKEN>>U+FDD0~~
```

The token marker is the literal ASCII tag `<<NAME>>` immediately followed
by the Unicode non-character codepoint **U+FDD0**. The non-character
sentinel is what makes the marker collision-proof: even if a user
legitimately writes `<<mark>>` in prose, they will never adjacent-paste a
U+FDD0 codepoint by accident.

The same marker form appears at both opener and closer; the fold finds
the first marker after the opening `~~` and the last marker before the
closing `~~`. Payload is the content between.

Examples (paired forms):

| Source | Rewritten (sentinels shown as `␦`) |
|--------|------------------------------------|
| `==highlighted==`   | `~~<<mark>>␦highlighted<<mark>>␦~~` |
| `⌄dim text⌄`         | `~~<<dim>>␦dim text<<dim>>␦~~` |

Examples (atomic forms):

| Source | Rewritten |
|--------|-----------|
| `:smile:`           | `~~<<emoji>>␦smile<<emoji>>␦~~` |
| `{{username}}`      | `~~<<var>>␦username<<var>>␦~~` |
| `[term]^{def}`      | `~~<<tooltip>>␦term‖def<<tooltip>>␦~~` (`‖` = field separator; see [Multi-field payloads](#multi-field-payloads)) |

### Parsing

Pulldown-cmark runs on the rewritten source with `ENABLE_STRIKETHROUGH`.
It emits `Tag::Strikethrough` events for the envelopes — paid as part of
its native parse pass, free of darkmatter-specific overhead.

### Fold-side dispatch

The plain fold (no `SpannedInlineEvent` wrapping) folds normally until it
encounters a `Strikethrough` container. At that point it peeks the first
child Text event:

- **No leading `<<NAME>>U+FDD0`** → emit `NodeKind::Delete` (standard
  strikethrough).
- **Leading `<<NAME>>U+FDD0`** where `NAME` is registered → strip the
  envelope, parse the payload according to the token's handler, emit an
  `NodeKind::Extended { token, children, payload }` node. Drop the outer
  `Delete` wrapper.
- **Leading `<<NAME>>U+FDD0`** where `NAME` is unknown → emit
  `NodeKind::Delete` with a diagnostic. (Should be impossible if rewriter
  and fold share the same token registry; the diagnostic catches a
  registry drift bug.)

### Tree IR addition

A single new variant in `renderable::tree::NodeKind`:

```rust
NodeKind::Extended {
    token: Cow<'static, str>,
    children: Vec<RenderNode>,
    payload: Option<String>,
}
```

- `token` is the registered extension identifier (`"mark"`, `"dim"`,
  `"emoji"`, etc.). `Cow<'static, str>` so built-in tokens are
  zero-allocation literals; dynamic tokens (if any) own their string.
- `children` carries nested tree content for wrap-style features (mark,
  dim, tooltip's term). Empty `vec![]` for atomic features.
- `payload` carries an identifier or scalar value for atomic features
  (emoji name, var name) and the secondary field for multi-field
  features (tooltip's definition). `None` for pure wrap features.

Renderers dispatch on `token.as_str()`. Unknown tokens fall back to a
sensible default (`<span class="extended-{token}">` in browser, plain
text in terminal and markdown).

### Per-target lowering for built-in tokens

| Token | Browser | Terminal | Markdown roundtrip |
|---|---|---|---|
| `mark` | `<mark>` (semantic element, recovering legacy fidelity) | Reverse video (SGR 7) | `==children==` |
| `dim` | `<span style="opacity:0.6">` | `<dim>` Prose markup → SGR 2 (`\x1b[2m`) | `⌄children⌄` |
| `emoji` | `<span class="emoji">{payload}</span>` (or a future emoji-lookup hook) | `:{payload}:` literal or a future emoji-lookup | `:{payload}:` |
| `var` | `<span class="var">{payload}</span>` (or evaluated value if context provides) | `{payload}` literal (or evaluated) | `{{payload}}` |
| `tooltip` | `<span title="{payload}">{children}</span>` | `{children}` (terminals can't tooltip) | `[children]^{payload}` |

The `mark` lowering specifically **recovers the `<mark>` semantic
element** that the legacy-vs-tree comparison identified as a fidelity
regression. The browser renderer matches `Extended { token: "mark", … }`
and emits `<mark>…</mark>` instead of the current `<span class="mark">`.

### Token registry

Each inline extension registers four small pieces in a central registry:

1. **Source pattern** — the user-facing syntax the rewriter recognizes
   (e.g. `==X==`, `:NAME:`).
2. **Token name** — short kebab-case identifier (`mark`, `dim`, `emoji`,
   `var`, `tooltip`).
3. **Fold handler** — receives the payload string, decides whether the
   payload is itself Markdown content (parsed recursively into
   `children`) or a scalar (stored in `payload`), produces the
   `NodeKind::Extended` node.
4. **Roundtrip rule** — markdown renderer's per-token emission when it
   encounters `NodeKind::Extended { token, … }`.

## Concrete Token Examples

For reference (not normative; the actual signatures will be settled in
the prototype). These show how each token type fits the registration
shape.

```rust
// mark — wrap-style, payload is nested Markdown inline content.
register_inline_token(InlineTokenSpec {
    name: "mark",
    source_pattern: SourcePattern::Paired("=="),
    fold_handler: |payload, ctx| {
        let children = ctx.parse_inline_markdown(payload);
        NodeKind::Extended {
            token: Cow::Borrowed("mark"),
            children,
            payload: None,
        }
    },
    roundtrip: |node, out| {
        out.push_str("==");
        out.render_inline_children(&node.children);
        out.push_str("==");
    },
});

// emoji — atomic, payload is a shortcode identifier.
register_inline_token(InlineTokenSpec {
    name: "emoji",
    source_pattern: SourcePattern::Atomic { open: ":", close: ":" },
    fold_handler: |payload, _ctx| NodeKind::Extended {
        token: Cow::Borrowed("emoji"),
        children: vec![],
        payload: Some(payload.into()),
    },
    roundtrip: |node, out| {
        out.push(':');
        out.push_str(node.payload.as_deref().unwrap_or(""));
        out.push(':');
    },
});

// tooltip — multi-field, payload is the definition, children are the term.
register_inline_token(InlineTokenSpec {
    name: "tooltip",
    source_pattern: SourcePattern::TooltipForm, // [term]^{def}
    fold_handler: |raw_payload, ctx| {
        // The rewriter encoded the two fields with the U+2016 separator.
        let (term, def) = split_fields(raw_payload);
        let children = ctx.parse_inline_markdown(term);
        NodeKind::Extended {
            token: Cow::Borrowed("tooltip"),
            children,
            payload: Some(def.into()),
        }
    },
    roundtrip: |node, out| {
        out.push('[');
        out.render_inline_children(&node.children);
        out.push_str("]^{");
        out.push_str(node.payload.as_deref().unwrap_or(""));
        out.push('}');
    },
});
```

## Goals

- Eliminate the per-event `SpannedInlineEvent` wrapping for inline
  extension processing.
- Provide a single, uniform fold-side dispatcher so adding a new inline
  feature is "register a token" — not "extend a span-aware processor."
- Recover `<mark>` element fidelity (closes a known regression vs the
  legacy HTML renderer).
- Keep multi-target lowering intact: tokens produce typed tree nodes
  that each target lowers in its own vocabulary.
- Preserve byte-range provenance for diagnostics.
- Replace the existing `SpannedInlineStyleProcessor` cleanly — big-bang
  cutover, no coexistence period.

## Non-Goals

- **Math (`$x^2$`).** Out of scope for this architecture. Math's
  verbatim payload semantics suit a different envelope (likely inline
  code) and a future spec.
- **Block-level darkmatter extensions** (HR attributes today; future
  block extensions if any arrive). Owned by
  [`../2026-05-26-block-extension/spec.md`](../2026-05-26-block-extension/spec.md).
- **Graphics / image policy.** Owned by
  [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md).
- **Replacing pulldown-cmark or switching parser crates.**
- **Per-target style policy.** Renderers decide their own lowerings; the
  IR carries intent (`token`), not implementation.

## Migration Plan

Single-step replacement, ordered by reverse dependency so each step lands
on a green tree:

1. **Add `NodeKind::Extended` to `renderable::tree`.** Validation +
   default no-op handling in every existing renderer (browser, terminal,
   markdown). At this stage no fold produces the variant; this is
   IR-only scaffolding.
2. **Implement per-target lowering for built-in tokens** (`mark`, `dim`)
   in the three tree renderers. Each renderer's `NodeKind::Extended` arm
   matches on `token` and emits the appropriate output.
3. **Build the source rewriter** (`darkmatter::markdown::rewrite` or
   similar) with the `<<NAME>>U+FDD0` envelope, the registry shape, and
   the `==`/`⌄` handlers.
4. **Build the fold-side dispatcher** that consumes
   `Tag::Strikethrough` events, peeks for the envelope, and emits
   `NodeKind::Extended` nodes.
5. **Replace `fold_markdown_spanned_with_frontmatter`'s public surface**
   with a fold that runs the rewriter then plain-folds. Same function
   signature; internal mechanism replaced.
6. **Delete `SpannedInlineStyleProcessor`** and its support types from
   `darkmatter/lib/src/markdown/render_tree/span.rs`. Confirm no other
   consumers.
7. **Run the full test corpus + `migration_parity` benches.** Verify
   byte-identical output for `mark_dim_hr` (except the deliberate
   `<mark>` recovery — that fixture's HTML expectations update). Record
   new perf numbers; this is where the architecture-driven-vs-perf-gated
   decision gets its receipts.

The HR-attribute path stays in the current span-aware processor *until*
the sibling block-extension spec's Phase-1 refactor lifts it into its own
block-prefix scanner. The two specs' implementations interleave at step
6: don't delete `SpannedInlineStyleProcessor` until HR attributes have
moved.

## Performance Expectation

Architecture-driven, so this section is expectation rather than gate.

- **Documents without darkmatter inline:** rewriter scans the source
  once, finds no patterns, exits. Cost approaches one `memchr` pass
  (microseconds per MB). Fold runs at plain-fold speed (≈ 9 µs on the
  small fixtures). **Expected: substantial improvement over the current
  always-span-aware path.**
- **Documents with darkmatter inline:** rewriter scans the source once,
  rewrites in place (one allocation for the rewritten string + a
  translation table for provenance), fold runs at plain-fold speed plus
  a small per-strikethrough peek-and-dispatch. **Expected: meaningful
  improvement over the 164 µs span-aware baseline, but exact magnitude
  depends on where the 18× cost actually lives — verified by the
  `migration_parity` numbers in step 7.**

If post-implementation measurement shows no perf improvement, the
architectural cleanup still stands — the extensibility and `<mark>`
fidelity wins justify the change on their own. The perf upside is the
expected outcome, not the justification.

## Open Questions

The architecture is locked; these are implementation-level sub-decisions
that surface during the prototype step.

### Pulldown-cmark interaction with `<<NAME>>`

Confidence is high (~95%) that pulldown-cmark passes `<<NAME>>` through
as literal text — `<` followed by non-letter content isn't a tokenizer
trigger. **Smoke-test this in the prototype before locking the
envelope.** If it falsely matches as an autolink or partial HTML tag,
the format breaks and we need a different visible bracket pair (e.g.
`{{|NAME|}}`).

> **RESOLVED (Phase 1).** The `<<NAME>>` form is **broken**: pulldown-cmark
> tokenizes the inner `<name>` as inline HTML (`<mark>` is a real element,
> `<dim>` still matches the open-tag production). The locked envelope marker
> is the **pipe-free** **`{{!TOKEN!}}`** + U+FDD0, giving the canonical envelope
> `~~{{!TOKEN!}}\u{FDD0}payload{{!TOKEN!}}\u{FDD0}~~`. See
> [`phase-1-prototype-notes.md`](./phase-1-prototype-notes.md) and the
> prototype `darkmatter/lib/tests/inline_envelope_prototype.rs`. All later
> references to `<<NAME>>` / `<<TOKEN>>` in this spec are superseded by
> `{{!TOKEN!}}`.
>
> **REVISED (review-1).** The marker was originally `{{|TOKEN|}}`; the `|` bytes
> corrupted GFM table cells holding a mark/dim span, so the marker is now
> pipe-free (`{{!TOKEN!}}`). The rewriter also pre-parses the source to leave
> code spans, code blocks, raw HTML, link destinations/titles, and image
> constructs untouched. See `darkmatter/lib/src/markdown/render_tree/inline_extension.rs`.

### Multi-field payload separator

Tooltips encode `[term]^{def}` as `<<tooltip>>U+FDD0 term ‖ def
<<tooltip>>U+FDD0`. The separator `‖` (U+2016, double vertical line)
is a placeholder; the prototype should commit to a specific separator
character. Two reasonable options:

- A second non-character codepoint (e.g. U+FDD1) — bulletproof but
  unreadable.
- A visible punctuation character that's unlikely in payload content
  (e.g. `‖`, `⁞`, or just `\u{1F}` ASCII unit-separator) — readable but
  needs an escape rule for user content that legitimately contains it.

Recommendation: U+FDD1 for symmetry with the marker sentinel; defer
visible-separator decision to the first multi-field feature beyond
tooltips.

### Provenance translation table layout

Translation table maps `(rewritten_offset → original_offset)`. Two
shapes work:

- `Vec<(usize, usize)>` sorted by rewritten offset; binary search to
  resolve. Memory: O(rewrites); lookup: O(log rewrites).
- Per-rewrite-site list of (start, len_delta) entries; cumulative scan
  to resolve. Same complexity, slightly different cache profile.

Pick during the prototype; either works.

### User-content escape

A user who legitimately wants `==literal==` in prose (e.g. quoting
Markdown source) needs an escape. Convention: `\==` in source means
"literal `==`, no rewrite." The rewriter honors the backslash and emits
plain text. Same rule for `\⌄`, `\:`, `\{{`, etc.

The current `SpannedInlineStyleProcessor` already handles `\==`
specifically; the rewriter should mirror that semantics so existing
documents don't break.

### Error recovery

Mechanical once the format is locked, but pin down explicitly:

- `==unclosed` → no pair found, rewriter emits literal `==`.
- `==text==text==` (three openers) → pair greedy left-to-right; third
  becomes orphan literal. Document this in user-facing docs.
- Rewriter produces a malformed envelope (bug): fold-side dispatcher
  emits a diagnostic and falls through to standard `Delete`. Defense in
  depth.

### Rewriter as architectural surface

The rewriter must scan in **one pass** using a shared multi-pattern
matcher (memchr-based or aho-corasick), not N independent passes per
token. New tokens add patterns to the shared scan, not new passes. This
is the equivalent of the architectural rule from the previous
investigation: "all inline features must share one processor walk."

Document this as a hard rule in the rewriter's module docs so future
contributors don't accidentally regress it.

## Decision Sequencing

1. **Prototype the rewriter** with just `mark`. Verify pulldown-cmark
   passes `<<NAME>>` through as literal text. Lock the envelope format.
2. **Add `NodeKind::Extended`** to the renderable tree IR with default
   no-op handling in all three renderers. Land as a standalone PR — IR
   scaffolding only.
3. **Implement `mark` and `dim` lowering** in the three renderers.
   Including the `<mark>` element recovery in browser.
4. **Wire the rewriter + fold dispatcher** into
   `fold_markdown_spanned_with_frontmatter`. Keep the existing
   `SpannedInlineStyleProcessor` in place; route a feature flag to
   choose which.
5. **Run the full test corpus.** Diff outputs. Expected diff: `<mark>`
   for `==mark==` in HTML; everything else byte-identical.
6. **Flip the default** and remove the feature flag.
7. **Delete `SpannedInlineStyleProcessor`** once the block-extension spec's
   Phase-1 has lifted HR attributes out.
8. **Run `migration_parity`** and record numbers in
   `../_completed/2026-05-20-darkmatter-tree/baselines.md` (or this
   spec's eventual `baselines.md`).

## Out of Scope

- Math, block-level extensions, graphics policy — explicit non-goals
  above.
- Replacing pulldown-cmark.
- Changes to the tree IR beyond `NodeKind::Extended`.
- Per-feature renderer policies (theming, accessibility hooks) — those
  layer on top of the typed-node lowering, in each renderer.

## Related Specs

- [`../2026-05-26-block-extension/spec.md`](../2026-05-26-block-extension/spec.md) —
  sibling spec; lifts HR attributes out of `SpannedInlineStyleProcessor`.
  Implementations interleave at the `SpannedInlineStyleProcessor`
  deletion step (block-extension's Phase 1 must land first).
- [`../2026-05-21-isolated-perf/spec.md`](../2026-05-21-isolated-perf/spec.md) —
  the perf spec. Once measurement lands in step 8, the perf numbers
  graduate from this spec's expectation section into the perf spec's
  recorded outcomes.
- [`../2026-05-26-graphics-policy/spec.md`](../2026-05-26-graphics-policy/spec.md) —
  cross-target graphics policy (unrelated; listed for navigation).
- [`../_completed/2026-05-20-darkmatter-tree/spec.md`](../_completed/2026-05-20-darkmatter-tree/spec.md) —
  the parent migration spec.
- [`../_completed/2026-05-20-darkmatter-tree/baselines.md`](../_completed/2026-05-20-darkmatter-tree/baselines.md) —
  recorded `mark_dim_hr` ratios that motivated the investigation.
