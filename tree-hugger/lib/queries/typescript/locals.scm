; extends

; Tree Hugger overlay for TypeScript class analysis.
(class_declaration
  name: (type_identifier) @local.definition.class) @local.definition.class.context

(method_definition
  name: (property_identifier) @local.definition.method
  (#set! definition.var.scope parent)) @local.definition.method.context

(public_field_definition
  name: [
    (property_identifier)
    (private_property_identifier)
  ] @local.definition.field) @local.definition.field.context
