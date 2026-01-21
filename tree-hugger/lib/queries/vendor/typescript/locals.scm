; inherits: ecma

(required_parameter
  (identifier) @local.definition)

(optional_parameter
  (identifier) @local.definition)

; x => x
(arrow_function
  parameter: (identifier) @local.definition.parameter)

; ({ a }) => null
(required_parameter
  (object_pattern
    (shorthand_property_identifier_pattern) @local.definition.parameter))

; ({ a: b }) => null
(required_parameter
  (object_pattern
    (pair_pattern
      value: (identifier) @local.definition.parameter)))

; ([ a ]) => null
(required_parameter
  (array_pattern
    (identifier) @local.definition.parameter))

(required_parameter
  (rest_pattern
    (identifier) @local.definition.parameter))

; Interface declarations - capture full node
(interface_declaration
  name: (type_identifier) @local.definition.interface) @local.definition.interface.context

; Type alias declarations - capture full node
(type_alias_declaration
  name: (type_identifier) @local.definition.type) @local.definition.type.context

; Enum declarations - capture full node (separate from types for distinction)
(enum_declaration
  name: (identifier) @local.definition.enum) @local.definition.enum.context
