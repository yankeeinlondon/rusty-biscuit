---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/fixes/2026-10-03-visibility-and-kind/spec.md"
plan: "tree-hugger/fixes/2026-10-03-visibility-and-kind/plan.md"
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/l1/symbols_static.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_decl_text.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/lib/tests/fixtures/symbols/
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/prelude.rs
docs_updated_during_phase_2:
  - tree-hugger/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/extract/visibility/mod.rs
  - tree-hugger/lib/src/file/extract/visibility/c.rs
  - tree-hugger/lib/src/file/extract/visibility/cpp.rs
  - tree-hugger/lib/src/file/extract/visibility/csharp.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/file/extract/visibility/go.rs
  - tree-hugger/lib/src/file/extract/visibility/java.rs
  - tree-hugger/lib/src/file/extract/visibility/lua.rs
  - tree-hugger/lib/src/file/extract/visibility/php.rs
  - tree-hugger/lib/src/file/extract/visibility/rust.rs
  - tree-hugger/lib/src/file/extract/visibility/scala.rs
  - tree-hugger/lib/src/file/extract/visibility/swift.rs
  - tree-hugger/lib/src/file/extract/static_flags/mod.rs
  - tree-hugger/lib/src/file/extract/static_flags/cpp.rs
  - tree-hugger/lib/src/file/extract/static_flags/ecma.rs
  - tree-hugger/lib/src/file/extract/static_flags/java.rs
  - tree-hugger/lib/src/file/extract/static_flags/php.rs
  - tree-hugger/lib/src/file/extract/static_flags/python.rs
  - tree-hugger/lib/src/file/extract/static_flags/rust.rs
  - tree-hugger/lib/src/file/extract/static_flags/scala.rs
  - tree-hugger/lib/src/file/extract/static_flags/swift.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/l1/symbols_static.rs
  - tree-hugger/lib/tests/l1/symbols_decl_text.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/cli.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/query-system.md
packages:
  - tree-hugger
  - tree-hugger-cli
---

# Implementation Log for 2026-10-03-visibility-and-kind (7 phases)

## Phase 1

### Rulings

The plan's twelve "Necessary Rules" are acknowledged as decided. Rulings 2
(Rust associated const/type kinds) and 10 (no benchmark spike) are still
marked **AUTHOR** in the plan; the plan's defaults are what later phases
proceed with. The fixture now declares associated consts and types (see
below), so ruling 2 has rows to assert once Phase 4 fills the matrix.

### Baseline re-run

- Built `hug` at `21debbcc5` (the same rev the spike used) and ran
  `hug --json symbols --no-cache` over all 19 spike fixtures in a scratch git
  repo.
- Compared each result with `spike/raw/<stem>.json` through
  `spike/summarize.py` (kind, container, signature presence, signature
  visibility/static, v2 kind/level/modifiers/module path, field metadata).
  **0 differences in all 19 files.** The raw JSON differs only in path-derived
  values (`root_dir`, symbol IDs), which embed the scratch directory.
- **No further `ok` cell flips to `FIX`.** The spike's flips (Swift Method,
  Java Namespace, C++ union; partial Rust Method, Python Method, C named union
  tag) are already reflected in the spec's matrix.

### Fixtures seeded

Copied the 19 spike fixtures to `lib/tests/fixtures/symbols/` and added
`javascript_extension_module.mjs` and `typescript_extension_module.cts`
(module status from the extension alone: no import, export, `require` or
`module.exports`).

Auditing the spike fixtures against "The acceptance matrix" and test 9 found
constructs the spec requires that no fixture declared. Added (fixture-only):

