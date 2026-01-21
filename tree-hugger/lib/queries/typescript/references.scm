; TypeScript identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Type references
(type_identifier) @reference

; Member expression object reference
(member_expression
  object: (identifier) @reference)

; Call expression function reference
(call_expression
  function: (identifier) @reference)

; Generic type arguments
(type_arguments
  (type_identifier) @reference)

; Template literal substitutions
(template_substitution
  (identifier) @reference)

; Shorthand property identifiers that are references
(shorthand_property_identifier) @reference
