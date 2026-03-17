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
  ] @local.definition.field) @local.definition.field.context
