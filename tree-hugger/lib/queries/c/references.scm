; C identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Type identifiers
(type_identifier) @reference

; Field access references
(field_expression
  argument: (identifier) @reference)

; Pointer dereference references
(pointer_expression
  argument: (identifier) @reference)

; Function call references
(call_expression
  function: (identifier) @reference)

; Sizeof type references
(sizeof_expression
  type: (type_descriptor
    type: (type_identifier) @reference))
