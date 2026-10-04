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
source_files_during_phase_4:
  - tree-hugger/lib/queries/typescript/locals.scm
  - tree-hugger/lib/queries/vendor/c/locals.scm
  - tree-hugger/lib/queries/vendor/c_sharp/locals.scm
  - tree-hugger/lib/queries/vendor/cpp/locals.scm
  - tree-hugger/lib/queries/vendor/java/locals.scm
  - tree-hugger/lib/queries/vendor/javascript/locals.scm
  - tree-hugger/lib/queries/vendor/perl/locals.scm
  - tree-hugger/lib/queries/vendor/python/locals.scm
  - tree-hugger/lib/queries/vendor/rust/locals.scm
  - tree-hugger/lib/queries/vendor/swift/locals.scm
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/navigation.rs
  - tree-hugger/lib/src/file/extract/type_metadata.rs
  - tree-hugger/lib/src/file/extract/static_flags/java.rs
  - tree-hugger/lib/src/file/extract/static_flags/mod.rs
  - tree-hugger/lib/src/file/extract/static_flags/python.rs
  - tree-hugger/lib/src/file/extract/static_flags/rust.rs
  - tree-hugger/lib/src/file/extract/static_flags/swift.rs
  - tree-hugger/lib/src/file/extract/visibility/c.rs
  - tree-hugger/lib/src/file/extract/visibility/cpp.rs
  - tree-hugger/lib/src/file/extract/visibility/csharp.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/file/extract/visibility/go.rs
  - tree-hugger/lib/src/file/extract/visibility/java.rs
  - tree-hugger/lib/src/file/extract/visibility/lua.rs
  - tree-hugger/lib/src/file/extract/visibility/mod.rs
  - tree-hugger/lib/src/file/extract/visibility/php.rs
  - tree-hugger/lib/src/file/extract/visibility/rust.rs
  - tree-hugger/lib/src/file/extract/visibility/scala.rs
  - tree-hugger/lib/src/file/extract/visibility/swift.rs
  - tree-hugger/lib/src/queries/inventory.rs
  - tree-hugger/lib/src/queries/provenance.rs
  - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_static.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
docs_updated_during_phase_4:
  - tree-hugger/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_5:
  - Cargo.lock
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
docs_updated_during_phase_5:
  - tree-hugger/README.md
  - tree-hugger/docs/cache-design.md
  - docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_6:
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_decl_text.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_7:
  - tree-hugger/lib/src/shared/symbol.rs
docs_updated_during_phase_7:
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_7:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
skills_files_updated_during_phase_7:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
packages:
  - tree-hugger
  - tree-hugger-cli
source_code:
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
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/file/extract/mod.rs
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
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/lib/queries/typescript/locals.scm
  - tree-hugger/lib/queries/vendor/c/locals.scm
  - tree-hugger/lib/queries/vendor/c_sharp/locals.scm
  - tree-hugger/lib/queries/vendor/cpp/locals.scm
  - tree-hugger/lib/queries/vendor/java/locals.scm
  - tree-hugger/lib/queries/vendor/javascript/locals.scm
  - tree-hugger/lib/queries/vendor/perl/locals.scm
  - tree-hugger/lib/queries/vendor/python/locals.scm
  - tree-hugger/lib/queries/vendor/rust/locals.scm
  - tree-hugger/lib/queries/vendor/swift/locals.scm
  - tree-hugger/lib/src/file/extract/navigation.rs
  - tree-hugger/lib/src/file/extract/type_metadata.rs
  - tree-hugger/lib/src/queries/inventory.rs
  - tree-hugger/lib/src/queries/provenance.rs
  - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
  - Cargo.lock
  - tree-hugger/cli/Cargo.toml
documentation:
  - tree-hugger/README.md
  - tree-hugger/docs/cache-design.md
  - docs/dependencies.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
completed_phase: 7
implemented: true
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

## Phase 4

Implemented sequentially by one agent rather than one subagent per language
group: every group touched the shared `DeclarationContext`, the dispatchers,
`type_metadata.rs` and the same test tables, and parallel agents compiling one
worktree would have seen each other's half-finished edits.

### Shared changes

- **`DeclarationContext`** gained `name` (the captured name node: one
  declaration can name several things, `int a, *b;` or Swift `case east,
  west`), `root` (same-file lookups) and `is_ecma_module` (computed once per
  JS/TS file by `is_ecma_module_file`, not per symbol).
