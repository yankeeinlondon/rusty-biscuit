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
