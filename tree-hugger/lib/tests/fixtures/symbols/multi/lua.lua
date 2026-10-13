-- Multi-file fixture: Lua.

local M = {}

local default_size = 10

function M.new_bag(size)
  return { size = size or default_size }
end

function M:describe()
  return "bag"
end

local function helper()
  return default_size
end

return M
