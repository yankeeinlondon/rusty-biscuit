- Ok so the rendering of styles and layouts in Markdown _has no standard_ and really comes down to the app rendering the Markdown.
- Darkmatter is a renderer of Markdown, so it **Marked**, so is **Typora**, so is the VSCode plugin
- What's consistent across most of these -- especially the standalone apps -- is that they provide a theming system which allows people to express how they'd like their markdown to look.
- I believe almost all theme systems are CSS derived and certainly we would want that for Darkmatter
- if you're interested you can look at some research into how **Marked** and **Typeora** do this: @renderable/features/2026-04-17-layout-and-style/marked-and-typora.md
- I think what we really have with these apps are built in HTML renderers of Markdown
- so when we say _render to Markdown_ we should immediately think of the `MarkdownRenderable` trait which requires the following methods:

    ```rust
    /// Renders the component as a Markdown string (including Markdown body and optionally 
    /// YAML Frontmatter).
    /// 
    /// - any valid Markdown can be passed through "as is"
    /// - some renderers might offer (on their input) an option to "clean" the markdown to make it more idiomatic
    /// - some renderers might decide to provide an option (on their input) to 
    fn render_markdown(&self) -> String;
    
    /// Renders the component as a Markdown string (including Markdown body and optionally 
    /// YAML Frontmatter).
    /// 
    /// - Features often supported by markdown renders like:
    ///     - iframes (darkmatter will convert ergonomic markdown directives to the HTML variant)
    ///     - disclosures (darkmatter will convert ergonomic markdown directives into HTML variant)
    ///     - etc.
    /// 
    /// > **Note:** the features supported should _never_ rely on Javascript!
    fn render_markdown_plus(&self) -> String;
    ```

- in essence if a component wanted to implement the `MarkdownRenderable` trait and they had a valid Markdown string they could just return that on both functions.
    - that wouldn't be exactly in keeping with the intent of the **MarkdownPlus** separation but it wouldn't be incorrect for any component type that was trying to represent something that Markdown can already represent in full fidelity.
- **Darkmatter** will be the quintessential example for this trait and it would _maintain_ the `style` Frontmatter for the `render_markdown()` function but "upgrade" the `style` property for any Markdown that was being converted from a Markdown representation to a inline-HTML representation.
- Darkmatter's intent when a caller targets the MarkdownPlus format is to preserve as much of the writing ergonomics as possible but add in features that would _likely_ be supported by most of the Markdown render apps. The output is still "valid Markdown" but it should look nicer in many cases.
- I think the best example of this is the Disclosure feature which is perfectly implemented in modern browsers -- without even any CSS -- just by using the `<details>` and `<summary>` tags. Every Markdown renderer I know of will render this but if you force someone to author in it it gives a way a lot of the ergonomics.

So ultimately, the Darkmatter library will have a lot of work to convert Markdown to the terminal or the browser, but the Markdown and MarkdownPlus targets are something I think Darkmatter will own not other higher level abstractions.
