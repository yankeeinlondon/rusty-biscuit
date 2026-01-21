; Lua lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect print() calls (debug prints)
(function_call
  name: (identifier) @_fn
  (#eq? @_fn "print")) @diagnostic.debug-print
