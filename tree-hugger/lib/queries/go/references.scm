; Go identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Type identifiers in type specs
(type_identifier) @reference

; Selector expression field access
(selector_expression
  operand: (identifier) @reference)

; Call expression function reference
(call_expression
  function: (identifier) @reference)

; Package-qualified identifiers
(qualified_type
  package: (package_identifier) @reference)

; Composite literal type reference
(composite_literal
  type: (type_identifier) @reference)

; A method or package-qualified function invoked through a selector
; (`obj.m()`, `pkg.F()`): the callee name, an occurrence only
(call_expression
  function: (selector_expression
    field: (field_identifier) @reference.member))
