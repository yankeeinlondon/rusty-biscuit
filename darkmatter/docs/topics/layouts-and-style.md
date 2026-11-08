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
        margin-left: 2ch
        margin-right: 2ch
        margin-top: 1
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
4. Style given on the command line (`--style` files and `style.KEY=VALUE`
   setters) overrides the frontmatter field by field; see
   [CLI Overrides](#cli-overrides).

`style.markdown` is the one `style:` key that does not affect rendering: it
holds the formatting rules `md clean` and DMLS formatting apply to the
source. See [Markdown Formatting Rules](./markdown-formatting.md).

Use canonical kebab-case keys: `max-width`, `bg-color`, `block-quote`,
`local-style`, `margin-left`. Older snake-case spellings such as `max_width`
and `block_quote` are accepted for compatibility, but `--deny style` rejects
them as deprecated. The page spacing keys follow CSS order
(`page.margin-left`, `page.padding-top`); the old `page.left-margin`-style
names are unknown keys, and their warning names the replacement.

## Page Layout

The `page` bucket controls the outer frame:

```yaml
style:
    page:
        margin-left: 4ch
        margin-right: 4ch
        margin-top: 1
        margin-bottom: 1
        padding-left: 2ch
        padding-right: 2ch
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

Vertical page fields are row counts, so `margin-top: 1` is valid but
`margin-top: 1ch` is not. `max-width` must be positive.

### Shorthands and Top-Level Page Keys

Page margins and padding have CSS-style shorthands:

| Key | Sets | Value |
| --- | --- | --- |
| `margin`, `padding` | all four sides | a row count, used as cells for the left and right sides |
| `mx`, `px` | left and right | a horizontal length |
| `my`, `py` | top and bottom | a row count |
| `ml`, `mr`, `pl`, `pr` | one horizontal side | a horizontal length |
| `mt`, `mb`, `pt`, `pb` | one vertical side | a row count |

Because `margin` and `padding` also set vertical sides, they take only a whole
number: `margin: 2` is valid, `margin: '10%'` is not, and `mx: '10%'` is.

Every page key and shorthand can also be written at the top level of `style`,
where it means the same `page` field. These two blocks are identical:

```yaml
style:
    max-width: 80
    mx: 4
    theme: nord
```

```yaml
style:
    page:
        max-width: 80
        margin-left: 4
        margin-right: 4
        theme: nord
```

Within one `style` value, the most specific spelling wins whatever order you
write the keys in: a single side beats `mx`/`my`, which beats `margin`. So
`{ margin: 2, ml: 6 }` gives a left margin of 6 and 2 on every other side.
Two spellings of the _same_ field, such as `ml` beside `page.margin-left`, or
a top-level `max-width` beside `page.max-width`, are a conflict and fail the
render; keep one.

`alignment` (or `page.alignment`) also sets the alignment of every component
bucket. A bucket's own `alignment` in the same `style` value wins for that
bucket:

```yaml
style:
    alignment: center      # every component centers...
    table:
        alignment: left    # ...except tables
```

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

Most of these buckets also take `margin`, `padding`, `border`, `emphasis`,
and `word-wrap` (see [Style Hints](../rendering/style.md#margin-padding-border-emphasis-and-word-wrap)).
`images` takes `padding`, `padding-left`, and `padding-right` beside the
common properties, and `hyperlinks` takes only the common properties plus
`local-style`.

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

### Padding

`padding` sets all four sides of a component's padding; `padding-left` and
`padding-right` then set one horizontal side each. Within one bucket the sides
always apply after `padding`, whatever order you write them in:

```yaml
style:
    table:
        padding: 0          # top, right, bottom, and left are 0
        padding-left: 4     # then the left side is 4 cells
```

A side on its own changes only that side, so a component keeps whatever top
and bottom padding it already had. To indent a component with no vertical
padding, reset with `padding: 0` first, as above. The same keys work in
`table`, `block-quote`, `code-block`, `disclosure`, `ul`, `ol`, `li`, `hr`,
`images`, and the `local-style` buckets of `hyperlinks` and `images`. In HTML they become
`padding-*` CSS on the element; in a terminal they move the component inside
the content width.

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

Padding moves a rule inside the content width. In a 50-column terminal,
`padding-left: 4` starts the rule 4 cells in (46 wide), and
`{ padding: 0, padding-left: 4, padding-right: 4 }` draws it 42 wide:

```yaml
style:
    hr:
        padding-left: 4
```

## HTML Page Fields

The `page.stylesheet` field adds CSS to HTML output:

```yaml
style:
    page:
        stylesheet: ./docs.css
```

The value is a file reference, so `./docs.css`, `docs.css`, `&styles/docs.css`,
`~/styles/docs.css`, and an absolute path all work. It resolves from where the
value was written:

| Written in | A relative `stylesheet` resolves from |
| --- | --- |
| the document's `style:` | the document's directory (the launch directory for standard input) |
| a `--style` file | that style file's directory |
| a setter (`style.page.stylesheet=…`) | the directory `md` was started in |

The value that wins the merge keeps its own origin, so a later layer that
sets other fields does not move it. A local stylesheet is read only for HTML and Markdown Plus
output, where it is inlined into the generated page; terminal, plain, Markdown,
and data output never read it. A stylesheet that does not exist, or a
reference that cannot be resolved, fails an HTML render with an error naming
it. `http` and `https` stylesheet URLs are emitted as external stylesheet links
and never fetched. `file://` stylesheet URLs are rejected; use a normal local
path instead.

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

## Themes, Code Blocks, and Line Numbers

Four keys choose how prose and code look:

```yaml
style:
    page:
        theme: nord           # prose theme
        code:
            theme: github     # code-block theme
            mode: inverse     # inverse, dark, light, or same
    code-block:
        line-numbers: true    # false turns the gutter off
```

Theme names are the ones `md list-themes` prints, in any case (`Base16-Ocean`,
`vs-dark`). `mode` chooses a code block's light or dark variant relative to the
page: `inverse` (the default) gives a light panel on a dark page and the
reverse, `same` follows the page, and `dark`/`light` pin one variant.

When a key is not set, Darkmatter decides it like this:

| Key | Unset |
| --- | --- |
| `page.theme` | the `THEME` environment variable, else `one-half` |
| `page.code.theme` | the `CODE_THEME` environment variable, else the prose theme (including one set by `page.theme`) |
| `page.code.mode` | `inverse` |
| `code-block.line-numbers` | off |

An environment value that names no theme falls back to `one-half`. A theme set
on one fence (`theme=dracula`) still wins over all of these for that block.
A style setter (`style.page.theme=github`) or a `--style` file overrides
these fields; one you leave out leaves the document's value.

## CLI Overrides

`md` (and `md render`) has no layout flags. To change a document's style for
one render, add style on the command line in either of two forms:

- **Setters** are words of the form `style.KEY=VALUE`, written anywhere
  among the input and options. `style='{ … }'` sets several keys at once.
- **Style files** are given with `--style FILE`. A style file is a local JSON,
  JSON5, YAML, or TOML file holding the same mapping you would write under
  `style:`; `--style style@shared.yaml` reads the `style` key of a larger file.

```sh
md guide.md style.mx=6                                # one setter
md guide.md style="{ max-width: 80, theme: nord }"    # several keys at once
md guide.md --style team.yaml style.page.theme=nord   # a shared file plus one tweak
```

Every key in this page can be set either way, with the same spelling and
values as in frontmatter, so a value pastes between a document and the
command line. Use frontmatter for defaults that belong to the document; use
setters or a `--style` file for a one-off rendering choice. The full grammar,
including file offsets and completion, is in
[Styling from the command line](../cli/render.md#styling-from-the-command-line).
The old layout flags (`--mx`, `--fill`, `--theme`, …) are gone; the
[migration table](../cli/render.md#presentation-flags) gives the replacement
for each.

### Later Layers Win, Field by Field

Each source of style is a _layer_ that holds only the fields it sets. The
layers merge in this order, each over the one before:

1. the document's `style:` frontmatter
2. each `--style` file, in command-line order
3. all setters, as one final layer, wherever they appear on the line

A later layer replaces only the fields it sets. This command keeps the
document's `margin-top`, `margin-bottom`, and `max-width`, and replaces the
left and right margins:

```sh
md guide.md style.mx=6
```

The merge follows a few rules:

- Mappings merge key by key; a number, string, or list replaces the earlier
  value whole.
- A value that changes form replaces the whole earlier value: a later
  `table.border: { left: false }` does not keep anything from an earlier
  `border: thick`.
- Setting `width` drops an inherited `max-width` (and the reverse), and
  setting a component's `padding` drops inherited `padding-left` and
  `padding-right`, so the later layer never produces a conflicting pair.
- `null` means "not set". A `null` in a style file or setter does not reset
  the document's value; there is no reset syntax.

[Precedence: Later Style Layers Win, Field by
Field](../rendering/style.md#precedence-later-style-layers-win-field-by-field)
shows how each layer is checked before the merge.

### Shorthands Across Layers

Specificity decides between spellings only _inside_ one layer. Each layer
expands its shorthands first, and then the layers merge in order, so a later
broad key beats an earlier specific one:

```sh
# doc.md has `style: { ml: 6 }`; team.yaml holds `margin: 2`
md doc.md --style team.yaml       # left margin 2: the later layer wins
md doc.md style.ml=6 style.margin=2   # left margin 6: one layer, `ml` is more specific
```

### Repeated Setters

Setting the same path twice keeps the last value. A whole-object setter is one
assignment to the path `style`, so repeating it replaces the whole earlier
object rather than merging with it:

```sh
md notes.md style.ml=2 style.ml=4                         # left margin 4
md notes.md style="{ ml: 6 }" style="{ max-width: 50 }"   # only max-width: 50 applies
md notes.md style="{ ml: 6 }" style.max-width=50          # both apply
```

Two different spellings of one field among the setters, such as
`style.ml=2` and `style.page.margin-left=3`, fail rather than depending on
their order.

### Duplicate Keys in a Style File

A style file is read by the same readers as other data files, so a key written
twice in one mapping follows that format's rule:

| Format | A key repeated in one mapping |
| --- | --- |
| JSON, JSON5 | the last value is used |
| YAML, TOML | the file fails to load |

This is separate from two _spellings_ of one field (`ml` beside
`page.margin-left`), which is a conflict in every format.

## Validation and Troubleshooting

Use `--deny style` when you want style frontmatter to fail fast on typos or
deprecated keys (see [Policy Flags](../cli/policy.md) for the other policy
verbs):

```sh
md guide.md --deny style
```

`--deny style` rejects:

- Unknown keys such as `style.page.lft-margin`, in the document or in a
  `--style` file.
- Deprecated snake-case aliases such as `max_width`.
- Deprecated horizontal-rule inline aliases.

Without `--deny style`, an unknown key is treated according to where it was
written:

| Written in | An unknown key |
| --- | --- |
| the document's `style:` | a warning; the key is ignored |
| a `--style` file | a warning naming the file and line; the key is ignored and the rest of the file applies |
| a setter | an error (`dm.schema.unknown_key`, exit 1) |

A file is lenient because it may be shared between tools or versions; a
setter is something you just typed, so a typo stops the run. A member a
compound value does not declare is an unknown key too: `glow` in
`table.border: { weight: thick, glow: 1 }` is warned about and dropped from a
document or file, while the setter `style.table.border.glow=1` fails.

Other `--style` file rules:

- A file that sets nothing once unknown keys and nulls are removed, such as
  `{}`, fails (`dm.style.empty_file`). An inline `style='{}'` is a valid no-op.
- Files are local only: a URL, `--style -` (standard input holds the
  Markdown), and an inline value such as `--style '{ margin: 4 }'` are refused.
  The last one points you at the setter `style='{ margin: 4 }'`.

Invalid values fail even without `--deny style`, and every supplied file and
setter is checked, even one whose value a later layer replaces. A failure
names the property and where it came from:

```text
Error: invalid style: `style.page.max-width`: "abc" is not valid under any of the schemas listed in the 'anyOf' keyword (in the style file `bad.yaml` at line 1, column 12)
```

Common examples:

| Invalid Value | Why It Fails |
| --- | --- |
| `margin-left: -2` | Negative lengths are not allowed. |
| `max-width: 120%` | Percent lengths must be between `0%` and `100%`. |
| `margin-top: 2ch` | Vertical margins are row counts, not horizontal lengths. |
| `width` and `max-width` in the same bucket | The bucket can have only one width strategy. |
| `color: bluish` | Color must be Tailwind, hex, or a web color name. |
| `ml: 2` beside `page.margin-left: 3` | Two spellings of one field; keep one. |

### Custom Keys and Cleanup Rules Do Not Render

The keys a layer may hold come from the document's effective schema, the same
schema that judges the rest of its frontmatter. A document can admit keys of
its own with a pattern key:

```yaml
$schema:
    style:
        "<string>": unknown
style:
    brand: acme
```

`brand` is then accepted from the document, a file, or a setter
(`style.brand=acme`), merged like any other key, and offered by completion,
but nothing renders it: admitting a key is not implementing a rendering
feature. A known key still has to hold a value the renderer accepts, whatever
the schema allows. A document that declares `style` without a pattern key
replaces the built-in definition, so only the keys it declares are accepted.

`style.markdown` holds the formatting rules of `md clean` (see
[Markdown Formatting Rules](./markdown-formatting.md)). A `style.markdown`
value from a setter or a `--style` file is validated, but it never changes
the cleanup `md` performs before rendering, never edits the input, and never
reaches `md clean`.

When a comment or example disagrees with rendered behavior, trust the rendered
behavior and update the document. The style system is intentionally narrow:
small, typed frontmatter keys lower directly onto the render tree, and target
renderers handle the terminal or browser details.
