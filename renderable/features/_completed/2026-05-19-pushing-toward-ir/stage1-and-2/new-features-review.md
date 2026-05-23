# New Features Review

Review target: implementation of the approved render-tree functionality in
`approved-render-tree-functionality.md`, with `renderable/docs/tree-rendering.md`
as the status reference.

## Findings

### High: browser `Style` emphasis wraps block elements in invalid HTML

RT-TEXTBLOCK-001 requires browser lowering to use semantic wrappers or
equivalent valid HTML. The implementation applies `wrap_style_emphasis` to every
rendered fragment after `render_kind` returns, including block nodes such as
`Paragraph` (`renderable/src/tree/render/browser.rs:199`). The wrapper then
places the entire block fragment inside `<strong>`, `<em>`, and `<s>`
(`renderable/src/tree/render/browser.rs:972`).

For a styled paragraph this produces shapes like:

```html
<strong><em><s><p>text</p></s></em></strong>
```

That is not valid semantic HTML for the block case. The current test
`style_bold_italic_strikethrough_use_semantic_wrappers` effectively locks in
the invalid shape by asserting the output starts with `<strong><em><s>`.

Suggestion: for block nodes, lower bold/italic/strikethrough to CSS on the
block (`font-weight`, `font-style`, `text-decoration-line`) or wrap the
phrasing children inside the block rather than wrapping the block itself. Keep
semantic wrappers for inline nodes such as `Span`. Replace the current test
with exact assertions for both a styled paragraph and a styled span.

### High: progress HTML emits duplicate `style` attributes on `.progress-filled`

RT-PROGRESS-001 and RT-PROGRESS-002 require filled width and color lowering.
`progress_html` builds `filled_style` as a complete `style="background-color:..."`
attribute, then always emits a second `style="width:{pct}%"` on the same
`progress-filled` span (`renderable/src/tree/render/shared.rs:87`,
`renderable/src/tree/render/shared.rs:110`). When `filled_color` is present the
HTML has duplicate `style` attributes, which is invalid and can cause one of
the declarations to be ignored by the browser/parser.

Suggestion: combine filled declarations into one style string, e.g.
`style="width:50%;background-color:#..."`. Add exact-output or DOM-style tests
for the color+width case in both browser and MarkdownPlus.

### High: browser progress labels are double-escaped and brittle

The browser progress path renders child fragments to HTML strings, concatenates
them, and sends that rendered HTML to `progress_html` as label/fallback text
(`renderable/src/tree/render/browser.rs:440`). `progress_html` then escapes that
string again (`renderable/src/tree/render/shared.rs:76`,
`renderable/src/tree/render/shared.rs:100`). A label such as `A < B 50%` becomes
`A &amp;lt; B` in the accessible label and visible label, not `A &lt; B`.

The label extraction is also coupled to the clamped percentage suffix
(`renderable/src/tree/render/shared.rs:47`). If the fallback text carries the
component's original value while the hint clamps to a different percentage,
the label disappears.

Suggestion: collect plain text from the progress paragraph children for the
browser path instead of rendered HTML, and avoid deriving the label from a
rendered percentage suffix when a component label can be carried explicitly.
At minimum, add browser tests for labels containing `<`, `&`, quotes, and a
clamped value whose fallback text does not end in the clamped percent.

### Medium: Markdown table-cell escaping misses link/image targets and image alt text

RT-TABLE-002 requires table-cell-specific escaping so literal `|` and newlines
cannot corrupt GFM tables. The implementation correctly escapes `Text`,
`SoftBreak`, `HardBreak`, and `InlineCode` while inside a table cell
(`renderable/src/tree/render/markdown.rs:282`,
`renderable/src/tree/render/markdown.rs:291`,
`renderable/src/tree/render/markdown.rs:313`).

Links and images still format raw `url`, `title`, and `alt` values through
`link_target` without a table-cell escaping mode
(`renderable/src/tree/render/markdown.rs:298`,
`renderable/src/tree/render/markdown.rs:306`,
`renderable/src/tree/render/markdown.rs:603`). A pipe in an image alt, link URL,
or link title will still split the table row.

