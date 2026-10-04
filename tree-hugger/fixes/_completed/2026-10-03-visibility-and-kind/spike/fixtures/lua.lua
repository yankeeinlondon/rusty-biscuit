-- Symbol-kind and visibility fixture for Lua.

function globalFunction(parameterValue)
  local blockLocalValue = parameterValue + 1
  return blockLocalValue
end

local function chunkLocalFunction()
  return 1
end

globalVariable = 2

local chunkLocalVariable = 3

local Widget = {}

function Widget.tableFunction()
  return 4
end

function Widget:methodWithSelf()
  return self
end

Widget.tableField = 5

if globalVariable then
  local nestedBlockLocal = 6
  local function nestedBlockLocalFunction() end
end

return Widget
