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
packages:
  - tree-hugger
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
