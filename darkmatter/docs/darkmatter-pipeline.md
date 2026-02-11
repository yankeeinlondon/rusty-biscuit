# Darkmatter Processing Pipeline

| **Stage 1**: Preparation  | **Stage 2**: Early Composition | Stage 3: AI Mut & Gen |  Stage 4: Optimization |
| -------------             | -------------                  | -------               | ---------------        |
| Cleaning                  | Transclusion                   | Summarization         | Table Rendering        |
| FM Interpolation          |                                | Consolidation         | YouTube Embedding      |
|           |                                | Normalization         | Popover                |
| Text Replacement          |                                |                       | List Expansion         |
|                           |                                |                       | Smart Image            |
|                           |                                |                       | Image Rendering        |
|                           |                                |                       | Disclosure Blocks      |
|                           |                                |                       | Block Columns          |
|                           |                                |                       | Audio Content          |
|                           |                                |                       | Charting               |
|                           |                                |                       | Mermaid Rendering      |


## Variance by Output Target

The kind of output, as well as the kinds of processing done depends in part on what our target output is. Outputs are:

1. Markdown
2. HTML
3. Terminal

### Markdown-to-Markdown

This is the most common use case for AI prompting. It's utility is in the ability to compose documents together at "runtime" and generate the prompt on the fly.

- While it's very unlikely that AI use cases would ever benefit from transforms which result in some inline HTML (Markdown of course does support this)
- It is beneficial to human consumption so long as you be assured that the inline HTML support would

### Markdown-to-Terminal

This can be useful for CLI apps to provide rich rendering to the console that doesn't suck.

- we leverage crates like `two-face` and `syntect` to get a good theming solution which can work with code highlighting as well as just prose
- we also benefit from the ability to borrow functionality from the `biscuit-terminal` application for rendering tables, columns, etc.

### Markdown-to-HTML

Rendering to HTML (aka, the Browser) is deploying to the most feature-rich platform and therefore there are a large number of features that only operate on this output target. This can be useful for producing web content and documentation for users. It _could_ be used for AI too but in general AI tends to like Markdown better.

## Features to Output Target Mapping

| Feature                                    | Status | HTML | MD | Term |
| ----------------                           | -------| ---- | -- | ---- |
|[Cleaning](./cleaning.md)                   | ✅     |   -  | ✅ |   -  |
|[Interpolation](./interpolation.md)         | -      |  ✅  | ✅ |   ✅ |
|[Text Replacement](./text-replacement.md)   | -      |  ✅  | ✅ |   ✅ |
|[Transclusion](./transclusion.md)           | -      |  ✅  | ✅ |   ✅ |
|[Summarization](./summarization.md)         | -      |  ✅  | ✅ |   ✅ |
|[Consolidation](./consolidation.md)         | -      |  ✅  | ✅ |   ✅ |
|[Normalization](./Normalization.md)         | -      |  ✅  | ✅ |   ✅ |
|[Charting](./charting.md)                   | -      |  ✅  |    |      |
|[Mermaid Rendering](./mermaid-rendering.md) | ✅     |  ✅   | -  |  ✅  |


