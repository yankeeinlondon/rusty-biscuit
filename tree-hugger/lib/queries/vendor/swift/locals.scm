(import_declaration
  (identifier) @local.definition.import)

; Functions - capture full node for signature extraction
(function_declaration
  name: (simple_identifier) @local.definition.function) @local.definition.function.context

; Scopes
[
  (statements)
  (for_statement)
  (while_statement)
  (repeat_while_statement)
  (do_statement)
  (if_statement)
  (guard_statement)
  (switch_statement)
  (property_declaration)
  (function_declaration)
  (class_declaration)
  (protocol_declaration)
] @local.scope
