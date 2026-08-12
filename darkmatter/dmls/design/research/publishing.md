---
prompt: |-
  DMLS is an LSP for Darkmatter which targets any editor which supports a modern LSP implementation but pay particular attention in supporting:

  1. VSCode
  2. [Zed](https://zed.dev/docs/extensions/languages)
  3. Neovim
  4. Helix

  Your task is to research how the use of DMLS with these editors can be made as simplified as possible. This includes publishing the LSP to any plugin catalog that the editor provides.

  Note: in some cases there might not be a formal global catalog (neovim for instance) but we should allow for an easy way to bring this LSP into scope when using the popular Lazy loader and the new built-in loader in version 13 of neovim.
last_updated: 2026-07-08
hash: 5600d3fceaba7c0f-0621fce5bcde9b63
---
# DMLS Editor Distribution Research

DMLS should be distributed as one native `dmls` binary plus thin editor integrations. The binary remains the product; editor plugins should only locate, install, update, configure, and launch it over stdio.

## Recommended Distribution Model

Publish DMLS in these channels:

| Channel                         | Purpose                                                                                                                          |
|---------------------------------|----------------------------------------------------------------------------------------------------------------------------------|
| GitHub Releases                 | Canonical cross-platform binary archives for macOS, Linux, and Windows. Required by Zed and useful for editor-managed downloads. |
| crates.io                       | Rust-native install path: `cargo install dmls`.                                                                                  |
| cargo-binstall                  | Fast binary install path for Rust users without a local build.                                                                   |
| Homebrew                        | Lowest-friction macOS/Linux install: `brew install dmls`.                                                                        |
| WinGet and/or Scoop             | Lowest-friction Windows install.                                                                                                 |
| VS Code Marketplace             | Primary VS Code/Cursor install path.                                                                                             |
| Open VSX                        | VSCodium, Theia, Gitpod, and other Open VSX-based editors.                                                                       |
| Zed Extension Registry          | Primary Zed install path.                                                                                                        |
| nvim-lspconfig                  | Community standard Neovim server definition.                                                                                     |
| Mason Registry                  | Popular Neovim binary installer path.                                                                                            |
| dmls.nvim                       | Optional first-party Neovim convenience plugin for Lazy and `vim.pack`.                                                          |
| Helix upstream `languages.toml` | Best Helix discoverability.                                                                                                      |

The GitHub Release asset names should stay stable and machine-resolvable:

```text
dmls-<version>-macos-universal.tar.gz
dmls-<version>-linux-x86_64.tar.gz
dmls-<version>-linux-aarch64.tar.gz
dmls-<version>-windows-x86_64.zip
```

Each archive should contain:

```text
dmls / dmls.exe
README.md
LICENSE
checksums.txt or a detached checksum file in the release
```

## VS Code

VS Code should get a first-party extension published to both the Visual Studio Marketplace and Open VSX.

The extension should be intentionally thin:

- Activate for `markdown`, `mdx` if DMLS supports it later, and any future `darkmatter` language ID.
- Start `dmls` with `vscode-languageclient/node`.
- Prefer an explicit `dmls.server.path` setting.
- Fall back to `PATH`.
- If not found, offer to download the matching GitHub Release binary into extension-managed storage.
- Validate downloads by checksum.
- Expose commands:
    - `DMLS: Restart Server`
    - `DMLS: Show Server Output`
    - `DMLS: Select Server Binary`
    - `DMLS: Install or Update Server`

- Publish one normal extension package first. If binary size or platform behavior becomes painful, split into platform-targeted VSIX packages later.

Publishing steps:

```sh
npm install -g @vscode/vsce
vsce package
vsce publish
```

Open VSX should be published from the same VSIX metadata:

```sh
npx ovsx publish
```

VS Code’s docs identify `vsce` as the official packaging/publishing tool and note that the Marketplace supports publishing installable VSIX packages. Open VSX uses its own namespace/token flow and `ovsx` CLI.

Sources:

- <https://code.visualstudio.com/api/working-with-extensions/publishing-extension>
- <https://github.com/eclipse-openvsx/openvsx/wiki/Publishing-Extensions>

## Zed

Zed requires a first-party extension. Settings alone are not enough for registering an arbitrary stdio language server.

Use the existing `darkmatter/dmls/zed-dmls` scaffold as the basis for a separate public `zed-dmls` repository. The extension should:

- Implement `zed_extension_api::Extension`.
- Register `dmls` under `[language_servers.dmls]`.
- Check `PATH` first.
- Respect a user-configured binary path.
- Download the native `dmls` release binary only when needed.
- Store downloaded binaries in Zed’s extension work directory.
- Never embed the DMLS binary inside the WASM extension.

Minimal manifest shape:

```toml
id = "dmls"
name = "DMLS"
version = "0.1.0"
schema_version = 1

[language_servers.dmls]
name = "Darkmatter Language Server"
languages = ["Markdown"]
```

Publishing is via PR to `zed-industries/extensions`:

1. Add the public extension repo as a submodule under `extensions/dmls`.
2. Add an `[dmls]` entry to `extensions.toml`.
3. Run `pnpm sort-extensions`.
4. After merge, Zed packages and publishes it to the Zed extension registry.

Zed’s docs explicitly require language-server extensions to download/check for external language servers rather than shipping them inside the extension.

Sources:

- <https://zed.dev/docs/extensions/languages>
- <https://zed.dev/docs/extensions/developing-extensions>
- <https://zed.dev/docs/extensions/installing-extensions>

## Neovim

Neovim has no single global plugin catalog, so DMLS should support three paths.

### Native Config

Document the zero-plugin setup:

```lua
vim.lsp.config('dmls', {
  cmd = { 'dmls' },
  filetypes = { 'markdown' },
  root_markers = { '.dmls.toml', '.git' },
})

vim.lsp.enable('dmls')
```

Neovim’s current docs describe `vim.lsp.config()` and `vim.lsp.enable()` as the native configuration path, with config files loadable from `lsp/<name>.lua`.

### nvim-lspconfig

Submit `lsp/dmls.lua` to `neovim/nvim-lspconfig`.

That gives users:

```lua
vim.lsp.enable('dmls')
```

after installing `nvim-lspconfig`, and lets LazyVim and other distributions inherit the server definition.

### dmls.nvim

Create a tiny first-party plugin repository for users who want one-line setup and automatic binary management.

Recommended plugin layout:

```text
dmls.nvim/
  lua/dmls/init.lua
  lsp/dmls.lua
  plugin/dmls.lua
  doc/dmls.txt
```

Lazy setup:

```lua
{
  'rusty-biscuit/dmls.nvim',
  ft = 'markdown',
  opts = {
    auto_enable = true,
  },
}
```

Built-in `vim.pack` setup:

```lua
vim.pack.add({
  { src = 'https://github.com/rusty-biscuit/dmls.nvim' },
})

require('dmls').setup({
  auto_enable = true,
})
```

The user mentioned Neovim version 13; the relevant upstream mechanisms are already documented as native LSP config in Neovim 0.11+ and `vim.pack` in Neovim 0.12+. If DMLS targets a future Neovim 0.13 baseline, keep the docs phrased around those APIs rather than the exact release number.

### Mason

Add DMLS to Mason’s registry so users can install the binary with `:MasonInstall dmls`. This is separate from LSP configuration but is important for practical Neovim adoption.

Sources:

- <https://neovim.io/doc/user/lsp/>
- <https://neovim.io/doc/user/pack/>
- <https://github.com/neovim/nvim-lspconfig>

## Helix

Helix should work with plain `languages.toml` and should not require a plugin.

Document the manual configuration:

```toml
[language-server.dmls]
command = "dmls"

[[language]]
name = "markdown"
language-servers = ["dmls"]
```

Then submit an upstream Helix PR adding `dmls` to the built-in language-server catalog. The best user experience is:

1. Install `dmls` through Homebrew, cargo-binstall, crates.io, WinGet, Scoop, or a GitHub Release archive.
2. Use the built-in Helix Markdown language config where possible.
3. Add a small `.helix/languages.toml` only when selecting DMLS over another Markdown server.

Helix’s docs state that language servers are configured in `languages.toml`, and that the server command must be on `PATH`.

Source:

- <https://docs.helix-editor.com/languages.html>

## Priority Order

1. Ship stable GitHub Release binaries and checksums.
2. Publish crates.io and cargo-binstall metadata.
3. Publish VS Code extension to Marketplace and Open VSX.
4. Publish Zed extension through `zed-industries/extensions`.
5. Add DMLS to `nvim-lspconfig`.
6. Add DMLS to Mason.
7. Publish `dmls.nvim` for Lazy and `vim.pack`.
8. Add DMLS to Helix’s upstream language server configuration.
9. Add Homebrew, WinGet, and Scoop once release assets are stable.

## Success Criteria

A user should be able to reach a working DMLS setup with these flows:

```sh
# VS Code / Cursor
code --install-extension rusty-biscuit.dmls
```

```text
Zed: Extensions -> DMLS -> Install
```

```lua
-- Neovim with Lazy
{ 'rusty-biscuit/dmls.nvim', ft = 'markdown', opts = {} }
```

```lua
-- Neovim native
vim.lsp.config('dmls', { cmd = { 'dmls' }, filetypes = { 'markdown' }, root_markers = { '.dmls.toml', '.git' } })
vim.lsp.enable('dmls')
```

```toml
# Helix
[language-server.dmls]
command = "dmls"

[[language]]
name = "markdown"
language-servers = ["dmls"]
```

The lowest-friction path is not the same for every editor, but the invariant should be: install the editor integration, open Markdown, and DMLS starts without the user learning LSP internals.
