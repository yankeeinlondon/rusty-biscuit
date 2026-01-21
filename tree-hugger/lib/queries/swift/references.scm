; Swift identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references
(simple_identifier) @reference

; Type identifiers
(user_type
  (type_identifier) @reference)

; Navigation expression base reference
(navigation_expression
  target: (simple_identifier) @reference)

; Call expression function reference
(call_expression
  (simple_identifier) @reference)

; Generic arguments
(type_arguments
  (user_type
    (type_identifier) @reference))

; Attribute references
(attribute
  (user_type
    (type_identifier) @reference))
