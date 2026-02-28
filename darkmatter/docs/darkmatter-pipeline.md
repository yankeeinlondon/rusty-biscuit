# Darkmatter Processing Pipeline

## High Level Flow

```mermaid title="Pipeline Flow"
flowchart LR

  S1[Preparation]
  S2[Transclusion]
  S3[Output]

  S1 --> S2 --> S3
```

The most important part of the Darkmatter pipelining process is [transclusion](https://en.wikipedia.org/wiki/Transclusion) but before we do that we ensure that the base document is run through some preparation steps to get it ready for transclusion.

The **output** of the Markdown pipelining process is any of the following formats:

- Markdown
- HTML (_with inline JS and CSS_)
- AST (_[mdast](https://github.com/syntax-tree/mdast) based AST in JSON format_)
- Terminal rendering of Markdown (_e.g., using escape codes, inline image embeddings, etc._)

> **Note:** we may at some future stage add output formats like `PDF` and `Word`

The final stage of the Markdown processing will take the _composed_ output and render it to one of the supported output types.

### Operations Per Stage

| **Stage 1**: Preparation  | **Stage 2**: Transclusion |  **Stage 3**: Rendering    |
| -------------             | -------------             |  ---------------           |
| Text Replacement          | Block Transclusion        |  Table Rendering           |
| FM Interpolation          | Frontmatter Transclusion  |  YouTube Embedding         |
| TOC Linking               | Code Block Transclusion   |  Popover                   |
| Cleaning                  |                           |  List Expansion            |
| Normalization             | ▎AI: Prompt Expansion     |  Smart Image               |
|                           | ▎AI: Summarization        |  Image Rendering           |
|                           | ▎AI: Consolidation        |  Disclosure Blocks         |
|                           | ▎AI: Normalization        |  Block Columns             |
|                           |                           |  Audio Content             |
|                           |                           |  Charting                  |
|                           |                           |  Mermaid Rendering         |
|                           |                           |  TOC Generation            |

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
