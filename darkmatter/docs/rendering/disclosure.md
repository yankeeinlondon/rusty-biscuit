# Disclosure Blocks

The Darkmatter DSL provides a more ergonomic way to express the desire for a "disclosure block" (e.g., a small text block that when clicked on opens up a much larger text area ... aka, it _discloses_ it).

## Markdown Syntax

To create a disclosure block in Darkmatter Markdown you would create something like:

```md
::disclosure
License Agreement
::details
Keep your dirty hands off my stuff. You have the right to leave immediately.
::end-disclosure
```

## Lifecycle

During the [_compose lifecycle_](../darkmatter-compose-pipeline.md) this will remain untouched because Markdown provides no way to render a disclosure unless you resort to using inline-HTML and relying on the Markdown viewer supporting this.

Instead this feature is activated during the [_rendering pipeline_](../darkmatter-rendering-pipeline.md) where it can target any of the following targets:

- `markdown`

    - when we specify the target to be "markdown" we will convert the disclosure to an inline HTML section
    - this makes the markdown less ergonomic to edit going forward
    - however, it does allow Markdown renderers who support inline HTML like this to operate.

- `html`

    - leverage the HTML `details` and `summary` elements which are supported in all modern browser
    - no JS is needed

> Note: 
>
> - if the rendering pipeline is `terminal` then it will leave the content unchanged
> - if the rendering pipeline is `ast` then it will render the incoming markdown with the `markdown` output first (creating some inline HTML) and then convert that into an AST

It's worth noting that if you run (assuming that `README.md` has a disclosure in it):

```sh
# implicit render
md README.md
# explicit render
md render README.md
```

- the disclosure will be rendered for the **terminal** as a default so there will be no mutation of the Darkmatter DSL
- the CLI does offer an alternate render path for the terminal though which is:
    - `--render-disclosure`
    - this will only have an _alternate effect_ when targeting the terminal or AST (other targets just ignore the switch)
    - when this flag is used while targeting the terminal we will:
        - render the title section as bold faced text and yellow
        - render the detailed text section as a block quote where the vertical bar is the same yellow color
- the [`style`](./style.md) property can also specify the alternative render style:

    ```yaml
    style:
        disclosure: alternate
    ```
- the [`style`](./style.md) property also provides a few additional stylistic controls which can be used:
    - `indent` _allows the detailed section to be indented by a certain number of characters_
    - `color` _allows the vertical-bar color which is used to be changed from the default yellow
