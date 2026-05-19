# Approved Render Tree Functionality

## RT-COMPOSE-001: Explicit no-separator sequence rendering

**APPROVED**

This feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: Compose's public contract is ordered concatenation with no automatic
separators. The current `NodeKind::Root` rendering contract is ordered block
rendering with blank-line separators in Terminal and Markdown. Treating Compose
as a plain root would change observable output for basic inputs like `["foo",
"bar"]`. The render tree needs an explicit sequence/fragment join policy so
components can preserve target-agnostic structural children without inheriting
document-block spacing.

Required behavior:

- Add a typed render-tree representation for sequence joining. This may be a
  dedicated node kind or a typed `NodeAttrs` hint on `Root`; prefer the smallest
  change that keeps exhaustive renderer handling explicit.
- Support at least `SequenceJoin::None`, meaning render children in order with
  no renderer-inserted separator.
- Terminal, Markdown, MarkdownPlus, and Browser renderers must honor the same
  child order and no-separator semantics.
- Normal document `Root` behavior must remain unchanged unless the sequence
  marker is present.
- Validation must reject sequence semantics in structurally invalid positions
  if the chosen representation can appear outside a block/container context.
- Tests must cover root/document behavior unchanged, Compose-style no-separator
  behavior, nested sequences, and mixed inline/block children.

## RT-FILESYSTEM-001: Typed list marker policy for custom list presentation

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: FileSystem has semantic nested-list structure, but its terminal contract
uses connector geometry (`├──`, `└──`, `│`) rather than normal unordered-list
bullets. Embedding those connectors into text nodes would make the canonical
tree target-specific and would prevent Browser and Markdown renderers from
producing native nested-list output. A typed list marker policy lets the tree
stay semantic while giving renderers an explicit hook for target-specific marker
presentation.

Required behavior:

- Add a typed list marker policy for `NodeKind::List`; prefer a typed helper on
  `NodeAttrs` over ad hoc JSON string reads at call sites.
- Support at least `Default`, `None`, and `TreeConnectors`.
- Normal ordered and unordered list rendering must remain unchanged when no
  policy is present or when the policy is `Default`.
- Validation must reject marker policies placed on non-list nodes.
- Terminal rendering for `TreeConnectors` must infer depth, last-child state,
  and ancestor continuation lines from nested `List` / `ListItem` structure.
- Browser and Markdown renderers must preserve valid output. They may degrade
  `TreeConnectors` to native nested lists or no-marker presentation according
  to strictness and dialect, but they must not emit terminal box-drawing
  connector text by default.
- Tests must cover default behavior unchanged, no-marker lists, tree-connector
  lists, single-child lists, nested continuation lines, and strict/warn/lossy
  behavior for targets that cannot faithfully represent a marker policy.

## RT-PROGRESS-001: Browser rendering for progress hints

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: the tree already carries a target-agnostic `ProgressHints` payload, and
the terminal renderer already treats that payload as native widget semantics.
Adding browser handling to the tree renderer keeps the tree as the single
source of truth and avoids a second bespoke `Progress` browser implementation.

Required behavior:

- In the browser renderer, handle `ProgressHints` in the
  `NodeKind::Paragraph` branch before normal paragraph rendering.
- Emit semantic HTML with `role="progressbar"`, `aria-valuemin="0"`,
  `aria-valuemax="100"`, `aria-valuenow`, and an accessible label derived from
  the component label when present.
- Use stable classes such as `progress`, `progress-label`, `progress-track`,
  `progress-filled`, and `progress-percentage`.
- Apply node `Layout` to the outer progress element via the existing browser
  layout lowering.
- Clamp `hints.value` to `0.0..=1.0`; render filled width as
  `round(value * 100)%`; render track width as `bar_width` in `ch`.
- Lower `filled_color` and `empty_color` to CSS `background-color` values on
  the filled segment and track. Lower `bracket_color` only if the browser
  output chooses to render bracket affordances; otherwise preserve it as a
  typed hint and do not invent terminal glyph output.
- Do not render terminal fill/empty glyph repetition by default. Browser output
  is a CSS progress bar, not terminal character art.
- Preserve non-default glyph and bracket values in `data-fill-char`,
  `data-empty-char`, `data-left-bracket`, and `data-right-bracket` attributes
  so the information is not silently discarded from the HTML surface.
- HTML-escape all label and percentage text.
- Normal paragraph rendering must remain unchanged when no `ProgressHints` are
  present.

## RT-PROGRESS-002: MarkdownPlus rendering for progress hints

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: MarkdownPlus is the rich Markdown dialect in this codebase. Rendering
`ProgressHints` as inline HTML in MarkdownPlus gives components one canonical
projection while allowing richer outputs to preserve the progress widget
visually. Portable Markdown remains clean, valid, and semantic by using the
paragraph fallback text.

Required behavior:

- In the Markdown renderer, keep `MarkdownDialect::Markdown` behavior as plain
  text from the paragraph children.
- In `MarkdownDialect::MarkdownPlus`, handle `ProgressHints` on paragraphs by
  emitting the same semantic progress HTML shape used by the browser renderer,
  serialized as inline/block HTML acceptable in MarkdownPlus.
- Apply color slots as inline CSS when present.
- Do not apply `Layout` in either Markdown dialect; this matches the documented
  Markdown layout contract in `layout-and-style.md`.
- Preserve non-default glyph and bracket values in `data-*` attributes for the
  same reason as browser rendering.
- Plain paragraphs without `ProgressHints` must remain unchanged.

## RT-TABLE-001: Typed table title/caption hint

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: Table has an existing public title feature, and the current render-tree
shape does not carry it. Adding a `NodeKind::Table` enum field would be too
large for this component-level metadata. A typed table title/caption hint on
`NodeAttrs` preserves the information without changing the canonical node
variant or serialized tree shape more than necessary.

