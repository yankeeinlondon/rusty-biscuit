; Functions - capture full node for signature extraction
((function_declaration
  name: (identifier) @local.definition.function) @local.definition.function.context
  )

; Methods - capture full node for signature extraction
((method_declaration
  name: (field_identifier) @local.definition.method) @local.definition.method.context
  )

(short_var_declaration
  left: (expression_list
    (identifier) @local.definition.var))

(var_spec
  name: (identifier) @local.definition.var)

(parameter_declaration
  (identifier) @local.definition.var)

(variadic_parameter_declaration
  (identifier) @local.definition.var)

(for_statement
  (range_clause
    left: (expression_list
      (identifier) @local.definition.var)))

(const_declaration
  (const_spec
    name: (identifier) @local.definition.var))

; Type definitions - capture full node
(type_declaration
  (type_spec
    name: (type_identifier) @local.definition.type)) @local.definition.type.context

; Type aliases (`type A = B`), alone or grouped
(type_declaration
  (type_alias
    name: (type_identifier) @local.definition.type_alias)) @local.definition.type_alias.context

; Named fields of a named struct type. Embedded fields have no name, and the
; fields of an anonymous `struct { … }` are not captured.
(type_spec
  type: (struct_type
    (field_declaration_list
      (field_declaration
        name: (field_identifier) @local.definition.field) @local.definition.field.context)))

; reference
(identifier) @local.reference

(type_identifier) @local.reference

(field_identifier) @local.reference

((package_identifier) @local.reference
  (#set! reference.kind "namespace"))

(package_clause
  (package_identifier) @local.definition.namespace)

; Imports: an alias (a blank `_` included, which binds nothing) is the
; import, else the path (interpreted or raw string), whose last segment
; names the package (a dot
; import, which binds no package name, is on demand).
; Imports in grouped imports (import_spec_list)
(import_spec_list
  (import_spec
    name: [(package_identifier) (blank_identifier)] @local.definition.import))

(import_spec_list
  (import_spec
    path: [(interpreted_string_literal) (raw_string_literal)] @local.definition.import))

; Single import declarations
(import_declaration
  (import_spec
    name: [(package_identifier) (blank_identifier)] @local.definition.import))

(import_declaration
  (import_spec
    path: [(interpreted_string_literal) (raw_string_literal)] @local.definition.import))

; Call references
((call_expression
  function: (identifier) @local.reference)
  (#set! reference.kind "call"))

((call_expression
  function: (selector_expression
    field: (field_identifier) @local.reference))
  (#set! reference.kind "call"))

((call_expression
  function: (parenthesized_expression
    (identifier) @local.reference))
  (#set! reference.kind "call"))

((call_expression
  function: (parenthesized_expression
    (selector_expression
      field: (field_identifier) @local.reference)))
  (#set! reference.kind "call"))

; Scopes
(func_literal) @local.scope

(source_file) @local.scope

(function_declaration) @local.scope

(if_statement) @local.scope

(block) @local.scope

(expression_switch_statement) @local.scope

(for_statement) @local.scope

(method_declaration) @local.scope