| Fixture | Added | Required by |
|---|---|---|
| `rust.rs` | `mod geo { pub struct Coord }` + `impl crate::geo::Coord`; `pub struct Wrapper<T>` + `impl<T> Wrapper<T>`; `impl PrivateTrait for String` (impl for a type not declared in the file); trait/impl associated `const` and `type`; inherent associated `const` | kind matrix Rust `impl` bullets; ruling 2 |
| `javascript.js` | `static uninitializedStaticProperty;` (the `static y;` form); `this.assignedOnlyThroughThis = …` (a `this.w = …` naming no declared field) | kind matrix JS field paragraph |
| `java.java` | `@Deprecated class AnnotatedPackageWidget` | test 9 (annotated class) |
| `c_sharp.cs` | `public int Count { get; private set; }`, `internal int Total { get; private set; }` | C# Property `FIX`; setter-only modifiers; test 9 |
| `cpp.cpp` | `constexpr int CONSTEXPR_LIMIT`, `const char* constCharPointer`, `char* const constPointerToChar`; member function `countCalls()` holding a function-local `static` | C/C++ linkage list; test 7 C++ `hug classes` bullet |
| `swift.swift` | `public private(set) var count`, `private(set) var total`, `fileprivate(set) var fileprivateTotal` in `PublicApi` | setter-only modifiers; test 9 |
| `scala.scala` | `class Shape` with `protected[this] def tick()`, `private[example] def scale()`, `private[Shape] def secret() = 2` | Scala qualified access; test 9 |
| `perl.pl` | `package Fixture::Block { our …; sub block_package_sub { my … } }` and `sub after_block_sub` after the block (resumes `Fixture::Other`) | Perl block-form package; sibling scoping |

Native syntax checks (all pass): `rustc --edition 2024 --emit=metadata`,
`node --check` (4 JS files), `tsc --noEmit` (3 TS files), Python
`ast.parse`, `php -l`, `luac -p`, `perl -c`, `bash -n`, `zsh -n`,
`clang -fsyntax-only`, `clang++ -std=c++20 -fsyntax-only`, `swiftc -parse`.
No Go, Scala or C# toolchain on this host (`gofmt` absent); `javac` would
reject `java.java` by design (several public top-level types). For every
fixture, tree-sitter parsing is checked by the new L1 test
`symbols_coverage::every_symbols_fixture_parses_without_syntax_errors`.

### Grammar node-shape spike

Dumped with a throwaway example (`to_sexp()` over the fixture trees, deleted
after use). Field names in `name:`/`type:` form are tree-sitter fields.

**Rust** (`tree-sitter-rust` 0.24.0)
- `function_signature_item name: parameters: return_type:`; parent is the
  trait's `declaration_list`.
- `union_item (visibility_modifier) name: (type_identifier) body: (field_declaration_list …)`.
- `type_item (visibility_modifier) name: (type_identifier) type: …`; associated
  type in a trait is a separate node, `associated_type name:`; in an impl it is
  a plain `type_item`.
- `static_item (visibility_modifier)? name: (identifier) type: value:`; a
  function-local static has parent `block`. `static mut` adds a
  `mutable_specifier` child.
- `impl_item trait: … type: … body: (declaration_list …)`. `trait:` is a
  `type_identifier` or `scoped_type_identifier path: name:` (`fmt::Display`);
  `type:` is a `type_identifier`, a `scoped_type_identifier`
  (`crate::geo::Coord`), or `generic_type type: (type_identifier) type_arguments:`
  (`Wrapper<T>`).
- `visibility_modifier` children: none for `pub`; `(crate)`, `(super)`,
  `(self)`, or `(scoped_identifier path: (crate) name: …)` for `pub(in …)`.
- `mod_item (visibility_modifier) name: (identifier) body: (declaration_list)`.
- `enum_variant name: (identifier)` (struct variants carry
  `body: (field_declaration_list)`).

**TypeScript** (0.23.2)
- `abstract_class_declaration name: (type_identifier) body: (class_body)`.
- `abstract_method_signature name: (property_identifier) parameters: return_type:`
  inside `class_body`.
- `method_signature` and `property_signature` (`name: type:`) inside
  `interface_body`.
- `#name`: `private_property_identifier` as the `name` of both
  `public_field_definition` and `method_definition`.
- `public_field_definition (accessibility_modifier)? name: type: value:`.
- `type_alias_declaration name: (type_identifier) value:`; an exported one has
  parent `export_statement`.

**JavaScript**: class fields are `field_definition property: (property_identifier) value:`
(not `public_field_definition`); `this.x = …` is
`assignment_expression left: (member_expression object: (this) property:)`.

**Java** (0.23.5)
- `package_declaration (scoped_identifier scope: (scoped_identifier …) name: (identifier))`
  — no field name on the child, and it is a `scoped_identifier`, not a bare
  `identifier`, for dotted packages.
- interface constant: `constant_declaration type: declarator: (variable_declarator name:)`
  inside `interface_body`.
- annotation: `marker_annotation name:` inside the declaration's `modifiers`,
  so it is part of the `class_declaration` node.

**C / C++** (0.24.1 / 0.23.4)
- `union_specifier name: (type_identifier) body: (field_declaration_list)`; same
  shape in both grammars.
