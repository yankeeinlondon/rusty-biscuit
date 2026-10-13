# Analysis Pipeline

How a `SymbolQuery` becomes records: the schema, the selective planner,
scanner identity, the parent index, reference occurrences and
post-lookup predicates.

## Schema

- **Schema v2 module, schema version 5.2**: `shared/schema_v2` with facet-based
  `SymbolRecord` and staged `FileSymbolIndex`; records, file indexes, CLI
  envelopes and analyzer fingerprints emit `SchemaVersion::CURRENT` (the module
  keeps its historical `v2` name). Records carry no timestamps, so output and
  cache entries are byte-identical across runs; the container is
  `identity.container_name`

## Pipeline

- **Selective analysis** (`analysis/mod.rs`, `file/query.rs`,
  `file/source_file.rs`, `file/tree_file/definitions.rs`): one planner serves
  `symbol_index_with_passes`, `symbol_index_v2` (=
  `SymbolQuery::complete_index()`) and `query`; passes run once after
  prerequisites (parse → bind → semantic; parse → docs; parse → diagnostics) and
  `completed_passes` lists only what ran. `Capability::Containment` links
  parents/members without the reference query.
  `TreeFile::definitions(&MetadataSelection)` runs the locals query **once** per
  analysis; records, exports, references and semantic lint all take that result
  (`*_from` methods) — never call `symbols()`/`exported_symbols()` inside the
  pipeline. De-duplication happens before metadata: the winner supplies each
  field, later equal-priority captures only fill what it lacks (while no
  signature is known, a later capture's signature, visibility and `static` flags
  replace the current ones together). Unselected kinds stay light (name, kind,
  span, container) and are bound, then dropped; selecting a type kind widens
  `MetadataSelection::wants` to Field/Property/Constant (and an enum to
  EnumVariant) for the access, `static` flags and doc its lists copy, but
  `records_from` gives header texts, attributes, modifiers, flags and metrics
  only to kinds `MetadataSelection::selects` (the kinds asked for). Reference
  edges come from the occurrences (`bind_references`: only `Scoped` occurrences
  with an owner; an in-file target ID → `references` + the target's
  `referenced_by`; a bound import (`ResolvedOccurrences::bound_imports`, aligned
  with the sorted occurrences) → `references` + `dependencies`;
  `NameOnly`/`Unresolved`/ownerless make no edge, so limited languages have no
  edges); `NameIndex` serves containment only: `resolve_containers`
  (`smallest_enclosing`, then `find_container_by_identity`) takes the smallest
  enclosing same-named record, else the record of `container_kind` whose in-file
  qualified name qualifies the member's (nearest preceding on ties, Lua repeated
  `local M`), else the only one whose qualified name agrees with the member's
  owner path (`agrees_with_owner_path`: a sole local `W` never owns
  `other::W::probe`; nearest-preceding among equal qualified names only where
  one owner repeats, `repeats_one_owner`: Lua tables, C++ forward declarations;
  two Go types of one name own nothing; Rust impl and Swift extension members
  never reach this rule under Containment: `link_relations` pre-links them from
  the impl's resolved self type / the extension's `extension_owners` decision,
  textual container with no ID when unproved; `resolve_container_kind` applies
  the same rule to `SameFileType`/`SameFileTable` kinds, except a Rust impl
  member, whose kind is its impl's resolved self type from
  `relations::impl_self_types` and absent when unresolved), else none — never
  the first of a repeated name (`symbols_owner_identity` is the row table,
  `ABSENT_OWNER_MATRIX` the absent-owner rows, `NESTED_SHADOW_MATRIX` the Swift
  closure-shadow rows, `UNPROVED_OWNER_MATRIX` the Swift unproved-owner rows
  (duplicate/alias/import/external owners: no conformance, textual container, no
  membership), both also fed into `declaration_fact_sources`); `RefSet` dedupes
  resolved refs by ID and unresolved ones by name + qualified name.
  `semantic_pass` (functions, methods, constructors) reads `is_recursive` from
  `index.occurrences`/`reference_coverage`/`imports` only, with each
  occurrence's `import_span` selecting exactly its bound import (never
  owner/file-wide name membership), so a cache widening from stored occurrences
  equals a fresh run (`direct_recursion`, `may_call_itself`).
  `Capability::Occurrences` (between References and Semantics; Semantics
  requires it) keeps the evidence; References alone resolves it and drops it in
  `run_query`; `coverage::record_passes_suffice` widens Semantics without a
  parse only from stored Occurrences. `work_counters()` (atomics, incl.
  `reference_captures`, `occurrence_resolutions`) proves skipped work in tests;
  `symbols_query` is the L1 file to extend, `symbols_reference_summaries` for
  edges, recursion (`RECURSION_MATRIX`) and whole-file evidence.
