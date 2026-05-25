When we talk about page composition we should talk about it from two levels of granularity:

- Page
- Component

## The Page

The Page's composition -- using [`HtmlPage` struct](@renderable/src/mod.rs) -- is relatively crude in it's approach but that's appropriate because it's looking at things through a macro lens. The way it's modeled currently it just has a vector of [`BrowserFragment`](@renderable/src/browser.rs) components. You could imagine that a Page's fragments might often just be a series of `Section` components (note: the `Section` component does not exist yet but imagine that it's a container for large sections of content with an optional heading level and allowing for nested sections which would structurally represent a more fine grained set of sections).

### Example #1

- so in our simple example we'll discuss a document that has three main section called:
    - Problem Statement
    - Impact
    - Solution Approaches
- let's imagine that `Solution Approaches` is further broken down by the following subsections:
    - Approach 1
    - Approach 2
    - Approach 3

In this example, the **Page** is only aware of the top level sections so it's fragments property would consist of: `[ Section, Section, Section ]`. It literally doesn't need to known anything more than the body of the page it's rendering can be composed by iterating over the vector and calling the `render()` function.

- What is actually rendered per fragment is the responsibility of the component
- I have shown the fragments as consisting of three sections but the Page doesn't care and it's visibility is solely that these are fragments that have a render function.

Once the page's body has been constructed, the composition of the HEAD section of the page needs to be considered:

- The end goal is to be able to present:
    - a single "stylesheet" (aka, a set of class names with CSS definitions for the page)
        - the stylesheet will be composed from:
            - the aggregate demand from the fragments the page contains 
            - as well as any page-level demands which were brought in at render time with the [`PageOptions`](@renderable/src/browser/mod.rs) struct.
        - the fragments will use the [`ComponentStylesheet` struct](@renderable/src/browser/mod.rs) to define their requirements and one of the desirable side effects of this approach is that all classes defined are automatically using a descendant based scope convention which "namespaces" the classes it can define under a base class name for the component:
            - that means that so long as every component has a unique base class name then component's should not collide in their CSS definitions
            - it would make sense for the **BrowserFragment** to have a `validate()` function which can be called that will ensure among other things that:
                - if the 
        - to start we can assume this stylesheet will always be an inline spreadsheet but in the future we can build in PageOptions which allow for it to be either inline or a file reference
    - if any Javascript is needed for this page to operate correctly then we will want to roll up and dedup all the demands from the contained fragments and merge them with any page-level demands for javascript
        - just like with the stylesheet, to start we should assume that this is just inline javascript but over time we'll introduce ways to configure that in [`PageOptions`](@renderable/src/browser/mod.rs)

## The Component

While a page views the fragments it contains completely generically, components have the benefit of being able to think in terms of how they should be laid out structurally/semantically, etc.

We have completely revamped the [`BrowserFragment`](@renderable/src/browser/fragment.rs) design to provide a type strong solution where composition of HTML elements and sub-components is made explicit.
