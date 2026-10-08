; SCOPES
; declarations
(program) @local.scope

(class_declaration
  body: (_) @local.scope)

(record_declaration
  body: (_) @local.scope)

(enum_declaration
  body: (_) @local.scope)

(lambda_expression) @local.scope

(enhanced_for_statement) @local.scope

; block
(block) @local.scope

; if/else
(if_statement) @local.scope ; if+else

(if_statement
  consequence: (_) @local.scope) ; if body in case there are no braces

(if_statement
  alternative: (_) @local.scope) ; else body in case there are no braces

; try/catch
(try_statement) @local.scope ; covers try+catch, individual try and catch are covered by (block)

(catch_clause) @local.scope ; needed because `Exception` variable

; loops
(for_statement) @local.scope ; whole for_statement because loop iterator variable

(for_statement
  ; "for" body in case there are no braces
  body: (_) @local.scope)

(do_statement
  body: (_) @local.scope)

(while_statement
  body: (_) @local.scope)

; Functions
(constructor_declaration) @local.scope

(method_declaration) @local.scope

; DEFINITIONS
; A dotted package name is a `scoped_identifier`, not an `identifier`.
(package_declaration
  [
    (identifier)
    (scoped_identifier)
  ] @local.definition.namespace)

; Classes - capture full node for context
(class_declaration
  name: (identifier) @local.definition.class) @local.definition.class.context

; Interfaces - capture full node for context
(interface_declaration
  name: (identifier) @local.definition.interface) @local.definition.interface.context

; Records - capture full node for context
(record_declaration
  name: (identifier) @local.definition.type) @local.definition.type.context

; Enums - capture full node for context
(enum_declaration
  name: (identifier) @local.definition.enum) @local.definition.enum.context

; Methods - capture full node for signature extraction
(method_declaration
  name: (identifier) @local.definition.method) @local.definition.method.context

; Explicit constructors; a record's compact constructor has no parameter list
; and is not captured.
(constructor_declaration
  name: (identifier) @local.definition.constructor) @local.definition.constructor.context

; Enum constants
(enum_constant
  name: (identifier) @local.definition.enum_variant) @local.definition.enum_variant.context

(local_variable_declaration
  declarator: (variable_declarator
    name: (identifier) @local.definition.var))

(enhanced_for_statement
  ; for (var item : items) {
  name: (identifier) @local.definition.var)

(formal_parameter
  name: (identifier) @local.definition.parameter)

(catch_formal_parameter
  name: (identifier) @local.definition.parameter)

(inferred_parameters
  (identifier) @local.definition.parameter) ; (x,y) -> ...

(lambda_expression
  parameters: (identifier) @local.definition.parameter) ; x -> ...

; The imported name is the final segment of the import's own path, so the
; capture is import-scoped by shape (the runtime does not enforce upstream's
; `#has-ancestor?`, which let every qualified annotation, module name and
; module directive operand become an import).
(import_declaration
  [
    (scoped_identifier
      name: (identifier) @local.definition.import)
    (identifier) @local.definition.import
  ])

(field_declaration
  declarator: (variable_declarator
    name: (identifier) @local.definition.field))

; Interface constants (implicitly public static final)
(constant_declaration
  declarator: (variable_declarator
    name: (identifier) @local.definition.field))

; REFERENCES
(identifier) @local.reference

(type_identifier) @local.reference
