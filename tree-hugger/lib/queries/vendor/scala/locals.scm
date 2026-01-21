; Scopes
[
  (template_body)
  (lambda_expression)
  (function_definition)
  (block)
  (for_expression)
] @local.scope

; References
(identifier) @local.reference

; Definitions
; Function declarations - capture full node for context
(function_declaration
  name: (identifier) @local.definition.function) @local.definition.function.context

; Function definitions - capture full node for signature extraction
(function_definition
  name: (identifier) @local.definition.function
  (#set! definition.var.scope parent)) @local.definition.function.context

; Classes - capture full node for context
(class_definition
  name: (identifier) @local.definition.class) @local.definition.class.context

; Traits (Scala's interfaces) - capture full node for context
(trait_definition
  name: (identifier) @local.definition.trait) @local.definition.trait.context

; Objects (Scala singletons/modules) - capture full node for context
(object_definition
  name: (identifier) @local.definition.module) @local.definition.module.context

; Enums (Scala 3) - capture full node for context
(enum_definition
  name: (identifier) @local.definition.enum) @local.definition.enum.context

(parameter
  name: (identifier) @local.definition.parameter)

(class_parameter
  name: (identifier) @local.definition.parameter)

(lambda_expression
  parameters: (identifier) @local.definition.var)

(binding
  name: (identifier) @local.definition.var)

(val_definition
  pattern: (identifier) @local.definition.var)

(var_definition
  pattern: (identifier) @local.definition.var)

(val_declaration
  name: (identifier) @local.definition.var)

(var_declaration
  name: (identifier) @local.definition.var)

(for_expression
  enumerators: (enumerators
    (enumerator
      (tuple_pattern
        (identifier) @local.definition.var))))
