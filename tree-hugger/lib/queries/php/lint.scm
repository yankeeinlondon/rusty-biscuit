; PHP lint rules
; Capture names follow @diagnostic.{rule-id} convention
; PHP function names are case-insensitive, so names match with anchored (?i).

; Detect eval() calls
(function_call_expression
  function: (name) @_fn
  (#match? @_fn "(?i)^eval$")) @diagnostic.eval-call

; Detect exec() and the global \exec(). The whole callee text is matched
; because a qualified_name's `prefix:` field holds the bare `\` too, so only
; the text tells \exec apart from Foo\exec, a different function.
(function_call_expression
  function: [(name) (qualified_name)] @_fn
  (#match? @_fn "(?i)^\\\\?exec$")) @diagnostic.exec-call
