# Mermaid Diagrams

Mermaid are a very popular way of rendering diagrams/charts in Markdown content. They are typically rendered inside of a `mermaid` code block and while the original chart type was a **flowchart** there are now tons of different types of diagrams to choose from.

## JS Based Mermaid Rendering

::toc-linking ./mermaid-js.md 

## Chart Types

- Flowchart
- Swimlanes Diagram
- Sequence Diagram
- Class Diagram
- State Diagram
- Entity Relationship Diagram
- User Journey
- Gantt
- Pie Chart
- Quadrant Chart
- Requirement Diagram
- Use Case Diagram
- GitGraph (Git) Diagram
- C4 Diagram
- Mindmaps
- Timeline
- ZenUML
- Sankey
- XY Chart
- Block Diagram
- Packet
- Kanban
- Architecture
- Radar
- Event Modeling
- Treemap
- Venn
- Ishikawa
- Wardley
- Cynefin
- TreeView

## Rust Mermaid Rendering

We have been using the [`mermaid-rs-renderer`](https://github.com/1jehuang/mermaid-rs-renderer) crate for rendering Mermaid diagrams but it's falling a bit behind in supporting some of the newer visualizations and we've found that [Merman](https://github.com/Latias94/merman) is used by the [Zed](https://zed.dev/) editor for it's Mermaid renders and looks to be a more maintained library.

### [`mermaid-rs-renderer`](https://github.com/1jehuang/mermaid-rs-renderer.git)

::toc-linking ./mermaid-rs-renderer.md

### [Merman](https://github.com/Latias94/merman.git)

::toc-linking ./merman.md

#### Zed's use of Merman

::toc-linking ./zed-merman.md
