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

### Library API (`TreeFile`)

```rust
let file = TreeFile::new("src/lib.rs")?;
file.source();              // Source text of the parsed file
file.comment_ranges()?;     // Byte ranges of comment nodes
file.symbols()?;            // All symbols
file.symbol_records()?;     // v2 parse records
file.symbol_index_v2()?;    // v2 parse->bind->semantic->docs pipeline output
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

- **Readers get a `DeclarationContext`** (`declaration.rs`): language, source, final kind, `is_local`, the captured `name` node (one declaration can name several things: `int a, *b;`, Swift `case east, west`), the file `root` (same-file lookups: Rust trait level, earlier C/C++ `static` prototype, Scala `package` segments) and `is_ecma_module` (computed once per JS/TS file by `is_ecma_module_file`). Add a field here rather than re-scanning the file per symbol.
- **The C/C++ earlier-`static` scan matches only the same function** (`visibility/c.rs`). `declared_functions` (`c_declarators.rs`) takes the `function_declarator` nearest the name with only parentheses between, so a returned function-pointer, function-reference or array-pointer type is never the function; an unqualified C++ declaration has exactly its enclosing namespace; a qualified one resolves its first qualifier against the file's `namespace` blocks, nearest enclosing first; a function template (under `template<…>` or abbreviated with an `auto` parameter) also compares its header, leading and trailing return type, and the `requires` clause after the header and after the parameters (`TemplateSignature`; never `noexcept`). Whatever cannot be established is a miss, never a match. `symbols_visibility::earlier_static_linkage_matrix_holds_for_c_and_cpp` is the row table to extend.
- **C/C++ signatures read the named function's own declarator** (`signatures.rs`). `extract_signature` gets the captured name; `declared_function_named` (`c_declarators.rs`, shared with linkage) picks that function's `parameter_list`, so a pointer/reference/array return keeps its parameters and a returned callback's never replace them. Each parameter (`parameter_declaration`, `optional_parameter_declaration` with its default, `variadic_parameter_declaration` pack, C `variadic_parameter` and the C++ `...` token) is named by `declarator_name`, which follows a function declarator only through its `declarator` field. `symbols_signatures::c_and_cpp_signatures_read_the_named_functions_own_parameters` is the row table to extend.
- **C++ const-object linkage reads the declarator nearest the name** (`is_const_object`, `visibility/c.rs`). It reuses `declarator_layers` (parentheses and attributes add no layer; arrays are skipped to their element), so `char **const p` is const and `char *const *p` is not; a reference is checked before the `constexpr` shortcut and is never const. Any new reader of a declarator's type goes through `declarator_layers`, not the outermost node. `symbols_visibility::const_object_linkage_matrix_reads_the_outermost_declarator` is the table to extend.
- **Membership starts from `member_slot`** (`classify.rs`): the declaration or its outermost transparent wrapper (`is_transparent_wrapper_kind`: C++ `template_declaration`, C/C++/C# `preproc_*` conditionals, Python `decorated_definition`, Scala `extension_definition`). Kind, both `static` readers, C++ and C# access (owner read from `member_slot(..).parent()`) and C/C++/C# field, interface-property and enum-member lists (`flattened_members` in `type_metadata.rs`) all look through those wrappers; add a new wrapper there, not in one reader, and route any new reader of a declaration's parent or a type body's children through `member_slot`/`flattened_members`. Swift `#if` and C# `#region` are flat `directive`/`preproc_region` siblings, not wrappers.
- **Member kinds are validated, not trusted.** `refine_symbol_kind` demotes any Method/Constructor (to Function) or Field/Property (to Variable), whether from a query capture or a classifier rule, when `is_function_local_member` finds a function before any type or object owner above the member slot (C# local functions are captured as methods by the vendored query). Lua `M:m`/`M.f` names its owner and is exempt. The same nearest-owner rule (`is_member_owner_kind`) bounds `is_local_declaration`, so members of a type declared inside a function keep their access; a JS/TS `constructor` is a Constructor only directly in a `class_body`.
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
- **CLI filters**: `hug <subcommand> <filter...>` with all-files default scan for symbol commands. A pathless scan honors `.gitignore` and skips test-data dirs by default (`**/fixtures/**`, `**/__fixtures__/**`, `**/snapshots/**`, `**/testdata/**`); an explicit path/glob re-includes a targeted file. `lint` scans hide clean files (single-file lint still shows `(no diagnostics)`).

## Detailed Documentation

- [API Reference](./api-reference.md) - Complete type definitions and methods
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
   schema-2 literal key and the schema-3 key formula for each earlier
   extractor revision (`revision_key`); keep them unchanged when the current
   key changes, and add a `StaleRevision` case for the revision a bump
   retires. `cache_robustness` pins which malformed
   shapes of twelve load-bearing snapshot fields are served or recomputed (table in
   `docs/cache-design.md`); a schema change that adds a default or makes a
   field optional must update that row
7. `lib/tests/fixtures/symbols/multi/` is a 16-language tree (one file per
   language, some under `nested/`, `Widget` declared in two files) for
   multi-file, ordering, parallel and golden tests;
   `symbols_coverage::multi_tree_has_one_clean_file_per_language` guards it.
   `golden::plain_symbols_over_multi_match_the_golden` (CLI `symbols` binary)
   pins `hug --plain symbols` over it against
   `cli/tests/fixtures/golden/symbols.plain`; regenerate with
   `HUG_UPDATE_GOLDEN=1` only for a deliberate output change. Grammar limits
   found while writing fixtures: C# rejects an attribute between modifiers,
   and tree-sitter-zsh rejects a function body brace on the line after the
   header

The CLI's real-terminal `god-files` test is behind the internal
`terminal-tests` feature. Ordinary local L1 omits its harness and target;
`just test-l2` and CI enable it.

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

## Caching (Phase 5)

The `cache` module provides multi-tier caching for parse trees, symbol indexes, and diagnostics:

### Pass-Specific Fingerprints

`PassFingerprint` includes all inputs that affect a pass output:
- Schema version, tree-hugger version, grammar version/id
- Query hash, rule metadata hash, enabled rules hash, config hash
- External tool version (for adapter-backed passes)

### Cache Units

Cache entries are split by analysis stage for granular invalidation:
- `Parse`: Source text and grammar fingerprint
- `Symbols`: Symbol records from parse pass
- `Imports` / `Exports`: Relation records from bind pass
- `References`: Identifier usages
- `Diagnostics`: Lint and syntax diagnostics
- `Comments`: Ignore directives
- `ProjectGraph`: Project-level structures

### In-Process Cache

`InProcessCache` stores parsed file snapshots and symbol indexes in memory for the duration of a command:
- `get_parse_tree()` / `put_parse_tree()` — reuse parse trees
- `get_symbol_index()` / `put_symbol_index()` — reuse symbol indexes
- `record_hit()` / `stats()` / `hit_history()` — cache hit/miss tracking with timing

### Persistent Cache

`PersistentCache` stores serialized entries on disk under a tool-owned temp directory:
- Enabled by default; disabled with `--no-cache`
- Recomputes on any fingerprint mismatch or corrupt entry
- `invalidate()` for single entries, `invalidate_all()` for full cache clear

### Cache Invalidation Reasons

`CacheInvalidationReason` identifies why a cache entry was rejected:
- `SourceChanged`, `QueryChanged`, `GrammarChanged`
- `RuleMetadataChanged`, `ConfigChanged`, `ExternalToolChanged`
- `CorruptEntry`, `PassOptionsChanged`, `CacheDisabled`

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
