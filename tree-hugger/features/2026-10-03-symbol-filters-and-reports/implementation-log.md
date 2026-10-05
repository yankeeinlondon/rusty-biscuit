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
source_files_during_phase_4:
  - tree-hugger/lib/src/file/pattern.rs
  - tree-hugger/lib/src/file/naming.rs
  - tree-hugger/lib/src/file/filter.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_export.rs
  - tree-hugger/lib/tests/l1/symbols_fixtures.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_qualified_names.rs
  - tree-hugger/lib/tests/l1/symbols_visibility.rs
  - tree-hugger/lib/tests/l1/symbols_query_filters.rs
  - tree-hugger/lib/tests/l1/symbols_selection_filters.rs
  - tree-hugger/lib/tests/l1/symbols_report_matrices.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/tests/symbols/cache_lookup.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/order.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
docs_updated_during_phase_4:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/cli/src/selection.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/classes.rs
  - tree-hugger/cli/src/scanner.rs
  - tree-hugger/cli/tests/symbols/filters.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/prelude.rs
  - tree-hugger/cli/tests/symbols/classes.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/order.rs
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
docs_updated_during_phase_5:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
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

## Phase 4

`SymbolQuery` predicates, the smart-case matcher, the post-lookup
qualified-name prefix and the one library record filter.
`EXTRACTOR_REVISION` is now **`"9"`** (two return-type repairs found by the
selection-filter matrix, below).

### Design: unfinished vs finished results

The plan asked for predicates and naming inputs "applied after the cache
lookup, excluded from the cache key". Implemented as one rule:

- `run_query`, `coverage::project` and the new crate-private
  `SourceFile::analyze` return **unfinished** results: in-file
  `qualified_name`, every record of the effective kinds.
  `SymbolCache` analyzes and stores only unfinished indexes.
- `SymbolQueryResult::finish(query)` (crate-private; called by
  `TreeFile::query`, `SourceFile::query`, `SymbolCache::query` and
  `narrowed`) prefixes every name that is not `is_namespace_rooted` once,
  then applies the record filter. The result remembers the prefix
  (`name_prefix()`, public) and the applied predicates (private).
- `narrowed` strips the old prefix, projects, and finishes for the new
  query. A **filtered** result narrows only to the same predicates under the
  same prefix (a qualified-name predicate matched under that prefix; the
  dropped records cannot come back); an unfiltered one narrows to any
  covered query and is re-prefixed. `hug --prelude` relies on the
  unfiltered case.

### 4.1 Query predicates

`SymbolQuery` now keeps the requested kinds/capabilities separately from the
effective ones and recomputes the effective sets (`settle`) after every
builder call, so pushdown is independent of call order:

| Predicate | Pushes |
|---|---|
| `scopes`, `visibility` (repeatable, exact), `exported`, `in_container`, `name`, `exclude_name` | nothing |
| `exclude_tests` / `tests_only` (last call wins), `deprecated` | `Flags` |
| `undocumented` | `Metrics` |
| `is_async`, `returns`, `param_type` | kinds ∩ Function/Method/Constructor |
| `min_params` | function kinds and `Metrics` |

`union`/`without` (cache coverage) build predicate-free queries.
`SymbolQuery::matches(record)` exposes the filter.

**Departures from the plan's 4.1 list:**

