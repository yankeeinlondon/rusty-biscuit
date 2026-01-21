; Rust identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Type references (type identifiers used in type annotations)
(type_identifier) @reference

; Field access references (the object being accessed)
(field_expression
  value: (identifier) @reference)

; Scoped identifiers (module paths like std::io)
(scoped_identifier
  path: (identifier) @reference)

; Macro invocations
(macro_invocation
  macro: (identifier) @reference)

; Generic type arguments
(type_arguments
  (type_identifier) @reference)