- **Scanner identity and literal paths** (`scanner.rs`, `file/source_file.rs`):
  `collect_selected_files` returns `file_identity` values, so the CLI's
  `Analyzer::read` and god-files use `SourceFile::read_identified` /
  crate-private `from_identity` (beside `from_read`) and never resolve again;
  public `read`/`read_with_language` still canonicalize arbitrary spellings.
  When every walked pattern is a literal anchored path (`literal_targets`: has
  `/`, no `*?[]{}\:`, no leading `!#/`, no trailing `/` or whitespace, no
  empty/`.`/`..` component), the walker gets a `filter_entry` that only prunes
  directories no target lies under; `ignore` still decides every entry it
  reaches (override precedence, nested `.ignore`, fixtures, hidden, symlinks),
  so selection is identical. The one difference: unrelated unreadable
  directories are never opened, so they no longer fail the scan. Tests compare
  each literal with the same pattern bracketed (`src/lib.r[s]`, forces the
  walk): `filters::literal_positional_paths_select_what_the_walker_selects`.
- **Re-exports** (`analysis/reexports/`, types in
  `shared/schema_v2/reexports.rs`): `Capability::Reexports` (prerequisite
  `Declarations`; `Diagnostics` requires it) makes `run_indexed` call
  `reexports::flag_forwarded_imports` once, right after the imports are read and
  before diagnostics: it runs `extract`, then sets `ImportSymbol::is_reexport`
  (and so `ImportRecord::reexported`) by exact import span for
  `syntax`/`declared` entries only. It is the only place the flag is decided:
  `imported_symbols()`, `lint_diagnostics()` and the CLI's index conversion all
  use it or the stored marker, never a Rust `visibility_modifier` check. `FileSymbolIndex::reexports` is `None`
  exactly when the capability was not computed (key omitted); `coverage::project`
  drops the report and clears the markers together, and
  `record_passes_suffice` never widens it without a parse. `extract`
  (`reexports/mod.rs`) matches every language: Java/C/Bash/Zsh/PHP are
  `not_applicable`, every other language has its own module
  (`rust`, `ecma` for JS+TS, `python`, `go`, `perl`, `lua`, `swift`, `scala`,
  `cpp`, `csharp`) taking `(&TreeFile, &[ImportRecord])`; `reexports/common.rs`
  holds `entry`, `limit`, `finish` (source order, one entry per span, sorted
  limits, state derived) and `damaged_nodes`. Extractors read only the
  analyzed file; an ERROR/MISSING node limits coverage only when it touches a
  forwarding trigger, and clean parses are not proof (C++ module files and
  Perl `@Pkg::EXPORT_OK` parse silently wrong, so they are limited by
  structure). Same-file binding links follow replacement and scope, never
  spelling alone: Python's `later_import_replacement`/`statement_relation`
  drop an import a later import of the same name always replaces (sibling
  `try`/`if`/`match` arms are alternatives; a conditional later import is a
  `rebound_binding` limit), and Scala's `link_import` takes the nearest
  preceding import in the same or an enclosing scope unless a definition or
  class parameter between them hides it. Python `__all__` edits fold into
  `declared_all_names` only through `apply_all`, which turns a `conditional`
  edit into `computed_export_list`; append/remove/extend/`=`/`+=` all route
  there, and `nested_call`/`note_deferred_all_edits` limit `__all__` method
  calls inside expressions or function/class/lambda/comprehension bodies
  (and `global __all__`). ECMAScript CommonJS sites resolve
  through `Table::resolve_at` (module-scope `resolve_any` unless
  `bindings::shadowed_below_program` finds a nearer declaration: block
  lexicals, params, hoisted function/namespace `var`s, `catch`, `for`
  heads, `switch` bodies, class-expression names); ES export sites in
  namespaces use the same walk. `Table::add_writes` also counts `for (x of
  ..)` targets, block `var` initializers that hoist to module scope and
  function declarations in module-level blocks (Annex B, skipped for ES
  import names); `Table::build` counts a name two module-scope `var`
  declarators *initialize* (`initialized_names`; a bare `var m;` is not a
  write); a write to `require`/`module`/`exports`
  (`global_replaced`) is `shadowed_global` for every dependent site,
  including `Binding::loader` bindings (`require` and TS
  `import x = require`). After `cjs::scan`, `cjs::Interface` (every
  `module.exports`/`exports` assignment, augmented write, update, `delete`,
  `Object.assign` merge and TS `export =`, numbered by an *epoch* of
  definite `module.exports =` writes) `settle`s each `declared` entry by the
  innermost write around it: a later definite same-member or whole write
  drops it (an `exports.` write after a definite whole write is detached and
  dropped); a guarded, function-held, augmented, computed or merge write, or
  a member write onto a forwarded whole module, withholds it as
  `rebound_binding`. Lua (`reexports/lua.rs`) ties every name to the
  declaration `resolve` finds at its site (`Resolved::Chunk(statement)`,
  `Nested` for a nearer local/`local function`/parameter/loop variable/
  `repeat` local, `Global`); `collect_writes` keys chunk-level writes
  (assignments and `function x() end`) by declaration id and name, `Scan::bound`
  matches the env by that declaration, and a `require` call derives only
  where `require` resolves to the global. For a returned table `M`,
  `table_uses` classifies every reference resolving to its declaration:
  a write inside a block or function body withholds that field, and a
  write under a `Key::Unknown` key withholds every field (`rebound_binding`
  at the statement); a call argument, method receiver (`M:f()`) or stored
  value (`local a = M`, `{M}`, `M or x`, a nested `return M`) withholds
  every field (`dynamic_forwarding` at the call or stored expression).
  Reads, chunk-level named-key writes, number/boolean-key writes and the
  final `return M` pass. "Chunk-level named-key write" means exactly what
  `field_target` folds: the table is `table_identifier` (the identifier
  after `unparenthesized`), so `(M).x = v` folds and `(M or t).x = v` is a
  `rebound_binding` write withholding `x`. Every table, callee
  (`callee`/`call_name`), `require` argument, declared constructor and
  metatable argument goes through `unparenthesized`, so the classifier and
  the collector cannot disagree on a parenthesized spelling. A field
  written twice in one assignment statement with an import-derived value
  is `TableField::undecided` (withheld, `rebound_binding` at that
  statement when its table is returned; a later write clears it). Every key (constructor `field_key`, assignment
  and member-read `index_key`) goes through `table_key`: an identifier or
  a verbatim string literal is `Key::Name` (so `x`, `["x"]`, `M.x` and
  `M["x"]` share one `upsert` identity), a number/boolean is
  `Key::NonString` (skipped), anything else, including a string with an
  escape sequence, a long bracket starting with a newline, or `""`, is
  `Key::Unknown`. An unknown constructor key, or a repeated constructor
  key where either value is import-derived (Lua leaves constructor order
  undefined for repeats), makes `constructor_fields` return `Err(field)`, reported as `rebound_binding` only when that table
  is returned. Tests: `tests/l1/symbols_reexports_<lang>.rs` plus the
  16-language guard in `symbols_reexports.rs`. `work_counters().reexport_extractions` proves
  one extraction per analysis and none without the capability. The cache entry
  writes a required, nullable `reexports` key and `imports[].reexported`;
  `StoredEntry::validate` refuses a report without the capability (or the
  reverse) and a marker without it, and `cache/stored_reexports.rs`
  (`validate_reexports`) refuses state/limits/language disagreement, spans
  outside the file, out-of-order entries or limits, form-vs-name
  contradictions, an `import_span` that is no import, and marker-vs-entries
  disagreement (the import inventory digest covers each marker, so it fires
  first on a flipped marker).
