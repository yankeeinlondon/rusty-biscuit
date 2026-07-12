---
hash: ef46db3751d8e999-19ddabcbba01b554
last_updated: 2026-07-12
---
# Darkmatter

<img src="../assets/darkmatter-512.png" style="width: 250px" />

- [Compose](./docs/topics/what-is-composition.md) documents together dynamically
- Render to [multiple output formats](./docs/topics/output-formats.md)
- Compose supports body `::shell` expansion, `::shell-block` / `::end-block` multi-command blocks, and top-level frontmatter `$(...)` shell expansion with shared approval and timeout controls
- Report on [differences/changes](./docs/topics/delta.md), TOC, graph dependencies, and more
- Provides shell auto-completions in the terminal (bash, elvish, fish, powershell, zsh) and the [DMLS language server](./dmls/README.md) in an editor.

## Packages

For details, choose one or more of the packages in this package area.

| Type                           | Package &nbsp;&nbsp;&nbsp; | Description                                                                                                                                                                                               |
| ------------------------------ | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [**Library**](./lib/README.md) | `darkmatter`               | Core library; follow the link for a much deeper functional and technical overview of what Darkmatter provides                                                                                             |
| [**CLI**](./cli/README.md)     | `darkmatter-cli`           | The Darkmatter CLI (binary: `md`); follow the link for a full description on how to use the CLI, what sub-commands exist, what CLI switches exist, example usage and how to get shell completions working |
| [**LSP**](./dmls/README.md)    | `dmls`                     | The Darkmatter Language Server (binary: `dmls`); LSP 3.17 over stdio for Markdown + the Darkmatter DSL — schema-aware frontmatter, wiki links, navigation, diagnostics, rename, formatting, semantic tokens |

## Documentation

- to get details on the **Composition Pipeline** in Darkmatter read: [Darkmatter Composition Pipeline](./docs/darkmatter-compose-pipeline.md)
- to get details on the **Rendering Pipeline** in Darkmatter read: [Darkmatter Render Pipeline](./docs/darkmatter-rendering-pipeline.md)
- for the **experimental, internal** Markdown-to-render-tree fold (does not affect `as_html` / `for_terminal`) read: [Render-Tree Fold](./docs/render-tree-fold.md)
- for more information on how to use CLI read: [Darkmatter CLI](./docs/cli/index.md)
- for shell expansion details read: [Body Shell Expansion](./docs/inline/shell-expansion.md), [Shell Blocks](./docs/inline/shell-blocks.md), and [Frontmatter Shell Expansion](./docs/inline/fm-shell-expansion.md)
- Other topics you may be interested in:
    - [What is Composition?](./docs/topics/what-is-composition.md)
    - [Transclusion](./docs/topics/transclusion.md)
    - [Rendering Output Formats](./docs/topics/output-formats.md)
    - [Delta Processing](./docs/topics/delta.md)
    - [Context Variables provided to Composition](./docs/topics/context-variables.md) — date/time, repo/monorepo, file changes, OS, hardware, and document discovery via `sniff`
    - [Error Rendering Conventions](./docs/errors/README.md) — `BlockError` body
      contract, `SourceContext`, snapshot tests

## License

AGPL-3.0-only