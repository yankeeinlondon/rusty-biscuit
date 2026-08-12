# Layouts and Style

Darkmatter can render the same Markdown document to a terminal or to HTML. The
`style:` frontmatter block lets you give that renderer layout and presentation
hints without changing the Markdown body.

Use this feature when you want a document to carry its own rendering defaults:
page margins, content width, table alignment, list indentation, colors,
horizontal-rule style, link styling, HTML metadata, or a page stylesheet.

## A First Styled Document

Put a `style:` object in the document frontmatter:

```yaml
---
title: Release Notes
style:
    page:
        left-margin: 2ch
        right-margin: 2ch
        top-margin: 1
        max-width: 96
        background: subtle
        color: slate-100
        bg-color: slate-950
        code:
            theme: dracula
    table:
        alignment: center
        max-width: 80%
    block-quote:
        max-width: 72ch
        color: amber-100
        bg-color: amber-900/40
    ul:
        left-margin: 2ch
        max-width: 72
    hyperlinks:
        color: cyan-300
        local-style:
            color: blue-300
    hr:
        kind: waves
        weight: medium
        alignment: center
        max-width: 60ch
---

# Release Notes

> Important changes are called out in block quotes.

| Area | Status |
| --- | --- |
| CLI | Stable |
| HTML | Stable |

- Local links use the local-link color.
- Remote links use the global link color.

---
```

Render it normally:

```sh
md release-notes.md
md release-notes.md --output html --show
```

The same frontmatter is parsed for both targets. Target-specific fields are
ignored where they do not apply; for example, HTML metadata has no terminal
effect.

## How Style Is Applied

Darkmatter folds Markdown into a render tree, attaches the relevant style
policy to each node, then renders that tree for the requested target. You do not
need to use render-tree APIs directly. As a user, the important model is:

1. `style.page` describes the page frame around the document.
2. Component buckets such as `table`, `block-quote`, `ul`, `images`, and
   `hyperlinks` describe matching Markdown elements.
3. A component bucket overrides the page defaults for that component.
4. CLI flags override frontmatter field by field.

Use canonical kebab-case keys: `max-width`, `bg-color`, `block-quote`,
`local-style`, `left-margin`. Older snake-case spellings such as `max_width`
and `block_quote` are accepted for compatibility, but `--strict-style` rejects
them as deprecated.

## Page Layout

The `page` bucket controls the outer frame:

```yaml
style:
    page:
        left-margin: 4ch
        right-margin: 4ch
        top-margin: 1
        bottom-margin: 1
        left-padding: 2ch
        right-padding: 2ch
        max-width: 100
        alignment: center
        background: pronounced
```

Horizontal lengths accept:

| Form | Meaning |
| --- | --- |
| `40` | 40 character cells |
| `40ch` | 40 character cells |
| `50%` | half of the available width |

Vertical page fields are row counts, so `top-margin: 1` is valid but
`top-margin: 1ch` is not.

`background` accepts:

| Value | Meaning |
| --- | --- |
| `transparent` | No page fill. This is the default. |
| `subtle` | Low-contrast page fill. |
| `pronounced` | High-contrast page fill with adjusted code-theme contrast. |

`page.bg-color` and `page.background` are separate controls. `bg-color` chooses
the paint color; `background` chooses the fill behavior. If you set `bg-color`
without `background`, Darkmatter still paints the page so the chosen color is
visible.

## Component Layout

Most component buckets share the same small set of properties:

| Property | Values | Applies To |
| --- | --- | --- |
| `width` | length | Explicit render width. Mutually exclusive with `max-width`. |
| `max-width` | length | Upper bound for the component width. |
| `alignment` | `left`, `center`, `right` | Position within available space. |
| `color` | color | Foreground color. |
| `bg-color` | color | Background color. |

These buckets support the common properties:

| Bucket | Markdown Element |
| --- | --- |
| `table` | Tables |
| `block-quote` | Block quotes |
| `images` | Image references and terminal fallback text |
| `hyperlinks` | Link display text |
| `ul` | Unordered lists |
| `ol` | Ordered lists |
| `li` | List items |

`ul` also supports `left-margin`, which is useful for list indentation:

```yaml
style:
    ul:
        left-margin: 4ch
        max-width: 72
    ol:
        alignment: right
    li:
        max-width: 64
```

Set either `width` or `max-width` in a bucket, not both. Darkmatter rejects a
bucket that sets both so terminal and HTML behavior stay consistent.

## Colors

Color fields accept Tailwind palette names, hex colors, and CSS web color
names:

```yaml
style:
    page:
        color: slate-100
        bg-color: slate-950
    table:
        color: "#f8fafc"
        bg-color: navy
    block-quote:
        bg-color: amber-900/40
```

Tailwind-style opacity uses `/0` through `/100`. Opacity is preserved for HTML.
Terminal output uses the nearest supported terminal color and ignores alpha
because terminal color protocols do not have an opacity channel.