- **Parent navigation goes through the parent index** (`file/parents.rs`):
  `run_query` installs one per file (`TreeFile::with_parent_index`, one
  `TreeCursor` pass); inside it `node.indexed_parent()` / `parents::parent` /
  `scope::ancestors` are one map read, and anywhere else they fall back to
  `Node::parent()` (same answer). Use `indexed_parent()`, never `.parent()`, on
  a `tree_sitter::Node` in library code: `Node::parent()` descends from the
  root, so a climb is quadratic in depth (a generated 400 KB Rust file took 536
  s). The module holds the crate's only `unsafe` (one `Node::from_raw`, tied to
  the installed tree).
- **Rust facts built once per file**: the definitions pass's pattern placement
  (`occurrences::RustPatterns`: walk, scope tree, placed sites) is finished by
  the bind pass (`OccurrenceSyntax::resolve` retargets the sites to records),
  and the relation walk `impl_self_types` builds (`relations::LexicalWalk`,
  keyed by entry node ids and kinds) is reused by `link_relations`. Both travel
  on `Definitions` (`rust_patterns`, `rust_walk`). `Walk::run` also records the
  nodes inside an excluded context and the TS/JS export names, so occurrences
  never climb for them. `link_relations` returns before building `Declarations`
  outside Rust/Swift when no selected record writes a base.