Required behavior:

- Add typed `set_table_title` / `table_title` helpers on `NodeAttrs`, using a
  namespaced table hint key consistent with existing table hint helpers.
- Validation must reject the title hint on non-`Table` nodes.
- Empty or whitespace-only titles must be ignored by renderers.
- Browser rendering must emit the title as a `<caption>` inside the `<table>`,
  before `<thead>` / `<tbody>`.
- Terminal rendering must emit the title above the top border, preserving the
  existing table title behavior as closely as possible for normal and
  cursor-alignment rendering.
- Markdown and MarkdownPlus rendering must emit the title as escaped plain text
  before the table with a blank line separator. Do not render it as a heading,
  because a table caption should not change document outline semantics.
- Tests must cover accessor round-trip, validation on the wrong node kind,
  Browser caption output, Terminal title output, and Markdown title output.

## RT-TABLE-002: Markdown-safe table cell serialization

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: The current Markdown tree renderer renders text raw and joins table cells
with pipe delimiters. That corrupts GFM tables when cell content contains `|`
and breaks table structure when cell content contains literal newlines. Table
supports arbitrary text and multi-line cells, so Markdown table-cell rendering
needs a table-cell-specific escaping mode in the render-tree implementation.

Required behavior:

- Apply table-cell escaping only while rendering descendants of
  `NodeKind::TableCell`; normal text outside tables must remain unchanged.
- Escape literal pipe characters as `\|`.
- Normalize `SoftBreak` to a single space inside table cells.
- Normalize literal newlines in text nodes and `HardBreak` nodes to `<br>`
  inside table cells for both Markdown and MarkdownPlus.
- Preserve existing inline emphasis, strong, delete, link, and inline-code
  rendering where possible, but make the serialized result safe inside a
  pipe-delimited table cell.
- Add tests for literal pipes, multiline text, explicit soft breaks, explicit
  hard breaks, inline code containing `|`, and ordinary non-table text
  unchanged.

## RT-TEXTBLOCK-001: Browser lowering for `Style` text appearance and colors

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: `TextBlock` is exactly the simple styled-paragraph case that `Style` was
created to support. The terminal renderer already consumes `Style`; leaving
browser output unstyled would make `TextBlock` browser support structurally
present but visually incomplete. Browser `Style` lowering is already called
out as designed but unwired in `layout-and-style.md`, so this closes a known
render-tree gap rather than adding a component-specific special case.

Required behavior:

- Lower `Style.color` to CSS `color`.
- Lower `Style.background` to CSS `background-color`.
- Lower `TextEmphasis.bold`, `italic`, and `strikethrough` to semantic
  wrappers (`<strong>`, `<em>`, `<s>`) or equivalent valid HTML that preserves
  nesting.
- Lower underline variants with `UnderlineStyle::css_declaration()`.
- Lower dim to `opacity: 0.6`.
- Lower blink to `text-decoration: blink`, accepting that browser support is
  limited.
- Apply style to block nodes and inline `Span` nodes without overwriting
  existing layout CSS in the same `style` attribute.
- Continue to ignore `border` and `fill` in the browser until the broader
  Browser Style lowering work defines those box-painting semantics.
- Preserve current plain paragraph output when no `Style` is present.
- Tests must cover foreground, background, bold, italic, strikethrough, each
  underline variant, dim, blink, style plus layout on the same node, and
  no-style output unchanged.

## RT-TODO-001: Typed task-state hints for task-list items

**APPROVED**

this feature request has been approved and WILL be included as part of the
render-tree implementation BEFORE you are asked to implement this solution. Always
refer to the @renderable/docs/tree-rendering.md and
@renderable/docs/layout-and-style.md documents as the definitive guide.

Why: Todo is semantically a task-list item, but its public terminal behavior has
five states while GFM task-list syntax only has checked and unchecked. A typed
task-state hint preserves the semantic `List` / `ListItem` tree shape while
letting terminal rendering preserve state-specific glyphs without parsing CSS
classes or embedding terminal glyphs into text nodes.

Required behavior:

- Add a `TaskState` enum with `Open`, `InProgress`, `Completed`, `Blocked`, and
  `Cancelled` variants, plus a `TaskHints { state: TaskState }` payload.
- Store the payload under a new `renderable.widget.task` namespace with typed
  `NodeAttrs::set_task_hints` and `NodeAttrs::task_hints` helpers.
- Validation must reject task hints on any node other than `NodeKind::ListItem`.
- The terminal renderer must read task hints from `ListItem` nodes and replace
  the default `[ ]` / `[x]` marker with the Todo-compatible marker for the task
  state, honoring Nerd Font support, color support, and no-color fallbacks.
- The custom marker must apply only to the checkbox marker. Description styling
  remains represented by normal child nodes and `Style`, such as `Delete` and
  strikethrough/dim for cancelled items.
- Browser rendering may continue to use the existing `checked` checkbox output
  and preserve `todo-*` classes for CSS hooks; no browser special case is
  required for the initial feature.
- Markdown and MarkdownPlus must continue to emit portable GFM task-list syntax:
  `Completed` as `- [x]`, and all other task states as `- [ ]`, with cancelled
  strikethrough preserved through the existing `Delete` node rendering.
- Do not add `chrono` or `biscuit-terminal` component types to `renderable` for
  this feature. Todo timestamps remain component metadata until a separate
  cross-target metadata display contract is designed.
- Tests must cover hint accessor round-trip, validation on the wrong node kind,
  terminal output for all five states across Nerd Font/color/no-color branches,
  Markdown degradation for non-completed states, Browser checkbox/class
  preservation, and default task-list rendering unchanged when no task hints are
  present.
