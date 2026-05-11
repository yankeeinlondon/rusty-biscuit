# Darkmatter Style Hints

## Style Frontmatter

Darkmatter reserves the Frontmatter property `style` for defining stylistic preferences. This is used by page elements like:

1. `page` _this is where can set style and metadata information which pertains to the whole page._
1. `hr` - _Horizontal Rules have a number of rendering characteristics which can be adjusted as part of the style property_
1. `table`
1. `hyperlinks`
1. `images`
1. `block_quote`

## Style Mutation

### Common Mutations

Each of the properties defined under `style` provide the following mutations:

- `class` _provides class name(s) to relevant section of the page; this will only be leveraged when targeting HTML outputs_

    ```yaml
    style:
        page:
            class: "page my-page"
    ```

- `style` _provides a direct way to manipulate the relevant section of the page's style attributes directly_

    - unlike `class` which is only able to be leveraged when targeting HTML, the `style` property can be leveraged across multiple targets:
        - `html` target:
            - renders the section using HTML's `style` attribute
            - in most cases the string is literally used "as is" as the HTML's style value
            - the only exception is our inclusion of "tailwind colors" (see below)
        - `terminal` target:
            - the `color
        - `markdown` target:
        -

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
    - `
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
