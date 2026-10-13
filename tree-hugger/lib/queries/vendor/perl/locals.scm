; Tree Hugger's own Perl locals query (tree-sitter-perl 1.1.2). It is not
; vendored from nvim-treesitter, which has no Perl locals query.

; Scopes
[
  (block)
  (function_definition)
] @local.scope

; Named subs; anonymous subs declare no name
(function_definition
  name: (identifier) @local.definition.function) @local.definition.function.context

; Packages: the statement form (`package Foo;`) scopes the declarations after
; it, the block form (`package Foo { ... }`) the ones inside it
(package_statement
  (package_name) @local.definition.module) @local.definition.module.context

; `my`, `our` and `state` declare variables; `local` only overrides an existing
; one, so it is not a declaration
((variable_declaration
  (scope) @_scope
  variable_name: (_) @local.definition.var)
  (#any-of? @_scope "my" "our" "state"))

; List form: `my ($a, $b) = @_;` declares each name
((variable_declaration
  (scope) @_scope
  (array
    [
      (scalar_variable)
      (array_variable)
      (hash_variable)
    ] @local.definition.var))
  (#any-of? @_scope "my" "our" "state"))
