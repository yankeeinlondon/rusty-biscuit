---
feature: "@renderable/features/2026-05-20-darkmatter-tree"
prompt: |-
    Before we kickoff the Darkmatter movement toward using the new [tree-rendering]() architecture we need to
    Decide what pulldown-cmark options the tree path is allowed to use relative to the legacy renderers.

    This should happen first because it affects parity expectations. If the fold enables task lists, footnotes, superscript, or subscript while
    legacy renderers do not, then “parity failure” may actually be an intentional behavior expansion.
    
    ## Deliverable

    Write mini-design spec to the body of this Markdown document that classifies each option as public now, tree experimental only, or deferred. This mini-design is meant to compliments the existing {{ feature }}/spec.md
---
