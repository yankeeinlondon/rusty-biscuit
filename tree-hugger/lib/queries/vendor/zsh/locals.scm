; Scopes
(function_definition) @local.scope

; Definitions
(variable_assignment
  name: (variable_name) @local.definition.var)

; Functions - capture full node for context
; Note: Zsh functions have no type annotations/parameters in the traditional sense
(function_definition
  name: (word) @local.definition.function) @local.definition.function.context

; References
(variable_name) @local.reference

(word) @local.reference
