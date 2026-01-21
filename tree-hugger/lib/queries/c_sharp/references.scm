; C# identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Member access references
(member_access_expression
  expression: (identifier) @reference)

; Invocation expression references
(invocation_expression
  function: (identifier) @reference)

; Generic type arguments
(type_argument_list
  (identifier) @reference)

; Object creation type reference
(object_creation_expression
  type: (identifier) @reference)

; Base type references in class declarations
(base_list
  (identifier) @reference)

; Attribute references
(attribute
  name: (identifier) @reference)

; Typeof expressions
(typeof_expression
  type: (identifier) @reference)
