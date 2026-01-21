; Java identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Type identifiers
(type_identifier) @reference

; Method invocation object reference
(method_invocation
  object: (identifier) @reference)

; Field access object reference
(field_access
  object: (identifier) @reference)

; Generic type arguments
(type_arguments
  (type_identifier) @reference)

; Class instance creation type reference
(object_creation_expression
  type: (type_identifier) @reference)

; Annotation references
(marker_annotation
  name: (identifier) @reference)
(annotation
  name: (identifier) @reference)
