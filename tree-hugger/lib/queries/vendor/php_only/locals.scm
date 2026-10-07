; Scopes
;-------
; Classes - capture full node for context
((class_declaration
  name: (name) @local.definition.class) @local.scope @local.definition.class.context
  (#set! definition.class.scope "parent"))

; Interfaces - capture full node for context
((interface_declaration
  name: (name) @local.definition.interface) @local.scope @local.definition.interface.context
  (#set! definition.interface.scope "parent"))

; Traits - capture full node for context
((trait_declaration
  name: (name) @local.definition.trait) @local.scope @local.definition.trait.context
  (#set! definition.trait.scope "parent"))

; Enums (PHP 8.1+) - capture full node for context
((enum_declaration
  name: (name) @local.definition.enum) @local.scope @local.definition.enum.context
  (#set! definition.enum.scope "parent"))

; Enum cases, unit and backed
(enum_case
  name: (name) @local.definition.enum_variant) @local.definition.enum_variant.context

; Named namespaces, semicolon and braced; a global `namespace { … }` has no
; name and declares none
(namespace_definition
  name: (namespace_name) @local.definition.namespace) @local.definition.namespace.context

; Methods - capture full node for signature extraction
((method_declaration
  name: (name) @local.definition.method) @local.scope @local.definition.method.context
  (#set! definition.method.scope "parent"))

; Functions - capture full node for signature extraction
((function_definition
  name: (name) @local.definition.function) @local.scope @local.definition.function.context
  (#set! definition.function.scope "parent"))

(anonymous_function
  (anonymous_function_use_clause
    (variable_name
      (name) @local.definition.var))) @local.scope

; Definitions
;------------
(simple_parameter
  name: (variable_name
    (name) @local.definition.var))

(foreach_statement
  (pair
    (variable_name
      (name) @local.definition.var)))

(foreach_statement
  (variable_name
    (name) @local.reference
    (#set! reference.kind "var"))
  (variable_name
    (name) @local.definition.var))

(property_declaration
  (property_element
    name: (variable_name
      (name) @local.definition.field)))

(const_declaration
  (const_element
    .
    (name) @local.definition.constant))

; Imports (use statements): only the name each clause binds, its alias or
; else its path's last segment (`use A;`, `use A\B;`, `B` in
; `use A\{B, C as D};`); a qualifier and an aliased original are reads.
(namespace_use_clause
  alias: (name) @local.definition.import)

(namespace_use_clause
  (name) @local.definition.import
  !alias)

(namespace_use_clause
  (qualified_name
    (name) @local.definition.import)
  !alias)

; References
;------------
(named_type
  (name) @local.reference
  (#set! reference.kind "type"))

(named_type
  (qualified_name) @local.reference
  (#set! reference.kind "type"))

(variable_name
  (name) @local.reference
  (#set! reference.kind "var"))

(member_access_expression
  name: (name) @local.reference
  (#set! reference.kind "field"))

(member_call_expression
  name: (name) @local.reference
  (#set! reference.kind "method"))

(function_call_expression
  function: (qualified_name
    (name) @local.reference
    (#set! reference.kind "function")))

(object_creation_expression
  (qualified_name
    (name) @local.reference
    (#set! reference.kind "type")))

(scoped_call_expression
  scope: (qualified_name
    (name) @local.reference
    (#set! reference.kind "type"))
  name: (name) @local.reference
  (#set! reference.kind "method"))
