# Merman Rendering

Currently biscuit-terminal and biscuit-visualized use the [mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer.git) crate to render Mermaid diagrams to SVG or raster images but it appears that [Merman](https://github.com/Latias94/merman) is likely the better option:

- it is much more up-to-date
- it has the Zed editor's developers backing Merman

This specification is about:

1. doing a spike where we can compare all the mermaid diagrams from both renderers
2. we then decide if Merman is going to replace mermaid-rs-renderer
3. if it is, then we need to evaluate and optimize the themes that it uses so it renders in a way to that is more like what Zed's preview editor shows then what Merman renders out of the box
4. complete the cutover to using Merman everywhere

To aid in this work, we have done extensive research that should be used to get deeper understanding of the two crates as well as what Zed is doing on top of Merman:

- [mermaid-rs-renderer research](biscuit-visualized/docs/research/mermaid/mermaid-rs-renderer.md)
- [Merman research](biscuit-visualized/docs/research/mermaid/merman.md)
- [Zed's abstraction over Merman](biscuit-visualized/docs/research/mermaid/zed-merman.md)
- [Mermaid JS](biscuit-visualized/docs/research/mermaid/mermaid-js.md)

## Completion

We are done once we've successfully run the spike comparison and chosen a path forward:

- if we choose Merman then:
    - we will have a full implementation for Merman 
    - which includes work on the CSS/styling to make it look as good as it can
    - we also need to make sure we have a solid perf test for the Merman rendering that can be compared against the `mermaid-rs-renderer` baseline
- if we choose to stay with `mermaid-rs-renderer` then we simply roll back any changes that we may have during the spike comparison
