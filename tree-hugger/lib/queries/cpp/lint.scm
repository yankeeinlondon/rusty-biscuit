; C++ lint rules
; Capture names follow @diagnostic.{rule-id} convention

; Detect TODO/FIXME comments
((comment) @diagnostic.todo-comment
 (#match? @diagnostic.todo-comment "TODO|FIXME"))

; Detect std::cout usage (debug prints)
(call_expression
  function: (qualified_identifier
    scope: (namespace_identifier) @_ns
    name: (identifier) @_fn)
  (#eq? @_ns "std")
  (#eq? @_fn "cout")) @diagnostic.debug-print

; Detect printf calls (debug prints)
(call_expression
  function: (identifier) @_fn
  (#match? @_fn "^(printf|fprintf|puts|cout)$")) @diagnostic.debug-print

; Detect empty compound statements
(compound_statement . "}" @diagnostic.empty-block)
