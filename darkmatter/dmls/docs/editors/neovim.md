# Neovim

Neovim's built-in LSP client (0.10+) drives `dmls` directly — no plugin beyond
your normal LSP setup is required. Modern Neovim advertises UTF-8, so `dmls`
negotiates UTF-8 there.

## Neovim 0.11+ (native config API)

```lua
vim.lsp.config('dmls', {
  cmd = { 'dmls' },
  filetypes = { 'markdown' },
  root_markers = { '.dmls.toml', '.git' },
})

vim.lsp.enable('dmls')
```

## Neovim 0.10 (direct start)

```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'markdown',
  callback = function(args)
    local root = vim.fs.root(args.buf, { '.dmls.toml', '.git' }) or vim.fn.getcwd()
    vim.lsp.start({
      name = 'dmls',
      cmd = { 'dmls' },
      root_dir = root,
    })
  end,
})
```

## Client quirks that affect `dmls`

- **File watching on Linux is limited.** Neovim's client-side watcher is
  disabled/limited on Linux. `dmls` keeps a server-side rescan fallback, so
  changes to unopened files still update the graph — no configuration needed.
- **No `workspace/willRenameFiles`.** Neovim does not report file-operation
  notifications, so **file rename does not rewrite links** on Neovim in v1.
  Heading rename (`textDocument/rename`) works normally.
- **Line-only folding.** Neovim requests line-only folding ranges; `dmls`
  already emits line-safe ranges. On 0.11+ you can wire folds through
  `vim.lsp.foldexpr()`.
- **Dynamic registration.** Neovim registers capabilities dynamically; if a
  feature seems missing, check `client:supports_method()` rather than the static
  `server_capabilities` table.

## What you get

Navigation, diagnostics, completion, hover, frontmatter schema intelligence,
directive/transclusion/interpolation intelligence, read-only shell-policy hover,
heading rename, the v1 code-action set, and whole-document formatting. Hover is
rendered in a floating window (text-first Markdown; no inline images).
