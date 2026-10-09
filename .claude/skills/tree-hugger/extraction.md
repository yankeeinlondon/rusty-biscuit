# Symbol Extraction Internals

How kinds, containers, visibility, `static` flags, headers, modifiers,
generics, relations and metrics are read from each declaration node
(`lib/src/file/extract/`). User-facing per-language rules live in
[Symbol Kinds, Visibility and
`static`](../../../tree-hugger/docs/symbol-kinds-and-visibility.md).

Kinds, containers, visibility and both `static` flags are decided once, at
extraction, from each symbol's declaration node (`file/extract/`:
`classify.rs`, `declaration.rs`, `rust_impl.rs`, per-language `visibility/`
and `static_flags/` readers behind one dispatcher each); there is no
post-conversion kind override. Bump `EXTRACTOR_REVISION` with any change to
extracted records, including repairs on an unreleased branch. See [Query
System](./query-system.md#capture-to-symbolkind-mapping).

**Keyword and display header** (`file/extract/header/`): `identity.keyword` is
read from one anonymous grammar token per `(language, declaration kind)` table
entry in `header/keyword.rs`, never from text; add an entry only for a
declaration kind the extractor actually captures
(`symbols_header::less_common_declarations_report_their_keyword` pins the
non-fixture ones). `source.signature_text` is cut from `declaration_text` at
node boundaries only (body, `value`/`accessors`/… fields, first `=`/`:=` in
declarator children, terminator), and is `None` when the cut range would not
contain the captured name. `symbols_header` holds `KEYWORD_MATRIX` and
`SIGNATURE_MATRIX`; the coverage guard requires a keyword row per language and a
signature row (case, `NotApplicable` or `Unsupported`) for every `Shape`.

Access lives on `SymbolInfo::visibility` (`None` = Unknown), not on the
signature; `is_static` (member without an instance) and `is_static_storage`
(storage/linkage `static`) are separate flags. The v1↔v2 adapter round-trips all
of them plus `container_kind`; see [API
Reference](./api-reference.md#symbolinfo). It does not round-trip
`TypeMetadata::type_parameters` (no `TypeData` slot) or a `TypeAlias`'s field
metadata (C `typedef struct {…} Alias`; `TypeAliasData` holds only the aliased
type); `symbols_adapter::every_fixture_symbol_round_trips_through_the_record`
pins both.

## Extraction rules

- **Import captures are the bound name, by shape**
  (`TreeFile::imported_symbols`, `@local.definition.import`): the query captures
  exactly the token an import brings in, so the reader filters nothing but Go's
  path-beside-alias duplicate (`should_skip_import`, a blank `_` included) and
  Scala's qualifier segments (below) and merges two captures of one token
  (sorted by position); every import span is excluded from occurrences and
  `referenced_symbols` (`definition_ranges`), so an over-capture deletes real
  reads and an under-capture leaves an import token as a read. Java:
  `(import_declaration (scoped_identifier name: …))`, the final segment only;
  qualified annotations, module names and directive operands are reads. Swift: a
  selective import (`import struct Foo.Bar`,
  `declaration_names::swift_import_selector`, shared with the relation binding)
  is named by its last segment, `source` the path; a whole-module import by its
  path (`tree_file::swift_selective_imports_report_their_bound_name`, CLI
  `refs::swift_selective_imports_are_listed_by_their_bound_name`). C#: an alias
  directive's `name:`, else the final segment of the path
  (`identifier`/`generic_name`/`qualified_name`/`alias_qualified_name`),
  `source` the path or alias target as written, `original_name` the target's
  final name (`csharp_final_name`); qualifiers and alias targets are reads.
  `symbols_occurrences::JAVA_QUALIFIED_REFERENCE_MATRIX` (`R7-F3.nn`) and
  `CSHARP_USING_MATRIX` (`R7-F3.S1.nn`) through `assert_import_rows`,
  `tree_file::java_imports_report_only_their_final_segment`/`csharp_using_directives_report_every_directive`,
  cache `symbols_cache::qualified_reads_and_imports_hold_through_the_cache`, CLI
  `refs::qualified_java_annotations_and_services_are_suggestions_not_imports`/`csharp_using_directives_are_listed_by_imports`.
  An on-demand import (Java/Scala `a.b.*`, Rust glob, Python `from m import *`,
  Go dot import) is one record named by the opened path's last segment, `alias`
  that name, `original_name` `*` (Scala `given`); `unused-import` skips it
  (never a TS/JS `* as ns`). Scala: the query captures every flat
  `import_declaration` path segment and `should_skip_import` keeps only
  `declaration_names::scala_import_binds` (last segment before `,`/end, or
  opening a wildcard/`given` selector group), plus selectors and rename aliases;
  qualifier written paths come from `declaration_names::scala_import_path_start`
  (shared with `written_path`); segment kinds are one set,
  `SCALA_IMPORT_SEGMENT_KINDS`. Comments: a `.` query anchor, `named_child(n)`
  or `next_named_sibling` sees a comment as a named node, so every import/path
  reader steps over extras (`extract/navigation.rs`: `next_syntax_sibling`,
  `prev_named_syntax_sibling`, …, shared with the parameter-default readers); a
  TS entity alias's original is `tokens_without_extras` of the entity after `=`
  (its last segment drives recursion), other sources keep a spanned comment as
  written (`symbols_occurrences::COMMENTED_IMPORT_MATRIX`, rows
  `R8-F1.nn`/`.Sn`, also through
  `symbols_cache::commented_imports_hold_through_the_cache`,
  `tree_file::commented_imports_report_their_bound_names`, CLI
  `refs::commented_imports_keep_their_bindings_in_every_report`). Python:
  `import a.b` binds `a` (`source` `a.b`), future and wildcard imports are
  records; `is_excluded_context` drops `import_statement` for TS/JS only, so
  Python module-path segments are reads (`dotted_name` is a `SEGMENTED_KINDS`
  path). Rust: `use a;`, list `g::h`, `self` (the list path's last segment, also
  bound in `scope/rust.rs::use_names`), `extern crate y as z` → `z`; `source` is
  the path before the name (`rust_use_prefix`). PHP: alias else last segment,
  groups prefixed. TS/JS: default imports, a specifier's alias else name (never
  both), TS `import q = require()`/`import R = N.S` (the identifier before `=`,
  unanchored; also binding sites in `scope/ecma.rs`);
  `occurrences::is_ecma_export_name` (also applied by
  `TreeFile::referenced_symbol`) drops every token of a re-export `export … from
  "m"`, an `export_specifier`'s `alias` and the `export as namespace X` name,
  while a local export's own name stays a `Scoped` read (`EXPORT_CLAUSE_MATRIX`,
  `R7-F3.X2.nn`, CLI `refs::export_clause_names_are_never_references`). Go:
  interpreted and raw-string paths alike, quotes stripped by `go_import_path`
  (`GO_IMPORT_MATRIX` `R7-F3.X3`,
  `tree_file::go_raw_string_import_paths_match_interpreted_ones`, CLI
  `refs::go_raw_string_imports_are_listed_by_imports`). C/C++
  `struct`/`union`/`class` names are records only with a body or written alone
  (the same rule as `declaration_names::c_type_specifier`); elaborated uses are
  reads. User contract `docs/references/language-support.md#what-an-import-binds`; rows
  `SCALA_IMPORT_MATRIX`/`SCALA_IMPORT_ALTERNATIVES` (unbraced renames and
  operator names/qualifiers, `for_each_import_row` crosses independently stated
  controls with comments)/`PYTHON_IMPORT_MATRIX`/`PHP_USE_MATRIX`
  (`R7-F3.S3`-`.S6`), `IMPORT_BINDING_MATRIX` (Rust/TS/JS binding +
  dependencies), `ELABORATED_TYPE_MATRIX` (`R7-F2.X1`),
  `tree_file::*_imports_report_only_their_bound_names`/`php_use_clauses_report_their_bound_names`/`go_blank_and_dot_imports_report_what_they_bind`,
  CLI `refs::imports_list_only_the_names_each_import_binds`; `hug imports
  --name` with a qualified pattern matches the original, the source or source +
  original (`ImportSelection::matches`).
