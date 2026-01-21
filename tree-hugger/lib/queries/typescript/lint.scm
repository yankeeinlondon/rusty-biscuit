; TypeScript lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect debugger statements
(debugger_statement) @diagnostic.debugger-statement

; Detect eval() calls
(call_expression
  function: (identifier) @_fn
  (#eq? @_fn "eval")) @diagnostic.eval-call
