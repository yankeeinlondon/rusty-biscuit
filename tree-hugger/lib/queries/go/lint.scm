; Go lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect fmt.Println calls (debug prints)
(call_expression
  function: (selector_expression
    operand: (identifier) @_pkg
    field: (field_identifier) @_fn)
  (#eq? @_pkg "fmt")
  (#match? @_fn "^(Print|Println|Printf)$")) @diagnostic.debug-print

; Detect empty blocks
(block . "}" @diagnostic.empty-block)
