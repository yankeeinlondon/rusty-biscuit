# Link Resolve

The **Link Resolve** operation converts all local links in a document to absolute paths. This happens during the **Inline Pre** phase, ensuring that as documents move through the pipeline (especially during transclusion), their links remain correct regardless of where they are finally placed.

## Behavior

Link Resolve identifies all local path references and resolves them to their absolute physical location on disk.

- **Inputs:** Markdown content with relative links (e.g., `[Image](./assets/logo.png)`).
- **Outputs:** Markdown content with absolute links (e.g., `[Image](/Users/ken/project/docs/assets/logo.png)`).

## Supported Syntaxes

Link Resolve processes both standard Markdown and HTML reference syntaxes:

- **Markdown:** `[link](path)`, `![image](path)`
- **HTML Links:** `<a href="path">`, `<link href="path">`
- **HTML Media:** `<img src="path">`, `<video src="path">`, `<audio src="path">`, `<source src="path">`
- **HTML Frames:** `<iframe src="path">`

## Why Resolve to Absolute?

During **Transclusion**, child documents are pulled into a parent document. If the child document has a relative link like `[Readme](./README.md)`, and that child is transcluded into a parent in a different directory, the relative link would break if left as-is.

By resolving all links to absolute paths early in the pipeline:
1. Every document's links are anchored to their own source location.
2. The transclusion engine can safely move content without breaking references.
3. Downstream operations (like [Link Normalization](./link-normalization.md)) have a consistent absolute starting point to work from.

## Phase

- **Phase:** `Inline-Pre`
- **Order:** Runs after all other Inline-Pre operations, immediately before Transclusion.

## Source Files

- `darkmatter/lib/src/markdown/compose/link_resolve.rs`
