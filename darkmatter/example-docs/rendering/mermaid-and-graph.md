# Rendering Mermaid and Graphs Diagrams

Darkmatter provides support for rendering both [Mermaid](https://mermaid.js.org/intro/) diagrams and [Object Graphs]() using **dot** syntax.

## Mermaid

Mermaid provides lots of different diagrams but the OG is the **flowchart**:

```mermaid title="Mermaid's Flowchart"
flowchart LR
    A(First)
    B(Second)
    Choice{ Feeling Lucky? }
    Wowsa(Wowsa)
    Whoopsie(Whoopsie)
    

    A -->|then| B
    B -->|and then| Choice

    Choice -->|yes| Wowsa
    Choice -->|no| Whoopsie
```

This support includes being able to ornament the diagrams like adding a **title** (_seen above_).
