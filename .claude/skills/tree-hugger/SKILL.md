---
name: tree-hugger
description: Expert knowledge for multi-language symbol extraction using Tree-sitter. Use when working with tree-hugger-lib or tree-hugger-cli (hug), extracting symbols/imports/exports, implementing lint diagnostics, adding new language support, writing tree-sitter queries, or troubleshooting the underlying tree-sitter runtime (grammar versions, ABI mismatches, query debugging).
---

## Purpose

Tree Hugger provides Tree-sitter-based symbol extraction and diagnostics across 16 programming languages. Use this skill when:

- Working in `tree-hugger/lib/` or `tree-hugger/cli/`
- Extracting functions, types, imports, or exports from source files
- Implementing or debugging lint diagnostics
- Adding support for new languages or fixing query issues
- Understanding the query vendoring system

## Quick Reference

### Supported Languages

Rust, TypeScript, JavaScript, Go, Python, Java, C#, C, C++, Swift, Scala, PHP, Perl, Bash, Zsh, Lua

### CLI Commands (`hug`)

| Command | Description |
|---------|-------------|
| `functions` | List functions and methods |
| `types` | List type definitions (struct, enum, class, interface, trait) |
| `symbols` | List all symbols |
| `imports` | List imported symbols |
| `classes` | List classes, structs, interfaces and traits with static/instance member partitioning (on `SymbolInfo::is_static`) |
| `lint` | Run lint and syntax diagnostics |
| `lint --deny <rule>` | Promote rule to error |
| `lint --warn <rule>` | Promote rule to warning |
| `lint --allow <rule>` | Demote rule to info |
| `lint --strict` | Promote all warnings to errors |
| `lint --experimental-semantics` | Enable experimental semantic rules |
| `god-files [DIR]` | Identify oversized source files ripe for refactoring |
| `god-files --high-risk` | Show only high-risk files |
| `completions` | Generate shell completions (Bash, Zsh, Fish, PowerShell) |
| `<declaration cmd> --sort file\|name\|kind` | Order (default `file`: path, then position); only `functions`/`types`/`symbols`/`classes` accept it |
| `--json-full` | Complete per-file indexes (declaration commands only; symbol selectors rejected) |
| `--verbose` | Cache stats line (hits / partial / misses, parses, captures, diagnostics runs) and cache failures on stderr |
| `<declaration cmd> --jsonl` | One self-describing JSON envelope per selected record (`classes` lines add a `members` list) |
| `<declaration cmd> --count` / `--stats` | One integer line / counts by kind, visibility, language + doc coverage (`--stats --json` for the envelope) |
| `<declaration cmd> --format plain\|table` | Unstyled, unwrapped listing / one biscuit-terminal `Table` per file |
| `functions\|types\|symbols --tree` | Members nested under their container (by container ID) |
| `<declaration cmd> --signatures full\|short\|none` | Header as written / compact with `… +N` past four fields (default) / names only |
| `<declaration cmd>` selectors | `--file`/`--name`/`--exact`, `--kind`, `--scope`/`--all-scopes`, `--visibility`, `--exported`, `--in`, `--exclude-tests`/`--tests-only`, `--undocumented`, `--deprecated`, `--async`, `--returns`, `--param-type`, `--min-params` (AND across selectors, OR within one; smart-case names) |
| positional directory (`src/`) or `--file <dir>` | selects no files (`NoSourceFiles`): inputs are `ignore` override globs and a directory pattern does not whitelist its files; use `'src/**'` |

### Library API (`TreeFile`)

```rust
let file = TreeFile::new("src/lib.rs")?;
file.source();              // Source text of the parsed file
file.comment_ranges()?;     // Byte ranges of comment nodes
file.symbols()?;            // All symbols
file.symbol_records()?;     // v2 parse records
file.symbol_index_v2()?;    // complete index: every pass, diagnostics included
file.query(&SymbolQuery::new().kinds([..]).capabilities([..]))?; // selective
file.query(&SymbolQuery::new().scopes([..]).name(NamePattern::new("parse")).scan_root("."))?; // filtered, prefixed
// Capability::Flags -> is_test/is_deprecated; Capability::Metrics -> metrics facet
SourceFile::read(path)?.query(&query)?; // read+hash once, parse on demand
file.imported_symbols()?;   // Imports
file.exported_symbols()?;   // Exports
file.local_symbols()?;      // Local definitions
file.referenced_symbols()?; // Identifier usages
file.lint_diagnostics();            // Pattern lint rules (with metadata)
file.syntax_diagnostics();          // Parse errors
file.dead_code();                   // Unreachable code blocks
file.experimental_semantics = true; // Enable experimental semantic rules
```

### SymbolKind Enum

Function, Method, Constructor, Type, TypeAlias, Class, Interface, Enum, EnumVariant, Trait, Module, Namespace, Variable, Parameter, Field, Property, Macro, Constant, Unknown

