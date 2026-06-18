---
status: ready for planning and implementation
reviewed: true
---

# StatusBlock Hint Formatting

## Purpose

Improve `StatusBlock` diagnostics so actionable hints render as part of the same block-quoted error surface as the main diagnostic body.

The motivating example currently reads as though the final guidance line is detached from the error body:

```sh
 CompositionError: unsupported interactive schema
┃ Required property `spec` in /Users/ken/.claudine/worktrees/rusty-biscuit/icon/prompts/review.md has shape (unknown), which
┃ cannot be collected interactively.
Pass the value with key=value or --set, or provide it in the prompt's frontmatter.
```

The desired visual shape is:

```sh
 CompositionError: unsupported interactive schema
┃
┃ Required property `spec` in /Users/ken/.claudine/worktrees/rusty-biscuit/icon/prompts/review.md has shape (unknown), which
┃ cannot be collected interactively.
┃
┃ Pass the value with key=value or --set, or provide it in the prompt's frontmatter.
```

The hint line is italicized when the render target supports italics.

## Reader's Note

This specification intentionally changes the original draft's target from "Claudine errors" to the shared `biscuit-terminal::components::status_block::StatusBlock` component. Claudine and Darkmatter already render many user-facing diagnostics through `StatusBlock`, so fixing the shared component gives both package areas the new styling without duplicating formatter logic.

The shared render-tree projection remains the normative path for default `StatusBlock` output. Existing terminal compatibility behavior must be preserved: `StatusBlock::border()` with a custom border prefix and headers that require Prose rendering may continue through the sanctioned bespoke terminal renderer, but that fallback must mirror the same body/hint spacing contract.

## Scope

This specification defines:

- the structural layout of `StatusBlock` bodies and hints;
- behavior for every current `StatusState`, including `NotStarted`, `Active`, `Success`, `Error`, deprecated `Failure`, `Warning`, `Info`, `ToolUse`, and `Subagent`;
- behavior for future severities that reuse `StatusBlock`;
- the expected behavior across Terminal, Markdown, and Browser render targets;
- the compatibility requirements for the existing bespoke terminal fallback; and
- regression coverage required in `biscuit-terminal`, with enough integration confidence for Claudine and Darkmatter consumers.

## Out of Scope

- Changing individual Claudine or Darkmatter error wording.
- Migrating errors that do not already use `StatusBlock`.
- Removing the deprecated `StatusState::Failure` alias.
- Removing or redesigning the `StatusBlock::border()` custom-prefix compatibility surface.
- Adding arbitrary custom-border support to the render tree.
- Changing `BlockQuote` behavior outside the `StatusBlock` projection/fallback.

## Normative Behavior

### Body Structure

When a `StatusBlock` has body content, the rendered body block quote must start with a leading blank paragraph inside the block quote.

A `StatusBlock` body consists of the configured body prose items in order. Existing body-item semantics are preserved: multiple body items remain separated by a blank paragraph inside one continuous block quote. A body item that contains its own line breaks may still render as a multi-line paragraph according to the existing Prose and target renderer behavior.

### Hint Structure

When a `StatusBlock` has a non-blank hint and body content:

1. The hint is rendered inside the same block quote as the body.
2. A blank paragraph separates the body content from the hint.
3. The hint paragraph is italicized.
4. The hint text remains actionable prose, not a separate `Status` line.

A blank hint means `None`, an empty string, or a string whose Unicode whitespace-trimmed form is empty. Blank hints are omitted. When the hint is omitted, the body still receives the leading blank paragraph, but there is no hint separator.

When a `StatusBlock` has a non-blank hint and no body, existing behavior is preserved: the hint renders as a standalone paragraph outside a block quote. This specification only moves hints inside a block quote when there is body content to attach them to.

### Render Tree Contract

For default-border `StatusBlock` output, the canonical render tree is the source of truth. The public `TreeRenderable::render_tree()` implementation must delegate to the same private projection helper used by Terminal, Markdown, and Browser adapters.

The default body projection is:

