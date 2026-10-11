local m = require("m")
local M = {}
function M.foo(...) return m.foo(...) end
return M
