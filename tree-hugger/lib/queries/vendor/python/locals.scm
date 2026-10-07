; Program structure
(module) @local.scope

(class_definition
  body: (block
    (expression_statement
      (assignment
        left: (identifier) @local.definition.field)))) @local.scope

(class_definition
  body: (block
    (expression_statement
      (assignment
        left: [
          (pattern_list
            (identifier) @local.definition.field)
          (tuple_pattern
            (identifier) @local.definition.field)
          (list_pattern
            (identifier) @local.definition.field)
        ])))) @local.scope

; Imports: only the name each import binds. `import a.b` binds `a`, its
; first segment; `from m import x` and `from __future__ import x` bind `x`;
; an alias binds the alias; `from a.b import *` is an on-demand import named
; by its module's last segment (`b`), or by the relative prefix (`.`) alone.
(aliased_import
  alias: (identifier) @local.definition.import) @local.scope

(import_statement
  name: (dotted_name
    .
    (identifier) @local.definition.import)) @local.scope

(import_from_statement
  name: (dotted_name
    (identifier) @local.definition.import)) @local.scope

(future_import_statement
  name: (dotted_name
    (identifier) @local.definition.import))

(import_from_statement
  module_name: (dotted_name
    (identifier) @local.definition.import
    .)
  (wildcard_import))

(import_from_statement
  module_name: (relative_import
    (dotted_name
      (identifier) @local.definition.import
      .))
  (wildcard_import))

(import_from_statement
  module_name: (relative_import
    (import_prefix) @local.definition.import
    .)
  (wildcard_import))

; Function with parameters, defines parameters
(parameters
  (identifier) @local.definition.parameter)

(default_parameter
  name: (identifier) @local.definition.parameter)

(typed_parameter
  (identifier) @local.definition.parameter)

(typed_default_parameter
  name: (identifier) @local.definition.parameter)

; *args parameter
(parameters
  (list_splat_pattern
    (identifier) @local.definition.parameter))

; **kwargs parameter
(parameters
  (dictionary_splat_pattern
    (identifier) @local.definition.parameter))

; Function defines function and scope - capture full node for signature extraction
((function_definition
  name: (identifier) @local.definition.function) @local.scope @local.definition.function.context
  (#set! definition.function.scope "parent"))

; Class defines type and scope - capture full node
((class_definition
  name: (identifier) @local.definition.class) @local.scope @local.definition.class.context
  (#set! definition.type.scope "parent"))

; Methods - capture full node for signature extraction
(class_definition
  body: (block
    (function_definition
      name: (identifier) @local.definition.method) @local.definition.method.context))

; Loops
; not a scope!
(for_statement
  left: (pattern_list
    (identifier) @local.definition.var))

(for_statement
  left: (tuple_pattern
    (identifier) @local.definition.var))

(for_statement
  left: (identifier) @local.definition.var)

; not a scope!
;(while_statement) @local.scope
; for in list comprehension
(for_in_clause
  left: (identifier) @local.definition.var)

(for_in_clause
  left: (tuple_pattern
    (identifier) @local.definition.var))

(for_in_clause
  left: (pattern_list
    (identifier) @local.definition.var))

(dictionary_comprehension) @local.scope

(list_comprehension) @local.scope

(set_comprehension) @local.scope

; Assignments
(assignment
  left: (identifier) @local.definition.var)

(assignment
  left: (pattern_list
    (identifier) @local.definition.var))

(assignment
  left: (tuple_pattern
    (identifier) @local.definition.var))


; Walrus operator  x := 1
(named_expression
  name: (identifier) @local.definition.var)

(as_pattern
  alias: (as_pattern_target) @local.definition.var)

; REFERENCES
(identifier) @local.reference
