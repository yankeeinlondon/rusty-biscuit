-- Level-2 headless probe driven by tests/level2_editor_neovim.rs.
--
-- Runs under `nvim --clean --headless -l probe.lua`, attaches Neovim's real
-- LSP client to the dmls binary, and reports the decoded semantic-token state
-- as JSON on stdout between DMLS_L2_JSON< ... >DMLS_L2_JSON markers. This
-- exercises the editor-side half the in-process session tests cannot: the
-- client's token decode, byte-column mapping after position-encoding
-- negotiation, and the workspace/semanticTokens/refresh repaint loop.
--
-- Inputs (environment):
--   DMLS_L2_BIN   absolute path to the dmls binary
--   DMLS_L2_ROOT  workspace root containing tokens.md and .dmls.toml
--   DMLS_L2_MODE  'tokens' (default) or 'refresh'

local mode = vim.env.DMLS_L2_MODE or 'tokens'
local bin = assert(vim.env.DMLS_L2_BIN, 'DMLS_L2_BIN not set')
local root = assert(vim.env.DMLS_L2_ROOT, 'DMLS_L2_ROOT not set')

vim.o.swapfile = false

local out = { mode = mode, probes = vim.empty_dict() }

local function fail(msg)
  out.errors = out.errors or {}
  out.errors[#out.errors + 1] = msg
end

vim.cmd.edit(vim.fn.fnameescape(root .. '/tokens.md'))
local buf = vim.api.nvim_get_current_buf()

local client_id = vim.lsp.start({
  name = 'dmls',
  cmd = { bin },
  root_dir = root,
})

local attached = client_id ~= nil
  and vim.wait(15000, function()
    return next(vim.lsp.get_clients({ bufnr = buf, id = client_id })) ~= nil
  end, 50)
if not attached then
  fail('dmls did not attach within 15s')
end

local client = client_id and vim.lsp.get_clients({ id = client_id })[1] or nil
out.offset_encoding = client and client.offset_encoding or nil

-- `client:notify` on 0.11+, dot-call on 0.10 (where methods took no self).
local function notify_client(method, params)
  if not client then
    return
  end
  if vim.fn.has('nvim-0.11') == 1 then
    client:notify(method, params)
  else
    client.notify(method, params)
  end
end

-- Normalizes the `modifiers` set (name -> true map, or a list on older
-- releases) into a sorted array of names for stable JSON.
local function mods_list(mods)
  local names = {}
  if type(mods) == 'table' then
    for k, v in pairs(mods) do
      if type(k) == 'number' then
        names[#names + 1] = v
      elseif v then
        names[#names + 1] = k
      end
    end
  end
  table.sort(names)
  return names
end

local function tokens_at(row, col)
  local raw = vim.lsp.semantic_tokens.get_at_pos(buf, row, col) or {}
  local list = {}
  for _, t in ipairs(raw) do
    list[#list + 1] = {
      line = t.line,
      start_col = t.start_col,
      end_col = t.end_col,
      type = t.type,
      modifiers = mods_list(t.modifiers),
    }
  end
  return list
end

-- (0-based row, 0-based byte col) of the nth occurrence of a literal needle.
local function pos_of(needle, nth)
  nth = nth or 1
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local seen = 0
  for i, line in ipairs(lines) do
    local from = 1
    while true do
      local s = string.find(line, needle, from, true)
      if not s then
        break
      end
      seen = seen + 1
      if seen == nth then
        return i - 1, s - 1
      end
      from = s + 1
    end
  end
  return nil, nil
end

local function probe(name, needle, nth)
  local row, col = pos_of(needle, nth)
  if not row then
    fail('needle not found: ' .. needle)
    return
  end
  out.probes[name] = { row = row, col = col, tokens = tokens_at(row, col) }
end

-- Wait until the client has decoded and applied the first token batch.
local ir, ic = pos_of('{{ title }}')
local painted = ir ~= nil
  and vim.wait(15000, function()
    return #tokens_at(ir, ic) > 0
  end, 100)
if not painted then
  fail('no semantic tokens applied within 15s')
end

if mode == 'tokens' then
  probe('interpolation', '{{ title }}')
  probe('unicode_interpolation', '{{ east }}')
  probe('wiki_frame', '[[')
  probe('wiki_path', 'missing-note')
  probe('wiki_alias', 'alias]]')
  probe('literal_open', '{{{ raw')
  probe('literal_continuation', 'still raw }}}')
  probe('directive_keyword', '::file')
  probe('directive_target', './other.md')
  probe('option_key', 'when=')
  probe('option_interpolation', '{{ cond }}')
  probe('closer_keyword', '::end-block')
  probe('fenced_interpolation', '{{ nope }}')
  probe('prose', 'Intro')
elseif mode == 'refresh' then
  out.initial = #tokens_at(ir, ic)

  local function set_enable(enable)
    local f = assert(io.open(root .. '/.dmls.toml', 'w'))
    f:write('[semantic_tokens]\nenable = ' .. tostring(enable) .. '\n')
    f:close()
    notify_client('workspace/didChangeConfiguration', { settings = vim.empty_dict() })
  end

  set_enable(false)
  out.cleared = vim.wait(15000, function()
    return #tokens_at(ir, ic) == 0
  end, 100)

  set_enable(true)
  out.repainted = vim.wait(15000, function()
    return #tokens_at(ir, ic) > 0
  end, 100)

  -- `wiki.enable = false` must suppress only the wiki family: the `[[` frame
  -- token disappears while the interpolation stays painted.
  local wr, wc = pos_of('[[')
  local f = assert(io.open(root .. '/.dmls.toml', 'w'))
  f:write('[semantic_tokens]\nenable = true\n\n[wiki]\nenable = false\n')
  f:close()
  notify_client('workspace/didChangeConfiguration', { settings = vim.empty_dict() })
  out.wiki_suppressed = wr ~= nil
    and vim.wait(15000, function()
      return #tokens_at(wr, wc) == 0 and #tokens_at(ir, ic) > 0
    end, 100)
else
  fail('unknown mode: ' .. mode)
end

if client then
  if vim.fn.has('nvim-0.11') == 1 then
    client:stop()
  else
    vim.lsp.stop_client(client_id)
  end
  vim.wait(3000, function()
    return next(vim.lsp.get_clients({ id = client_id })) == nil
  end, 50)
end

print('DMLS_L2_JSON<' .. vim.json.encode(out) .. '>DMLS_L2_JSON')
