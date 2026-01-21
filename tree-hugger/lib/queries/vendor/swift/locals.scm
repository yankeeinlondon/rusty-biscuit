(import_declaration
  (identifier) @local.definition.import)

; Functions - capture full node for signature extraction
(function_declaration
  name: (simple_identifier) @local.definition.function) @local.definition.function.context

; All class/struct/enum declarations - capture as type
; Note: Swift's tree-sitter grammar uses class_declaration for struct, class, and enum
; with a declaration_kind field. Fine-grained distinction would require Rust-level handling.
(class_declaration
  name: (type_identifier) @local.definition.type) @local.definition.type.context

; Protocols (Swift's interfaces) - capture full node for context
(protocol_declaration
  name: (type_identifier) @local.definition.interface) @local.definition.interface.context

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