```text
Root
├── Paragraph status-block__header?     (optional)
└── BlockQuote status-block__body       (when body is present)
    ├── Paragraph ""                    (leading blank paragraph)
    ├── Paragraph body item 1
    ├── Paragraph ""                    (between body items, when needed)
    ├── Paragraph body item N
    ├── Paragraph ""                    (hint separator, only when non-blank hint is present)
    └── Paragraph status-block__hint    (italic hint, only when non-blank hint is present)
```

If an implementation keeps body items flattened into one text node for compatibility with existing wrapping behavior, it must still preserve the externally visible contract: leading blank block-quoted line, blank separation between body and hint, italic hint, and no hint outside the block quote when a body exists.

Blank lines are represented structurally as empty paragraph nodes in the render tree. They are not sentinel strings, magic glyphs, or terminal-only newline hacks.

The hint's italic style must use the existing render-tree styling model, specifically `renderable::style::Style` with `TextEmphasis { italic: true, .. }` or the nearest existing equivalent. The spec does not require a nonexistent `Style::Italic` enum variant.

### Bespoke Terminal Fallback

The bespoke terminal renderer remains valid for cases that the render tree cannot faithfully express today:

- arbitrary custom border prefixes configured with `StatusBlock::border()`;
- headers where Prose rendering would otherwise lose terminal-only styling or OSC 8 links.

That fallback must produce the same body contract as the render-tree path:

- leading blank line inside the block quote;
- all body content inside the block quote;
- non-blank hint inside the block quote;
- blank separator before the hint;
- italic hint styling where supported; and
- blank hints omitted.

The fallback may continue to differ from Browser and Markdown for terminal-only features, such as custom border prefix strings and terminal Prose header styling. Those differences are intended compatibility behavior, not regressions.

## Implementation Notes

The primary implementation belongs in `biscuit-terminal/lib/src/components/status_block.rs`.

Use `StatusBlock::render_tree()` in tests rather than calling the private `to_render_node()` helper. The helper can remain private as long as all public render adapters share it.

The projection should continue to assign existing structural classes:

- `status-block`
- `status-block--{severity}`
- `status-block__header`
- `status-block__body`
- `status-block__hint`

Preserve the existing default-border mapping to a typed thick left border. Do not encode custom terminal border prefix strings into Markdown or Browser output.

## Testing Requirements

Add or update focused `biscuit-terminal` tests for:

- every current `StatusState` renders a body with the leading blank line inside the block quote;
- `StatusState::Failure` keeps the same behavior as `Error` while it remains supported;
- body plus non-blank hint renders the hint inside the block quote for Terminal, Markdown, and Browser output;
- body plus blank hint omits the separator and hint;
- hint-only output remains outside the block quote;
- multiple body items keep their blank-line separation inside a continuous block quote;
- the default-border terminal path uses the shared render tree;
- the custom-border terminal path mirrors the new body/hint layout while still honoring the custom prefix;
- Markdown output does not leak a custom terminal border prefix; and
- Browser output preserves the `status-block__hint` class and italic styling for a hint inside a body block quote.

Add at least one Claudine-facing regression test or snapshot that exercises a composition error with a body and hint. The test should assert the user-visible contract rather than duplicate `biscuit-terminal` internals.

Darkmatter does not need a new package-specific formatter. It benefits through its existing `StatusBlock` usage; add a targeted Darkmatter regression only if an existing Darkmatter diagnostic uses a custom or unusual `StatusBlock` configuration that could bypass the shared behavior.

## Acceptance Criteria

- [ ] `StatusBlock` with body content renders a leading blank paragraph inside the body block quote.
- [ ] `StatusBlock` with body content and a non-blank hint renders the hint inside the same block quote.
- [ ] The hint is separated from the body by a blank paragraph.
- [ ] The hint is italicized in Terminal, MarkdownPlus/Browser where supported, and represented as portable Markdown emphasis when Markdown rendering supports it.
- [ ] Blank hints are omitted along with their separator.
- [ ] Hint-only blocks preserve existing standalone hint behavior.
- [ ] Default-border Terminal, Markdown, and Browser outputs consume the shared render-tree projection.
- [ ] The bespoke terminal fallback for custom borders and Prose-rich headers mirrors the same body/hint layout.
- [ ] No terminal-only formatting hacks are introduced into the shared render-tree path.
- [ ] Claudine and Darkmatter diagnostics that already use `StatusBlock` pick up the improved styling without package-specific duplication.
