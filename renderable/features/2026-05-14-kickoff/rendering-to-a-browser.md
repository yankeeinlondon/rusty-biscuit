# Rendering to a Browser

We are trying to create a reusable API surface for **components** who want to be able to put out content that can be rendered in a browser. To do this we must first try to understand:

- how a component can:
    - define it's default styles
    - define it's dependencies on code or external CSS
    - allow for a _caller_ to **override** the default styles or even the default dependencies (though these are likely to be a bit more stable)

## Components versus Pages

**Decision:** a "components" scope is not an HTML Page but rather a **part** of a page.

- components should produce HTML fragments
- but we should also be able to easily compose these fragments into full fledged HTML pages
- a component _should not_ 
    - have an opinion on whether it's CSS is rendered inline or referenced as an external file 
- but it should be able to
    - define the CSS for it's default state, and
    - define external CSS or JS dependencies
    - provide JS code blocks to the page's benefit
    - (future) declare dependencies to shared JS features
    - (future) 

## Infrastructure

We have agree that a component defines a part of a page not a page but we will need to render both so we will create a struct called `HtmlPage`:

```rust
struct HtmlPage {
    stylesheet: HtmlStyleSheet;
    links: Vec<LinkTag>;
    meta: Vec<MetaTag>;
    title: Option<String>;
    features: Vec<CodeFeature>;
    fragments: Vec<BrowserFragment>;
}
```

> There is a "rough draft" implementation of this in @renderable/src/html/mod.rs 
>
> There are also draft definitions of `HtmlStyleSheet`, `LinkTag`, `MetaTag`, `CodeFeature` and `BrowserFragment` in the source code

## CSS Variables

Using CSS variables is a great form of abstraction these days with all evergreen browsers providing full support. We should
inject CSS variables for the Tailwind colors. Colors in general feel like the area of greatest impact here but what other kinds of things are good candidates for CSS variables?

## Rendering

### Component Scope

When rendering for _part of the page_ you must ensure that the component can compose an HTML fragment which can:

1. Define good defaults

    - each component has it's own internal structure that only it knows about 
    - it should, as part of it's internal rendering process include CSS classes throughout this structure to make targeting parts of the component easier

        As an example, a fictional "simple_table" component might render something like:

        ```html
        <table class="simple-table">
            <tr class="heading"><td class="heading col-string">Name</td><td class="heading col-num">Age</td></tr>
            <tr><td class="data col-string">Bob</td><td class="col-num">
        </table>
        ```

    - in addition it needs to be able to express a "stylesheet" (aka, classes and their CSS definitions) that map into it's own internal structure

    **Scoping rule (decided):** internal class names registered against a `ComponentStylesheet` are lowered to **descendant selectors** under the component's wrapper class — not concatenated or class-chained.

    For the `simple-table` example below, registering an entry for `"col-string"` produces:

    ```css
    .simple-table .col-string { /* … */ }
    ```

    **not** `.simple-table-col-string` (name mangling) and **not** `.simple-table.col-string` (class-chain). The descendant form lets internal elements nest arbitrarily within the wrapper while still being scoped by it.

        Using our fictional "simple_table" example again:

        ```rust
        let fragment = BrowserFragment::new();
        let my_style = ComponentStyle::new("simple-table")
            .add("table", StyleSheet::new({ padding: "4px" }))
            .add("col-string", StyleSheet::new({ align: "left" }))
            .add("col-number", StyleSheet::new({ align: "right" }));
        fragment.set_default_styles(my_style);
        ```

    - in addition to styling a component should also be able to express:
        - meta tags which it wants in the HEAD

            ```rust
            let fragment = BrowserFragment::new()
                .add_metadata_keypair(key, value) 
                .title("That's all she wrote")
            ```

        > Note: there is some rough draft code ["microdata" module](@renderable/src/microdata.rs) to help implement the `metadata` function

    - link tags which it wants in the HEAD

2. Allow for the caller's parameters to mutate the "defaults" by providing it the capacity to express:

    - the caller's parameters will be represented by `style` definitions in the appropriate places of the HTML Fragment they are responsible for rendering. 
    - the `style` will always override the default "class" definition
    - there is still an opportunity to override the override but that would require the use of the undesirable `!important` operator but that seems like an acceptable trade off in having a nice clear separation between "component" styling and "instance" styling.

### Page Scope

The `BrowserRenderable` trait requires that components who render to the browser must implement the following functions:

```rust
fn render_html_fragment(&self) -> BrowserFragment;
fn render_html_page<T: into PageOptions>(&self, page: Option<T>) -> String;
```

