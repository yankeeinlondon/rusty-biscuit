; JavaScript lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect console.log calls (debug prints)
(call_expression
  function: (member_expression
    object: (identifier) @_obj
    property: (property_identifier) @_prop)
  (#eq? @_obj "console")
  (#match? @_prop "^(log|debug|info|warn|error)$")) @diagnostic.debug-print

; Detect empty blocks
(statement_block . "}" @diagnostic.empty-block)