- `exported(bool)` added (not in the plan's list): `--exported` is an
  existing CLI filter that spec C3 keeps and C10 puts on the one path, so
  Phase 5 needs it in the library filter.
- **No library file selector.** Files are chosen before analysis
  (`collect_files`, Phase 5.5); a per-record file predicate inside a
  one-file query would duplicate discovery and do no work. Phase 5 keeps file
  selection in discovery.

### 4.2 Smart-case matcher

Moved into the library (`lib/src/file/pattern.rs`): `NamePattern` (smart
case, substring unless `*` = whole-string glob, exact via `NamePattern::exact`
or a trailing `!` with `*` literal, case folded with `to_lowercase` on both
sides so no offset splits a character) and `glob_match` (whole-string,
case-sensitive; also the type globs). `is_qualified_for(language)`: a
pattern holding `#` or the language's separator matches the displayed
`qualified_name`, otherwise the bare name.

**Bug fixed on the way:** the CLI's `wildcard_match` treated a pattern with
no `*` as "prefix and suffix", so `hug symbols parse_width_spec!` also matched
`parse_width_spec_parse_width_spec`. The CLI's `wildcard_match` is deleted;
its one caller (`matches_symbol_filters`) calls `tree_hugger::glob_match`,
which keeps the CLI's case-sensitive semantics until Phase 5.2 replaces the
whole CLI filter path with the library predicates.

### 4.3 Post-lookup qualified-name prefix

`lib/src/file/naming.rs`: `NamingContext::new(root, prefix)` and the prefix
rule (`Naming::prefix_for`): a naming context's module path for a file below
its root (extension dropped; Rust `mod.rs`, root `lib.rs`/`main.rs` and
Python `__init__.py` add no segment), else the file's portable path from the
scan root (default: the file's own directory) plus `#`.

- **`portable_relative` moved from `cli/src/report.rs` into the library**
  (re-exported by `report.rs`), so the prefix and every CLI path use one
  projection. Its four unit tests moved with it.
- **The CLI now passes its scan root** (`declarations::run`), so `hug --json`
  shows `nested/python.py#MAX_SIZE` rather than a default file-name prefix.
  Prelude synthetic exports get the prelude file's prefix.
- `SymbolRef::qualified_name` (relations, export targets) **keeps the in-file
  name**: a referenced record may not be in the result, and its
  namespace-rootedness is then unknown. Documented.
- The library default (no scan root) prefixes the file name, per spec B5
  ("the standalone library defaults to the file's parent as its root"), so
  `symbol_index_v2()` / `TreeFile::query` results changed too.

### 4.4 Library record filter

`lib/src/file/filter.rs::RecordFilter::matches`: AND of every predicate; a
list matches when any value does; exclusions win. Shape predicates select
function-like kinds only; missing type text never matches; `param_type`
skips a Rust `self`; `has_docs`/`parameters` of `None` never match;
`in_container` needs a `container_name`, and a qualified pattern matches the
record's qualified name minus its own name and separator.

### 4.5 Selection-filter matrix

New module `symbols_selection_filters` (`SELECTION_FILTER_MATRIX`), replacing
the last `Pending` skeleton: `symbols_report_matrices.rs` is deleted and the
coverage guard lists eight filled matrices. Each row is a list of cells
`(region, predicates, expected in-file names | Err(reason) = matches
nothing)`, evaluated through `SourceFile::query` on an isolated copy over
the fixture's "Flags and metrics" block (`B`) or the whole file (`F`, for
async declarations, declared return types and named types). Every row must
cover all 14 predicate kinds (asserted). Every expected set was checked
against the fixture source.

**Bugs the matrix found (fixed, revision 8 → 9):**

| Finding | Fix | Regression test |
|---|---|---|
| Rust: a function without `->` reported return type `()` (an inferred type, not declared text; spec C7) | `extract_rust_return_type` reads the `return_type` field only | matrix `rust.rs B [Returns("*")]` = none; `cache_upgrade::entry_from_extractor_revision_eight_is_not_served_after_the_repairs` |
| Go: a method's **parameter list** was its return type (the receiver list counted as the first `parameter_list`); `hug` printed `Grow(amount int) (amount int)` | `extract_go_return_type` reads the `result` field only | matrix `go.go B [Returns("*")]`; golden `symbols.plain` |
| JS/TS: a function bound to a top-level `const` (`const f = () => …`) had visibility Unknown instead of ModulePrivate/Public | the top-level climb passes through `variable_declarator` | `symbols_visibility` rows `asyncArrow` (TS, JS), `documentedArrow` |
| JS/TS: `export const f = () => …` and JS `export const X = 1` were not `is_exported` | `is_exported_definition` climbs declarators before checking the export parent | `symbols_export::export_follows_export_syntax_not_access` (+5 asserts); golden `DEFAULT_LABEL` |

The visibility and export fixes change extracted records too and ride on the
same revision bump.

**Observed, not changed** (recorded as matrix cells with a reason):
`is_exported` is set only for Rust and JS/TS; Java `int... rest` and PHP
`int ...$rest` produce no parameter *record* (metrics and signatures do
count them); shell nested functions have no `container_name` (so `--in
outer_function` matches nothing, while the qualified name includes it);
Swift `@available(iOS, deprecated:)` is not a deprecation.

### Superseded expectations

Rewritten because the displayed name now carries the prefix:
`symbols_qualified_names` (displayed = `{file}#{in-file}` unless rooted),
`symbols_flags`/`symbols_metrics` (key by `symbols_fixtures::in_file_name`),
`cache_lookup::a_run_from_another_scan_root_hits_the_same_entry_with_the_same_ids`
(same ID and in-file name; prefixes `sub/python.py#` vs `python.py#`),
`cache_robustness` (qualified-name control value), the revision-6
`cache_upgrade` probe. Because of the return-type repair: `order.rs` and
`wiring.rs` lines lose `-> ()`. Goldens regenerated after review:
`symbols.json` (`qualified_name` prefixes, `+rev.9`, the two return types,
`DEFAULT_LABEL` exported), `symbols.plain` (two lines).

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| each predicate per language, n/a = nothing (16 rows × 14 predicates) | `symbols_selection_filters::selection_filters_keep_exactly_the_expected_declarations` |
| pushdown of kinds/capabilities, order independence, last test selection wins | `symbols_query_filters::predicates_push_down_only_the_kinds_and_capabilities_they_read` |
| metrics/flags filters never force diagnostics; one capture per file | `symbols_query_filters::a_filtered_query_runs_no_diagnostics` |
| predicates and prefix never reach the entry; another scan root is a `Hit` with the same IDs and its own prefix | `symbols_query_filters::predicates_and_prefixes_never_reach_the_cache_entry` |
| narrowing to a filtered, re-rooted query equals uncached; filtered results refuse other predicates/prefixes | `symbols_query_filters::a_wide_result_narrows_to_a_filtered_query_with_its_own_prefix` |
| naming context: `crate::shared::symbol::SymbolInfo::new`, `package.module.Class.method`; IDs unchanged; rooted names unprefixed | `symbols_query_filters::a_naming_context_names_modules_without_changing_ids` |
| non-ASCII smart case through records (`ünï`/`Ünï`, `ß`, exact) | `symbols_query_filters::non_ascii_names_match_with_smart_case` |
| matcher unit cases incl. Turkish `İ`, `ẞ`, CJK, literal-equality bug | `file::pattern::tests::*` (4) |
| qualified `--in` disambiguation, smart case on `--in` | `symbols_query_filters::qualified_container_patterns_disambiguate_repeated_names` |
| prefix and naming-context rules, `portable_relative` | `file::naming::tests::*` (6, four moved from the CLI) |
| CLI JSON carries the scan-root prefix | `golden::json_symbols_over_multi_match_the_golden`, `cache_lookup::a_run_from_another_scan_root_…` |

Input robustness matrix: not applicable. No persisted field or reader
changed shape; the predicates and naming inputs are in-memory query inputs.
Code-smell grep over the new modules: no `#[serde(default)]`,
`unwrap_or_default`, `.ok()` or `filter_map(.. as_str())` on a load-bearing
value.

### Docs and skill

`docs/symbol-kinds-and-visibility.md` (the prefix is current behavior, with
the scan-root/default/naming-context table; JS/TS `const` arrow access and
export), `docs/symbol-schema-design.md` (predicate table, `NamePattern`
rules, `narrowed`, declared-only return types, revision 9),
`docs/cache-design.md` (predicates are post-lookup, `rev.9`),
`docs/symbol-reports.md` (JSON example and prefix note), `lib/README.md`,
`.claude/skills/tree-hugger/SKILL.md` and `api-reference.md`.

### Verification

- `just test`: 879 run, 879 passed, 0 skipped (872 before the new modules).
- `just test-l2`: 6 passed. `just lint`: clean (one `type_complexity` fixed
  with a type alias). `just check-tier-coverage tree-hugger`: 0 stranded.
- New doctests (`glob_match`, `NamePattern`, `NamingContext`,
  `SymbolQuery`): `cargo test --doc -p tree-hugger` passes.
- `just cross-check tree-hugger --os linux` and `tree-hugger-cli --os linux`:
  pass.
- Windows: `just cross-check tree-hugger --os windows` failed
  `_storage_preflight` again; nothing was overridden or deleted. OS-sensitive
  code this phase: `portable_relative` moved unchanged (its Windows-only
  drive test moved with it) and `NamingContext` routes through
  `biscuit_file::PathIdentity`.
- No pre-existing failures; nothing skipped.

## Phase 5

CLI filters on the one library path. No library behavior changed except
file discovery (`collect_selected_files`); `EXTRACTOR_REVISION` stays `"9"`.

### 5.1 Flags

New module `cli/src/selection.rs`:

- `SelectionArgs` (flattened into `functions`/`types`/`symbols` and
  `classes`): `--kind` (repeatable; `KindArg`, `#[value(rename_all =
  "snake_case")]`, so `type_alias` parses and `type-alias` is an argument
  error), `--scope` (repeatable) / `--all-scopes` (conflict), `--visibility`
  (repeatable; `VisibilityArg` snake_case), `--in` (repeatable),
  `--exclude-tests` / `--tests-only` (conflict), `--undocumented`,
  `--deprecated`, `--async`, `--returns`, `--param-type`, `--min-params`.
- `TargetArgs` (`--file`, `--name`/`-n`, `--exact`), flattened into
  `SelectionArgs` and into the new `ImportArgs`.
- `classes --static-only` now `conflicts_with = "instance_only"`.
- `symbols` defaults to `Item` + `Member` (`SYMBOLS_DEFAULT_SCOPES` in
  `declarations.rs`); the other commands start with every scope.
  `--exported` stays its own `is_exported` predicate.
- Completions are dynamic (`CompleteEnv` over `Cli::command`), so the new
  flags and their values complete without a change: checked with
  `COMPLETE=fish hug -- hug symbols --ki` → `--kind`, `--kind type_` →
  `type_alias`, `imports --e` → `--exact`.
- Unit tests in `selection.rs` pin that `KindArg`/`VisibilityArg` cover every
  `SymbolKindV2`/`VisibilityLevel` variant under its serialized name in
  snake_case (19 and 10 values).

### 5.2 One filter path

- `Declarations::query(format, &Selection)`: command kinds ∩ `--kind`,
  scopes, identity predicates, shape predicates and `--in` as `SymbolQuery`
  predicates. The library applies them after the cache
  (`SymbolQueryResult::finish`), so records are filtered once, before
  presentation.
- Deleted: `scanner::normalize_symbol_glob`, `normalize_excluded_symbol_glob`,
  `matches_symbol_filters`, `DeclarationScan::{include_globs, exclude_globs}`,
  `Membership` (now `prelude: bool`; `--exported` is a predicate), the
  `classes --name` substring filter. Positional name tokens and
  `--exclude-symbols` are `NamePattern`s.
- `prepare` keeps only `--prelude`. A prelude file's synthetic exports
  replace its records after the library filtered them, so `prepare` applies
  `query.selects(kind) && query.matches(record)` to them
  (`prelude::selectors_apply_to_prelude_exports`).
- The plan's `cli/src/main.rs:1056-1084` reference was stale: that code had
  already moved into `declarations::prepare` and `scanner.rs` during
  `2026-10-03-symbol-improvements`; those are what was replaced.

### 5.3 `classes`

`classes::ClassSelection` holds three queries: `containers` (container kinds
∩ `--kind`, scopes, identity predicates), `within` (`--in` patterns as names
of the container itself, ANDed with `--name`), and `members` (shape
predicates over Method/Constructor). The analysis query carries no predicate:
selected container kinds ∪ member kinds, `Containment`, and the capabilities
both filters need. A shape predicate hides the field sections (fields have no
signature to match). `--json` keeps the existing `members` map keyed by
container ID; a container with no member left has an empty list.
`Declarations::Classes` boxes the selection (`clippy::large_enum_variant`).

### 5.4 `imports`

`ImportArgs` = positional file filters + `TargetArgs` + `ScanOptions`.
`ImportSelection` filters `index.imports` (so the summary and the JSON
`symbol_indexes` agree): a pattern matches the local name, the alias, or the
original name's last segment (`std::collections::HashMap` → `HashMap`,
`App\Models\User` → `User`), and a qualified pattern (`std::collections::*`)
matches the whole original path. `--exclude-symbols` applies here too.

