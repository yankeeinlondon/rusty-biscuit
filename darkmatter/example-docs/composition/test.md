---
epilogue: "---\n\n- No [animals](./animals.md) were hurt in the preparation of this document"
---
# Testing Composition

<img src="https://site.com/logo.png" />

This is a basic test of _composition_ through the Markdown pipeline we use in Darkmatter. The overall flow is:

```mermaid
flowchart LR
    S1(Preparation)
    S2(Transclusion)
    S3(Rendering)

    S1 --> S2 --> S3
```

::file ./preparation.md

::file ./what-is-transclusion.md exclude="## Secret Section"

## Conditional Disclosure

::file ./disclosure-cc.md when="env.AGENT == 'claude'"
::file ./disclosure-oc.md when="env.AGENT == 'opencode'"
::file ./disclosure.md when="!env.AGENT"

## Some Other Things

::toc-linking ./other.md
