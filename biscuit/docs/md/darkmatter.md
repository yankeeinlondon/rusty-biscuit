# Darkmatter

In this repo we introduce a _variant_ of Markdown which we will call **Darkmatter**.

- **Darkmatter** is really just [CommonMark](https://commonmark.org/) Markdown with [GFM](https://github.github.com/gfm/) with a small DSL sprinkled on top
- So the real question is ... what is the DSL which defined **Darkmatter**
- this document is meant to answer that question

## The DSL

The DSL will be broken into the following categories:

1. **Links** - _non-destructive additions the Markdown link syntax_
2. **Images** - _non-destructive additions to the Markdown image syntax_
3. **Inline Tokens** - _new **inline** tokens which a **Darkmatter** aware render will be able to render into HTML, terminal, etc._
4. **Block Tokens** - _new **block** tokens which a **Darkmatter** aware render will be able to render into HTML, terminal, etc._

## Links

In Markdown we typically see a straight forward syntax which looks like:

> [Link Text](https://somewhere.com)

This is of course perfectly a perfectly valid link in Markdown but there _is_ a second parameter you can include with the parenthesis:

> [Link Text](https://somewhere.com My glorious title)

This second reference is ALSO completely valid Markdown but is so uncommonly used that some Markdown LSP's may not recognize it and other linters like the popular Markdown linter `markdownlint` will warn you about **no-bare-urls** when the first syntax it took no issue with.

> Note: if you want to remove this lint warning then add a `.markdownlint.{json|jsonc}` file and the entry `"MD034": false` to it.

What this optional second parameter was intended for was setting a "title" for the link. Many renders will just ignore it but we can leverage it to do our bidding.

### Using the Title of a Markdown Link

In Darkmatter you _can_ use the "title" component of a Markdown link exactly as god intended:

- in HTML renders it will add the `title` tag which most browsers will present as a crude "popover" render when hovered over.

However, with Darkmatter you can wrap this section with curly braces and then put in key value pairs to your heart's content. Here's a completely valid Darkmatter syntax:

```md
[Link Text](https://somewhere.com {title="My glorious title",style="color: red"})
```

The surprising thing is, is that this is not only valid **DarkMatter** but it is also valid **Markdown**! The only difference is _what happens when you render it_?

- If you render it in with a traditional Markdown renderer then it should just render as a link; possibly with a hover effect.
- If you render it with a Darkmatter renderer then all your hopes and dreams will come true:
    - HTML Rendering:
        - the effect is pretty straight forward, the key/values you defined will be added into the `<a>...</a>` hyperlink which is rendered.
        - in addition if the "popover" element is included in your Markdown file then we will inject some modern CSS to take advantage of the modern **Popover** functionality provided in modern browsers.
    - Terminal Rendering:
        - if the `style` property is defined then the CSS key-value properties will be scanned and any properties which can effectively be rendered to the terminal (using escape codes) will included to help
        - if the `class` property is defined then the renderer will look for a definition of the class and will render those CSS properties associated with the class which are possible in a terminal.

        > For more detail on what is and is not supported in `style` or `class` properties in the terminal refer to the [Mapping CSS Styling to the Terminal](./mapping-css-styling-to-the-terminal.md).

