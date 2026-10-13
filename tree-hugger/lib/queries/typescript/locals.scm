; extends

; Tree Hugger overlay for TypeScript class analysis.
(class_declaration
  name: (type_identifier) @local.definition.class) @local.definition.class.context

(abstract_class_declaration
  name: (type_identifier) @local.definition.class) @local.definition.class.context

(method_definition
  name: [
    (property_identifier)
    (private_property_identifier)
  ] @local.definition.method
  (#set! definition.var.scope parent)) @local.definition.method.context

(abstract_method_signature
  name: (property_identifier) @local.definition.method) @local.definition.method.context

; Interface members; type-literal members are not declarations.
(interface_body
  (method_signature
    name: (property_identifier) @local.definition.method) @local.definition.method.context)

(interface_body
  (property_signature
    name: [
      (property_identifier)
      (number)
    ] @local.definition.property) @local.definition.property.context)

; A quoted name is reported without its quotes.
(interface_body
  (property_signature
    name: (string
      (string_fragment) @local.definition.property)) @local.definition.property.context)

(public_field_definition
  name: [
    (property_identifier)
    (private_property_identifier)
    (number)
  ] @local.definition.field) @local.definition.field.context

(public_field_definition
  name: (string
    (string_fragment) @local.definition.field)) @local.definition.field.context

; Enum members. A bare member has no node of its own: its name is its
; declaration.
(enum_body
  (property_identifier) @local.definition.enum_variant)

(enum_body
  (enum_assignment
    name: (property_identifier) @local.definition.enum_variant) @local.definition.enum_variant.context)

; A quoted member is reported without its quotes.
(enum_body
  (string
    (string_fragment) @local.definition.enum_variant))

(enum_body
  (enum_assignment
    name: (string
      (string_fragment) @local.definition.enum_variant)) @local.definition.enum_variant.context)

; Named namespaces: `namespace A {}`, `namespace A.B {}` and `module A {}`.
; A string-named ambient `declare module "x" {}` declares no namespace.
(internal_module
  name: [
    (identifier)
    (nested_identifier)
  ] @local.definition.namespace) @local.definition.namespace.context

(module
  name: [
    (identifier)
    (nested_identifier)
  ] @local.definition.namespace) @local.definition.namespace.context

; Overloads and ambient functions bind callable names even without a body.
(function_signature
  name: (identifier) @local.definition.function) @local.definition.function.context
