# Renderable

- provides the traits and utilities to allow for type strong, multi-target renderable components
- targets recognized are:

    1. Markdown 
    2. MarkdownPlus - _still just a text/Markdown output but with a richer feature set enabled through more use of inline-html_
    3. Terminal
    4. Browser
    5. AST

    > Note: this enumeration of targets can be found in this library as [`RenderTarget`](./src/target.rs)

- the traits defined here include:

    1. `MarkdownRenderable`
    1. `BrowserRenderable`
    1. `AstRenderable`

    > **Note:** the `TerminalRenderable` trait is defined in the `biscuit-terminal` library
