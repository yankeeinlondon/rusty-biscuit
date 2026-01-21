; Rust lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((line_comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect unwrap() calls
(call_expression
  function: (field_expression
    field: (field_identifier) @_method)
  (#eq? @_method "unwrap")) @diagnostic.unwrap-call

; Detect expect() calls (similar concern to unwrap)
(call_expression
  function: (field_expression
    field: (field_identifier) @_method2)
  (#eq? @_method2 "expect")) @diagnostic.expect-call
