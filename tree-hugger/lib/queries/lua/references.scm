; Lua identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references
(identifier) @reference

; Dot index expression base reference
(dot_index_expression
  table: (identifier) @reference)

; Bracket index expression base reference
(bracket_index_expression
  table: (identifier) @reference)

; Function call references
(function_call
  name: (identifier) @reference)

; Method call base reference
(method_index_expression
  table: (identifier) @reference)
