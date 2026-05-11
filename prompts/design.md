---
spec: ""
dir: ""
success:
    say: "The technical design has been completed"
    message: "The technical design has been completed for {{spec}}"
failure:
    message: "The technical design for {{spec}} failed!"
---

The specification file -- {{spec}} -- has been created as a functional description of a new feature we want to develop. Your task is to provide a detailed technical design that **compliments** this specification.

- use the '{{ctx.current_package_area}}' skill during this design
- make sure to reference the specification file in body of the design document
- save the design to '{{dir}}/tech-design.md'
- make sure the content in the tech design is well formed, idiomatic Markdown (CommonMark + GFM)
- if you want to create data visualizations then use a **mermaid** code block
- if the specification has a lot of technical detail do not duplicate and be sure not contradict design goals setup in the specification
    - Things which are often valuable additions to the design process when the specification has a lot of technical content include:

        1. Module dependency graph — visualizing how provider/ interacts with existing modules (especially the lib/CLI boundary)
        2. Error type hierarchy — defining McpError, ConfigError variants without bloating the functional spec
        3. Testing patterns — how to test trait implementations (mock providers? test fixtures?)
        4. Performance notes — memory layout of ProviderInfo, v-table overhead specifics, &'static lifetime guarantees
        5. Deprecation mechanics — exact #[deprecated] attributes, timeline, and consumer migration guide
              - NOTE: deprecations are rarely needed in Rusty Biscuit repo right now because the repo is not yet being used by a large ecosystem although there are library dependencies where a deprecated solution might make sense as an early phase of a project but in general we want to avoid the deprecation baggage for now.
