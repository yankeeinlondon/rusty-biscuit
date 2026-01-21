; Rust lint rules
; Capture names follow @diagnostic.{rule-id} convention

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

; Detect dbg!() macro calls
(macro_invocation
  macro: (identifier) @_macro
  (#eq? @_macro "dbg")) @diagnostic.dbg-macro
