# Disclosure Blocks

Darkmatter provides render-time disclosure blocks using the `::disclosure` / `::details` / `::end-disclosure` directive triple — a portable equivalent of the HTML `<details>`/`<summary>` pair. The summary region is the always-visible label; the body region is the disclosed content. Each target lowers the block its own way (see [Render Targets](#render-targets)): in the browser the body is collapsed behind a clickable summary, while in the terminal both regions are always shown with the body styled as a dim, italic block quote.

## Syntax

A disclosure block has three required directives:

```md
::disclosure Optional summary text
::details
Body content — any block-level Markdown, including nested disclosure blocks.
::end-disclosure
```

- `::disclosure` opens the block. The remainder of the line is the summary. It must contain only phrasing content — no paragraph breaks, hard line breaks, or block-level elements.
- `::details` separates the summary from the body.
- `::end-disclosure` closes the block.

Each directive must appear at the start of a line and be followed by ASCII whitespace or end-of-line. Near-miss text such as `::disclosurex` is treated as literal prose.

## Render Targets

Disclosure blocks are recognized during the render-tree fold and lowered differently for each target:

| Target | Output |
|---|---|
| `markdown` | DSL emitted verbatim: `::disclosure`, `::details`, `::end-disclosure`. |
| `markdown-plus` | Summary and body rendered to Markdown, then wrapped in `<details><summary>…</summary>…</details>`. |
| `html` / `browser` | Native HTML `<details>` / `<summary>` elements; no JavaScript. |
| `terminal` | Summary rendered normally; body rendered as a block quote with dim and italic text. |
| `json` / `ast` | Native `NodeKind::Disclosure` node in the render tree. |

Use `--output markdown-plus` to render disclosures as inline HTML while keeping the rest of the document as Markdown. Use `--output html` for full HTML output.

## Transclusion Integration

`::file` and `::code` transclusion can wrap imported content in a disclosure block with the `disclosure` option:

```md
::file ./long-section.md disclosure="License Agreement"
::code ./demo.rs disclosure=true
```

- `disclosure="Summary text"` wraps the transcluded content in a `::disclosure` block with the given summary.
- `disclosure=true` (or an empty summary) uses the default summary `"Details"`.

The transclusion stage emits the DSL triple; no inline HTML is produced at compose time. See [Block Transclusion](../transclusion/block-transclusion.md) and [Code Transclusion](../transclusion/code-transclusion.md).

## Inline Style Parameters

The `::disclosure` opener accepts whitespace-separated `key=value` style tokens before the summary text:

```md
::disclosure max-width=60ch color=red-500 License Agreement
::details
Keep your hands off.
::end-disclosure
```

Recognized keys mirror the `style.disclosure.*` bucket:

| Key | Value |
|---|---|
| `width` | `Nch` or `N%` |
| `max-width` | `Nch` or `N%` (snake-case `max_width` also accepted) |
| `alignment` | `left`, `center`, or `right` |
| `color` | Tailwind, hex, or web named color |
| `bg-color` | Tailwind, hex, or web named color (snake-case `bg_color` also accepted) |

Tokens that are not recognized style pairs become part of the summary. An invalid value (for example, `max-width=not-a-length`) is treated as summary text rather than raising an error.

## Style Frontmatter

The `style.disclosure.*` bucket configures disclosure blocks document-wide:

```yaml
---
style:
    disclosure:
        max-width: 60ch
        alignment: center
        color: red-500
---
```

Supported keys are the same five `CommonStyle` mutations used by other component buckets:

| Key | Value |
|---|---|
| `width` | `Nch` or `N%` |
| `max-width` | `Nch` or `N%` |
| `alignment` | `left`, `center`, or `right` |
| `color` | Tailwind, hex, or web named color |
| `bg-color` | Tailwind, hex, or web named color |

Multi-word keys use kebab-case (`max-width`, `bg-color`). Snake-case aliases (`max_width`, `bg_color`) parse but emit a `Deprecated` warning; `--strict-style` rejects them.

### Precedence

Style values resolve from most specific to least specific:

1. Inline `key=value` tokens on the `::disclosure` opener.
2. `style.disclosure.*` frontmatter.
3. Page-level `style.page.alignment` broadcast and any future disclosure CLI flags.
4. Built-in default.

### Constraints

- `width` and `max-width` are mutually exclusive within the same bucket. Setting both raises a style error before rendering.
- `Length::Css` values such as `10px` are rejected for terminal targets.

## Errors

Malformed disclosures raise `MarkdownError::MalformedDisclosure { reason, range }`:

- Missing `::details` or `::end-disclosure`.
- `::details` without a matching closer.
- Empty summary region.
- Hard line break in the summary region.
- Any block-level element in the summary region.

The summary region is parsed as phrasing content only; a paragraph break or list inside the summary is treated as a structural error.
