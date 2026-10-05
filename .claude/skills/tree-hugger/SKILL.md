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

- **Selective analysis** (`analysis/mod.rs`, `file/query.rs`, `file/source_file.rs`, `file/tree_file/definitions.rs`): one planner serves `symbol_index_with_passes`, `symbol_index_v2` (= `SymbolQuery::complete_index()`) and `query`; passes run once after prerequisites (parse → bind → semantic; parse → docs; parse → diagnostics) and `completed_passes` lists only what ran. `Capability::Containment` links parents/members without the reference query. `TreeFile::definitions(&MetadataSelection)` runs the locals query **once** per analysis; records, exports, references and semantic lint all take that result (`*_from` methods) — never call `symbols()`/`exported_symbols()` inside the pipeline. De-duplication happens before metadata: the winner supplies each field, later equal-priority captures only fill what it lacks (while no signature is known, a later capture's signature, visibility and `static` flags replace the current ones together). Unselected kinds stay light (name, kind, span, container) and are bound, then dropped; selecting a type kind also selects Field/Property/Constant. Binding is by name through `NameIndex` (first record of the name); containment is not: `find_container_symbol` takes the smallest enclosing same-named record, else the record of `container_kind` whose in-file qualified name qualifies the member's (nearest preceding on ties, Lua repeated `local M`), else the only one, else none — never the first of a repeated name (`symbols_owner_identity` is the row table); `RefSet` dedupes resolved refs by ID and unresolved ones by name + qualified name. `semantics.is_recursive` is always `Unknown` when the semantic pass runs. `work_counters()` (atomics) proves skipped work in tests; `symbols_query` is the L1 file to extend.
- **Predicates and the displayed prefix are post-lookup** (`file/query.rs`, `file/filter.rs`, `file/pattern.rs`, `file/naming.rs`): analysis and the cache produce *unfinished* results (in-file `qualified_name`, every record of the effective kinds); `SymbolQueryResult::finish` first applies the file-wide test rules (`is_test_file`: file names and the Rust package-`tests/` rule, which reads the nearest `Cargo.toml` and so can change while the file and its entry do not; records and entries keep only attribute/`cfg(test)` flags, `cache_package_boundaries` and `symbols_flags::cached_test_flags_follow_the_package_layout` pin it), then prefixes every name not `is_namespace_rooted` (scan-root-relative `file#`, default the file's own directory, or a `NamingContext` module path), then applies `RecordFilter`. Never store or key on a finished result. A predicate pushes only what it reads into the effective kinds/capabilities (`SymbolQuery::settle`). `SymbolRef::qualified_name` in relations/exports stays in-file. `symbols_selection_filters` (`SELECTION_FILTER_MATRIX`, every predicate per language, block or whole-file regions) and `symbols_query_filters` (pushdown, cache isolation, `narrowed`, naming context, non-ASCII) are the tests to extend; tests that key records by in-file name use `symbols_fixtures::in_file_name`.
- **Scope, qualified name and `async` are computed for every symbol** in `definitions.rs::materialize` (light records included). `extract/scope.rs::symbol_scope` walks out from `member_slot`: first function node / `is_local_scope_kind` → `Local`, first owner (`is_scope_owner_kind`: member owners minus bodies, plus `enum_specifier`/Go `struct_type`/`interface_type`, or any `is_unnamed_type_owner`) → `Member`, root → `Item`; bodies are skipped so `declaration_list` is decided by its parent (`mod_item`/namespace → keep walking; `impl_item`/`trait_item`/class → Member). Parameters and C++ template type params are `Local`; Go `method_declaration` and Lua `M.f`/`M:m`/`M.x` (`lua_table_of_name`) are `Member`. `extract/qualified_name.rs::qualified_names` runs once per file after materialize: a symbol's prefix is the nearest enclosing *naming* symbol (function/type/module/namespace/enum variant, by declaration or anchor node id, memoized), else a Rust `impl` self type's path as written (`rust_impl_prefix`: `impl right::T` → `right::T::m`; leading `super`/`crate`/`self` resolved; `impl_self_type_name` stays the bare container name), a TS `internal_module` or named unextracted function node (Java/C# constructor), else `ancestor_container`; it stops at `is_unnamed_type_owner`, and at the root takes the nearest preceding file-level `package`/`namespace` statement (`is_namespace_rooted`; consecutive Scala package clauses nest, `outer.inner`). Separators: `::` Rust/C++/Perl, PHP `\` after a namespace else `::`, Lua `:` before a method, `.` elsewhere; Perl package names are absolute. `stable_symbol_key` hashes the qualified name (ordinal per `(kind, qualified_name)`), so changing name construction changes IDs. `extract/modifiers.rs::is_async_declaration` reads the anonymous `async` token (Rust inside `function_modifiers`, C# inside `modifier`, JS/TS also through the `value` of the name's binding). Matrices: `symbols_scope`, `symbols_qualified_names`, `symbols_containment`, `symbols_async` (coverage guard requires one row per language).
- **Docs, flags and metrics** (`extract/doc_comments.rs`, `attributes.rs`, `flags.rs`, `metrics.rs`): `extract_doc_comment` keeps only doc-form comments per language (`is_doc_comment_text`: Rust `///`/`/**`, Go any comment in the group touching the declaration, Java/JS/TS/PHP/Scala `/**`, C#/Swift `///`/`/**`, C/C++ Doxygen forms), skips ordinary ones, climbs `export`/JS variable statement/`template` wrappers, reads Python's body docstring (`python_docstring`: a nonempty `str` constant, i.e. ordinary literals and their adjacent concatenation, prefix read from `string_start`; bytes and formatted literals are not docstrings; source never evaluated), and is `None` for Lua/Perl/Bash/Zsh; a context-less non-`Local` name is documented before its declaration, a C/C++ context via `declaration_for_context`. `declaration_attributes` reads full name + top-level arguments from the tree (`token_text`: comments skipped, so `#[cfg(/*c*/ test)]` is `cfg` + `["test"]`; a comma inside a string or nested list never splits; Rust outer attrs, C++ `[[…]]`, Java/Scala annotations, C# lists, Python decorators through parentheses, Swift attributes) and fills nothing in the `attributes` facet; never split attribute or doc text with string operations. Go block docs keep interior blank lines (`clean_block_comment`) for the `Deprecated:` paragraph rule; doc-tag lines inside a ``` fence are not tags. `is_deprecated`/`has_test_attribute`/`is_in_rust_cfg_test` (in `records_from`) and `is_test_file` (only in `finish`, never in a stored record) implement the spec tables exactly (no prefix or substring matches; `cfg(any(test,…))` is not test). `parameters` = own list (all curried Scala lists) minus Rust `self` (`declared_parameters` returns `None` when no list exists and names a pattern-bound parameter by its pattern text, Rust `(a, b)`/`_`, JS/TS `{x}`/`[a,b]`, so destructured parameters count and keep their type for `--param-type`; a TS `this: T` is skipped; `declared_parameter_count` counts unnamed C/C++/Go parameters, `(void)` = 0; `symbols_signatures::patterned_parameters_keep_their_pattern_and_type_text` is the row table to extend). Matrices: `symbols_flags` (`FLAGS_MATRIX`, run on a temp copy because the in-place Rust fixture sits under `tests/`), `symbols_metrics` (`METRICS_MATRIX`), `symbols_kind_coverage` (`unsupported` vs `landed` `FEAT` cells, `no_construct` kinds).
- **Go receiver methods and Lua table members have containers**: `find_symbol_container` returns the receiver base type (`go_receiver_type_name`, through pointer and generics) as `SameFileType` for a Go `method_declaration` only (its receiver/params/locals stay container-less); `materialize` gives a Lua `M.f`/`M:m`/`M.x` the table text with `ContainerLookup::SameFileTable` (kind = unanimous kind of same-named `Item`-scope symbols).
- **Readers get a `DeclarationContext`** (`declaration.rs`): language, source, final kind, `scope` (`is_local()` = `Local`; replaced `is_local_declaration`), the captured `name` node (one declaration can name several things: `int a, *b;`, Swift `case east, west`), the file `root` (same-file lookups: Rust trait level, earlier C/C++ `static` prototype, Scala `package` segments) and `is_ecma_module` (computed once per JS/TS file by `is_ecma_module_file`). Add a field here rather than re-scanning the file per symbol.
- **The C/C++ earlier-`static` scan matches only the same function** (`visibility/c.rs`). `declared_functions` (`c_declarators.rs`) takes the `function_declarator` nearest the name with only parentheses between, so a returned function-pointer, function-reference or array-pointer type is never the function; an unqualified C++ declaration has exactly its enclosing namespace; a qualified one resolves its first qualifier against the file's `namespace` blocks, nearest enclosing first; a function template (under `template<…>` or abbreviated with an `auto` parameter) also compares its header, leading and trailing return type, and the `requires` clause after the header and after the parameters (`TemplateSignature`; never `noexcept`). Whatever cannot be established is a miss, never a match. `symbols_visibility::earlier_static_linkage_matrix_holds_for_c_and_cpp` is the row table to extend.
- **C/C++ signatures read the named function's own declarator** (`signatures.rs`). `extract_signature` gets the captured name; `declared_function_named` (`c_declarators.rs`, shared with linkage) picks that function's `parameter_list`, so a pointer/reference/array return keeps its parameters and a returned callback's never replace them. Each parameter (`parameter_declaration`, `optional_parameter_declaration` with its default, `variadic_parameter_declaration` pack, C `variadic_parameter` and the C++ `...` token) is named by `declarator_name`, which follows a function declarator only through its `declarator` field. `symbols_signatures::c_and_cpp_signatures_read_the_named_functions_own_parameters` is the row table to extend.
- **C++ const-object linkage reads the declarator nearest the name** (`is_const_object`, `visibility/c.rs`). It reuses `declarator_layers` (parentheses and attributes add no layer; arrays are skipped to their element), so `char **const p` is const and `char *const *p` is not; a reference is checked before the `constexpr` shortcut and is never const. Any new reader of a declarator's type goes through `declarator_layers`, not the outermost node. `symbols_visibility::const_object_linkage_matrix_reads_the_outermost_declarator` is the table to extend.
- **Membership starts from `member_slot`** (`classify.rs`): the declaration or its outermost transparent wrapper (`is_transparent_wrapper_kind`: C++ `template_declaration`, C/C++/C# `preproc_*` conditionals, Python `decorated_definition`, Scala `extension_definition`). Kind, both `static` readers, C++ and C# access (owner read from `member_slot(..).parent()`) and C/C++/C# field, interface-property and enum-member lists (`flattened_members` in `type_metadata.rs`) all look through those wrappers; add a new wrapper there, not in one reader, and route any new reader of a declaration's parent or a type body's children through `member_slot`/`flattened_members`. Swift `#if` and C# `#region` are flat `directive`/`preproc_region` siblings, not wrappers.
- **Member kinds are validated, not trusted.** `refine_symbol_kind` demotes any Method/Constructor (to Function) or Field/Property (to Variable), whether from a query capture or a classifier rule, when `is_function_local_member` finds a function before any type or object owner above the member slot (C# local functions are captured as methods by the vendored query). Lua `M:m`/`M.f` names its owner and is exempt. `symbol_scope` (which decides `DeclarationContext::is_local()`) also stops at the nearest owner, so members of a type declared inside a function keep their access; a JS/TS `constructor` is a Constructor only directly in a `class_body`.
- **Unnamed owners end the container search.** `find_symbol_container` returns no container at `is_unnamed_type_owner` (anonymous classes, Scala `given … with` instances, JS/TS class expressions and object literals, tagless C/C++ `struct`/`class`/`union` that is not an anonymous member) instead of climbing to the named type outside.
- **Implicitly static members are static.** A member the language makes static without the keyword sets `is_static` at extraction: Java interface constants, C# `const` fields, Rust associated `const`s (impl and trait), PHP member `const`s (`const_declaration` is captured as Constant; a top-level `const` is not a member). C++ `static constexpr` says `static`; Swift/TS/JS/Scala/Python have no keyword-less case beyond Scala `object` members and Python class attributes.
- **Static readers match nodes, not text, and stop at the nearest owner.** A member-`static` reader matches the declaration's own keyword/modifier token or decorator expression (Python: `staticmethod`/`classmethod`, bare or `builtins.`-qualified, and a `ClassVar` annotation head under any qualifier; never a substring or prefix) and asks only the type that directly owns the member (Scala: the member slot's `template_body` belongs to an `object_definition`). A new reader must not `.contains()` node text or climb past a class, trait, enum or anonymous class.
- **Field lists are completed from the standalone members.** `align_field_metadata` (`tree_file.rs`) pairs each type-metadata entry with the standalone `Field`/`Property`/`Constant` of the same name whose container node is that type (k-th entry with k-th symbol of a name), copies its `visibility`/`is_static`, **adds** any standalone member no reader listed (doc comment, no type) and orders the list by source position, so `hug classes` and `hug symbols` list the same members. Readers still matter: they supply type annotations and the entries with no standalone symbol (record components, Rust tuple positions, Go fields, Scala class parameters). Enum owners are not completed, and Rust `impl` members (by-name container) never join the struct's list. `symbols_field_lists` pins complete per-language lists on both paths; a new member form needs a query capture **and** a reader case.
- **Type metadata lists only fields and properties.** Interface, protocol and trait methods are standalone `Method`s, never `FieldInfo` entries (Java interface constants, C# interface properties, Swift protocol property requirements and Rust trait associated constants are listed). Go interface method elements are the one exception left (Go fields are deferred).
- **C/C++ names sit under any declarator chain.** The C/C++ `locals.scm` capture a name by its immediate parent (pointer, array, parenthesized, attributed declarator; C++ adds reference and, for member functions, `(function_declarator declarator: (field_identifier))` under any return declarators), never by a fixed outer shape. `classify_symbol_kind` makes a captured variable/field a Function (Method in a type body) when `declared_function_named` says its declarator declares one (`int f [[deprecated]] (int)`, `int (f)(int)`), so a function-pointer field stays a Field; `implied_c_function_context` (`declaration.rs`) supplies the context such a capture lacks. `c_field_name` (`type_metadata.rs`) is `declarator_name` minus function declarators. Typedef names behind a declarator are TypeAliases with the typedef's doc comment and no field list. `symbols_declarator_wrappers` is the row table to extend. Anonymous `struct`/`union` members (`anonymous_member_declaration`, `classify.rs`) belong to the enclosing type, with the C++ access of the section holding them.

## Key Architecture

- **Runtime**: Built on the tree-sitter Rust runtime (`tree-sitter = "0.26.3"`) plus one `tree-sitter-<lang>` grammar crate per language, all at independent versions — see [Tree-sitter Internals](./tree-sitter-internals.md)
- **Query vendoring**: Uses nvim-treesitter `locals.scm` in `lib/queries/vendor/<lang>/`
- **Custom queries**: `lint.scm`, `references.scm`, `comments.scm` per language
- **Capture naming**: `@local.definition.<kind>` for symbols, `@diagnostic.<rule-id>` for lint
- **Query caching**: Global `OnceLock<QueryCache>` for thread-safe caching
- **Builtin database**: Per-language builtin lists to avoid false positive undefined-symbol/undefined-module errors
- **Schema v2 module, schema version 4.0**: `shared/schema_v2` with facet-based `SymbolRecord` and staged `FileSymbolIndex`; records, file indexes, CLI envelopes and analyzer fingerprints emit `SchemaVersion::V4_0` (the module keeps its historical `v2` name). Records carry no timestamps, so output and cache entries are byte-identical across runs; the container is `identity.container_name`
- **CLI declaration reports** (`cli/src/`: `declarations.rs` drives, `selection.rs`, `order.rs`, `report.rs`, `render.rs`, `classes.rs`, `scan.rs`, `prelude.rs`; `main.rs` only wires): `Declarations::query(format, &Selection)` is each command's fixed kind set ∩ `--kind`, the selectors as `SymbolQuery` predicates, and only the capabilities it shows (`classes` and `--tree` add containment; `--json`/`--jsonl` add `Flags` and `--stats` adds `Metrics`, all in `ReportFormat::capabilities`; no listing runs references, semantics, docs or diagnostics); `--json-full` asks `complete_index()` (every capability, flags and metrics included). `scan::in_file_order` runs rayon chunks of `4 × threads` and delivers results in file order, so `--json` is byte-identical across thread counts and cold/warm. The JSON envelope is written piece by piece with `errors` always last; records are projected (`report::trim_record`: scan-root-relative `file_path` and stable-key file segment, no `declaration_text`/source text/attached comments; IDs untouched). `tree_hugger::portable_relative` (`lib/src/file/naming.rs`, re-exported by `report.rs`) is the one path projection (entries, `errors[].file`, records, the qualified-name prefix; the CLI query passes `scan_root`): files outside the root get `../` routes via `biscuit_file::PathIdentity`, retried with directories canonicalized (macOS `/var` vs `/private/var`), and keep their absolute portable path only when no route exists (another Windows drive/share); the key's file segment is parsed from the key itself (`{lang}::{file}::{Kind:?}::`), never matched against `source.file_path`, because prelude aliases are keyed by the prelude file but located at the target. The one renderer (`render::Renderer`, built from `RenderOptions`) is `Prose` items in `UnorderedList`s (and a `Table` for `--format table`); it wraps only when `RenderOptions::width` is set (`main::wrap_width`: stdout is a TTY or `CLICOLOR_FORCE`, width from a positive `COLUMNS` else detection; `--plain`/`--format plain`/redirected output never wrap), and every source-derived string goes through `render::literal` (control chars → `\u{..}`, then `Prose::escape_text`, which on its own passes ANSI CSI/OSC through) or `render::neutralized` for table text cells; labels come from `identity.keyword` only for Type/Variable/Field/Constant, else the kind. Terminal output streams: `declarations::write_terminal` flushes after each `in_file_order` chunk (so a closed pipe stops scheduling within a chunk), `classes` writes per file, and `render::ListingStream` holds only the current unit (one file; one directory under `--group-by-module`; one package under flat `--sort name|kind` or `--prelude`) after `render::listing_order` reorders files by package, directory, path. Package sections are decided up front from `listing_order`'s package count (two or more packages in the scan, declarations or not), so nothing is held beyond the current unit; never reintroduce a layout decision that waits on later declarations, or a whole-scan accumulator ahead of the renderer. `write_json` flushes at the same chunk ends (`scan::flush_at_chunk_end`), so a closed pipe is noticed even when no entry fills the buffer. `--prelude` analyzes the prelude and its targets with an every-kind query first and the scan reuses them via `SymbolQueryResult::narrowed` (one parse per file; the reuse map, `collect_files` output, `SourceFile::path()` and the CLI roots are all `file_identity` spellings, so relative/`..`/symlinked inputs never split reuse or the cache); prelude exports are synthetic records (`visibility.is_synthetic`/`is_reexported`, own ID from the prelude file). Every output branch that writes declaration records (`--json` files, `--json-full` `index.symbols`, terminal listings, `classes` containers and member sections) orders through `order::compare`/`sort_file_records`: extraction order is not source order (C emits a typedef's inner `struct` specifier before the alias, Go a receiver parameter before its method, and Go grouped `type (…)` specs share one span, so only the ID breaks the tie). A failed file is stderr + `errors` + exit 1; a broken pipe exits 0 quietly. User-facing contract: `tree-hugger/docs/symbol-reports.md` (`order.rs` points at it); update it with any change to kind sets, envelope, order or `--verbose`.
- **Output modes** (`cli/src/output.rs`, `stats.rs`): `output::resolve(GivenOutput)` is the one place the "exactly one mode" rule lives (`--stats` composes with `--json` or a `--format`; `--sort`/`--signatures`/`--tree` reject `--stats`; `--tree` is terminal-only and not on `classes`); `main` maps the `OutputMode` to `declarations::ReportFormat`. `--sort` is `Option<SortOrder>` so "given" is detectable. `declarations::selected_records` is the one projection the JSON formats and the table share (classes: containers sorted, `displayed_members` trimmed); `write_json_lines`, `write_table`, `write_tree` stream per file in path order (not `listing_order`), `write_tally` holds only counters (`classes` counts containers). `selected_records` trims records (`trim_record`), so their `file_path` is scan-root-relative: anything that turns one into a file (the table's OSC 8 Location link) must join it onto `scan_root`, never resolve it against the working directory (`reports::table_location_links_resolve_against_the_scan_root`). `render::Outline` nests by `relations.container` ID, falling back to `identity.container_name` (a type declared in another file) for a marked `[in …]` heading that is never a record; it reattaches anything a containment cycle would orphan. `stats::Stats` keys are the records' serde spellings (`Function`, `Public`, `Rust`); synthetic exports count in the maps and apart from doc coverage. Tests: `cli/tests/symbols/reports.rs`, goldens `symbols.{table,tree,stats,jsonl}`, `symbols-stats.json`, `symbols-signatures-full.plain` (`--format plain` shares `symbols.plain`); any test forcing color must pin `COLUMNS` (forced color wraps at the detected width).
- **CLI filters**: `hug <subcommand> <filter...>` with all-files default scan for symbol commands. A pathless scan honors `.gitignore` and skips test-data dirs by default (`**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**`); an explicit path/glob re-includes a targeted file. `lint` scans hide clean files (single-file lint still shows `(no diagnostics)`).
- **Record selectors are one path** (`cli/src/selection.rs`): `SelectionArgs` (flattened into the declaration and `classes` args; `TargetArgs` = `--file`/`--name`/`--exact`, also on `imports`) resolves to a `Selection`, which becomes library predicates; the CLI keeps no name matcher of its own (positional names and `--exclude-symbols` are `NamePattern`s; `normalize_symbol_glob`/`matches_symbol_filters` are gone). Records are filtered by the library after the cache (`SymbolQueryResult::finish`); only `--prelude` is applied in `declarations::prepare` (its synthetic exports run `query.selects`/`query.matches` there). `symbols` defaults to `Item`+`Member` scopes (`SYMBOLS_DEFAULT_SCOPES`); other commands every scope. `classes` uses `classes::ClassSelection`: an analysis query with no predicates (container kinds ∪ member kinds, capabilities of both filters), identity predicates + `--kind` on containers, `--in` as the container's own name, shape predicates on members (fields hidden when one is given); `--static-only` conflicts with `--instance-only`. `imports` filters `index.imports` with `ImportSelection` (name, alias, original's last segment, or the whole original for a qualified pattern); positional tokens stay files. `--file` values: globs and unresolved values join the walker inputs, a `FileReference` that resolves to a file is an explicit target (`scanner::collect_selected_files`: bypasses `.gitignore`/fixture exclusions, not `--exclude-files`; never widens the scan). A bare `rs` is a name, not a file filter (`is_file_filter_token` needs a path, glob or `x.rs`). `--json-full` rejects every record selector (`SelectionArgs::given_flags`). Tests: `cli/tests/symbols/filters.rs`; tests that need locals from `symbols` pass `--all-scopes`.

## Detailed Documentation

- [API Reference](./api-reference.md) - Complete type definitions and methods
- [Symbol Reports](../../../tree-hugger/docs/symbol-reports.md) - The declaration commands end to end: kind sets, query → passes → render flow, JSON envelope and `errors`, `--json-full`, `--sort` and tie-breakers, `--verbose`, exit status
- [Symbol Kinds, Visibility and `static`](../../../tree-hugger/docs/symbol-kinds-and-visibility.md) - Per-language kinds, containers, visibility levels and defaults, the two meanings of `static`, declaration text, current limitations
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
   8 and 9 so far; the current revision is 10). `cache_robustness` pins which malformed
   shapes of the entry fields and twenty-two record columns are served or
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
  and **every record** must carry exactly `SchemaVersion::V4_0` (minor
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
- **Constructors**: only TypeScript/JavaScript report `Constructor`; elsewhere they are `Method` or absent
- **Go**: Struct and interface both captured as `SymbolKind::Type`
- **Go**: `type A = B` aliases are not reported at all
- **Python / JavaScript**: instance attributes assigned only through `self.x` / `this.x` are not reported (the catch-all assignment patterns were removed); class-body declarations are
- **Perl**: narrow coverage: named subs, `package` (Module) and `my`/`our`/`state` names only; variable names keep their sigil (`$name`); `local` is never a declaration; scope lives in the declaration text (`my`/`our`/`state`), not in the level
