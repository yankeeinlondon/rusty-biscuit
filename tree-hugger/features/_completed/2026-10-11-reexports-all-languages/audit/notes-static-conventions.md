# Audit notes: Python, Go, Perl, Lua (static conventions)

Scope: Phase 1 per-language audit of the re-exports spec, for the four
languages whose "re-export" is mostly a convention or a type-alias rule rather
than a dedicated statement. Samples live in `samples/{python,go,perl,lua}/`
(`pos-*` positive, `neg-*` negative, `amb-*` parse cleanly but are ambiguous
or dynamic and must yield a coverage limit, `gap-*` grammar defects,
`legacy-*` legacy forms).

Parse evidence: `th-probe` built on the repo's pinned grammars
(tree-sitter-python 0.25.0, tree-sitter-go 0.25.0, tree-sitter-perl 1.1.2,
tree-sitter-lua 0.5.0; tree-sitter 0.26.9). Every sample parsed with
`has_error=false` and no `MISSING` nodes (python 41/41, go 33/33,
perl 35/35, lua 40/40). A clean parse is only a statement about syntax, not
about semantics; the one case where the tree is clean but wrong is called out
as a grammar gap (Perl `@Pkg::VAR`).

Sources are the live docs fetched 2026-10-11. Where a fetch was truncated or
did not contain the passage, the cell says **source not verified**.

Terminology used in the "Expected result" columns:
`exported` = name visible to consumers, `original` = name / member path in the
source module, `local` = same-file binding the forward goes through,
`source` = module path as written, `scope` = enclosing scope, `form` =
named / wildcard / namespace-module, `cond` = conditional, `limit` = coverage
limit reason (the extractor must report this instead of guessing).

---

## Python

