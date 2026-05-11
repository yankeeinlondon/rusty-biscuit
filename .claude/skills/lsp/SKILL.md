---
name: lsp
description: Expert knowledge for the Language Server Protocol (LSP), Markdown language servers, editor-specific LSP choices for VS Code and Neovim, and Rust or TypeScript libraries for building or extending LSP implementations. Use when researching LSP architecture, choosing an LSP for an editor, comparing Markdown LSPs, or planning a new LSP implementation.
---

## Purpose

Use this skill when the task is about:

- Understanding LSP history, architecture, message flow, or common protocol gotchas
- Comparing Markdown language servers and their editor ecosystems
- Choosing common LSP implementations for VS Code or Neovim by language
- Building or extending an LSP in Rust
- Building or extending an LSP in TypeScript or JavaScript

This skill is primarily a navigation layer over the research documents in this folder. Read only the files that match the task.

## Document Map

- [lsp.md](./lsp.md): LSP history, specification versions, architecture, lifecycle, capability negotiation, and protocol gotchas
- [LSP Features](./features.md): provides a comprehensive set of features which an LSP can provide to an editor
- [IWES LSP](./iwes.md): details on IWES, a high performance Rust LSP
- [Tower LSP](./tower-lsp.md): details on the `tower-lsp` crate; one of the more popular LSP choices in Rust ecosystem
- [markdown-lsps.md](./markdown-lsps.md): Major Markdown LSP implementations, editor ecosystem fit, implementation languages, and comparison matrix
- [rust-crates.md](./rust-crates.md): Rust crate landscape for LSP servers, decision guide, simple example, and extension strategies
- [typescript-libraries.md](./typescript-libraries.md): TypeScript/JavaScript LSP library options, simple example, and extension strategies
- [supporting-vscode.md](./supporting-vscode.md): Common LSP choices for VS Code across Markdown, HTML, CSS, JS, TS, Rust, Go, PHP, Python, and Lua
- [supporting-neovim.md](./supporting-neovim.md): Common LSP choices for Neovim across Markdown, HTML, CSS, JS, TS, Rust, Go, PHP, Python, and Lua


## Which Document To Read

- For protocol background or implementation pitfalls, start with [lsp.md](./lsp.md).
- For Markdown-specific server selection, start with [markdown-lsps.md](./markdown-lsps.md).
- For Rust implementation work, read [rust-crates.md](./rust-crates.md) and then [lsp.md](./lsp.md) if protocol detail is needed.
- For TypeScript or JavaScript implementation work, read [typescript-libraries.md](./typescript-libraries.md) and then [lsp.md](./lsp.md) if protocol detail is needed.
- For editor recommendations, read the editor-specific guide first:
    - VS Code: [supporting-vscode.md](./supporting-vscode.md)
    - Neovim: [supporting-neovim.md](./supporting-neovim.md)
- For Markdown inside a specific editor, combine [markdown-lsps.md](./markdown-lsps.md) with the relevant editor guide.

## Recommended Workflow

1. Identify whether the task is about protocol fundamentals, editor support, Markdown tooling, or implementation libraries.
2. Load only the matching research file or pair of files.
3. If the task involves building a server, combine:
   - [lsp.md](./lsp.md)
   - one implementation-language guide:
     - [rust-crates.md](./rust-crates.md), or
     - [typescript-libraries.md](./typescript-libraries.md)
4. If the task involves choosing an existing server, combine:
   - [markdown-lsps.md](./markdown-lsps.md) for Markdown-specific comparisons, or
   - [supporting-vscode.md](./supporting-vscode.md) / [supporting-neovim.md](./supporting-neovim.md) for editor-specific choices

## Notes

- These documents are research summaries, not generated API references.
- Release versions and maintenance status can drift. If the task depends on the latest package or server status, verify upstream before making a final dependency recommendation.
