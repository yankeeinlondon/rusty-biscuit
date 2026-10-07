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

; Method-call names (`x.f()`, `x.f::<T>()`), which no pattern above
; captures. Read only by reference occurrences, never as `@reference`.
(call_expression
  function: (field_expression
    field: (field_identifier) @reference.member))

(call_expression
  function: (generic_function
    function: (field_expression
      field: (field_identifier) @reference.member)))

; Struct-pattern field shorthands (`S { f }`): a new binding, or a read of
; the constant of that name in view; reference occurrences decide which.
; Read only by reference occurrences, never as `@reference`.
(shorthand_field_identifier) @reference.pattern
