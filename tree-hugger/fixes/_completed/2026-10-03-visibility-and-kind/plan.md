---
kind: plan
name: visibility-and-kind
spec: 2026-10-03-visibility-and-kind
area: tree-hugger
total_phases: 7
created: 2026-10-03
phase: 7
agent: claude/sonnet
yolo: true
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

# Plan: preserve symbol kinds, declaration visibility, and static members

## Summary of the work

`hug` prints wrong kinds and no visibility because (a) extraction never computes
visibility for anything but functions, (b) the v2→v1 adapter
(`lib/src/shared/schema_v2/mod.rs`) hardcodes `visibility: None` /
`is_static: false`, drops `container_kind`, and collapses v2-only kinds to
`Unknown`, and (c) many per-language `locals.scm` queries have gaps. The spec
fixes this in one change (author ruling: single PR) and adds a 16-language
acceptance matrix so a construct cannot regress in one language unnoticed.

The work falls into five layers, built in dependency order:

1. **Model**: v1 `SymbolKind`/`Visibility` widening, v2 `VisibilityLevel`,
   `Modifier::StaticStorage`, `IdentityFacet::container_kind`,
   `SymbolInfo::{visibility,is_static,is_static_storage}`, `SchemaVersion::V3_0`,
   exhaustive (wildcard-free) conversions, adapter fixes, always-`Some` signatures.
2. **Extraction core**: kinds decided at extraction (delete
   `symbol_kind_v2_for_node` override), container + container kind, direct-member
   `Method` rule, Rust `impl` container, `relations.implements`,
   `declaration_text`, visibility for every kind, `static`/`StaticStorage`,
   `is_exported` decoupled from level, `EXTRACTOR_REVISION`.
