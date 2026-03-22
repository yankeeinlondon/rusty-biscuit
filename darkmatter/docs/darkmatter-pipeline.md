# Darkmatter Processing Pipeline (e.g., Composition)

## High Level Flow

```mermaid title="Pipeline Flow"
flowchart LR

  S1[Inline Mutation]
  S2[Transclusion]
  S3[Rendering]

  S1 --> S2 --> S3
```


| Stage 1: Inline Mutation                                  | Stage 2: Transclusion                                            |  **Stage 3**: Rendering                                |
| -------------                                             | -------------                                                    |  ---------------                                       |
| [Text Replacement 🏁](./inline/text-replacement.md)       | [Block Transclusion 🏁](./transclusion/block-transclusion.md)     |  [Table Rendering](./rendering/table-rendering.md)     |
| [Interpolation 🏁](./inline/interpolation.md)             | [Frontmatter Transclusion 🏁](./transclusion/fm-transclusion.md)  |  [YouTube Embedding](./rendering/youtube-embedding.md) |
| [TOC Linking 🏁](./inline/toc-linking.md)                 | [Code Block Transclusion 🏁](./transclusion/code-transclusion.md) |  [Popover](./rendering/popover.md)                     |
| [Shell Expansion 🏁](./inline/shell-expansion.md)         |                                                                   |  [List Expansion](./rendering/list-expansion.md)                 |
|                                                           | [AI Prompt Expansion](./transclusion/prompt-expansion.md)         |  [Smart Image](./rendering/smart-image.md)               |
|                                                           | [AI Summarization](./transclusion/summarization.md)             |  [Image Rendering 🏁](./rendering/image-rendering.md)           |
|                                                           | [AI Consolidation](./transclusion/consolidation.md)             |  [Disclosure Blocks](./rendering/disclosure.md)         |
|                                                           |                                                                |  [Block Columns](./rendering/block-columns.md)             |
|                                                           |                                                                |  [Audio Content](./rendering/audio-content.md)             |
| [Cleanup 🏁](./inline/cleaning.md)                           |                                                                |                            |
| [Normalization 🏁](./inline/normalization-and-releveling.md) |                                                                |  [Mermaid Rendering 🏁](./rendering/mermaid.md)         |
|                                                           |                                                                |  [TOC Generation](./rendering/toc-generation.md)            |
|                                                           |                                                                |  [Person Card](./rendering/person.md)               |
|                                                           |                                                                |  [Place Card](./rendering/place.md)                |
|                                                           |                                                                |  [Product Card](./rendering/product.md)              |

> **Note:** items marked with `🏁` are implemented

- **Inline Mutation** 
    - updates/mutates the current document but without using content from external sources
- **Transclusion**
    - update/mutates the current document by injecting external documents or assets into the current document
- **Rendering**
    - mutates the document for one of the [supported output formats](./topics/output-formats.md)

