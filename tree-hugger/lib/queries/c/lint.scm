; C lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect printf calls (debug prints)
(call_expression
  function: (identifier) @_fn
  (#match? @_fn "^(printf|fprintf|puts)$")) @diagnostic.debug-print

; Detect empty compound statements
(compound_statement . "}" @diagnostic.empty-block)
