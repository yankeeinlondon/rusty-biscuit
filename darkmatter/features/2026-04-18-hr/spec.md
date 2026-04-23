> **Status (Review 1, 2026-04-23):** The Tier 1 image path (SVG → PNG via `resvg` + `TerminalImage`) is **deferred**. The initial release ships Tier 2 (Unicode) and Tier 3 (ASCII) only — see [`tech-design.md`](./tech-design.md) "Deferred Work" and [`review-plan-1.md`](./review-plan-1.md) Phase 7.

In Markdown the "horizontal rule" (e.g., a horizontal line used to more visibly separate sections in a document) is represented by any of the following when it is on it's own line (and though not _required_ by Markdown but considered good practice, having a blank line before and afterwards):

- `---` (three dashes)
- `___` (three underscores)
- `***` (three asterisks)

The three syntaxes are equivalent and just a matter of preference for the author.

## Rendering

How a "horizontal rule" is _rendered_ is not a part of the Markdown specification but left to the render to decide. In **Darkmatter** we offer a variety of rendering options based on the following dimensions:

- placement (full, centered, left, right)
- visual style (dashes, dots, waves, line-star, line-circle, inset-line, curtain-rod)

### Placement

1. **Full**

    The most common way to render a horizontal rule is to have it cover the full width of the target (respecting margins/padding of course).

2. **Centered**

    Another common technique is to have a fixed width horizontal rule which is placed in a centered position on the output target.

3. **Left** _and_ **Right**

    Far less common but not without it's uses, the **left** and **right** positioning, like the **centered** positioning has a fixed width that is then aligned to the _left_ or _right_ margin of the target output.

### Visual Styles

Masked inside of "visual styles" is a nuance worth calling out up front:

- some visual styles rely on an image (an SVG image specifically) to be used to give the horizontal rule more character than just text characters can achieve
- in cases where an SVG image is used we will have some extra design work to make this work well in the "output targets" section below.

With that out of the way, let's now review the various styles:

- `dashes`
- `dots`
- `waves`
- `line-star`
- `line-circle`
- `inset-line`
- 


### SVG Implementation

- all SVG's will use `currentcolor` for the stroke color
    - this will make the text color the default color for the horizontal rule (usually a good thing)
    - while allowing it to be overridden by explicitly setting the `color`
- we will use CSS variables (and a good default) to provides useful ways to make the shape be resizable horizontally without causing any warping, stretching, etc of the intended style

#### Example

```svg
<svg xmlns="http://www.w3.org/2000/svg" height="40" style="width: var(--hr-width, 100%);">
  <line x1="0" y1="50%" x2="100%" y2="50%" stroke="currentColor" style="stroke-width: var(--hr-line-weight, 6px);" />
  
  <circle cx="50%" cy="50%" fill="currentColor" style="r: var(--hr-circle-radius, 16px);" />
</svg>
```

This example is the SVG we should use for the **line-circle** visual style described above.

## Proper Abstraction

While the Darkmatter library's rendering function will be a significant user of the the horizontal rule functionality, it would be nice if this functionality were available to any terminal rendering application. For that reason we will implement a _renderable_ component in the **biscuit-terminal** library called `HorizontalRule`. 

- the Darkmatter library already uses the `biscuit-terminal` library and therefore can easily incorporate this component into it's functionality
- by having the implementation in `biscuit-terminal` we allow for any consumer who renders to the terminal to leverage this feature for the terminal too

### A new `BrowserRenderable` trait

The `biscuit-terminal` library already defines the [**Renderable**](biscuit-terminal/lib/src/components/renderable.rs) trait which allows for rendering to the terminal but **Darkmatter** is an example of a library that needs to render to not only the terminal but also the browser.

As a part of this feature we will introduce a `BrowserRenderable` trait to the `biscuit-terminal`. On the surface this may seem like an odd location for a trait going by this name but in fact it allows us to expose the components which `biscuit-terminal` provides to a wider array of consumers and in most cases the effort required to make something "browser renderable" _versus_ "terminal renderable" is insignificant.


```rust
pub trait BrowserRenderable {
    /// Renders the component for a browser based target (e.g., HTML, maybe CSS, maybe JS).
    /// 
    /// **Note:** components targeting the browser will often expose CSS variables for 
    /// configurable rendering but in these cases they will always use a sensible default
    /// so that no CSS variables are required to make the component look good.
    render_to_browser(&self) -> String;
    /// Renders the component for a browser based target, including CSS variables which 
    /// the component uses for styling. These variables will be scoped for just this
    /// component. This allows this component to define it's own values for variables 
    /// which are distinct from other instances of the same component. 
    /// 
    /// **Note:** when using this rendering method you should only define variables which
    /// you want to be _distinct_ from other components on the page. In most cases this 
    /// variant behavior should mix nicely with the "defaults" the component exposes but
    /// if you want to also adjust the "defaults" then use either the `css_variables()` 
    /// function to define a global CSS definition for these components
    render_to_browser_with_inline_variables(&self, CssVariables) -> String;
}
```


## Output Targets


## Documentation

Once this feature has been implemented, we will need to create horizontal rule documentation for **Darkmatter**. In addition, `biscuit-terminal` will need to document the `BrowserRenderable` trait and the `HorizontalRule` component.

- darkmatter/docs/topics/horizontal-rules.md
- biscuit-terminal/docs/components/horizontal-rule.md
- biscuit-terminal/docs/components/browser-renderable-trait.md

Finally, once all other documentation has been written we need to ensure that the `biscuit-terminal` and `darkmatter` agent skills (`.claude/skills/{biscuit-terminal|darkmatter}`) defined in this monorepo are updated to reflect this functionality.
