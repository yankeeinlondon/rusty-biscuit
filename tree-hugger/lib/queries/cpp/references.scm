; C++ identifier references
; Captures identifier usages (not definitions) for semantic analysis

; Simple identifier references in expressions
(identifier) @reference

; Type identifiers
(type_identifier) @reference

; Namespace identifiers
(namespace_identifier) @reference

; Field access references
(field_expression
  argument: (identifier) @reference)

; Qualified identifiers (namespace::symbol)
(qualified_identifier
  scope: (namespace_identifier) @reference)

; Template type arguments
(template_argument_list
  (type_descriptor
    type: (type_identifier) @reference))

; Function call references
(call_expression
  function: (identifier) @reference)

; A method invoked through a field access (`obj.m()`, `p->m()`,
; `obj.template m<T>()`): the callee name, an occurrence only
(call_expression
  function: (field_expression
    field: [
      (field_identifier) @reference.member
      (template_method
        name: (field_identifier) @reference.member)
      (dependent_name
        (template_method
          name: (field_identifier) @reference.member))
    ]))
