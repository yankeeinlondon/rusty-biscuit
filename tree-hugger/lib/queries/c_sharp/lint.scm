; C# lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect Console.WriteLine calls (debug prints)
(invocation_expression
  function: (member_access_expression
    expression: (identifier) @_cls
    name: (identifier) @_method)
  (#eq? @_cls "Console")
  (#match? @_method "^(Write|WriteLine)$")) @diagnostic.debug-print

; Detect empty blocks
(block . "}" @diagnostic.empty-block)