- **Modifiers have one order** (`facets.rs`): `Modifier` derives `Ord` in
  serialization order (Static, StaticStorage, Async, Const, Readonly, Unsafe,
  Mut, then `Custom` lexically); `normalize_modifiers` (sort + dedup) runs once
  per record at the end of `records_from`'s per-record step, so readers may push
  modifiers in any order. Never reorder the variants.
- **Written modifiers are the declaration's own tokens**
  (`extract/modifiers.rs::declaration_modifiers`, called in `records_from` for
  every `wanted` record; no capability): direct keyword children or the
  declaration's own modifier list (Java/C# through `attributes::modifier_host`,
  the whole field/local declaration, shared with the attribute reader), C++
  `override`/`final` from the `declared_function_named` declarator, never
  body/type/parameter/string text. `Const` = Rust `const fn` only; `Mut` = Rust
  `let mut`/`static mut`/`mut x`/`mut self` bindings plus a method whose
  receiver is written with `mut` (a `&mut self` `self` record is not `Mut`);
  `Readonly` = TS/C#; `Custom` lowercase
  `abstract`/`final`/`override`/`virtual`, C# `sealed`, Swift `mutating`;
  keywords outside the spec table (Scala `sealed`, PHP `readonly`, Swift `open`)
  are not read. `symbols_modifiers::MODIFIERS_MATRIX` (one row per language,
  both query paths, grammar limits) is the table to extend.
