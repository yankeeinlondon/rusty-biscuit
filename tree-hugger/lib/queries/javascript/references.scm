; JavaScript identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Member expression object reference
(member_expression
  object: (identifier) @reference)

; Call expression function reference
(call_expression
  function: (identifier) @reference)

; Template literal substitutions
(template_substitution
  (identifier) @reference)

; Shorthand property identifiers that are references
(shorthand_property_identifier) @reference

; Object key that is a computed property
(computed_property_name
  (identifier) @reference)

; Method-call and constructor member names (`obj.run()`, `new ns.Widget()`),
; which no pattern above captures. Read only by reference occurrences, never
; as `@reference`.
(call_expression
  function: (member_expression
    property: [
      (property_identifier)
      (private_property_identifier)
    ] @reference.member))

(new_expression
  constructor: (member_expression
    property: [
      (property_identifier)
      (private_property_identifier)
    ] @reference.member))

