---
prompt: |-
	Your task is to research the most common LSP's which are used with the VSCode editor.
    
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

## Markdown

### 1. Marksman

Marksman is the most popular dedicated Markdown LSP. It provides completion, goto definition, find references, rename refactoring, and diagnostics. Notably, it supports **wiki-link** style references enabling Zettelkasten-like note-taking, in addition to standard inline and reference-style Markdown links. It works on macOS, Linux, and Windows as a self-contained binary written in F#.

- **Repo:** <https://github.com/artempyanykh/marksman>
- **Docs:** <https://github.com/artempyanykh/marksman/blob/main/docs/features.md>

### 2. markdown-oxide

A newer Markdown LSP written in Rust, focused on integrating with Obsidian-style vaults and PKM (Personal Knowledge Management) workflows. It provides link completion, references, and diagnostics with an emphasis on performance.

- **Repo:** <https://github.com/Feel-ix-343/markdown-oxide>
- **Docs:** <https://github.com/Feel-ix-343/markdown-oxide#readme>

---

## HTML

### 1. vscode-html-languageservice (Built-in to VSCode)

This is the HTML language service extracted from VSCode's built-in `html-language-features` extension. It powers HTML completion, hover, formatting, document links, document symbols, and folding ranges in VSCode out of the box. For Neovim and other editors, it is distributed as the `vscode-langservers-extracted` npm package, which bundles HTML, CSS, and JSON language servers together. The server itself is a library (not a standalone LSP binary) — the `vscode-langservers-extracted` package wraps it into a proper LSP server.

- **Repo:** <https://github.com/microsoft/vscode-html-languageservice>
- **Docs:** <https://github.com/microsoft/vscode-html-languageservice#readme>
- **Neovim package:** `vscode-langservers-extracted` on npm (provides `vscode-html-language-server`)

### 2. superhtml

A newer HTML LSP written in Zig, focusing on performance and correctness for HTML templating. Still emerging but gaining attention for its speed.

- **Repo:** <https://github.com/kristoff-it/superhtml>
- **Docs:** <https://github.com/kristoff-it/superhtml#readme>

---

## CSS

### 1. vscode-css-languageservice (Built-in to VSCode)

This is the CSS/LESS/SCSS language service extracted from VSCode's built-in `css-language-features` extension. It provides validation, completion, hover, definition, references, rename, color support, formatting, and folding ranges. Like the HTML server, it is bundled for non-VSCode editors via the `vscode-langservers-extracted` npm package.

- **Repo:** <https://github.com/microsoft/vscode-css-languageservice>
- **Docs:** <https://github.com/microsoft/vscode-css-languageservice#readme>
- **Neovim package:** `vscode-langservers-extracted` on npm (provides `vscode-css-language-server`)

### 2. Tailwind CSS Language Server

Not a general CSS LSP, but essential for projects using Tailwind CSS. Provides class name completion, hover previews, diagnostics, and CSS conflict detection. Ships as a standalone server and is widely used in both VSCode (official Tailwind extension) and Neovim.

- **Repo:** <https://github.com/tailwindlabs/tailwindcss-intellisense>
- **Docs:** <https://tailwindcss.com/docs/editor-setup>

---

## JavaScript

### 1. typescript-language-server

The primary LSP for JavaScript (and TypeScript) outside of VSCode. It wraps the `tsserver` component from Microsoft's TypeScript package with a thin LSP interface, making it accessible to any LSP-compatible editor. Notably, this project is **not** used by VSCode itself — VSCode uses a custom non-LSP extension called "TypeScript Language Features." Microsoft is currently developing [TypeScript 7](https://github.com/microsoft/typescript-go) in Go, which will include native LSP support and may eventually supersede this project.

- **Repo:** <https://github.com/typescript-language-server/typescript-language-server>
- **Docs:** <https://github.com/typescript-language-server/typescript-language-server#readme>
- **Install:** `npm install -g typescript-language-server typescript`

### 2. vtsls

An alternative that wraps the entire VSCode TypeScript extension (not just `tsserver`) in an LSP interface. This means it has feature parity with VSCode's built-in TypeScript experience, including features that `typescript-language-server` may not expose. Maintained by the yioneko project.

- **Repo:** <https://github.com/yioneko/vtsls>
- **Docs:** <https://github.com/yioneko/vtsls#readme>

---

## TypeScript

### 1. typescript-language-server

Same server as listed under JavaScript — it handles both `.js` and `.ts` files. It provides full TypeScript intelligence including type checking, completions, go-to-definition, references, rename, refactorings, organize imports, and inlay hints. Ships code actions for fix-all, remove unused imports, and add missing imports. The Neovim built-in LSP config calls this `ts_ls` (formerly `tsserver`).

- **Repo:** <https://github.com/typescript-language-server/typescript-language-server>
- **Docs:** <https://github.com/typescript-language-server/typescript-language-server/blob/master/docs/configuration.md>

### 2. vtsls

Same as listed under JavaScript — wraps the full VSCode TypeScript extension for maximum feature parity, including some TypeScript-specific features that the community LSP wrapper may not yet expose.

- **Repo:** <https://github.com/yioneko/vtsls>
- **Docs:** <https://github.com/yioneko/vtsls#readme>

---

## Rust

### 1. rust-analyzer

