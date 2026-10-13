; extends

; Tree Hugger overlay for ECMAScript class analysis.
(class_declaration
  name: (identifier) @local.definition.class) @local.definition.class.context

(method_definition
  name: (property_identifier) @local.definition.method
  (#set! definition.var.scope parent)) @local.definition.method.context

(field_definition
  property: [
    (property_identifier)
    (private_property_identifier)
    (number)
  ] @local.definition.field) @local.definition.field.context

; A quoted name is reported without its quotes.
(field_definition
  property: (string
    (string_fragment) @local.definition.field)) @local.definition.field.context
