-- Level-2 tmux init config, templated by tests/level2_editor_neovim.rs
-- (__DMLS_BIN__ / __ROOT__ are substituted before use; [[ ]] long strings
-- keep Windows backslashes literal).
--
-- Pins the documented Neovim styling recipe's highlight-group names
-- (docs/editors/neovim.md) to distinctive 256-color indexes so the SGR the
-- terminal actually renders is deterministic and theme-independent:
--   201  interpolations + wiki frames (muted-machinery groups)
--    93  directive keywords + closers
--    87  wiki inner segments (link-like group)
-- Syntax/treesitter and diagnostic visuals are disabled so semantic tokens
-- are the only source of color in the buffer.

vim.o.swapfile = false
vim.o.termguicolors = false
vim.cmd('syntax off')
vim.diagnostic.config({ virtual_text = false, signs = false, underline = false })

vim.api.nvim_set_hl(0, '@lsp.mod.interpolation.markdown', { ctermfg = 201 })
vim.api.nvim_set_hl(0, '@lsp.mod.directive.markdown', { ctermfg = 93 })
vim.api.nvim_set_hl(0, '@lsp.mod.closer.markdown', { ctermfg = 93 })
vim.api.nvim_set_hl(0, '@lsp.typemod.macro.wiki.markdown', { ctermfg = 201 })
vim.api.nvim_set_hl(0, '@lsp.typemod.string.wiki.markdown', { ctermfg = 87 })

vim.api.nvim_create_autocmd('FileType', {
  pattern = 'markdown',
  callback = function(args)
    pcall(vim.treesitter.stop, args.buf)
    vim.lsp.start({
      name = 'dmls',
      cmd = { [[__DMLS_BIN__]] },
      root_dir = [[__ROOT__]],
    })
  end,
})

-- Driven via `:lua DmlsToggle(false)` from the test: rewrites the workspace
-- .dmls.toml and nudges the server with didChangeConfiguration, which must
-- round-trip a workspace/semanticTokens/refresh and repaint the pane.
function DmlsToggle(enable)
  local f = assert(io.open([[__ROOT__]] .. '/.dmls.toml', 'w'))
  f:write('[semantic_tokens]\nenable = ' .. tostring(enable) .. '\n')
  f:close()
  for _, c in ipairs(vim.lsp.get_clients({ name = 'dmls' })) do
    if vim.fn.has('nvim-0.11') == 1 then
      c:notify('workspace/didChangeConfiguration', { settings = vim.empty_dict() })
    else
      c.notify('workspace/didChangeConfiguration', { settings = vim.empty_dict() })
    end
  end
end
