; Java lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((line_comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

((block_comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect System.out.println calls (debug prints)
(method_invocation
  object: (field_access
    object: (identifier) @_cls
    field: (identifier) @_field)
  name: (identifier) @_method
  (#eq? @_cls "System")
  (#eq? @_field "out")
  (#match? @_method "^(print|println)$")) @diagnostic.debug-print

; Detect empty blocks
(block . "}" @diagnostic.empty-block)
