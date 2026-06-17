# Phase 1 — Prototype Envelope Decisions (Locked)

Prototype: `darkmatter/lib/tests/inline_envelope_prototype.rs`
Spec: [`spec.md`](./spec.md) · Plan: [`plan.md`](./plan.md)

These are the implementation-level decisions the prototype locked before any
public IR or renderer behavior changes. Phases 2–6 must build to these.

## 1. Envelope marker — **CHANGED from the spec's primary form**

The spec's *primary* marker `<<TOKEN>>` U+FDD0 is **broken** and is not used.

**Finding (proven by `angle_bracket_marker_is_parsed_as_inline_html`):**
pulldown-cmark tokenizes the inner `<token>` of `<<token>>` as inline HTML.
`<mark>` is a real HTML element name, and even `<dim>` satisfies the
CommonMark inline-HTML open-tag production (`<` + ASCII letter + …). So
`<<mark>>` parses to:

```text
Text("<") · InlineHtml("<mark>") · Text(">\u{FDD0}")
```

This is exactly the failure mode the spec's Open Questions anticipated
("If it falsely matches … a partial HTML tag, the format breaks and we need a
different visible bracket pair (e.g. `{{|NAME|}}`)").

**Locked marker:** the **pipe-free** form,

```text
{{!TOKEN!}}
```

immediately followed by the U+FDD0 sentinel. Curly braces and `!` are inert in
CommonMark inline parsing (`!` is only special as the image lead-in `![`, which
the marker never forms), so `{{!mark!}}` survives verbatim as literal
`Event::Text`. Proven for `mark`, `dim`, `emoji`, `var`, `tooltip` by
`locked_marker_survives_for_token_names`.

> **REVISED (review-1 finding 2).** The original locked form was `{{|TOKEN|}}`.
> Its rationale — "pipes are only special inside GFM table rows, and the
> envelope never sits in one" — was wrong: `mark`/`dim` are ordinary inline
> spans and *do* appear in table cells. Inside a cell the raw `|` bytes were
> counted as extra column separators and split the row. The marker is now
> pipe-free; `locked_marker_is_pipe_free_and_table_safe` guards it.

The U+FDD0 sentinel (a permanent Unicode non-character) is what makes the
marker collision-proof: a user can type `{{!mark!}}` in prose, but never an
adjacent U+FDD0. The fold must require **marker + sentinel** together.

**Canonical envelope (paired form):**

```text
~~{{!TOKEN!}}\u{FDD0}payload{{!TOKEN!}}\u{FDD0}~~
```

The same `{{!TOKEN!}}`U+FDD0 marker appears at both opener and closer. The
fold finds the first marker after the opening `~~` and the last marker before
the closing `~~`; the payload is the content between.

| Source | Rewritten (sentinel shown as `␦`) |
|--------|------------------------------------|
| `==highlighted==` | `~~{{!mark!}}␦highlighted{{!mark!}}␦~~` |
| `⌄dim text⌄` | `~~{{!dim!}}␦dim text{{!dim!}}␦~~` |

**Verbatim regions.** The rewriter only rephrases eligible prose: a one-time
`pulldown-cmark` pre-parse records code spans, fenced/indented code blocks, raw
HTML, link destinations/titles, and image constructs, and any delimiter inside
one of those stays literal (review-1 finding 1). The pre-parse runs only when a
candidate delimiter is present, so no-inline documents keep the cheap scan.

## 2. Multi-field payload separator — **U+FDD1**

Locked: **U+FDD1** (the next Unicode non-character after the marker sentinel).
Proven by `multi_field_sentinel_survives_in_payload`: U+FDD1 passes through
pulldown-cmark as literal text exactly like U+FDD0, so the fold can split
multi-field payloads (e.g. tooltip `term ‖ def`) after stripping the markers.

No visible separator is adopted. A visible-separator decision is deferred to
the first multi-field feature that ships beyond `tooltip`; until then the
non-character keeps the format bulletproof with no escape rule for user
content. (`tooltip` itself is out of Phase 2–6 scope; `mark` and `dim` are
single-field and never use the separator.)

## 3. Provenance translation table — **`Vec<(usize, usize)>` sorted by rewritten offset**

Locked: `Vec<(rewritten_offset, original_offset)>`, sorted ascending by
`rewritten_offset`, resolved by binary search. Memory O(rewrites), lookup
O(log rewrites). One entry is pushed at each point where the rewritten and
original byte streams diverge (the start of each rewrite site and the point
where they re-converge). The fold applies it when constructing
`renderable::tree::SourceSpan`s and diagnostics so every emitted node still
points back to **original** source bytes, not rewritten ones.

A no-extension document produces an empty table and (per Phase 4) a borrowed,
unchanged source — the all-bytes-map-1:1 case needs no entries.

## 4. Escape semantics

Mirror the existing `SpannedInlineStyleProcessor`
(`darkmatter/lib/src/markdown/render_tree/span.rs`) wherever behavior already
exists; define the new tokens consistently.

| Source | Meaning | Existing behavior to match |
|--------|---------|----------------------------|
| `\==` | literal `==`, no `mark` rewrite | **Yes** — span processor reverts `\==` to a literal `==` whose byte range covers the backslash (`escaped_mark_delimiter_*` test). pulldown consumes `\=` as a CommonMark escape, so the rewriter must recover the backslash from the source bytes just before the delimiter. |
| `\⌄` | literal `⌄`, no `dim` rewrite | **Yes** — span processor sets the in-text-event `escaped` flag and emits a literal `⌄` covering the backslash (`escaped_dim_delimiter_*` test). |
| `\:` | literal `:`, no `emoji` rewrite | **New** — no existing behavior; define to match the same rule (backslash suppresses the rewrite, delimiter stays literal). |
| `\{{` | literal `{{`, no `var` rewrite | **New** — no existing behavior; define to match the same rule. |

An escaped delimiter is **never** rewritten into an envelope; it stays literal
text in the rewritten source so existing documents that quote `==literal==`
do not break.

## 5. Error recovery (locked)

- `==unclosed` → no closing pair found → rewriter leaves the literal `==`,
  emits no envelope.
- `==a==b==` (three openers) → pair greedily left-to-right; the third `==`
  becomes an orphan literal. Document in user-facing docs when `mark`/`dim`
  ship.
- A malformed envelope reaching the fold (rewriter bug) → fold-side dispatcher
  emits a diagnostic and falls through to standard `NodeKind::Delete`. Defense
  in depth; should be unreachable when rewriter and fold share one registry.

## 6. Rewriter architectural rule — **one shared scan**

Hard rule for the future rewriter module (Phase 4). Capture verbatim as that
module's `//!` doc:

> All inline token patterns are matched in **one shared scan** of the source
> text (a single multi-pattern matcher — `memchr`-based or `aho-corasick`),
> never one pass per token. Adding a new inline token adds a pattern to the
> shared scan; it must not add another full pass over the source. This is the
> direct equivalent of the prior architecture's rule that "all inline features
> share one processor walk," and it is the reason this rewrite exists.

## Status

All six prototype tasks resolved; all eight prototype tests pass
(`cargo test -p darkmatter --test inline_envelope_prototype`). No production
behavior has changed in Phase 1 — the only added artifact is the prototype
test, which exercises pulldown-cmark directly and is retained as a regression
guard.