In the Component Scope section we already discussed how a component can meet all it's goals for building a `BrowserFragment` and therefore fulfilling the contract for the first function. In this section we'll tackle how to take that `BrowserFragment` and convert it to a full HTML page.

The process would look something like:

```rust
let fragment = component.render_html_fragment();
let page = HtmlPage::from(&fragment)
    .apply_page_options(options.)
    .render();
```

In this simple example, we only have a single fragment that will resolve to being a page and that's all we need to complete the

## Render Pipeline

This section pins down the contract for `HtmlPage::render()` so component authors and page assemblers can reason about output without reading the renderer. Detailed policy hooks (inline vs external, theming) are still TBD — see [brainstorming.md](./brainstorming.md) — but the **shape** of the output and the **ordering of `<head>`** are stable decisions.

### `<head>` Ordering

The renderer emits `<head>` children in this fixed order. Browsers tolerate other orderings, but `<meta charset>` *must* appear in the first 1024 bytes for some parsers, and `<title>` placement affects how page previewers (Slack, link unfurlers) extract metadata.

1. `<meta charset="utf-8">` — always first.
2. `<meta name="viewport" …>` — responsive defaults.
3. `<title>` — sourced from microdata `Title` only (single path; see brainstorming).
4. Other microdata-driven `<meta>` tags, grouped by source in the order: HTML, OpenGraph, Twitter, Schema.org. Within each group, registration order is preserved.
5. `<link>` tags — page-level first, then deduplicated component dependency links in first-seen order (see [Dependency Deduplication](#dependency-deduplication)).
6. `<style>` blocks (when inlining is chosen):
    1. `:root { --name: value; … }` — page-level CSS variables, in declaration order.
    2. Page stylesheet rulesets.
    3. Component default stylesheets, in fragment-registration order.
7. `<script>` blocks — page-level first, then per-fragment. Components are expected to mark their scripts `defer` or `async` unless they intentionally need synchronous execution.

### Dependency Deduplication

`<link>` tags can arrive from two sources during page assembly:

- Page-level (added via `HtmlPage::add_link`).
- Fragment-level (added via `BrowserFragment::add_linked_dependency`, then collected when the fragment is composed into the page).

Both streams pass through a single dedup pass at render time, keyed by `(rel, href)`. First-seen wins for ordering; later duplicates are dropped silently. Page-level links are seen first, so they always win ties against fragment-level ones with the same key.

Differences in `media`, `hreflang`, or `title` are **not** part of the dedup key — the assumption is that if two components reach for the same `(rel, href)`, they want the same stylesheet, and divergent `media` queries indicate a bug or inconsistency that the caller should resolve explicitly.

See `LinkTag::dedup_key()` and `HtmlPage::collect_dedup_links()` for the implementation.

### CSS Variable Emission

Page-level CSS variables are emitted at the top of the first inlined `<style>` block as:

```css
:root {
    --color-blue-500: #3b82f6;
    --space-2: 0.5rem;
    /* … */
}
```

Components reference these via `var(--foo)` in their default stylesheets. Components **consume** variables; the **page** declares them. A component must not emit `:root { … }` blocks of its own.

### Inline vs External — Policy TBD

The decision of whether stylesheets, scripts, and CSS variable blocks are inlined into the page or referenced as external resources is a **render-time policy**, not a component concern. The current draft does not lock this down; see [brainstorming.md](./brainstorming.md#inline-vs-external) for the discussion queue.

## Inputs

There are some parameters which describe how a component should relate the page/canvas/terminal which it is to be rendered onto. These parameters are described well by the `Layout` struct defined in 

### Stylistic Overrides

One of the most important things when considering the _inputs_ is how a caller can manipulate the styling of the browser content. This is largely accomplished by defining **classes** and/or **CSS Variables**. The rendering process can then choose whether the classes and variables are injected directly into the same page's content or referenced as an external file. Ultimately though, the classes, CSS variables, as well as any inline styles embedded into the content will effect the style through CSS.


#### Colors

The world of colors is always a really important area of a web page's design and while "web safe colors" are an enumerable set of colors the overall number of colors which can be rendered by a modern browser is massive and not 

### More than Style

I think it is fair to say that the most important thing a caller must have control over is the _style_ of the page but it's not the only thing. Other things which a component might want be able to control are:

- be able to express what goes into the HEAD section of the page
    - meta tags
    - inline stylesheets
    - inline javascript
    - etc.