- **Reference occurrences** (`analysis/occurrences.rs`,
  `analysis/scope/{mod,rust,ecma}.rs`, user contract
  `tree-hugger/docs/references.md`): `run_query` fills
  `FileSymbolIndex::{occurrences, reference_coverage}` (types in
  `shared/schema_v2/occurrences.rs`, `skip_serializing_if` empty) under
  `Capability::Occurrences`; `TreeFile::reference_evidence()` is an
  `Occurrences` query selecting no kinds. With References it runs
  `definitions_with_scopes` (the one locals run, also returning `@local.scope`
  nodes), `imported_symbols`, `reference_captures` (the one references run,
  shared with diagnostics through `references_from_captures`), builds
  `OccurrenceSyntax` from the definitions' nodes **before** `records_from`
  consumes them (records align 1:1 with definitions, so index = record), then
  `resolve(&index.symbols)` before unselected records are dropped. The cache
  entry stores both lists (required keys; non-empty only under `occurrences`).
  `LanguageRules::for_language` resolves only Rust and TS/JS (`Ecma { typed }`);
  other languages get `NameOnly`/`Unresolved` over every same-named record and
  every region `Limited(unsupported_language)`; their declaration tokens are
  excluded before candidate selection by
  `analysis/declaration_names.rs::declaration_names` (called from `Walk::run`
  per node for a rule-less language, `Walk::declarations` joins
  `Resolver::excluded_names`): one grammar-shape inventory per language (fields
  and node kinds, Scala's own varid rule for pattern binders), never dependent
  on which declarations became records, never adding records, candidates or
  bindings (the wildcard `_` is dropped in every rule set where it names
  nothing, Rust/Go/Swift/Scala, `occurrences::is_wildcard`; rows `R5-F2.S20`); a
  declared name written as a path (`DECLARED_PATH_KINDS`:
  package/module/namespace paths, a Swift whole-module import path) contributes
  every segment, and a C++ `qualified_identifier` type name only its last,
  through `declared_name_tokens`, which both this inventory (in `Resolver::new`)
  and every record name (`tree_file::definition_ranges`, so TS/JS and Rust too:
  a TS `namespace A.B` prefix is never bound to a class `A`) go through (rows
  `R7-F2.nn`/`.Sn`,
  `typescript_namespace_prefixes_never_bind_to_a_same_named_class`, CLI
  `refs::unreported_and_compound_declaration_names_are_never_references`); it
  also owns the Python/Scala pattern readers (`python_targets`,
  `python_captures`, `scala_pattern_names`) the relation tree's `lexical.rs`
  shares (`symbols_occurrences::DECLARATION_EXCLUSION_MATRIX`, rows
  `R5-F2.nn`/`.Sn`;
  `symbols_cache::unreported_declaration_tokens_stay_excluded_through_the_cache`;
  CLI `refs::unreported_declaration_tokens_are_never_suggestions`). Contextual
  syntax markers are excluded the same way, in the same walk, by
  `analysis/syntax_markers.rs::syntax_markers` (`Walk::markers`, also joined
  into `excluded_names`; not declarations, so not in that inventory): by grammar
  role, narrowed to the one spelling the language reserves in a slot that also
  holds names — Java bare `var` in the inferred-type slots (local, enhanced
  `for`, resource, explicitly typed lambda parameter, record pattern component;
  Java 10+ reserves the type name; a file declaring or ordinary single-type
  importing a type `var` is legacy source, so `SyntaxMarkers::nodes` keeps none
  of its `var` markers (`java_declared_type_name`; a `static` import names
  fields/methods too and is never evidence, nor is an on-demand import or a
  `var` qualifier, rows `R7-F1.nn`); legacy `var` types elsewhere stay reads),
  the Lua `attribute` identifier (`<const>`/`<close>`), PHP
  `self`/`static`/`parent` (case-insensitive) as a `named_type`, `new` or
  `instanceof` operand, C# `global` as an `alias_qualified_name` alias
  (`global::X`; an extern alias reads), Swift compiler-configuration identifiers
  (`swift_markers`: a `precedence_group_attribute`'s directive and an
  `associativity` value, `higherThan`/`lowerThan` values and an operator's group
  staying reads; a `deprecated_operator_declaration_body`;
  `availability_condition` platforms; a one-segment attribute name spelled as a
  Swift 5.10 built-in (`SWIFT_BUILTIN_ATTRIBUTES`, or `_`-prefixed;
  `Sendable`/`MainActor` stay reads), every argument of `SWIFT_MARKER_ARGUMENTS`
  (`available`/`objc`/`convention`/`inline`/`exclusivity`) and any attribute
  argument label (rows `R7-F1.S6`); `directive` (`#if`) operands),
  named-argument and tuple labels (`argument_label`: Python `keyword_argument`
  name, C# `argument` name and `attribute_argument` `x:` label, PHP `argument`
  name, Swift `value_argument_label`, Scala `assignment_expression` directly in
  `arguments` (parenthesized stays an assignment), Perl bareword before
  `fat_comma`, Go key in a `struct_type` composite literal only;
  `swift_element_labels`: tuple expression/type `name` fields and `x: pattern`
  labels; rows `R7-F1.S5.*`), and the leading `-`/`+` options of a shell
  `declaration_command` through `--`
  (`symbols_occurrences::MARKER_EXCLUSION_MATRIX`, rows `R6-F3.nn`/`.Sn`;
  `symbols_cache::contextual_syntax_markers_stay_excluded_through_the_cache`;
  CLI `refs::contextual_syntax_markers_are_never_suggestions`). The shell
  inventory takes a `declaration_command`'s operands after those options
  (`syntax_markers::shell_option_count`: `local x`, `typeset +x y`, `declare -f
  g`, zsh `integer i`; only `local x=1` is a record, so a bare operand is
  excluded without one; rows `R6-F3.X2.nn`, Bash and Zsh as parity). The
  diagnostics references (`TreeFile::referenced_symbol`) use neither inventory;
  both they and the rule-less occurrences share the older
  `is_definition_like_reference` filter, whose `is_named_definition_node` reads
  name fields only (never `value`: a bare initializer `int k = xs;` is a read)
  and which excludes a TS/JS parameter's pattern but not its `value` default
  (`symbols_occurrences::INITIALIZER_READ_MATRIX`, rows `R6-F3.X1.nn`, one per
  language; `tree_file::initializer_and_default_values_are_referenced_symbols`;
  CLI
  `refs::bare_initializers_are_suggestions_and_shell_declaration_operands_are_not`).
  One cursor walk (`Walk::run`) collects anonymous callables, owner-absent
  initializer regions (Rust `const`/`static` in a fn, TS field initializers and
  static blocks), coverage reasons, parse errors, extra scopes (`extra_scope`:
  Rust inline `mod` body, trait, generic items, `const`/`static`; TS generators,
  classes, signatures, `switch_body`, a `with` body), binding sites
  (`binding_sites`: the truth for bindings (TS statement/specifier anonymous
  `type` modifiers put each affected import in TYPE only, noncallable;
  default/namespace/require aliases follow the statement modifier, ordinary
  exported `type` names stay ordinary imports; value-category `typeof F` and
  local function F remain independent), including names the locals query never
  reports — `for`/`catch`/destructured/generic params and a TS/JS named
  function/generator/class expression's own name, which
  `ScopeTree::names_itself` keeps inside the expression; bound `Scoped` with no
  ID; every site name is excluded from the occurrences) Rust globs and `Dynamic`
  name sources (`LanguageRules::dynamic`: a `with` body, the function around a
  direct `eval` (`ecma::is_direct_eval`, shared with coverage: an ordinary call
  with an argument list whose `invoked_operand` is `eval`; an optional
  `eval?.()` is indirect (coverage `indirect_dispatch` only) and a tagged
  ``eval`…` `` evaluates nothing, so neither binds; `EVAL_CALL_FORMS` is the row
  table), never the root; marked as globs, also in the relation tree, whose
  `scope_role` makes a `with` body a lexical scope). Records the walk did not
  find bind by kind (`record_rule`). A Rust pattern name is never decided by
  spelling: `mut`/`ref`/`x @` bind; a bare one (`BindingSite::bare_pattern` with
  its `Refutability`, in every pattern position, struct field shorthands
  included, captured as `@reference.pattern`) is placed after every other
  binding by `bare_pattern_meaning` (VALUE lookup where it is written: a
  `pattern_constant` binding (`const`, unit struct, `const` generic) → a read, a
  usage; a function, local or nothing → binds; an import or glob first →
  `BindingTarget::Unknown` in a refutable pattern (the token and every use
  uncertain), a binding in an irrefutable one; `None`/`Self` read),
  `place_sites` (established sites, then bare ones in walk order) is shared with
  extraction: `definitions.rs::drop_non_declarations` drops every capture that
  declares nothing before records exist — false captures (`is_false_capture`:
  Rust `Some` in `Some(x)`, a TS/JS parameter default; it runs only for Rust and
  TS/JS, so every other language's locals query must capture binding fields
  only: `symbols_occurrences::PARAMETER_CAPTURE_MATRIX`, see [Query
  System](./query-system.md#capture-naming-convention)) and Rust pattern names
  that read or are unknown in a refutable pattern (`rust_pattern_non_bindings`,
  run only when a Rust variable/parameter was captured) — so records,
  diagnostics and counts agree with the occurrences
  (`symbols_occurrences::PATTERN_IDENTITY_MATRIX`, `PATTERN_RECORD_MATRIX`; CLI
  `refs::symbols_list_no_declaration_for_pattern_names_that_bind_nothing`). The
  locals query records no nested moded (`Some(mut x)`, `ref`, `@`) or bare
  match-arm name; those bind `Scoped` with no ID. Scala pattern sites in the
  relation tree are `varid`s (lower-case or `_` first; Scala's own rule) plus
  every `x @` binder name. `ScopeTree::lookup` climbs parent links through a
  `(scope, name)` table, filtering by category bits (`TYPE`/`VALUE`/`MACRO`) and
  `visible_from..visible_until` (`visible_until`: a Rust `if let`/let-chain
  binding ends with the consequence); Rust `let` is `ordered` (latest visible
  wins), a glob ends the search; an ECMA `var` (`Placement::FunctionScope`) goes
  to its function, or to the function's body block when the parameter list holds
  an expression (`ecma::var_body`, precomputed per scope in
  `ScopeTree::var_bodies`), so defaults never see body declarations
  (`symbols_occurrences::PARAMETER_DEFAULT_FORMS` × `BODY_DECLARATIONS`,
  `PARAMETER_ENVIRONMENT_MATRIX`; the relation tree shares it); leaving a scope
  applies its precomputed `Boundary` (Rust `boundary`: a non-associated item
  drops `BindingRule::local` bindings — `let`, params, patterns, generic params
  — an inline `mod` keeps only `MACRO`).
  `symbols_occurrences::LEXICAL_BOUNDARY_MATRIX` is the boundary row table. A
  written path (`target.qualified_name`, `written_path`) is the source from the
  outermost qualifier through the name token, climbed by grammar shape:
  `qualified_parent` (one step; field qualifiers in `QUALIFIER_FIELDS` that are
  not the node itself, a root `::` (`is_rooted`), field-less `SEGMENTED_KINDS`
  (Java/Scala/Swift type paths, PHP `X::K`), the PHP static property step
  (`variable_name` → `scoped_property_access_expression`, `X::$f`), the C#
  `member_binding_expression` → `conditional_access_expression` step, Swift
  `navigation_suffix`, a Java `object_creation_expression` whose type is the
  node and whose `construction_receiver` (the unfielded named child before
  `new`) exists: `outer.new C` keeps `new`; shared with `is_call_target`'s
  climb), `path_wrapper` (every `generic_operand`), C++ right-nested
  `right_nested_scope` (`B` in `A::B::f` → `A::B`), PHP
  `php_namespace_segment_start`, `qualifier_start` (Perl: no `&` sigil; Swift
  `navigation_expression`: `swift_chain_start` sets aside a leading prefix
  operator, which the grammar nests as `(!x).isEmpty()` but which applies to the
  whole chain, so `x.isEmpty`, down through
  `navigation_expression`/`call_expression` (calls and
  subscripts)/`postfix_expression`; an operator written as an implicit member
  navigation's own `target` token (`-.some`) gives that member no path (`None`)
  and later members `.some.f`; `tuple_expression` parentheses `(!x).f` stay;
  Python `attribute`: `python_chain_start` sets aside the unpacking star an
  assignment target's grammar nests as `(*x).s`, so `[*x.s] = r` is `x.s`); type
  args and a Java anonymous body after the name are outside it
  (`symbols_occurrences::WRITTEN_PATH_MATRIX`, one row per language, review rows
  `R4-F1.nn`, `R5-F3.nn`/`.Sn`, `R5-X.06`/`.10`;
  `java_construction_receivers_never_establish_a_target`; cache
  `symbols_cache::written_receiver_paths_hold_through_the_cache`; CLI
  `refs::receiver_qualified_constructions_report_their_written_target`). Any
  written path makes a supported-language name uncertain before lexical lookup,
  so a rooted Rust `::f` never binds a shadowing local and a rooted self-call is
  `Unknown` (`rooted_rust_names_never_bind_a_lexical_local`, `RECURSION_MATRIX`
  rooted rows). Unbound names: candidates are `Item`-scope records only. Members
  (`SymbolScope::Member`) are never lexical; `x.f()` is `NameOnly` over member
  functions + `indirect_dispatch`; Rust `Self::f`/`Type::f` is `Scoped` only for
  exactly one inherent-impl member (or enum variant) of one in-file type;
  calling a value member (`Self::C()` with `const C: fn()`) keeps that binding
  and adds `indirect_dispatch` (`holds_a_value`:
  Constant/Variable/Parameter/Field/Property/Unknown kinds; the initializer is
  never followed). Calling a binding whose rule is not `callable` (a variable),
  or an uncertain resolution (`Resolver::uncertain`) with any value candidate,
  adds `indirect_dispatch` (`symbols_occurrences::VALUE_CALL_MATRIX`). `kind` is
  syntax only (`is_call_target`: transparent wrappers (`wrapper_operand`, at any
  depth: parentheses, TS `as`/`satisfies`/`<T>x`/`x!`, the Swift force unwrap
  `postfix_expression` and C# null-forgiving `postfix_unary_expression` only
  when the operator is exactly `!`, never `++`/`--` or optional `cb?()`; Swift
  `cb!!()`/`cb?!()` misparse as custom infix operators, a pinned grammar limit;
  rows `R6-F1.nn`,
  `null_assertion_invocations_keep_their_paths_and_stay_conservative`, cache
  `null_assertion_invocation_kinds_hold_through_the_cache`, CLI
  `refs::null_assertion_invocations_are_callees_and_callers`), generic
  instantiations (one table, `generic_operand`: Rust/Scala `generic_function`,
  C++ `template_function`/`template_method`/`dependent_name`/`template_type`, C#
  `generic_name`, TS `instantiation_expression`, Java/Scala `generic_type`, a
  one-segment Swift `user_type` with type arguments, so `(f<number>)()`, `new
  (C<number>)()`, `new a.b.C<String>()` and Swift `C<Int>()` call their operand
  while `const h = f<number>` and an annotation `C<T>` stay usages),
  constructions (`new_expression` without a C++ array `declarator`, Swift
  `constructor_expression`, C++ `compound_literal_expression` `F{}`),
  `implied_operand` (Swift implicit member `.f()`/`.init()`, a Swift prefix
  operator the grammar writes before a call `!f()`, Scala mixin base `new C with
  T`; adds no written path), qualified paths (rooted `::f` and C# `obj?.F`
  included, `qualified_parent`), PHP variables (`$cb()`, `($cb)()`, `new $C()`:
  `php_variable`; PHP parentheses are transparent only around a variable),
  Perl/shell forms (Perl code references `$cb->()`, `&$cb()` in
  `is_perl_invocation`); PHP callable creation (`f(...)`, the grammar's
  `variadic_placeholder`, `is_php_callable_creation`; `f(...$args)` still
  calls), PHP `(f)()`/`($o->m)()`, Go `f[T]`, a Swift subscript's operand
  (`is_swift_subscript`: the grammar writes `a[0]` as a `call_expression` with
  bracketed `value_arguments`; Scala `a(0)` is `apply` and calls), `(*f)()`,
  sequences and conditionals are never the callee); a member callee behind
  wrappers or a TS instantiation at any depth is captured in code
  (`wrapped_member_callees`, TS/JS/Go/C/C++, reads the callee through
  `invoked_operand`), never by query shape; a non-invoked member name is an
  occurrence (with its path, in every operand position) only in
  Python/Java/C#/PHP/Lua/Swift/Scala, never in TS/JS/C/C++/Go/Rust, whose
  reference queries capture member names only as callees
  (`symbols_occurrences::OPERAND_MEMBER_MATRIX`, rows `R5-X.nn`);
  `symbols_occurrences::INVOCATION_MATRIX` and `NOT_CALLED_MATRIX` are its row
  tables (review rows `R4-F2.nn`, `R5-F1.nn`/`.Sn`; `LIMITED_INVOCATION_ROWS`
  keeps the C#/PHP/Perl/Java/Scala/Swift/C++ repairs conservative,
  `symbols_cache::invocation_kinds_hold_through_narrow_widened_projected_and_warm_results`/`constructed_and_implicit_member_kinds_hold_through_the_cache`
  and
  `refs::invocation_kinds_decide_callees_callers_and_the_default_report`/`constructed_types_and_implicit_members_are_callees_and_callers`
  are the cache and CLI consumers). Swift generic constructor adjacency skips
  grammar extras (`C /* note */ <Int>()` is Call); annotation types remain
  Usage. The shared `next_named_syntax_sibling` helper owns that step. Coverage
  (`Walk::visit_rust`/`visit_ecma` call and `new` callees) and the direct-`eval`
  test (`ecma::is_direct_eval`) read a callee through `invoked_operand` (the
  inverse of that climb: `wrapper_operand` + `generic_operand`, at any depth),
  never the outer node, so a wrapped call of a name limits nothing a direct one
  would not (`symbols_occurrences::CALLEE_COVERAGE_MATRIX`, `RECURSION_MATRIX`
  wrapped-call-to-another rows). Output order is sorted (occurrences by span…,
  candidates by name position then ID, coverage outer-first), never capture
  order:
  `analysis::occurrences::tests::capture_order_never_changes_the_evidence`.
  `symbols_occurrences` is the L1 file to extend (`RESOLVED_LANGUAGES` = the
  supported set; every language must report non-empty occurrences).
- **Predicates and the displayed prefix are post-lookup** (`file/query.rs`,
  `file/filter.rs`, `file/pattern.rs`, `file/naming.rs`): analysis and the cache
  produce *unfinished* results (in-file `qualified_name`, every record of the
  effective kinds); `SymbolQueryResult::finish` first applies the file-wide test
  rules (`is_test_file`: file names and the Rust package-`tests/` rule, which
  reads the nearest `Cargo.toml` and so can change while the file and its entry
  do not; records and entries keep only attribute/`cfg(test)` flags; a result
  remembers which records only that rule marked (`file_wide_tests`) and every
  `finish` undoes them before deciding again, so `narrowed` reuse follows a
  changed layout; `narrowed` returns `None` for a test-selector-filtered result
  whose file-wide decision changed; `cache_package_boundaries`,
  `symbols_flags::cached_test_flags_follow_the_package_layout` and
  `symbols_flags::narrowed_*_follow_the_package_layout` pin it), then prefixes
  every name not `is_namespace_rooted` (scan-root-relative `file#`, default the
  file's own directory, or a `NamingContext` module path), then applies
  `RecordFilter`. Never store or key on a finished result. A predicate pushes
  only what it reads into the effective kinds/capabilities
  (`SymbolQuery::settle`). `SymbolRef::qualified_name` in relations/exports
  stays in-file. `symbols_selection_filters` (`SELECTION_FILTER_MATRIX`, every
  predicate per language, block or whole-file regions) and
  `symbols_query_filters` (pushdown, cache isolation, `narrowed`, naming
  context, non-ASCII) are the tests to extend; tests that key records by in-file
  name use `symbols_fixtures::in_file_name`.

## Supported lexical name comparison

`analysis/name_identity.rs` owns comparison keys: Rust removes the raw prefix
and applies NFC; ECMA decodes identifier Unicode escapes without NFC. Imported
ECMA string originals decode string escapes as one name (dots are literal).
Scopes, candidates, Rust relation owners, recursion, diagnostics and cache
evidence consumers share these keys; public names and token geometry remain
written text. TypeScript's locals overlay captures bodyless function signatures
so ambient targets retain IDs and overload candidate lists stay complete.
`symbols_name_identity` partitions six-stage source regressions into bounded
registered groups with independent expected IDs and a corpus coverage guard.
