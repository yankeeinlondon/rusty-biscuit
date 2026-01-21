; PHP identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Variable names (without $)
(variable_name
  (name) @reference)

; Simple identifier references
(name) @reference

; Qualified names (namespaced identifiers)
(qualified_name
  (name) @reference)

; Member access references
(member_access_expression
  object: (variable_name
    (name) @reference))

; Function call references
(function_call_expression
  function: (name) @reference)

; Class constant access
(class_constant_access_expression
  class: (name) @reference)
