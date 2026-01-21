; Scala lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect println calls (debug prints)
(call_expression
  function: (identifier) @_fn
  (#eq? @_fn "println")) @diagnostic.debug-print

; Detect empty blocks
(block . "}" @diagnostic.empty-block)
