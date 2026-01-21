; Python lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect eval() calls
(call
  function: (identifier) @_fn
  (#eq? @_fn "eval")) @diagnostic.eval-call

; Detect exec() calls
(call
  function: (identifier) @_fn2
  (#eq? @_fn2 "exec")) @diagnostic.exec-call

; Detect breakpoint() calls
(call
  function: (identifier) @_fn3
  (#eq? @_fn3 "breakpoint")) @diagnostic.breakpoint-call