3. **Per-language queries and rules**: the gap table (Rust, Java, TS, JS, Python,
   Swift, C, C++, PHP, Scala, C#, Perl), per-language visibility readers
   (including the Swift/C#/Rust/Scala/C++/JS-TS special rules), provenance and
   capture-inventory updates.
4. **CLI**: read `symbol.visibility`, drop the `impl ` special case, new JSON
   shape and schema 3.0, `query_fingerprint` from `EXTRACTOR_REVISION`.
5. **Verification and docs**: 16 primary fixtures, ten test groups from the spec,
   new docs page, README/skill/schema-design updates.

### Definition of success

- [x] Every committed `ok`, `FIX` and `LIM` cell in the spec's kind matrix passes
      through both `TreeFile::symbols()` and `symbol_index_v2()`; no `Unknown`
      kind in any primary fixture (except allow-listed with a reason).
- [x] The visibility matrix passes for all 16 languages on both
      `SymbolInfo::visibility` and `SymbolRecord.visibility.level`.
- [x] The static matrix and storage-`static` cases pass; `hug classes` partitions
      methods and fields correctly (TS, Java, C++, Rust representative tests).
- [x] Adapter round-trip preserves kind, visibility, `is_static`,
      `is_static_storage`, container kind, and signature presence for every
      fixture symbol.
- [x] A warm old-format cache entry is invalidated and the next run re-hits the
      new entry (test 8).
- [x] Every primary-fixture symbol has non-empty `declaration_text`; exports do
      not follow access (test 10).
- [x] `just test` and `just lint` pass in `tree-hugger/`; docs and skill updated;
      implementation log records any `ok`→`FIX` discoveries.
- [x] Agent terminal state: "implementation complete, ready for review"
      (no move to `_completed`, no commit unless asked).

## Phase 1: Baseline, rulings, and fixture seeding

Goal: remove ambiguity and put the test scaffold in place before touching
production code.

### Necessary Rules

Rulings the implementer must treat as decided (the spec is silent or only implies
them). Items marked **AUTHOR** are flagged for the author to confirm; the default
stated is what the plan proceeds with.

1. **File ownership for parallel waves.** `signatures.rs` (1.6k lines),
   `classify.rs` and `type_metadata.rs` are shared by every language. Parallel
   subagents must not edit the same file concurrently. Default: Wave 3.1 first
   splits per-language visibility/static/declaration logic into new modules
   under `lib/src/file/extract/` (e.g. `visibility/<lang>.rs`), after which
   language waves touch disjoint files plus one append-only registration point.
2. **Associated consts/types in a Rust `impl` / trait.** Spec gives their
   container but not their kind. Default: associated `const` → `Constant`,
   associated `type` → `TypeAlias`, both contained by the implemented type /
   trait, visibility per the Rust row. **AUTHOR** confirm.
3. **Rust unmarked item inside a *private* trait declared in the file.** Inherits
   the trait's level, i.e. `Private`. A trait-impl item whose same-file trait is
   unmarked takes `Private` as well (the trait's own level).
4. **Rust enum variant fields** inherit the enum's level (spec: "including
   variant fields"); struct/union fields without `pub` are `Private`.
5. **Swift enum case container kind** is `Type` (Swift enums are `LIM`/`Type`);
   container kind uses the same classifier as the standalone declaration.
6. **`EXTRACTOR_REVISION` format.** Default: a `&'static str`/`u32` in
   `tree-hugger-lib` (`pub const EXTRACTOR_REVISION: &str = "1"`), exported at
   crate root, appended to the CLI fingerprint string and the record's extractor
   provenance. Bumped on any change to extracted records.
7. **Declaration-text coverage (test 9).** "Every symbol" includes parameters and
   locals; for those, the text is the declaring node's text up to its body.
   Empty text for any fixture symbol is a test failure, not an allow-list case.
8. **Test 6 allow-list** starts empty. Any entry needs an inline reason and is
   reported in the implementation log.
9. **Input Robustness Matrix: not applicable.** The work changes tree-sitter
   queries and source-declaration extraction, not a manifest/config/lockfile
   reader. The only file-format reads (cache snapshots) already exist and are
   covered by test 8; no new `#[serde(default)]` is introduced on a load-bearing
   field *without a documented absence meaning* (new `SymbolInfo` fields default
   to `false`/`None` by spec). The implementer must grep for the code smells
   above in the diff anyway.
10. **Wider measurement (AUTHOR):** the spec names no performance cost; the new
    same-file scans (C/C++ linkage, Swift ancestor walk, JS/TS module detection)
    are expected linear. Ruling: no benchmark spike is scheduled; the author may
    request a corpus-timing comparison.
11. **`is_exported_definition` exclusions** for `Constructor`, `Property`,
    `EnumVariant` follow how Method/Field are excluded today (same match arm).
12. **Perl `package Foo;` scoping** is sibling scoping: declarations after the
    statement until the next `package` or end of enclosing block/file name `Foo`
    as container.

### Spikes

One-time, one host (macOS), quick sample. Do not repeat later.

- [x] **Grammar node-shape spike.** Dump `tree-sitter` parse trees (via a
      throwaway test or `hug` debug facility) for the exact constructs below and
      record node names/fields in the implementation log: Rust
      `function_signature_item`, `union_item`, `type_item`, `static_item`,
      `impl_item` `type`/`trait` fields; TypeScript `abstract_class_declaration`,
      `abstract_method_signature`, `method_signature`, `property_signature`,
      `private_property_identifier`; Java `constant_declaration`,
      `package_declaration` with `scoped_identifier`; C/C++ `union_specifier`,
      in-class `function_definition`; Swift protocol function/property
      requirement nodes and `modifiers`/`declaration_kind`; C# `explicit_interface_specifier`,
      `property_declaration`, `file_scoped_namespace_declaration`; Scala
      `access_qualifier`; Perl `variable_declaration` incl. list form,
      `package_statement`, `subroutine_declaration_statement`.
      Outcome: concrete query patterns for Phase 4, no spec changes.

### Tasks

- [x] **Baseline re-run.** Build `hug` at the current rev; run `--no-cache` over
      `spike/fixtures/*` in a scratch git repo; compare against
      `spike/baseline.md`. Any further `ok` cell found wrong → flip to `FIX` in
      this plan's working notes and record in the implementation log.
- [x] **Seed primary fixtures.** Copy `spike/fixtures/*` into
      `lib/tests/fixtures/symbols/<language>.<ext>` (16 primary files) plus
      `javascript_script.js`, `javascript_commonjs.js`, `typescript_script.ts`,
      and `.mjs`/`.cts` module-by-extension files. Confirm each parses with the
      language toolchain where available.
      - Prerequisite: spike baseline report is the source of truth.
- [x] **Register test modules.** Add empty modules to `lib/tests/l1/main.rs`
      (`autotests = false`): `symbols_coverage`, `symbols_kinds`,
      `symbols_visibility`, `symbols_static`, `symbols_adapter`,
      `symbols_decl_text`, `symbols_export`. Add CLI test modules in
      `cli/tests/cli.rs` (or a registered sibling) for tests 7 and 8.
- [x] **Fixture-path helper.** Reuse `manifest_dir!()`-based `fixture_path`; confirm
      the form appears in the `rust-testing` skill list so CI test-input
      scheduling sees the reads.

### Validation checkpoint 1

- [x] Baseline notes recorded; rulings above acknowledged; spike output logged;
      `just test` still green (no behavior changed yet).

## Phase 2: Model, schema 3.0, and adapter

Goal: make both models able to represent every kind/level/flag, with compile-time
exhaustiveness, before extraction changes. Existing consumers must compile.

### Wave 2.1 (parallel; disjoint files)

- [x] **v1 symbol types.** In `lib/src/shared/symbol.rs`: add `SymbolKind::{Constructor,
      Property, TypeAlias, EnumVariant}` (`is_function` includes Constructor,
      `is_type` includes TypeAlias); `Visibility` gains `Package, Crate,
      ModulePrivate, ProtectedInternal, PrivateProtected` (mirror
      `VisibilityLevel`; Unknown = `None`); `SymbolInfo` gains `visibility:
      Option<Visibility>`, `is_static: bool`, `is_static_storage: bool` (serde
      default false) and keeps `container_kind`; remove
      `FunctionSignature::visibility`. Fix the `Visibility::Internal` doc comment
      (Java default is Package).
- [x] **v2 kinds/facets.** In `schema_v2/kinds.rs` + `facets.rs`: add
      `VisibilityLevel::{ProtectedInternal, PrivateProtected}`,
      `Modifier::StaticStorage`, `IdentityFacet::container_kind:
      Option<SymbolKindV2>` (serde default + skip when none),
      `SchemaVersion::V3_0`.
      - Note: both tasks only share the type definitions consumed in Wave 2.2.

### Wave 2.2 (sequential after 2.1; one file)

- [x] **Adapter rewrite** (`schema_v2/mod.rs`):
      - `From<SymbolKindV2> for SymbolKind` and reverse: exhaustive, no wildcard.
      - `Option<Visibility>` ↔ `VisibilityLevel` exhaustive conversions.
      - `From<SymbolInfo> for SymbolRecord`: write visibility level,
        `Modifier::Static`, `Modifier::StaticStorage`, `container_kind`; set
        `is_exported = false`; leave `declaration_text` for Phase 3.
      - `TryFrom<SymbolRecord> for SymbolInfo`: restore visibility, `is_static`,
        `is_static_storage`, `container_kind`, signature copies.
      - `signature_from_function_data` returns `Some` for every function-like
        kind (empty params allowed); non-function kinds never get a signature.
      - Update every `SchemaVersion::V2_0` emit site (records, file indexes,
        analyzer fingerprint in `lib/src/cache/mod.rs`) to 3.0.
- [x] **Compile sweep.** Fix all non-wildcard match sites across lib and CLI
      (compile errors drive the list); keep behavior otherwise unchanged.
- [x] **Adapter unit tests.** Round-trip test with explicit expected values for
      every new field (test 5 skeleton), including parameterless methods and
      constructors.

### Validation checkpoint 2

- [x] `cargo check -p tree-hugger-lib -p tree-hugger-cli` clean; existing L1 passes
      except tests asserting schema `2.0` (rewrite per pre-authorization).
- [x] Adapter round-trip unit test green.

## Phase 3: Extraction core

Goal: kinds, containers, visibility machinery, static, declaration text decided
once at extraction. Language-specific readers are stubs registered per language
here and filled in Phase 4.

### Wave 3.1 (sequential; structural refactor to unlock parallel Phase 4)

- [x] **Module split.** Create `lib/src/file/extract/visibility/` with one file per
      language plus a dispatch `fn extract_visibility(node, lang, source, ctx)`
      accepting any declaration node (not just function nodes); move existing
      keyword readers (Java/C#/PHP/TS/Rust/C++/Swift) in, behavior-preserving.
      Same for static detection (`static_flags/<lang>`). Existing tests must stay
      green.
- [x] **`SymbolInfo` population.** `TreeFile::symbol_nodes` fills `visibility`,
      `is_static`, `is_static_storage` through the dispatcher; remove
      `FunctionSignature::visibility` uses.

### Wave 3.2 (parallel; disjoint files)

- [x] **Kind classification at extraction** (`classify.rs`): move constructor /
      property / type-alias refinement from `symbol_kind_v2_for_node` into
      `refine_symbol_kind`; delete the post-conversion override; constructor
      outranks method capture and property outranks variable at equal spans;
      no local-as-field promotion; update capture-name map, duplicate priority,
      display labels, exhaustive matches. Kind-specific data consistency
      (constructor → `FunctionData`, property → `FieldData`, alias →
      `TypeAliasData` incl. Rust RHS, variants keep enum metadata, no signature
      on variants). Rust `static` → `Variable` + `is_static_storage`.
- [x] **Container + direct-member rule** (`classify.rs::find_symbol_container`):
      store container kind; a declaration is never its own container; add
      abstract TS class, Rust union, C/C++ union, C++ class, PHP trait, Scala
      trait/object to containment; C++ class beats enclosing namespace;
      function directly in class/struct/enum/trait/interface/protocol/object/
      impl/extension body → `Method`; nested-in-function stays `Function`;
      sibling-scoped containers (C# file-scoped namespace, Perl `package Foo;`).
      - Complexity: depends on the kind classifier's classification of container
        kinds (same classifier); implement together to avoid divergence, i.e.
        one subagent owns `classify.rs`, a second owns the Rust `impl` logic in a
        new `rust_impl.rs`.
- [x] **Rust impl container + `relations.implements`**: container = last path
      segment of the self type without generics; kind from same-file declaration
      (absent if disagreeing or missing); `symbol_records()` fills
      `RelationFacet::implements` (name = trait last segment; `id` when the trait
      is in the same file); bind pass untouched.
- [x] **Signatures for all function-like symbols** (`signatures.rs`): provide
      `FunctionData` for parameterless fns, every Lua function, every Bash/Zsh
      function, C# expression-bodied methods (any return type).
- [x] **Declaration text** (`symbol_records()`): start-of-declaration to
      `body_span_for_declaration`, trailing whitespace trimmed, doc comments
      excluded; whole text when no body; Perl uses `variable_declaration`.
- [x] **`is_exported` decoupling**: `is_exported_definition` excludes
      `Constructor`/`Property`/`EnumVariant`; verify `symbol_records()`/bind are
      the only export sources.
- [x] **`EXTRACTOR_REVISION`**: export from lib root, stamp into record extractor
      provenance, add to CLI fingerprint (CLI wiring in Phase 5).

### Validation checkpoint 3

- [x] Existing suites green with updated superseded tests; kind matrix test
      skeleton runs for Rust (impl container, EnumVariant) as a smoke sample.

## Phase 4: Per-language queries and visibility/static readers

Goal: close every row of the "Query and classification gaps" table and implement
each language's visibility/static rules. All Wave 4.1 tasks are independent
(distinct query files and `visibility/<lang>.rs` modules); each subagent also adds
that language's rows to the kind/visibility/static matrix tests and its fixture
expectations. Subagents must not edit shared files other than appending a row/
registration.

### Wave 4.1 (parallel, one subagent per group)

- [x] **Rust.** Queries: `type_item`, `static_item`, `union_item`,
      `function_signature_item`, variants as `EnumVariant`, `mod_item` as
      `Module` with context capture (no self-container). Visibility: `pub` /
      `pub(crate|super|in)` → Crate / `pub(self)` → Private, default Private,
      trait items and variants inherit, trait-impl items via same-file trait
      else Unknown; keep `pub(...)` text in declaration_text; `static` →
      StaticStorage; associated fns without `self` → Method + static.
- [x] **TypeScript + JavaScript.** TS: `abstract_class_declaration`,
      `#private` methods, abstract/interface method signatures, interface
      property signatures (Public), type alias, constructor/property kinds,
      static, parameter-property sanity. JS: remove `this.x =` pattern, class-body
      fields as `Property`, constructor. Shared module-file detection
      (extension, import/export, `require`, `module.exports`/`exports.`
      anywhere) → ModulePrivate vs Unknown for unexported top level.
- [x] **Java + C#.** Java: `interface_declaration`, `package_declaration`
      (scoped_identifier), interface constants (static Field, Public), class →
      `Class`, default Package / interface Public. C#: file-scoped namespace,
      `property_declaration` → `Property`, `protected internal` /
      `private protected` / `file`, top-level Internal, member Private, interface
      member Public (incl. static/default/nested), explicit interface impl
      Private, setter-only accessor modifier ignored, interface/protocol
      methods removed from `FieldInfo` metadata, expression-bodied method
      signatures, compound levels in field metadata.
- [x] **C + C++.** C: `union_specifier`, typedef alias → `TypeAlias` (`Tag` Type
      + `Alias` TypeAlias once each; anonymous typedef struct; `typedef struct S
      {…} S`), static linkage → ModulePrivate. C++: `union_specifier`, class →
      `Class`, in-class `function_definition` methods, class container over
      namespace, access-section walk reused from type metadata for methods,
      linkage rule (static, unnamed namespace, `const` w/o extern/inline incl.
      `constexpr`, top-level-const-of-object rule, earlier same-file `static`
      declaration with normalized param text), `StaticStorage` for file-scope and
      function-local statics, `Static` for class members only.
- [x] **Swift.** Enum cases → `EnumVariant`; member functions → `Method`
      (classes/structs/enums/extensions/protocols); protocol function and
      property requirements (Field); function-local `let/var` → `Variable`;
      `open`/`public`/`package`/`internal`/`fileprivate`/`private`; setter-only
      modifiers; nearest-container default (private/fileprivate type →
      ModulePrivate, explicit extension level, else Internal; requirements and
      cases inherit); `class`/`static` members static.
- [x] **Python.** Class → `Class`; `@staticmethod`/`@classmethod` → Method +
      static; remove attribute-assignment catch-all; class-body fields with
      ClassVar/unannotated → static, annotated → instance; align
      `extract_python_class_fields` with the standalone path (same fields, same
      flag); visibility Unknown.
- [x] **PHP, Scala, Lua, Go.** PHP trait method container; Scala trait/object/
      class members as `Method` with container, `private[this]` / `protected
      [this]` / package-segment qualifier / other qualifier (Unknown +
      declaration_text), object members static; Lua chunk-level `local` →
      ModulePrivate, block-local Unknown, global Public, table fields Unknown,
      signatures; Go uppercase → Public else Package (locals/params/package
      clause Unknown), `type A = B` stays unreported (documented).
- [x] **Perl + shells.** New `queries/vendor/perl/locals.scm`: named `sub` →
      Function, `package` (statement and block) → Module, `my/our/state` names
      (each list name) → Variable, `local` excluded; container = enclosing sub
      else package (sibling-scoped); Unknown visibility. Bash/Zsh: signatures
      and Unknown visibility.

### Wave 4.2 (sequential; after 4.1)

- [x] **Provenance + inventory.** Update `QueryProvenance` helper per changed
      language (`MultipleChanges` where patterns were added; Perl `Original`,
      tree-hugger authorship and repo license, not nvim-treesitter), capture
      inventory expectations, `query_compile` tests and `phase6_neovim_query_reuse`
      tests together.
- [x] **Fill matrix tests.** Reconcile per-language matrix rows into the
      table-driven tests 2–4 and the language-coverage guard (test 1); no
      `Unknown` kinds (test 6).

### Validation checkpoint 4

- [x] `just test` in `tree-hugger/`: kind/visibility/static matrices green for all
      16 languages; query-compile and provenance tests green.

## Phase 5: CLI and cache

Goal: render the corrected data and ensure warm caches cannot hide the fix.

### Wave 5.1 (parallel)

- [x] **CLI rendering.** `cli/src/main.rs`: read `symbol.visibility`; omit prefix
      for Unknown; labels `package`, `crate`, `module-private`,
      `protected internal`, `private protected`; remove the `impl ` container
      special case; `[in class Box]` kind suffix; `hug classes` partitions on
      shared `is_static`; constructors in `functions`, aliases in `types`; JSON
      `files[].symbols[].visibility` (omitted for Unknown); schema 3.0 in
      envelopes. Use biscuit-terminal components for any rendering change.
- [x] **Cache fingerprint.** Replace the literal `"locals+imports+references"`
      with a value containing `EXTRACTOR_REVISION`; keep schema, grammar,
      language, crate version, options components.
      - No dependency on the rendering task other than the same file, so one
        subagent should own `main.rs` or edits must be sequenced.

### Wave 5.2 (sequential)

- [x] **CLI tests** (test 7 and 8): scratch git repo, isolated HOME/config/TMPDIR
      (`TMP`/`TEMP` on Windows), seeded old-format cache entry using the
      historical key formula hard-coded in the test; run 1 misses, run 2 hits
      after an edit to the new snapshot. Cover TS, Rust, C++, Java,
      Unknown-access language; failure messages name language, declaration,
      field. No terminal windows or focus changes.
- [x] **Superseded tests.** Rewrite `[in impl Greeter]` and other pre-authorized
      expectations.

### Validation checkpoint 5

- [x] `just test` green; run `just test-l2` (should be unaffected) and
      `just lint`.

## Phase 6: Remaining verification tests

Goal: finish the ten test groups from the spec.

### Wave 6.1 (parallel; separate test modules)

- [x] **Test 5 (adapter) over every fixture symbol** with explicit expected values
      and both conversion directions; retain signature/type-metadata regression
      coverage.
- [x] **Test 9 (declaration text)**: non-empty for every fixture symbol; exact
      text for `pub(in crate::a) fn f()`, bodyless trait method, Swift
      `public private(set) var count = 0`, C# `{ get; private set; }`, Scala
      `private[Shape] def secret() =`, Perl `my ($a, $b)` (both names), Java
      annotated class.
- [x] **Test 10 (export vs access)**: `From<SymbolInfo>` Public → `is_exported:
      false`; Java class and TS class method false; TS `export function` and Rust
      `pub fn` true.
- [x] **Static matrix and storage `static`** (test 4) including Python four
      class-body forms and `self.y`/`obj.attr` producing no symbol; Scala
      object vs class; Java interface constant.
- [x] **Perl/JS specifics from the fixture notes**: `local` produces no symbol;
      `this.w = …` produces no symbol; JS fields `x`, `y`, `#z` as `Property`.

### Validation checkpoint 6

- [x] All ten test groups green, `just test` and `just lint` pass.
- [x] Implementation log lists every `ok`→`FIX` discovery and any allow-listed
      `Unknown`.

## Phase 7: Documentation and closure

### Wave 7.1 (parallel; disjoint files)

- [x] **New page** `tree-hugger/docs/symbol-kinds-and-visibility.md` covering every
      bullet in the spec's documentation section (audience: developer new to the
      repo; compact example per rule; Mermaid diagrams for the pipeline and the
      visibility decision flow; never name a spec).
- [x] **READMEs**: `tree-hugger/lib/README.md` and `tree-hugger/cli/README.md`.
- [x] **Skill**: `.claude/skills/tree-hugger/SKILL.md` + `api-reference.md`
      (SymbolKind list, Known Limitations incl. Perl, Python `self.x`, JS `this.x`).
- [x] **Schema design**: `tree-hugger/docs/symbol-schema-design.md` (3.0, container
      kind, visibility ownership, export separation, declaration_text,
      `relations.implements`; planned items marked **planned**).
- [x] **Feature-spec sync**: update `2026-10-03-symbol-improvements` text for schema
      version numbers (3.0 → 4.0) and field names.
- [x] **Comment drift pass**: any `///`/`//` contradicting new levels; also
      `docs/dependencies.md` unaffected (no new crates).

### Wave 7.2 (sequential)

- [x] **Final gate.** `just test`, `just lint` in `tree-hugger/`; `just test-l2`
      once; confirm no `cargo fmt` run; write implementation log; mark
      "implementation complete, ready for review". Do not commit unless asked and
      do not move the spec to `_completed`.

### Validation checkpoint 7

- [x] All success criteria above checked; docs reviewed against behavior.
