; PHP lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect eval() calls
(function_call_expression
  function: (name) @_fn
  (#eq? @_fn "eval")) @diagnostic.eval-call