- **Scope, qualified name and `async` are computed for every symbol** in
  `definitions.rs::materialize` (light records included).
  `extract/scope.rs::symbol_scope` walks out from `member_slot`: first function
  node / `is_local_scope_kind` → `Local`, first owner (`is_scope_owner_kind`:
  member owners minus bodies, plus `enum_specifier`/Go
  `struct_type`/`interface_type`, or any `is_unnamed_type_owner`) → `Member`,
  root → `Item`; bodies are skipped so `declaration_list` is decided by its
  parent (`mod_item`/namespace → keep walking; `impl_item`/`trait_item`/class →
  Member). Parameters and C++ template type params are `Local`; Go
  `method_declaration` and Lua `M.f`/`M:m`/`M.x` (`lua_table_of_name`) are
  `Member`. `extract/qualified_name.rs::qualified_names` runs once per file
  after materialize: a symbol's prefix is the nearest enclosing *naming* symbol
  (function/type/module/namespace/enum variant, by declaration or anchor node
  id, memoized), else a Rust `impl` self type's path as written
  (`rust_impl_prefix`: `impl right::T` → `right::T::m`; leading
  `super`/`crate`/`self` resolved; `impl_self_type_name` stays the bare
  container name), a named unextracted function node (C# static constructor,
  Java compact record constructor; a TS `internal_module` only if its capture is
  missing), else `ancestor_container`; it stops at `is_unnamed_type_owner`, and
  at the root takes the nearest preceding file-level `package`/`namespace`
  statement (`is_namespace_rooted`; consecutive Scala package clauses nest,
  `outer.inner`). A Scala block package (`package p { … }`, never extracted) is
  a rooting `Named` segment in `Builder::prefix` (`is_scala_block_package`), so
  siblings stay distinct and statement clauses before it prefix it
  (`example.symbols.p.W`); `file_level_namespace` excludes it, so declarations
  after the block keep the statement packages; the Scala `private[p]` reader
  takes block-package segments too (`symbols_namespace_bodies` holds the
  block-namespace sibling/nesting rows for every block-namespace language).
  Separators: `::` Rust/C++/Perl, PHP `\` after a namespace else `::`, Lua `:`
  before a method, `.` elsewhere; Perl package names are absolute.
  `stable_symbol_key` hashes the qualified name (ordinal per `(kind,
  qualified_name)`), so changing name construction changes IDs.
  `extract/modifiers.rs::is_async_declaration` reads the anonymous `async` token
  (Rust inside `function_modifiers`, C# inside `modifier`, JS/TS also through
  the `value` of the name's binding). Matrices: `symbols_scope`,
  `symbols_qualified_names`, `symbols_containment`, `symbols_async` (coverage
  guard requires one row per language).
- **Docs, flags and metrics** (`extract/doc_comments.rs`, `attributes.rs`,
  `flags.rs`, `metrics.rs`): `extract_doc_comment` keeps only doc-form comments
  per language (`is_doc_comment_text`: Rust `///`/`/**`, Go any comment in the
  group touching the declaration, Java/JS/TS/PHP/Scala `/**`, C#/Swift
  `///`/`/**`, C/C++ Doxygen forms minus the member-after `<` forms
  `///<`/`//!<`/`/**<`/`/*!<`), skips ordinary ones, drops in Go/JS/TS a comment
  run trailing the preceding code on its line (`trailing_comment_count`: Go line
  comment, TS leading-comment scan; Rust/Java/C#/PHP/Scala/Swift/plain C `///`
  keep same-line docs for the next item), climbs to a wrapper's comments only
  when the declaration has none of its own (`documents_through`: `export`/JS
  variable statement/`template`, Go `var`/`const`/`type` declarations and groups
  (`go/doc`: a spec's own doc wins), C/C++ single-declaration `extern "C"`,
  Scala single-method `extension`; `collect_doc_comments` returns an
  `Attachment` (`Absent`/`Empty`/`Text`) so presence survives normalization: in
  Go only `Absent` climbs, an attached group that reads empty (blank `//`, `/*
  */`, directives only) means no doc and no inherited `Deprecated:`, while every
  other language's empty own doc still climbs; detached (blank line) and
  excluded trailing comments are `Absent`;
  `symbols_flags::go_specs_are_documented_through_their_declaration`
  (`GO_WRAPPER_MATRIX`),
  `go_spec_attachments_decide_group_doc_fallback_even_when_empty`
  (`GO_SPEC_ATTACHMENT_MATRIX` × `GO_SPEC_ATTACHMENT_SITES`) and
  `declaration_wrappers_pass_their_doc_to_a_single_declaration` are the tables
  to extend; Go deprecation reads the same doc), removes block decoration per
  language in `clean_doc_comment` (Java/JS/TS/PHP/Scala/C/C++: one leading `*`
  per line; Rust `rustdoc_block_lines` = rustdoc's `beautify_doc_string`, C#
  `csharp_block_lines` = Roslyn's common line pattern, Swift `swift_block_lines`
  = `measureASCIIArt` at the comment's column: a `*` they would keep is text, so
  a misaligned star-only block and `/** * */` document; a line comment loses
  only its marker, `/// *` documents;
  `symbols_metrics::block_decoration_is_removed_only_where_the_doc_tool_removes_it`
  (`DECORATION_SHAPES` × `DECORATION_LANGUAGES`) is the table to extend; the
  docs pass's `parse_docs_facet` reads that cleaned text and strips nothing),
  reads Python's body docstring (`python_docstring`: a nonempty `str` constant,
  i.e. ordinary literals and their adjacent concatenation, prefix read from
  `string_start`; bytes and formatted literals are not docstrings; source never
  evaluated), and is `None` for Lua/Perl/Bash/Zsh; a context-less non-`Local`
  name is documented before its declaration, a C/C++ context via
  `declaration_for_context`. `declaration_attributes` reads full name +
  top-level arguments from the tree (`token_text`: comments skipped, so
  `#[cfg(/*c*/ test)]` is `cfg` + `["test"]`; a comma inside a string or nested
  list never splits; Rust outer attrs, C++ `[[…]]` (leading, plus those right
  after the captured name on its own declarator path,
  `declarator_id_attributes`: `int a [[deprecated]], b` marks only `a`; after a
  parameter list or array bound they belong to the type), Java/Scala
  annotations, C# lists, Python decorators through parentheses, Swift
  attributes) plus each one's raw `value` (text between the outer delimiters,
  trimmed, comments kept; `None` without a list, `Some("")` for `()`;
  `From<&SyntacticAttribute> for Attribute` is the record conversion).
  `records_from` parses them **once** per declaration and hands the slice to
  `is_deprecated`/`has_test_attribute` (they take `&[SyntacticAttribute]`, never
  re-read the tree); the same list fills the record's `attributes`
  (`Attribute::from`, every `wanted` record, no capability; TS/JS decorators
  too: `ecma_decorators` reads the `export` statement's, the TS `class_body`
  siblings before a method, then the node's own; `decorator_attribute`
  (Python/TS/JS) keeps a non-path, non-call-of-path expression whole as the
  name, so `@(lambda f: f)` is `lambda f: f`; TS 0.23.2/JS 0.25.0 parse only
  paths, calls and parenthesized ones); never split attribute or doc text with
  string operations. Go docs never go through the block cleanup
  (`clean_doc_comment`): `go_comment_group_text` removes exactly `//` (plus one
  space) or `/*`…`*/` (plus the whitespace after `/*`), keeps every other `*` as
  text (`/* *Deprecated: … */` is no marker, `/* * */` is documentation), keeps
  interior blank lines and removes only the group's common indentation (after
  dropping `//` directives exactly as `go/ast`'s `isDirective`,
  `is_go_directive`: no paragraph break left behind;
  `symbols_flags::go_directives_and_trailing_comments_are_not_documentation`
  (`GO_DIRECTIVE_MATRIX` × `GO_DIRECTIVE_SITES`, `GO_TRAILING_ROWS`) and
  `doc_readers_skip_comments_their_language_does_not_attach`
  (`TRAILING_DOC_CASES`) are the tables to extend), so the `Deprecated:`
  paragraph rule (`has_deprecated_paragraph`) treats a still-indented line as a
  code block (`symbols_flags::go_doc_text_keeps_literal_stars`, `GO_DOC_MATRIX`,
  is the row table to extend); doc-tag lines inside a CommonMark fence are not
  tags (`lines_outside_fences`: a fence closes only on a run of its own
  character at least as long as the opening run with nothing after it;
  `symbols_flags::doc_tag_readers_follow_fence_boundaries` (`FENCE_MATRIX`) is
  the row table to extend).
  `is_deprecated`/`has_test_attribute`/`is_in_rust_cfg_test` (in `records_from`)
  and `is_test_file` (only in `finish`, never in a stored record) implement the
  spec tables exactly (no prefix or substring matches; `cfg(any(test,…))` is not
  test). `parameters` = own list (all curried Scala lists) minus Rust `self`
  (`declared_parameters` returns `None` when no list exists and names a
  pattern-bound parameter by its pattern text, Rust `(a, b)`/`_`, JS/TS
  `{x}`/`[a,b]`, so destructured parameters count and keep their type for
  `--param-type`; a TS `this: T` is skipped; `declared_parameter_count` counts
  unnamed C/C++/Go parameters, `(void)` = 0;
  `symbols_signatures::patterned_parameters_keep_their_pattern_and_type_text` is
  the row table to extend; a default is the grammar's
  `default_value`/`value`/`right` field, or for C# (no field) the expression
  after the `=` following the name, with comments (`Node::is_extra`) skipped at
  both steps, never the child beside a token:
  `symbols_signatures::comments_around_a_default_never_erase_or_supply_it`
  (`DEFAULT_LANGUAGES` × `COMMENT_POSITIONS`, every callable form) and
  `symbols_cache::commented_parameter_defaults_hold_through_the_cache`).
  Matrices: `symbols_flags` (`FLAGS_MATRIX`, run on a temp copy because the
  in-place Rust fixture sits under `tests/`), `symbols_metrics`
  (`METRICS_MATRIX`), `symbols_kind_coverage` (`unsupported` vs `landed` `FEAT`
  cells, `no_construct` kinds).
- **Generic parameters are the declaration's own list**
  (`extract/generics.rs::declared_generics`, called in `records_from` for every
  `wanted` record; no capability): `type_info.generic_params` (`GenericParam`
  per written parameter: name, `Type`/`Lifetime`/`Const`, bounds as written,
  default without `=`, variance as written, zero-based `position` = index in the
  written list) and `type_info.constraints` (requirements whose subject is not a
  bare declared parameter: Rust `where T::Item: Copy`, a method's `where` on its
  impl's `T`, Swift `U == Int`, C++ `requires`, a C# `where` naming no declared
  parameter). Never read an enclosing type's/impl's list: C++ reads only the
  `template_declaration` that is the declaration's direct parent (its parameters
  and its `requires`), Go the name's own `type_spec`. A requirement the
  declaration writes itself is read whether or not a wrapper or list exists: C++
  trailing `requires` (always inside the `function_declarator`: member
  declarations, definitions, constructors, friends), C# `where` on a list-less
  declaration, Rust/Swift `where` on a method of a generic type. Bound lists
  split at grammar entries only (skip comment children); Scala keeps each
  bound's operator (`>: L`, `<: U`, `: Ctx`, the context `:` is a separate
  `bound:`-fielded token) and its invariant parameters are flat `name:`/`bound:`
  siblings in the list. Read every field of a parameter from the node that
  actually holds it, never only from a wrapper: a C++ template-template's name
  and default come from its nested `class TT = Box` declaration, a Swift pack's
  name from inside `type_parameter_pack` (`each T` is `T`), a Swift `where`
  written inside `<…>` from the list's own `type_constraints`; a Rust attribute
  in the list (`<#[cfg(x)] T>`) takes no position. A C++ concept-constrained
  parameter reads as `Const` (grammar limit). Position overflow (`u16`) is
  `TreeHuggerError::TooManyGenericParameters` (CLI kind
  `too_many_generic_parameters`), never truncation; `records_from` returns
  `Result`. The older `TypeMetadata::type_parameters` readers in
  `type_metadata.rs` are separate and unchanged.
  `symbols_generics::GENERICS_MATRIX` (one row per language, limits included) is
  the table to extend.
