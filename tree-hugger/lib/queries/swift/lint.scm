; Swift lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

((multiline_comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect print() calls (debug prints)
(call_expression
  (simple_identifier) @_fn
  (#eq? @_fn "print")) @diagnostic.debug-print
