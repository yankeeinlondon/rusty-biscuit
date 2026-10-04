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
    name: (property_identifier) @local.definition.property) @local.definition.property.context)

(public_field_definition
  name: [
    (property_identifier)
    (private_property_identifier)
  ] @local.definition.field) @local.definition.field.context
