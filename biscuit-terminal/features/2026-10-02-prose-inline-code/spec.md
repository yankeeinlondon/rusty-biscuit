---
area: biscuit-terminal
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
created: 2026-10-02
owner: Ken Snyder <ken@ken.net>
packages:
    - biscuit-terminal
    - renderable
    - darkmatter
human_review: false
clarified: false
reviewed: false
needs_rulings: true
---

# Prose Code Spans Project to Inline-Code Nodes

## Problem

Prose has no notion of inline code. A code span is made opaque during
Markdown pre-processing (`escape_code_spans` in
`lib/src/components/prose/markdown.rs`) and then reaches the render tree as
ordinary `Text`, backticks included. Every target therefore shows the
backticks literally and nothing else:

| Input | Terminal | Markdown | HTML |
|---|---|---|---|
| `` See [`inline-block`](u) `` | OSC 8 link, label is plain `` `inline-block` `` | unchanged | ``<a href="u">`inline-block`</a>`` |
| `` See `plain_code` `` | `` `plain_code` `` as plain text | unchanged | `` `plain_code` `` as plain text |

The render tree already has `NodeKind::InlineCode`, and every shared renderer
handles it: the terminal renderer (`render_tree/render.rs`) styles it dim, or
with the prose theme background when the caller supplies one; the browser
renderer (`renderable/src/tree/render/browser.rs`) emits `<code>`; the
Markdown renderer (`renderable/src/tree/render/markdown.rs`) emits a
backtick span. Darkmatter documents already use it. Prose is the only
producer that never creates the node.

### Incident: the inside-out rewrite

Commit `0e02bb613` worked around the missing node for one case. A code span
whose whole content is one link, `` `[desc](ref)` ``, is rewritten to
`` [`desc`](ref) `` so the link survives. That rule:

- turns example syntax into a live link. Darkmatter's `LinkError::UnrecognizedFormat`
  body, ``Input did not look like an HTML `<a>` tag or a Markdown `[text](href)` link.``,
  now renders as "a Markdown `text` link" (snapshot
  `l1__error_snapshots__link__unrecognized_format.snap`);
- modifies Markdown output: `` `[a](u)` `` renders to Markdown as
  `` [`a`](u) ``;
- departs from CommonMark, where a code span is always literal.

It exists so templates can wrap a generated link in backticks, as in
`` The `{{ link(plan) }}` plan `` (`prompts/plan.md`,
`prompts/_reviews/review-spec-inline.md`, and fixture copies of the latter in
`claudine/cli/tests/fixtures/nested_span_regression/` and
`darkmatter/dmls/tests/fixtures/mapping_only_corpus/`).

## Expected Behavior

### Every code span becomes an `InlineCode` node

The Prose projection emits `NodeKind::InlineCode` for each code span, in
every context where Prose grammar applies: top level, inside a bracketed
style tag, and inside a link's text.

```text
See `plain_code` here        →  Text("See ") InlineCode("plain_code") Text(" here")
See [`inline-block`](u) here →  Text("See ") Link(u)[InlineCode("inline-block")] Text(" here")
<red>run `md hash`</red>      →  Span(red)[Text("run ") InlineCode("md hash")]
```

The span recognition rules do not change: a backtick run opens a span only
when a later run of the same length closes it, and an unmatched run is
literal text.

The node's `value` is the span content with these adjustments, in order:

1. Prose backslash escapes are resolved, as today (`` `a\_b` `` → `a_b`).
   Prose keeps this departure from CommonMark so text escaped by
   `Prose::escape_text` never shows a stray backslash.
2. Line endings become spaces (CommonMark §6.1).
3. When the content both begins and ends with a space and is not all
   spaces, one space is stripped from each side (CommonMark §6.1). This is
   what lets `` `` `a` `` `` hold a backtick at its edge, and it keeps
   Markdown round trips stable.

The content stays opaque: no emphasis, link, or tag syntax inside a span is
interpreted.

