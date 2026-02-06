# Darkmatter Processing Pipeline

| Stage 1          | Stage 2       | Stage 3        |  Stage 4          |
| -------------    | ------------- | -------        | ---------------   |
| Interpolation    | Transclusion  | Summarization  | Table Rendering   |
| Text Replacement | Charting      | Consolidation  | YouTube Embedding |
| Mermaid Rendering|               | Normalization  | Popover           |
| Image Rendering  |               |                | List Expansion    |
|                  |               |                | Smart Image       |
|                  |               |                | Disclosure Blocks |
|                  |               |                | Block Columns     |
|                  |               |                | Audio Content     |



## Variance by Output Target

The kind of output, as well as the kinds of processing done depends in part on what our target output is. Outputs are:

1. Markdown
2. HTML
3. Terminal

### Markdown-to-Markdown

This is the most common use case for AI prompting. It's utility is in the ability to compose documents together at "runtime" and generate the prompt on the fly.

### Markdown-to-Terminal

This can be useful for CLI apps to provide rich rendering to the console that doesn't suck.

### Markdown-to-HTML

Rendering to HTML (aka, the Browser) is deploying to the most feature-rich platform and therefore there are a large number of features that only operate on this output target. This can be useful for producing web content and documentation for users. It _could_ be used for AI too but in general AI tends to like Markdown better.