The standard LSP for Rust, developed under the `rust-lang` organization. It is **built into VSCode** as the recommended "rust-analyzer" extension. It provides go-to-definition, find-all-references, refactorings, code completion, integrated formatting via `rustfmt`, and diagnostics via `rustc` and `clippy`. Written entirely in Rust, it uses a demand-driven architecture (similar to IntelliJ) rather than invoking the compiler directly, which gives it excellent performance. It is structured as a set of reusable libraries for analyzing Rust code.

- **Repo:** <https://github.com/rust-lang/rust-analyzer>
- **Docs:** <https://rust-analyzer.github.io/book/>
- **Manual:** <https://rust-analyzer.github.io/book/>

---

## Golang

### 1. gopls (Official)

The official Go language server, pronounced "Go please," developed and maintained by the Go team at Google. It is the **standard and only production-quality LSP for Go.** Provides completion, hover, go-to-definition, references, rename, formatting, code lens, inlay hints, and diagnostics. Ships with the Go toolchain and is used by VSCode (via the official Go extension) and all other editors. Written in Go.

- **Repo:** <https://github.com/golang/tools/tree/master/gopls>
- **Docs:** <https://go.dev/gopls>

---

## PHP

### 1. Intelephense

The most popular PHP language server, providing high-performance PHP intelligence including completion, signature help, go-to-definition, references, workspace symbol search, diagnostics, formatting, hover, and rename. Notably, it has a **freemium model** — core features are free, but advanced features (rename across files, code folding, find implementations, go-to-type-definition, auto PHPDoc, smart select, code actions, type hierarchy, code lens, inlay hints, document links) require a paid licence key (one-time purchase, perpetual). The server binary itself is proprietary while the VSCode client extension is MIT-licensed.

- **Repo:** <https://github.com/bmewburn/vscode-intelephense>
- **Docs:** <https://github.com/bmewburn/intelephense-docs>
- **Website:** <https://intelephense.com>

### 2. phpactor

An open-source alternative PHP language server and IDE integration layer. Written in PHP, it provides completion, goto definition, references, code actions, and refactoring. Fully free and open source. Particularly popular among Neovim users who prefer an entirely open-source toolchain. It also offers a standalone RPC protocol beyond LSP for richer integrations.

- **Repo:** <https://github.com/phpactor/phpactor>
- **Docs:** <https://phpactor.readthedocs.io>

---

## Python

### 1. Pyright / basedpyright

**Pyright** is Microsoft's fast static type checker for Python, written in TypeScript. It powers the **Pylance** extension in VSCode (Pylance wraps Pyright with additional language service features). Pyright provides type checking, completions, hover, go-to-definition, references, and rename. It is **built into VSCode** via Pylance (which is the default Python language server in the official Python extension). For Neovim, the standalone `pyright` npm package or `basedpyright` (a community fork with stricter defaults and additional features) is used.

- **Repo:** <https://github.com/microsoft/pyright>
- **Docs:** <https://microsoft.github.io/pyright>
- **basedpyright fork:** <https://github.com/DetachHead/basedpyright>

### 2. Python LSP Server (pylsp)

The community-maintained Python Language Server Protocol implementation, successor to the original `python-language-server` (Palantir). It is the **default LSP** configured in many Neovim setups and supports a rich plugin architecture for additional features like rope (refactoring), pyflakes (linting), pycodestyle (style), pylint, and more. Written in Python. All-Round LSP with a broader scope than Pyright, but generally slower type inference.

- **Repo:** <https://github.com/python-lsp/python-lsp-server>
- **Docs:** <https://github.com/python-lsp/python-lsp-server#readme>

---

## Lua

### 1. lua-language-server (lua_ls)

The dominant Lua language server with nearly a million VSCode installs. Written in Lua itself, it supports **Lua 5.1 through 5.5 and LuaJIT**. Provides completion, hover, goto definition, find references, rename, diagnostics, formatting, spell checking, and dynamic type checking. Notably, it supports over 20 annotation types for documenting code and has a plugin system for extensibility. In Neovim, it is configured as `lua_ls` and is the standard LSP for Lua development as well as for Neovim's own configuration files.

- **Repo:** <https://github.com/LuaLS/lua-language-server>
- **Docs:** <https://luals.github.io/wiki>

---

## Quick Reference Table

| Language   | LSP                         | Built-in to VSCode?                | Neovim config name                 |
|------------|-----------------------------|------------------------------------|------------------------------------|
| Markdown   | Marksman                    | No                                 | `marksman`                         |
| HTML       | vscode-html-languageservice | Yes                                | via `vscode-langservers-extracted` |
| CSS        | vscode-css-languageservice  | Yes                                | via `vscode-langservers-extracted` |
| JavaScript | typescript-language-server  | No (VSCode uses tsserver directly) | `ts_ls`                            |
| TypeScript | typescript-language-server  | No (VSCode uses tsserver directly) | `ts_ls`                            |
| Rust       | rust-analyzer               | Yes (extension)                    | `rust_analyzer`                    |
| Go         | gopls                       | Yes (via Go extension)             | `gopls`                            |
| PHP        | Intelephense                | No (third-party extension)         | `intelephense`                     |
| Python     | Pyright                     | Yes (via Pylance)                  | `pyright`                          |
| Lua        | lua-language-server         | No (third-party extension)         | `lua_ls`                           |
