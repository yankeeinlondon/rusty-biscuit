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

; Local functions - capture full node for signature extraction
(local_function_statement
  name: (identifier) @local.definition.method) @local.definition.method.context

(property_declaration
  name: (identifier) @local.definition)

(type_parameter
  (identifier) @local.definition.type)

; Classes - capture full node for context
(class_declaration
  name: (identifier) @local.definition) @local.definition.type.context

; References
(identifier) @local.reference

; Scope
(block) @local.scope