Versions: Python 3.15 language reference (docs page footer "Last updated Oct
11, 2026") and the typing specification "Distributing type information"
(living spec, `typing.python.org/en/latest/spec/distributing.html`).
Grammar nodes: `import_statement`, `import_from_statement`,
`future_import_statement`, `aliased_import`, `wildcard_import`,
`relative_import` (`import_prefix` + optional `dotted_name`),
`assignment`, `augmented_assignment`, `call` on an `attribute`.

### Semantics (verified)

- Reference, section "The import statement"
  (<https://docs.python.org/3/reference/simple_stmts.html>): `import a.b as c`
  binds `c` directly to module `a.b`; `from m import a as b` binds `b` to
  `m.a` (or submodule `m.a` if attribute missing). A star import binds the
  module's "public names": `__all__` if defined ("a sequence of strings
  which are names defined or imported by that module"), otherwise every
  namespace name not starting with `_`. Star import is a module-level-only
  construct (SyntaxError elsewhere).
- Tutorial 6.4.1 (<https://docs.python.org/3/tutorial/modules.html>): in a
  package `__all__` may name submodules; absent `__all__`, star import takes
  "whatever names are defined" in `__init__.py`.
- Typing spec (<https://typing.python.org/en/latest/spec/distributing.html>):
  for typed libraries, `import X as X`, `from Y import X as X` and
  `from Y import *` re-export; every other import is private by default; a
  module-level `__all__` overrides the rules. Supported `__all__` idioms:
  `= (..)`, `= [..]`, `+= [..]`, `+= submodule.__all__`, `.extend([..])`,
  `.extend(submodule.__all__)`, `.append('a')`, `.remove('a')`. Anything else
  is outside the specified behavior, "so that type checkers can statically
  determine the value of `__all__`".

Two distinct notions therefore coexist and must be recorded separately in a
result (a `basis`-style field is recommended, final naming is for design):

1. **Runtime exposure** (reference): every non-underscore module-level import
   binding is visible as an attribute and to star importers when `__all__` is
   absent.
2. **Declared interface** (typing spec / `__all__`): redundant alias forms
   and `__all__` membership.

Classification: **convention** (no dedicated re-export syntax). Relative
imports and `__init__.py` are not required; every `.py` module is in scope.

### Construct table

| # | Construct | Sample | Nodes / fields | Class | Expected result (positive cases) | Limit / negative |
|---|-----------|--------|----------------|-------|----------------------------------|------------------|
| P1 | `from m import a` | `pos-from-import.py` | `import_from_statement` `module_name: dotted_name`, `name: dotted_name` | convention | exported `a`, original `a`, local `a`, source `m`, form named, scope module. Basis: runtime exposure only unless in `__all__` / alias-redundant | Never a declared re-export by itself (typing spec). Needs no-rebind check (P12) |
| P2 | `from m import a as b` | `pos-from-import-alias.py` | `name: aliased_import` (`name: dotted_name`, `alias: identifier`) | convention | exported `b`, original `a`, local `b`, source `m` | `from m import a as a` (redundant alias) is the typing-spec declared form: detect `alias == name` |
| P3 | `import m` / `import m as n` | `pos-import-module.py`, `pos-import-alias.py` | `import_statement` `name: dotted_name` or `aliased_import` | convention | exported `m` (or `n`), original = whole module, source `m`, form namespace-module | `import m as m` is the redundant-alias declared form |
| P4 | `import pkg.sub` / `as s` | `pos-import-dotted.py`, `pos-import-dotted-alias.py` | `dotted_name` has two `identifier` children | convention | no alias: local binding is top-level `pkg` (source `pkg.sub`, form namespace-module, only `pkg` bound). With alias: exported `s`, source `pkg.sub` | Without alias the exported name is NOT the source path. Docs page fetched; the unaliased binding sentence was not in the excerpt returned, so that detail is **source not verified** (the aliased form is verified) |
| P5 | multi / parenthesized / aliased lists | `pos-from-import-multi.py`, `pos-from-import-paren.py` | repeated `name:` fields on one `import_from_statement` | convention | one entry per `name:` | Parens produce no extra node |
| P6 | `from m import *` | `pos-star.py` | `import_from_statement` + unnamed-field child `wildcard_import "*"` (no field name) | convention (typing spec: declared re-export) | one wildcard entry, source `m`, form wildcard, names unknown | Must not enumerate names. If `__all__` literal present, names in it with no same-file binding are "bound by wildcard or definition: unresolved" |
| P7 | relative imports | `pos-relative.py`, `pos-relative-dot.py`, `pos-relative-parent.py`, `pos-relative-star.py` | `module_name: relative_import` -> `import_prefix` (`.`/`..` text gives level) + optional `dotted_name` | convention | source recorded as written with level: `.m`, `..m`, `.` ; `from . import x`: source `.`, original `x`, may be attribute or submodule | Resolution to a path is cross-file: do not resolve. `x` kind (submodule vs attribute) unknown |
| P8 | literal `__all__ = [..]` / `(..)` | `pos-all-list.py`, `pos-all-tuple.py`, `pos-all-annotated.py` | `assignment` `left: identifier "__all__"` `right: list` or `tuple` of `string` (`string_content`); annotated form adds `type:` field, no error | convention | for each string element equal to a same-file import local: exported = that string, original/source from the import | Elements that are not plain `string` (names, splats, f-strings): list becomes a coverage limit `all-computed` |
| P9 | `__all__ += [..]` | `pos-all-augassign.py` | `augmented_assignment` left/right | convention | adds elements | Right side must be list/tuple literal; `+= mod.__all__` is a typing-spec idiom but needs cross-file: limit `all-cross-module` |
| P10 | `__all__.extend([..])` / `.append("a")` | `pos-all-extend.py`, `pos-all-append.py` | expression statement `call` -> `function: attribute` (`object: identifier "__all__"`, `attribute: identifier extend/append`) | convention | adds elements | `.remove("a")` (typing spec) removes: model as removal; any other method -> limit |
| P11 | literal concatenation | `pos-all-concat-literal.py` | `binary_operator` of two `list`s | convention | allowed only if every operand literal | Not in the typing-spec list; treat as supported-by-extractor extension or limit (design decision) |
| P12 | rebinding after import | `neg-rebound.py`, `neg-rebound-def.py`, `neg-rebound-all.py` | later `assignment` / `function_definition` with same name at module level | n/a | Entry is not emitted as a forward when a later (or earlier, if it overrides) module-scope binding of the same name exists | limit `rebound`. Needs: all module-scope binding sites (assignment, augassign, `def`, `class`, `for`/`with`/`except as`, `del`, `import` again, walrus) |
| P13 | absent `__all__` | `neg-all-absent.py` | no `__all__` assignment | convention | Runtime-exposure entries only (P1-P4) | coverage note `all-absent`: interface = all non-underscore bindings; underscore-named imports are not public (`neg-underscore.py`) |
| P14 | computed `__all__` | `neg-all-computed.py`, `neg-all-call.py`, `neg-all-splat.py`, `neg-all-nonstring.py`, `neg-all-sub-assign.py` | `list_comprehension`, `call`, `list_splat`, non-string element, `subscript` target | n/a | No `__all__`-derived entries at all | limit `all-computed`. Imports remain reported with basis runtime-exposure at most |
| P15 | conditional `__all__` | `neg-all-shadow-cond.py`, `neg-all-dunder-del.py` | `__all__` under `if_statement` / `delete_statement` | n/a | none | limit `all-conditional`; two assignments in branches are not merged |
| P16 | `__all__` inside function/class | `neg-all-in-function.py` | assignment under `function_definition` body | n/a | ignored (not the module's `__all__`) | scope check required: only direct module-level statements (and `if`/`try` at module level -> conditional) |
| P17 | conditional / try imports | `pos-try-import.py`, `pos-if-import.py`, `pos-import-typing-if.py` | import under `try_statement body: block` / `except_clause` / `if_statement consequence: block` | convention | entries with `cond = true` (try, except, if/elif/else, `TYPE_CHECKING`). Two sites binding the same name stay two entries | `TYPE_CHECKING` imports exist only for checkers: still conditional, never unconditional |
| P18 | imports in function/class | `neg-import-in-function.py`, `neg-import-in-class.py` | import nested in `function_definition` / `class_definition` | n/a | not module interface | scope: class body names are class attributes, not module names |
| P19 | `__future__` | `neg-future.py` | `future_import_statement` (distinct node) | n/a | ignored | compiler directive, never a re-export |
| P20 | indirect | `neg-assign-alias.py`, `neg-importlib.py` | `assignment` of an `identifier` / `attribute` whose object is a `call` | n/a | `b = a` is an alias of an import: not reported (no import syntax) | dynamic import never supported; `b = a` may be supported only as a documented extension |

### Minimal same-file facts (accepted patterns)

- Import-forward: an import statement whose parent chain reaches the module
  through only `if`/`try`/`with`/`else`-style blocks (conditional) and no
  function/class.
- `__all__` membership: the name's last binding at module scope is the import
  (no later or ambiguous rebinding), and `__all__` is built only from literal
  assignment/augmented-assign/extend/append/remove at module scope.
- Wildcard: only the module's `from m import *` statement; no same-file facts
  beyond conditionality.

### Python summary

Convention-only. Supported statically: P1-P10, P13, P17 with the rebind and
scope checks. Coverage limits to record: `all-computed`, `all-conditional`,
`all-cross-module` (`+= sub.__all__`), `rebound`, `wildcard-names-unknown`,
`relative-unresolved`. Surprise: the typing-spec "declared" set (redundant
alias, `__all__`, star) is smaller than the runtime "exposed" set (every
public import), so the result needs a basis marker or `unused-import` will
either over- or under-suppress.

---

## Go

Versions: Go language specification, "Language version go1.27 (May 26,
2026)" (<https://go.dev/ref/spec>); `cmd/go` go1.27.2 for build constraints
(<https://pkg.go.dev/cmd/go#hdr-Build_constraints>, excerpt truncated).
Grammar nodes: `type_declaration` containing `type_alias` (name, optional
`type_parameters`, `type`) or `type_spec`; `import_spec` (optional
`name: package_identifier | blank_identifier | dot`, `path:
interpreted_string_literal`); `qualified_type` (`package`, `name`);
`var_spec`/`const_spec` (`name`, optional `type`, `value: expression_list`)
with `selector_expression` (`operand`, `field`).

### Semantics (verified, go.dev/ref/spec go1.27)

- Alias declaration: "An alias declaration binds an identifier to the given
  type" (Go 1.9); with type parameters it is a generic alias (Go 1.24),
  "must be instantiated when used". Parses in 0.25.0.
- Type definition: "creates a new, distinct type", "does not inherit any
  methods bound to the given type". So `type X pkg.X` is not a forward.
- Exported identifier: first letter uppercase **and** "declared in the package
  block" (or a field/method name). So a function-local alias is never exported.
- Import declaration: the import name is declared in the **file block**; dot
  import puts the package's exported identifiers in the importing file block
  unqualified; `_` imports for side effects only. An import is not part of
  the importing package's API.
- Not stated by the spec text fetched: that an alias and its target are the
  identical type (follows from "binds an identifier to the given type", the
  identity sentence itself was not fetched: inferred).

### Construct table

| # | Construct | Sample | Nodes / fields | Class | Expected result | Limit / negative |
|---|-----------|--------|----------------|-------|-----------------|------------------|
| G1 | `type X = pkg.X` | `pos-alias.go` | `type_declaration > type_alias` (`name: type_identifier`, `type: qualified_type` (`package: package_identifier`, `name: type_identifier`)) | **dedicated syntax** for naming another package's type (no "re-export" keyword) | exported `X`, original `X`, local = import binding `pkg`, source = import path of that binding, form named, scope package | `package` identifier must match a file-level import name (explicit alias, or implied package name). Implied name is not derivable from the path (see gap Go-1) |
| G2 | renamed alias | `pos-alias-renamed.go` | same | dedicated | exported `Y`, original `X` | -- |
| G3 | aliased import | `pos-alias-aliased-import.go` | `import_spec name: package_identifier "q"` | dedicated | source = path, local `q`; this is the only case where the qualifier maps to a path with certainty | -- |
| G4 | grouped alias | `pos-alias-grouped.go` | several `type_alias` under one `type_declaration` | dedicated | one entry per `type_alias` | -- |
| G5 | generic alias | `pos-alias-generic.go` | `type_alias` with `type_parameters: type_parameter_list`; `type: generic_type` (`type: qualified_type`, `type_arguments`) | dedicated (Go 1.24) | exported `G`, original `G`, flag `generic` | Pre-1.24 toolchains reject it: record only; it is fine with this grammar |
| G6 | pointer/composite alias | `pos-alias-pointer.go` | `type: pointer_type > qualified_type` | no (exposes a derived type) | Not a forward of a named entity | Nearest related; only a bare `qualified_type` (optionally `generic_type`) qualifies |
| G7 | defined type | `neg-defined-type.go`, `neg-defined-type-grouped.go` | `type_declaration > type_spec` (not `type_alias`) | no applicable construct | none | New distinct type; no methods inherited. The only syntactic difference is node type `type_spec` vs `type_alias` |
| G8 | unexported alias | `neg-alias-unexported.go` | lowercase `name` | n/a | none | first letter must be Unicode uppercase (Lu) |
| G9 | local / builtin alias | `neg-alias-local.go`, `neg-alias-builtin.go` | `type: type_identifier` / `map_type` | n/a | none | No import involved |
| G10 | alias in function | `neg-alias-in-func.go` | `type_alias` under `function_declaration > block` | n/a | none (not package block) | scope check against `source_file` parent |
| G11 | shadowed qualifier | `neg-alias-shadowed.go` | `qualified_type` inside function whose parameter is also named `pkg` | n/a | none | Already excluded by G10; for package-level forms no shadow is possible because a package-block identifier cannot reuse a file-block import name (assumption: spec "file block" scoping, conflict rule itself **source not verified**) |
| G12 | var from import | `pos-var.go`, `pos-var-typed.go`, `pos-func-var.go` | `var_spec` `name: identifier`, `value: expression_list > selector_expression` (`operand`, `field`) | convention (value copy, not an alias) | exported `X`, original member path `pkg.Y`, local `pkg`, kind `value-copy` | Mutable variable: other files of the same package may reassign (`neg-var-mutated.go`), so always carry limit `mutable-variable`; initial value only. `var X, Z = pkg.Y, 1` pairs names with values positionally; the grammar gives two `name:` and one `expression_list`, so pair by index |
| G13 | const from import | `pos-const.go` | `const_spec` same shape | convention | exported `C`, original `pkg.K`, kind `value-copy`, immutable | Spec text on const initialisation not fetched; immutability of constants is language-wide (inferred) |
| G14 | call / computed value | `neg-var-call.go`, `neg-var-mutable-nonconst.go`, `neg-method-expr.go` | `call_expression`, literal in value list | n/a | none (`pkg.New()` is a new value) | Method expression `pkg.T.Method` is a nested selector: not a plain member |
| G15 | function wrapper | `neg-func-wrapper.go` | `function_declaration` whose body returns `pkg.F(x)` | no applicable construct | none | A new function with its own identity; body inspection would be guessing |
| G16 | plain / aliased import | `pos-import-plain.go`, `neg-import-aliased.go` | `import_declaration > import_spec` | no applicable construct | none | File-block binding only; cannot be imported by consumers |
| G17 | blank import | `neg-import-blank.go` | `name: blank_identifier` | no | none | side effects only (verified) |
| G18 | dot import | `neg-import-dot.go`, `amb-dot-import-alias.go` | `name: dot` | no | none | With a dot import present, `type X = Y` has an unqualified target that may come from that package or from this package: limit `dot-import-ambiguous` (no `qualified_type` to anchor on) |
| G19 | embedded field / interface embedding | `neg-embedded-struct.go`, `neg-iface-embed.go` | field with `qualified_type`, `type_elem` | no | none | Promotes members of the *type*, not a package interface (spec asks for this distinction) |
| G20 | build constraints | `pos-alias-build-tag.go` | leading `comment "//go:build linux"` | n/a | `cond = true` for the whole file | Constraint applies to the whole file (page excerpt: "constraints ... must appear near the top of the file"; filename suffix rules **source not verified**, excerpt cut) |

### Minimal same-file facts

- G1-G5: an import spec in the same file whose name (explicit, or derivable)
  equals the `qualified_type.package` text, the declaration directly under
  `source_file`, an uppercase name.
- G12/G13: single-name or index-paired `var_spec`/`const_spec` at package
  level, `value` is exactly one `selector_expression` whose `operand` is an
  import name.
- No dependence on declaration order; Go package scope is order independent.

### Go summary

Dedicated syntax for aliases (G1-G5). Value forwarding (G12/G13) is a
convention that creates a new binding, reported as `value-copy` with a limit
for `var`. Everything else is negative. Nothing in Go lets a package expose an
imported *package*; this is the main reason Go has no namespace-module form.

---

## Perl

Versions: Exporter 5.79 (<https://perldoc.perl.org/Exporter>, page marks Perl
5.44.0 latest); `perlmod` (<https://perldoc.perl.org/perlmod>, Perl 5.44.0).
Grammar nodes: `package_statement` (`package_name`, optional `body: block`),
`use_parent_statement` (dedicated for `use parent`), `use_no_statement`
(`package_name:` field, optional `version`, then a `word_list_qw` or `array`
or string child; also used for `use base`, `use vars`, `use Exporter`,
`use strict`), `require_statement`, assignment represented as
`binary_expression` whose two sides are both `variable:` fields,
`variable_declaration` (`scope "our"`, `variable_name:`), `array_variable`,
`hash_variable`, `word_list_qw` (`list_item`s), `array` (parenthesized list).

### Semantics (verified unless noted)

- Exporter (5.79): `@EXPORT` default exports, `@EXPORT_OK` on request,
  `%EXPORT_TAGS` names "must also appear in @EXPORT or @EXPORT_OK"; "you
  cannot use tags in @EXPORT or @EXPORT_OK"; symbols are by full name with
  optional `&` and sigils `$ @ %` for variables ("functions, scalars,
  arrays, hashes, or typeglobs"). `our @ISA = qw(Exporter)` or
  `use parent 'Exporter'` inherits `import`; `use Exporter 'import'` /
  `qw(import)` exports the `import` method without inheritance. `export_to_level`
  exports "one level above the current package" for modules with their own
  `import`.
- perlmod: `use Module LIST` is "exactly equivalent to" `BEGIN { require
  'Module.pm'; 'Module'->import( LIST ); }`; `use Module ()` does not call
  `import`; Exporter "uses glob aliasing as the import/export mechanism".
- **Not verified**: that an `@EXPORT_OK` entry may name a sub that this
  module itself *imported* (rather than defined). It follows from glob
  aliasing (the imported name lives in the package symbol table), but neither
  fetched page states it. Mark "inferred".
- Whether `use` vs `no` and `use base`/`use fields` details: **source not
  verified** (perlfunc excerpt truncated).

### Construct table

| # | Construct | Sample | Nodes / fields | Class | Expected result | Limit / negative |
|---|-----------|--------|----------------|-------|-----------------|------------------|
| L1 | Exporter via `use parent` | `pos-export-ok-imported.pl`, `pos-use-parent-qw.pl` | `use_parent_statement` with child `string_double_quoted` or `word_list_qw` | convention (Exporter is a library contract; no dedicated syntax) | module is exporter-capable | Required precondition for any entry |
| L2 | Exporter via `use base`/`use Exporter`/`@ISA` | `pos-use-base.pl`, `pos-use-exporter-import.pl`, `pos-use-exporter-import-sq.pl`, `pos-export-imported-isa.pl`, `pos-isa-push.pl`, `pos-vars-isa.pl` | `use_no_statement package_name "base"/"Exporter"`; `binary_expression` with `variable_declaration our @ISA` and `array`/`word_list_qw`; `push @ISA, "Exporter"` is a `call_expression_with_spaced_args`; `use vars` then bare `array_variable` assignment | convention | exporter-capable | Each is a separate syntactic form; `push @ISA` form requires `our @ISA` declaration |
| L3 | `our @EXPORT` / `@EXPORT_OK` literal `qw()` | `pos-export-ok-imported.pl`, `pos-export-imported-isa.pl` | `binary_expression`: left `variable_declaration` (`scope "our"`, `variable_name: array_variable "@EXPORT_OK"`), right `word_list_qw` > `list_item` | convention | for each item that equals a name imported into this package: exported = item, original = item (Exporter has no renaming), local = same name, source = module of the `use`, form named, scope package; `@EXPORT` also flagged `default = true` | Items matching only a local `sub` are not re-exports (L8) |
| L4 | parenthesized string list | `neg-export-paren-list.pl` | right side `array` with `string_double_quoted` children (`normal_comma` between) | convention | same as L3 (accepted); file is named `neg-` because `other` has no import binding, `helper` does | Single-quoted strings: probe shows `string_single_quoted` presumably, **not sampled** |
| L5 | `push @EXPORT_OK, qw(..)` | `pos-push-export.pl` | `call_expression_with_spaced_args > call_expression_with_bareword(function_name push)`, `args: arguments (array_variable, word_list_qw)` | convention | same as L3 | requires a preceding `our @EXPORT_OK` declaration |
| L6 | sigils | `pos-sigil-var.pl` | `list_item "$config"`, `"@list"`, `"&func"` | convention | the imported item's name keeps its sigil; `&func` equals `func` | The `use ... qw($config ...)` import list must contain the same sigil form |
| L7 | `use Mod qw(x)` import binding | `pos-export-ok-imported.pl`, `pos-use-version-list.pl` | `use_no_statement` `package_name:` (identifier children, no `::` token), optional `version`, `word_list_qw` | convention | creates local binding `x` from source `Mod`; version literal is skipped | `use Mod;` (no list) -> `amb-use-no-list.pl`: imports Mod's `@EXPORT` (cross-file): limit `import-list-implicit`. `use Mod ()` -> `neg-use-empty-list.pl` (`array "()"`): no bindings. `qw(:all)` tag -> `amb-use-tag-import.pl`: limit `import-tag-unresolved` |
| L8 | locally defined sub in list | `neg-local-sub.pl` | list names a `sub` defined in the same file | no (this is a plain export) | none | Not a re-export; a name both imported and defined locally: `neg-export-shadow-redef.pl`, limit `redefined-locally` |
| L9 | listed but not imported | `neg-export-not-imported.pl` | list item with no `use ... qw()` binding | n/a | none (unbound: may come from glob assignment or elsewhere) | limit `export-name-unbound` |
| L10 | no Exporter | `neg-no-exporter.pl` | `@EXPORT_OK` without L1/L2 | n/a | none | `@EXPORT_OK` alone does nothing |
| L11 | `%EXPORT_TAGS` | `pos-export-tags-literal.pl`, `amb-export-tags-deref.pl` | `%EXPORT_TAGS` assignment: `hash_variable` left; right is `array` of `call_expression_with_bareword` (key as bareword function-name!) + `fat_comma` + `array_ref > word_list_qw`; deref form is `array_dereference > hash_access_variable_simple` | convention | tag membership recorded as tag -> names; tagged names must also be in `@EXPORT`/`@EXPORT_OK` | Tag keys are parsed as `call_expression_with_bareword`, a grammar quirk (gap Perl-2). `@EXPORT_OK = @{ $EXPORT_TAGS{all} }` is a computed list: limit `export-list-computed` |
| L12 | computed lists | `neg-export-computed.pl`, `neg-export-grep.pl`, `neg-export-var-interp.pl` | `map`/`grep` call, scalar variable | n/a | none | limit `export-list-computed` |
| L13 | glob assignment | `neg-export-glob.pl` | `typeglob` assignment inside `special_block` (BEGIN) | n/a | none | limit `symbol-table-manipulation` |
| L14 | `require` + `->import` | `amb-require-import.pl` | `special_block > block > require_statement; method_invocation (function_name import)` | n/a | none | limit `runtime-import` |
| L15 | custom `import` / `export_to_level` | `neg-custom-import.pl`, `neg-export-to-level.pl` | `subroutine` named `import` | n/a | none | limit `custom-import` (dynamic) |
| L16 | conditional / nested | `neg-export-cond.pl`, `neg-export-in-sub.pl` | statement modifier gives `single_line_statement` wrapping the assignment with `if_simple_statement`; or assignment inside a sub body | n/a | modifier-`if` could be `cond = true`; inside sub: ignored | Choose in design; recommended: `cond = true` for modifiers, limit for sub bodies |
| L17 | package scoping | `pos-block-package.pl`, `neg-export-other-package.pl` | `package_statement` with `body: block`, or flat `package` statements | convention | each export/import pair must belong to the same package (nearest preceding `package_statement` or enclosing block) | multiple packages per file: results carry the package name |
| L18 | qualified variable | `gap-qualified-var.pl` | `@My::Mod::EXPORT_OK = ...` parses as `package_variable` of nested `package_variable`s with `array_variable "@My"`, `scalar_variable "Mod"`, `scalar_variable "EXPORT_OK"` (no error flag) | convention | would be same as L3 | **Grammar gap Perl-1**: wrong tree, silently; treat qualified variable as unsupported (limit) |
| L19 | `@EXPORT_FAIL` | `pos-export-fail.pl` | plain assignment | n/a | no effect on re-export set | ignored |

### Minimal same-file facts

- Same package for: an exporter marker (L1/L2), a literal export list
  (L3-L6, L11 literal), and a `use` import binding whose list is a literal
  `qw()`/string list containing the name.
- No later `sub NAME`, typeglob assignment, or second import of `NAME` in the
  package (otherwise `redefined-locally`/`symbol-table-manipulation`).
- No `import` sub defined in the package (custom importers can re-export
  anything).

### Perl summary

Convention only. Entry shape is `exported == original == local`: Exporter
cannot rename. Supported: L1-L7 with literal lists. Each other form yields a
limit. Surprises: `use parent` has its own node while `use base` does not;
both `use` and `no` share `use_no_statement` (keyword token distinction not
visible in the probe's named-node output, **not verified**); the parenthesized
list is `array`, not a distinct list node.

---

## Lua

Versions: Lua 5.4 reference manual (copyright 2020-2026,
<https://www.lua.org/manual/5.4/manual.html>) and Lua 5.1 manual
(<https://www.lua.org/manual/5.1/manual.html>) for `module`.
Grammar nodes (tree-sitter-lua 0.5.0): `chunk`, `return_statement >
expression_list`, `function_call` (`name:`, `arguments: arguments`),
`string` (`content: string_content`; the same node for `"m"`, `'m'`, `[[m]]`),
`variable_declaration` (`local_declaration:` field) over `assignment_statement
(variable_list, expression_list)`, `dot_index_expression` (`table:`,
`field:`), `bracket_index_expression`, `table_constructor > field (name:,
value:)`, `parenthesized_expression`.

### Semantics (verified)

- 5.4 `require`: returns `package.loaded[modname]`; "If the loader returns
  any non-nil value, `require` assigns the returned value to
  `package.loaded[modname]`"; else assigns `true`; returns the final value.
  Chunks "can return values" (return statement). So a module's interface is
  the chunk's return value. No dedicated re-export syntax exists.
- 5.1 `module(name, ...)`: creates/uses table `package.loaded[name]` /
  global `name`, sets `_NAME`, `_M`, `_PACKAGE`, sets the module table as the
  function environment; `package.seeall` makes it inherit globals. Only
  paraphrased by the fetch (the fetcher refused verbatim quotes). That
  `module` was removed in 5.2 is **source not verified** (5.4 section 8 does
  not mention it; the removal is from memory only).

Classification: **convention** (return value of the chunk); `module()` is
legacy and global-environment based.

### Construct table

| # | Construct | Sample | Nodes / fields | Class | Expected result | Limit / negative |
|---|-----------|--------|----------------|-------|-----------------|------------------|
| U1 | `return require("m")` | `pos-return-require.lua`, `pos-return-require-nop.lua`, `pos-return-require-longstr.lua`, `pos-return-require-dotted.lua`, `pos-return-parenthesized.lua` | `return_statement > expression_list > function_call (name: identifier "require", arguments: arguments > string > content)`; parenthesized form wraps a `parenthesized_expression` | convention | whole module forward: exported = module itself (no name), source `m`, form namespace-module, scope chunk | `require` must not be shadowed (`neg-shadowed-require.lua`). Source must be a literal string (`neg-require-dynamic.lua`, `neg-require-concat.lua`). Quote style is not distinguished by the tree (all `string_content`) |
| U2 | `local m = require("m"); return m` | `pos-local-return.lua`, `pos-return-module-after-assign.lua` | local `variable_declaration` + `return_statement > identifier` | convention | same as U1, via local | local must not be reassigned (`neg-rebound-module.lua`). If fields are added (`M.extra = ...`) the module becomes a hybrid: report forward plus note `mutated-after-import` |
| U3 | `return require("m").foo` / `return m.foo` | `pos-return-field.lua`, `pos-return-local-field.lua` | `dot_index_expression` (`table: function_call` or `identifier`, `field:`) in the return | convention | exported (module value itself is the member), original `foo`, source `m`, form member-as-module | single return value only; the module's value is a non-table member |
| U4 | `M.foo = require("m").foo; return M` | `pos-table-field-require.lua`, `pos-require-string-call-chain.lua` | `assignment_statement`: `variable_list > name: dot_index_expression (table: identifier M, field: foo)`, `expression_list > value: dot_index_expression (table: function_call require...)`; final `return_statement > expression_list > identifier M` | convention | exported `foo`, original `foo`, source `m`, local none | needs M facts below |
| U5 | via local module | `pos-table-field-local.lua`, `pos-table-field-alias-index.lua` | value is `dot_index_expression (table: identifier m)` where `m` is a `require` local; bracket form `M["foo"]` is `bracket_index_expression` | convention | exported `foo`, original `foo` (may differ: `M.bar = m.foo`), local `m`, source `m` | computed key `M[key]` (`neg-computed-key.lua`) -> limit `computed-key` |
| U6 | constructor | `pos-table-ctor.lua`, `pos-table-ctor-localvar.lua` | `return_statement > expression_list > table_constructor > field (name: identifier, value: dot_index_expression)` | convention | exported `x`, original `x`, local `m`, source `m` | positional `{ m.x }` (`pos-table-ctor-positional.lua`, parsed as `field` with only `value`): no name, limit `positional-field` |
| U7 | whole module in constructor | `pos-table-ctor-module.lua`, `pos-multi-return.lua` | `field (name: m, value: identifier m)` | convention | exported `m`, form namespace-module, source `m` | -- |
| U8 | pcall/dynamic wrappers | `amb-pcall-require.lua`, `amb-setmetatable-index.lua`, `amb-copy-loop.lua` | `pcall(require, "m")`, `setmetatable({}, {__index = m})`, `for k,v in pairs(m)` | n/a | none | limits `protected-require`, `metatable-forward`, `copy-loop` |
| U9 | extra return values | `amb-extra-return-values.lua` | `expression_list` with 2+ children | n/a | only a single-expression return is accepted | `require` as non-last value is truncated to one value; treat as limit |
| U10 | no return | `neg-no-return.lua`, `neg-global-assign.lua` | assignment without any final `return` | n/a | none | A table assignment alone proves no module interface. `foo = require("m").foo` assigns a global |
| U11 | require not returned | `neg-require-unreturned.lua`, `neg-call-result.lua`, `neg-method-wrap.lua` | local result unused, call result, wrapper function | n/a | none | wrappers (`function M.foo(...) return m.foo(...) end`) are new functions |
| U12 | rebinding | `neg-rebound-table.lua`, `neg-rebound-module.lua` | second `assignment_statement` to the same identifier | n/a | none | limit `rebound` |
| U13 | non-top-level | `neg-return-in-function.lua`, `neg-return-conditional.lua`, `neg-return-ternary.lua`, `neg-field-in-function.lua` | `return_statement` nested in `function_definition`/`if_statement`; `and`/`or` | n/a | none | conditional returns: limit `conditional-return`; field assigned in a function runs only when called: `deferred-assignment` |
| U14 | `module(...)` legacy | `legacy-module.lua`, `legacy-module-name.lua` | plain `function_call (name: identifier "module")`; following global assignments | convention (legacy, 5.1) | not reported | limit `legacy-module`: in 5.1 `foo = m.foo` becomes a module field because of the environment switch, but the grammar has no construct that says so |
| U15 | local function as member | `pos-local-func-field.lua` | `M.f = f` where `f` is a local function | n/a | not an import: no `require` provenance | included for contrast |

### Minimal same-file facts

- `require` is the global: no `local require`, no assignment `require = ...`
  anywhere in the file (U1-U7).
- For `M` (U4-U7): `M` is declared once by `local M = {}` / `local M =
  <constructor>`, assignments `M.field = <import-derived>` are at chunk top
  level (not inside functions or conditionals), `M` is never reassigned, it
  is not passed as an argument or used as a metatable target before the
  return, the **last** statement is `return M`, and the last write to
  `M.field` is the import-derived value (a later `M.field = x` overrides).
- Import-derived value: a `function_call require("literal")` optionally
  followed by one `dot_index_expression`, or `<local>.<field>` where the local
  was assigned exactly once from `require("literal")`.

### Lua summary

Convention. Supported: U1-U7. Everything dynamic/legacy gets a limit.
Surprise: tree-sitter-lua does not distinguish call-with-string-literal
(`require "m"`, `require [[m]]`) from parenthesized calls, which is good for
extraction. The grammar also does not model `local x <const>` attributes in
my samples (not sampled).

---

## Open questions / grammar gaps

Grammar gaps (support gaps, not language gaps):

1. **Perl-1**: `@Pkg::Name::VAR` is mis-parsed into nested `package_variable`s
   with `array_variable "@My"` and no error flag (`samples/perl/gap-qualified-var.pl`).
   Clean `has_error` cannot be trusted for qualified variables. Recommend
   rejecting qualified array variables.
2. **Perl-2**: `%EXPORT_TAGS = (all => [..])` parses the key as
   `call_expression_with_bareword` + `fat_comma`; extraction must read the
   bareword as a string.
3. **Perl-3**: `use` and `no` share `use_no_statement`; `use parent` has a
   dedicated node but `use base` does not. The probe prints only named nodes,
   so `use`/`no` keyword distinction is unverified.
4. **Go-1** (not a grammar gap; a design limit): an unaliased import's
   binding name is the imported package's own `package` clause, not the path.
   One file cannot know it (e.g. `gopkg.in/yaml.v3` is package `yaml`).
   Decide: accept explicit alias only, or also match the last path segment
   and mark it inferred.
5. **Go-2**: `go:build` constraints appear as a plain `comment` node; the
   extractor would have to parse comment text for `cond`.
6. **Lua-1**: no gap found; quoting styles collapse to one node type.

Design questions raised by this audit:

- Python: do plain, unlisted imports count as re-exports (runtime exposure)
  or only `__all__`/redundant alias/star (declared interface)? Recommend
  reporting both with a basis marker.
- Python: is `__all__ += mod.__all__` (typing-spec idiom) a supported
  form or a cross-file limit? This audit says limit.
- Go: should `var`/`const` value-copy forwarding be a re-export or only
  `type_alias`? This audit reports it with a distinct kind.
- Perl: confirm from experiments that Exporter happily lists imported subs
  (inferred from glob aliasing, not stated in docs fetched).
- Lua: whether `module()` legacy gets any support (recommend no).
- Coverage-limit vocabulary above is a proposal; the spec owns final names.

## Per-language summary

| Language | Dedicated syntax | Convention | No applicable construct | Parse errors in samples |
|----------|------------------|------------|-------------------------|-------------------------|
| Python | none | imports, redundant aliases, star import, `__all__` (literal list/tuple, `+=`, `extend`, `append`) | `__future__`, function/class-local imports, dynamic imports | 0/41 |
| Go | alias declaration `type X = pkg.X` (incl. generic alias) | package-level `var`/`const` copies of `pkg.Y` | defined types, wrappers, plain/blank/dot/aliased imports, embedding | 0/33 |
| Perl | none (`use parent` is dedicated syntax for inheritance, not export) | `Exporter` + literal `@EXPORT`/`@EXPORT_OK`/`%EXPORT_TAGS` naming names bound by `use Mod qw(..)` | locally defined subs, custom `import`, glob/dynamic forms | 0/35 (one wrong tree: gap Perl-1) |
| Lua | none | `return require(..)`, returned table of imported values, `M.x = require(..).x; return M` | assignments with no return, `module()` legacy (limit only) | 0/40 |
