; PHP lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect var_dump/print_r calls (debug prints)
(function_call_expression
  function: (name) @_fn
  (#match? @_fn "^(var_dump|print_r|echo|print)$")) @diagnostic.debug-print

; Detect empty blocks
(compound_statement . "}" @diagnostic.empty-block)
