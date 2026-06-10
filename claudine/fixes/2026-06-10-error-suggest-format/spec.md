When an error like this is rendered:

```sh
 CompositionError: unsupported interactive schema
┃ Required property `spec` in /Users/ken/.claudine/worktrees/rusty-biscuit/icon/prompts/review.md has shape (unknown), which
┃ cannot be collected interactively.
Pass the value with key=value or --set, or provide it in the prompt's frontmatter.
```

The last line of this error -- I believe -- is rendered as a separate part of the error via ErrorBlock (or similarly named struct). The formatting is done in a way that is not as nice as it should be.

- the last line should be included as part of the block quote (so that the red vertical line is to it's left)
- it needs a blank line above it to separate it from the rest of the error message
- the contents of this section should be italicized

One last formatting improvement needed is:

- add a blank line at the very start of the error message inside the block quote (this gives it separation from the title line of the error)

## Scope

- **All severities:** This applies to **all** `StatusBlock` severities — Error, Warning, Info, Success, ToolUse, Subagent, and any future severity. It is not limited to `Error`.
- **All render targets:** The formatting change is implemented in the **shared render-tree projection** (`to_render_node()`), so Terminal, Markdown, and Browser outputs all render consistently. No terminal-only hacks.
- **Universal leading blank line:** Every `StatusBlock` body receives a leading blank paragraph inside the block quote, regardless of whether a hint is present.

This should be done in a way so that both Darkmatter and Claudine errors benefit from this improved styling.

## Implementation Details

### Render Tree Structure

The `StatusBlock` render tree changes from a single `Paragraph` (or flat text) inside a `BlockQuote` to **multiple child paragraphs** inside the same `BlockQuote`:

```
BlockQuote
├── Paragraph ""              (leading blank line — empty paragraph node)
├── Paragraph "Required property `spec` ..."  (main body)
├── Paragraph ""              (separator blank line, only when hint present)
└── Paragraph "Pass the value ..."  (hint — italicized)
```

### Blank Lines

Blank lines are represented as **empty `Paragraph` nodes** (zero-length text content). They are not special sentinel nodes; they are normal paragraph children with no text.

### Italics

The hint paragraph is styled with **italics** using a paragraph-level `Style::Italic` attribute on the `RenderNode`, or equivalent markup (e.g., `<i>` in HTML, `_text_` in Markdown). The main body paragraphs remain unstyled.

## Acceptance Criteria

- [ ] `StatusBlock` of every severity (Error, Warning, Info, Success, ToolUse, Subagent) renders with a leading blank paragraph inside its block quote.
- [ ] A `StatusBlock` with a non-empty hint renders the hint in italics, separated from the body by a blank paragraph.
- [ ] A `StatusBlock` with an empty hint omits both the separator blank line and the hint paragraph.
- [ ] Terminal output shows the same paragraph spacing and italic hint styling as Markdown and Browser outputs.
- [ ] No terminal-specific formatting overrides are introduced; all targets consume the shared `to_render_node()` projection.