- **Generic-bound lookup and work accounting**: Generic-bound attachment in
  Rust, C# and Swift uses `parameter_positions` once per list, retaining the
  first occurrence of duplicate names and written bound order.
  `shared_context_steps` counts parameter visits, name-set/index work, subject
  lookups and member-range comparisons. `symbols_relation_work` checks wide
  inline namespaces (injected/rooted/bare/explicit/ambiguous), many inline
  namespaces sharing one intermediate spelling with a distinct final name per
  path (qualified/rooted/bare/deeper ambiguous,
  explicit/ordinary/same-name/struct controls, a namespace beside same-named
  inline functions), fully written deep paths, both Swift extension consumers,
  repeated C# `partial` fragments (compatible, two arities, compatible beside
  incompatible categories/arities, outside paths by arity), one C# name at three
  arities (anchored final and intermediate member, outward past a nearer wrong
  arity, rooted past nearer declarations, reopened-namespace same-arity
  competition; `Expect::Indexed`), and separately written bounds (including both
  Swift clause locations), with exact facts alongside growth; each `WORK_SHAPES`
  row is its own test (`repeated_names_keep_relation_work_linear::<shape>`), as
  are the rows of `symbols_shared_context`'s `SHAPES`/`REPEATED_SHAPES` and
  `symbols_relations::RELATION_SHAPES`, all stamped by
  `symbols_shared_context::shape_tests!`
  (`symbols_shared_context::every_linear_work_shape_has_its_own_test` fails on
  an unstamped label).