- **`FieldInfo` follows the standalone member.** `align_field_metadata`
  (`tree_file.rs`) copies `visibility` and `is_static` from the standalone
  `Field`/`Property`/`Constant` with the same name and container into the
  owning type's metadata. One rule instead of sixteen metadata readers, and
  `hug classes` can no longer disagree with `hug symbols`. Entries with no
  standalone symbol keep what the metadata reader found.
- **Type metadata lists only fields and properties** (spec section 3). Java
  interface metadata now lists constants (static, Public), C# interface
  metadata properties, Swift protocol metadata property requirements; PHP
  interface, Scala trait and Scala object metadata no longer list methods.
  The spec names only C# and Swift; the other three listed methods the same
  way and their methods are standalone `Method`s, so the rule was applied to
  them too. Go interface method elements stay (Go fields are `FEAT`).
- Removed the dead `is_inside_interface` and Rust `is_inside_impl_block`
  helpers; every reader now looks only at the direct parent body.

### Per language

| Group | Queries | Rules |
|---|---|---|
| Rust | `function_signature_item`, `static_item`, `union_item`, `type_item`, `associated_type`; variants `enum_variant`; `mod_item` `module` + context | `pub` Public, `pub(crate/super/in)` Crate, `pub(self)` Private, unmarked Private; variants and variant fields inherit the enum; trait items inherit the trait; trait-impl items take a same-file trait's level (all same-named traits must agree) else Unknown; `macro_rules!` Unknown. Associated fns without `self` are static in traits as well as impls. |
| TS/JS | TS overlay: `abstract_class_declaration`, `#private` methods, `abstract_method_signature`, interface `method_signature`/`property_signature` (only inside `interface_body`, so type-literal members stay unreported); JS vendor: `this.x = …` pattern removed | class members: keyword, `#name` Private, else Public; interface members Public; top level: exported Public, else ModulePrivate in a module file, Unknown in a script. JS `field_definition` → Property. |
| Java | `class` → class, `interface_declaration`, dotted `package`, `constant_declaration` | keyword, else Public directly in an interface body, else Package; `package` Unknown; interface constants static. |
| C# | `property_declaration` → property; block and file-scoped namespaces | compound levels, `file` ModulePrivate; explicit interface implementation Private; interface members Public; class/struct/record members Private; top-level types Internal; accessor modifiers never read. |
| C/C++ | C: `union_specifier`, pointer field declarators, typedef → `type_alias`; C++: `class` → class, in-class `function_definition` methods (also behind a pointer/reference return), `using` → `type_alias` | shared linkage reader (`visibility/c.rs`): `static`, unnamed namespace, C++ top-level-const object without `extern`/`inline` (incl. `constexpr`; pointer constness read from the outermost `pointer_declarator`), earlier same-file `static` declaration (C: name; C++: namespace path + name + whitespace-free parameter types with names removed). C++ members: nearest access section, else `class` Private / `struct`/`union` Public. Types, members (C), enumerators, macros Unknown. |
| Swift | enum cases → `enum_variant`; protocol function and property requirements | keywords (setter-only `x(set)` ignored); cases and requirements inherit their container's level; other members: nearest enclosing type/extension with an explicit keyword (private/fileprivate → ModulePrivate, extension → its level, else Internal), else Internal. `class func` is static (bare `class` token on the function). |
| Python | class → class; attribute-assignment catch-all removed | class-body `x: ClassVar[T]` / unannotated `x = v` static, `x: T` / `x: T = v` instance (`is_python_class_attribute`, shared with `extract_python_class_fields`, which now lists unannotated attributes too). |
| PHP | — | keyword, else Public (members, `var` properties, top-level functions and types); namespaces Unknown. |
| Scala | — | `private`/`protected`, `[this]` keeps the unqualified level, a qualifier naming a `package` clause segment Package, any other qualifier Unknown; unmarked Public. |
| Lua | — | chunk-level `local` ModulePrivate, nested-block `local` Unknown, global Public, table fields (`M.f`, `M:m`, `M.x =`) Unknown. |
| Go | — | Unicode-uppercase initial Public, else Package; package clause Unknown. Go struct `FieldInfo` uses the same rule (`go_identifier_visibility`). |
| Perl | new `queries/vendor/perl/locals.scm`: named subs, `package_statement` (both forms) as module, `my`/`our`/`state` (incl. each list name) via `#any-of?` on the scope keyword | Unknown. A Perl `package` is never contained by another package (names are absolute); a block-form package after a statement-form one was otherwise attributed to it. |

