; Python lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect print() calls (debug prints)
(call
  function: (identifier) @_fn
  (#eq? @_fn "print")) @diagnostic.debug-print

; Detect pass statements (potential empty blocks)
(pass_statement) @diagnostic.empty-block