- **A callable's body span is its own body, and `metrics.lines` and
  `metrics.cyclomatic` measure it** (`classify.rs::callable_body`,
  `metrics.rs::body_lines`, `extract/complexity/`): `records_from` reads the
  body once as nodes (`CallableBody`: one node, a Python/Lua sibling run, or
  Lua's empty body at `end`), stores `CallableBody::span` as `source.body_span`
  (others: `body_span_for_declaration(anchor)`) and passes the same body to
  `symbol_metrics`, so the span, `lines` and `cyclomatic` never disagree; never
  add a second body reader. Callables read only the declaration's `body` field
  (a C/C++ anchor is the `function_declarator`, the body is on the
  `function_definition`), never the type-body fallbacks; C#
  `arrow_expression_clause` gives its expression (no `=>`); Python/Lua run from
  the first child after `:`/`parameters` (comments the grammar leaves beside the
  `block` included, a header-line comment not) to the last before `end`; an
  empty Lua body (no `block` node) is the empty span at `end` → `Some(0)`; every
  span is trimmed of trailing whitespace (Scala `indented_block`, Perl `block`).
  `lines` = `0` for an empty span, else `end.line - start.line + (end.column >
  1)`; `None` for other kinds. `body_node_for_declaration`
  (declaration/signature text cuts) is separate and unchanged.
  `symbols_metrics::BODY_LINES_MATRIX` (one row per language, LF and CRLF) is
  the table to extend. `cyclomatic` (`complexity::cyclomatic`) is `1 +` the
  decisions one per-language `Rules` table counts (`complexity/<language>.rs`:
  `boundaries` pruned and never error-checked, `opaque` pruned but
  error-checked, `types` (every kind of the grammar's type supertypes not shared
  with values, plus `type_arguments`;
  `complexity::tests::every_grammar_type_kind_is_pruned_or_a_shared_value_kind`
  checks the list against `node-types.json`) and `type_sides` (TS
  `as`/`satisfies`: children after the keyword) pruned and error-checked like
  `opaque`, so type syntax in a body never counts, `decisions` by node kind with
  `Counts::{Always, WithField, Operator, Tokens, UnlessLast, UnlessFieldKind,
  Arm}`; `Arm` decides a switch label group from every label sharing its body,
  so a `default` anywhere in the group makes it add 0; TypeScript reuses
  `javascript::DECISIONS` with its own boundaries), walked with one `TreeCursor`
  (no recursion; `complexity::tests::deep_nesting_is_counted_without_recursion`
  runs 20 000 levels on a 256 KiB stack) and checked `u32` arithmetic; `None`
  for non-callables, no body, a language without a table (only Rust, JavaScript
  and TypeScript have one; `METRICS_MATRIX`'s `cyclomatic` column marks the rest
  `NO_CYCLOMATIC`), an error/missing node in a visited or opaque region, or
  overflow. Only the body is walked, so defaults, attributes and return types
  never count.
  `symbols_metrics::{RUST,JAVASCRIPT,TYPESCRIPT,SWITCH_GROUP}_CYCLOMATIC` (exact
  totals per rule, LF/CRLF, both query paths, via `assert_cyclomatic_rows`),
  `MALFORMED_BODIES`
  (`cyclomatic_is_unavailable_only_for_a_malformed_counted_body`) and
  `an_outer_total_does_not_depend_on_selecting_its_nested_callables` are the
  tables to extend; `WorkCounters::complexity_walks` (one per measured callable,
  only under Metrics) is pinned by
  `symbols_query::complexity_is_walked_only_for_metrics_once_per_measured_callable`
  and, through the cache (widen once, `null` stored, warm `Hit` walks nothing),
  `symbols_cache::complexity_is_stored_once_and_served_on_warm_hits` (both over
  `symbols_query::COMPLEXITY_SOURCES`); CLI exposure (`--json-full` always;
  `--json`/`--jsonl` only when `--undocumented`/`--min-params` request Metrics;
  `--stats` never) is
  `reports::cyclomatic_is_reported_only_where_metrics_already_are`; user
  contract `tree-hugger/docs/cyclomatic-complexity.md` (update its tables with
  any rule change), and the cache's `metrics.cyclomatic` shapes are a
  `cache_robustness` column plus `metrics_counts_serve_only_a_u32` (JS generator
  declarations, function expressions and TS `declare function` are not extracted
  as functions, so they cannot be rows); a rule change bumps
  `EXTRACTOR_REVISION`.
- **Go receiver methods and Lua table members have containers**:
  `find_symbol_container` returns the receiver base type
  (`go_receiver_type_name`, through pointer and generics) as `SameFileType` for
  a Go `method_declaration` only (its receiver/params/locals stay
  container-less); `materialize` gives a Lua `M.f`/`M:m`/`M.x` the table text
  with `ContainerLookup::SameFileTable` (kind = unanimous kind of same-named
  `Item`-scope symbols).
- **Declared relations are written bases resolved once per file**
  (`extract/relations.rs::declared_bases`,
  `tree_file/relations.rs::link_relations`, called at the end of
  `records_from`): `extends`/`implements`/`unclassified_bases` keep each base as
  written (`WrittenBase.text`, trimmed; C++ access/`virtual` dropped) plus a
  `BasePath` (segments, generic args dropped; Rust `crate`/`self`/`super`, PHP
  leading `\`, C++ leading `::` as `PathAnchor::Root`/`SelfModule`/`Super`; a
  Rust leading `::` is `Extern` and never resolves) when it is a name;
  `Declarations` indexes **every** record (selected or not) once by the name it
  is bound under (`Declarations::binding_name`: the in-file qualified name,
  except a PHP class/interface/trait/enum/function, which binds in its
  `php_statement_namespace` even inside a function, method, closure or
  conditional, and a Scala `package object q` member, bound in package `q`; C++
  inline namespaces are never expanded into aliases: `inline_children` (a
  namespace is inline when any definition says so), `InlineNames` (canonical
  inline-subtree ranges and segment member points), and memoized `denoted`
  prefixes (`Denoted`: canonical names, at most two qualifier-group owners,
  `beyond_ambiguous`) match a written name segment by segment without walking
  every sibling namespace, also through the inline namespaces below each match,
  so work grows with depth, not `2^depth`; a segment is looked up only below the
  one owner its prefix reaches (a function sharing the spelling is no owner),
  never below a prefix with two owners, and `InlineNames::members` reads a scope
  without inline descendants by exact name; displayed names and IDs never
  change; `docs/type-relations/name-lookup.md#where-a-declaration-is-bound`), and `resolve`
  decides the **first segment** before the rest, from a `Site` (the
  declaration/impl/extension node and the base's byte): first
  `relations/lexical.rs::Lexical::lookup` climbs the syntax scopes inside
  callables, blocks and anonymous owners (one `analysis::scope::ScopeTree` per
  file built by the relation walk over a per-language `scope_role` set, no
  References pass; Rust/TS/JS binding sites from the shared
  `LanguageRules::binding_sites`, Python/Scala pattern sites and Python
  `global`/`nonlocal` from its own readers (a directive redirects writes too:
  every write of the name in the directive's scope, records and imports
  included, binds at the destination, `global` as an `ALL` unrecorded name at
  the module root via `Collected::global_writes`, `nonlocal` as an unreported
  binding of the nearest enclosing function that binds it,
  `nonlocal_destination`; so the destination's declaration is unproved for every
  reader of it, module-wide under `global`); records placed by
  `scope_around(declaration)` with `kind_categories` name groups and
  `visible_from` = Go local type at its name/var after its decl, C++ at its
  name, Java local class at its name, else 0; lexical owners'
  `declared_generic_names` in `generic_names`; a captured generic parameter
  (`is_generic_parameter`, C++ template params) is `Reach::GenericParam`: an
  unreported binding, never a target); one `Record` binding decides, any other
  binding (import, pattern, type parameter, several) or a glob, a type body with
  bases (`body_barrier`: named or anonymous Java/C++/Scala owners) or a Rust
  `mod` ends unresolved; JS/TS class bodies and Python class bodies past the
  first scope are skipped. Only when no lexical scope binds it do
  `site.type_params` (`type_parameters_around`: named owners only, plus a Swift
  extension's resolved owner via `extension_owners`), `owner_binds` (parameters
  of an owner without a record, a Rust impl or a TS bodiless signature:
  `FileWalk::owned`, by owning lexical scope index and name, read along
  `Lexical::scopes_from(site)`) and `binding_scope` over the named scopes (PHP:
  the syntactic namespace, `php_statement_namespace`, never a function's display
  prefix; a Scala declaration in a package object: its bound name's owner path)
  apply; there every lookup filters by name group
  (`final_categories`/`qualifier_categories`: a type for a base clause, a value
  for a JS/TS class `extends`, a term for a Scala qualifier,
  `RELATION_NAMESPACE` for a TS type qualifier, every group in Go/Python) and
  sees only `Reach::Qualified` records, so in a language with separate groups a
  value never hides a type and a block's declaration is never in view. A binding
  no record reports outside every lexical scope (`lexical::Unrecorded`: name,
  owning syntax scope, the site's categories; Python targets `ALL`, a TS/Scala
  pattern value `VALUE`, a Rust `const` generic `VALUE`) goes to
  `FileWalk::named` (by bound name) and `FileWalk::by_owner` (by enclosing
  record) `ScopeFacts::unrecorded` with those groups (`names_scope`: root, a
  record's node or its body), else to `FileWalk::owned`; never fold it into
  `imports` or widen it to every group. The first named scope that binds the
  segment (a same-group declaration; a qualifier of one, `implied`; an import,
  which never resolves) decides, with no fallback outward; a wildcard import/C++
  using-directive scope (after local declarations in C#/Swift/Scala/Java/Go), a
  Java/C++/C#/Swift/Scala type with bases and a Rust module end the search;
  every Python class scope but the innermost (`scopes[0]`;
  `class_scope_is_lexical`) is skipped. Each further segment is looked up inside
  the owner the previous one selected (`Owner`): below a lexical owner (a first
  segment `Lexical` found, or a local Rust module named by `self::`/`super::`)
  only among that declaration's own members by syntax (`Declarations::members`
  over `direct_members`, one index by owner and bound-name rest built once from
  `direct_owner`: nearest scope record, no `opens_lexical_region` node between),
  never by displayed name, so same-named sibling-block owners never lend or hide
  a member and a Rust local module's items are reachable through its path; below
  a named scope by qualified name, inside one owner of the qualifier group
  (`is_traversable`: one declaration, or one `OwnerGroups` group
  (`not_merging`): the definitions of one reopened `Namespace`, or the C#
  `partial` fragments of one type identity, `MergeKey::Partial` = category
  (class/struct/interface/record = record class/record struct,
  `extract::csharp_partial_shape`) + generic arity + enclosing owner group,
  built once per file with one ancestor walk per fragment — never two Rust
  `mod`s, Scala `object`s, non-partial types, `partial` types of different
  category/arity/enclosing owner, or a namespace beside a type — and, in C++,
  one canonical owner: `NameSummary::canonical` counts the distinct
  inline-namespace members a spelling reaches, two stop the path at that
  segment); a named prefix competing owners share (`is_ambiguous_owner`: two
  declarations not all one owner group, `NameSummary::not_merging`; `partial`
  alone proves nothing) is read, from a site inside one of them
  (`binding_scope`/`may_bind` via `anchor_around` → `scope_at`,
  `self::`/`super::` via `anchors_members`), or in C# through a name whose
  written generic argument count denotes one group (`BasePath::arities`,
  `written_group` over `OwnerGroups::by_arity`: `H.Base` is the non-generic
  `H`'s, `H<int>.Base` `H<T>`'s; no group or several of that arity stops the
  path; everywhere else a C# written count is part of the name before any
  uniqueness or stop decision: `declared_arity` (type parameter count, namespace
  0, other kinds and languages `None` = any count) indexes
  `NameSummary::written` and `direct_members` as `ByArity` (per-arity `Few`,
  filled before truncation), read by `named_written` (final named target,
  bare/qualified/nested/rooted), `members(owner, segment, groups, arity)`
  (anchored and syntax owners, final and intermediate) and `is_bound(name,
  groups, arity)` in `binding_scope`/`may_bind`, so a nearer type or namespace
  declaring the name only at other arities never stops the outward search (a
  pure qualifier binds only count 0 and only when no type or namespace declares
  the name); `arity_fits` remains for a lexical binding), as that owner group
  alone (`Owner::Anchored(group)`): its fragments' members (`members` by syntax,
  keyed and valued by group; a member in several fragments has no one ID,
  `sole`), its imports/wildcards/unrecorded bindings (`FileWalk::by_owner`,
  merged per group once in `group_facts`, `owner_rebinds(FactsOf::Owner)`,
  `is_written` by group) and its kind and bases (`owner_flags`,
  `OwnerGroups::with_bases`), never the bound name's combined `FileWalk::named`
  facts or `scope_flags`, so a sibling owner's import, glob, bases or member
  types change nothing (`SOURCE_OWNER_MATRIX`); an unresolved Swift extension
  owner makes the context unknown (`TypeParams::unknown`: every lexically
  unbound first segment is unproved, raw-type spellings kept), never empty;
  `named`, `is_traversable`, `is_bound` and the `binding_scope`/`may_bind` owner
  flags read one `NameSummary` per bound name (at most two declarations per
  group in a `Few`, plus non-owner/non-namespace group bits and
  type-like/with-bases/module flags), so a reopened namespace or repeated name
  never costs a bucket scan; written-target dedup is a membership set
  (`WrittenTargets` on records, `dedupe` at the end of `declared_bases`) and
  every candidate visit, inline step and insertion counts in
  `shared_context_steps` (`symbols_relation_work` holds the repeated-name,
  reopened-namespace, many-bases, impl-aggregation, generic-owner and
  inline-depth shapes); at every segment, under either owner, the owner's other
  bindings of the name leave the member unproved (`named_member_rebound`:
  `ScopeFacts::unrecorded` at the owner, its imports where
  `imports_bind_members` (all but Scala/Swift: elsewhere an import beside a
  same-named member is a compile error), a Python star import
  (`globs_bind_members`); `syntax_member_rebound`: `Lexical::binds_only` over
  the member's own body scope), and so does a Python attribute write
  (`Collected::qualified_writes`: assignment, augmented, `del`, `for`/`with`
  targets) whose plain dotted object resolves to that owner (`written_members`,
  resolved once per file with writes off); a record a Python directive
  redirects, and every declaration inside it, is `Reach::Redirected` (never seen
  by `named`, `members` or `implied`); the full name resolves only for exactly
  one eligible same-group declaration (so aliases and duplicates shadow).
  Imports are read only as bindings with their name groups
  (`extract/relations.rs::import_binding` → `ImportBinding::Names(names,
  groups)`: Rust/C++/PHP/Python/JS/TS/C#/Swift/Scala/Java/Go; Swift `import
  func`/`let`/`var` bind a value, `class`/`struct`/`enum`/`protocol`/`typealias`
  and a Java single-type import a type; an unaliased Go import binds its assumed
  package name; an import whose names cannot all be read is `Opaque`, never a
  shorter list; Scala reads every `,`-separated expression, selector groups and
  both rename nodes, and only `namespace_wildcard`/`given` selectors are
  opaque); imports in lexical scopes are `ScopeTree` bindings/globs, the rest go
  to `FileWalk` by named scope. `FileWalk` is the one lazy tree walk per file
  (scopes, sites, imports, Rust impls, Swift extensions); PHP `namespace N;`
  statements are root children, read in `Declarations::new`.
  `symbols_relations::PATH_MEANING_MATRIX`'s block, name-group, pattern,
  anonymous-owner and import rows are the class tables to extend;
  `PYTHON_MEMBER_SHAPES` (member rebinding, qualified writes, class-body
  directives) is written once and generated at module level and inside a
  function, and every consumer reads both through `path_meaning_rows()`, which
  also chains `OWNER_AMBIGUITY_MATRIX` (C++ inline/direct/reopened namespaces,
  Rust duplicate and anchored modules and every impl consumer, Scala objects,
  TS/C#/PHP reopenings, duplicate-owner controls) and the Swift unproved-owner
  rows (ambiguous/alias/import/external owners, all 30 raw-type spellings,
  nearer lexical controls), then `SOURCE_OWNER_MATRIX` (sibling-owner
  imports/globs/patterns/bases in Rust, Python, Scala, Java, Swift, C#, C++, the
  four Rust impl consumers, same-owner negative and namespace/`partial`/lexical
  controls) and the Swift sibling-shadow rows (all 30 raw-type spellings
  declared only in the other owner: no base), then
  `CSHARP_PARTIAL_IDENTITY_ROWS` (70 rows pinned by
  `csharp_partial_identity_rows_cover_every_swept_shape`: arities 0/1, 0/2, 1/2
  per form, matching-fragment controls, all 30 directed category pairs with
  `record`/`record class` the compatible pair, own member, inherited barrier,
  outside paths by arity, enclosing `Outer`/`Outer<T>`, namespace beside a type,
  generic/non-partial controls, and a containment row whose `Probe` impl member
  also runs through C#'s record-only Containment widening;
  `csharp_partial_identity_keeps_source_contained_memberships`), then
  `CSHARP_WRITTEN_ARITY_ROWS` (117 rows pinned by
  `csharp_written_arity_rows_cover_every_swept_site`: 9 `ArityLookup` sites
  (bare, namespace-qualified, nested, anchored final and intermediate member,
  outward from a unique type, an anchored type and a namespace, rooted
  `global::`) × six forms × `partial` or not, each declaring every arity on its
  own line with children writing counts 0/1/2 and extends/implements/uncertain
  by target form, plus same-arity competition (duplicates, repeated `partial`
  fragments, anchored, nested), wrong-arity, no-outer-declaration, using-alias
  and type-parameter barrier rows and a named-intermediate control). Rust impls
  are resolved once each (`impl_header`, with every generic in view): a positive
  trait impl's trait as written goes to members (`impl_of_member`) and the
  resolved self type; negative/inherent add nothing; composite self types (`&W`,
  `dyn T`) never attach. Under Containment (`RecordFacts::containment`) every
  record whose `ContainerLookup::SameFileType { impl_item }` names an impl gets
  `relations.container` (+ `parent` when resolved; else the self type as
  written, no ID), and `analysis::link_containers` keeps pre-linked records; so
  a Rust or Swift entry widened by Containment parses (`record_passes_suffice`
  takes the language; Swift extension members are linked the same way,
  `RelationInput::container_extension` → `Change::Container` from
  `extension_owners`). C#/Swift/Scala classify syntax first, then same-file
  targets (`declares_class`: a `Class` record, or where the kind is broader than
  the category, the target's own declaration: Swift `class`, C# `record`/`record
  class` but never `record struct`, `extract::is_csharp_struct`, shared with the
  owner-side struct check), retaining unknown categories in
  `relations.unclassified_bases`; a built-in spelling (Go interface element
  named like a predeclared non-interface type; Swift enum first entry named like
  a standard raw type, bare, `Swift.`-qualified or `Unicode.Scalar`) is marked
  `WrittenBase::builtin_spelling` and dropped only when `Declarations::may_bind`
  (`Lexical::may_bind` then every named scope, no early stop, in the base's name
  group) finds no declaration in view, qualifier, same-group import of that
  name, unreadable import or type parameter for its first segment (a Swift value
  import or a finished block's declaration is no evidence), else kept and
  resolved normally (Swift whole-module imports are `ImportBinding::Module`,
  never evidence; `PATH_MEANING_MATRIX` has rows for all 19 Go and 15 Swift
  spellings); Swift local-owner extensions aggregate conformance and targets
  retain source spans for rendering. `owner_generic_context` shares unnumbered
  parameter layers by actual owner identity, including all declaration ancestors
  of nested extended types; both extension bodies and header conformances use
  it, preserving nearer lexical precedence without importing ordinary enclosing
  members or copying enclosing parameters into serialized child metadata.
  `SWIFT_NESTED_GENERIC_ROWS` adds those cases and all 30 shadowed raw-type
  spellings to `path_meaning_rows()`; cache and outline regressions cover
  widening and genuine hits. `symbols_relations::RELATIONS_MATRIX` (one row per
  language), `PATH_MEANING_MATRIX` (labeled rows, several per language: roots,
  first-segment bindings, owners passed through, generics in view; impl rows
  check method-only queries, and
  `symbols_cache::path_meaning_impl_rows_widen_to_containment_with_the_same_facts`
  the widened and warm cache;
  `symbols_cache::path_meaning_rows_hold_on_warm_hits` checks every row's
  targets on complete and type-only warm `Hit`s) and its Rust impl tests are the
  tables to extend (`GENERIC_BOUNDARY_ROWS`, one row per generic reader, checks
  65,536/65,537 parameters per reader and side in
  `generic_boundary::<reader>::*`, and warm `Hit`s in
  `symbols_cache::oversized_owner_warm_hits`; the giant sources have a nextest
  timeout override); user contract `tree-hugger/docs/type-relations.md`.
  `link_relations` runs once per analysis (`WorkCounters::relation_links`).
  `symbols_relations::declaration_fact_sources` (every generics, modifiers,
  body-lines, relations and path-meaning matrix source plus the Rust impls)
  feeds the query/index equivalence (`symbols_query::declaration_facts_*`),
  cache projection and widening (`symbols_cache::declaration_fact_*`) and
  adapter-loss
  (`symbols_adapter::record_only_declaration_facts_are_lost_through_the_older_model`)
  tests; a new declaration-fact matrix adds its sources there. Each consumer
  (and the two path-meaning consumers
  `path_meaning_matrix_holds_on_complete_and_declarations_only_paths` and
  `symbols_cache::path_meaning_rows_hold_on_warm_hits`, through
  `path_meaning_group`) is one test per corpus group stamped by
  `declaration_fact_group_tests!` (`per_language_matrices`: the
  one-row-per-language matrices plus the Rust impls, which carry the "some
  source exercised X" guards; `shared_rows_N`: owner-identity and path-meaning
  rows dealt round robin); `declaration_fact_groups_partition_the_corpus` fails
  when a row is in no group or a group exceeds `MAX_FACT_GROUP_SOURCES`: add a
  group (`SHARED_FACT_GROUPS` plus a stamped name), never a timeout override.
- **Readers get a `DeclarationContext`** (`declaration.rs`): language, source,
  final kind, `scope` (`is_local()` = `Local`; replaced `is_local_declaration`),
  the captured `name` node (one declaration can name several things: `int a,
  *b;`, Swift `case east, west`), the file `root` (same-file lookups: Rust trait
  level, earlier C/C++ `static` prototype, Scala `package` segments)
  `is_ecma_module` (computed once per JS/TS file by `is_ecma_module_file`) and
  `memo` (`VisibilityMemo`: C/C++ `static` prototypes and namespaces, C++ access
  sections per class body, Scala package segments, Rust trait levels, each
  derived once per file on first use). Add a field here rather than re-scanning
  the file per symbol; any context several declarations share is built once and
  looked up (`find_symbol_container` takes a per-file `SiblingScopes`,
  `is_in_rust_cfg_test` a per-file inner-attribute map, `qualified_names` a
  one-pass file-namespace index), and `symbols_shared_context` asserts
  `shared_context_steps` grows linearly. Key shared context by the decision,
  never by bare name alone: names repeat across modules, namespaces and
  overloads (Rust trait levels aggregated per name, C++ `static` lookups by
  complete `FunctionIdentity`, namespaces by path, Scala segments a set,
  container kinds by qualified name/segment tail in
  `definitions.rs::KindAgreement`, container records by
  `analysis::smallest_enclosing` (one span-ordered sweep per name) then
  `OwnerIndex`, export targets by name span); its `REPEATED_SHAPES` are the rows
  to extend.
- **The C/C++ earlier-`static` scan matches only the same function**
  (`visibility/c.rs`). `declared_functions` (`c_declarators.rs`) takes the
  `function_declarator` nearest the name with only parentheses between, so a
  returned function-pointer, function-reference or array-pointer type is never
  the function; an unqualified C++ declaration has exactly its enclosing
  namespace; a qualified one resolves its first qualifier against the file's
  `namespace` blocks, nearest enclosing first; a function template (under
  `template<…>` or abbreviated with an `auto` parameter) also compares its
  header, leading and trailing return type, and the `requires` clause after the
  header and after the parameters (`TemplateSignature`; never `noexcept`).
  Whatever cannot be established is a miss, never a match.
  `symbols_visibility::earlier_static_linkage_matrix_holds_for_c_and_cpp` is the
  row table to extend.
- **C/C++ signatures read the named function's own declarator**
  (`signatures.rs`). `extract_signature` gets the captured name;
  `declared_function_named` (`c_declarators.rs`, shared with linkage) picks that
  function's `parameter_list`, so a pointer/reference/array return keeps its
  parameters and a returned callback's never replace them. Each parameter
  (`parameter_declaration`, `optional_parameter_declaration` with its default,
  `variadic_parameter_declaration` pack, C `variadic_parameter` and the C++
  `...` token) is named by `declarator_name`, which follows a function
  declarator only through its `declarator` field.
  `symbols_signatures::c_and_cpp_signatures_read_the_named_functions_own_parameters`
  is the row table to extend.
- **C++ const-object linkage reads the declarator nearest the name**
  (`is_const_object`, `visibility/c.rs`). It reuses `declarator_layers`
  (parentheses and attributes add no layer; arrays are skipped to their
  element), so `char **const p` is const and `char *const *p` is not; a
  reference is checked before the `constexpr` shortcut and is never const. Any
  new reader of a declarator's type goes through `declarator_layers`, not the
  outermost node.
  `symbols_visibility::const_object_linkage_matrix_reads_the_outermost_declarator`
  is the table to extend.
- **Membership starts from `member_slot`** (`classify.rs`): the declaration or
  its outermost transparent wrapper (`is_transparent_wrapper_kind`: C++
  `template_declaration`, C/C++/C# `preproc_*` conditionals, Python
  `decorated_definition`, Scala `extension_definition`). Kind, both `static`
  readers, C++ and C# access (owner read from `member_slot(..).parent()`) and
  C/C++/C# field, interface-property and enum-member lists (`flattened_members`
  in `type_metadata.rs`) all look through those wrappers; add a new wrapper
  there, not in one reader, and route any new reader of a declaration's parent
  or a type body's children through `member_slot`/`flattened_members`. Swift
  `#if` and C# `#region` are flat `directive`/`preproc_region` siblings, not
  wrappers.
- **Member kinds are validated, not trusted.** `refine_symbol_kind` demotes any
  Method/Constructor (to Function) or Field/Property (to Variable), whether from
  a query capture or a classifier rule, when `is_function_local_member` finds a
  function before any type or object owner above the member slot (C# local
  functions are captured as methods by the vendored query). Lua `M:m`/`M.f`
  names its owner and is exempt. `symbol_scope` (which decides
  `DeclarationContext::is_local()`) also stops at the nearest owner, so members
  of a type declared inside a function keep their access; a JS/TS `constructor`
  is a Constructor only directly in a `class_body`. Other constructors:
  Java/C#/Swift by query capture (C# `static` excluded in the query); Python
  `__init__` and PHP `__construct` (`eq_ignore_ascii_case`) by name only after
  the declaration is a method; C++ by syntax alone
  (`declaration`/`function_definition` in a class body with no `type` field
  whose function declarator, reached through attributed/parenthesized wrappers
  only (`transparent_declarator`), is the `declared_function_named` one and
  declares the captured `identifier` itself: `C [[deprecated]] ()` and `(C)()`
  qualify, `~C`, `operator T`, `C::C` and pointer/reference wrappers do not;
  never a name comparison). Scala body `val`/`var` is reclassified to Field by a
  classifier arm, one per name of a grouped `val a, b: Int` (the query captures
  each `identifier` under `identifiers`; the field reader lists every name with
  the shared type). Recorded limits (Scala access-modified `enum`,
  tree-sitter-scala 0.24.0) live in `symbols_kind_coverage::GRAMMAR_LIMITS`; a
  demonstrated limit goes there, never in `unsupported` (which requires a clean
  parse).
- **Unnamed owners end the container search.** `find_symbol_container` returns
  no container at `is_unnamed_type_owner` (anonymous classes, Scala `given …
  with` instances, JS/TS class expressions and object literals, tagless C/C++
  `struct`/`class`/`union` that is not an anonymous member) instead of climbing
  to the named type outside.
- **Implicitly static members are static.** A member the language makes static
  without the keyword sets `is_static` at extraction: Java interface constants,
  C# `const` fields, Rust associated `const`s (impl and trait), PHP member
  `const`s (`const_declaration` is captured as Constant; a top-level `const` is
  not a member). C++ `static constexpr` says `static`; Swift/TS/JS/Scala/Python
  have no keyword-less case beyond Scala `object` members and Python class
  attributes.
- **Static readers match nodes, not text, and stop at the nearest owner.** A
  member-`static` reader matches the declaration's own keyword/modifier token or
  decorator expression (Python: `staticmethod`/`classmethod`, bare or
  `builtins.`-qualified, and a `ClassVar` annotation head under any qualifier;
  never a substring or prefix) and asks only the type that directly owns the
  member (Scala: the member slot's `template_body` belongs to an
  `object_definition`). A new reader must not `.contains()` node text or climb
  past a class, trait, enum or anonymous class.
- **Field lists are completed from the standalone members.**
  `align_field_metadata` (`tree_file/definitions.rs`) pairs each type-metadata
  entry with the standalone `Field`/`Property`/`Constant` of the same name whose
  container node is that type (k-th entry with k-th symbol of a name), copies
  its `visibility`/`is_static`, **adds** any standalone member no reader listed
  (doc comment, no type) and orders the list by source position, so `hug
  classes` and `hug symbols` list the same members. Readers still matter: they
  supply type annotations and the entries with no standalone symbol (record
  components, Rust tuple positions, Scala class parameters). Enum owners get no
  field entries; their variant lists are completed from standalone
  `EnumVariant`s by `align_variant_metadata`/`reconcile_variants` (Enum owners
  only: a Swift enum is `Type` and has no list; `discriminant` from the member's
  `value` field; a standalone-only entry is unit, so a payload-bearing form
  needs a reader entry: Rust tuple/struct variants, Scala `full_enum_case` class
  parameters as `struct_fields`, every clause, defaults never a discriminant),
  and `MetadataSelection::for_kinds` keeps variant metadata when Enum is
  selected, and Rust `impl` members (by-name container) never join the struct's
  list. `symbols_field_lists` pins complete per-language lists on both paths; a
  new member form needs a query capture **and** a reader case.
- **Type metadata lists only fields and properties.** Interface, protocol and
  trait methods are standalone `Method`s, never `FieldInfo` entries (Java
  interface constants, C# interface properties, Swift protocol property
  requirements and Rust trait associated constants are listed). Go interface
  method elements are neither records nor entries:
  `extract_go_interface_methods` matches `method_spec`, which tree-sitter-go
  0.25 no longer produces (`method_elem`); `symbols_field_lists` pins the empty
  list.
- **C/C++ names sit under any declarator chain.** The C/C++ `locals.scm` capture
  a name by its immediate parent (pointer, array, parenthesized, attributed
  declarator; C++ adds reference and, for member functions,
  `(function_declarator declarator: (field_identifier))` under any return
  declarators), never by a fixed outer shape. `classify_symbol_kind` makes a
  captured variable/field a Function (Method in a type body) when
  `declared_function_named` says its declarator declares one (`int f
  [[deprecated]] (int)`, `int (f)(int)`), so a function-pointer field stays a
  Field; `implied_c_function_context` (`declaration.rs`) supplies the context
  such a capture lacks. `c_field_name` (`type_metadata.rs`) is `declarator_name`
  minus function declarators. Typedef names behind a declarator are TypeAliases
  with the typedef's doc comment and no field list.
  `symbols_declarator_wrappers` is the row table to extend. Anonymous
  `struct`/`union` members (`anonymous_member_declaration`, `classify.rs`)
  belong to the enclosing type, with the C++ access of the section holding them.

Java import captures are scoped to `import_declaration` and accept both a
qualified path's final identifier and a direct identifier (`import p.*`, `import
static p.*`, comments around the dot). On-demand records retain source `p`,
alias/name `p`, original `*`; exclude only that bound token, preserving
qualifiers and body reads.
`symbols_cache_imports::java_single_segment_on_demand_imports_keep_public_facts_and_six_cache_stages`
pins public diagnostics, conservative evidence, summaries, filtering and cache
projections; CLI
`refs::java_single_segment_on_demand_imports_are_reported_without_token_suggestions`
pins reports.

## Known limitations

- **Swift**: structs, classes and enums all captured as `SymbolKind::Type` (an
  extractor limit; the grammar's `declaration_kind` field could distinguish
  them)
- **Constructors**: explicit instance constructors only; Java compact record, C#
  static/primary constructors and destructors are absent, C++ out-of-class
  `C::C() {}` is a container-less `Function`
- **Go**: Struct and interface both captured as `SymbolKind::Type`
- **Go**: embedded fields and fields of anonymous struct types are not reported;
  grouped `X, Y int` fields share one span, so `hug` orders them by record ID
- **Scala**: `private enum E { … }` does not parse (grammar limit G1 in
  `docs/symbol-kinds-and-visibility/limitations.md#grammar-limits`)
- **Python / JavaScript**: instance attributes assigned only through `self.x` /
  `this.x` are not reported (the catch-all assignment patterns were removed);
  class-body declarations are
- **C# / Swift / Scala**: mixed lists retain unknown categories in
  `relations.unclassified_bases`; Swift extensions of external, aliased or
  ambiguous owners cannot attach conformance to a local record
  (`docs/type-relations.md#c-swift-and-scala`)
- **Declaration-fact grammar limits** (pinned by `*_grammar_limits_*` tests,
  table in `docs/symbol-kinds-and-visibility/limitations.md#grammar-limits`): Python PEP 696
  type-parameter defaults and TS `in`/`out` variance do not parse; a C++
  concept-constrained parameter (`template <std::integral T>`) reads as `Const`;
  a TS/JS decorator is only a path or a call of one
- **Perl**: narrow coverage: named subs, `package` (Module) and
  `my`/`our`/`state` names only; variable names keep their sigil (`$name`);
  `local` is never a declaration; scope lives in the declaration text
  (`my`/`our`/`state`), not in the level
