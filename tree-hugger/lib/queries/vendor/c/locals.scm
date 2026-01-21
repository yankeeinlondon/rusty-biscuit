; Functions definitions - capture full node for signature extraction
(function_declarator
  declarator: (identifier) @local.definition.function) @local.definition.function.context

(preproc_function_def
  name: (identifier) @local.definition.macro) @local.scope

(preproc_def
  name: (identifier) @local.definition.macro)

(pointer_declarator
  declarator: (identifier) @local.definition.var)

(parameter_declaration
  declarator: (identifier) @local.definition.parameter)

(init_declarator
  declarator: (identifier) @local.definition.var)

(array_declarator
  declarator: (identifier) @local.definition.var)

(declaration
  declarator: (identifier) @local.definition.var)

; Enum definitions - capture full node for context
(enum_specifier
  name: (_) @local.definition.enum
  (enumerator_list)) @local.definition.enum.context

; Enum enumerators - capture as variables
(enumerator
  name: (identifier) @local.definition.var)

; Type / Struct / Enum
(field_declaration
  declarator: (field_identifier) @local.definition.field)

; Type definitions - capture full node for context
(type_definition
  declarator: (type_identifier) @local.definition.type) @local.definition.type.context

; Struct definitions - capture full node for context
(struct_specifier
  name: (type_identifier) @local.definition.type) @local.definition.type.context

; goto
(labeled_statement
  (statement_identifier) @local.definition)

; References
(identifier) @local.reference

((field_identifier) @local.reference
  (#set! reference.kind "field"))

((type_identifier) @local.reference
  (#set! reference.kind "type"))

(goto_statement
  (statement_identifier) @local.reference)

; Scope
[
  (for_statement)
  (if_statement)
  (while_statement)
  (translation_unit)
  (function_definition)
  (compound_statement) ; a block in curly braces
  (struct_specifier)
] @local.scope