**Mechanism.** Follow the fenced-code-block pattern already in
`markdown.rs`: lift each span into a lifted-content table and leave a
sentinel placeholder in the text, which `tokens::parse_render_nodes`
resolves straight into an `InlineCode` node. The placeholder must survive
the link, bold, and italics phases, including inside a link description, so
`` [`x`](u) `` arrives as `<a href="u">PLACEHOLDER</a>` and the recursive
parse of the link's inner text produces the `InlineCode` child. A
placeholder is never re-expanded into string markup.

### Per-target output

For `` See [`inline-block`](https://x.io/a) here ``:

| Target | Output |
|---|---|
| Markdown | `` See [`inline-block`](https://x.io/a) here `` (unchanged) |
| Terminal, OSC 8 supported | `See ` + `ESC]8;;https://x.io/a ESC\` + inline-code styling + `inline-block` + reset + `ESC]8;;ESC\` + ` here` |
| Terminal, OSC 8 unsupported | `` See [<styled inline-block>](https://x.io/a) here `` |
| Browser | `See <a href="https://x.io/a"><code>inline-block</code></a> here` |

"Inline-code styling" is whatever the shared terminal renderer applies to
`InlineCode`: the prose theme background and color when the render context
carries them, otherwise dim. Prose's standalone `render`/`render_optimistic`
path supplies no theme, so it renders dim. The terminal output drops the
backticks; see Open Question 1 for terminals that cannot style.

The browser shape is the existing `<code>` element. No class is added;
`<code>` is the standard element and the convention every common Markdown
renderer (CommonMark, GFM, markdown-it, pulldown-cmark, Goldmark, Pandoc)
follows.

### The inside-out rewrite is removed

`` `[desc](ref)` `` is an ordinary, opaque code span again. On every target
it shows the literal text `[desc](ref)` as inline code, and Markdown output
is `` `[desc](ref)` ``. `` [`desc`](ref) `` is the one way to write a link
whose text is code.

Remove `whole_span_link` and its branch in `escape_code_spans`, the
`code_span_wrapping_a_whole_link_becomes_a_link_with_code_text` and
`code_span_wrapping_a_link_emits_osc8_with_code_text` tests, and the
corresponding rows and paragraph in `docs/components/prose.md` and the
`markdown.rs` module docs. Restore the darkmatter snapshot
`l1__error_snapshots__link__unrecognized_format.snap` to show the literal
`[text](href)`; after this change it renders without backticks, as inline
code.

The template call sites that relied on the rewrite are migrated per Open
Question 2.

### Markdown output picks a safe backtick fence

`renderable`'s Markdown renderer emits `` `{value}` `` for `InlineCode`,
which breaks when `value` contains a backtick. It must follow CommonMark:

- fence with a backtick run one longer than the longest run inside `value`
  (one backtick when there is none);
- pad with one space on each side when `value` begins or ends with a
  backtick, or begins and ends with a space and is not all spaces.

| `value` | Markdown |
|---|---|
| `plain` | `` `plain` `` |
| ``a`b`` | ```` ``a`b`` ```` |
| `` `a` `` | ```` `` `a` `` ```` |

The existing table-cell pipe escaping is kept. This change affects every
render-tree producer, darkmatter included, and only alters output that is
broken today.

## Scope

**In scope**

- `biscuit-terminal` Prose: `markdown.rs` (lifting, space handling, removal
  of the inside-out rule), `tokens.rs` (placeholder resolution), `tree.rs`
  module docs, `parity.rs` expectations, unit tests.
- `biscuit-terminal/docs/components/prose.md`: code spans are inline code on
  every target; the inside-out paragraph is removed.
- `renderable` Markdown renderer: backtick fence selection.
- Snapshot updates in every package whose output passes Prose code spans
  through. Snapshots that strip ANSI will lose the backticks around code;
  that is the expected change, not a regression. At the time of writing,
  roughly 190 files call `Prose::new`/`Prose::from` across 14 package areas.
- The decided answers to the Open Questions below.

**Out of scope**

- Changing the browser element, adding a class, or adding CSS for `<code>`.
- Giving Prose's standalone render path a theme background.
- Word-wrap behavior of inline code in the terminal.
- Making the backtick character escapable in Prose (`` \` ``). Today
  `Prose::escape_text` does not escape backticks, so interpolated text that
  contains a pair of backticks becomes inline code. That was already true of
  opacity; this change only alters how the span looks. Revisit separately if
  it bites.

## Decisions

- **Every code span projects to `InlineCode`**, not only code inside links,
  so Prose and darkmatter agree on what a code span is. (Ken, 2026-10-02)
- **The inside-out rewrite from `0e02bb613` is reverted**; CommonMark's
  `` [`desc`](ref) `` is the only link-with-code-text form, and Markdown
  output is never rewritten. (Ken, 2026-10-02)
- **Browser output stays `<code>`**, the shared renderer's existing shape,
  with no class. (Ken, 2026-10-02)
- **Prose backslash escapes still apply inside a span.** Keeps
  `Prose::escape_text` output clean; unchanged from today.

## Open Questions

1. **Terminals that cannot style.** On a `ColorDepth::None` terminal (piped
   output, `NO_COLOR`), the shared terminal renderer emits no SGR for
   `InlineCode`, so the code is plain text with nothing marking it. Error
   messages such as "an HTML `<a>` tag" would lose their meaning.
   **Recommendation:** when the inline-code appearance produces no escape
   sequence, the terminal renderer emits the value wrapped in a backtick
   fence (the same fence rule as Markdown). This is in the shared renderer,
   so darkmatter's terminal output gains it too. Rejected alternative: leave
   it unmarked.
2. **Replacement for `` `{{ link(x) }}` `` in templates.** Without the
   rewrite these render as the literal `[name](path)` in code. `link()`
   produces the whole `[desc](dest)`, so the author cannot put backticks
   around only the description. **Recommendation:** add a darkmatter
   expression function `code_link(file)` / `code_link(target, desc)` that
   matches `link()` and emits `` [`desc`](dest) `` with a safe backtick fence,
   and migrate the four call sites listed under Problem. Alternative: an
   optional argument on `link()`.

## Acceptance Criteria

1. A Prose L1 test asserts the render tree for each input in the projection
   example above, including a span inside a style tag and inside a link.
2. A Prose L1 test asserts, for `` See [`inline-block`](https://x.io/a) here ``:
   - `render_markdown()` returns the input unchanged;
   - terminal output with OSC 8 support contains the OSC 8 open for
     `https://x.io/a`, then the dim SGR, `inline-block`, a reset, and the
     OSC 8 close, and contains no backtick;
   - `render_html_fragment()` contains
     `<a href="https://x.io/a"><code>inline-block</code></a>`.
3. `` `[desc](ref)` `` renders as inline code with value `[desc](ref)` on all
   three targets, and its Markdown output is `` `[desc](ref)` ``.
4. Space handling: `` `` `a` `` `` produces value `` `a` ``; `` ` a ` ``
   produces `a`; `` `  ` `` produces two spaces.
5. `renderable` Markdown renderer tests cover the fence table above, inside
   and outside a table cell.
6. The darkmatter `unrecognized_format` snapshot shows `[text](href)`
   literally.
7. Open Question 1 and 2 behavior, as ruled, has tests.
8. `just test` and `just lint` pass in `biscuit-terminal`, `renderable`, and
   `darkmatter`, and `just test` passes in every other package area whose
   snapshots moved.

## Definition of Done

- All acceptance criteria are met.
- `docs/components/prose.md`, the `markdown.rs` and `tree.rs` module docs,
  and any `renderable` docs that describe inline-code Markdown output match
  the new behavior.
- The `biscuit-terminal` skill's styling topic is updated if it describes
  code spans.
- Every moved snapshot was reviewed for an unexpected change, not accepted
  in bulk.
