---
description:
---
# Frontmatter in the Pipelining Process

## Frontmatter Discarded After Use

Frontmatter is an important part of the Markdown _pipelining_ process as it provides metadata to many stages of the pipeline transformation, however, at the completion of the process the default approach is to remove all Frontmatter from the composed output. The rationale of that is that the Frontmatter was there to help the composition/pipelining of content but once that's complete it is no longer needed.

## Frontmatter Propagation

When the CLI's `compose` or `publish` (note: future command) _subcommands_ are used to trigger the Markdown pipeline we have the option of providing a key/value dictionary to initialize the "state" of the pipeline. Throughout the pipelining process we use Frontmatter to represent a form of state and so when we move from any parent-to-child based transclusion we must pass along the parent's Frontmatter to the child's:

- the first time we see this propagation effect is when we kickoff with a key/value pair:
      - We must propagate this key/value pair into the base document we are composing
- every time there is a transclusion event we are also propagating the frontmatter from

### Exceptions to Propagation

There are few Frontmatter properties which are _excluded_ from propagation:

- `prologue` and `epilogue` are not propagated down to children; these are both involved in [Frontmatter Transclusion](../transclusion/fm-transclusion.md)
- `prompt` and `model` are used in the [Prompt Expansion]() functionality and are not propagated

## Finalization

We propagate frontmatter through the the graph of Markdown documents referenced in transclusion or otherwise. 
