; Functions definitions - capture full node for signature extraction
(function_declarator
  declarator: (identifier) @local.definition.function) @local.definition.function.context

(preproc_function_def
  name: (identifier) @local.definition.macro) @local.scope

(preproc_def
  name: (identifier) @local.definition.macro)

(pointer_declarator
  declarator: (identifier) @local.definition.var)

(parameter_declaration
  declarator: (identifier) @local.definition.parameter)

(init_declarator
  declarator: (identifier) @local.definition.var)

(array_declarator
  declarator: (identifier) @local.definition.var)

(declaration
  declarator: (identifier) @local.definition.var)

; A name behind an attribute or extra parentheses: `int x [[maybe_unused]];`,
; `int f [[deprecated]] (int);`, `int (f)(int);`. The pattern sees only the
; name's parent; Tree Hugger's classifier makes the name a function (or
; method) when the declarator chain declares one.
(attributed_declarator
  (identifier) @local.definition.var)

(parenthesized_declarator
  (identifier) @local.definition.var)

; Enum definitions - capture full node for context
(enum_specifier
  name: (_) @local.definition.enum
  (enumerator_list)) @local.definition.enum.context

; Enum enumerators
(enumerator
  name: (identifier) @local.definition.enum_variant) @local.definition.enum_variant.context

; Type / Struct / Enum
(field_declaration
  declarator: (field_identifier) @local.definition.field)

; A field name under any declarator chain: `*p`, `**p`, `a[3]`, `*a[2]`,
; `(*callback)(int)`. A field name directly in a `function_declarator` is a
; method, never a field.
(pointer_declarator
  declarator: (field_identifier) @local.definition.field)

(array_declarator
  declarator: (field_identifier) @local.definition.field)

(parenthesized_declarator
  (field_identifier) @local.definition.field)

(attributed_declarator
  (field_identifier) @local.definition.field)

; Type definitions - capture full node for context
(type_definition
  declarator: (type_identifier) @local.definition.type_alias) @local.definition.type_alias.context

; A typedef name under a declarator chain: `typedef int *P;`,
; `typedef int (*Fn)(int);`, `typedef int A[3];`, `typedef int F(int);`,
; `typedef int T [[maybe_unused]];`. A type name sits under a declarator only
; in a typedef.
(pointer_declarator
  declarator: (type_identifier) @local.definition.type_alias)

(array_declarator
  declarator: (type_identifier) @local.definition.type_alias)

(function_declarator
  declarator: (type_identifier) @local.definition.type_alias)

(parenthesized_declarator
  (type_identifier) @local.definition.type_alias)

(attributed_declarator
  (type_identifier) @local.definition.type_alias)

; Struct and union definitions (with a body) and declarations written alone
; (`struct S;`), capturing the full node for context. An elaborated use
; (`struct S x;`, a parameter `struct S *p`, `typedef struct S T;`) is a
; type reference, never a declaration.
(struct_specifier
  name: (type_identifier) @local.definition.type
  body: (_)) @local.definition.type.context

(union_specifier
  name: (type_identifier) @local.definition.type
  body: (_)) @local.definition.type.context

(translation_unit
  [
    (struct_specifier
      name: (type_identifier) @local.definition.type
      !body)
    (union_specifier
      name: (type_identifier) @local.definition.type
      !body)
  ] @local.definition.type.context)

(declaration
  type: [
    (struct_specifier
      name: (type_identifier) @local.definition.type
      !body)
    (union_specifier
      name: (type_identifier) @local.definition.type
      !body)
  ] @local.definition.type.context
  !declarator)

(field_declaration
  type: [
    (struct_specifier
      name: (type_identifier) @local.definition.type
      !body)
    (union_specifier
      name: (type_identifier) @local.definition.type
      !body)
  ] @local.definition.type.context
  !declarator)

; goto
(labeled_statement
  (statement_identifier) @local.definition)

; References
(identifier) @local.reference

((field_identifier) @local.reference
  (#set! reference.kind "field"))

((type_identifier) @local.reference
  (#set! reference.kind "type"))

(goto_statement
  (statement_identifier) @local.reference)

; Scope
[
  (for_statement)
  (if_statement)
  (while_statement)
  (translation_unit)
  (function_definition)
  (compound_statement) ; a block in curly braces
  (struct_specifier)
] @local.scope
