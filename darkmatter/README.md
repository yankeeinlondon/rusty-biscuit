# Darkmatter

<img src="../assets/darkmatter-512.png" style="width: 250px" />

- [Compose](./docs/topics/what-is-composition.md) documents together dynamically
- Render to [multiple output formats](./docs/topics/output-formats.md)
- Compose supports both body `::shell` expansion and top-level frontmatter `$(...)` shell expansion with shared approval and timeout controls
- Report on [differences/changes](./docs/topics/delta.md), TOC, graph dependencies, and more
- Provides auto-completions via [shell completions](./docs/cli/completions.md) in the terminal and the [LSP](./lsp/README.md) in an editor.

## Packages

For details, choose one or more of the packages in this package area.

| Type | Package  &nbsp;&nbsp;&nbsp; | Description |
|---------|-------|-------------      |
| [**Library**](./lib/README.md) | `darkmatter` | Core library; follow the link for a much deeper functional and technical overview of what Darkmatter provides |
| [**CLI**](./cli/README.md) | `darkmatter-cli` | The Darkmatter CLI (binary: `md`); follow the link for a full description on how to use the CLI, what sub-commands exist, what CLI switches exist, example usage and how to get shell completions working |
| [**LSP**](./lsp/README.md) | `darkmatter-lsp` | **FUTURE:** A language server for Darkmatter (aka, Markdown + DSL) |

## Documentation

- to get details on the **Composition Pipeline** in Darkmatter read: [Darkmatter Composition Pipeline](./docs/darkmatter-compose-pipeline.md)
- to get details on the **Rendering Pipeline** in Darkmatter read: [Darkmatter Render Pipeline](./docs/darkmatter-rendering-pipeline.md)
- for more information on how to use CLI read: [Darkmatter CLI](./docs/cli/index.md)
- for shell expansion details read: [Body Shell Expansion](./docs/inline/shell-expansion.md) and [Frontmatter Shell Expansion](./docs/inline/fm-shell-expansion.md)
- Other topics you may be interested in:
    - [What is Composition?](./docs/topics/what-is-composition.md)
    - [Transclusion](./docs/topics/transclusion.md)
    - [Rendering Output Formats](./docs/topics/output-formats.md)
    - [Delta Processing](./docs/topics/delta.md)
    - [Context Variables provided to Composition](./docs/topics/context-variables.md)


## License

AGPL-3.0-only
