; inherits: c

; Parameters
(variadic_parameter_declaration
  declarator: (variadic_declarator
    (identifier) @local.definition.parameter))

(optional_parameter_declaration
  declarator: (identifier) @local.definition.parameter)

; Class / struct definitions
(class_specifier) @local.scope

(reference_declarator
  (identifier) @local.definition.var)

(reference_declarator
  (field_identifier) @local.definition.field)

(reference_declarator
  (type_identifier) @local.definition.type_alias)

(variadic_declarator
  (identifier) @local.definition.var)

; Struct, union and class definitions (with a body) and declarations written
; alone (`class C;`, `struct M;` in a namespace, class, template or linkage
; body); an elaborated use (`class C *c;`, a parameter `struct S &s`) is a
; type reference, never a declaration. The C query supplies unqualified
; `struct`/`union` definitions and their top-level and `declaration` forms.
(struct_specifier
  name: (qualified_identifier
    name: (type_identifier) @local.definition.type)
  body: (_))

; Class definitions - capture full node for context
(class_specifier
  name: (type_identifier) @local.definition.class
  body: (_)) @local.definition.class.context

(concept_definition
  name: (identifier) @local.definition.type) @local.definition.type.context

(class_specifier
  name: (qualified_identifier
    name: (type_identifier) @local.definition.class)
  body: (_)) @local.definition.class.context

(translation_unit
  [
    (struct_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.type)
      !body)
    (class_specifier
      name: (type_identifier) @local.definition.class
      !body) @local.definition.class.context
    (class_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.class)
      !body) @local.definition.class.context
  ])

(declaration_list
  [
    (struct_specifier
      name: (type_identifier) @local.definition.type
      !body) @local.definition.type.context
    (union_specifier
      name: (type_identifier) @local.definition.type
      !body) @local.definition.type.context
    (struct_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.type)
      !body)
    (class_specifier
      name: (type_identifier) @local.definition.class
      !body) @local.definition.class.context
    (class_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.class)
      !body) @local.definition.class.context
  ])

(template_declaration
  [
    (struct_specifier
      name: (type_identifier) @local.definition.type
      !body) @local.definition.type.context
    (union_specifier
      name: (type_identifier) @local.definition.type
      !body) @local.definition.type.context
    (struct_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.type)
      !body)
    (class_specifier
      name: (type_identifier) @local.definition.class
      !body) @local.definition.class.context
    (class_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.class)
      !body) @local.definition.class.context
  ])

(declaration
  type: [
    (struct_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.type)
      !body)
    (class_specifier
      name: (type_identifier) @local.definition.class
      !body) @local.definition.class.context
    (class_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.class)
      !body) @local.definition.class.context
  ]
  !declarator)

(field_declaration
  type: [
    (struct_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.type)
      !body)
    (class_specifier
      name: (type_identifier) @local.definition.class
      !body) @local.definition.class.context
    (class_specifier
      name: (qualified_identifier
        name: (type_identifier) @local.definition.class)
      !body) @local.definition.class.context
  ]
  !declarator)

(alias_declaration
  name: (type_identifier) @local.definition.type_alias) @local.definition.type_alias.context

;template <typename T>
(type_parameter_declaration
  (type_identifier) @local.definition.type)

(template_declaration) @local.scope

; Namespaces
(namespace_definition
  name: (namespace_identifier) @local.definition.namespace
  body: (_) @local.scope)

(namespace_definition
  name: (nested_namespace_specifier) @local.definition.namespace
  body: (_) @local.scope)

((namespace_identifier) @local.reference
  (#set! reference.kind "namespace"))

; Function definitions - capture full node for context
; An explicit specialization's name (`template <> void f<int>() {}`,
; `void N::f<int>()`, `void S::m<int>()`). A `template_function` or
; `template_method` outside a function declarator is a use (`f<int>()`,
; `obj.template m<int>()`), never a definition.
(function_declarator
  declarator: [
    (template_function
      name: (identifier) @local.definition.function)
    (qualified_identifier
      name: (template_function
        name: (identifier) @local.definition.function))
  ]) @local.scope @local.definition.function.context

(function_declarator
  declarator: (qualified_identifier
    name: (template_method
      name: (field_identifier) @local.definition.method))) @local.scope @local.definition.method.context

(function_declarator
  declarator: (qualified_identifier
    name: (identifier) @local.definition.function)) @local.scope @local.definition.function.context

; The same qualified name behind an attribute or parentheses:
; `int S::f [[deprecated]] (int) {…}`.
(function_declarator
  declarator: [
    (attributed_declarator
      (qualified_identifier
        name: (identifier) @local.definition.function))
    (parenthesized_declarator
      (qualified_identifier
        name: (identifier) @local.definition.function))
  ]) @local.scope @local.definition.function.context

; Member functions, declared or defined in the class body, under any return
; declarator (`int *m(int);`, `int (*m(int))(long) {…}`): a field name is the
; function's own declarator only for a member function, never for a
; function-pointer field (`int (*p)(int)` puts the name under a pointer).
; Tree Hugger supplies the method's declaration as its context.
(function_declarator
  declarator: (field_identifier) @local.definition.method)

(lambda_expression) @local.scope

; Control structures
(try_statement
  body: (_) @local.scope)

(catch_clause) @local.scope

(requires_expression) @local.scope
