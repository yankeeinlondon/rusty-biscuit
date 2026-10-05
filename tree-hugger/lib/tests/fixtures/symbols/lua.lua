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

-- ---- Declaration boundaries (keyword and signature text) ----

-- A comment naming declaration keywords: function local
local boundaryTable = {
  1, 2,
  3, 4,
}

function Widget.boundaryMultiLineHeader(
  firstValue,
  secondValue
)
  return firstValue + secondValue
end

-- ---- Scope and qualified names ----

local Other = {}

function Other:methodWithSelf()
  return self
end

-- ---- Flags and metrics ----

--- An LDoc comment is not detected as documentation.
local function documented_pair(first, second) end

local function variadic(first, ...) end

local function test_helper() end

return Widget
