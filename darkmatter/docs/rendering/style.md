# Darkmatter Style Hints

## Style Frontmatter

Darkmatter reserves the Frontmatter property `style` for defining stylistic preferences. This is used by the following page elements:

1. `page`
1. `table`
1. `hyperlinks`
1. `images`
1. `hr` - Horizontal Rules
1. `ul` - Unordered Lists
1. `ol` - Ordered Lists
1. `li` - List Item
1. `block_quote`

When performing a composition argument the the caller can send in a style object to modify the default style of the full page graph (`style` property is passed down to child properties).

## Style Mutation

### Common Mutations

Each of the properties defined under `style` provide the following mutations:

- `width(ch | %)` - set's a fixed width for the element in question (ch or %)
- `max_width(ch | %)` - set's a maximum width; this can be used in conjunction with width but that only really makes sense when one has a fixed value and the other a percentage. Since the percentage is lazily loaded at render time 
- `alignment`
- `color`
- `bg_color`

### Bespoke Style

While _every_ style property provides the _common_ mutations, each of the types provide their own bespoke properties which can be set:

- `page`
    - `stylesheet` - allows you to point to an external CSS stylesheet (local file or HTTP pointer)
    - `meta`
    - `code`
    - `max_width`
    - `alignment`
- `hr`
    - **IMPORTANT:** currently the `hr` functionality has implemented their bespoke styles directly to the top-level `hr` property and that needs to be moved here as `style.hr`
        - `darkmatter/lib/src/markdown/block/hr_builder.rs:117`
    - `kind` (this replaces `hr.style` when moved from current implementation)
    - 
- `table`
    - `width`
    - `max-width`
    - `alignment`
- `hyperlinks`
    - `local_style` - provides an override of `style` but only for links to local files
- `images`
    - `local_style` - provides an override of `style` but only for links to local files

## Tailwind Colors

The `biscuit-terminal` already provides a very handy mapping to Tailwind color names to their RGB values. The base
colors supported are:

- red
- orange
- amber
- yellow
- lime
- green
- emerald
- teal
- cyan
- sky
- blue
- indigo
- violet
- purple
- fuchsia
- pink
- rose
- slate
- gray
- zinc
- neutral
- stone

Each of colors provides the following luminosity levels:

- 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, and 950

- whenever someone sets a **style** value to a color, they may choose to use Tailwind names like `red-500` which combine the
color with the luminosity level.
- you can also add in Tailwind's convention of an opacity setting with `red-500/50` where the trailing `/50` indicates the opacity setting 
    - Note: the opacity is only used when targeting HTML, it is dropped everywhere else

## Code Block Theme & Contrast

The `page.code` theme (`--code-theme`, `code_theme`) is a mode-agnostic
`ThemePair`, resolved to a concrete light/dark theme at render time. In
**terminal** output, code blocks deliberately resolve against the *inverted*
color mode so the panel contrasts against the page (light code on a dark
terminal, and vice versa); prose, tables, and the page background follow the
real terminal mode. **HTML output does not invert.** Single-variant themes
(`dracula`/`nord`/`monokai`/`vs-dark`) ignore the mode by design.

See [Code Highlighting](./code-highlighting.md) for the full model.
