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

; Enum cases (`case Red, Blue`, `case Value(x: Int)`)
(simple_enum_case
  name: (identifier) @local.definition.enum_variant) @local.definition.enum_variant.context

(full_enum_case
  name: (identifier) @local.definition.enum_variant) @local.definition.enum_variant.context

; Type aliases; an abstract `type Element` has no right-hand side
(type_definition
  name: (type_identifier) @local.definition.type_alias
  type: (_)) @local.definition.type_alias.context

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

; Grouped names (`val a, b: Int = 1`) sit under one `identifiers` pattern;
; each is its own definition. A tuple pattern (`val (a, b) = …`) is not
; captured.
(val_definition
  pattern: (identifiers
    (identifier) @local.definition.var))

(var_definition
  pattern: (identifiers
    (identifier) @local.definition.var))

(val_declaration
  name: (identifier) @local.definition.var)

(var_declaration
  name: (identifier) @local.definition.var)

(for_expression
  enumerators: (enumerators
    (enumerator
      (tuple_pattern
        (identifier) @local.definition.var))))

; Imports: only the name each import binds. A path's final segment (before
; `,` or at the end), a selector, a rename's alias (braced, or Scala 3's
; unbraced `a.b.C as D`), and, for an on-demand import (`a.b.*`, `a.b._`,
; `a.given`, `a.{_, given T}`), the final segment of the path it opens;
; qualifiers and a rename's original are reads. A segment, selector or
; alias is an `identifier` (backticked included) or an `operator_identifier`
; (`a.b.+`, `a.{C as ++}`); a `wildcard` alias (`C as _`) hides a name and
; binds nothing.
; Which flat path segment binds depends on what follows it, and a comment
; may sit in between, which a `.` anchor cannot step over; so every segment
; is captured here and `TreeFile::imported_symbols` keeps only the binding
; ones (`declaration_names::scala_import_binds`).
(import_declaration
  [(identifier) (operator_identifier)] @local.definition.import)

(import_declaration
  (as_renamed_identifier
    alias: [(identifier) (operator_identifier)] @local.definition.import))

(namespace_selectors
  [(identifier) (operator_identifier)] @local.definition.import)

(namespace_selectors
  (arrow_renamed_identifier
    alias: [(identifier) (operator_identifier)] @local.definition.import))

(namespace_selectors
  (as_renamed_identifier
    alias: [(identifier) (operator_identifier)] @local.definition.import))
