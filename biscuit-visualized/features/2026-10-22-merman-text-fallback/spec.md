---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the clarify.md prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
clarified: false
reviewed: false
review_iterations: 0
implemented: false
depends-on:
    - 2026-10-21-merman
related:
    - 2026-10-21-merman
references:
    biscuit-visualized/docs/research/mermaid/merman.md: >-
        Research on Merman, including its Unicode/ASCII output and the coverage
        recorded in Merman's `docs/rendering/ASCII_SUPPORT_MATRIX.md`.
---

# Merman text fallback

> **Draft.** Must go through clarification, so every ruling is made, before
> planning.

## Goal

Use Merman's Unicode/ASCII text output as the fallback for terminals that
cannot display images, for the diagram types where Merman's text output is
complete. Today that fallback shows the Mermaid source as a code block
(`2026-10-21-merman`, D20 there).

## Evidence

Merman renders some families to Unicode or ASCII text. Its own coverage table,
`docs/rendering/ASCII_SUPPORT_MATRIX.md` in the Merman repository, is
summarized in the [Merman research](../../docs/research/mermaid/merman.md):

- flowcharts are only "Partial": icons, images, callbacks, links, and some
  shapes are unsupported;
- mindmap and treeView render as outlines;
- ASCII output does not render icons.

`2026-10-21-merman` kept the code-block fallback and scheduled this work as a
follow-up (D48 there).

## Owners

| Area | Responsibility |
| --- | --- |
| `biscuit-visualized` | Rendering diagrams to text through Merman |
| `biscuit-terminal` | Selecting the text fallback for terminals that cannot display images |

## Scope to clarify

Clarification must settle which diagram types use the text fallback and how
"complete" is judged for each against Merman's support matrix; the quality
bar a text rendering must meet before it replaces the code block; how the text
fallback relates to the existing code-block fallback (which one a parse error,
an unsupported family, or a partially supported diagram gets, and whether the
source stays reachable); and how text output is tested, including goldens.
