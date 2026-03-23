# What is Composition?

A dictionary for the word **composition** would be something like:

- _the action of putting things together; formation or construction_
- _a thing composed of various elements_

These definitions _align_ with what we mean by composition in [**Darkmatter**](../../README.md), but when we narrow the domain down to just Darkmatter we can be more specific:

- Our primary _compositional element_ is a **Markdown document**
- Using the **DSL** (_**D**omain **S**pecific **L**anguage_) that [**Darkmatter**](../../README.md) provides, we compose from a set of _operations_ or _directives_ (both terms are used)
    - The operations in a Darkmatter composition fit into one of three categories:
        - **Inline Mutation** - _mutates the current document in some manner_
        - **Transclusion** - _incorporates an external document or asset into the current document_
        - **Rendering** - _mutates the content to a specific [output format](./output-formats.md)_
- When we _compose_ something, that something always starts out as a Markdown document
- If the Markdown content we're composing is just regular old Markdown content with no DSL operations/directives that doesn't mean that there's nothing to do. We will always run these two operations:
    - [`clean`](../preparation/cleaning.md) - makes sure that the Markdown is as well formed as possible and improves consistency of things like vertical spacing, indent spacing, etc.
    - [`normalize`](../preparation/normalization-and-releveling.md) - makes sure the headings structure of the document is valid and corrects it

    We may also run some other operations based on the Markdown document's Frontmatter and Body content; for instance:

    - [`text-replacement`](../preparation/text-replacement.md) will be used if the frontmatter of the page has a frontmatter property called `replace` which is defined as a key/value dictionary.

All of the operations we've mentioned so far are in the _inline mutation_ category but things become more interesting when we consider [transclusion](./transclusion.md) based operations. Transclusion operations change the "structure" of a composition from an atomic mutation of a document to a recursive graph of documents collaborating to create a final document.

Transclusion operations include:

- [Block Transclusion](../transclusion/block-transclusion.md)
- [Frontmatter Transclusion](../transclusion/fm-transclusion.md)
- [Code Block Transclusion](../transclusion/code-transclusion.md)
- [Consolidation Transclusion](../transclusion/consolidation.md)
- [Summary Transclusion](../transclusion/summary.md)
- _and more_

Review the [Darkmatter Pipeline](../darkmatter-compose-pipeline.md) for a full list of operations.
