; Imports: only the name each import binds. A path's last segment
; (`use a;`, `use a::b;`, `g::h` in a list), an alias (`use a as b;`,
; `extern crate y as z;`), and for `self` in a list (`use a::{self};`) the
; list's own path's last segment. A glob (`use a::b::*;`, `use a::{*}`)
; binds nothing; it is an on-demand import named by its path's last segment
; (`b`, `a`).
(extern_crate_declaration
  name: (identifier) @local.definition.import
  !alias)

(extern_crate_declaration
  alias: (identifier) @local.definition.import)

(use_declaration
  argument: (identifier) @local.definition.import)

(use_declaration
  argument: (scoped_identifier
    name: (identifier) @local.definition.import))

(use_as_clause
  alias: (identifier) @local.definition.import)

(use_list
  (identifier) @local.definition.import) ; use std::process::{Child, Command, Stdio};

(use_list
  (scoped_identifier
    name: (identifier) @local.definition.import))

(scoped_use_list
  path: [
    (identifier) @local.definition.import
    (scoped_identifier
      name: (identifier) @local.definition.import)
  ]
  list: (use_list
    (self)))

(scoped_use_list
  path: [
    (identifier) @local.definition.import
    (scoped_identifier
      name: (identifier) @local.definition.import)
  ]
  list: (use_list
    (use_wildcard
      .
      "*")))

(use_wildcard
  [
    (identifier) @local.definition.import
    (scoped_identifier
      name: (identifier) @local.definition.import)
    (crate) @local.definition.import
    (super) @local.definition.import
    (self) @local.definition.import
  ])

; Functions - capture full node for signature extraction
(function_item
  name: (identifier) @local.definition.function) @local.definition.function.context

; Bodyless trait method declarations; the direct-member rule makes them methods
(function_signature_item
  name: (identifier) @local.definition.function) @local.definition.function.context

; Methods (functions with self parameter)
(function_item
  name: (identifier) @local.definition.method
  parameters: (parameters
    (self_parameter))) @local.definition.method.context

; Variables
(parameter
  pattern: (identifier) @local.definition.var)

(let_declaration
  pattern: (identifier) @local.definition.var)

(const_item
  name: (identifier) @local.definition.var)

; `static` items: variables with static storage, not constants
(static_item
  name: (identifier) @local.definition.var) @local.definition.var.context

(tuple_pattern
  (identifier) @local.definition.var)

; Only patterns whose identifier children are binding positions: a range
; bound, a path segment, a generic pattern's path and a macro name read.
(let_condition
  pattern: [
    (captured_pattern
      (identifier) @local.definition.var)
    (mut_pattern
      (identifier) @local.definition.var)
    (or_pattern
      (identifier) @local.definition.var)
    (ref_pattern
      (identifier) @local.definition.var)
    (reference_pattern
      (identifier) @local.definition.var)
    (slice_pattern
      (identifier) @local.definition.var)
    (tuple_pattern
      (identifier) @local.definition.var)
    (tuple_struct_pattern
      (identifier) @local.definition.var)
  ])

(tuple_struct_pattern
  (identifier) @local.definition.var)

(closure_parameters
  (identifier) @local.definition.var)

(self_parameter
  (self) @local.definition.var)

(for_expression
  pattern: (identifier) @local.definition.var)

; Types
(struct_item
  name: (type_identifier) @local.definition.type) @local.definition.type.context

(union_item
  name: (type_identifier) @local.definition.type) @local.definition.type.context

; Type aliases, including associated types in traits and impls
(type_item
  name: (type_identifier) @local.definition.type_alias) @local.definition.type_alias.context

(associated_type
  name: (type_identifier) @local.definition.type_alias) @local.definition.type_alias.context

; Enums (separate from types for distinction)
(enum_item
  name: (type_identifier) @local.definition.enum) @local.definition.enum.context

; Traits
(trait_item
  name: (type_identifier) @local.definition.trait) @local.definition.trait.context

; Fields
(field_declaration
  name: (field_identifier) @local.definition.field)

(enum_variant
  name: (identifier) @local.definition.enum_variant)

; References
(identifier) @local.reference

((type_identifier) @local.reference
  (#set! reference.kind "type"))

((field_identifier) @local.reference
  (#set! reference.kind "field"))

; Macros
(macro_definition
  name: (identifier) @local.definition.macro)

; Module
(mod_item
  name: (identifier) @local.definition.module) @local.definition.module.context

; Scopes
[
  (block)
  (function_item)
  (closure_expression)
  (while_expression)
  (for_expression)
  (loop_expression)
  (if_expression)
  (match_expression)
  (match_arm)
  (struct_item)
  (enum_item)
  (impl_item)
] @local.scope
