; Scopes
;-------
(statement_block) @local.scope

(function_expression) @local.scope

(arrow_function) @local.scope

(function_declaration) @local.scope

(method_definition) @local.scope

(for_statement) @local.scope

(for_in_statement) @local.scope

(catch_clause) @local.scope

; Definitions
;------------
(variable_declarator
  name: (identifier) @local.definition.var)

(variable_declarator
  name: (object_pattern
    (shorthand_property_identifier_pattern) @local.definition.var))

; Imports: only the name each import binds: a default import, a
; specifier's alias (else its name), `* as ns`. An aliased specifier's
; original name (`a` in `{ a as b }`) is never bound.
(import_clause
  (identifier) @local.definition.import)

(import_specifier
  name: (identifier) @local.definition.import
  !alias)

(import_specifier
  alias: (identifier) @local.definition.import)

(namespace_import
  (identifier) @local.definition.import)

; Functions - capture full node for signature extraction
(function_declaration
  (identifier) @local.definition.function
  (#set! definition.var.scope parent)) @local.definition.function.context

; Methods - capture full node for signature extraction
(method_definition
  (property_identifier) @local.definition.function
  (#set! definition.var.scope parent)) @local.definition.function.context

; Arrow functions assigned to variables - capture name and use arrow function as context
(variable_declarator
  name: (identifier) @local.definition.function
  value: (arrow_function) @local.definition.function.context)

; References
;------------
(identifier) @local.reference

(shorthand_property_identifier) @local.reference