### 5.5 `--file` / `--name`

`selection::resolve_file_selectors`: a glob (`*`, `?`, `[`) joins the walker
inputs like a positional file filter; any other value goes through
`biscuit_file::FileReference::new(..).resolve_in_context(&FileResolutionContext::new(cwd))`.
A value resolving to a file is an explicit target; one that resolves to
nothing (a directory pattern) joins the walker inputs; a resolution error (for
example `~someone/x.rs`) is an argument error (exit 2). `is_glob` ignores the
`?` of a Windows verbatim prefix (`\\?\C:\…`; `os` skill, windows-paths trap
12), unit-tested in `selection::tests::file_values_are_globs_only_with_a_wildcard`.

Library: `scanner::collect_selected_files(root, inputs, explicit_files,
excluded, language)` (`collect_files` delegates with no explicit files).
Explicit files bypass `.gitignore` and the default fixture exclusions, are
still removed by `--exclude-files`, never widen the scan (no inputs + explicit
files = only those files), and come back as `file_identity`.

### 5.6 Superseded tests

| Test | Change | Why |
|---|---|---|
| `classes::static_and_instance_sections_follow_the_member_flags` | both flags → argument error (exit 2) | plan 5.1 conflict |
| `cli::test_filter_help_describes_path_wildcard_and_exact_matching` | `fuzzy` → smart-case wording | C5 |
| `wiring::plain_symbols_omit_the_visibility_prefix_for_unknown_access`, `json_symbols_carry_visibility_and_schema_4_0`, `json_records_match_the_complete_index` | `symbols --all-scopes` | they assert locals (`localValue`, every index record) |
| `report::functions_and_types_select_exactly_their_kind_sets` | oracle is `symbols --all-scopes` | `types` keeps a local type that default `symbols` hides |
| `order::each_sort_order_lists_declarations_by_its_keys_and_tie_breakers` | `--all-scopes` on its three runs | tie-breaker cases are parameters (locals) |
| `level2_declarations` (`symbols --group-by-file` scenario) | `--all-scopes` | asserts parameter rows and their label style |
| goldens `symbols.plain`, `symbols.json` | regenerated | default `symbols` drops the 29 local lines (parameters, `local`/`my` variables, a parameter's `struct point`, a Zsh assignment inside a function), each checked against the fixture |

### 5.7 Focused cases

New module `cli/tests/symbols/filters.rs` (declared in the `symbols`
binary's `main.rs`; L1) plus one prelude case:

| Requirement | Test |
|---|---|
| `symbols` hides locals; `--all-scopes` adds exactly the locals; `--scope local`; three explicit scopes = `--all-scopes`; other commands keep locals; `--scope` + `--all-scopes` conflict | `filters::symbols_hides_locals_until_scopes_say_otherwise` |
| AND across filters, OR within `--name`/`--file`, exclusions win, shape filters, `--exclude-tests` + `--tests-only` conflict | `filters::filters_intersect_and_values_within_a_list_are_alternatives` |
| smart case, `!`, `--exact` (case kept, `*` literal), `*` glob, non-ASCII `ünï`/`Ünï`, smart-case exclusion | `filters::names_match_with_smart_case_and_exact_matching_including_non_ascii` |
| `--name` vs positional (`*/deep.py#*` positionally is a file glob), `*.rs` vs `--name *_rs`, `--file` on gitignored / fixture / `./` / glob / several, `--exclude-files` still wins, invalid file reference | `filters::file_and_name_flags_bypass_positional_classification` |
| file-qualified `--name` anchored at the prefix (leading `*` for files below the root), Rust `Widget::*` vs `*Widget::*`, C++ namespace-rooted `geometry::*`, `--in` plain/qualified/smart-case/exact | `filters::qualified_name_patterns_anchor_at_the_displayed_name` |
| `classes`: containers vs members, `--in`, `--undocumented`, `--kind class`/`function`, zero-member container listed, fields hidden under a shape filter | `filters::classes_select_containers_and_members_separately` |
| `--kind type_alias`, no widening, empty selection, unknown `--kind`/`--visibility` values | `filters::kind_selection_narrows_the_command_set_and_rejects_unknown_kinds` |
| `imports --name` alias / original / exact / qualified / exclusion; positional stays a file; `imports` rejects `--kind`, `--exported` | `filters::imports_match_an_alias_or_an_original_name_and_keep_positional_files` |
| `--undocumented` never Bash (`has_docs == None`), `--tests-only`/`--exclude-tests`/`--deprecated`, `--visibility unknown`, `--exported` vs `--visibility public` | `filters::test_docs_and_deprecation_filters_use_the_computed_flags` |
| empty selections exit 0 (JSON and plain); a failing file still exits 1 with `errors` | `filters::empty_selections_succeed_and_analysis_errors_still_fail` |
| `--json-full` rejects all 14 new selectors; accepts `--file`/`--exact` | `filters::json_full_rejects_every_record_selector` |
| filtered run is a complete `Hit` on the unfiltered entry; another scan root: same IDs, own prefix, one entry | `filters::selectors_never_reach_the_cache_entry` |
| selectors on prelude synthetic exports | `prelude::selectors_apply_to_prelude_exports` |
| kind/scope resolution, import last segment, value-enum coverage, `--file` glob detection | `selection::tests::*` (5) |

Stale-revision entry after the bump: covered by Phase 4's
`cache_upgrade::entry_from_extractor_revision_eight_is_not_served_after_the_repairs`
(no revision change this phase).

Input robustness matrix: not applicable. No file format, config or persisted
field is read or changed; the new inputs are command-line values, whose
malformed shapes (unknown kind/visibility, conflicting flags, invalid file
reference) are covered above.

### Departures and decisions

- **`imports` no longer accepts `--exported`/`--prelude`.** They came with
  the shared `CommonArgs` and were silently ignored; `ImportArgs` drops them,
  so they are argument errors now.
- **`--exclude-symbols` follows the name rules** (spec C5: "inclusion and
  exclusion patterns"): a plain value is a smart-case substring, not the
  literal whole name it was before (`--exclude-symbols foo` now also drops
  `foo_bar`; `foo!` keeps the old meaning). `--exact` does not apply to
  exclusions (ruling 8 lists `--name`, positional names and `--in`).
- **`--in` is repeatable** (alternatives), like the other list selectors.
- **`-n` is now on every declaration command** (it was `classes`-only).
- **Doc drift fixed (code wins):** `docs/symbol-reports.md` and the
  positional FILTER help said a bare extension such as `rs` is a file filter;
  `is_file_filter_token` takes only paths, globs and names with a source
  extension (`lib.rs`, `*.rs`). The docs and help now say so.
- **An empty file set is still an error.** `--file x --exclude-files x`
  leaves no file and fails with `NoSourceFiles`, as positional filters always
  did; "empty selections succeed" (spec) is about records.
- **Checkpoint numbers.** `hug symbols` over `tree-hugger/` now prints 4,388
  lines and `--all-scopes` 15,711; `--scope local` 11,335. The spec's 2,550 /
  8,593 were measured on a smaller tree; the ratio is the same (28% vs 30%).

### Docs and skill

`docs/symbol-reports.md` (new "Selecting declarations" section: selector
table, name patterns, `--file`/`--name`, `imports`, `--prelude`, selectors and
the cache; `classes` container/member split; `--json-full` rejection list;
flow diagram), `docs/symbol-kinds-and-visibility.md` (locals hidden by
default: **planned** marker removed), `docs/cache-design.md`
(`collect_selected_files` returns identities),
`.claude/skills/tree-hugger/SKILL.md` (selector path bullet),
`api-reference.md` (`collect_selected_files`).

### Verification

- `just test`: 897 run, 897 passed, 0 skipped (879 at the end of Phase 4;
  +13 in `filters`/`prelude`, +5 `selection` unit tests).
- `just test-l2`: 6 passed. `just lint`: clean (fixed
  `bool_assert_comparison` in a test and `large_enum_variant` by boxing
  `ClassSelection`). `just check-tier-coverage tree-hugger`: 0 stranded; the
  new tests are L1 (`symbols` binary, no tier marker).
- `just cross-check tree-hugger-cli --os linux`: 216 passed (6 L2 skipped);
  `just cross-check tree-hugger --os linux`: 680 passed.
- Windows: `just cross-check tree-hugger-cli --os windows` failed
  `_storage_preflight` again (host disk headroom, as in Phase 4); nothing
  overridden or deleted. OS-sensitive code this phase: `--file` resolution
  (`FileReference` + `file_identity`) and `is_glob`; the tests use relative
  `/`-separated values only.
- Validation checkpoint 5: `hug symbols` over `tree-hugger/` 4,388 lines,
  `--all-scopes` 15,711 (see Departures for the spec's older numbers).
- No pre-existing failures; nothing skipped.

