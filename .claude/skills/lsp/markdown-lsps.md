---
prompt: |-
	Your task is to research the major implementations of Markdown LSP's and write your findings to the body of this document.
    
    - What are the most commonly used LSP's for Markdown used in:
        - VSCode
        - Neovim
        - other editors
    - Are there any notable differences between these implementations?
    - What programming languages were used for these implementations?
    - What software library was used in the development of these implementations?

last_updated: 2026-04-16
---
# Markdown LSPs

## Overview

There are four major Markdown Language Server Protocol (LSP) implementations, each with different primary audiences, feature sets, and technical foundations. Three are listed on the official [LSP Implementations page](https://microsoft.github.io/language-server-protocol/implementors/servers/); a fourth (markdown-oxide) has emerged as a popular PKM-focused alternative.

## Major Implementations

### 1. VSCode Markdown Language Server (Microsoft)

- **Repository**: [microsoft/vscode-markdown-languageserver](https://github.com/microsoft/vscode-markdown-languageserver)
- **Language Service Library**: [microsoft/vscode-markdown-languageservice](https://github.com/microsoft/vscode-markdown-languageservice)
- **Primary Editor**: VSCode (built-in, bundled with the editor)
- **Implementation Language**: TypeScript
- **LSP Library**: [vscode-languageserver-node](https://github.com/microsoft/vscode-languageserver-node) (Microsoft's official Node.js LSP SDK)
- **Parser**: Delegates Markdown parsing to the client via a custom `markdown/parse` request; clients are expected to use [Markdown-it](https://github.com/markdown-it/markdown-it)
- **License**: MIT
- **Status**: Still in alpha (v0.5.0-alpha.12 as of July 2025), but actively used by VSCode's built-in Markdown extension. Explicitly noted as "not yet tested with other clients."

**Features**:

- Completions for Markdown links
- Folding ranges (regions, block elements, header sections)
- Smart selection (expand selection)
- Document symbols (headers)
- Workspace symbols
- Document links (clickable spans)
- Find all references (headers, links across workspace)
- Go to definition (links to headers / link definitions)
- Rename (headers and links across workspace)
- Code actions (organize link definitions, extract link to definition)
- Pull diagnostics (link validation: reference, fragment, file, unused/duplicate definitions)
- Link updating on file rename/move
- Custom requests for file system operations (readFile, readDirectory, stat, watcher)

**Notable design decisions**:

- Uses a client-server architecture where the client provides the Markdown parser. This allows VSCode to use its own Markdown-it configuration and extensions.
- Requires custom protocol messages (`markdown/fs/*`, `markdown/parse`) that non-VSCode clients must implement.
- Targets CommonMark strictly; does not support wiki-links or non-standard Markdown extensions.
- The core logic is factored into the separate `vscode-markdown-languageservice` npm package, making it theoretically reusable as a library outside the LSP server.

### 2. Marksman

- **Repository**: [artempyanykh/marksman](https://github.com/artempyanykh/marksman)
- **Stars**: ~3,100
- **Primary Editors**: Neovim, Helix, Emacs, VSCode, Sublime Text, Kakoune, Zed, BBEdit
- **Implementation Language**: F# (~97%), with some C# (~2.4%)
- **LSP Library**: Custom LSP implementation (found in the `LanguageServerProtocol/` directory of the repo)
- **Parser**: Custom Markdown parser built on patches to [Markdig](https://github.com/xoofx/markdown) (a .NET Markdown library), located in `MarkdigPatches/`
- **License**: MIT
- **Distribution**: Self-contained native binary for macOS, Linux, and Windows

**Features**:

- Document symbols from headings
- Workspace symbols from headings (subsequence-based matching)
- Completion for inline links, reference links, and wiki-links
- Hover preview for links
- Go to definition for links
- Find references for headings and links
- Diagnostics for wiki-links (broken references, duplicate/ambiguous headings)
- Rename refactoring
- Code Lens with reference counts on headings
- Table of Contents code action
- Multi-folder workspace support
- Single-file mode for editing files outside of a project
- Wiki-link support (`[[link]]`, `[[link#heading]]`) for Zettelkasten-style note taking
- Configurable via `.marksman.toml` (title-from-heading, completion style, ignore patterns)
- Reads `.gitignore`, `.hgignore`, `.ignore` for file exclusion

**Notable design decisions**:

- Focused on note-taking and PKM (Personal Knowledge Management) workflows, particularly Zettelkasten/Obsidian-style linking.
- Wiki-link support is a first-class feature, unlike the Microsoft server.
- Custom parser allows fine-grained document structure analysis beyond what standard Markdown ASTs provide.
- Self-contained binary distribution (no Node.js or runtime dependency).
- Explicit support for single-file mode when files are opened outside a project root.

### 3. Markdown Oxide

- **Repository**: [Feel-ix-343/markdown-oxide](https://github.com/Feel-ix-343/markdown-oxide)
- **Stars**: ~2,000
- **Primary Editors**: Neovim, VSCode, Zed, Helix, Kakoune
- **Implementation Language**: Rust (~93.5%), with TypeScript for editor extensions (~5.8%)
- **LSP Library**: [tower-lsp](https://github.com/ebkalderon/tower-lsp) (Rust LSP framework built on the Tower ecosystem)
- **Parser**: [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) (Rust CommonMark parser) plus custom parsing
- **License**: Apache-2.0
- **Documentation site**: [oxide.md](https://oxide.md)

**Features**:

- Full LSP feature set: completion, hover, go to definition, find references, rename, diagnostics
- PKM-focused features:

    - Daily notes with natural language date parsing (`:Daily two days ago`, `:Daily next monday`)
    - Code Lens for reference counts
    - Backlinks navigation
    - Obsidian vault compatibility (`.obsidian` root detection)
    - Block-level referencing

- Configurable via `.moxide.toml`
- Dynamic registration support for file watching
- Create Unresolved File code action

**Notable design decisions**:

- Strongly positioned as a PKM/Obsidian-compatible language server.
- The most actively developed of the Rust-based Markdown LSPs.
- Available via many package managers: Homebrew, Arch Linux pacman, Nix, Alpine apk, openSUSE zypper, Conda, Winget, cargo install/binstall.
- Bundles VSCode and Zed extensions for easy installation.
- Uses `dynamicRegistration` for workspace file watching, which requires client-side support.

### 4. Markmark

- **Repository**: [nikku/markmark](https://github.com/nikku/markmark)
- **Stars**: ~19
- **Primary Editor**: Any LSP-compatible editor (generic)
- **Implementation Language**: JavaScript (100%)
- **LSP Library**: [vscode-languageserver-node](https://github.com/microsoft/vscode-languageserver-node)
- **Parser**: Custom
- **License**: MIT
- **Distribution**: npm package (`npm install -g markmark`), exposes `markmark-lsp` binary

**Features**:

- Go to definition
- Find references
- Complete links and tags
- Validate links
- Project awareness (workspace root scanning)
- Built-in or external file watching support
- Also usable as a standalone library (not just as an LSP server)

**Notable design decisions**:

- Minimal, lightweight implementation.
- Designed to be used both as an LSP server and as an embeddable library via `new Markmark()`.
- Supports Zettelkasten-style workflows.
- Least adoption of the four implementations, but notable for its simplicity and dual-use design.

## Comparison Matrix

| Feature                   | VSCode LS                     | Marksman              | Markdown Oxide          | Markmark                   |
|---------------------------|-------------------------------|-----------------------|-------------------------|----------------------------|
| **Language**              | TypeScript                    | F#                    | Rust                    | JavaScript                 |
| **LSP SDK**               | vscode-languageserver-node    | Custom                | tower-lsp               | vscode-languageserver-node |
| **Parser**                | Markdown-it (client-provided) | Markdig (patched)     | pulldown-cmark          | Custom                     |
| **Wiki-links**            | No                            | Yes                   | Yes                     | Yes                        |
| **Completion**            | Yes (links)                   | Yes (links, wiki)     | Yes (links, wiki, tags) | Yes (links, tags)          |
| **Go to Def**             | Yes                           | Yes                   | Yes                     | Yes                        |
| **Find Refs**             | Yes                           | Yes                   | Yes                     | Yes                        |
| **Rename**                | Yes                           | Yes                   | Yes                     | No                         |
| **Diagnostics**           | Yes (pull)                    | Yes (wiki-links)      | Yes                     | Yes (links)                |
| **Hover**                 | Yes (images/video)            | Yes (links)           | Yes                     | No                         |
| **Folding**               | Yes                           | No                    | No                      | No                         |
| **Smart Select**          | Yes                           | No                    | No                      | No                         |
| **Document Symbols**      | Yes                           | Yes                   | Yes                     | No                         |
| **Workspace Symbols**     | Yes                           | Yes                   | Yes                     | No                         |
| **Code Actions**          | Yes                           | Yes (ToC)             | Yes                     | No                         |
| **Code Lens**             | No                            | Yes (ref count)       | Yes (ref count)         | No                         |
| **Daily Notes**           | No                            | No                    | Yes                     | No                         |
| **Obsidian Compat**       | No                            | Partial               | Yes                     | No                         |
| **Link Validation**       | Yes (granular config)         | Yes (wiki-links)      | Yes                     | Yes                        |
| **Single-file Mode**      | No                            | Yes                   | No                      | No                         |
| **Link Update on Rename** | Yes                           | No                    | No                      | No                         |
| **Distribution**          | Bundled with VSCode           | Self-contained binary | Binary + extensions     | npm package                |
| **Runtime Deps**          | Node.js                       | None                  | None                    | Node.js                    |

## Editor Ecosystem Summary

### VSCode

VSCode ships with Microsoft's own `vscode-markdown-languageserver` as the built-in Markdown language server. Additionally, users can install:

- **Marksman VSCode** extension for wiki-link and PKM support.
- **Markdown Oxide** extension for Obsidian-compatible PKM features.

### Neovim

Neovim does not bundle a Markdown LSP. The community primarily uses:

- **Marksman** (via [mason.nvim](https://github.com/williamboman/mason.nvim) or [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig)): The most established option with broad feature coverage.
- **Markdown Oxide** (via mason.nvim or cargo install): Growing rapidly for PKM/Obsidian users, especially with daily notes and backlinks.

Both are well-supported in the Neovim ecosystem. Marksman has broader adoption historically; markdown-oxide is the newer, fast-growing alternative with deeper PKM features.

### Other Editors

- **Helix**: Supports both Marksman and Markdown Oxide out of the box (binary must be on PATH).
- **Emacs**: Marksman via LSP Mode or Eglot.
- **Kakoune**: Both Marksman and Markdown Oxide via [kakoune-lsp](https://github.com/kakoune-lsp/kakoune-lsp).
- **Sublime Text**: Marksman via [LSP-marksman](https://github.com/sublimelsp/LSP-marksman).
- **Zed**: Both Marksman and Markdown Oxide available as extensions.

## Key Takeaways

1. **The Microsoft server is the most feature-complete for standard Markdown** (CommonMark), with folding, smart select, link-on-rename, and granular diagnostics configuration. However, it is tightly coupled to VSCode's architecture and requires custom protocol messages that non-VSCode clients must implement.
2. **Marksman and Markdown Oxide are the go-to choices for non-VSCode editors** and for PKM/note-taking workflows. Both support wiki-links and Obsidian-style linking. Marksman is F# with a custom parser; Markdown Oxide is Rust using pulldown-cmark and tower-lsp.
3. **The two most common LSP SDKs** are Microsoft's `vscode-languageserver-node` (used by the Microsoft server and Markmark) and Rust's `tower-lsp` (used by Markdown Oxide). Marksman implements its own LSP protocol layer.
4. **Parser choices vary significantly**: Markdown-it (JS, client-provided), Markdig (.NET, patched), pulldown-cmark (Rust), and custom parsers. Each implementation chose a parser aligned with its implementation language.
5. **No single server covers all use cases**. The Microsoft server excels at standard Markdown in VSCode; Marksman and Markdown Oxide excel at cross-editor PKM workflows with wiki-links.
