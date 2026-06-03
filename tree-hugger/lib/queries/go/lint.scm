; Go lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect fmt.Println() calls
(call_expression
  function: (selector_expression
    operand: (identifier) @_pkg
    field: (field_identifier) @_fn)
  (#eq? @_pkg "fmt")
  (#eq? @_fn "Println")) @diagnostic.fmt-println
