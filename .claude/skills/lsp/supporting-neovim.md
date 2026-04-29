---
prompt: |-
	Your task is to research the most common LSP's which are used with the neovim editor.
    
    - languages analyzed should include:
        - Markdown
        - HTML
        - CSS
        - Javascript
        - Typescript
        - Rust
        - Golang
        - PHP
        - Python
        - Lua
    - For each language:
        - discuss the 1-2 most popular LSP's used to support Neovim
        - if this LSP is "built-in" mention that
        - if there is anything notable about this LSP implementation mention that
        - document the main repo and documentation URLs for each LSP

last_updated: 2026-04-16
---
# Common LSPs for Neovim

## Markdown

### 1. Marksman

The dominant Markdown LSP for Neovim. Written in F#, it provides completion, goto definition, find references, rename refactoring, and diagnostics. It supports standard Markdown links, reference links, and wiki-link style references for Zettelkasten-like note taking. Works in both single-file and multi-file (workspace) modes.

- **Repo:** <https://github.com/artempyanykh/marksman>
- **Docs:** <https://github.com/artempyanykh/marksman/blob/main/docs/features.md>

### 2. Markdown Oxide (markdown-oxide)

A newer Markdown LSP with a focus on personal knowledge management and obsidian-compatible features. Supports wiki-links, tags, and daily notes.

- **Repo:** <https://github.com/Feel-ix-343/markdown-oxide>

## HTML

### 1. vscode-html-language-server

The standard HTML language server, extracted from VSCode's built-in HTML support and distributed via the `vscode-langservers-extracted` npm package. Provides completion, hover, diagnostics, document symbols, formatting, and document links. Supports embedded JavaScript and CSS within HTML.

- **Repo:** <https://github.com/hrsh7th/vscode-langservers-extracted>
- **Upstream:** <https://github.com/microsoft/vscode-html-languageservice>

## CSS

### 1. vscode-css-language-server

The standard CSS/LESS/SCSS language server, extracted from VSCode and distributed via the `vscode-langservers-extracted` npm package. Provides completion (properties, values, selectors), hover documentation, diagnostics, formatting, document symbols, color presentation, and rename refactoring.

- **Repo:** <https://github.com/hrsh7th/vscode-langservers-extracted>
- **Upstream:** <https://github.com/microsoft/vscode-css-languageservice>
- **Docs:** <https://github.com/microsoft/vscode-css-languageservice/blob/main/docs>

### 2. tailwindcss-language-server

Essential for projects using Tailwind CSS. Provides autocomplete, hover previews, diagnostics, and CSS conflict detection for Tailwind utility classes.

- **Repo:** <https://github.com/tailwindlabs/tailwindcss-intellisense>

## JavaScript

### 1. typescript-language-server

The most widely used JavaScript/TypeScript LSP in the Neovim ecosystem. It wraps the TypeScript `tsserver` API with a standard LSP interface, originally based on the VSCode TypeScript extension. Not directly associated with Microsoft. Supports completion, hover, goto definition, references, rename, code actions, organize imports, inlay hints, and refactoring. Requires a TypeScript installation in the workspace or globally.

- **Repo:** <https://github.com/typescript-language-server/typescript-language-server>
- **Docs:** <https://github.com/typescript-language-server/typescript-language-server/blob/master/docs/configuration.md>

### 2. Deno LSP

Built into the Deno runtime. Provides full LSP support for JavaScript and TypeScript without requiring a `node_modules` or `tsconfig.json` setup. Notable for its zero-config approach and built-in formatting/linting. Ideal for Deno projects.

- **Repo:** <https://github.com/denoland/deno>
- **Docs:** <https://docs.deno.com/runtime/getting_started/setup_your_environment/>

## TypeScript

### 1. typescript-language-server

(Same as JavaScript above.) This is the standard choice for TypeScript in Neovim. It leverages the same `tsserver` that powers VSCode's TypeScript extension, providing full type-checking, completion, and refactoring capabilities.

- **Repo:** <https://github.com/typescript-language-server/typescript-language-server>

### 2. Deno LSP / vtsls

Deno's built-in LSP also handles TypeScript natively. Additionally, **vtsls** is a newer alternative that wraps the VSCode TypeScript extension directly (without the intermediary `tsserver` custom protocol), offering better feature parity with VSCode's TypeScript experience.

- **vtsls Repo:** <https://github.com/yioneko/vtsls>

## Rust

### 1. rust-analyzer

