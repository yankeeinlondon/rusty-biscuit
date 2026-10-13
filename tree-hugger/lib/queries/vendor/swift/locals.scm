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

; Initializers (`init`, `init?`, `init!`): the name is the `init` keyword
(init_declaration
  name: "init" @local.definition.constructor) @local.definition.constructor.context

; Type aliases; an `associatedtype` is not one
(typealias_declaration
  name: (type_identifier) @local.definition.type_alias) @local.definition.type_alias.context

; Protocols (Swift's interfaces) - capture full node for context
(protocol_declaration
  name: (type_identifier) @local.definition.interface) @local.definition.interface.context

; Function parameters
(parameter
  name: (simple_identifier) @local.definition.parameter)

; Enum cases; `case east, west` declares two names in one entry
(enum_entry
  name: (simple_identifier) @local.definition.enum_variant)

; Protocol requirements: functions are methods (direct-member rule), properties are fields
(protocol_function_declaration
  name: (simple_identifier) @local.definition.function) @local.definition.function.context

(protocol_property_declaration
  name: (pattern
    bound_identifier: (simple_identifier) @local.definition.field))

; Property bindings (var/let declarations)
(pattern
  bound_identifier: (simple_identifier) @local.definition.var)

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
