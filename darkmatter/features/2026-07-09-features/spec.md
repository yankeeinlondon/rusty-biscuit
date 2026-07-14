# Style Features

When we designed the "style" system for Darkmatter we built a set of CSS-like configuration for certain block items like tables, code-blocks, block quotes, and more. This grammar allows for some
useful configuration of stylistic rendering but one thing that was designed but not yet realized is the idea of a "feature" which a renderable component could declare. A feature can specify it's dependencies on:

- Javascript
- CSS, including CSS variables
- other?

## Multi Target Output

A feature is meant to be output target aware. It must know that if the output target is Markdown that no Javascript can be used. It knows that terminal output can render some but not all CSS and in areas where a `px`/`rem`/etc. unit is used in the CSS that it is downsampled to CSS that will work in the terminal (e.g., `ch`).

## A Feature Example

Mermaid charts are really useful and so we support rendering them in both the terminal and in the browser. However, the approach to rendering in these different targets is very different:

1. **Terminal** - we can't render with Javascript so instead we statically build the image and embed the image inline into the render output
2. **Markdown** - mermaid is left as a `mermaid` code block
3. **MarkdownPlus** - we could either embed the image inline or just keep the `mermaid` code block
4. **Browser** 
    - here two we have a choice but by default we would render the code block as HTML and add the MermaidJS as inline Javascript.
        - letting Mermaid be rendered by Javascript in the browser enables some dynamic features that can't be supported with a static image
        - the more diagrams used the less "bytes" will be needed loaded into the browser to get the visual representation we desire (initially the Javascript code _might_ make the payload larger but )
        - 

## Deduplication

## Implementation Targets

To ensure that our implementation of features has legs we will implement the following features:

- mermaid feature
- popover feature (CSS only)
