; Definitions
(variable_declarator
  .
  (identifier) @local.definition.var)

(variable_declarator
  (tuple_pattern
    (identifier) @local.definition.var))

(declaration_expression
  name: (identifier) @local.definition.var)

(foreach_statement
  left: (identifier) @local.definition.var)

(foreach_statement
  left: (tuple_pattern
    (identifier) @local.definition.var))

(parameter
  (identifier) @local.definition.parameter)

; Methods - capture full node for signature extraction
(method_declaration
  name: (identifier) @local.definition.method) @local.definition.method.context

; Explicit instance constructors; a static constructor (`static C() {}`)
; is not captured.
((constructor_declaration
  (modifier)* @_modifier
  name: (identifier) @local.definition.constructor) @local.definition.constructor.context
  (#not-eq? @_modifier "static"))

; Enum members
(enum_member_declaration
  name: (identifier) @local.definition.enum_variant) @local.definition.enum_variant.context

; Local functions - capture full node for signature extraction
(local_function_statement
  name: (identifier) @local.definition.method) @local.definition.method.context

(property_declaration
  name: (identifier) @local.definition.property) @local.definition.property.context

; Namespaces, block-scoped and file-scoped
(namespace_declaration
  name: [
    (identifier)
    (qualified_name)
  ] @local.definition.namespace) @local.definition.namespace.context

(file_scoped_namespace_declaration
  name: [
    (identifier)
    (qualified_name)
  ] @local.definition.namespace) @local.definition.namespace.context

(type_parameter
  (identifier) @local.definition.type)

; Classes - capture full node for context
(class_declaration
  name: (identifier) @local.definition.class) @local.definition.class.context

; Structs - capture full node for context
(struct_declaration
  name: (identifier) @local.definition.type) @local.definition.type.context

; Interfaces - capture full node for context
(interface_declaration
  name: (identifier) @local.definition.interface) @local.definition.interface.context

; Enums - capture full node for context
(enum_declaration
  name: (identifier) @local.definition.enum) @local.definition.enum.context

; Records - capture full node for context (C# 9+)
(record_declaration
  name: (identifier) @local.definition.type) @local.definition.type.context

; Imports (using directives)
(using_directive
  (qualified_name) @local.definition.import)

(using_directive
  (identifier) @local.definition.import)

; References
(identifier) @local.reference

; Scope
(block) @local.scope
