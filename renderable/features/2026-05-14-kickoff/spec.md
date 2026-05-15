# Renderable Kickoff

The new `renderable` library is the result of wanting to better modularize the traits and utilities for "renderable targets" that a component may want to target.

## Context

We have spent a lot of time focusing on rendering to the terminal and we've arrived with some good solutions but in almost every case where we might want to render to the terminal we are going to start to need also render to the browser. The combination of the `Renderable` (soon to be renamed `TerminalRenderable`) and `BrowserRenderable` traits are a good way to help us align all our components for both renderable targets.

In addition to terminal and browser based targets we also want to develop and make other targets first class citizens too:

- AST
- Markdown
- and maybe others in the future

> Note: it may be worth defining the terms Markdown and MarkdownPlus as targets. In some regards there really is no thing as MarkdownPlus but we're using it as a way to describe a Markdown target which includes more features by including more inline HTML in the produced Markdown content.

## Reference

### Components in `biscuit-terminal`

```txt
┌─────────────────┬─────────────────────────────────────────────────┬────────────┬──────────────────────┐
│    Component    │                      File                       │ Renderable │  BrowserRenderable   │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ BlockQuote      │ src/components/block_quote.rs                   │ ✅ (L248)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ Compose         │ src/components/compose.rs                       │ ✅ (L110)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ FileSystem      │ src/components/filesystem/mod.rs                │ ✅ (L1491) │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ GraphExpression │ src/components/graph_expression.rs              │ ✅ (L374)  │      ✅ (L393)       │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ HorizontalRule  │ src/components/horizontal_rule/{mod,browser}.rs │ ✅ (L152)  │ ✅ (browser.rs:L103) │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ InlineContent   │ src/components/inline_content.rs                │ ✅ (L274)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ MermaidDiagram  │ src/components/mermaid.rs                       │ ✅ (L575)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ OrderedList     │ src/components/list.rs                          │ ✅ (L230)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ PadLeft         │ src/components/pad.rs                           │  ✅ (L75)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ PadRight        │ src/components/pad.rs                           │ ✅ (L176)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ Progress        │ src/components/progress.rs                      │ ✅ (L115)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ Prose           │ src/components/prose/render.rs                  │  ✅ (L7)   │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ Section         │ src/components/section.rs                       │ ✅ (L180)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ Status          │ src/components/status.rs                        │ ✅ (L528)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ StatusBlock     │ src/components/status_block.rs                  │ ✅ (L100)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ Table           │ src/components/table/table.rs                   │ ✅ (L1315) │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ TerminalImage   │ src/components/terminal_image/mod.rs            │ ✅ (L226)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ TextBlock       │ src/components/text_block.rs                    │ ✅ (L150)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ Todo            │ src/components/todo.rs                          │ ✅ (L292)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ TwoColumn       │ src/components/two_column.rs                    │ ✅ (L437)  │          ❌          │
├─────────────────┼─────────────────────────────────────────────────┼────────────┼──────────────────────┤
│ UnorderedList   │ src/components/list.rs                          │ ✅ (L531)  │          ❌          │
└─────────────────┴─────────────────────────────────────────────────┴────────────┴──────────────────────┘
```

### Components Outside of `biscuit-terminal`

Most of the components live in `biscuit-terminal` but `darkmatter` also hosts these three:

```txt
┌────────────────┬─────────────────────────────────────────┬────────────┬───────────────────┐
│   Component    │                  File                   │ Renderable │ BrowserRenderable │
├────────────────┼─────────────────────────────────────────┼────────────┼───────────────────┤
│ YamlBlock      │ src/markdown/yaml_block.rs              │ ✅ (L169)  │     ✅ (L246)     │
├────────────────┼─────────────────────────────────────────┼────────────┼───────────────────┤
│ FileTree       │ src/markdown/reference/file_tree/mod.rs │ ✅ (L268)  │        ❌          │
├────────────────┼─────────────────────────────────────────┼────────────┼───────────────────┤
│ DarkmatterPage │ src/layout/page.rs                      │ ✅ (L701)  │     ✅ (L728)     │
└────────────────┴─────────────────────────────────────────┴────────────┴───────────────────┘
```

## Projects

Below are a set of "projects" where each project will be broken down into it's own multi-phase implementation plan.

### 1. Establish a Stable Base

