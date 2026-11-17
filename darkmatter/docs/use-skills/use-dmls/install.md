# Installing DMLS

Every setup has two parts:

1. a native `dmls` binary on your machine;
2. an editor told to launch it, over stdio, for Markdown files.

VS Code and Zed also need a small extension that does only the launching.

## 1. Install the binary

```bash
# from darkmatter/ in the rusty-biscuit repo (the canonical recipe)
just install-dmls

# equivalent, from the repo root
cargo install --path darkmatter/dmls --force

dmls --version
```

Without a checkout, you can use a release archive built by `just dist`, for
example `dmls-<version>-macos-universal.tar.gz` or `…-windows-x86_64.zip`.
Put `dmls` (`dmls.exe` on Windows) on your `PATH`.

`just install-dmls` also sets up Zed when Zed is installed (see below).
Re-run it after pulling, so the editor uses a binary that matches the
source. Then restart the language server.

### Server flags

| Flag | Effect |
|---|---|
| `--log-level error\|warn\|info\|debug\|trace` | how much to log (default `info`) |
| `--log-file <path>` | append logs to a file instead of stderr |
| `--config <path>` | use this `.dmls.toml` and skip discovery |
| `--stdio` | accepted and ignored (some clients always pass it) |
| `--version` | print the version |
| `--bench-index <dir>` | index a folder with no editor attached; a quick health check |

stdout carries the LSP protocol, so never point a terminal at a running
server's stdout.

## 2. Connect your editor

### VS Code

```bash
just install-vscode-package     # from darkmatter/: packages and installs vscode-dmls
```

Then run **Developer: Reload Window**. The extension has two settings:

- `dmls.server.path`: an absolute path to the binary. Set it when VS Code
  was launched from the GUI, usually to `~/.cargo/bin/dmls`.
- `dmls.server.args`: extra arguments, for example
  `["--log-level", "debug"]`.

The extension sends no other settings, so put configuration in
`.dmls.toml`. VS Code applies semantic tokens automatically. To mute
Darkmatter syntax so prose stands out:

```json
"editor.semanticTokenColorCustomizations": {
  "enabled": true,
  "rules": {
    "*.interpolation": { "foreground": "#7d8590" },
    "*.directive":     { "foreground": "#7d8590" },
    "*.closer":        { "foreground": "#6e7681" },
    "macro.wiki":      { "foreground": "#7d8590" },
    "string.wiki":     { "foreground": "#539bf5" }
  }
}
```

### Zed

Zed needs an extension: a small WASM module that launches the native binary.

1. Run `just install-dmls` from `darkmatter/`. When Zed's data directory
   exists, it stages the extension outside the checkout, registers it as a
   dev extension, and then runs a check.
2. Restart Zed and open a Markdown file. `dmls` should be listed, and green,
   in the language-server menu in the bottom bar.

If registration couldn't be done automatically, the output prints a
directory. Select it with **command palette → `zed: install dev extension`**.
Never select `vscode-dmls`, or a checkout or worktree folder. To diagnose
problems, run `just zed-doctor`.

Zed ships with semantic tokens turned off. To turn them on:

```json
{ "languages": { "Markdown": { "semantic_tokens": "combined" } } }
```

If Zed starts **no** language servers for a project at all, the cause is
usually your shell's init file blocking. Zed runs your login shell without a
terminal to capture its environment. Guard interactive-only setup with
`[ -t 0 ] || [ -t 1 ]`.

### Neovim (0.11+)

```lua
vim.lsp.config('dmls', {
  cmd = { 'dmls' },
  filetypes = { 'markdown' },        -- add 'yaml' for YAML schema checks (see below)
  root_markers = { '.dmls.toml', '.git' },
})
vim.lsp.enable('dmls')

-- optional: mute Darkmatter syntax
vim.api.nvim_set_hl(0, '@lsp.mod.interpolation.markdown', { link = 'Comment' })
vim.api.nvim_set_hl(0, '@lsp.mod.directive.markdown',     { link = 'Comment' })
vim.api.nvim_set_hl(0, '@lsp.typemod.string.wiki.markdown', { link = '@markup.link' })
```

For Neovim 0.10, call `vim.lsp.start({ name = 'dmls', cmd = { 'dmls' },
root_dir = vim.fs.root(0, { '.dmls.toml', '.git' }) })` from a `FileType
markdown` autocommand.

Renaming a file doesn't rewrite its links in Neovim. On Linux, client-side
file watching is limited, so DMLS rescans when you save.

### Helix

```toml
# ~/.config/helix/languages.toml (or .helix/languages.toml in a project)
[language-server.dmls]
command = "dmls"

[[language]]
name = "markdown"
language-servers = ["dmls"]
```

Run `:config-reload`. Helix gets no semantic tokens or LSP folding; it uses
its own tree-sitter highlighting and folding instead.

### Any other LSP client

The server command is just `dmls`. Register it for the `markdown` language,
use `.dmls.toml` or `.git` as the root marker, and use
`**/*.{md,markdown,mdown,mkdn}` for file watching.

## YAML files

DMLS can validate whole `.yaml`/`.yml` files: a schema-trigger file you're
writing, a `kind: yaml-schema` definition, or a YAML document typed by its
`kind:`. It decides by the file's extension, so it serves any YAML buffer the
editor sends it.

- **Neovim:** add `'yaml'` to `filetypes`.
- **Helix:** add a `[[language]] name = "yaml"` entry that lists
  `language-servers = ["yaml-language-server", "dmls"]`, keeping your
  existing YAML server.
- **VS Code and Zed:** the shipped extensions register DMLS for Markdown
  only. Check YAML files with `md schema validate file.yaml` instead.

## Is it working?

1. Open a Markdown file containing a broken link, `[x](./nope.md)`. You
   should see a broken-path diagnostic.
2. Type `{{ ` in the body. You should be offered frontmatter keys, `ctx.`,
   and functions.
3. Hover a frontmatter key such as `title`. You should see its type from the
   baseline schema.

If nothing happens, check the logs (see SKILL.md → Gotchas). Also confirm
the editor attached the server to Markdown files, and that it isn't using a
different `dmls` binary than the one you installed.