Kinds, containers, visibility and both `static` flags are decided once, at
extraction, from each symbol's declaration node (`file/extract/`:
`classify.rs`, `declaration.rs`, `rust_impl.rs`, per-language `visibility/`
and `static_flags/` readers behind one dispatcher each); there is no
post-conversion kind override. Bump `EXTRACTOR_REVISION` with any change to
extracted records, including repairs on an unreleased branch. See [Query System](./query-system.md#capture-to-symbolkind-mapping).

**Keyword and display header** (`file/extract/header/`): `identity.keyword` is read from one anonymous grammar token per `(language, declaration kind)` table entry in `header/keyword.rs`, never from text; add an entry only for a declaration kind the extractor actually captures (`symbols_header::less_common_declarations_report_their_keyword` pins the non-fixture ones). `source.signature_text` is cut from `declaration_text` at node boundaries only (body, `value`/`accessors`/… fields, first `=`/`:=` in declarator children, terminator), and is `None` when the cut range would not contain the captured name. `symbols_header` holds `KEYWORD_MATRIX` and `SIGNATURE_MATRIX`; the coverage guard requires a keyword row per language and a signature row (case, `NotApplicable` or `Unsupported`) for every `Shape`.

Access lives on `SymbolInfo::visibility` (`None` = Unknown), not on the signature; `is_static` (member without an instance) and `is_static_storage` (storage/linkage `static`) are separate flags. The v1↔v2 adapter round-trips all of them plus `container_kind`; see [API Reference](./api-reference.md#symbolinfo). It does not round-trip `TypeMetadata::type_parameters` (no `TypeData` slot) or a `TypeAlias`'s field metadata (C `typedef struct {…} Alias`; `TypeAliasData` holds only the aliased type); `symbols_adapter::every_fixture_symbol_round_trips_through_the_record` pins both.

- **Selective analysis** (`analysis/mod.rs`, `file/query.rs`, `file/source_file.rs`, `file/tree_file/definitions.rs`): one planner serves `symbol_index_with_passes`, `symbol_index_v2` (= `SymbolQuery::complete_index()`) and `query`; passes run once after prerequisites (parse → bind → semantic; parse → docs; parse → diagnostics) and `completed_passes` lists only what ran. `Capability::Containment` links parents/members without the reference query. `TreeFile::definitions(&MetadataSelection)` runs the locals query **once** per analysis; records, exports, references and semantic lint all take that result (`*_from` methods) — never call `symbols()`/`exported_symbols()` inside the pipeline. De-duplication happens before metadata: the winner supplies each field, later equal-priority captures only fill what it lacks (while no signature is known, a later capture's signature, visibility and `static` flags replace the current ones together). Unselected kinds stay light (name, kind, span, container) and are bound, then dropped; selecting a type kind also selects Field/Property/Constant. Binding is by name through `NameIndex` (first record of the name); containment is not: `resolve_containers` (`smallest_enclosing`, then `find_container_by_identity`) takes the smallest enclosing same-named record, else the record of `container_kind` whose in-file qualified name qualifies the member's (nearest preceding on ties, Lua repeated `local M`), else the only one whose qualified name agrees with the member's owner path (`agrees_with_owner_path`: a sole local `W` never owns `other::W::probe`; `resolve_container_kind` applies the same rule to `SameFileType`/`SameFileTable` kinds), else none — never the first of a repeated name (`symbols_owner_identity` is the row table, `ABSENT_OWNER_MATRIX` the absent-owner rows); `RefSet` dedupes resolved refs by ID and unresolved ones by name + qualified name. `semantics.is_recursive` is always `Unknown` when the semantic pass runs. `work_counters()` (atomics) proves skipped work in tests; `symbols_query` is the L1 file to extend.
- **Predicates and the displayed prefix are post-lookup** (`file/query.rs`, `file/filter.rs`, `file/pattern.rs`, `file/naming.rs`): analysis and the cache produce *unfinished* results (in-file `qualified_name`, every record of the effective kinds); `SymbolQueryResult::finish` first applies the file-wide test rules (`is_test_file`: file names and the Rust package-`tests/` rule, which reads the nearest `Cargo.toml` and so can change while the file and its entry do not; records and entries keep only attribute/`cfg(test)` flags; a result remembers which records only that rule marked (`file_wide_tests`) and every `finish` undoes them before deciding again, so `narrowed` reuse follows a changed layout; `narrowed` returns `None` for a test-selector-filtered result whose file-wide decision changed; `cache_package_boundaries`, `symbols_flags::cached_test_flags_follow_the_package_layout` and `symbols_flags::narrowed_*_follow_the_package_layout` pin it), then prefixes every name not `is_namespace_rooted` (scan-root-relative `file#`, default the file's own directory, or a `NamingContext` module path), then applies `RecordFilter`. Never store or key on a finished result. A predicate pushes only what it reads into the effective kinds/capabilities (`SymbolQuery::settle`). `SymbolRef::qualified_name` in relations/exports stays in-file. `symbols_selection_filters` (`SELECTION_FILTER_MATRIX`, every predicate per language, block or whole-file regions) and `symbols_query_filters` (pushdown, cache isolation, `narrowed`, naming context, non-ASCII) are the tests to extend; tests that key records by in-file name use `symbols_fixtures::in_file_name`.
- **Modifiers have one order** (`facets.rs`): `Modifier` derives `Ord` in serialization order (Static, StaticStorage, Async, Const, Readonly, Unsafe, Mut, then `Custom` lexically); `normalize_modifiers` (sort + dedup) runs once per record at the end of `records_from`'s per-record step, so readers may push modifiers in any order. Never reorder the variants.
- **Written modifiers are the declaration's own tokens** (`extract/modifiers.rs::declaration_modifiers`, called in `records_from` for every `wanted` record; no capability): direct keyword children or the declaration's own modifier list (Java/C# through `attributes::modifier_host`, the whole field/local declaration, shared with the attribute reader), C++ `override`/`final` from the `declared_function_named` declarator, never body/type/parameter/string text. `Const` = Rust `const fn` only; `Mut` = Rust `let mut`/`static mut`/`mut x`/`mut self` bindings plus a method whose receiver is written with `mut` (a `&mut self` `self` record is not `Mut`); `Readonly` = TS/C#; `Custom` lowercase `abstract`/`final`/`override`/`virtual`, C# `sealed`, Swift `mutating`; keywords outside the spec table (Scala `sealed`, PHP `readonly`, Swift `open`) are not read. `symbols_modifiers::MODIFIERS_MATRIX` (one row per language, both query paths, grammar limits) is the table to extend.
- **Scope, qualified name and `async` are computed for every symbol** in `definitions.rs::materialize` (light records included). `extract/scope.rs::symbol_scope` walks out from `member_slot`: first function node / `is_local_scope_kind` → `Local`, first owner (`is_scope_owner_kind`: member owners minus bodies, plus `enum_specifier`/Go `struct_type`/`interface_type`, or any `is_unnamed_type_owner`) → `Member`, root → `Item`; bodies are skipped so `declaration_list` is decided by its parent (`mod_item`/namespace → keep walking; `impl_item`/`trait_item`/class → Member). Parameters and C++ template type params are `Local`; Go `method_declaration` and Lua `M.f`/`M:m`/`M.x` (`lua_table_of_name`) are `Member`. `extract/qualified_name.rs::qualified_names` runs once per file after materialize: a symbol's prefix is the nearest enclosing *naming* symbol (function/type/module/namespace/enum variant, by declaration or anchor node id, memoized), else a Rust `impl` self type's path as written (`rust_impl_prefix`: `impl right::T` → `right::T::m`; leading `super`/`crate`/`self` resolved; `impl_self_type_name` stays the bare container name), a named unextracted function node (C# static constructor, Java compact record constructor; a TS `internal_module` only if its capture is missing), else `ancestor_container`; it stops at `is_unnamed_type_owner`, and at the root takes the nearest preceding file-level `package`/`namespace` statement (`is_namespace_rooted`; consecutive Scala package clauses nest, `outer.inner`). A Scala block package (`package p { … }`, never extracted) is a rooting `Named` segment in `Builder::prefix` (`is_scala_block_package`), so siblings stay distinct and statement clauses before it prefix it (`example.symbols.p.W`); `file_level_namespace` excludes it, so declarations after the block keep the statement packages; the Scala `private[p]` reader takes block-package segments too (`symbols_namespace_bodies` holds the block-namespace sibling/nesting rows for every block-namespace language). Separators: `::` Rust/C++/Perl, PHP `\` after a namespace else `::`, Lua `:` before a method, `.` elsewhere; Perl package names are absolute. `stable_symbol_key` hashes the qualified name (ordinal per `(kind, qualified_name)`), so changing name construction changes IDs. `extract/modifiers.rs::is_async_declaration` reads the anonymous `async` token (Rust inside `function_modifiers`, C# inside `modifier`, JS/TS also through the `value` of the name's binding). Matrices: `symbols_scope`, `symbols_qualified_names`, `symbols_containment`, `symbols_async` (coverage guard requires one row per language).
- **Docs, flags and metrics** (`extract/doc_comments.rs`, `attributes.rs`, `flags.rs`, `metrics.rs`): `extract_doc_comment` keeps only doc-form comments per language (`is_doc_comment_text`: Rust `///`/`/**`, Go any comment in the group touching the declaration, Java/JS/TS/PHP/Scala `/**`, C#/Swift `///`/`/**`, C/C++ Doxygen forms minus the member-after `<` forms `///<`/`//!<`/`/**<`/`/*!<`), skips ordinary ones, drops in Go/JS/TS a comment run trailing the preceding code on its line (`trailing_comment_count`: Go line comment, TS leading-comment scan; Rust/Java/C#/PHP/Scala/Swift/plain C `///` keep same-line docs for the next item), climbs to a wrapper's comments only when the declaration has none of its own (`documents_through`: `export`/JS variable statement/`template`, Go `var`/`const`/`type` declarations and groups (`go/doc`: a spec's own doc wins), C/C++ single-declaration `extern "C"`, Scala single-method `extension`; `collect_doc_comments` returns an `Attachment` (`Absent`/`Empty`/`Text`) so presence survives normalization: in Go only `Absent` climbs, an attached group that reads empty (blank `//`, `/* */`, directives only) means no doc and no inherited `Deprecated:`, while every other language's empty own doc still climbs; detached (blank line) and excluded trailing comments are `Absent`; `symbols_flags::go_specs_are_documented_through_their_declaration` (`GO_WRAPPER_MATRIX`), `go_spec_attachments_decide_group_doc_fallback_even_when_empty` (`GO_SPEC_ATTACHMENT_MATRIX` × `GO_SPEC_ATTACHMENT_SITES`) and `declaration_wrappers_pass_their_doc_to_a_single_declaration` are the tables to extend; Go deprecation reads the same doc), removes block decoration per language in `clean_doc_comment` (Java/JS/TS/PHP/Scala/C/C++: one leading `*` per line; Rust `rustdoc_block_lines` = rustdoc's `beautify_doc_string`, C# `csharp_block_lines` = Roslyn's common line pattern, Swift `swift_block_lines` = `measureASCIIArt` at the comment's column: a `*` they would keep is text, so a misaligned star-only block and `/** * */` document; a line comment loses only its marker, `/// *` documents; `symbols_metrics::block_decoration_is_removed_only_where_the_doc_tool_removes_it` (`DECORATION_SHAPES` × `DECORATION_LANGUAGES`) is the table to extend; the docs pass's `parse_docs_facet` reads that cleaned text and strips nothing), reads Python's body docstring (`python_docstring`: a nonempty `str` constant, i.e. ordinary literals and their adjacent concatenation, prefix read from `string_start`; bytes and formatted literals are not docstrings; source never evaluated), and is `None` for Lua/Perl/Bash/Zsh; a context-less non-`Local` name is documented before its declaration, a C/C++ context via `declaration_for_context`. `declaration_attributes` reads full name + top-level arguments from the tree (`token_text`: comments skipped, so `#[cfg(/*c*/ test)]` is `cfg` + `["test"]`; a comma inside a string or nested list never splits; Rust outer attrs, C++ `[[…]]` (leading, plus those right after the captured name on its own declarator path, `declarator_id_attributes`: `int a [[deprecated]], b` marks only `a`; after a parameter list or array bound they belong to the type), Java/Scala annotations, C# lists, Python decorators through parentheses, Swift attributes) plus each one's raw `value` (text between the outer delimiters, trimmed, comments kept; `None` without a list, `Some("")` for `()`; `From<&SyntacticAttribute> for Attribute` is the record conversion). `records_from` parses them **once** per declaration and hands the slice to `is_deprecated`/`has_test_attribute` (they take `&[SyntacticAttribute]`, never re-read the tree); the same list fills the record's `attributes` (`Attribute::from`, every `wanted` record, no capability; TS/JS decorators too: `ecma_decorators` reads the `export` statement's, the TS `class_body` siblings before a method, then the node's own; `decorator_attribute` (Python/TS/JS) keeps a non-path, non-call-of-path expression whole as the name, so `@(lambda f: f)` is `lambda f: f`; TS 0.23.2/JS 0.25.0 parse only paths, calls and parenthesized ones); never split attribute or doc text with string operations. Go docs never go through the block cleanup (`clean_doc_comment`): `go_comment_group_text` removes exactly `//` (plus one space) or `/*`…`*/` (plus the whitespace after `/*`), keeps every other `*` as text (`/* *Deprecated: … */` is no marker, `/* * */` is documentation), keeps interior blank lines and removes only the group's common indentation (after dropping `//` directives exactly as `go/ast`'s `isDirective`, `is_go_directive`: no paragraph break left behind; `symbols_flags::go_directives_and_trailing_comments_are_not_documentation` (`GO_DIRECTIVE_MATRIX` × `GO_DIRECTIVE_SITES`, `GO_TRAILING_ROWS`) and `doc_readers_skip_comments_their_language_does_not_attach` (`TRAILING_DOC_CASES`) are the tables to extend), so the `Deprecated:` paragraph rule (`has_deprecated_paragraph`) treats a still-indented line as a code block (`symbols_flags::go_doc_text_keeps_literal_stars`, `GO_DOC_MATRIX`, is the row table to extend); doc-tag lines inside a CommonMark fence are not tags (`lines_outside_fences`: a fence closes only on a run of its own character at least as long as the opening run with nothing after it; `symbols_flags::doc_tag_readers_follow_fence_boundaries` (`FENCE_MATRIX`) is the row table to extend). `is_deprecated`/`has_test_attribute`/`is_in_rust_cfg_test` (in `records_from`) and `is_test_file` (only in `finish`, never in a stored record) implement the spec tables exactly (no prefix or substring matches; `cfg(any(test,…))` is not test). `parameters` = own list (all curried Scala lists) minus Rust `self` (`declared_parameters` returns `None` when no list exists and names a pattern-bound parameter by its pattern text, Rust `(a, b)`/`_`, JS/TS `{x}`/`[a,b]`, so destructured parameters count and keep their type for `--param-type`; a TS `this: T` is skipped; `declared_parameter_count` counts unnamed C/C++/Go parameters, `(void)` = 0; `symbols_signatures::patterned_parameters_keep_their_pattern_and_type_text` is the row table to extend). Matrices: `symbols_flags` (`FLAGS_MATRIX`, run on a temp copy because the in-place Rust fixture sits under `tests/`), `symbols_metrics` (`METRICS_MATRIX`), `symbols_kind_coverage` (`unsupported` vs `landed` `FEAT` cells, `no_construct` kinds).
- **Generic parameters are the declaration's own list** (`extract/generics.rs::declared_generics`, called in `records_from` for every `wanted` record; no capability): `type_info.generic_params` (`GenericParam` per written parameter: name, `Type`/`Lifetime`/`Const`, bounds as written, default without `=`, variance as written, zero-based `position` = index in the written list) and `type_info.constraints` (requirements whose subject is not a bare declared parameter: Rust `where T::Item: Copy`, a method's `where` on its impl's `T`, Swift `U == Int`, C++ `requires`, a C# `where` naming no declared parameter). Never read an enclosing type's/impl's list: C++ reads only the `template_declaration` that is the declaration's direct parent, Go the name's own `type_spec`. Bound lists split at grammar entries only (skip comment children); Scala keeps each bound's operator (`>: L`, `<: U`, `: Ctx`, the context `:` is a separate `bound:`-fielded token) and its invariant parameters are flat `name:`/`bound:` siblings in the list. A C++ concept-constrained parameter reads as `Const` (grammar limit). Position overflow (`u16`) is `TreeHuggerError::TooManyGenericParameters` (CLI kind `too_many_generic_parameters`), never truncation; `records_from` returns `Result`. The older `TypeMetadata::type_parameters` readers in `type_metadata.rs` are separate and unchanged. `symbols_generics::GENERICS_MATRIX` (one row per language, limits included) is the table to extend.
- **A callable's body span is its own body, and `metrics.lines` counts it** (`classify.rs::callable_body_span`, `metrics.rs::body_lines`): `records_from` computes `source.body_span` once (callables: `callable_body_span(declaration, …)`, others: `body_span_for_declaration(anchor)`) and passes it to `symbol_metrics`, so the span and `lines` never disagree. Callables read only the declaration's `body` field (a C/C++ anchor is the `function_declarator`, the body is on the `function_definition`), never the type-body fallbacks; C# `arrow_expression_clause` gives its expression (no `=>`); Python/Lua run from the first child after `:`/`parameters` (comments the grammar leaves beside the `block` included, a header-line comment not) to the last before `end`; an empty Lua body (no `block` node) is the empty span at `end` → `Some(0)`; every span is trimmed of trailing whitespace (Scala `indented_block`, Perl `block`). `lines` = `0` for an empty span, else `end.line - start.line + (end.column > 1)`; `None` for other kinds. `body_node_for_declaration` (declaration/signature text cuts) is separate and unchanged. `symbols_metrics::BODY_LINES_MATRIX` (one row per language, LF and CRLF) is the table to extend.
- **Go receiver methods and Lua table members have containers**: `find_symbol_container` returns the receiver base type (`go_receiver_type_name`, through pointer and generics) as `SameFileType` for a Go `method_declaration` only (its receiver/params/locals stay container-less); `materialize` gives a Lua `M.f`/`M:m`/`M.x` the table text with `ContainerLookup::SameFileTable` (kind = unanimous kind of same-named `Item`-scope symbols).
- **Declared relations are written bases resolved once per file** (`extract/relations.rs::declared_bases`, `tree_file/relations.rs::link_relations`, called at the end of `records_from`): `extends`/`implements` keep each base as written (`WrittenBase.text`, trimmed; C++ access/`virtual` dropped) plus a `BasePath` (segments, generic args dropped; Rust `crate`/`self`/`super`, PHP leading `\` as `PathAnchor`) when it is a name; `Declarations` indexes **every** record (selected or not) by in-file qualified name once, and `resolve` joins the path onto each enclosing scope innermost first (Rust stops at the nearest module): the first scope where any declaration has the name decides, resolving only for exactly one eligible kind (so aliases, variables and duplicates shadow), and a visible type parameter never resolves; imports are not read. Rust impls are walked once per file (`impl_header`): a positive trait impl's trait as written goes to members (`impl_of_member`) and the resolved self type; negative/inherent add nothing; composite self types (`&W`, `dyn T`) never attach. Under Containment (`RecordFacts::containment`) every record whose `ContainerLookup::SameFileType { impl_item }` names an impl gets `relations.container` (+ `parent` when resolved; else the self type as written, no ID), and `analysis::link_containers` keeps pre-linked records; so a Rust entry widened by Containment parses (`record_passes_suffice` takes the language). C#/Swift/Scala lists are not read (mixed-list ruling pending). `symbols_relations::RELATIONS_MATRIX` (one row per language) and its Rust impl tests are the tables to extend; user contract `tree-hugger/docs/type-relations.md`. `link_relations` runs once per analysis (`WorkCounters::relation_links`). `symbols_relations::declaration_fact_sources` (every generics, modifiers, body-lines and relations matrix source plus the Rust impls) feeds the query/index equivalence (`symbols_query::declaration_facts_*`), cache projection and widening (`symbols_cache::declaration_fact_*`) and adapter-loss (`symbols_adapter::record_only_declaration_facts_are_lost_through_the_older_model`) tests; a new declaration-fact matrix adds its sources there.
- **Readers get a `DeclarationContext`** (`declaration.rs`): language, source, final kind, `scope` (`is_local()` = `Local`; replaced `is_local_declaration`), the captured `name` node (one declaration can name several things: `int a, *b;`, Swift `case east, west`), the file `root` (same-file lookups: Rust trait level, earlier C/C++ `static` prototype, Scala `package` segments) `is_ecma_module` (computed once per JS/TS file by `is_ecma_module_file`) and `memo` (`VisibilityMemo`: C/C++ `static` prototypes and namespaces, C++ access sections per class body, Scala package segments, Rust trait levels, each derived once per file on first use). Add a field here rather than re-scanning the file per symbol; any context several declarations share is built once and looked up (`find_symbol_container` takes a per-file `SiblingScopes`, `is_in_rust_cfg_test` a per-file inner-attribute map, `qualified_names` a one-pass file-namespace index), and `symbols_shared_context` asserts `shared_context_steps` grows linearly. Key shared context by the decision, never by bare name alone: names repeat across modules, namespaces and overloads (Rust trait levels aggregated per name, C++ `static` lookups by complete `FunctionIdentity`, namespaces by path, Scala segments a set, container kinds by qualified name/segment tail in `definitions.rs::KindAgreement`, container records by `analysis::smallest_enclosing` (one span-ordered sweep per name) then `OwnerIndex`, export targets by name span); its `REPEATED_SHAPES` are the rows to extend.
- **The C/C++ earlier-`static` scan matches only the same function** (`visibility/c.rs`). `declared_functions` (`c_declarators.rs`) takes the `function_declarator` nearest the name with only parentheses between, so a returned function-pointer, function-reference or array-pointer type is never the function; an unqualified C++ declaration has exactly its enclosing namespace; a qualified one resolves its first qualifier against the file's `namespace` blocks, nearest enclosing first; a function template (under `template<…>` or abbreviated with an `auto` parameter) also compares its header, leading and trailing return type, and the `requires` clause after the header and after the parameters (`TemplateSignature`; never `noexcept`). Whatever cannot be established is a miss, never a match. `symbols_visibility::earlier_static_linkage_matrix_holds_for_c_and_cpp` is the row table to extend.
- **C/C++ signatures read the named function's own declarator** (`signatures.rs`). `extract_signature` gets the captured name; `declared_function_named` (`c_declarators.rs`, shared with linkage) picks that function's `parameter_list`, so a pointer/reference/array return keeps its parameters and a returned callback's never replace them. Each parameter (`parameter_declaration`, `optional_parameter_declaration` with its default, `variadic_parameter_declaration` pack, C `variadic_parameter` and the C++ `...` token) is named by `declarator_name`, which follows a function declarator only through its `declarator` field. `symbols_signatures::c_and_cpp_signatures_read_the_named_functions_own_parameters` is the row table to extend.
- **C++ const-object linkage reads the declarator nearest the name** (`is_const_object`, `visibility/c.rs`). It reuses `declarator_layers` (parentheses and attributes add no layer; arrays are skipped to their element), so `char **const p` is const and `char *const *p` is not; a reference is checked before the `constexpr` shortcut and is never const. Any new reader of a declarator's type goes through `declarator_layers`, not the outermost node. `symbols_visibility::const_object_linkage_matrix_reads_the_outermost_declarator` is the table to extend.
- **Membership starts from `member_slot`** (`classify.rs`): the declaration or its outermost transparent wrapper (`is_transparent_wrapper_kind`: C++ `template_declaration`, C/C++/C# `preproc_*` conditionals, Python `decorated_definition`, Scala `extension_definition`). Kind, both `static` readers, C++ and C# access (owner read from `member_slot(..).parent()`) and C/C++/C# field, interface-property and enum-member lists (`flattened_members` in `type_metadata.rs`) all look through those wrappers; add a new wrapper there, not in one reader, and route any new reader of a declaration's parent or a type body's children through `member_slot`/`flattened_members`. Swift `#if` and C# `#region` are flat `directive`/`preproc_region` siblings, not wrappers.
- **Member kinds are validated, not trusted.** `refine_symbol_kind` demotes any Method/Constructor (to Function) or Field/Property (to Variable), whether from a query capture or a classifier rule, when `is_function_local_member` finds a function before any type or object owner above the member slot (C# local functions are captured as methods by the vendored query). Lua `M:m`/`M.f` names its owner and is exempt. `symbol_scope` (which decides `DeclarationContext::is_local()`) also stops at the nearest owner, so members of a type declared inside a function keep their access; a JS/TS `constructor` is a Constructor only directly in a `class_body`. Other constructors: Java/C#/Swift by query capture (C# `static` excluded in the query); Python `__init__` and PHP `__construct` (`eq_ignore_ascii_case`) by name only after the declaration is a method; C++ by syntax alone (`declaration`/`function_definition` in a class body with no `type` field whose function declarator, reached through attributed/parenthesized wrappers only (`transparent_declarator`), is the `declared_function_named` one and declares the captured `identifier` itself: `C [[deprecated]] ()` and `(C)()` qualify, `~C`, `operator T`, `C::C` and pointer/reference wrappers do not; never a name comparison). Scala body `val`/`var` is reclassified to Field by a classifier arm, one per name of a grouped `val a, b: Int` (the query captures each `identifier` under `identifiers`; the field reader lists every name with the shared type). Recorded limits (Scala access-modified `enum`, tree-sitter-scala 0.24.0) live in `symbols_kind_coverage::GRAMMAR_LIMITS`; a demonstrated limit goes there, never in `unsupported` (which requires a clean parse).
- **Unnamed owners end the container search.** `find_symbol_container` returns no container at `is_unnamed_type_owner` (anonymous classes, Scala `given … with` instances, JS/TS class expressions and object literals, tagless C/C++ `struct`/`class`/`union` that is not an anonymous member) instead of climbing to the named type outside.
- **Implicitly static members are static.** A member the language makes static without the keyword sets `is_static` at extraction: Java interface constants, C# `const` fields, Rust associated `const`s (impl and trait), PHP member `const`s (`const_declaration` is captured as Constant; a top-level `const` is not a member). C++ `static constexpr` says `static`; Swift/TS/JS/Scala/Python have no keyword-less case beyond Scala `object` members and Python class attributes.
- **Static readers match nodes, not text, and stop at the nearest owner.** A member-`static` reader matches the declaration's own keyword/modifier token or decorator expression (Python: `staticmethod`/`classmethod`, bare or `builtins.`-qualified, and a `ClassVar` annotation head under any qualifier; never a substring or prefix) and asks only the type that directly owns the member (Scala: the member slot's `template_body` belongs to an `object_definition`). A new reader must not `.contains()` node text or climb past a class, trait, enum or anonymous class.
- **Field lists are completed from the standalone members.** `align_field_metadata` (`tree_file/definitions.rs`) pairs each type-metadata entry with the standalone `Field`/`Property`/`Constant` of the same name whose container node is that type (k-th entry with k-th symbol of a name), copies its `visibility`/`is_static`, **adds** any standalone member no reader listed (doc comment, no type) and orders the list by source position, so `hug classes` and `hug symbols` list the same members. Readers still matter: they supply type annotations and the entries with no standalone symbol (record components, Rust tuple positions, Scala class parameters). Enum owners get no field entries; their variant lists are completed from standalone `EnumVariant`s by `align_variant_metadata`/`reconcile_variants` (Enum owners only: a Swift enum is `Type` and has no list; `discriminant` from the member's `value` field; a standalone-only entry is unit, so a payload-bearing form needs a reader entry: Rust tuple/struct variants, Scala `full_enum_case` class parameters as `struct_fields`, every clause, defaults never a discriminant), and `MetadataSelection::for_kinds` keeps variant metadata when Enum is selected, and Rust `impl` members (by-name container) never join the struct's list. `symbols_field_lists` pins complete per-language lists on both paths; a new member form needs a query capture **and** a reader case.
- **Type metadata lists only fields and properties.** Interface, protocol and trait methods are standalone `Method`s, never `FieldInfo` entries (Java interface constants, C# interface properties, Swift protocol property requirements and Rust trait associated constants are listed). Go interface method elements are neither records nor entries: `extract_go_interface_methods` matches `method_spec`, which tree-sitter-go 0.25 no longer produces (`method_elem`); `symbols_field_lists` pins the empty list.
- **C/C++ names sit under any declarator chain.** The C/C++ `locals.scm` capture a name by its immediate parent (pointer, array, parenthesized, attributed declarator; C++ adds reference and, for member functions, `(function_declarator declarator: (field_identifier))` under any return declarators), never by a fixed outer shape. `classify_symbol_kind` makes a captured variable/field a Function (Method in a type body) when `declared_function_named` says its declarator declares one (`int f [[deprecated]] (int)`, `int (f)(int)`), so a function-pointer field stays a Field; `implied_c_function_context` (`declaration.rs`) supplies the context such a capture lacks. `c_field_name` (`type_metadata.rs`) is `declarator_name` minus function declarators. Typedef names behind a declarator are TypeAliases with the typedef's doc comment and no field list. `symbols_declarator_wrappers` is the row table to extend. Anonymous `struct`/`union` members (`anonymous_member_declaration`, `classify.rs`) belong to the enclosing type, with the C++ access of the section holding them.

## Key Architecture

- **Runtime**: Built on the tree-sitter Rust runtime (`tree-sitter = "0.26.3"`) plus one `tree-sitter-<lang>` grammar crate per language, all at independent versions — see [Tree-sitter Internals](./tree-sitter-internals.md)
- **Query vendoring**: Uses nvim-treesitter `locals.scm` in `lib/queries/vendor/<lang>/`
- **Custom queries**: `lint.scm`, `references.scm`, `comments.scm` per language
- **Capture naming**: `@local.definition.<kind>` for symbols, `@diagnostic.<rule-id>` for lint
- **Query caching**: Global `OnceLock<QueryCache>` for thread-safe caching
- **Builtin database**: Per-language builtin lists to avoid false positive undefined-symbol/undefined-module errors
- **Schema v2 module, schema version 4.1**: `shared/schema_v2` with facet-based `SymbolRecord` and staged `FileSymbolIndex`; records, file indexes, CLI envelopes and analyzer fingerprints emit `SchemaVersion::V4_1` (the module keeps its historical `v2` name). Records carry no timestamps, so output and cache entries are byte-identical across runs; the container is `identity.container_name`
- **CLI declaration reports** (`cli/src/`: `declarations.rs` drives, `selection.rs`, `order.rs`, `report.rs`, `render.rs`, `classes.rs`, `scan.rs`, `prelude.rs`; `main.rs` only wires): `Declarations::query(format, &Selection)` is each command's fixed kind set ∩ `--kind`, the selectors as `SymbolQuery` predicates, and only the capabilities it shows (`classes` and `--tree` add containment; `--json`/`--jsonl` add `Flags` and `--stats` adds `Metrics`, all in `ReportFormat::capabilities`; no listing runs references, semantics, docs or diagnostics); `--json-full` asks `complete_index()` (every capability, flags and metrics included). `scan::in_file_order` runs rayon chunks of `4 × threads` and delivers results in file order, so `--json` is byte-identical across thread counts and cold/warm. The JSON envelope is written piece by piece with `errors` always last; records are projected (`report::trim_record`: scan-root-relative `file_path` and stable-key file segment, no `declaration_text`/source text/attached comments; IDs untouched). `tree_hugger::portable_relative` (`lib/src/file/naming.rs`, re-exported by `report.rs`) is the one path projection (entries, `errors[].file`, records, the qualified-name prefix; the CLI query passes `scan_root`): files outside the root get `../` routes via `biscuit_file::PathIdentity`, retried with directories canonicalized (macOS `/var` vs `/private/var`), and keep their absolute portable path only when no route exists (another Windows drive/share); the key's file segment is parsed from the key itself (`{lang}::{file}::{Kind:?}::`), never matched against `source.file_path`, because prelude aliases are keyed by the prelude file but located at the target. The one renderer (`render::Renderer`, built from `RenderOptions`) is `Prose` items in `UnorderedList`s (and a `Table` for `--format table`); it wraps only when `RenderOptions::width` is set (`main::wrap_width`: stdout is a TTY or `CLICOLOR_FORCE`, width from a positive `COLUMNS` else detection; `--plain`/`--format plain`/redirected output never wrap), and every source-derived string goes through `render::literal` (control chars → `\u{..}`, then `Prose::escape_text`, which on its own passes ANSI CSI/OSC through) or `render::neutralized` for table text cells; labels come from `identity.keyword` only for Type/Variable/Field/Constant, else the kind. Terminal output streams: `declarations::write_terminal` flushes after each `in_file_order` chunk (so a closed pipe stops scheduling within a chunk), `classes` writes per file, and `render::ListingStream` holds only the current unit (one file; one directory under `--group-by-module`; one package under flat `--sort name|kind` or `--prelude`) after `render::listing_order` reorders files by package, directory, path. Package sections are decided up front from `listing_order`'s package count (two or more packages in the scan, declarations or not), so nothing is held beyond the current unit; never reintroduce a layout decision that waits on later declarations, or a whole-scan accumulator ahead of the renderer. `write_json` flushes at the same chunk ends (`scan::flush_at_chunk_end`), so a closed pipe is noticed even when no entry fills the buffer. `--prelude` analyzes the prelude and its targets with an every-kind query first and the scan reuses them via `SymbolQueryResult::narrowed` (one parse per file; the reuse map, `collect_files` output, `SourceFile::path()` and the CLI roots are all `file_identity` spellings, so relative/`..`/symlinked inputs never split reuse or the cache); prelude exports are synthetic records (`visibility.is_synthetic`/`is_reexported`, own ID from the prelude file). Every output branch that writes declaration records (`--json` files, `--json-full` `index.symbols`, terminal listings, `classes` containers and member sections) orders through `order::compare`/`sort_file_records`: extraction order is not source order (C emits a typedef's inner `struct` specifier before the alias, Go a receiver parameter before its method). A failed file is stderr + `errors` + exit 1; a broken pipe exits 0 quietly. User-facing contract: `tree-hugger/docs/symbol-reports.md` (`order.rs` points at it); update it with any change to kind sets, envelope, order or `--verbose`.
- **Output modes** (`cli/src/output.rs`, `stats.rs`): `output::resolve(GivenOutput)` is the one place the "exactly one mode" rule lives (`--stats` composes with `--json` or a `--format`; `--sort`/`--signatures`/`--tree` reject `--stats`; `--tree` is terminal-only and not on `classes`); `main` maps the `OutputMode` to `declarations::ReportFormat`. `--sort` is `Option<SortOrder>` so "given" is detectable. `declarations::selected_records` is the one projection the JSON formats and the table share (classes: containers sorted, `displayed_members` trimmed); `write_json_lines`, `write_table`, `write_tree` stream per file in path order (not `listing_order`), `write_tally` holds only counters (`classes` counts containers). `selected_records` trims records (`trim_record`), so their `file_path` is scan-root-relative: anything that turns one into a file (the table's OSC 8 Location link) must join it onto `scan_root`, never resolve it against the working directory (`reports::table_location_links_resolve_against_the_scan_root`). `render::Outline` nests by `relations.container` ID, falling back to `identity.container_name` (a type declared in another file) for a marked `[in …]` heading that is never a record (an unlinked member is headed by its in-file owner path when that qualifies the name further, `[in other::W]`, so distinct external owners never share a heading); it reattaches anything a containment cycle would orphan. `stats::Stats` keys are the records' serde spellings (`Function`, `Public`, `Rust`); synthetic exports count in the maps and apart from doc coverage. Tests: `cli/tests/symbols/reports.rs`, goldens `symbols.{table,tree,stats,jsonl}`, `symbols-stats.json`, `symbols-signatures-full.plain` (`--format plain` shares `symbols.plain`); any test forcing color must pin `COLUMNS` (forced color wraps at the detected width).
- **CLI filters**: `hug <subcommand> <filter...>` with all-files default scan for symbol commands. A pathless scan honors `.gitignore` and skips test-data dirs by default (`**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**`); an explicit path/glob re-includes a targeted file. `lint` scans hide clean files (single-file lint still shows `(no diagnostics)`).
- **Record selectors are one path** (`cli/src/selection.rs`): `SelectionArgs` (flattened into the declaration and `classes` args; `TargetArgs` = `--file`/`--name`/`--exact`, also on `imports`) resolves to a `Selection`, which becomes library predicates; the CLI keeps no name matcher of its own (positional names and `--exclude-symbols` are `NamePattern`s; `normalize_symbol_glob`/`matches_symbol_filters` are gone). Records are filtered by the library after the cache (`SymbolQueryResult::finish`); only `--prelude` is applied in `declarations::prepare` (its synthetic exports run `query.selects`/`query.matches` there). `symbols` defaults to `Item`+`Member` scopes (`SYMBOLS_DEFAULT_SCOPES`); other commands every scope. `classes` uses `classes::ClassSelection`: an analysis query with no predicates (container kinds ∪ member kinds, capabilities of both filters), identity predicates + `--kind` on containers, `--in` as the container's own name, shape predicates on members (fields hidden when one is given); `--static-only` conflicts with `--instance-only`. `imports` filters `index.imports` with `ImportSelection` (name, alias, original's last segment, or the whole original for a qualified pattern); positional tokens stay files. `--file` values: globs and unresolved values join the walker inputs, a `FileReference` that resolves to a file is an explicit target (`scanner::collect_selected_files`: bypasses `.gitignore`/fixture exclusions, not `--exclude-files`; never widens the scan). A bare `rs` is a name, not a file filter (`is_file_filter_token` needs a path, glob or `x.rs`). `--json-full` rejects every record selector (`SelectionArgs::given_flags`). Tests: `cli/tests/symbols/filters.rs`; tests that need locals from `symbols` pass `--all-scopes`.

## Detailed Documentation

- [API Reference](./api-reference.md) - Complete type definitions and methods
- [Symbol Reports](../../../tree-hugger/docs/symbol-reports.md) - The declaration commands end to end: kind sets, query → passes → render flow, JSON envelope and `errors`, `--json-full`, `--sort` and tie-breakers, `--verbose`, exit status
- [Symbol Kinds, Visibility and `static`](../../../tree-hugger/docs/symbol-kinds-and-visibility.md) - Per-language kinds, containers, visibility levels and defaults, the two meanings of `static`, declaration text, current limitations
- [Declared Type Relations](../../../tree-hugger/docs/type-relations.md) - `extends`/`implements` per language, when a base gets a same-file ID, Rust impl ownership
- [Query System](./query-system.md) - Tree Hugger's `.scm` queries, capture conventions, and how to extend them
- [Diagnostics](./diagnostics.md) - Lint rules, ignore directives, dead code detection
- [Tree-sitter Internals](./tree-sitter-internals.md) - The underlying runtime: Parser/Tree/Node API, query mechanics, AST debugging, and grammar version/ABI compatibility

## Diagnostic Metadata (Phase 1)

Every diagnostic now carries stable metadata for policy and display:
- **Category**: `correctness`, `suspicious`, `style`, `performance`, `pedantic`, `restriction`, `experimental`
- **Confidence**: `high`, `medium`, `low`, `experimental`
- **Source**: `tree-sitter-query`, `semantic-analysis`, `syntax-parser`, `external-tool`
- **Severity**: `default_severity` and `effective_severity` (after policy)

## Rule Registry

Built-in rules are registered in `RuleRegistry` with metadata:
- Rule id, version, title, category, default severity, confidence
- Language support, default enablement, examples, caveats
- `requires_experimental_semantics` gates low-confidence semantic rules

Default-off pattern rules are silent unless opted in with `--warn <rule>` or `--deny <rule>` (the registry's `enabled_by_default` flag is enforced by the CLI policy layer, `apply_lint_policy`):
- Debug artifacts: `console-log`, `print-call`, `fmt-println`.
- `restriction` category: `unwrap-call`, `expect-call` — valid, deliberate constructs, not anomalies. Enable per-rule or via `--deny category:restriction`. `--allow`/`--strict` never enable an off-by-default rule.

Experimental rules (`undefined-symbol`, `unused-symbol`, `undefined-module`) are disabled by default. Enable with `TreeFile::experimental_semantics = true` or `--experimental-semantics` in the CLI.

## Dead Code Detection

Dead code is detected only within the same block as a terminal statement (`return`, `panic!`, `break`, etc.). Terminal statements inside nested control-flow constructs (e.g., an `if` branch, a `match` arm, or a closure) do not propagate dead-code detection to the enclosing function or method body.

## CLI Policy

Policy selectors target rules by ID or category (`category:correctness`):
- `--deny <rule>`: promote to error
- `--warn <rule>`: promote to warning
- `--allow <rule>`: demote to info
- `--strict`: promote all warnings to errors (does not enable experimental rules)
- Syntax diagnostics are always errors and always fail the exit code

## Testing Requirements

When modifying queries or symbol extraction:

1. Every language with type constructs must have type distinction tests
2. All typed languages need `types.*` fixture files
3. Bug fixes require regression tests
4. Kind, visibility and static expectations live against the per-language
   fixtures in `lib/tests/fixtures/symbols/<query_name>.<ext>` (one primary
   file per language, plus JS/TS script, CommonJS and module-by-extension
   extras), read through `lib/tests/l1/symbols_fixtures.rs`;
   `symbols_coverage` fails when a language has no primary fixture or a
   fixture stops parsing cleanly
5. Run `just test` in `tree-hugger/` to verify (the library package is
   `tree-hugger`, not `tree-hugger-lib`)
6. CLI rendering checks over those fixtures live in the `cli/tests/symbols/`
   binary: `Scratch` (in its `main.rs`) copies fixtures (`include_str!`) into a
   scratch Git repo and runs `hug` with isolated `HOME`/config and
   `TMPDIR`/`TMP`/`TEMP`, which is where the persistent cache lives. `hug`
   will not scan files under the repo's own `fixtures/` directories, so CLI
   checks always need that copy. The cache-upgrade tests hard-code the
   schema-2 literal key and the analyzer key formula for each earlier
   extractor revision (`revision_key`, with that revision's record
   schema: `3.0` through revision 4, `4.0` from revision 5); keep them
   unchanged. That formula was retired with revision 6 by the computed
   `SymbolCache` key, which cannot be hard-coded (it digests the grammar and
   query text): a later bump adds a `StaleRevision` with `OldKey::Computed`,
   whose `computed_revision_key` rebuilds the key from the public
   `FingerprintInputs` with only `tree_hugger_version` changed, after
   asserting the same formula reproduces the current key (revisions 6, 7,
   8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21 and 22 so far; the current revision is 23). Revision 17 (schema 4.0, no `metrics.lines`) cannot be a servable stale payload, so `entry_from_extractor_revision_seventeen_is_not_served_after_the_schema_bump` checks it by hand: rejected under both keys. `cache_robustness` pins which malformed
   shapes of the entry fields and twenty-four record columns are served or
   recomputed (its `WrongType` cell writes `"123"` for a numeric field) (tables in `docs/cache-design.md`); a schema change that adds a default or makes a
   field optional must update that row. The cache tests probe through
   `hug --json-full` (the complete index), since `--json` reports only the
   command's records. `Scratch` pins `RAYON_NUM_THREADS=2` so concurrent
   test processes do not each start a worker per core (tests that compare
   thread counts set it explicitly); `Scratch::process` gives a raw
   `std::process::Command` for pipe tests. `--verbose`'s stats line is how a
   CLI test proves work counts (`parsed N files, … 0 diagnostics runs`)
7. `lib/tests/fixtures/symbols/multi/` is a 16-language tree (one file per
   language, some under `nested/`, `Widget` declared in two files) for
   multi-file, ordering, parallel and golden tests;
   `symbols_coverage::multi_tree_has_one_clean_file_per_language` guards it.
   `golden::plain_symbols_over_multi_match_the_golden` (CLI `symbols` binary)
   pins `hug --plain symbols` over it against
   `cli/tests/fixtures/golden/symbols.plain`, and
   `golden::json_symbols_over_multi_match_the_golden` pins `--json symbols`
   against `symbols.json` (keys sorted, `root_dir` and every 64-hex record ID
   normalized, since IDs hash the absolute path); regenerate with
   `HUG_UPDATE_GOLDEN=1` only for a deliberate output change. `order.rs`
   builds its oracle from the JSON records (add a root `Cargo.toml` to a
   scratch tree, or every manifest-less directory is its own package and the
   listing splits). Grammar limits
   found while writing fixtures: C# rejects an attribute between modifiers,
   and tree-sitter-zsh rejects a function body brace on the line after the
   header
8. `lib/tests/fixtures/symbols/test_files/` holds one file per test-file
   name rule (Go `*_test.go`, Python `test_*.py`/`*_test.py`/`conftest.py`,
   JS/TS `*.test.*`/`*.spec.*`, Rust under a package `tests/`), listed in
   `symbols_fixtures::TEST_FILE_FIXTURES` and guarded by
   `symbols_coverage::every_test_file_rule_has_a_clean_fixture`. The Rust
   one is a test in place only because it sits under this crate's `tests/`;
   assert a non-test reading from a scratch-package copy.
   Every report matrix is filled (`symbols_scope`, `symbols_qualified_names`,
   `symbols_containment`, `symbols_async`, `symbols_flags`, `symbols_metrics`,
   `symbols_kind_coverage`, `symbols_selection_filters`) and
   `symbols_coverage::every_language_has_one_row_in_each_report_matrix`
   requires one row per language in each

The CLI's real-terminal tests are behind the internal `terminal-tests`
feature: `level2_god_files` (the `god-files` report) and
`level2_declarations` (the `functions`/`types`/`symbols`/`classes` renderer
over `cli/tests/fixtures/declarations/`, run without `CLICOLOR_FORCE` so `hug`
detects the pane's TTY; the styling scenarios pin `COLUMNS=10000`, while
`check_wrapping`/`check_table` assert `hug`'s own wrapping at the pane width
(`Pane::cols`) and at an injected `COLUMNS`; `check_outline`, `check_stats`,
`check_signature_modes` and `check_classes_table` compare `--tree`, `--stats`,
`--signatures full|none` and `classes --format table` row for row and need an
80-column pane). Every styled view a feature adds needs a check here, called
from all three backend entry points. Ordinary local L1 omits their harness and targets;
`just test-l2` and CI enable them. A new `level2_*` target needs a `[[test]]`
entry with `required-features = ["terminal-tests"]`. Both targets read panes
through `cli/tests/support/terminal_report.rs` (`#[path]`-included): every
command runs between unique `printf`-printed begin/end markers, the capture
is the pane's whole history (tmux `capture-pane -S - -J`, WezTerm
`capture_scrollback`, Kitty `--extent=all`), and `Report::scope` keeps only
the rows strictly between the markers, failing on a missing, repeated or
reversed marker. A shared pane keeps earlier reports, other tests and a
colored prompt, so never assert on a whole capture. Style checks name one
uniquely matching row and read the cell's effective SGR state replayed from
the capture's start (tmux re-emits only changed attributes, across rows),
not the escapes next to the text; WezTerm's split soft wraps are rejoined
only for the whole-line long-row check (`has_logical_row`). The helpers are
L1-tested on synthetic captures in the `symbols` binary (`terminal_scope`).

## Resolver and Adapter Architecture (Phase 3)

### Semantic Resolvers

The `resolver` module defines interfaces for project-level semantic analysis:
- `ProjectContext`: Root path, manifests, language configs, dependency hints, generated-file markers, target environment
- `SemanticResolver` trait: Reports confidence and supported-scope metadata, returns `ResolverOutput` with diagnostics and resolved imports
- `ResolverScope`: `FileLocal`, `Module`, `Project`, `Workspace`

Resolvers are gated until corpus precision is measured. Low-confidence resolver-backed diagnostics use `DiagnosticConfidence::Low` or `Experimental`.

### External Diagnostic Adapters

The `adapter` module defines interfaces for delegating to external tools:
- `ExternalDiagnosticAdapter` trait: `name()`, `version()`, `is_available()`, `supported_languages()`, `run()`
- `AdapterConfig`: Tool path, config path, extra args, environment, strict mode, timeout
- `AdapterResult`: Normalized diagnostics + `AdapterMetadata` (tool name, version, config files, exit status, elapsed time, cache status, fix availability)
- `AdapterError`: `ToolNotFound`, `IncompatibleVersion`, `ExecutionFailed`, `ParseFailed`, `UnsupportedLanguage`, `ConfigError`

Adapters preserve source tool rule IDs and report fix availability without applying fixes.

### Oxlint Adapter

`OxlintAdapter` is the first external adapter for JavaScript/TypeScript:
- Discovers `oxlint` via explicit config -> project-local `node_modules/.bin/oxlint` -> `PATH`
- Parses Oxlint JSON output into normalized `Diagnostic` values with `DiagnosticSource::ExternalTool`
- Maps Oxlint severity and categories to Tree Hugger equivalents
- Falls back to native diagnostics when Oxlint is unavailable
- In non-strict mode, missing tools produce empty results with metadata rather than errors

## Corpus and Default-On Gates (Phase 4)

The `corpus` module provides infrastructure for measuring rule precision against real-world code:

### Corpus Harness

- **`CorpusManifest`**: Defines repositories/fixtures with metadata (SHA, license, paths, rules, oracles)
- **`CorpusTier`**: `Smoke` (fast PR checks), `Expanded` (scheduled CI), `Benchmark` (performance timing)
- **`CorpusResult`**: Analysis output with per-rule threshold reports
- **`ThresholdReport`**: Tracks diagnostic counts against rule thresholds (`Zero`, `Budget(n)`, `Unlimited`)

### Snapshot Stabilization

- **`redact_snapshot_text()`**: Redacts absolute paths, temp dirs, tool locations
- **`normalize_diagnostics()`**: Sorts and deduplicates for stable comparison
- **`compact_diagnostic_text()`**: Produces compact stable representations
- **`strip_ansi_codes()`**: Removes terminal escape sequences

High-confidence syntax rules require `Threshold::Zero` corpus false positives. Experimental semantic rules use `Threshold::Unlimited` until resolver-backed precision is proven.

## Persistent Symbol Cache

`cache::SymbolCache` (`cache/symbol_cache.rs`) is the only cache `hug` uses
(`--no-cache` skips it); full design and the robustness table are in
[`docs/cache-design.md`](../../../tree-hugger/docs/cache-design.md).

- **One entry per file.** Key = `canonical_digest` (length-prefixed BLAKE3)
  of path (`/` separators), source xxHash, forced language or `detected`,
  and the records `PassFingerprint` (schema, `<crate>+rev.<EXTRACTOR_REVISION>`,
  grammar identity read from the `Language`, effective `locals`/`imports`/
  `references` text). Kinds and capabilities are **not** key inputs; the
  entry records `covered_kinds` (`"all"` or a non-empty ascending list) and
  `completed_capabilities`.
- **Diagnostics carry their own fingerprint inside the entry** (lint +
  comments text, rule metadata, `experimental_semantics`); a mismatch makes
  the diagnostics "not computed", never a key change.
- **Lookup**: `Hit` (no parse) / `WidenedCapabilities { parsed }` (docs,
  semantics-with-references and containment-over-all-kinds run on stored
  records; references and diagnostics re-parse) / `WidenedKinds` (one
  analysis for the union) / `Miss(CacheMiss)`. Every result goes through
  `analysis::coverage::project`, which narrows a wider entry to exactly the
  uncached result; a new pass that changes records must be reflected there
  (`symbols_cache::projected_results_*` fail otherwise).
- **Reader**: `deny_unknown_fields`, no defaults on entry fields, custom
  visitors for the coverage lists; failures are `CacheMiss::Malformed(serde
  message)` / `Mismatched` / `Inconsistent`, never an empty result. The entry
  and **every record** must carry exactly `SchemaVersion::V4_1` (minor
  included), else `Mismatched("record schema version")`; a nested
  version-bearing value must be checked in `StoredEntry::validate`. Extend
  `cli/tests/symbols/cache_robustness.rs` (entry and record columns) for a
  new load-bearing field, and assert `CacheLookup::Hit` (not just equal
  output) in any warm-run test: a silently undecodable entry still produces
  equal output.
- **Writes** are temp file + rename (`PersistentCache::put`); last rename
  wins.
- **One directory for every scan root**: `hug` opens
  `default_cache("hug")` (`CACHE_PROJECT_ID` in `cli/src/main.rs`), never a
  package-root partition, so a file reached from two scan roots shares one
  entry (`cache_lookup::a_run_from_another_scan_root_hits_the_same_entry_with_the_same_ids`).
  Nothing scan-root-dependent may enter a stored record or the key.
- `StoredEntry::validate` refuses (`Inconsistent`) a `metrics` facet or a
  `true` `is_test`/`is_deprecated` whose capability (`metrics`/`flags`) the
  entry does not list, and a record without `metrics` under `metrics`. Flags
  and metrics need the tree, so widening by them parses. Query text is
  injectable for tests (`SymbolCache::with_query_text`).
- `InProcessCache`, `InMemorySymbolCache`, `SymbolSnapshot`,
  `AnalyzerFingerprint` remain as library types but no CLI path uses them.

## Neovim Query Reuse (Phase 6)

### Query Provenance

Every query carries provenance metadata (`QueryProvenance`) tracking:
- **Source project**: e.g., `nvim-treesitter` for vendored queries, `tree-hugger` for originals
- **Upstream revision**: Tag or commit SHA when vendored
- **Original path** and **local path** for audit trails
- **License**: SPDX identifier (e.g., `Apache-2.0`)
- **Translation status**: `Unchanged`, `PredicatesAdapted`, `CapturesRenamed`, `MultipleChanges`, `Original`

Access via `tree_hugger::query_provenance(language, kind)` or `all_query_provenance()`.

Vendored `locals.scm` provenance is per language (`vendored_locals_changes` in `queries/provenance.rs`, an exhaustive match): a language whose query gained or lost patterns is `MultipleChanges` with one note per change, else `CapturesRenamed`. **Perl's `queries/vendor/perl/locals.scm` is a Tree Hugger `Original`** (repository license `AGPL-3.0-only`), not nvim-treesitter's, so drift checks skip it. When you edit a vendored query, update its notes in the same change; `phase6_neovim_query_reuse::every_locals_definition_capture_is_inventoried` fails if a new `@local.definition.*` capture is missing from the inventory.

### Nvim-treesitter Inventory

`QueryInventory` records capture groups from all upstream query suites:
- **Locals** (`locals.scm`): Reused by Tree Hugger for symbol definitions and references
- **Highlights** (`highlights.scm`): Inventoried but not yet reused
- **Injections** (`injections.scm`): Inventoried for future language-injection support
- **Folds** (`folds.scm`): Inventoried for future folding support
- **Indents** (`indents.scm`): Inventoried for future indentation support

`CaptureEntry` tracks whether a capture is reused and includes translation notes.

### Predicate and Directive Compatibility

`CompatibilityRegistry` documents how Neovim predicates/directives behave in the tree-sitter Rust runtime:
- **Native**: `eq?`, `match?`, `lua-match?`, `any-of?`, `contains?`, `any-contains?`, `set!`, `offset!`
- **RequiresHook**: `has-parent?`, `is?`, `has-ancestor?`, `not-has-ancestor?` — use named post-processing hooks
- **Unsupported**: `make-range!`, `strip!` — must not be silently dropped

Use `find_unsupported_predicates(query_text)` and `find_hook_predicates(query_text)` to audit queries.

### Upstream Drift Detection

`check_query_drift(language, kind, current_revision)` and `check_all_drift()` report when vendored queries diverge from upstream:
- Detects newer upstream revisions
- Flags content mismatches and capture divergences
- Only applies to vendored queries (not original overlays)

`summarize_drift()` produces human-readable reports.

## Known Limitations

- **Swift**: structs, classes and enums all captured as `SymbolKind::Type` (an extractor limit; the grammar's `declaration_kind` field could distinguish them)
- **Constructors**: explicit instance constructors only; Java compact record, C# static/primary constructors and destructors are absent, C++ out-of-class `C::C() {}` is a container-less `Function`
- **Go**: Struct and interface both captured as `SymbolKind::Type`
- **Go**: embedded fields and fields of anonymous struct types are not reported; grouped `X, Y int` fields share one span, so `hug` orders them by record ID
- **Scala**: `private enum E { … }` does not parse (grammar limit G1 in `docs/symbol-kinds-and-visibility.md#grammar-limits`)
- **Python / JavaScript**: instance attributes assigned only through `self.x` / `this.x` are not reported (the catch-all assignment patterns were removed); class-body declarations are
- **Perl**: narrow coverage: named subs, `package` (Module) and `my`/`our`/`state` names only; variable names keep their sigil (`$name`); `local` is never a declaration; scope lives in the declaration text (`my`/`our`/`state`), not in the level
