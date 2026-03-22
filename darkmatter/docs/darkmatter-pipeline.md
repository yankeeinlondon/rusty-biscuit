# Darkmatter Processing Pipeline (e.g., Composition)

## High Level Flow



| Stage 1: Inline Mutation                                  | Stage 2: Transclusion                                            |  **Stage 3**: Rendering                                |
| -------------                                             | -------------                                                    |  ---------------                                       |
| [1.1 Text Replacement 🏁](./inline/text-replacement.md)    | [Block Transclusion 🏁](./transclusion/block-transclusion.md)     |  [Table Rendering](./rendering/table-rendering.md)     |
| [1.2 Page Blocks 🏁](./inline/page-blocks.md)             | [Frontmatter Transclusion 🏁](./transclusion/fm-transclusion.md)  |  [YouTube Embedding](./rendering/youtube-embedding.md) |
| [1.3 TOC Linking 🏁](./inline/toc-linking.md)              | [Code Block Transclusion 🏁](./transclusion/code-transclusion.md) |  [Popover](./rendering/popover.md)                     |
| [1.4 Shell Expansion 🏁](./inline/shell-expansion.md)      |                                                                   |  [List Expansion](./rendering/list-expansion.md)                 |
| [1.5 Interpolation 🏁](./inline/interpolation.md)          | [AI Prompt Expansion](./transclusion/prompt-expansion.md)         |  [Smart Image](./rendering/smart-image.md)               |
|                                                           | [AI Summarization](./transclusion/summarization.md)             |  [Image Rendering 🏁](./rendering/image-rendering.md)           |
|  ---                                                      | [AI Consolidation](./transclusion/consolidation.md)             |  [Disclosure Blocks](./rendering/disclosure.md)         |
|                                                           |                                                                |  [Block Columns](./rendering/block-columns.md)             |
|                                                           |                                                                |  [Audio Content](./rendering/audio-content.md)             |
| [2.1 Cleaning 🏁](./inline/cleaning.md)                           |                                                                |                            |
| [2.2 Normalization 🏁](./inline/normalization-and-releveling.md) |                                                                |  [Mermaid Rendering 🏁](./rendering/mermaid.md)         |
|                                                           |                                                                |  [TOC Generation](./rendering/toc-generation.md)            |
|                                                           |                                                                |  [Person Card](./rendering/person.md)               |
|                                                           |                                                                |  [Place Card](./rendering/place.md)                |
|                                                           |                                                                |  [Product Card](./rendering/product.md)              |

> **Note:** items marked with `🏁` are implemented


## Stages

### Inline Mutation

What defines the _inline mutation_ group is that the updates on a document are isolated to the document at hand. In this group, however,
we have two subset groups:

1. Pre Transclusion
2. Post Transclusion

For the **Pre Transclusion** group, these operations are run in a serial process, one operation after another. During these early steps we
have the potential for one operation to _setup_ or _effect_ the next operation. Rather then be a side effect, this is intentional and often adds useful power to the
pipeline.

> As a example, if a conditional page block is evaluated to _false_ (aka, do not render this page), the identifying this first means any
shell expansion commands (or any other inline mutation) contained within the block will now be ignored because this part of the page has
been removed.

In contrast the **Post Transclusion** group of operations -- which are also run serially -- are focused on finalizing output and structure
into the most valid form we can deterministically reach.

#### Pre Ops

- [Text Replacement](./inline/text-replacement.md) - when `replace` property in frontmatter is a key/value dictionary we will replace all instances of the _keys_ with the _values_ in the body of the document
- [Page Blocks](./inline/page-blocks.md) - allow for blocks in the page to be defined, often with _conditional_ logic to determine whether the block should be rendered or removed       
- [Interpolation](./inline/interpolation.md) - looks for handlebars template markers in the page's body and replaces the template markers with data from frontmatter, ENV variables, or [context variables](./topics/context-variables.md).
- [TOC Linking](./inline/toc-linking.md) - links to a referenced page's Heading tags rather than just the page/document      
- [Shell Expansion](./inline/shell-expansion.md) - allows _approved_ commands to be run and have the STDOUT replace the directive
- [Link Validation](./inline/link-validation.md) - looks at all of the linked references on the base page -- that includes both hyperlinks _and_ image references -- and makes sure all are valid.

#### Post Ops

- [Cleaning](./inline/cleaning.md) - makes the markdown as standard bearing and consistent as possible                    
- [Normalization](./inline/normalization-and-releveling.md) - ensures that the heading structure is valid and fixes where it is not

### Transclusion

The **transclusion** stage is typified by recursive operations which have the potential to be time consuming (and more dependent on 
[caching](./topics/caching.md)) then those found in the inline steps. Not all operations are expensive -- for instance the most common 
transclusion directive is the `::file <ref>` directive which points to another local Markdown document. Assuming the document it references
doesn't have it's own transclusions this operation will be lightning fast and no slower than any of the the inline mutation operations.
However, even in this example, we don't know how expensive the operation is until 

### Rendering

- mutates the document for one of the [supported output formats](./topics/output-formats.md)


## Ordering and Concurrency

The macro flow for execution is as follows:

```mermaid title="Pipeline Flow"
flowchart LR

  InlinePre["Inline (pre)"]
  Transclusion[Transclusion]
  InlinePost["Inline (post)"]
  Render[Rendering]

  InlinePre --> Transclusion --> InlinePost --> Render
```

