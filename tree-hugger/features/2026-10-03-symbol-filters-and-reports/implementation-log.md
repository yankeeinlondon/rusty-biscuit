---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-03-symbol-filters-and-reports/spec.md"
plan: "tree-hugger/features/2026-10-03-symbol-filters-and-reports/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - tree-hugger/lib/src/file/extract/scope_dump.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/lib/tests/fixtures/symbols/test_files/go/point_test.go
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/test_point.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/point_test.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/python/conftest.py
  - tree-hugger/lib/tests/fixtures/symbols/test_files/javascript/point.test.js
  - tree-hugger/lib/tests/fixtures/symbols/test_files/javascript/point.spec.js
  - tree-hugger/lib/tests/fixtures/symbols/test_files/typescript/point.test.ts
  - tree-hugger/lib/tests/fixtures/symbols/test_files/typescript/point.spec.ts
  - tree-hugger/lib/tests/fixtures/symbols/test_files/rust/tests/point.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-03-symbol-filters-and-reports/scope-dump.md
skills_files_updated_during_phase_1:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/kinds.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/scope.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/scope_dump.rs
  - tree-hugger/lib/src/file/extract/static_flags/mod.rs
  - tree-hugger/lib/src/file/extract/visibility/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_async.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_containment.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_header.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_qualified_names.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/lib/tests/l1/symbols_scope.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/fixtures/symbols/bash.sh
  - tree-hugger/lib/tests/fixtures/symbols/c_sharp.cs
  - tree-hugger/lib/tests/fixtures/symbols/cpp.cpp
  - tree-hugger/lib/tests/fixtures/symbols/go.go
  - tree-hugger/lib/tests/fixtures/symbols/java.java
  - tree-hugger/lib/tests/fixtures/symbols/javascript.js
  - tree-hugger/lib/tests/fixtures/symbols/lua.lua
  - tree-hugger/lib/tests/fixtures/symbols/python.py
  - tree-hugger/lib/tests/fixtures/symbols/rust.rs
  - tree-hugger/lib/tests/fixtures/symbols/scala.scala
  - tree-hugger/lib/tests/fixtures/symbols/swift.swift
  - tree-hugger/lib/tests/fixtures/symbols/typescript.ts
  - tree-hugger/lib/tests/fixtures/symbols/zsh.zsh
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/lib/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/doc_comments.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_kind_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/fixtures/symbols/bash.sh
  - tree-hugger/lib/tests/fixtures/symbols/c.c
  - tree-hugger/lib/tests/fixtures/symbols/c_sharp.cs
  - tree-hugger/lib/tests/fixtures/symbols/cpp.cpp
  - tree-hugger/lib/tests/fixtures/symbols/go.go
  - tree-hugger/lib/tests/fixtures/symbols/java.java
  - tree-hugger/lib/tests/fixtures/symbols/javascript.js
  - tree-hugger/lib/tests/fixtures/symbols/lua.lua
  - tree-hugger/lib/tests/fixtures/symbols/perl.pl
  - tree-hugger/lib/tests/fixtures/symbols/php.php
  - tree-hugger/lib/tests/fixtures/symbols/python.py
  - tree-hugger/lib/tests/fixtures/symbols/rust.rs
  - tree-hugger/lib/tests/fixtures/symbols/scala.scala
  - tree-hugger/lib/tests/fixtures/symbols/swift.swift
  - tree-hugger/lib/tests/fixtures/symbols/typescript.ts
  - tree-hugger/lib/tests/fixtures/symbols/zsh.zsh
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
docs_updated_during_phase_3:
  - tree-hugger/features/_unscheduled/deferred-kind-precision/spec.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/lib/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
packages:
  - tree-hugger
  - tree-hugger-cli
---

# Implementation Log for 2026-10-03-symbol-filters-and-reports (7 phases)

## Phase 1

### 1.1 Prerequisite check (2026-10-05)

Ran in `tree-hugger/` on macOS, on the uncommitted symbol-improvements
working state of this worktree (ruling 1 default: proceed on top of it).

- `just test`: 833 tests run, 833 passed, 0 skipped.
- `just lint`: clean for `tree-hugger` and `tree-hugger-cli`.
- Present: `SymbolQuery` and `Capability` (`lib/src/file/query.rs`),
  `SymbolCache` (`lib/src/cache/symbol_cache.rs`), `render::Renderer`
  (`cli/src/render.rs`), `docs/symbol-reports.md`.
- `EXTRACTOR_REVISION` is **`"6"`**, not `"5"` as the plan expected (the
  symbol-improvements Phase 4 cache work already bumped it). Phase 2's bump is
  therefore to `"7"`, and `cache_upgrade` gains no hard-coded case for
  revision 6: revision 6 introduced the computed `SymbolCache` key, which a
  test cannot hard-code (see the tree-hugger skill); the revision is pinned as
  a key input by `symbols_cache::each_key_input_and_stored_identity_decides_a_miss`.
