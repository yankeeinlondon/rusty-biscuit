/planning:plan we need to add a `--align <alignment>` switch in the Biscuit Terminal CLI for both the **image** subcommand and all mermaid subcommands. Before we make this change, however, we are going to need to standardize the library a bit on the whole idea of "renderable" components as well as how to define and render a "layout".

## Terminal Layout

A _renderable component_ does not directly render into the terminal window directly, instead it renders into a "layout". This is an important distinction which needs to be clearly defined.

First let's remind ourselves of what a "renderable component" is:

- defined as a Trait in @biscuit-terminal/lib/src/components/renderable.rs
- it requires that a renderable component provide both a
    - **render** function:
        - used to render the component to a string and _without_ the knowledge of the underlying Terminal's capabilities
        - when used the terminal will be assumed to have all capabilities
        - this can be useful for some _pre-rendering_ or caching scenarios as well as some test scenarios
    - and a **fallback_render** function
        - in most cases, the **fallback_renderer** is the preferred choice as it allows the detected capabilities of a real terminal to be included and in cases where it's possible to gracefully fallback to reduced functionality if the terminal can not handle it

Examples of components include:

- `TextBlock` - allows a text block to have uniform colors, styles, etc. applied to it
- `Prose` - allows a text block to lazily define styling, colors, etc. with "atomic" or "block" tokens; these tokens will be replaced by the appropriate escape codes when the prose text block is rendered.
- `BlockQuote` - renders block quotes to the terminal
- `TerminalImage` - renders images to the terminal
- `MermaidDiagram` - renders a [Mermaid](https://mermaid.js.org) diagram as an image in the terminal

### Breaking Change!!!

The **Renderable** trait now requires a `Option<&Layout>` is provided to both the render and fallback_render methods.

- this change was introduced so that we can have a uniform way to describe the layout we want.
- if you choose `None` for layout then the default layout will be used
- many of the components are currently quite immature and may need some work to make them fully compliant
- all of the components likely need MORE tests to ensure we have good coverage

## Layout

The trigger for all of our work in the Library is the idea of "component layout" and the CLI's addition of the `--layout <layout>` switch maps directly to the `Layout` struct found in @biscuit-terminal/lib/src/utils/layout.rs .

Fortunately for everyone the `Layout` struct _does_ implement the **Default** trait and that means if you don't want to concern yourself with the intricacies of a layout you don't really need to.