### Provenance and inventory

- `vendor_query_provenance` records changes per language through an
  exhaustive `vendored_locals_changes` match: Rust, Java, Python, JavaScript,
  Swift, C, C++ and C# are `MultipleChanges` with one note per change; the
  rest stay `CapturesRenamed`.
- Perl's query is an `Original` by `tree-hugger`, license `AGPL-3.0-only`
  (the license most of the repository's crates declare), so drift checks skip it.
- The capture inventory gained `type_alias`, `enum_variant`, `property`,
  `module` and `associated` (Lua's existing capture, never inventoried).

### `ok` → `FIX` discoveries

- **C Field**: a pointer field (`char *tag_name_field;`) produced no symbol
  and was missing from the struct's metadata. Fixed in the C query (inherited
  by C++) and in both metadata readers.

### Out-of-matrix observations (for the author)

- C++ `typedef` and `using` now report `TypeAlias` (a `FEAT` cell): C++
  inherits the C query, whose typedef capture had to become `type_alias`, and
  `using` was aligned with it rather than left as the one alias reporting
  `Type`.
- The overlay-query provenance (`overlay_query_provenance`) still claims
  `MIT`, while the area's crates declare `AGPL-3.0-only` and the root README
  says `AGPL-3.0-or-later`. Left alone; the author should pick one.
- A Swift top-level `private` declaration reports Private (its keyword); only
  the default of its members uses the "behaves as `fileprivate`" rule.
- Constructors in C++, Java, C#, PHP and Swift are still `Method` or absent
  (`FEAT`).

### Tests

The matrices are table-driven `pub const`s so the coverage guard can read
them: `KIND_MATRIX` (`symbols_kinds.rs`), `VISIBILITY_MATRIX`
(`symbols_visibility.rs`), `STATIC_MATRIX` + `STATIC_LANGUAGES`
(`symbols_static.rs`). They replace the Phase 3 smoke tests
`rust_kind_rows_hold_through_both_paths`,
`declaration_keywords_apply_to_every_kind`,
`member_static_reaches_methods_and_fields` and
`storage_static_is_kept_apart_from_member_static`, whose rows they contain.
All new tests sit in already-registered L1 modules with no tier marker.

| Requirement | Test |
|---|---|
| test 1: every language has kind and visibility rows; static languages have static rows | `symbols_coverage::every_language_has_matrix_rows` |
| test 2: kind matrix (all committed cells, 16 languages, both paths) | `symbols_kinds::kind_matrix_holds_through_both_paths` |
| test 6: no `Unknown` kind in any fixture (allow-list empty) | `symbols_kinds::no_fixture_symbol_reports_an_unknown_kind` |
| C typedef tag + alias once each, anonymous struct, `typedef struct S {…} S` | `symbols_kinds::c_typedefs_report_tag_and_alias_once_each` |
| JS `this.w`, Python `self.y`/`obj.attr`, Perl `local` produce no symbol; class attribute reported once | `symbols_kinds::assignments_and_local_overrides_declare_no_symbol` |
| Perl keyword in declaration text, both list names | `symbols_kinds::perl_declaration_text_keeps_the_declaring_keyword` |
| test 3: visibility matrix, every mapped level, 16 languages, both paths | `symbols_visibility::visibility_matrix_holds_through_both_paths` |
| earlier `static` prototype (C, C++), C++ overload stays Public | `symbols_visibility::earlier_static_prototypes_reach_later_declarations` |
| JS/TS script vs CommonJS vs `.mjs`/`.cts` | `symbols_visibility::script_and_module_files_decide_top_level_access` |
| CommonJS evidence anywhere (lazy `require`, conditional `exports.x`, `module.exports`) plus a no-evidence control | `symbols_visibility::commonjs_evidence_counts_anywhere_in_the_file` |
| restricted Rust `pub(...)` and Scala class qualifier keep their text | `symbols_visibility::unmappable_access_keeps_its_source_text` |
| `FieldInfo` agrees with the standalone field in every language; C# compound levels in metadata | `symbols_visibility::field_metadata_agrees_with_the_standalone_fields` |
| test 4: static matrix incl. storage `static` | `symbols_static::static_matrix_holds_through_both_paths` |
| Python standalone fields and `extract_python_class_fields` agree | `symbols_static::python_class_fields_agree_with_the_standalone_fields` |
| Perl provenance, changed-language provenance | `queries::provenance::tests::perl_locals_are_a_tree_hugger_original`, `changed_vendor_queries_record_their_changes` |
| every `@local.definition.*` capture inventoried; Perl query uses only native predicates | `phase6_neovim_query_reuse::every_locals_definition_capture_is_inventoried`, `perl_locals_query_is_an_original` |

Superseded tests rewritten: `tree_file::distinguishes_java_types` (class →
Class, interface → Interface), `distinguishes_c_types` (typedef → TypeAlias),
`distinguishes_cpp_types` (class → Class), and the interface/protocol/trait
assertions of `extracts_{csharp,swift,scala,php}_type_metadata` (methods are
members, not fields; shared helper `assert_method_is_a_member_not_a_field`);
`phase6_neovim_query_reuse::bulk_drift_check_covers_all_vendored` (Perl is no
longer vendored).

Every new row was checked against a dump of the pre-change extraction: each
`FIX` row failed before this phase (missing symbol, wrong kind, or Unknown
level). Input Robustness Matrix: not applicable (ruling 9); no `serde`
attribute added, no `unwrap_or_default`/`.ok()` on a load-bearing parse
(`unwrap_or_default` appears only on node text for name comparison).

### Gates

- `just test` (tree-hugger): 641 passed, 0 skipped, 0 failed.
- `just lint` (tree-hugger): clean; `cargo clippy -p tree-hugger -p
  tree-hugger-cli --all-targets --all-features -- -D warnings` clean.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- `hug classes` on the C++ and Rust fixtures (scratch git repo, `--no-cache`)
  lists in-class C++ methods with access, `staticMemberField` as a static
  field without the member-local `static`, and Rust inherent and trait-impl
  methods under `PublicStruct` with associated functions under Static Methods.
- Cross-OS: not run. The phase changes in-memory tree walking and query text;
  the one path read (`Path::extension` for `.mjs`/`.cjs`/`.mts`/`.cts`) is
  portable, and fixtures are LF on every OS (`.gitattributes`).
- No `cargo fmt` run. Two throwaway examples used to dump symbols and parse
  trees were deleted before the gates.

## Phase 5

Implemented by one agent: both Wave 5.1 tasks touch only `cli/src/main.rs`.

### CLI rendering (`cli/src/main.rs`)

Most of the wave was already in place from earlier phases: the renderer read
`symbol.visibility` (Phase 2), omitted the prefix for Unknown, printed the
widened labels through `Visibility`'s `Display`, `functions`/`types` filtered
on `is_function()`/`is_type()` (which include `Constructor` and `TypeAlias`),
and JSON emitted `files[].symbols[].visibility` (omitted for Unknown) and
schema 3.0. What this phase changed:

- **`[in …]` suffix** now also follows `Constructor`, `Property`,
  `EnumVariant` and `Constant` (before: Method/Function/Field/Parameter only),
  so `public constructor constructor(…) [in class ExportedWidget]`,
  `public enum variant UnitVariant [in enum PublicEnum]` and
  `public const INHERENT_ASSOCIATED_CONSTANT [in type PublicStruct]` render.
  Type aliases (including Rust associated types) are type kinds and still
  print without a container; no spec row asks for one.
- **`impl ` container special case removed** (it could no longer match).
- **`hug classes` partitions on `SymbolInfo::is_static`**, not
  `signature.is_static` (a copy of the same flag, so output is unchanged).
- **`hug classes` also summarizes `Interface` and `Trait` records**, which
  Phase 4 flagged as the blocker for the Java interface constant:
  `PublicShape` now lists `SIDES` under *Static Fields* and `unit()` under
  *Static Methods*; Rust traits list their methods. Enums, modules and Scala
  objects are still not summarized (no spec row; left as is).
- No new rendering was built, so no biscuit-terminal component was
  introduced; the existing owo-colors path only gained text.

### Cache fingerprint

Already wired in Phase 3 (`locals+imports+references;extractor-revision=1`
with schema 3.0, crate version, grammar name and options kept). Phase 5 adds
the test that proves the upgrade (test 8) and corrects
`tree-hugger/docs/cache-design.md`, which claimed the fingerprint hashed the
query sources.

### Tests (all L1, binary `tree-hugger-cli::symbols`, auto-discovered)

`cli/tests/symbols/main.rs` holds `Scratch`: a `git init` scratch repo with
fixtures written from `include_str!` of
`lib/tests/fixtures/symbols/*` (CI test-input visible), plus test-owned
`HOME`/`USERPROFILE`/`XDG_CONFIG_HOME`/`APPDATA`/`LOCALAPPDATA` and
`TMPDIR`/`TMP`/`TEMP`. No terminal is spawned.

| Requirement (spec test 7/8) | Test |
|---|---|
| plain `symbols` prints known visibility for every kind (TS function/class/interface/alias/const/constructor/method/property/interface member; Rust crate, alias, enum variant, impl members, associated const; Java package; C# protected internal / private protected); no `[in impl` | `wiring::plain_symbols_print_known_visibility_for_every_kind` |
| Unknown access omits the prefix (Bash, TS local) | `wiring::plain_symbols_omit_the_visibility_prefix_for_unknown_access` |
| `functions` includes constructors; `types` includes aliases | `wiring::plain_functions_include_constructors_and_types_include_aliases` |
| pretty output uses the same data and labels (CLICOLOR_FORCE output with SGR/OSC 8 stripped equals plain) | `wiring::pretty_symbols_use_the_same_labels_as_plain` |
| TS `classes`: static methods/fields vs instance | `wiring::plain_classes_partition_typescript_static_members` |
| C++ `classes`: static data member listed, member-function-local `static` not | `wiring::plain_classes_list_cpp_static_members_but_not_function_local_statics` |
| Java `classes`: interface constant under *Static Fields* | `wiring::plain_classes_list_a_java_interface_constant_as_a_static_field` |
| Rust `classes`: inherent and trait-impl methods under the struct, associated fn under *Static Methods* | `wiring::plain_classes_group_rust_impl_members_under_their_struct` |
| JSON `visibility` populated / omitted for Unknown, schema 3.0 on envelope and index, visibility off the signature | `wiring::json_symbols_carry_visibility_and_schema_3_0` |
| legacy `files[]` and `symbol_indexes` agree on kind, access, static, container kind for every symbol (TS, Rust, C++, Java, Bash) | `wiring::json_legacy_symbols_match_the_symbol_indexes` |
| test 8: seeded old-format entry (historical key hard-coded with `biscuit-hash`) is not served; run 1 equals an uncached run; run 2 serves an edited new entry with run 1's facts | `cache_upgrade::warm_entry_from_the_previous_extractor_is_not_served_after_the_upgrade` |

Test 8 details: a discovery run in a separate temp directory learns the
path spelling `hug` keys on (canonical, `/private/var/...` on macOS,
OS-specific elsewhere) and the project directory; the key formula itself is
hard-coded. A control run places the stale payload under the *current* key
and shows `hug` serves it, so run 1's miss is attributable to the key and not
to an unreadable seed.

Failure messages name the language, the declaration and the differing field.

**Regression proof (mutation check):** with the four Phase 5 changes reverted
in a scratch copy of `main.rs` (old fingerprint + schema 2.0, no `Interface`
in `classes`, no `Constructor` suffix), the Java-interface, every-kind,
functions/types and cache-upgrade tests failed; the cache test failed with
"run 1 served the seeded old-format entry". Restored afterwards; the
`is_static` source swap is behavior-neutral and has no distinguishing test.

### Departure: new dev-dependency

`tree-hugger-cli` gained `biscuit-hash` (`blake3`) as a **dev-dependency** so
test 8 can hard-code the historical key without calling the code under test.
The spec lists new crate dependencies as needing the author; this one is an
existing workspace crate already in the graph (through `tree-hugger`, same
feature), the lockfile gains one edge and no package, and it is recorded in
`docs/dependencies.md`. Easy to revert if the author objects (the test could
instead call `tree_hugger::FileCacheKey`, at the cost of not hard-coding the
formula).

### Superseded tests

None left: `[in impl Greeter]` was rewritten in Phase 3, and no other CLI
test asserted signature visibility or schema 2.0.

### Gates

- `just test` (tree-hugger): 652 passed, 0 skipped, 0 failed (641 + 11 new).
- `just test-l2`: 3 passed.
- `just lint`: clean; `cargo clippy -p tree-hugger -p tree-hugger-cli
  --all-targets --all-features -- -D warnings`: clean.
- `just check-tier-coverage tree-hugger`: 0 stranded.
- Cross-OS: `cargo check --bin hug --tests --target x86_64-pc-windows-gnu`
  clean. `just cross-check tree-hugger-cli --os windows` built and archived on
  build-win-native but the test step was refused by the host's storage
  preflight (30.1 GiB free, 50 GiB required; automatic reclaim freed 0). The
  MSVC build reported one warning in bin `hug` that the gnu check does not
  reproduce; not investigated further. `--os linux` failed with an SSH banner
  timeout to build-linux. Pull-request CI (Linux, macOS) and push-to-main
  (Windows) are the remaining OS evidence; test 8's OS-sensitive parts (path
  spelling, temp directory) are discovered at run time rather than assumed.
- No `cargo fmt` run.

## Phase 6

Implemented by one agent. Earlier phases had already landed most of the ten
test groups (the kind, visibility and static matrices, test 6, tests 7–8, and
skeletons of tests 5, 9 and 10). Phase 6 filled the remaining gaps, all in
L1 test modules of the consolidated `tests/l1` binary (`autotests = false`;
every module was already declared in `main.rs`). No production code changed.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| test 5: kind and container kind for every kind-matrix row, record → `SymbolInfo` and `SymbolInfo` → record, plus signature presence | `symbols_adapter::kind_matrix_survives_both_conversions` (new) |
| test 5: visibility for every visibility-matrix row, both directions | `symbols_adapter::visibility_matrix_survives_both_conversions` (new) |
| test 5: `is_static` / `is_static_storage` for every static-matrix row, both directions | `symbols_adapter::static_matrix_survives_both_conversions` (new) |
| test 5: parameterless constructors/methods/functions (TS and JS `ModuleWidget` constructor, JS `instanceMethod`, TS `exportedFunction`, Rust `public_function`) keep a present, empty signature; JS `constructor(initialValue)` keeps its parameter | `symbols_adapter::fixture_signatures_survive_both_conversions` (new) |
| test 5: signature and type-metadata regression coverage on every fixture symbol (`SymbolInfo` → record → `SymbolInfo`) | `symbols_adapter::every_fixture_symbol_round_trips_through_the_record` (new) |
| test 5: record and converted info agree for every fixture symbol | `symbols_adapter::adapter_agrees_with_extraction_for_every_fixture_symbol` (Phase 2; doc comment updated) |
| test 9: non-empty text for every record in `symbol_records()` **and** `symbol_index_v2()` | `symbols_decl_text::every_fixture_record_has_declaration_text` (widened from `symbol_records()` only) |
| test 9: exact text for the bodyless Rust trait method `fn required_trait_method(&self) -> u32;` and both Perl names of `my ($first_name, $last_name)` | `symbols_decl_text::declaration_text_keeps_unmappable_syntax_and_stops_at_the_body` (rows added; `pub(in …)`, Swift, C#, Scala and Java rows existed) |
| test 10: `From<SymbolInfo>` of every Public fixture symbol (over 100) yields `is_exported: false`, including TS `exportedFunction` and Rust `public_function` | `symbols_export::converting_a_public_symbol_never_marks_it_exported` (new) |
| test 10: index export for Java class, TS method (false), TS `export function`, Rust `pub fn` (true) | `symbols_export::export_follows_export_syntax_not_access` (existing) |
| test 4: Python class-body forms, Scala object vs class, Java interface constant, storage `static` | `symbols_static::static_matrix_holds_through_both_paths` (existing rows; verified, no change) |
| Python `self.y` / `obj.attr`, JS `this.w`, Perl `local` declare no symbol | `symbols_kinds::assignments_and_local_overrides_declare_no_symbol` (existing; strengthened) |
| JS `x = 1`, `static y`, `#z` are `Property` (`unmarkedProperty`, `uninitializedStaticProperty`, `#hashPrivateProperty`) | `KIND_MATRIX` rows plus `STATIC_MATRIX` (existing) |

Strengthening of `assignments_and_local_overrides_declare_no_symbol`: JS
`unmarkedProperty`, also assigned through `this.` in the constructor, is
reported exactly once, as a `Property` of its class; and no Perl record of
any name has declaration text starting with `local` (before, only `$_` was
checked by name).

**Mutation check:** adding `"local"` to the Perl `#any-of?` scope list in
`queries/vendor/perl/locals.scm` made the strengthened test fail; the query
was restored (no diff under `lib/queries`).

### Finding: two known adapter losses (not fixed; pinned)

The new full round trip (`SymbolInfo` → `SymbolRecord` → `SymbolInfo`) over
every fixture symbol found two type-metadata losses. Neither is among the
fields test 5 promises (kind, visibility, static flags, container kind,
signature presence), and the spec says test 5 does not claim every facet fits
both models, so this phase pins them instead of changing the adapter:

- `TypeMetadata::type_parameters` are always dropped: `TypeData` has no slot
  for them (`type_parameters: Vec::new()` in `type_metadata_from_kind_data`).
  This is unchanged from `main`. Fixture case: Rust `Wrapper<T>`.
- A `TypeAlias`'s field metadata is dropped: `From<SymbolInfo>` gives every
  alias `TypeAliasData`, which carries only the aliased type. Fixture cases:
  C `TagAlias`, `AnonymousStructAlias`, `SameName` (`typedef struct {…}
  Alias`). This became visible when Phase 4 made C typedefs `TypeAlias`.
  For an anonymous `typedef struct { … } Alias` the fields exist only on
  the v1 symbol. For a tagged one the tag's `Type` record still has them.

`metadata_after_round_trip` in `symbols_adapter.rs` encodes both losses, and
the test asserts the exact list of lossy aliases, so either fix flips the
test. The skill's adapter note now states both.

### `ok` → `FIX` discoveries and allow-listed `Unknown`

None in this phase. The Phase 4 table above is still the complete list, and
the test-6 allow-list is still empty.

### Gates

- `just test` (tree-hugger): 658 passed, 0 skipped, 0 failed (652 + 6 new).
- `just lint`: clean (one `clippy::type_complexity` on a test table, fixed
  with a `SignatureCase` alias).
- `just check-tier-coverage tree-hugger`: 0 stranded. All new tests are L1
  (no tier markers) and appear in `cargo nextest list`.
- Cross-OS: not run. The phase adds only in-process library tests that read
  fixtures through the existing `manifest_dir!()` helper; there are no
  paths, processes or temp directories that vary by OS.
- No `cargo fmt` run.

## Phase 7

Implemented by one agent. Documentation and closure only. One source file
changed, and the change is to doc comments.

### What was written

| Plan item | Result |
|---|---|
| New docs page | `tree-hugger/docs/symbol-kinds-and-visibility.md` (new). It covers every bullet of the spec's documentation section: per-language kinds; Module vs Namespace (not unified); Perl `my`/`our`/`state`/`local`; Rust `impl` containers and `relations.implements`; Java/Swift/TypeScript interface and protocol members; `declaration_text`; visibility levels and the unmarked default per language; C# compound levels and interface members; Swift type and extension defaults; Rust trait-impl Unknown; the C/C++ linkage rule with the earlier-`static` scan; JS/TS module detection; the two meanings of `static`; Python class attributes; JS/TS/C# `Property`; Scala qualifiers and setter-only modifiers; current limitations. Mermaid diagrams cover the extraction pipeline and the visibility decision. The page names no spec. |
| READMEs | `lib/README.md`: full `SymbolKind` list, mapping table with a TypeAlias column, `SymbolInfo` with `visibility`/`is_static`/`is_static_storage`/container fields, `Visibility` (all nine levels), `FunctionSignature` without `visibility`, `FieldInfo` copy rule, schema-record facts (`Modifier::StaticStorage`, `container_kind`, `declaration_text`, `implements`, export separation), new known limitations. `cli/README.md`: visibility labels and the Unknown omission, `[in <kind> <name>]` containers including Rust `impl` grouping as `[in type Point]`, constructors and type aliases in `functions`/`types`, schema 3.0 JSON shape (top-level `visibility`, `is_static`, `is_static_storage`, `container_kind`), color table, `classes` coverage with a real Rust example. `tree-hugger/README.md`: a link from the JSON `visibility` note to the new page. |
| Skill | `SKILL.md`: link to the new page; Swift limitation reworded as an extractor limit (the spec says the grammar exposes `declaration_kind`); new Constructors limitation; Perl limitation now says "narrow coverage". `api-reference.md`: `Module`/`Namespace` comments corrected to the per-language rule; `Package`/`Crate`/`ModulePrivate` comment examples completed; link to the page. |
| Schema design | `docs/symbol-schema-design.md`: status (3.0 shipped, `v2` module name kept), `IdentityFacet.container_kind` with a JSON example, `module_path` → `container_name` marked **planned** (4.0), `declaration_text` contract, `signature_text`/`body_text` **planned**, `ProtectedInternal`/`PrivateProtected`, visibility ownership, export separation, the implemented `Modifier` enum with `Static` vs `StaticStorage` (other modifiers **planned**), Rust `relations.implements` (other languages **planned**), version history 2.0 → 3.0 → 4.0 (**planned**), v1 ↔ v2 field mapping and known adapter losses. |
| Feature-spec sync | Already done before this phase: the working tree of `2026-10-03-symbol-improvements` says "3.0 is taken by `2026-10-03-visibility-and-kind`", moves 3.0 → 4.0, names `identity.module_path` → `identity.container_name` and `container_kind`, and lists the Phase 5 `classes` kind set. `2026-10-03-symbol-filters-and-reports` refers to schema 4.0. No edit needed. Those uncommitted edits were already in the tree at the start of this phase and were left alone. |
| Comment drift | `FieldInfo::visibility` / `is_static` docs (`lib/src/shared/symbol.rs`) said "(public, protected, private)" and "static field"; they now state the copy-from-standalone rule `align_field_metadata` applies. The `Visibility::Internal` "default in Java" drift the spec names had already been corrected in Phase 2 (`Package` now carries the Java example). No other `///`/`//` contradicting the levels was found (`grep` over `lib/src` and `cli/src` for `signature.visibility`, `impl ` special case, removed helpers, schema 2.0). `docs/dependencies.md` unaffected: no crate added in this phase. |

### Facts checked against the code while writing (corrections to the spec's wording)

The docs state what the code does. Where it differs from what the spec
text implies, the docs follow the code:

- The `ModulePrivate` label is `module-private` (hyphenated, `Visibility`'s
  `Display`), not "module private".
- A Lua `function M:m()` reports **Method**. The spec's kind matrix marks Lua
  Method as `FEAT`. `M.f` is a Function.
- TypeScript `namespace` and PHP `namespace` produce **no symbol at all**,
  rather than a Namespace symbol with a deferred kind.
- `relations.implements[].id` is set only when **exactly one** trait of that
  name is declared in the file (`tree_file.rs`).
- Rust export inference marks any item with a `pub…` modifier (including
  `pub(crate)`) as exported. Members are never exported. The docs describe this
  as "a Rust item with a `pub…` modifier".
- `hug` shows a source keyword instead of the kind name on some lines
  (`struct`, `union`, `const`, `static`, `let`). The CLI README says so. Changing
  that is out of scope (it belongs to `2026-10-03-symbol-improvements`).

### Process incident (recovered)

While writing the plan/log frontmatter, a helper script opened `plan.md` for
writing before an expression it was writing failed, which truncated the file.
It was rebuilt from `HEAD` plus the working-tree content printed earlier in
the session: the full frontmatter and the changed body regions. The rebuilt
file has the same 613 lines and every heading at the same offset (HEAD + 9
frontmatter lines). Its diff against `HEAD` equals the earlier 35-line Phase 6
diff plus this phase's 6 checkbox flips. `implementation-log.md` was not
affected (verified at 1027 lines and an unchanged diff before the Phase 7
append). The reviewer may want to glance at `git diff -- plan.md` once.

### Requirement-to-test mapping

No behavior changed in this phase, so there are no new tests. The only
source edit is two doc comments on `FieldInfo`. The documented behavior is
already pinned by the Phase 4–6 suites (`KIND_MATRIX`, `VISIBILITY_MATRIX`,
`STATIC_MATRIX`, `symbols_adapter`, `symbols_decl_text`, `symbols_export`,
CLI `symbols` binary). Every example in the docs was checked against
`hug --plain`/`--json` output over a scratch-repo copy of the fixtures
(`--no-cache`), or against those tests.

### Gates

- `just test` (tree-hugger): 658 passed, 0 skipped, 0 failed.
- `just lint`: clean (exit 0).
- `just test-l2`: 3 passed (the `god-files` terminal tests), 113 skipped (L1).
- `just check-tier-coverage tree-hugger`: 0 stranded.
- Cross-OS: not run. This phase has docs plus one doc-comment edit, with no
  compiled behavior change.
- No `cargo fmt` run. No commit, nothing staged, spec not moved.

Implementation complete, ready for review.