- `type_definition type: (struct_specifier name: body:) declarator: (type_identifier)`.
- C++ in-class method with body: `function_definition` whose parent is
  `field_declaration_list` and whose declarator is
  `function_declarator declarator: (field_identifier)` (namespace-level
  functions use `(identifier)`).
- access sections: `access_specifier` siblings inside `field_declaration_list`.
- linkage: `static` / `extern` / `inline` are `storage_class_specifier`;
  `const` and **`constexpr` are both `type_qualifier`** (text distinguishes
  them). `const char* p` puts `type_qualifier` on the declaration;
  `char* const p` puts it inside `pointer_declarator`. A `static` prototype is
  `declaration (storage_class_specifier) declarator: (function_declarator …)`.

**Swift** (0.7.1)
- every type kind, including `extension`, `enum` and `struct`, is
  `class_declaration`; an extension's `name:` is `user_type`, others are
  `type_identifier`. The keyword (`class`/`struct`/`enum`/`extension`) is an
  anonymous token (the `declaration_kind` field).
- `modifiers` holds `visibility_modifier` (one per keyword:
  `public private(set)` is two `visibility_modifier` children; `private(set)`
  alone is one, whose text includes `(set)`), and `property_modifier` for
  `static`/`class`.
- protocol requirements: `protocol_function_declaration name: (simple_identifier)`
  and `protocol_property_declaration name: (pattern bound_identifier:) … (protocol_property_requirements)`
  inside `protocol_body`.
- enum cases: `enum_entry name: (simple_identifier)` in `enum_class_body`;
  `case east, west` is **one** `enum_entry` with two `name:` fields.
- initializer: `init_declaration (modifiers)? body:`.
- function-local `let`: `property_declaration` with parent `statements`.

**C#** (0.23.1)
- `file_scoped_namespace_declaration name: (qualified_name qualifier: name:)` is
  a sibling of the declarations it scopes (child of `compilation_unit`).
- `property_declaration (modifier)* type: name: accessors: (accessor_list (accessor_declaration (modifier)?)…)`.
- explicit interface implementation: `explicit_interface_specifier (identifier)`
  as a child of `method_declaration`.
- each access keyword is its own `modifier` node.

**Scala** (0.24.0)
- `modifiers (access_modifier (access_qualifier (identifier)))`; `[this]`,
  `[symbols]`, `[Shape]` all have that same shape, so the qualifier is read by
  text.
- trait members: `function_declaration` (abstract) or `function_definition`
  inside `template_body`; object: `object_definition name: body: (template_body)`.

**Perl** (`tree-sitter-perl` 1.1.2)
- named sub: **`function_definition name: (identifier) body: (block)`** —
  there is no `subroutine_declaration_statement` node in this grammar version.
- `package Foo;`: `package_statement (package_name (identifier) (identifier))`.
  The block form `package Foo { … }` is the **same node with a `block`
  child**.
- `my`/`our`/`state`: `variable_declaration (scope) variable_name: (scalar_variable)`;
  the list form is `variable_declaration (scope) (array (scalar_variable) … )`
  with no `variable_name:` field.
- **`local $_` also parses as `variable_declaration`** with `(scope)` and
  `variable_name: (special_scalar_variable)`; the new Perl query must filter
  the scope keyword by text to exclude `local`.

**Python**: `decorated_definition (decorator (identifier)) definition: (function_definition …)`.

### Tests and scaffold

- `lib/tests/l1/symbols_fixtures.rs`: shared helper. `symbols_fixture_dir()`
  is `manifest_dir!().join("tests/fixtures/symbols")`, a root-anchored
  directory form that `docs/cicd/test-inputs.md` counts as a read of every
  file under it (and that the `rust-testing` skill lists).
- `lib/tests/l1/symbols_coverage.rs`: two green L1 tests —
  `every_language_has_a_primary_symbols_fixture` (the fixture half of test 1;
  the matrix-row half lands with the matrix tables) and
  `every_symbols_fixture_parses_without_syntax_errors`.
- Empty, documented modules registered in `lib/tests/l1/main.rs`:
  `symbols_kinds`, `symbols_visibility`, `symbols_static`, `symbols_adapter`,
  `symbols_decl_text`, `symbols_export`.
- CLI: new test binary `cli/tests/symbols/main.rs` (Cargo autotests picks up
  `tests/<dir>/main.rs`) declaring `wiring` (test 7) and `cache_upgrade`
  (test 8). Kept out of `cli.rs`, which is already 2k lines.