Suggestion: make table-cell escaping apply to every literal Markdown segment
that can contain `|` or newlines, including link/image alt, URL, and title
serialization. Add tests for `[label](a|b)`, `[label](url "a|b")`,
`![a|b](img.png)`, and a newline in URL/title if those values are permitted.

### Medium: browser table cell alignment can overwrite or duplicate existing style CSS

RT-TEXTBLOCK-001 says browser style lowering should apply to block nodes without
overwriting existing layout CSS. `render_table_cell` first applies all node
attributes, including `Style` and `Layout`, then appends a separate
`style="text-align:..."` when the column has alignment
(`renderable/src/tree/render/browser.rs:653`,
`renderable/src/tree/render/browser.rs:656`). A styled or laid-out table cell
with alignment can therefore emit duplicate `style` attributes, or have one
style override the other depending on renderer behavior.

Suggestion: merge table alignment into the existing style declaration the same
way the columns and list marker policy paths do. Add a test for a table cell
with both a foreground/background style and column alignment.

### Medium: malformed typed hints are silently treated as absent/default

The approved features add typed hints, but the accessors generally hide malformed
serialized data:

- `sequence_join()` returns `None` for unrecognized tokens
  (`renderable/src/tree/attrs.rs:1498`), so validation does not reject an
  invalid sequence marker.
- `list_marker_policy()` falls back to `Default` for unrecognized tokens
  (`renderable/src/tree/attrs.rs:1544`), so a malformed explicit marker policy
  silently changes behavior.
- `task_hints()` returns `None` for an invalid state token
  (`renderable/src/tree/attrs.rs:1583`), so an explicit but malformed task hint
  is not validated or reported.
- `columns_hints()` falls back to default width semantics for malformed width
  kind/value combinations (`renderable/src/tree/attrs.rs:1060`).

This weakens the render tree as a typed IR once it is serialized and reloaded.

Suggestion: add validation checks for known hint keys with invalid tokens or
wrong value types, at least for the new approved feature namespaces. Tests
should construct `NodeAttrs::data` directly with bad values and assert
validation errors.

### Low: list marker policy strictness is not symmetrical across targets

Markdown rejects non-default marker policies under `Strict`
(`renderable/src/tree/render/markdown.rs:451`), but browser silently degrades
`None` and `TreeConnectors` to `list-style:none`
(`renderable/src/tree/render/browser.rs:471`). The approved behavior allowed
browser/Markdown degradation according to strictness and dialect, and required
strict/warn/lossy coverage for targets that cannot faithfully represent the
marker policy.

Suggestion: either document browser's `list-style:none` as faithful enough for
`None` and accepted lossy behavior for `TreeConnectors`, or emit diagnostics /
reject under `Strict` for `TreeConnectors`. Add strict/warn/lossy browser tests
so the contract is pinned.

## Test Coverage Gaps

- Browser progress tests use substring assertions and miss duplicate attributes.
  Add exact-output or parsed-DOM assertions for progress with filled color,
  empty color, layout, and escaped labels.
- Browser style tests currently assert the invalid wrapper order for block
  nodes. Add separate exact tests for styled block nodes and styled inline spans.
- Markdown table-cell tests cover direct text, breaks, and inline code, but not
  links/images or title/URL escaping inside table cells.
- Accessor tests cover happy-path round trips, but not malformed serialized
  hint payloads in `NodeAttrs::data`.
- Terminal task-state tests assert marker absence/presence for Nerd Font and
  only one color fallback glyph. Add exact marker tests for all five states in
  the color fallback branch.
- Browser task-state tests only cover the default checked checkbox path. Add a
  test showing `TaskState::Cancelled` and `TaskState::InProgress` preserve the
  expected portable checkbox/classes and do not leak terminal-only glyphs.
- Sequence join tests cover Markdown and Terminal behavior, but browser coverage
  is implicit. Add a browser `Root` with `SequenceJoin::None` and adjacent text
  and block children, especially through `render_browser_document`, to pin the
  document entry point.

## Overall Assessment

The planned features are substantially implemented: typed sequence/list/task/
table/progress/columns hints exist, validation rejects several wrong-node
placements, and all three render targets have focused tests. The biggest
remaining risk is not missing functionality but output fidelity: several tests
assert broad substrings, so invalid HTML and escaping bugs can pass unnoticed.