Page foreground color inherits through the document unless a component sets its
own `color`. Background color does not inherit the same way; component
backgrounds use their own `bg-color`, while the page background is painted by
the page frame.

## Links and Images

The `hyperlinks` bucket styles link display text. `local-style` lets you style
local links differently from remote links:

```yaml
style:
    hyperlinks:
        color: cyan-400
        bg-color: slate-900
        local-style:
            color: blue-300
            bg-color: transparent
```

Local links include relative paths, absolute paths, `file://` URLs, and anchor
links such as `#installation`. `http` and `https` links are remote.

Images also support `local-style`:

```yaml
style:
    images:
        alignment: center
        max-width: 80%
        local-style:
            color: green-400
            max-width: 60%
```

For HTML, image style lowers to CSS on the image element. For terminal output,
these fields affect fallback text rather than changing terminal image protocol
decoding.

## Horizontal Rules

Use `style.hr` to style every Markdown horizontal rule:

```yaml
style:
    hr:
        kind: line-star
        weight: thick
        alignment: center
        color: slate-400
        max-width: 70ch
```

`kind` accepts:

| Value | Description |
| --- | --- |
| `dashes` | Dashed rule |
| `dots` | Dotted rule |
| `waves` | Wave rule |
| `line-star` | Line with star ornament |
| `line-circle` | Line with circle ornament |
| `inset-line` | Inset line |
| `curtain-rod` | Curtain-rod rule |

`weight` accepts `thin`, `medium`, or `thick`. `alignment` accepts `full`,
`left`, `center`, or `right`.

## HTML Page Fields

The `page.stylesheet` field adds CSS to HTML output:

```yaml
style:
    page:
        stylesheet: ./docs.css
```

Relative local stylesheets resolve from the Markdown file's directory and are
inlined into the generated HTML. `http` and `https` stylesheet URLs are emitted
as external stylesheet links. `file://` stylesheet URLs are rejected; use a
normal local path instead.

The `page.meta` field emits HTML metadata:

```yaml
style:
    page:
        meta:
            description: "Project documentation"
            author: "Ken"
            keywords: ["rust", "markdown", "cli"]
            "og:title": "Project Docs"
            charset: "utf-8"
```

String, number, and boolean values become `content` values. `keywords` may be a
list, and Open Graph or Twitter-style keys become property metadata. These
fields are ignored by terminal rendering.

Set the document's code theme with `page.code.theme`:

```yaml
style:
    page:
        code:
            theme: github
```

The `--code-theme` CLI flag overrides this field.

## CLI Overrides

CLI layout flags win over matching frontmatter fields. The override is
field-level, so this command keeps the document's `top-margin`, `bottom-margin`,
and `max-width`, but replaces the left and right margins:

```sh
md guide.md --mx 6
```

Common CLI overrides include:

| CLI Flag | Frontmatter It Overrides |
| --- | --- |
| `--margin`, `--mx`, `--my`, `--mt`, `--mb`, `--ml`, `--mr` | `style.page.*-margin` |
| `--padding`, `--px`, `--py`, `--pt`, `--pb`, `--pl`, `--pr` | `style.page.*-padding` |
| `--max-width` | `style.page.max-width` |
| `--page-bg` | `style.page.background` |
| `--alignment` | component alignment defaults |
| `--align-tables`, `--align-images`, `--align-block-quotes` | matching component alignment |
| `--align-lists`, `--align-ul`, `--align-ol`, `--align-li` | list alignment |
| `--fill`, `--fill-tables`, `--fill-images`, `--fill-block-quotes` | matching component fill |
| `--fill-lists`, `--fill-ul`, `--fill-ol`, `--fill-li` | list fill |
| `--code-theme` | `style.page.code.theme` |

Fill flags use this grammar:

```text
full
pad=4
indent=4
max=80
explicit=50%
```

Use frontmatter for defaults that belong to the document. Use CLI flags for a
one-off rendering choice.

## Validation and Troubleshooting

Use `--strict-style` when you want style frontmatter to fail fast on typos or
deprecated keys:

```sh
md guide.md --strict-style
```

Strict mode rejects:

- Unknown keys such as `style.page.lft-margin`.
- Deprecated snake-case aliases such as `max_width`.
- Deprecated horizontal-rule inline aliases.

Invalid values fail even without strict mode. Common examples:

| Invalid Value | Why It Fails |
| --- | --- |
| `left-margin: -2` | Negative lengths are not allowed. |
| `max-width: 120%` | Percent lengths must be between `0%` and `100%`. |
| `top-margin: 2ch` | Vertical margins are row counts, not horizontal lengths. |
| `width` and `max-width` in the same bucket | The bucket can have only one width strategy. |
| `color: bluish` | Color must be Tailwind, hex, or a web color name. |

When a comment or example disagrees with rendered behavior, trust the rendered
behavior and update the document. The style system is intentionally narrow:
small, typed frontmatter keys lower directly onto the render tree, and target
renderers handle the terminal or browser details.
