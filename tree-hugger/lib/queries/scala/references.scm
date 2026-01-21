; Scala identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Type identifiers
(type_identifier) @reference

; Field expression references
(field_expression
  value: (identifier) @reference)

; Call expression function reference
(call_expression
  function: (identifier) @reference)

; Generic type arguments
(type_arguments
  (type_identifier) @reference)

; Instance expression type reference
(instance_expression
  (type_identifier) @reference)

; Annotation references
(annotation
  name: (type_identifier) @reference)