### Out-of-matrix observations (for the author, not fixed here)

None beyond the spike report's list.

### Requirement-to-test mapping (Phase 1)

Phase 1 changes no production behavior, so it adds no regression tests. The
two tests it adds guard the scaffold:

| Requirement | Test |
|---|---|
| every language has a primary `symbols/<language>.<ext>` fixture (fixture half of test 1) | `symbols_coverage::every_language_has_a_primary_symbols_fixture` |
| every symbol fixture is valid input, including Go/Scala/C#, which have no local compiler | `symbols_coverage::every_symbols_fixture_parses_without_syntax_errors` (asserts ≥ 21 files so it cannot pass vacuously) |

Input Robustness Matrix: not applicable (ruling 9); no file-format reader
changed.

### Gates

- `just test` (tree-hugger): 588 passed, 0 skipped, 0 failed.
- `just lint` (tree-hugger): clean; `cargo clippy --tests` on both packages
  also clean.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- Cross-OS: not run. Phase 1 adds fixtures and test scaffolding only;
  `.gitattributes` (`* text=auto eol=lf`) keeps the fixtures LF on Windows, so
  later exact `declaration_text` assertions will not see CRLF.
- No `cargo fmt` run.

## Phase 2

### Model changes

- **v1 (`lib/src/shared/symbol.rs`)**: `SymbolKind` gains `Constructor`,
  `TypeAlias`, `EnumVariant`, `Property` (declared next to their nearest
  relatives, so the derived `Ord` keeps every existing pair's order);
  `is_function()` includes `Constructor`, `is_type()` includes `TypeAlias`.
  `Visibility` gains `Package`, `Crate`, `ModulePrivate`,
  `ProtectedInternal`, `PrivateProtected` with display labels `package`,
  `crate`, `module-private`, `protected internal`, `private protected`; the
  stale `Internal` doc comment (it claimed the Java default) now reads C#/Swift
  `internal`. `SymbolInfo` gains `visibility: Option<Visibility>`,
  `is_static`, `is_static_storage` (all `serde(default)`, omitted when
  `None`/`false`). `FunctionSignature::visibility` is removed.
- **v2**: `VisibilityLevel::{ProtectedInternal, PrivateProtected}`,
  `Modifier::StaticStorage`, `IdentityFacet::container_kind:
  Option<SymbolKindV2>` (`serde(default)`, omitted when absent),
  `SchemaVersion::V3_0`. `V2_0` is kept, documented as no longer emitted, so
  Phase 5's cache-upgrade test can seed an old-format key without inventing a
  literal.
- **Schema 3.0 emit sites**: `analysis/mod.rs` (file index),
  `cli/src/main.rs` (JSON envelope and the analyzer fingerprint), the
  adapter, and every test that built a 2.0 record or fingerprint. The two
  `tree_file.rs` assertions on `major == 2` moved to 3 (pre-authorized).

### Adapter (`schema_v2/mod.rs`)

- Kind conversions in both directions and `Option<Visibility>` ↔
  `VisibilityLevel` are wildcard-free `match`es. The visibility pair is now
  two `From` impls (`From<Option<Visibility>> for VisibilityLevel`,
  `From<VisibilityLevel> for Option<Visibility>`) replacing the private
  `visibility_to_level`/`level_to_visibility` helpers, so tests and later
  phases use the same conversion.
- `From<SymbolInfo>`: level from `SymbolInfo::visibility`; `Modifier::Static`
  from `is_static`, `Modifier::StaticStorage` from `is_static_storage`;
  `identity.container_kind` from `container_kind`; `is_exported` is always
  `false`. `Field` and `Property` both get `FieldData` (with
  `is_static` from the shared flag); `TypeAlias` gets
  `TypeAliasData::default()` (Phase 3 fills the RHS); `EnumVariant` stays
  `SymbolKindData::Unknown` until Phase 3 decides its data.
- `TryFrom<SymbolRecord>`: restores kind, visibility, `is_static`,
  `is_static_storage`, `container_kind`. Every function-like kind gets
  `Some(signature)` (empty when the record has no `FunctionData`), with
  `signature.is_static` copied from the shared flag; non-function kinds never
  get one.

### Compile sweep (behavior otherwise unchanged)

- `extract_signature` no longer reads visibility and now **always returns a
  signature** for a function-like capture with a context node. Its old
  "empty → None" gate counted visibility, so keeping the gate without it
  would have dropped signatures from parameterless `public`/`private`
  methods that existing tests require. Returning `Some` is the spec's
  section 5 rule anyway; Phase 3 still owns the cases with no context node
  (Lua, Bash/Zsh, C# expression-bodied methods).
- `TreeFile::symbol_nodes` sets `SymbolInfo::visibility` from the same
  `extract_visibility` call, still for function kinds only, and `is_static`
  from the signature, so the extracted values are exactly what the signature
  used to carry. Phase 3 widens both to every kind. The duplicate-capture
  merge copies `visibility`/`is_static` together with the signature it
  adopts.
- `symbol_kind_priority`: `Constructor` 105 (above `Method`), `Property` 82
  (above `Field` and `Variable`), `TypeAlias`/`EnumVariant` with the other
  type-like kinds. Nothing extracts these v1 kinds yet; Phase 3 makes them
  real.
- CLI: renders `symbol.visibility` (three sites); new kinds take the color
  of their relatives (constructor green, alias magenta, variant/property
  cyan); `prelude.rs` sets the new fields. The `impl ` container special
  case and the `hug classes` partition are left for Phase 5.

### User-visible effects already present after Phase 2

Because the adapter no longer drops data, `hug` output changes before the
extraction work lands (checked with `hug symbols --plain --no-cache` over the
TS and Rust fixtures):

- container kinds print: `abstractMethod(): void [in class ExportedWidget]`,
  `crate_field [in type PublicStruct]` (Rust `impl` members still show
  `[in impl …]`, handled by the CLI special case until Phases 3/5);
- constructors and type aliases appear as `constructor` / `type alias`
  instead of collapsing to `Unknown` and dropping out of `hug functions` /
  `hug types`;
- JSON: `files[].symbols[].signature.visibility` is gone and
  `files[].symbols[].visibility` appears when known; `schema_version` is 3.0.

No CLI test asserted the old shapes, so none needed rewriting in this phase.

### Requirement-to-test mapping (Phase 2)

All in `lib/tests/l1/symbols_adapter.rs` (registered L1 module, no tier
marker):

| Requirement | Test |
|---|---|
| every kind maps both ways, none collapses to Unknown | `every_kind_survives_both_conversions` (explicit 19-pair table) |
| `is_function` ⊇ Constructor, `is_type` ⊇ TypeAlias | `constructor_is_function_like_and_type_alias_is_type_like` |
| every level incl. compound C# levels and Unknown↔`None`, on function, type, field and constant kinds | `every_visibility_level_survives_both_conversions` |
| new display labels | `compound_csharp_levels_have_their_own_labels` |
| export is not inferred from Public | `public_level_never_marks_a_record_exported` |
| `Static` vs `StaticStorage` separate, restored, signature copy agrees | `static_and_static_storage_are_separate_markers`, `static_field_reaches_field_data` |
| container kind round-trips (every kind) and absent stays absent | `container_kind_survives_both_conversions` |
| function-like kinds always get a signature (parameterless fn/method/constructor), non-function kinds never | `function_like_kinds_always_carry_a_signature` |
| signature content preserved | `signature_content_is_preserved` |
| persisted record write/read/convert/write/read keeps new fields; `container_kind` omitted when absent; schema 3.0 | `serialized_record_round_trips_the_new_fields` |
| v1 JSON: new fields absent → Unknown/false; present → read back | `symbol_info_json_defaults_and_omissions` |
| both directions agree for every symbol of every fixture (test 5 skeleton; explicit per-symbol values are Phase 6) | `adapter_agrees_with_extraction_for_every_fixture_symbol` (non-vacuity: >100 records) |

Updated existing tests: `tree_file::extracts_typescript_visibility_modifiers`,
`infers_csharp_interface_method_visibility`,
`infers_java_interface_method_visibility` read `symbol.visibility` (they still
assert a signature is present); `emits_symbol_records_v2` and
`emits_symbol_index_v2_with_completed_passes` expect major 3.

Input Robustness Matrix: not applicable (ruling 9). Code-smell grep over the
diff: the new `serde(default)` fields (`SymbolInfo::{visibility, is_static,
is_static_storage}`, `IdentityFacet::container_kind`) have a documented
absence meaning (Unknown / `false` / no container) from the spec, and absent
vs explicit null is not a distinction any of them draws.

### Gates

- `just test` (tree-hugger): 601 passed, 0 skipped, 0 failed (588 + 13 new).
- `just lint` (tree-hugger): clean; `cargo clippy --all-targets
  --all-features` on both packages also clean.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- Cross-OS: not run. Phase 2 changes only in-memory types, serde shapes and
  match arms; no path, process or filesystem behavior changed.
- No `cargo fmt` run.


## Phase 3

### Structure

- **Module split (behavior-preserving).** The keyword readers moved out of
  `signatures.rs` into `lib/src/file/extract/visibility/<lang>.rs` (`ecma`,
  `java`, `csharp`, `php`, `rust`, `cpp`, `swift`) and
  `static_flags/<lang>.rs` (`ecma`, `java`, `php`, `python`, `swift`,
  `scala`, `cpp`, `rust`), code unchanged. Each directory has one dispatcher
  (`extract_visibility`, `extract_is_static`, `extract_is_static_storage`)
  taking any declaration node plus a `DeclarationContext { language, source,
  kind, is_local }`. C, Go, Scala and Lua have stub visibility readers that
  return Unknown, so Phase 4 fills one file per language plus nothing shared.
  Python, Perl, Bash and Zsh return Unknown in the dispatcher (permanent). The
  dispatcher match is exhaustive over `ProgrammingLanguage`.
- **Declaration node (`declaration.rs`).** Each symbol now has two nodes:
  the *anchor* (context capture, else the name; unchanged, still the record's
  `declaration_span`, `body_span` and export node, so reference ownership in
  the bind pass is unchanged) and the *declaration* (context capture, else
  `resolve_declaration_node(name)`: the nearest declaration-like ancestor,
  never crossing a body, block, file root, parameter list or lambda). A C/C++
  `function_declarator` context is widened to the enclosing
  `function_definition` / `declaration` / `field_declaration`
  (`declaration_for_context`), where storage class and access sections
  live. Visibility, both static flags, signatures, the container search and
  `declaration_text` all read the declaration node.
- **Locals.** `is_local_declaration` (any function-like ancestor) gates
  visibility and member static to `None`/`false` for parameters, locals and
  nested functions. Storage `static` is not gated (function-local C/C++
  statics need it).

### Extraction core

- **Kinds at extraction.** `refine_symbol_kind(kind, name, declaration,
  language, source)` now owns the former `symbol_kind_v2_for_node` rules (TS
  constructor, `public_field_definition`/`property_definition`/
  `property_signature` → Property, `type_alias_declaration` → TypeAlias) plus
  Rust `type_item`/`associated_type` → TypeAlias, `enum_variant` →
  EnumVariant, `mod_item` → Module, `static_item` → Variable. The override is
  deleted; `symbol_records()` uses the converted kind. The `var` refinement
  walks only from the name up to its declaration (was: every ancestor), and a
  Swift `property_declaration` outside a type body is a Variable. Capture names
  `constructor`, `property`, `type_alias`, `enum_variant` now map to their
  kinds for Phase 4 queries.
- **Direct-member rule.** A `Function` whose declaration sits directly in a
  type-like body (wrappers: `*_body`, `block`, `declaration_list`,
  `field_declaration_list`, `template_body`, `decorated_definition`) becomes
  `Method`.
- **Containers.** `find_symbol_container` starts at the declaration's parent
  (never self), returns a `ContainerRef { name, lookup }`. Container kind is
  resolved after all symbols are extracted: the kind extracted at the
  container's own declaration node (same classifier), else a per-language
  fallback (Swift `class_declaration` → Type). Added abstract TS class, Rust
  union, C/C++ union, C++ `class_specifier`, PHP trait, Scala trait/object.
  Sibling-scoped containers: C# `file_scoped_namespace_declaration` and Perl
  statement-form `package Foo;` (scanned among preceding siblings at each
  level); Perl block packages and subs are ancestor containers.
- **Rust impl (`rust_impl.rs`).** Container = last path segment of the self
  type without generics; kind = the same-file type declarations of that name
  when they agree, else absent. `symbol_records()` fills
  `relations.implements` for direct members of `impl Trait for Type` (`id`
  only when exactly one same-file trait has that name). The bind pass's
  container linking falls back to name + `container_kind` when no enclosing
  span matches, so `hug classes` lists `impl` members (verified: PublicStruct
  shows 2 static, 4 instance methods; Coord/Wrapper/PrivateStruct their
  methods; `impl … for String` links nothing).
- **Signatures.** Every function-like symbol gets `extract_signature` on its
  declaration node. Added Lua parameters (`extract_lua_parameters`, `...` is
  variadic). C# return type now read from the `returns`/`type` field first,
  so user-defined return types (`PublicWidget StaticFactory() => …`) are no
  longer dropped. `extract_signature` takes the shared `is_static`; the
  signature carries a copy.
- **Declaration text.** `declaration_text(declaration, source)`: start of
  the declaration node to the start of `body_node_for_declaration` (the body
  `body_span_for_declaration` reports), trailing whitespace trimmed.
- **Export.** `is_exported_definition` also excludes Property, Constructor
  and EnumVariant. Only `symbol_records()` and the bind pass assign
  `is_exported` (grep-verified).
- **`EXTRACTOR_REVISION`** = `"1"` (`lib/src/file/mod.rs`, re-exported at the
  crate root). Records stamp `provenance.extractor_version =
  "<crate version>+rev.<revision>"`.

### Departure from the plan: CLI fingerprint wired now, not in Phase 5

After the extraction change, `test_symbols_include_container_context_for_methods`
(expects `[in impl Greeter]`) still **passed locally** while
`hug --no-cache` printed `[in type Greeter]`: the warm persistent cache served
the old record, which is the stale-cache defect itself. On a cold CI cache the
test would have failed. So `query_fingerprint` in `cli/src/main.rs` now reads
`locals+imports+references;extractor-revision={EXTRACTOR_REVISION}`, and the
test expectation moved to `[in type Greeter]` (pre-authorized rewrite).
Phase 5 still owns test 8 (cache-upgrade) and the rest of the CLI work; the
`impl ` special case in the renderer is still there (it now never matches).

### Superseded tests rewritten (pre-authorized)

- `tree_file::distinguishes_typescript_interface_from_enum`: `GreetFn` is
  `TypeAlias` (override removal).
- `cli::test_symbols_include_container_context_for_methods`:
  `[in type Greeter]`.
- `cli::test_rust_structs_render_as_struct_not_type`: types now carry
  visibility, so the line reads `public struct Greeter`.

### Interim states (intended; later phases change them)

- Container kinds follow the standalone classifier, so Java and Python
  containers now print `[in type X]` instead of `[in class X]` because their
  classes are still captured as `Type`. Phase 4 changes those classes to
  `Class` and the containers follow automatically.
- Existing keyword readers now run on every kind: Rust `pub(crate)` fields
  and items report Public, unmarked items Unknown; Swift `fileprivate` →
  Private; C++ access only where a section label directly precedes; C#
  compound levels collapse. These are the Phase 4 rows.
- Swift `class func` is not flagged static (the existing reader does not see
  `class`); Phase 4 Swift.
- C/C++ typedefs still `Type`; C++ `class` still `Type`; Rust `type`/
  `static`/`union` items and bodyless trait methods are not captured yet
  (classification and alias RHS are in place and unit-tested, the queries are
  Phase 4).
- The plain CLI renderer prints `[in …]` only for Method/Function/Field/
  Parameter, so `enum variant`, `property` and `constructor` lines show no
  container even though `container_name`/`container_kind` are set (Phase 5
  rendering).

### Out-of-matrix observation (recorded, not fixed)

- Go type declarations' `declaration_text` includes the struct body:
  `body_span_for_declaration` finds no `body` on `type_declaration`, and the
  spec defines the body as that function's span. Changing it would also change
  Go `body_span`; left for the author.

### Requirement-to-test mapping (Phase 3)

All L1, in registered modules of `lib/tests/l1/` (no tier markers) or in-crate
unit tests (`file::extract::classify::tests`, run by `just test`):

| Requirement | Test |
|---|---|
| Rust kind/container smoke sample (EnumVariant, Module not self-contained, impl members → implemented type incl. `crate::geo::Coord`, `Wrapper<T>`, foreign type with no kind, union fields), both paths | `symbols_kinds::rust_kind_rows_hold_through_both_paths` |
| `relations.implements`: same-file trait with id, std trait without, inherent impl empty, parameters excluded | `symbols_kinds::rust_trait_impl_members_record_the_implemented_trait` |
| bind links impl members to the type (`hug classes` input); foreign type unlinked | `symbols_kinds::rust_impl_members_are_members_of_the_implemented_type` |
| kinds decided once: `symbols()` and `symbol_records()` agree on kind/container/container kind for every fixture symbol | `symbols_kinds::symbols_and_records_agree_for_every_fixture_symbol` (>400 symbols) |
| container kind uses the standalone classifier | `symbols_kinds::container_kind_matches_the_containers_standalone_kind` (>100 containers) |
| TS constructor/property/alias reach v1 symbols; alias RHS; constructor has a signature, property none | `symbols_kinds::kinds_decided_at_extraction_reach_the_v1_symbols` |
| variants: no signature, `Unknown` data, enum keeps its variant metadata | `symbols_kinds::enum_variants_never_carry_a_signature` |
| Swift function-local `let` → Variable; type property stays Field | `symbols_kinds::swift_function_local_binding_is_a_variable_not_a_field` |
| direct-member rule (Python static/class methods, Scala trait/object/class, Swift class/extension/struct, C++ in-class declaration) | `symbols_kinds::functions_declared_directly_in_a_type_body_are_methods` |
| nested function stays Function (Python, Rust) | `symbols_kinds::function_nested_in_a_function_body_stays_a_function` |
| C++ class beats namespace, C/C++ unions, PHP trait, TS abstract class, C# file-scoped namespace | `symbols_kinds::members_name_their_nearest_type_container` |
| signatures: Lua params + variadic, Bash/Zsh, C# user-defined return type | `symbols_kinds::function_like_symbols_carry_extracted_signatures` |
| Perl sibling/block package and sub containers | `classify::tests::perl_packages_and_subs_contain_the_declarations_they_scope` |
| Perl list form shares `my ($a, $b)` text | `classify::tests::perl_list_declaration_text_covers_every_name` |
| Rust alias RHS + TypeAlias, Rust `static` → Variable | `classify::tests::rust_type_alias_target_is_the_right_hand_side`, `rust_static_item_is_a_variable` |
| new capture names | `classify::tests::capture_names_cover_every_extracted_kind` |
| visibility on types/fields/constants/modules, C++ section on in-class declaration | `symbols_visibility::declaration_keywords_apply_to_every_kind` |
| params/locals/nested functions Unknown even in an interface | `symbols_visibility::parameters_and_locals_stay_unknown` |
| member static on methods and fields with every copy (signature, FieldData, Modifier) | `symbols_static::member_static_reaches_methods_and_fields` |
| storage vs member static (C/C++ file-scope, function-local, member-local; C++ members) | `symbols_static::storage_static_is_kept_apart_from_member_static` |
| non-empty, trimmed declaration text for every fixture record | `symbols_decl_text::every_fixture_record_has_declaration_text` (>400) |
| exact text: `pub(in …)`, `pub(crate)` field, Swift `public private(set)`, C# `{ get; private set; }`, Scala `private[Shape] def secret() =`, Java annotation, TS `export` excluded | `symbols_decl_text::declaration_text_keeps_unmappable_syntax_and_stops_at_the_body` |
| export from syntax, not access; members (constructor, property, variant) never exported, on both paths | `symbols_export::export_follows_export_syntax_not_access`, `members_of_exported_types_are_not_exports` |
| records stamp `EXTRACTOR_REVISION` | `symbols_adapter::records_stamp_the_extractor_revision` |

Input Robustness Matrix: not applicable (ruling 9). Code-smell grep over the
diff: no new `serde(default)`, `unwrap_or_default()` only on node text in
test helpers, no `filter_map(.. as_str())` on a load-bearing parse.

### Gates

- `just test` (tree-hugger): 627 passed, 0 skipped, 0 failed (601 + 26 new).
- `just lint` (tree-hugger): clean; `cargo clippy -p tree-hugger -p
  tree-hugger-cli --all-targets --all-features -- -D warnings` clean.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- Cross-OS: not run. The phase changes in-memory tree walking only; the one
  filesystem touch is test-side (`tempfile` with an extension suffix, and an
  `include_str!` with `/` separators, both portable). No path comparison or
  process behavior changed.
- No `cargo fmt` run.

### Re-verification (2026-10-03, second Phase 3 invocation)

A later invocation for Phase 3 found every Phase 3 task already checked off and
this section complete, so it did not change any source. It re-ran the gates
against the current working tree instead:

- `just test` (tree-hugger): 627 passed, 0 skipped, 0 failed.
- `just lint` (tree-hugger): clean for `tree-hugger` and `tree-hugger-cli`.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- Every test named in the requirement-to-test mapping above exists, and its
  module is declared in `lib/tests/l1/main.rs` (or it is an in-crate unit test).