- currently the `biscuit-terminal` library is the source both `Renderable` and `BrowserRenderable` traits and the first thing we need to do is

    1. Rename the `Renderable` trait to `TerminalRenderable` which makes the naming much clearer

        - there are no other external repos that we need to be concerned about; just libraries in this monorepo (darkmatter and biscuit-terminal specifically)
        - we should _also_ rename `RenderableContent` to be `RenderableTerminalContent` 

    2. Extract the `BrowserRenderable` definition from `biscuit-terminal` where it is currently defined

        - move the implementation/definition of the trait from `biscuit-terminal` to `renderable`
        - update `biscuit-terminal` to use the trait in the new location

    3. Extract the `Stylesheet` struct from `darkmatter` and move to `renderable`

        - read more details in [Stylesheet Extraction](./stylesheet-extraction.md) section for details

    4. Extract `Layout` and `Color` structs from `biscuit-terminal` to `renderable`

        - read more details in [Layout and Color Extraction](./layout-and-color-move.md) section for details


At this point we need to make sure all tests in `biscuit-terminal` and darkmatter pass all tests and take our time to make sure our tests are comprehensive. 

### 2. Building off the Base

At this point we will have reached a stable base and now we should start to build out the symbols to meet our current and immediate future needs:

#### `MarkdownRenderable`

The `MarkdownRenderable` trait does not yet exist but it's goal is to represent the API contract that components who can render to Markdown
output are obliged to follow. Below is a rough draft of what this should look like:

```rust 
pub trait MarkdownRenderable {
    pub fn render_markdown(&self): String;
    // allows a stylesheet struct to be passed in and the Markdown addressable components
    // extracted to be used 
    pub fn render_markdown_with_style(&self, Option<StyleSheet>): String;
}
```

#### `BrowserRenderable`

We already have a working trait, we've just moved it's location to `renderable` in Project #1. Now we are going to mold it to fit our future goals but preserve the existing API surface for now. 

- In this phase we will retain the existing API surface of `BrowserRenderable`
- but add the future API surface and provide dummy default implementation to not burden callers with this yet
- in essence the _existing_ API surface is deprecated but we don't need any deprecation precautions as these libraries are not released yet.

The API surface this Project will aim for is:

```rust
// the update API surface
pub trait BrowserRenderable: std::fmt::Debug + Any {
    // pre-existing
    fn render_to_browser(&self) -> String;
    fn render_to_browser_with_inline_variables(&self,_variables: &HashMap<String, String>) -> String;
    // adding for future API surface
    fn render_html_fragment(&self) -> BrowserFragment;
    fn render_html_page<T: into PageOptions>(&self, page: Option<T>) -> String;
}
```

- to better understand the thinking of these two new functions and what they imply we went through an exploration phase about [what rendering to the browser means](./rendering-to-the-browser.md).
- and then we wrote the following details around symbols and ideas we think will be important to reaching the intended 

##### New Functions

In order to understand the new API surface we need to take a step back and think carefully about what rendering to a browser should look like. We provided the design output on that very topic: TODO.

##### HorizontalRule

What HorizontalRule's override does

```rust
fn render_to_browser_with_inline_variables(
    &self,
    variables: &HashMap<String, String>,
) -> String {
    let mut svg = self.render_to_browser();
    for (key, value) in variables {
        svg = svg.replace(&format!("var(--{})", key), value);
    }
    svg
}
```

It calls the regular render_to_browser() then does a literal var(--name) → value string substitution for each entry in the
variables map. This is backward-compatibility behavior: callers that embed var(--rule-width) literally in things like
HorizontalRule::new().width("var(--rule-width)") can have those tokens resolved at render time. The default impl just calls
render_to_browser() and ignores the variables map entirely, so without this override the substitution would never happen.

#### `AstRenderable`

### 3. Using the new API Surface

Now that we've built out the new interface operating side-by-side with the old and simpler approach we're ready to graduate to
getting all call sites to to use the new API surface. The broad scope of this Project is:

1. Small Naming Housekeeping

    - our `Stylesheet` struct would be better named `CssStyle`
    - then our `HtmlClassDefinition` struct can be renamed `Stylesheet` because that's really what it is :)

2. Migrate all callers to the new API surface

    - migrate all biscuit-terminal callers
    - add test to ensure STRONG test coverage
    - have `biscuit-terminal` compile a representative example of using components to create an HTML page

3.