The official Rust language server, developed under the `rust-lang` organization. Written entirely in Rust, it does not invoke `rustc` for IDE features; instead it implements its own compiler frontend optimized for latency. Provides code completion, goto definition, find references, rename, code actions, inlay hints, formatting (via rustfmt), diagnostics (via rustc and clippy), and macro expansion. Ships as part of the Rust toolchain via `rustup component add rust-analyzer`.

- **Repo:** <https://github.com/rust-lang/rust-analyzer>
- **Docs:** <https://rust-analyzer.github.io/book/>
- **Notable:** Pre-installed with Rust via rustup. Implements its own parser and type inference rather than delegating to `rustc`.

## Golang

### 1. gopls

The official Go language server (pronounced "Go please"), developed and maintained by the Go team at Google. The only mainstream LSP for Go. Provides completion, goto definition, find references, rename, code actions, formatting, diagnostics, and refactoring. Installed via `go install golang.org/x/tools/gopls@latest`.

- **Repo:** <https://github.com/golang/tools/tree/master/gopls>
- **Docs:** <https://go.dev/gopls>
- **Notable:** The single official LSP for Go. No meaningful alternatives exist.

## PHP

### 1. Intelephense

The most popular PHP language server. A high-performance PHP code intelligence server with fast workspace indexing. Supports completion, signature help, goto definition, find references, diagnostics, document formatting, hover, and rename. Advanced features (code actions, go to implementation, go to type definition, smart select, inlay hints, code lens) require a paid licence key, though the free tier covers most essential IDE features.

- **Repo:** <https://github.com/bmewburn/vscode-intelephense>
- **Docs:** <https://github.com/bmewburn/intelephense-docs>
- **Notable:** Freeware with premium features. The server itself is proprietary; the VSCode client is MIT licensed.

### 2. phpactor

An open-source alternative PHP language server focused on refactoring and code intelligence. Supports completion, goto definition, references, rename, and code actions. Written in PHP and fully open source (MIT licence). Popular with users who prefer a fully free solution.

- **Repo:** <https://github.com/phpactor/phpactor>
- **Docs:** <https://phpactor.readthedocs.io/>

## Python

### 1. Pyright (and basedpyright)

Microsoft's static type checker for Python, which includes a full LSP server (`pyright-langserver`). Provides fast type checking, completion, hover, goto definition, references, rename, and code actions. Written in TypeScript for high performance. **basedpyright** is a community fork that adds extra features from Microsoft's proprietary Pylance extension (e.g., inline values, semantic highlighting) while remaining open source.

- **Pyright Repo:** <https://github.com/microsoft/pyright>
- **Pyright Docs:** <https://microsoft.github.io/pyright/>
- **basedpyright Repo:** <https://github.com/DetachHead/basedpyright>
- **Notable:** Pyright is the basis for VSCode's Pylance extension. basedpyright is the recommended choice for Neovim users wanting maximum feature parity with VSCode.

### 2. Python LSP Server (pylsp)

The community-maintained Python LSP, originally a fork of Palantir's `python-language-server`. Has a plugin architecture that integrates Jedi (completion/definitions), Rope (refactoring), pyflakes, pycodestyle, flake8, pylint, autopep8, black, and more via optional extras. A good choice for users who want a fully Python-native, extensible, and configuration-driven LSP.

- **Repo:** <https://github.com/python-lsp/python-lsp-server>
- **Docs:** <https://github.com/python-lsp/python-lsp-server/blob/develop/CONFIGURATION.md>

## Lua

### 1. lua-language-server (LuaLS)

The dominant Lua language server, often still referred to by its original author's handle "sumneko". Written in Lua and C++, it supports Lua 5.1 through 5.4 and LuaJIT. Provides completion, goto definition, find references, rename, diagnostics, hover, formatting, and an extensive annotation system for type documentation. With nearly a million VSCode installs, it is by far the most feature-rich Lua LSP. Particularly important for Neovim users since Neovim's config is written in Lua.

- **Repo:** <https://github.com/LuaLS/lua-language-server>
- **Docs:** <https://luals.github.io/wiki/>
- **Notable:** Self-hosted (written in Lua). Provides comprehensive Neovim API type definitions via its annotation system, making it essential for Neovim plugin and config development.

### 2. emmylua_ls

A newer Lua language server written in Rust, based on the EmmyLua analyzer. Aims for better performance and is gaining adoption, particularly for larger Lua codebases.

- **Repo:** <https://github.com/CppCXY/emmylua-analyzer-rust>
