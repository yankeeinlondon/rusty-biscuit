; Python identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Attribute access base object
(attribute
  object: (identifier) @reference)

; Call expression function reference
(call
  function: (identifier) @reference)

; Subscript base reference
(subscript
  value: (identifier) @reference)

; Type annotations
(type
  (identifier) @reference)

; Decorator references
(decorator
  (identifier) @reference)

; Comprehension iterables
(for_in_clause
  (identifier) @reference)
