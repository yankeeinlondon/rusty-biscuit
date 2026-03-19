# Data Visualization

Data visualization is a really nice feature of the `biscuit-terminal` library. Today that is provided by MermaidJS's CLI to render Mermaid code blocks as an image. In this feature we are going to make two substantial improvements:

1. Instead of using the Javascript CLI like we do today, we will switch to using the `mermaid-rs` crate for much higher performance rendering and without an external dependency. We will keep the temp file based caching that we have unless there's a reason not to.
2. We are going to add the ability to visualize graph structures both programmatically and via a [DOT](https://graphviz.org/doc/info/lang.html) codeblock.

    ~~~md
    Look at my pretty graph:

    ```dot
    strict graph { 
        a -- b
        a -- b
        b -- a [color=blue]
      }
    ```
    ~~~~

## Better Packaging for more Reuse

Currently all of our Mermaid rendering resides in `biscuit-terminal` and since it was mainly the terminal where we need to render these diagrams it made sense to put it there historically. However, with the dependency-free and high performance nature of rendering using `mermaid-rs` there are definitely use-cases where rendering to an HTML output could make sense too. Now add on top that we're adding more visualizations with the addition of graph structures. 

It now makes sense to create an additional abstraction. 

- we have created a largely empty new _package area_ called `biscuit-visualized` which will be a library for these two visualizations
    - read the [README.md](@biscuit-visualized/README.md) for an overview
- the current core functionality for rendering Mermaid will:
    - will be moved to the `biscuit-visualized` package
    - `biscuit-terminal` will then add `biscuit-visualized` as a dependency to provide it's current Mermaid functionality available in `biscuit-terminal`
    - the implementation of Mermaid rendering will be converted from using the JS CLI to using `mermaid-rs`
        - the implementation will continue to support the current temporary file caching which we do (unless there is a good reason not to)
- the `biscuit-visualized` package will also implement our new graph expression visualizer and again `biscuit-terminal` will incorporate a `Renderable` component for displaying these visualizations.
    - that includes exposing a new subcommand to the biscuit-terminal-cli: `bt graph-expression <exp>`; 
    - and like we've done with all the Mermaid visualizations exposed via the CLI, we'll add a `--example` switch which can be added to allow the user to see an example easily of a working graph visualization

## Re-exporting?

One open question is whether `biscuit-terminal` should re-export the underlying API surface from `biscuit-visualized` or just expose the **Renderable** components used to render this into a terminal.

One likely new consumer of the `biscuit-visualized` library will be `darkmatter` so that should be given some weight in the design decision.

## Reference Material

Detailed information on graph visualizations can be found at:

- [Visualizing Graph Expressions](@biscuit-terminal/docs/data-visualization/visualizing-graph-expressions.md)
