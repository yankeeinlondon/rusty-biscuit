We originally had all of the image rendering, including Mermaid rendering, implemented in the `biscuit-terminal` package but since then we've introduced the `biscuit-visualized` package which takes ownership of all "data visualization" tasks.

Unfortunately when this separation was done, there were gaps in both documenting the changes as well as completing all of it. In this feature we will fix this.

Major changes which took place at the point of `biscuit-visualized` taking over some responsibilities include:

- all image generation for the terminal is done in `biscuit-visualized`
- rendering of Mermaid diagrams is done in `biscuit-visualized`
    - historically `biscuit-terminal` took on this responsibility
    - when `biscuit-terminal` did this it used **mmdc** (e.g, the Mermaid CLI) to render a raster image whereas now `biscuit-visualized` uses a much faster render path by leveraging `mermaid-rs-renderer` crate
- we introduced graph based data-visualizations at the same time that we created the `data-visualized` 
    - this leverages the `layout-rs` crate
    - these data visualizations are made available in `biscuit-terminal` (via library and CLI) but the implementation exists mainly in `
- while the initial implementation of `biscuit-visualized` was to service the needs of `biscuit-terminal`, one of the reasons for the separation in the first place is that we wanted it's functionality to be made available to a larger variety of consumers and target platforms (Terminal, HTML, and maybe more)
- `biscuit-terminal` introduces the `Renderable` trait and using that trait it produces a lot of useful components for rendering to the terminal. 