- `2026-10-03-visibility-and-kind` is in `fixes/_completed/`; its fixtures and
  matrices (`symbols_kinds`, `symbols_visibility`, `symbols_static`) are in
  the `l1` binary and green.

Rulings 1-10 of the plan are taken at their defaults; nothing found here
argues for overriding any of them (ruling 2, keeping `SchemaVersion::V4_0`,
stays the author's call before Phase 2).

### 1.2 Scope dump

The dumper is a test-only module, `lib/src/file/extract/scope_dump.rs`
(`#[cfg(test)]` in `extract/mod.rs`), because the classifier needs the
declaration node, which only crate-internal code can reach
(`TreeFile::definitions` → `ExtractedSymbol::declaration`). It holds a
prototype `proposed_scope` and two L1 tests in the library's unit-test binary:

- `file::extract::scope_dump::every_fixture_declaration_gets_a_proposed_scope`:
  classifies every declaration of every `tests/fixtures/symbols/*` file;
  asserts each of the 16 languages has declarations and an `Item`, every
  `Parameter` is `Local`, and no member kind (Method, Constructor, Field,
  Property, EnumVariant) is `Item`.
- `file::extract::scope_dump::spec_cases_missing_from_the_fixtures_get_their_scope`:
  inline snippets for spec cases no fixture holds yet (Bash/Zsh nested
  functions, Rust inline `mod` and `impl` inside a function, JS object literal
  and class expression, Python class and function inside a function, Scala
  `given … with`, C++ namespace and local struct, Java local class), each name
  asserted to its expected scope.

`TREE_HUGGER_SCOPE_DUMP=<file>` writes the Markdown dump (fixtures, then the
snippets). It was written to `scope-dump.md` in this feature directory with:

```sh
TREE_HUGGER_SCOPE_DUMP=$PWD/features/2026-10-03-symbol-filters-and-reports/scope-dump.md \
  cargo nextest run -p tree-hugger --lib scope_dump
```

`is_local_scope_kind` (`extract/declaration.rs`) became `pub(crate)` so the
prototype reuses it; no behavior change.

Prototype rules (the input to Phase 2.2):

1. `Parameter`, and a C++ template type parameter (captured as a `Type`), is
   `Local`.
2. A Go `method_declaration` and a Lua name written `M:m` / `M.f` / `M.x = …`
   (name under `method_index_expression` / `dot_index_expression`) is
   `Member`: each names its owner wherever it is written.
3. Otherwise walk out from `member_slot(declaration)`: the first function
   node (`is_function_node_kind`) or local body (`is_local_scope_kind`) gives
   `Local`; the first scope owner gives `Member`; the file root gives `Item`.
   Scope owners are `is_member_owner_kind` minus the body kinds (`*_body`,
   `block`, `declaration_list`, `field_declaration_list`, `template_body`),
   plus C/C++ `enum_specifier`, Go `struct_type`/`interface_type` and Java
   `annotation_type_declaration`. Bodies are walked past, so the shared
   `declaration_list` is decided by the declaration that owns it (`mod_item`,
   `namespace_definition`, C# namespace → keep walking → `Item`;
   `impl_item`, `trait_item`, C# class → `Member`).

### 1.3 Dump review

Totals over the fixtures: every language has Item and Local rows; Bash, Zsh
and Perl have no Member rows (no member constructs extracted). The prototype
agrees with today's `is_local_declaration` on every fixture declaration; the
only disagreement is the snippet below (Rust module inside a function).

Reviewed per language, including the plan's named cases:

- **Rust**: inline `mod` contents `Item` (`outer_module`, `crate_inner_module`,
  `geo::Coord`); `impl` and `trait` contents `Member`, including `impl Trait
  for String`; `static` inside a function `Local`. *Correction recorded:* a
  module declared inside a function body is `Local` and so is everything in
  it (`fn g() { mod inner { pub fn h() {} } }` → `h` is `Local`). Today's
  `is_local_declaration` returns false for `h` because it stops at
  `declaration_list` (listed in `is_member_owner_kind`), so `h` currently gets
  a visibility. Phase 2.2 must resolve `declaration_list` by its parent there
  too, or the two walks will disagree.
- **Go**: methods are `Member` by rule 2; receivers are `Local` parameters.
  Go methods have **no container** today (`container_name` is empty), which
  the Containment matrix requires (spec: "Go receiver methods").
- **Python**: decorated methods (`decorated_definition` wrapper) and class
  attributes are `Member`; a class and a function inside a function are
  `Local`, the nested class's methods `Member`.
- **Lua**: `function M:m()` (Method) and `function M.f()` / `M.x = …`
  (Function / Variable) are `Member`; none of them has a container today. The
  dump marks `M.f` as "function at member scope": its kind stays `Function`.
  Chunk-level `local` and `do … end` locals are `Item` (not in a function).
- **C/C++**: preprocessor and `template_declaration` wrappers keep membership
  (`DeclaratorFieldForms::cache`); enumerators (kind `Variable`) are `Member`
  of their `enum_specifier`; namespace contents `Item`; struct inside a
  function `Local`, its fields `Member`. *Correction recorded:* the C++
  template type parameter `T` (`type_parameter_declaration`, kind `Type`) was
  `Member`; it is now `Local` (rule 1).
- **C#, Java**: namespace/package contents `Item`; local class inside a method
  `Local`, its fields `Member`.
- **Scala**: class, trait, `object` (companion and standalone) contents
  `Member`; `given … with` members `Member` (snippet; no container today).
- **JavaScript/TypeScript**: class members `Member`; object-literal methods
  and class-expression methods `Member` (snippet; no container today); a class
  expression or object bound inside a function is a `Local` variable.
- **Swift**: class/struct/enum/protocol/extension contents `Member`;
  computed-property bodies `Local` (via `is_local_scope_kind`).
- **PHP**: class, interface, trait and enum members (incl. `const`) `Member`.
- **Bash/Zsh**: nested function `Local` (snippet), top-level functions `Item`.
- **Perl**: `package` blocks do not change scope; subs `Item`, `my` in subs
  `Local`.

Observations handed to Phase 2 (not scope rules, but they affect qualified
names and containment):

- Containers on locals differ by language: C#, Java and C++ locals carry the
  enclosing class or namespace (`localValue` in `PublicWidget`), Perl locals
  carry the enclosing sub, most others carry none. The in-file qualified name
  of a local depends on this.
- C enumerators are kind `Variable`, not `EnumVariant` (existing limit).

### 1.4 Fixture and matrix skeleton

- `lib/tests/l1/symbols_report_matrices.rs` (new, declared in `l1/main.rs`):
  the seven matrices (`SCOPE_MATRIX`, `QUALIFIED_NAME_MATRIX`,
  `CONTAINMENT_MATRIX` → Phase 2; `FLAGS_MATRIX`, `METRICS_MATRIX`,
  `KIND_COVERAGE_MATRIX` → Phase 3; `SELECTION_FILTER_MATRIX` → Phase 4), one
  `Pending { phase }` row per language, listed in `PENDING_MATRICES`.
- `symbols_coverage::every_language_has_one_row_in_each_report_matrix`: exactly
  one row per language in each of the seven (fails closed on a missing or
  repeated row) and every pending row names a library phase (2-4).
- **Departure from the plan's checkpoint wording.** The plan's checkpoint
  says "guard fails only on the unfilled matrix rows", but the phase must end
  with `just test` green. The guard therefore fails on a *missing* row, not on
  a `Pending` one; a filling phase moves its matrix into its own module with
  a real row type, and the last filler deletes `Pending`, which turns any row
  left unfilled into a compile error.
- Test-file fixtures under `lib/tests/fixtures/symbols/test_files/` (one per
  file-name rule: `go/point_test.go`, `python/{test_point,point_test,conftest}.py`,
  `javascript/point.{test,spec}.js`, `typescript/point.{test,spec}.ts`,
  `rust/tests/point.rs`), each with a test and a non-test helper, listed in
  `symbols_fixtures::TEST_FILE_FIXTURES` (rule label → path).
  `symbols_coverage::every_test_file_rule_has_a_clean_fixture` asserts each
  parses cleanly and declares at least two functions. The Rust fixture has no
  `Cargo.toml` beside it on purpose (a stray manifest would be discovered as a
  package by repo tooling); in place it reads as a test because its package
  root is `tree-hugger/lib` and the path starts with `tests/`. Phase 3's
  non-test assertion copies it into a scratch package (`Cargo.toml` +
  `src/latest.rs`).

### Verification

Requirement → test:

| Requirement | Test |
|---|---|
| prerequisites green | `just test` (833 before, 837 after), `just lint` |
| scope dump covers all 16 languages; parameters `Local`; no member kind at `Item` | `file::extract::scope_dump::every_fixture_declaration_gets_a_proposed_scope` |
| spec scope cases absent from fixtures | `file::extract::scope_dump::spec_cases_missing_from_the_fixtures_get_their_scope` |
| every report matrix has exactly one row per language | `symbols_coverage::every_language_has_one_row_in_each_report_matrix` |
| every test-file rule has a clean fixture with a test and a helper | `symbols_coverage::every_test_file_rule_has_a_clean_fixture` |

Gates: `just test` in `tree-hugger/` → 837 run, 837 passed, 0 skipped;
`just lint` clean (one `clippy::type_complexity` on the snippet table fixed
with type aliases); `just check-tier-coverage tree-hugger` → 0 stranded.
All new tests are L1 (no tier marker in any path segment), in the library's
unit-test binary or the declared `l1` binary. No pre-existing failures. No
cross-OS run: the phase adds only test code and fixtures with portable names
and `tempfile`/`manifest_dir!` paths. The Input Robustness Matrix does not
apply (no reader or format code changed).

## Phase 2

Always-computed metadata: scope, the `async` modifier and in-file qualified
names, with the record and cache changes they need. `EXTRACTOR_REVISION` is
now **`"7"`**.

### 2.1 Schema additions

- `SymbolScope { Item, Member, Local }` (`schema_v2/kinds.rs`, public) and a
  required `SymbolRecord::scope`.
- `identity.qualified_name` changed from `Option<String>` to a required,
  non-empty `String` (custom `non_empty_string` deserializer): the in-file
  name always exists (at least the bare name).
- **Addition not named by the plan:** `identity.is_namespace_rooted: bool`
  (required). Phase 4's post-lookup prefix must skip names that already start
  at a declared namespace/package (`com.example.Point`, spec B5), and that is
  a per-record fact (a C++ name is rooted inside `namespace ns {}`, not outside
  it). Adding it now keeps the revision bump to this phase.
- `visibility.is_test: bool` (required; always `false` until Phase 3).
- `metrics: Option<MetricsFacet>` with `#[serde(default, deserialize_with =
  "present")]`: absent = not requested, `null` is an error. Inside it
  `parameters: Option<u32>` and `has_docs: Option<bool>` use a `nullable`
  deserializer (required key, `null` = unsupported); `deny_unknown_fields`.
  Ruling 5 as planned.
- `Modifier::Async` (already in the enum) is now filled.
- Ruling 2 default kept: records stay `SchemaVersion::V4_0`; the revision bump
  invalidates old entries.
- v1 `SymbolInfo` gained `scope`, `qualified_name`, `is_namespace_rooted` and
  `is_async`, so the adapter round-trips them (an empty v1 `qualified_name`
  converts as the bare name). `From<SymbolInfo>` keys the ID on the qualified
  name.

### 2.2 Scope classifier

`lib/src/file/extract/scope.rs::symbol_scope`, the Phase 1 prototype rules
plus one correction found while wiring it: **members of unnamed owners**
(Java anonymous class, PHP `new class`, JS object literal / class
expression, Scala `given … with`) are `Member` (`is_unnamed_type_owner` is an
owner). The prototype walked past an anonymous class's `class_body` into the
enclosing method and called `run()` in `new Runnable() { … }` `Local`, which
would also have dropped its access (Phase 1's dump had no such case).

`DeclarationContext::is_local: bool` was replaced by `scope: SymbolScope`
(`is_local()` helper) and `is_local_declaration` deleted, so visibility and
`static` readers and the report scope cannot disagree. The one behavior change
is the Phase 1 correction: everything inside a Rust `mod` declared in a
function is now local (no access level).

The test-only `extract/scope_dump.rs` prototype was deleted; its snippet cases
moved to the public `symbols_scope` test. `scope-dump.md` stays as the Phase 1
review artifact (it can no longer be regenerated).

### 2.3 Qualified-name builder and IDs

`lib/src/file/extract/qualified_name.rs::qualified_names` runs once per file
after every symbol is materialized (so it does not depend on the kind
selection). Rules (documented in `docs/symbol-kinds-and-visibility.md`
"Qualified names"):

- prefix = nearest enclosing extracted naming symbol (function, method,
  constructor, type, class, interface, enum, enum variant, trait, module,
  namespace), memoized; otherwise a Rust `impl` self type, a TS
  `internal_module`, a named but unextracted function node (Java/C#
  constructor), or `ancestor_container` (Swift extension, …); the walk stops
  at an unnamed owner; at the root the nearest preceding file-level
  `package`/`namespace` statement roots the name.
- **Locals include their function** (`PlainWidget.instance_method.self`),
  unlike `container_name`, which for Java/C#/C++ locals is the class. This was
  chosen so a local never reads like a member.
- Separators: `::` Rust/C++/Perl; PHP `\` after a namespace and `::` after a
  type; Lua `:` before a method; `.` elsewhere. Perl package names are
  absolute.
- `stable_symbol_key` gets the qualified name; the sibling ordinal is now per
  `(kind, qualified_name)` (was `(kind, container, name)`).

**Containers changed for Go and Lua** (Containment matrix, Phase 1 finding):
a Go `method_declaration` has its receiver's base type as container
(`SameFileType`), a Lua `M.f`/`M:m`/`M.x = …` has its table
(`ContainerLookup::SameFileTable`). Go receivers/params/locals stay
container-less, as in any Go function.

`async`: `lib/src/file/extract/modifiers.rs::is_async_declaration`, reading
the anonymous `async` token per language (Rust, JS/TS incl. a name bound to an
async function, Python, C#, Swift); every other language returns `false`.

`EXTRACTOR_REVISION` 6 → 7 (its doc comment also claimed the cache key holds
no query hash or grammar version; that has been false since revision 6 and was
corrected). `cache_upgrade` gained
`entry_from_extractor_revision_six_is_not_served_after_the_repairs` with a new
`OldKey::Computed` key formula: it rebuilds the computed `SymbolCache` key
from the public `FingerprintInputs` with only the revision changed, after
asserting the formula reproduces the current key. Earlier cases are
unchanged.

### 2.4-2.7 Per-language fixtures and matrices

Fixture additions (appended in a "Scope, async and qualified names" block):
async declarations (Rust, TS, JS, Python, C#, Swift), same-named methods on
two types (TS, JS, Go, Python, Java, C++, Swift, Scala, Lua; C#/PHP/Rust
already had one), a Rust inline module with an `impl` plus a type and module
inside a function, C++ nested namespaces, Bash/Zsh nested functions. No
vendored query needed editing.

New L1 modules (all in the declared `l1` binary, no tier markers):

| Requirement | Test |
|---|---|
| Scope per language, Item/Member/Local each asserted, parameter `Local` where extracted, n/a proven empty | `symbols_scope::scope_matrix_holds_through_both_paths` |
| Parameters always Local, member kinds never Item, every fixture | `symbols_scope::parameters_are_local_and_member_kinds_are_never_items` |
| JS object literal/class expression, Scala `given`, Java local + anonymous class, PHP anonymous class, Python/C++ types in functions | `symbols_scope::constructs_outside_the_fixtures_get_their_scope` |
| Qualified names per language, rooted where namespace syntax exists and never otherwise, same-named methods get distinct names and IDs | `symbols_qualified_names::qualified_name_matrix_holds_through_both_paths` |
| Every name ends in its own name; IDs unique per file | `symbols_qualified_names::every_qualified_name_ends_in_its_name_and_ids_are_unique` |
| Repeated locals: same name, distinct IDs | `symbols_qualified_names::same_qualified_names_get_distinct_ids` |
| Two path spellings of one file → same IDs | `symbols_qualified_names::two_spellings_of_one_file_give_the_same_ids` |
| Containment (Rust impl, TS classes, Go receivers, Python classes, Lua tables, …), via a containment-only query | `symbols_containment::containment_matrix_links_members_to_their_containers` |
| `async` exactly on each fixture's async declarations; none where no syntax | `symbols_async::async_is_set_exactly_on_each_fixtures_async_declarations` |
| async forms outside fixtures (trait fn, fn expression, class field, object method, decorated, C# local fn, Swift protocol) and negatives (async block) | `symbols_async::async_forms_outside_the_fixtures` |
| adapter round trip of the new fields, ID from qualified name | `symbols_adapter::scope_qualified_name_and_async_survive_both_conversions`, `the_record_id_derives_from_the_qualified_name`, extended `serialized_record_round_trips_the_new_fields`, `adapter_agrees_with_extraction_for_every_fixture_symbol` |
| reader shapes of the new record fields (metrics facet incl. null/`{}`/missing/unknown/duplicate metric) | `symbols_adapter::new_record_fields_read_per_shape` |

`symbols_report_matrices` lost `SCOPE_MATRIX`, `QUALIFIED_NAME_MATRIX` and
`CONTAINMENT_MATRIX`; `symbols_coverage::every_language_has_one_row_in_each_report_matrix`
now checks the four filled matrices (scope, qualified names, containment,
async) plus the four pending ones (pending rows must name phase 3 or 4).
`FLAGS_MATRIX` remains pending for deprecation/test only; `async` has its own
`ASYNC_MATRIX` because it is computed in this phase.

The "library-only context-supplied names" cell of the spec's qualified-name
matrix is Phase 4 (4.3, naming context); not attempted here.

### 2.8 Superseded expectations

Rewritten: Go method and Lua table-member rows in `symbols_kinds`,
`symbols_visibility` and `symbols_header` (container `None` → receiver type /
table). Goldens regenerated with `HUG_UPDATE_GOLDEN=1` after reviewing the
diff: `symbols.plain` changes only three lines (`[in variable M]` on two Lua
functions, `[in type Widget]` on a Go method); `symbols.json` adds `scope`,
`is_namespace_rooted`, `is_test` per record, changes `qualified_name` /
`stable_key` and the `+rev.7` stamp, and adds the same three containers.

### 2.9 Cache wiring and robustness

- Scope, qualified name, namespace root and `async` ride in the stored
  records; `analysis::coverage::project` needs no change because they are
  always computed (the `projected_results_*` tests stay green).
- **Finding: `hug` partitioned its cache by package root**
  (`<temp>/tree-hugger/cache/<pkg root>/…`), so two scan roots could never
  share an entry, contradicting the spec's "one cache entry" requirement. The
  CLI now opens one directory, `default_cache("hug")` (`CACHE_PROJECT_ID`);
  the key already names the file by identity. Old per-root directories are
  simply orphaned (clearing the cache is always safe). New test
  `cache_lookup::a_run_from_another_scan_root_hits_the_same_entry_with_the_same_ids`
  (root run misses, sub-directory run is a `1 hits … parsed 0 files` hit,
  one entry, same IDs and qualified names, different `file_path`).
- A stored record carrying a `metrics` facet is `CacheMiss::Inconsistent`
  (no capability computes metrics yet; Phase 3 replaces this with a
  capability check): `symbols_cache::covered_kind_lists_are_served_only_in_canonical_form` extended.
- `cache_robustness` gained five record columns walked through every shape
  from a real entry via `hug --json-full`: `modifiers (async)` (absent/`[]`
  served as not async; else rejected), `scope`, `identity.qualified_name`,
  `identity.is_namespace_rooted`, `visibility.is_test` (control served; every
  other shape rejected). The capability-field cells (`is_deprecated`,
  `metrics.*` through `hug`) are Phase 3's 3.8 as planned; their serde shapes
  are pinned at library level by `new_record_fields_read_per_shape`.
- Code-smell grep: no `#[serde(default)]` on `scope`, `qualified_name`,
  `is_namespace_rooted`, `is_test`; `metrics`' `default` is the deliberate
  "absent = not requested" and is paired with `present` so `null` is an error;
  no `unwrap_or_default`/`.ok()` on any decode of them.

### Docs and skill

`docs/symbol-kinds-and-visibility.md` (Scope, Qualified names, `async`, Go
and Lua containers, two new limitations), `docs/symbol-schema-design.md`
(additive fields, always-computed vs planned capabilities, ID change, metrics
absence semantics), `docs/cache-design.md` (one `hug` directory, scan root and
naming context are not key inputs, new required record fields, metrics
inconsistency), `docs/symbol-reports.md` (JSON example), `lib/README.md`,
`.claude/skills/tree-hugger/SKILL.md` and `api-reference.md`.

### Verification

- `just test` (tree-hugger): 850 run, 850 passed, 0 skipped (837 before the
  phase; the 13 new tests are listed above plus the cache tests).
- `just test-l2`: 6 run, 6 passed.
- `just lint`: clean (two `type_complexity` hits in new tests fixed with type
  aliases).
- `just check-tier-coverage tree-hugger`: 0 stranded.
- No pre-existing failures; nothing skipped.
- Cross-OS: `just cross-check tree-hugger-cli --os windows` did not run: the
  `build-win-native` rig failed `_storage_preflight` (free space below the
  required 50 GiB after automatic reclaim). Nothing was deleted on that host.
  The OS-sensitive pieces are the shared cache directory, the computed
  revision-6 key (uses the entry's `file` with `\` → `/`, as `entry_key`
  does) and the two-scan-roots test (relies on the existing portable path
  projection); CI's Windows leg on `main` will cover them.

## Phase 3

Capability-computed metadata: `Capability::Flags` (`visibility.is_test`,
`visibility.is_deprecated`) and `Capability::Metrics` (the `metrics` facet).
`EXTRACTOR_REVISION` is now **`"8"`** (see "Doc-comment detection" for why a
second bump was needed).

### 3.1 Capability plumbing

- `Capability::Flags` and `Capability::Metrics` (appended to the enum, so
  existing serialized capability lists stay ascending), each requiring only
  `Declarations`; `Capability::ALL` has 8 entries, so `complete_index()` (and
  `hug --json-full`) computes both.
- **Departure from ruling 3:** one `Metrics` capability computes both metrics
  instead of carrying a requested metric set. Both are read from the same
  declaration node while the record is built (no extra pass, no extra query),
  so a set would add API and cache-coverage surface for no saved work.
- Both are computed inside `TreeFile::records_from` (`RecordFacts`), for
  wanted kinds only; no new pass and no `completed_passes` entry. Both need the
  syntax tree (attributes, the parameter-list node), so
  `record_passes_suffice` says no and a cache widening by either is
  `WidenedCapabilities { parsed: true }` (a partial hit, never a miss).
- `coverage::project` resets flags and drops `metrics` when not requested.
- `StoredEntry::validate` (replacing the Phase 2 blanket rule): `Inconsistent`
  for a `metrics` facet without `metrics`, a `true` flag without `flags`, and a
  record without `metrics` under `metrics`.

### 3.2 Attribute reader

`extract/attributes.rs::declaration_attributes` returns
`SyntacticAttribute { name, arguments }` (full syntactic name with whitespace
removed, raw argument text without delimiters) for Rust outer attributes, C++
`[[…]]` (with `prefix::`), Java and Scala annotations, C# attribute lists
(climbing `variable_declarator`/`variable_declaration`), Python decorators
(name or call) and Swift attributes. It populates nothing in the record's
`attributes` facet (asserted empty by `symbols_flags`).

### 3.3-3.6 Flags, metrics and fixtures (all 16 languages)

`extract/flags.rs` implements the spec tables literally (`is_deprecated`,
`has_test_attribute`, `is_in_rust_cfg_test` incl. `#![cfg(test)]` in a module
body or file, `is_test_file` per ruling 4: nearest ancestor directory with a
`Cargo.toml`). `extract/metrics.rs::symbol_metrics`.

**Doc-comment detection rewritten (records change, revision 7 → 8).** The
existing detection, which `has_docs` and the doc-tag deprecation rules
consume, was wrong in ways the spec's "ordinary comments do not count" rule
exposes:

| Before | After |
|---|---|
| Python read the *preceding* `expression_statement` (a method took its class's docstring; a function lost its own) | the docstring in the `def`/`class` body |
| JS/TS/Java/PHP/Scala/C/C++/C#/Swift: any adjacent comment was a doc | doc forms only (`is_doc_comment_text`, table in `docs/symbol-kinds-and-visibility.md#documentation`) |
| Rust `//!` attached to the next item | skipped (it documents the enclosing module) |
| C#/Swift/C `///` cleaned to `/ text` | delimiters removed |
| `/** */` before `const f = () => …` missed (context-less name, wrapper not climbed) | wrappers climbed (`export`, `lexical_declaration`, `variable_declarator`, `template_declaration`); a context-less non-`Local` name is documented before its declaration |
| C/C++ function docs read from the `function_declarator` (never found) | from `declaration_for_context` |
| Lua/Bash/Zsh: ordinary comments became docs | `None` (spec: no doc detection there) |
| Go: any earlier comment, across blank lines | the group touching the declaration |

The goldens changed only in `raw_doc` values that were wrong before (file
header comments no longer attach to the first declaration; the Python class
docstring lands on its class; Go's file comment documents `package`).

**Parameter extraction fixes found by the metrics matrix** (these also fix
signatures): Java `int... rest` (spread parameter named through its
declarator), C# `params int[] rest` (the grammar spells it as three list
children), a lone unparenthesized JS/TS arrow parameter (`x => x`).
`declared_parameters` returns `None` when no list exists (`def area: Int`).
`declared_parameter_count` counts unnamed C/C++/Go parameters (`int f(int,
char *)` = 2, `(void)` = 0), a documented signature limit the metric must not
inherit.

Fixtures: each primary fixture gained a "Flags and metrics" block (documented
pair, undocumented function under an ordinary "deprecated" comment, the
language's deprecation markers incl. qualified forms and look-alikes, test
attributes, `test_helper`, `cfg(test)`/`cfg(any(test, …))`, variadic and
receiver functions). Lua's block is placed before the final `return`.

| Requirement | Test |
|---|---|
| `is_deprecated` / `is_test` exactly per language, decoys false, attribute facet empty (isolated copy) | `symbols_flags::flags_are_set_exactly_by_each_languages_markers` (`FLAGS_MATRIX`) |
| file-name rules mark helpers too; renamed copy is not a test | `symbols_flags::test_file_rules_mark_every_declaration` |
| Rust `tests/` rule needs a Cargo package; `src/tests/`, `latest.rs`, manifest-less `tests/` are not; in-place Rust fixture all-test | `symbols_flags::the_rust_tests_directory_rule_needs_a_package` |
| `#[tokio::test]`, `#![cfg(test)]` (file, module), `cfg(all(test,…))` not, qualified Java/C# tests, `[ObsoleteAttribute]`, Swift `@available(*, unavailable)` not, TS `// @deprecated` and mid-line tag not, Go non-paragraph `Deprecated:` not, `@functools.deprecated` not, C++ variable | `symbols_flags::flag_forms_outside_the_fixtures` |
| `parameters`/`has_docs` per fixture function; non-functions `None`; unsupported languages `None` everywhere | `symbols_metrics::metrics_match_each_fixture_function` (`METRICS_MATRIX`) |
| arrow param, Python docstring ownership, empty/inner/block Rust docs, C/C++ Doxygen vs plain, Go detached group, unnamed C/C++/Go params, `(void)` | `symbols_metrics::metric_forms_outside_the_fixtures` |
| `--undocumented`-style selection over `.rs` + `.sh` lists only Rust (checkpoint) | `symbols_metrics::undocumented_selection_skips_languages_without_doc_detection` |
| `FEAT` cells unsupported vs landed; `—` kinds never in fixtures | `symbols_kind_coverage::deferred_constructs_are_unsupported_extraction`, `kinds_a_language_lacks_never_appear_in_its_fixture` (`KIND_COVERAGE_MATRIX`) |
| flag/metric request runs no other pass, unselected kinds get neither | `symbols_query::flags_and_metrics_run_with_the_declarations_only` |
| projection of flags and metrics (every fixture, `multi/`) | `symbols_cache::projected_results_*` (+2 cells, non-vacuity counters) |
| flag-only and metrics widening parse once, no diagnostics | `symbols_cache::widening_capabilities_runs_only_the_missing_passes` (+2 rows) |
| Inconsistent cross-checks (facet / flag without capability, missing facet under it) | `symbols_cache::covered_kind_lists_are_served_only_in_canonical_form`, `flags_and_metrics_are_served_only_under_their_capabilities` |

**Kind-coverage finding:** three `FEAT` cells already extract precisely as a
side effect of earlier work (C++ `using` aliases and namespaces, Lua `M:m`
methods). They are asserted as `landed`, not `unsupported`, and were
removed from the unscheduled `deferred-kind-precision` entry (pre-authorized
stale-entry correction; `features/_unscheduled/deferred-kind-precision/spec.md`). `no_construct` follows the fix's `—`
cells except C# Function (C# local functions are `Function`).

### 3.7 JSON always requests flags

`Declarations::query(format)` adds `Flags` when
`ReportFormat::always_computes_flags` (`Json`, `JsonFull`; the JSON Lines hook
for Phase 6). Tests: `report::the_json_envelope_holds_only_selected_records_and_ends_with_errors`
(`completed_capabilities == ["declarations","flags"]`, both flags boolean, no
`metrics`), `report::json_records_carry_computed_test_and_deprecation_flags`
(scratch Cargo package: `tests/`, `#[test]`, `cfg(test)`, `#[deprecated]`,
`test_helper`, `latest.rs`, ordinary "deprecated" comment),
`cache_lookup::a_json_run_after_a_listing_widens_the_entry_by_the_flags`
(checkpoint: `0 hits, 1 partial hits, 0 misses; parsed 1 files`, then both
commands full hits).

### 3.8 Cache robustness

`cache_robustness` gained four record columns walked through every shape from
a real `--json-full` entry: `visibility.is_deprecated` (control `true` served
under `flags`; every other shape rejected), `metrics` (control served; absent,
`null`, wrong type, bad members, `{}`, duplicate, trailing rejected),
`metrics.parameters` and `metrics.has_docs` (control and `null` served; absent
and every other shape rejected). The harness's `WrongType` edit writes
`"123"` for a numeric field (`123` is a valid `u32`). The completed
capabilities assertion now lists `flags` and `metrics`.

`cache_upgrade::entry_from_extractor_revision_seven_is_not_served_after_the_repairs`
covers the revision bump (probe: `documented_pair`'s docstring, absent under
revision 7).

Code-smell grep: no `#[serde(default)]` on `is_deprecated`, `is_test` or the
metric keys; `metrics`' `default` is the Phase 2 "absent = not requested"
paired with `present`; no `unwrap_or_default`/`.ok()` on their decode.

### Docs and skill

`docs/symbol-kinds-and-visibility.md` (new "Documentation", "Deprecation and
test flags" and "Metrics" sections, flow diagram, adapter note, five new
limitations), `docs/symbol-schema-design.md` (capability table, fact table,
metric definitions), `docs/cache-design.md` (consistency rules, record-field
table and counts, flags/metrics widening parses, `rev.8`),
`docs/symbol-reports.md` (`--json` always computes flags; `--json-full` every
capability), `lib/README.md`, `.claude/skills/tree-hugger/SKILL.md` and
`api-reference.md`.

### Verification

- `just test`: 864 run, 864 passed, 0 skipped (852 before the new modules).
- `just test-l2`: 6 passed. `just lint`: clean. `just check-tier-coverage
  tree-hugger`: 0 stranded.
- `just cross-check tree-hugger --os linux` and `tree-hugger-cli --os linux`:
  pass.
- Windows: `just cross-check tree-hugger --os windows` failed
  `_storage_preflight` again (automatic reclaim did not restore the required
  headroom); nothing was overridden or deleted. OS-sensitive new code: the
  Cargo `tests/` rule (`Path::ancestors`/`strip_prefix`, component compare)
  and its temp-dir test, which assumes no `Cargo.toml` above the temp dir.
- No pre-existing failures; nothing skipped.
