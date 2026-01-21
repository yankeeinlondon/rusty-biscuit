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
