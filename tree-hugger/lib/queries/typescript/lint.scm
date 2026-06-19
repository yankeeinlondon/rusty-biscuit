; TypeScript lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect debugger statements
(debugger_statement) @diagnostic.debugger-statement

; Detect eval() calls
(call_expression
  function: (identifier) @_fn
  (#eq? @_fn "eval")) @diagnostic.eval-call

; Detect console.log() calls
(call_expression
  function: (member_expression
    object: (identifier) @_obj
    property: (property_identifier) @_prop)
  (#eq? @_obj "console")
  (#eq? @_prop "log")) @diagnostic.console-log
