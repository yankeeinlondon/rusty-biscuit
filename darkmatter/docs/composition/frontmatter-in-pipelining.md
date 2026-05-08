---
description:
---
# Frontmatter in the Pipelining Process

## Frontmatter Discarded After Use

Frontmatter is an important part of the Markdown _pipelining_ process as it provides metadata to many stages of the pipeline transformation, however, at the completion of the process the default approach is to remove all Frontmatter from the composed output. The rationale is that the Frontmatter was there to help the composition/pipelining of content but once that's complete it is no longer needed.

The `--frontmatter` (or `--fm`) flag on `md compose` overrides this behavior, including frontmatter in the output. This is useful for pipeline workflows where the composed document's frontmatter needs to be further manipulated (e.g., via `md set`).

## Frontmatter State Initialization

When the CLI's `compose` subcommand is used to trigger the Markdown pipeline, the `--state` flag provides **default values** as a JSON or JSON5 dictionary:

- Null or missing frontmatter keys are filled in from `--state`.
- Existing non-null frontmatter values are preserved (document wins).
- This "default-fill" semantic means `--state` cannot override intentional values in the document.

```bash
# Given frontmatter: { stage: "plan", feature: null }
md compose doc.md --state '{feature: "auth", stage: "build"}'
# Result: stage stays "plan", feature becomes "auth"
```

## Frontmatter Propagation

Throughout the pipelining process we use Frontmatter to represent a form of state and so when we move from any parent-to-child based transclusion we must pass along the parent's Frontmatter to the child's:

- the first time we see this propagation effect is when we kickoff with `--state`:
      - We must propagate this key/value pair into the base document we are composing
- every time there is a transclusion event we are also propagating the frontmatter from

### Exceptions to Propagation

There are few Frontmatter properties which are _excluded_ from propagation:

- `prologue` and `epilogue` are not propagated down to children; these are both involved in [Frontmatter Transclusion](../transclusion/fm-transclusion.md)
- `prompt` and `model` are used in the [Prompt Expansion]() functionality and are not propagated

## Finalization

The **finalization** stage runs only on the root document after all other composition and transclusion has completed. Operations in this stage (like [Link Normalization](../operations/link-normalization.md)) see the fully-composed body but are still driven by the root document's configuration.
