---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-06-symbol-type-relations/spec.md"
plan: "tree-hugger/features/2026-10-06-symbol-type-relations/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_2:
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/error.rs
  - tree-hugger/lib/tests/l1/symbols_generics.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_3:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_4:
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/tests/l1/symbols_modifiers.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_4:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/lib/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_5:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_6:
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/tests/l1/symbols_relations.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_owner_identity.rs
  - tree-hugger/lib/tests/l1/symbols_shared_context.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_6:
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_6:
  - tree-hugger/docs/type-relations.md
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_7:
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_relations.rs
  - tree-hugger/lib/tests/l1/symbols_shared_context.rs
docs_updated_during_phase_7:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_8:
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/outline.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
docs_updated_during_phase_8:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/type-relations.md
  - tree-hugger/cli/README.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_9:
  - tree-hugger/lib/src/file/tree_file.rs
docs_updated_during_phase_9:
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/type-relations.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_code:
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/error.rs
  - tree-hugger/lib/tests/l1/symbols_generics.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_coverage.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/tests/l1/symbols_modifiers.rs
  - tree-hugger/lib/tests/l1/symbols_flags.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/tests/l1/symbols_relations.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_owner_identity.rs
  - tree-hugger/lib/tests/l1/symbols_shared_context.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/outline.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
documentation:
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/type-relations.md
completed_phase: 9
implemented: true
---

# Implementation Log for 2026-10-06-symbol-type-relations (9 phases)

## Phase 1

Phase 1 is a gate: baseline, grammar probe and rulings. No repository source,
test, doc or skill file changed. The probe ran as a throwaway Cargo project
outside the repository (`/tmp/th-probe`, deleted at the end of the phase) that
pins exactly the grammar versions in `tree-hugger/lib/Cargo.toml` and prints
each snippet's tree with field names.

### 1.1 Prerequisite check

**Baseline gates (macOS, 2026-10-05, working tree at `1dcf7db12` plus this
feature's untracked log):**

- `just lint`: green (`tree-hugger` and `tree-hugger-cli`).
- `just test`: **1021 / 1021 passed**, 0 skipped (both packages, one nextest
  invocation, 5 binaries).
    - The first run reported one `TIMEOUT` (30 s) in
      `tree-hugger::l1 file_identity::god_files_paths_are_relative_to_any_spelling_of_the_root`
      while the probe crate was compiling on the same machine; the rerun
      (`just test --no-fail-fast`) passed everything. Treat it as a
      load-sensitive test, not a pre-existing failure. No pre-existing
      failures.

**Current versions (plan drift corrected here):**

| Item | Plan said | Actual |
|---|---|---|
| `SchemaVersion` | 4.0 | `SchemaVersion::V4_0` (`lib/src/shared/schema_v2/types.rs:28`) — still current, so Phase 2 introduces **4.1** |
| `EXTRACTOR_REVISION` | `"10"` | **`"17"`** (`lib/src/file/mod.rs:24`; re-exported from `lib/src/lib.rs:30`). Phase 2 bumps it to `"18"` and adds revision 17 as a `StaleRevision` with `OldKey::Computed` in the CLI cache-upgrade tests |

**`2026-10-05-deferred-kind-precision` state:** its spec frontmatter says
`implemented: true` (directory still active, in review). Kinds as they stand,
read from `lib/src/file/extract/classify.rs`:

- Swift: `class_declaration` (class, struct, enum, extension) → `Type`;
  `protocol_declaration` → `Interface`. The class/struct/extension
  distinction lives only in the anonymous `declaration_kind` token.
- Go: struct and interface types both → `Type`.
- Constructors: Java/C#/Swift by query, Python `__init__`, PHP
  `__construct`, JS/TS `constructor` in a `class_body`, C++ by syntax.
- So, per ruling 3, Phases 3–6 must read declaration syntax (Swift
  `declaration_kind`, Go `interface_type`/`struct_type`) wherever the kind
  is too broad.

**Files later phases touch — all exist:**
`lib/src/shared/schema_v2/{facets,mod}.rs`,
`lib/src/file/extract/{attributes,classify,declaration,modifiers,rust_impl,type_metadata,metrics,flags}.rs`,
`lib/src/file/tree_file.rs`, `lib/src/file/tree_file/definitions.rs`,
`lib/src/file/query.rs`, `lib/src/analysis/coverage.rs`,
`lib/src/cache/symbol_cache.rs`, `cli/src/{classes,declarations,render}.rs`,
`cli/tests/symbols/cache_upgrade.rs`,
`lib/tests/l1/{symbols_metrics,symbols_query,symbols_cache,symbols_adapter,symbols_coverage}.rs`.

**Existing record shapes later phases build on** (`facets.rs`):

- `MetricsFacet { parameters, has_docs }`, both `deserialize_with =
  "nullable"` (absent key is an error, `null` is `None`). `lines` follows the
  same pattern.
- `GenericParam { name, kind: GenericParamKind{Type,Lifetime,Const},
  constraints (skip-if-empty, **`#[serde(default)]`**), default, variance:
  Option<String>, position: u16 }`. The `#[serde(default)]` on
  `constraints` means a cached record cannot distinguish an absent list
  from an empty one; it is serialized only when non-empty, so this is the
  intended encoding, but Phase 7's cache robustness rows must state it.
- `Modifier` is `#[serde(tag = "type", content = "value")]` with
  `Static, StaticStorage, Async, Const, Readonly, Unsafe, Mut, Custom(String)`.
- `Attribute { name, value: Option<String> }`, value skipped when `None`, so
  `None` (no arguments) and `Some("")` (explicit `()`) serialize differently.

### 1.2 Grammar probe

Pinned grammars (`tree-sitter = 0.26.3`): bash 0.25.1, c 0.24.1, c-sharp
0.23.1, cpp 0.23.4, go 0.25.0, java 0.23.5, javascript 0.25.0, lua 0.5.0,
perl 1.1.2, php 0.24.2 (`LANGUAGE_PHP`), python 0.25.0, rust 0.24.0, scala
0.24.0, swift 0.7.1, typescript 0.23.2 (`LANGUAGE_TYPESCRIPT`), zsh 0.56.0.

Notation: `node` = named node kind, `field:` = field name, `'tok` = anonymous
token. "no node" = the language has no such construct in required scope.

#### Generics (parameters, bounds, defaults, variance, declaration-level constraints)

| Language | Parameter list / parameter nodes | Bound | Default | Variance | Declaration-level constraint |
|---|---|---|---|---|---|
| Rust | `type_parameters:` → `lifetime_parameter`, `type_parameter` (`name:`), `const_parameter` (`name:`, `type:`, `value:`) | `bounds: trait_bounds` (entries are its named children; lifetimes as `lifetime`) | `default_type:` (type), `value:` (const) | no node | `where_clause` → `where_predicate` (`left:`, `bounds:`) |
| TypeScript | `type_parameters:` → `type_parameter` (`name:`) on class, interface, alias, function, method, arrow function | `constraint: constraint` (`'extends` + type) | `value: default_type` (`'=` + type) | **grammar limit** (below) | no node |
| JavaScript | no node | no node | no node | no node | no node |
| Go | `type_parameters: type_parameter_list` → `type_parameter_declaration` (`name:` repeated, `type: type_constraint`) on `type_spec` and `function_declaration` | `type: type_constraint` (text includes unions, `~` terms, inline `interface{…}`) | no node | no node | no node |
| Python | `type_parameters: type_parameter` → children `type`, each wrapping `constrained_type` (`T: int`: two `type` children), `splat_type` (`*Ts`, `**P`) or a bare `identifier`; on `class_definition`, `function_definition`; on `type_alias_statement` inside `left: type → generic_type → type_parameter` | second `type` child of `constrained_type` | **grammar limit** (below) | no node | no node |
| Java | `type_parameters:` → `type_parameter` (unfielded `type_identifier`, `type_bound`) | `type_bound` (entries split by `'&`) | no node | no node | no node |
| C# | `type_parameter_list` → `type_parameter` (`name:`); **field `type_parameters:` on interface but unfielded on class** | via `type_parameter_constraints_clause` (`identifier` + `type_parameter_constraint`*: `class`, `new()`→`constructor_constraint`, `type:`) | no node | anonymous `'in` / `'out` inside `type_parameter` | `type_parameter_constraints_clause` is always per named parameter, so it attaches to that parameter |
| C | no node | no node | no node | no node | no node |
| C++ | `template_declaration` `parameters: template_parameter_list` → `type_parameter_declaration`, `optional_type_parameter_declaration` (`name:`, `default_type:`), `variadic_type_parameter_declaration`, `parameter_declaration` (non-type), `optional_parameter_declaration` (`default_value:`), `template_template_parameter_declaration` | concept-constrained: **see limit below** | `default_type:` / `default_value:` | no node | `requires_clause` (`constraint:`) after the parameter list |
| Swift | `type_parameters` (unfielded) → `type_parameter` (`type_identifier` + **bound under field `name:`**, a grammar field-name quirk) on `class_declaration` and `function_declaration` | `type_parameter` field `name: user_type` | no node | no node | `type_constraints` → `type_constraint` → `inheritance_constraint` (`constrained_type:`, `name:`) |
| Scala | `type_parameters:` → **flat**: an invariant parameter has no wrapper (`name: identifier`, then `bound:` siblings, then optional nested `type_parameters` for higher-kinded `F[_]`); `covariant_type_parameter` / `contravariant_type_parameter` wrap their own `name:`/`bound:`. On a `function_definition` the list is under field **`parameters:`** (field-name quirk) | `bound: upper_bound` (`<: T`), `lower_bound` (`>: T`), `context_bound` (`: Ctx`; the `':` token is also tagged `bound:`) | no node | `'+` / `'-` tokens in the wrapper node | no node |
| PHP | no node | no node | no node | no node | no node |
| Perl, Bash, Zsh, Lua | no node | no node | no node | no node | no node |

#### Declared relations (base lists)

| Language | Node / fields |
|---|---|
| Rust | `trait_item` `bounds: trait_bounds` (supertraits; `lifetime` entries are constraints, not bases). `impl_item` `trait:` + `'for` + `type:`; negative impl = anonymous `'!` before `trait:`; `type_parameters:` on the impl (never copied); empty impl still has `body: declaration_list` |
| TypeScript | `class_heritage` → `extends_clause` (**`value:` expression + separate `type_arguments:` field**, so the written target is the clause text minus `'extends`) and `implements_clause` (one named child per entry); interface `extends_type_clause` (`type:` per entry) |
| JavaScript | `class_heritage` → single expression child (may be a `call_expression`: dynamic) |
| Go | `interface_type` → `type_elem` per line; a named embed is `type_elem` → `type_identifier` / `qualified_type`, a union / `~` term is `type_elem` → `negated_type` or `'|`-separated children. Struct embeds are `field_declaration` with no `name:` (composition) |
| Python | `superclasses: argument_list`; positional entries vs `keyword_argument` (`metaclass=`) |
| Java | `superclass: superclass`, `interfaces: super_interfaces` → `type_list`; interface `extends_interfaces` → `type_list` |
| C# | `base_list` on class, struct, interface, record (mixed list; ruling 1) |
| C | no node |
| C++ | `base_class_clause` → per entry optional `access_specifier`, optional `'virtual`, then the type node (`type_identifier`, `template_type`, `qualified_identifier`) |
| Swift | `inheritance_specifier` (`inherits_from:`) repeated directly under `class_declaration` (class/struct/enum/**extension**, told apart only by `declaration_kind:` token) and `protocol_declaration` (mixed list; ruling 1). Extension's self type is `name: user_type` |
| Scala | `extend: extends_clause` with `type:` entries; the `'with` tokens also carry field `type:` (filter to named nodes); constructor `arguments:` follow the first type (not part of the target) |
| PHP | `base_clause` (`name` entries), `class_interface_clause` (class and enum); trait `use_declaration` in the body (composition) |
| Perl | `use_parent_statement`, `our @ISA = …` exist as nodes; not inferred (spec) |
| Bash, Zsh | no node |
| Lua | `setmetatable(…)` is an ordinary `function_call`; not inferred (spec) |

#### Attributes and decorators

| Language | Node |
|---|---|
| Rust | `attribute_item` → `attribute` (path + `arguments: token_tree`); `inner_attribute_item` is file/module-level (must not attach) |
| TypeScript | `decorator` (field `decorator:`) on the class, method, field; **an exported class's decorator sits on the `export_statement`** (wrapper), a non-exported one on `class_declaration` |
| JavaScript | `decorator` supported by the 0.25.0 grammar, on `class_declaration` (including `export @dec class`) and `method_definition` |
| Go | no node |
| Python | `decorated_definition` → `decorator`* (expression: identifier, `attribute`, `call`) |
| Java | `modifiers` → `annotation` (`name:`, `arguments: annotation_argument_list`) / `marker_annotation` |
| C# | `attribute_list` → `attribute` (`name:`, `attribute_argument_list`); `global_attribute` (`[assembly: …]`) is file-level (must not attach) |
| C | GNU `attribute_specifier` (`__attribute__((…))`) exists; spec requires no new C attribute forms |
| C++ | `attribute_declaration` → `attribute` (already read by `declaration_attributes`) |
| Swift | `modifiers` → `attribute` (`user_type` name, then argument nodes as siblings) |
| Scala | `annotation` (`name:`, `arguments:`) |
| PHP | `attributes: attribute_list` → `attribute_group` → `attribute` (outside required set) |
| Perl | `function_attribute` (`:lvalue`) exists; no newly required form |
| Bash, Zsh, Lua | no node |

#### Modifier tokens

| Language | Node |
|---|---|
| Rust | `function_modifiers` → `'const`, `'unsafe`, `'async`; `self_parameter` / `parameter` → `mutable_specifier` (receiver/binding `mut`) |
| TypeScript | `'abstract` (`abstract_class_declaration`, `abstract_method_signature`), `override_modifier`, `'readonly` (field and constructor parameter property), `accessibility_modifier` |
| JavaScript | only `'static`; no TS modifiers |
| Java | `modifiers` with anonymous `'abstract`, `'final`, … |
| C# | `modifier` nodes (`abstract`, `override`, `virtual`, `sealed`, `readonly`) |
| C++ | `'virtual` token on the member declaration; `virtual_specifier` (`override`, `final`) inside the `function_declarator`; `default_method_clause` / `delete_method_clause` on defaulted/deleted members |
| Swift | `modifiers` → `inheritance_modifier` (`final`), `member_modifier` (`override`), `mutation_modifier` (`mutating`) |
| Scala | `modifiers` → `'abstract`, `'final`, `'override` |
| PHP | `abstract_modifier`, `final_modifier` |
| Go, C, Perl, Bash, Zsh, Lua | none of the selected set |

#### Callable bodies (for Phase 5)

| Language | Block body | Expression body |
|---|---|---|
| Rust | `body: block` | no node (functions always have a block) |
| TS / JS | `body: statement_block` | `arrow_function` `body:` = expression (no `=>` inside the node) |
| Go | `body: block` | no node |
| Python | `body: block` | `lambda` `body:` (only as a value, e.g. a default — must not be taken as the enclosing callable's body) |
| Java | `body: block` / `constructor_body` | no node for methods |
| C# | `body: block` | `body: arrow_expression_clause` (includes `'=>`; the expression is its named child) |
| C / C++ | `body: compound_statement` | C++ `default_method_clause` / `delete_method_clause` are not bodies; lambdas are values |
| Swift | `body: function_body` | no node (single-expression bodies are still `function_body`) |
| Scala | `body: block` | `body:` = expression (no `'=` inside the node) |
| PHP | `body: compound_statement` | `arrow_function` `body:` |
| Lua / Bash / Zsh / Perl | block / `compound_statement` / `block` | no node |

#### Demonstrated grammar limits

1. **Python type-parameter default (PEP 696)** — tree-sitter-python 0.25.0.
   `class C[T = str]: pass` → `has_error=true`; `type_parameter` holds
   `ERROR(identifier "T", '=')` then `type "str"`. Bounds, `*Ts` and `**P`
   parse cleanly. Defaults are unsupported for Python; everything else in
   the cell is supported.
2. **TypeScript variance annotations (`in` / `out`, TS 4.7)** —
   tree-sitter-typescript 0.23.2. `interface K<in T> {}`, `<out T>` and
   `<in out T>` all give `has_error=true` (`type_parameter "in"` then
   `ERROR(identifier "out", identifier "T")`). The spec does not require TS
   variance; recorded so the matrix row says "grammar limit", not
   "unsupported". `class K<const T>` parses.
3. **C++ concept-constrained type parameter** — tree-sitter-cpp 0.23.4.
   `template <std::integral T> void f(T)` parses cleanly but as
   `parameter_declaration(type: qualified_identifier "std::integral",
   declarator: identifier "T")`, the same shape as the non-type
   `template <int N>`. Syntax alone cannot tell a concept-constrained type
   parameter from a non-type parameter whose type is a named (non-primitive)
   type. Phase 3 must decide (see the message to the next agent); the
   `requires` clause form (`requires_clause`) and `typename`/`class`
   parameters are unaffected.

Not limits (snippet errors, recorded so nobody rediscovers them): two Swift
members on one line without a separator (`{ mutating func z() {} var p … }`)
is invalid Swift and errors; on separate lines `mutation_modifier` parses
fine.

**Every "None" cell of the spec table is confirmed an absent construct** for
generics in JavaScript, C, PHP, Perl, Bash, Zsh and Lua; for relations in C,
Bash, Zsh (Perl `@ISA`/`use parent` and Lua metatables exist as ordinary
nodes and are excluded by the spec, not by the grammar); for modifiers in Go,
C, Perl, Bash, Zsh and Lua.

### 1.3 Rulings

This session is non-interactive, so the author could not be asked. Per the
plan, every ruling proceeds on its default **except ruling 1**, which stays
open and gates only tasks 6.5–6.7:

| # | Ruling | Status |
|---|---|---|
| 1 | Mixed C#/Swift/Scala base lists | **Open — author decision required before 6.5–6.7.** Plan default (spec's recommended option) noted, not adopted |
| 2 | Schema version | Default: introduce `V4_1`; tests use a genuinely different version (e.g. 5.0) as the "incompatible" example |
| 3 | Prerequisite state | Default: proceed on the working tree (baseline green); read syntax where kinds are broad (Swift `declaration_kind`, Go `interface_type`) |
| 4 | Full written Rust trait text | Default: accept; method-level `implements` text changes are intended |
| 5 | Modifier order | Default: one normalizer at record construction |
| 6 | Unresolved impl self type | Default: as spec |
| 7 | `u16` overflow | Default: explicit analysis error, unit-tested with a synthesized list |
| 8 | Wider performance measurement | Default: not planned |
| 9 | Spike scope | Done once (1.2); not repeated |

### Phase 1 outcome

- Requirement → evidence: baseline (1.1) → `just test` 1021/1021, `just lint`
  green; probe (1.2) → tables above; rulings (1.3) → table above.
- No tests were added or changed: Phase 1 changes no behavior.
- Skipped: nothing. Pre-existing failures: none (one load-induced timeout
  noted above, passed on rerun).

## Phase 2

Schema 4.1, extractor revision 18, the `metrics.lines` key, one attribute
parse per declaration and the modifier normalizer. Single owner, no
parallel work. `lines` is plumbed but always `null`; Phase 5 computes it.

### 2.1 `lines` key

- `MetricsFacet` gained `lines: Option<u32>` with `deserialize_with =
  "nullable"` like its siblings (`facets.rs`): absent key → error, `null` →
  `None`, no `#[serde(default)]`. Every constructor sets `lines: None`
  (`metrics.rs::symbol_metrics`, `cli/src/stats.rs` test, two L1 tests).

### 2.2 Version bumps

- `SchemaVersion::V4_1` added; `V4_0` kept as a documented historical
  constant (like `V2_0`/`V3_0`). Every emitter and the exact-version checks
  in `cache/symbol_cache.rs` now use `V4_1` (records, file indexes, CLI
  envelope in `report.rs`, fingerprints, god-files, `main.rs`,
  `declarations.rs`).
- `EXTRACTOR_REVISION` `"17"` → `"18"`.
- "4.1 is incompatible" examples replaced: the CLI matrix's
  `IncompatibleMinor` is now the previous `4.0`; the lib record-version test
  walks 2.0, 3.0, 4.0, 4.2, 5.0; the entry-version test adds 4.0; the CLI
  upgrade test walks 2.0, 3.0, 4.0, 5.0. `symbols_adapter::new_record_fields_read_per_shape`
  used `"lines": 3` as its *unknown metric key* example; it now uses
  `"bogus"` and gained the `lines` rows below.
- **Revision 17 test departs from the `StaleRevision` pattern.** That helper
  requires the stale payload to be servable under the current key (its
  control). A revision 17 entry (4.0 records, metrics without `lines`) is
  not servable by revision 18 at all, so
  `cache_upgrade::entry_from_extractor_revision_seventeen_is_not_served_after_the_schema_bump`
  checks by hand: rejected and recomputed under the current key, never read
  under its own computed key, rewritten as 4.1 with `lines`, and hit on
  run 2.
- **IDs:** `schema_v2::tests::symbol_ids_survive_schema_and_revision_bumps`
  pins `rust::src/lib.rs::Function::hello::0` → `7d883dc4…a7a4` literally.
  The ID formula reads neither version; goldens normalize IDs, and their
  diff shows no other change.

### 2.3 Shared parsed attributes

- `is_deprecated(&[SyntacticAttribute], doc, language)` and
  `has_test_attribute(&[SyntacticAttribute], language)` no longer read the
  tree; `records_from` calls `declaration_attributes` once per selected
  declaration (inside the Flags branch, since nothing else reads them yet)
  and passes the slice to both. Previously each reader parsed separately.
- `SyntacticAttribute` gained `value: Option<String>`: the raw source
  between the outer delimiters, trimmed, comments and inner whitespace kept;
  `None` exactly when `arguments` is; `Some("")` for `()`; Rust `= value`
  gives the value text. `impl From<&SyntacticAttribute> for Attribute` is the
  record conversion Phase 4 uses.
- **Not done here (Phase 4.2):** filling `record.attributes`, TS/JS
  decorators, the "unusual decorator expression" name rule. When 4.2 fills
  the facet for every `wanted` record, hoist the `declaration_attributes`
  call out of the Flags closure so flags and the facet share it (clippy's
  `obfuscated_if_else` rejected the `then(..).unwrap_or_default()` form;
  use an `if`).

### 2.4 Modifier normalizer

- `Modifier` derives `PartialEq, Eq, PartialOrd, Ord`; the variant order is
  the spec order, so `normalize_modifiers` (`facets.rs`, `pub(crate)`) is
  sort + dedup. It runs once per record in `records_from`, after every
  modifier contribution and before relations are attached. Phase 4 readers
  push in any order before that line.
- Existing Static/StaticStorage/Async sets are unchanged (they were already
  in order; every existing test passes).

### Input robustness matrix (`metrics.lines`)

| Shape | Edit | Outcome | Test |
|---|---|---|---|
| control | `lines: 3` | served, `3` | CLI `cache_robustness` column `metrics.lines` |
| absent | key removed | rejected → recompute | same column; lib `lines absent` |
| explicit null | `null` | served, `null` | same column |
| wrong type | `"3"` | rejected | same column; lib `lines wrong type` |
| wrong number | `3.5`, `-1`, `4294967296` | rejected | `cache_robustness::metrics_lines_serves_only_a_u32_count`; lib `lines fraction`/`lines negative` |
| boundary | `4294967295` | served | `metrics_lines_serves_only_a_u32_count` |
| one / every element wrong | `[3, 123]` / `[123]` | rejected | matrix column; lib `lines one wrong element` |
| empty | `0` | served, `0` (distinct from `null`) | matrix column; lib `empty_body` |
| duplicate key | `lines` twice | rejected | matrix column; lib `duplicate lines` |
| trailing content | valid entry + garbage | rejected | matrix column; lib `trailing content` |
| facet absent, entry lists `metrics` | `metrics` removed | rejected (`Inconsistent`, existing contract) | matrix column `metrics` / Absent |
| facet absent, entry without `metrics` | entry from `hug --json functions` | served to a non-metrics request; a metrics request recomputes the whole facet with `lines` | `cache_robustness::an_absent_metrics_facet_is_not_requested_never_computed_null` |
| old version | 4.0 entry / record | rejected → recompute | matrix `IncompatibleMinor`; lib version tests; revision 17 test |

The plan's "facet absent → accepted as not requested" row holds only for an
entry that does not list `metrics`; an entry that lists it and lacks the
facet stays `Inconsistent`, as the cache design already required. Smell grep
(`#[serde(default)]` on `lines`, `unwrap_or_default`/`.ok()` in the cache
reader): none.

### 2.5 Foundation tests and goldens

- `golden/symbols.json` and `symbols.jsonl` regenerated
  (`HUG_UPDATE_GOLDEN=1`); the diff is only `"minor": 0 → 1` and
  `rev.17 → rev.18` (95–96 lines each kind).
- `just test`: **1027 / 1027 passed** (baseline 1021 + 6 new tests).
  `just lint`: green.
- No rendering change, so `just test-l2` was not run. No OS-specific code
  changed (no paths, `cfg`, or process handling), so no cross-check.

### Requirement → test

| Requirement | Test |
|---|---|
| `lines` required, nullable, strict | `symbols_adapter::new_record_fields_read_per_shape`, CLI `cache_robustness` (matrix + two new tests) |
| 4.1 everywhere, older versions recomputed | `wiring::json_symbols_carry_visibility_and_current_schema`, `reports`/`report` envelope prefixes, `symbols_cache::incompatible_record_schema_versions_*`, CLI matrix version cells |
| revision bump invalidates old entries | `cache_upgrade::entry_from_extractor_revision_seventeen_is_not_served_after_the_schema_bump` |
| IDs unchanged | `schema_v2::tests::symbol_ids_survive_schema_and_revision_bumps`, golden diff |
| raw attribute value, conversion | `attributes::tests::attributes_keep_their_raw_argument_text` (Rust, Java, C#, Python, C++, Scala, Swift) |
| flags unchanged by the shared parse | every existing `symbols_flags` test (unchanged, passing) |
| modifier order and dedup | `schema_v2::tests::modifiers_normalize_to_one_order_without_repeats` |

### Docs and skill

READMEs (area, lib, CLI), `cache-design.md` (versions, `rev.18`, `lines`
rows, twenty-three columns), `symbol-schema-design.md` (4.1 history row,
`lines` **planned**, modifier order), `symbol-kinds-and-visibility.md`
(`lines` row **planned**), `symbol-reports.md` (envelope versions). Skill:
`SKILL.md` (4.1, revision 18, attribute sharing, modifier order) and
`api-reference.md`. The area README's historical release-note snippet still
shows the 4.0 envelope it shipped with, on purpose.

### Notes

- Files under `features/2026-10-03-symbol-filters-and-reports/` changed
  during this session from another process (historical log content); they
  are not part of this phase and were left alone.
- Skipped: nothing. Pre-existing failures: none.

## Phase 3

Structured generic parameters, extractor revision 19. One owner; the four
language groups were done in one pass because they share one new reader
module and one matrix file.

### 3.1 Shared generic reader

- New `lib/src/file/extract/generics.rs`: `declared_generics(declaration,
  name, language, source) -> Result<DeclaredGenerics, GenericParamOverflow>`
  returns `params: Vec<GenericParam>` and declaration-level `constraints`.
  One function per language with a node table taken from the Phase 1 probe,
  re-probed this phase with a throwaway in-crate test (deleted) that printed
  each capture's `declaration` node, its parent and the sexp.
- Which node owns the list (the probe settled it):
    - Rust/TS/Java/Python: `type_parameters` field of the declaration
      itself; a TS `const f = <T,>() => …` record's declaration is the
      `arrow_function`, so it is covered.
    - Go: the declaration is the whole `type_declaration`; the list is on
      the name's parent `type_spec`/`type_alias` (so each spec of a group
      gets its own list); `function_declaration` directly. A method's
      receiver `[K, V]` are type arguments, not parameters.
    - C#: unfielded `type_parameter_list` on classes/structs/records,
      fielded on interfaces/methods; `type_parameter_constraints_clause`
      siblings attach to the named parameter.
    - C++: only the `template_declaration` that is the declaration's
      **direct parent**, so a member template does not inherit the class
      template's list and `template <class T> template <class U> void
      A<T>::f(U)` reads `U`.
    - Swift: unfielded `type_parameters` / `type_constraints`; the inline
      bound is under field `name:` (grammar quirk).
    - Scala: found by kind (a function's list is under field
      `parameters:`); invariant parameters are flat `name:` + `bound:`
      siblings, variant ones wrapped.
- Rules as encoded: source order; `position` = index in the written list
  (an unnamed C++ `typename` keeps its slot and has no entry); Rust
  lifetimes keep `'`; `const`/non-type parameters are `Const` and their type
  is not a constraint; bounds are grammar entries (comment children such as
  Rust `/* … */` inside `trait_bounds` skipped), outer whitespace trimmed;
  defaults exclude `=`; variance as written.
- `where`/requirement routing: a predicate whose subject is a bare declared
  parameter name (Rust `type_identifier`/`lifetime`, Swift
  `inheritance_constraint`, C# clause identifier) appends to that
  parameter after its inline bounds; anything else goes to
  `type_info.constraints` once, as written: Rust `T::Item: Copy`,
  `Vec<K>: Send`, `for<'b> …`, a method's `where T: …` on its impl's `T`;
  Swift `U == Int` (an equality requirement is not a bound); C++ `requires`
  (after the template header and trailing after the function declarator);
  a C# clause naming no declared parameter (text without `where`).
- **Decision (carried open from Phase 1): C++ concept-constrained type
  parameters stay conservative.** `template <std::integral T>` is read as
  `Const` with no constraint, because tree-sitter-cpp 0.23.4 gives it the
  exact shape of `template <MyEnum E>`. Resolving against a same-file
  `concept` was rejected: it never helps the common `std::` concepts and it
  needs a per-file concept index for one rare case. Documented as a grammar
  limit and pinned by the matrix's limit row.
- **Decision: Scala bounds keep their operator** (`>: Null`, `<: AnyRef`,
  `: Ordering`) because one list mixes lower, upper and context bounds; the
  context bound's `:` is a separate `bound:`-fielded token, so the text is
  sliced from it. Other languages list only upper bounds and drop the
  keyword.
- **Decision: Python packs are named without stars** (`*Ts` → `Ts`, `**P`
  → `P`), kind `Type`; the schema has no pack/param-spec flag.
- **Older name lists left alone (plan: "only where behavior is
  preserved").** `type_metadata.rs`'s `extract_*_type_parameters` differ
  from the new reader (Rust drops `const` parameters, Scala lists only
  invariant names, Swift may pick a bound's identifier), so projecting them
  would change `TypeMetadata::type_parameters`. Their tests are untouched
  and pass.
- **Ruling 7 (`u16` overflow):** `TreeHuggerError::TooManyGenericParameters
  { path, symbol, count }`; `records_from` now returns `Result` (both
  callers already did). The representable boundary is 65 536 parameters
  (positions 0..=65535); the 65 537th is the error. The plan's "more than
  65 535 parameters" wording was one off; the spec's "unrepresentable"
  wording is what is implemented. The CLI maps it to `errors[].kind =
  "too_many_generic_parameters"` (`cli/src/report.rs::error_kind`).

### 3.2 Record wiring

- `records_from` (`tree_file.rs`) fills `type_info.generic_params` and
  `type_info.constraints` inside the existing `if wanted` block: every
  selected record, no capability, so a declarations-only query carries
  them. Unselected records stay light; `coverage::project` drops
  unselected records outright, so no projection change was needed.
- `EXTRACTOR_REVISION` `"18"` → `"19"`; revision 18 added as a
  `StaleRevision` (`OldKey::Computed`) in `cli/tests/symbols/cache_upgrade.rs`
  (`entry_from_extractor_revision_eighteen_is_not_served_after_the_repairs`:
  a revision-18 entry without `generic_params` is not served; the repaired
  value is `[["T", ["Clone"], "String", 0]]`, then a warm hit).
- Goldens regenerated: diff against the Phase 2 state is only
  `rev.18 → rev.19` (verified by rewriting HEAD's goldens and diffing). The
  `multi/` tree declares no generics.
- Schema stays 4.1 (filling existing fields is not a shape change).

### 3.3–3.6 Language groups and 3.7 coverage

`lib/tests/l1/symbols_generics.rs`, `GENERICS_MATRIX`, one row per language
(16), each `Supported` or `Absent(reason)` with declarations asserted on the
complete index **and** on a declarations-only `SymbolQuery::kinds([kind])`
(which must run only the parse pass), plus `limits`.
`symbols_coverage::every_language_has_one_row_in_each_report_matrix` now
requires a `GENERICS_MATRIX` row per language.

| Language | Status | Positive | Negative |
|---|---|---|---|
| Rust | supported | `Bag<T: Clone = String>`; trait `'a`, `K: 'a + Ord` + `where K: Clone`, `const N: usize = 4`; alias default `B = A`; method `where U: Copy` | impl `T` on neither type nor method; `size` empty; `T::Item`, `Vec<K>: Send`, `for<'b>` → declaration constraints; comment inside bounds skipped |
| TypeScript | supported | class bound+default, interface `string \| number`, alias `keyof T` default, nested `Array<Map<…>>`, arrow function | `size`, `infer` empty |
| JavaScript | absent | — | class, method, function with JSDoc `@template` → empty |
| Go | supported | struct, alias, functions, `K, V any`, union/`~` constraint text, group spec | receiver method `Key`, non-generic interface, plain group spec |
| Python | supported | `T: (int, str)`, `*Ts`, `**P`, method/function lists | `Generic[T]` base → empty; `infer` |
| Java | supported | `T extends Comparable<T> & Cloneable`, method/constructor lists, interface, record | `size` |
| C# | supported | `in`/`out` variance, `where T : class, IComparable<T>, new()`, method `where V : notnull`, struct, record | `Size` |
| C | absent | — | struct, function → empty |
| C++ | supported | type/non-type/pack with defaults, member template, `requires` (both positions), alias template, template template, unnamed slot | `size` gets no class params; `infer` |
| Swift | supported | inline + `where` bounds, protocol composition bound, function, alias | `U == Int` on the class's `U` → method constraint; `size` |
| Scala | supported | `+`/`-` variance, `>: <: :` bounds with operators, context bound on a function, trait, alias | `F[_ <: AnyRef]`'s nested bound not on `F`; `size` |
| PHP | absent | — | class with PHPDoc `@template`, method → empty |
| Perl, Bash, Zsh, Lua | absent | — | function → empty |

Demonstrated grammar limits (rows assert they still hold):

| Grammar | Snippet | Observation |
|---|---|---|
| tree-sitter-typescript 0.23.2 | `interface Variant<in T> {}` | syntax error (TS 4.7 variance) |
| tree-sitter-python 0.25.0 | `class Defaulted[T = str]: pass` | syntax error (PEP 696 default) |
| tree-sitter-cpp 0.23.4 | `template <std::integral T> void constrained(T)` | parses as non-type `parameter_declaration` → `Const`, no constraint |

Extractor gaps, not grammar limits, recorded in the docs: Python
`type A[T] = …` and C# `delegate` produce no record at all, so they carry no
generics (the spec forbids adding records only to fill a metadata cell).

### Requirement → test

| Requirement | Test |
|---|---|
| per-language parameters, bounds, defaults, variance, positions, declaration constraints; ownership; no inference | `symbols_generics::generics_matrix_holds_on_complete_and_declarations_only_paths` |
| declarations-only query equals the complete index and runs only parse | same test (selective path per declaration) |
| grammar limits | `symbols_generics::generics_grammar_limits_hold_for_the_pinned_grammars` |
| `u16` boundary and explicit error; unselected oversized declaration does not fail | `symbols_generics::a_generic_list_beyond_u16_positions_is_an_analysis_error` |
| 16-language row guard | `symbols_coverage::every_language_has_one_row_in_each_report_matrix` |
| JSON / JSONL / `--json-full` carry it (normal invocation, persistent cache on) | CLI `report::generic_parameters_reach_every_json_format` |
| CLI error kind, exit 1, non-selecting command unaffected | CLI `report::an_unrepresentable_generic_list_fails_only_a_command_that_selects_it` |
| revision bump invalidates old entries; warm hit after | CLI `cache_upgrade::entry_from_extractor_revision_eighteen_is_not_served_after_the_repairs` |
| older name lists unchanged | existing `TypeMetadata` tests, unchanged and passing |

Input robustness matrix: not applicable this phase. No reader of a file
format changed; `GenericParam`/`TypeFacet` deserialization is unchanged
(their pre-existing `#[serde(default)]` on `generic_params`,
`constraints` and `GenericParam::constraints` means absent = empty, which is
the serialization's `skip_serializing_if = "Vec::is_empty"` encoding). Phase
7's cache robustness rows own stating that.

### Gates

- `just test`: **1035 / 1035 passed** (Phase 2: 1027; +3 lib generics
  tests, +2 CLI report tests, +1 cache-upgrade test = 1033; the other two
  are not this phase's, most likely the concurrent session's below).
    - One earlier run in this phase reported the known load-sensitive
      `TIMEOUT` of `file_identity::god_files_paths_are_relative_to_any_spelling_of_the_root`
      (Phase 1 note); alone it passes in 6 s, and the two final full runs
      passed it.
- `just lint`: **fails, not from this phase.** The only clippy error is
  `type_complexity` at `lib/src/file/extract/visibility/mod.rs:81`
  (`VisibilityMemo::cpp_sections`), plus a `dead_code` warning at
  `qualified_name.rs:132` (`file_namespaces`). Both files were unmodified
  at the start of this session and were edited by another process during it
  (mtime 17:19 PDT, mid-phase), alongside `classify.rs`, `declaration.rs`
  and `static_flags/mod.rs`. Left untouched. Clippy run non-fatally over
  `--all-targets` of both crates reports nothing in any file this phase
  changed.
- No rendering change, so `just test-l2` was not run. No OS-specific code
  (no paths, `cfg`, processes), so no cross-check. The overflow tests parse
  a ~460 KB generated file (4.7 s locally for two parses in the lib test,
  1.6 s in the CLI test); slower CI hosts stay well under the 30 s limit.

### Docs and skill

- `docs/symbol-schema-design.md`: new "Generic parameters" section (rules,
  example, per-language table, limits, adapter loss), the "still empty"
  paragraph and the "when computed" table corrected.
- `docs/symbol-reports.md`: `too_many_generic_parameters` error kind.
- `docs/cache-design.md`: `rev.19`.
- Skill: `SKILL.md` (generics bullet, revision 19, stale-revision list),
  `api-reference.md` (record facets filled by extraction).

## Phase 4

Written modifiers and raw attributes, extractor revision 20. One owner; the
three language groups were done in one pass because they share one reader
per concern (`modifiers.rs`, `attributes.rs`) and one matrix file.

### Probe (throwaway, deleted)

An in-crate test (`src/file/probe_tmp.rs`, removed with its `mod` line)
printed each capture's `declaration` node, its children and its preceding
siblings for one snippet per language. What it settled:

- TypeScript method decorators are `decorator:`-fielded **`class_body`
  children before the `method_definition`**; field decorators are inside
  `public_field_definition`; an exported class's decorator written before
  `export` is on the `export_statement`; `export @d class` puts it on the
  `class_declaration`. JavaScript (0.25.0) keeps method and field
  decorators inside the member node.
- A TS constructor parameter property is its own `required_parameter`
  record (its `readonly` and decorator are the parameter's).
- C++ `override`/`final` are `virtual_specifier` nodes inside the
  `function_declarator`; a class's `final` is a `virtual_specifier` child of
  the `class_specifier`; `virtual` is an anonymous token on the member
  declaration.
- Rust `let mut x` puts `mutable_specifier` directly on the
  `let_declaration`; `&mut self` and `mut self` both put it on the
  `self_parameter` (`&` distinguishes them); `self: &mut Self` is a
  `parameter` whose `type` is a `reference_type`.
- C#: each keyword is its own `modifier` node; Java: one `modifiers` node
  with anonymous keyword tokens; Swift: `modifiers` → `inheritance_modifier`
  / `member_modifier` / `mutation_modifier`; Scala: `modifiers` with
  anonymous tokens; PHP: `abstract_modifier` / `final_modifier`.

### 4.1 Modifier reader

- New `extract/modifiers.rs::declaration_modifiers(declaration, name,
  language)`; `records_from` extends `record.modifiers` with it inside the
  `if wanted` block, before the one `normalize_modifiers` call. It reads
  only tokens the declaration node itself owns (direct children, the
  declaration's own modifier list, or for C++ the function declarator that
  declares the captured name via `declared_function_named`), so a keyword
  in a body, a string, a type (`size: readonly string[]`) or a parameter
  never marks the declaration. No source text is scanned.
- Per language (the table now lives in `docs/symbol-schema-design.md`):
  Rust `Const`/`Unsafe` (`function_modifiers`), `unsafe trait`, `Mut` on
  `let mut`, `static mut`, `mut x: T`, `mut self` and on a method whose
  receiver is written with `mut`; C++ `virtual`, `override`, `final`
  (members and classes); TS `abstract`, `override`, `Readonly`; Java
  `abstract`, `final`; C# `abstract`, `override`, `virtual`, `sealed`,
  `Readonly`; Swift `final`, `override`, `mutating`; Scala `abstract`,
  `final`, `override`; PHP `abstract`, `final`.
- **Decision: a `&mut self` receiver gives the method `Mut`, but the
  `self` parameter record gets `Mut` only for `mut self`.** The spec's
  "receiver `mut`" means the method's receiver is written with `mut`
  (`&mut self`, `mut self`, `self: &mut Self`); the parameter record's own
  `Mut` is the binding rule, and in `&mut self` the `mut` is the borrow's,
  not the binding's.
- **Decision: keywords outside the spec's language table are not read**:
  Scala/Java `sealed`, Swift `open`/`nonmutating`, PHP `readonly`, C++
  `constexpr`/`const` member functions. PHP `readonly` arguably meets the
  generic "explicit declaration readonly" rule, but the table lists only
  `abstract`/`final` for PHP; adding it is a one-line change if wanted.
- Java and C# read the modifier list from the whole field/local
  declaration, never the declarator: the host climb that
  `declaration_attributes` already did inline is now the shared
  `attributes::modifier_host`, used by both readers.

### 4.2 Attribute population

- `declaration_attributes` is hoisted out of the Flags closure in
  `records_from`: it runs once per selected declaration, and both the
  record's `attributes` (`Attribute::from`) and the flag readers take that
  one list.
- New TS/JS decorators (`ecma_decorators`): the `export` statement's, then
  the TS `class_body` siblings directly before the member, then the
  declaration's own; a parameter's are the parameter's.
- Decorator expressions (Python, TS, JS share `decorator_attribute`): a
  path is the name with no arguments; a call of a path is the path plus
  the call's arguments; parentheses are read through (unchanged flag
  behavior for `@(deprecated)`); **anything else keeps its whole
  expression, as written and trimmed, as the name, with no arguments**
  (`@(lambda f: f)` → `lambda f: f`, `@registry["  k "]` keeps the literal
  whitespace). Before, Python token-joined such expressions
  (`lambdaf:f`), which also compacted whitespace inside literals.
- No flag changed: TS/JS deprecation reads only the doc block, and every
  other flag reader matches exact names, which no "whole expression" name
  can equal. `symbols_flags` is unchanged except for the assertion below.
- Rust inner attributes (`#![…]`) and C# `[assembly: …]` never attach
  (outer-only Rust walk; `global_attribute` is not under a declaration);
  both pinned in the matrix.

### Demonstrated grammar limit

| Grammar | Snippet | Observation |
|---|---|---|
| tree-sitter-typescript 0.23.2 | `class A {\n  @(flag ? one : two) m() {}\n}` | syntax error: the decorator is `parenthesized_expression(identifier, ERROR(…))`; the grammar accepts only a path, a call of a path or a parenthesized one of those |
| tree-sitter-javascript 0.25.0 | same snippet | syntax error (top-level `ERROR`); same restriction |

So the spec's "unusual decorator expression" case exists only in Python.
`@f()()` and `` @g `x` `` also fail to parse in TS (probe).

### 4.3–4.5 Language groups and 4.6 merge

`lib/tests/l1/symbols_modifiers.rs`, `MODIFIERS_MATRIX`, one row per
language (16), checked on the complete index **and** a declarations-only
`SymbolQuery::new().kinds([kind])` that must run only the parse pass;
`symbols_coverage::every_language_has_one_row_in_each_report_matrix` now
requires a `MODIFIERS_MATRIX` row per language.

| Language | Status | Positive | Negative |
|---|---|---|---|
| Rust | supported | `const unsafe fn` + 5 attributes (repeat `#[allow()]` ×2, `cfg_attr` raw, `doc = "a  b"`), `async unsafe fn` + `tokio::test`, `mut` param/let, `static mut`, `unsafe trait`, `&mut self`/`mut self`/`self: &mut Self` methods, `const fn new` (`Static`, `Const`) | file `#![allow]` not on the first item; `unsafe {}` / string in body; `takes(mut count)` unmarked; `fixed`; `read(&self, other: &mut u8)` |
| TypeScript | supported | `@sealed` before `export`, `abstract` class/method/property, `override` + two decorators, `readonly` field/interface property/parameter property, parameter decorator, `static async` | `run` (no inherited decorator), constructor (parameter's decorator not its), `size: readonly string[]` |
| JavaScript | supported (decorators only) | class, method (two), field, `export @tagged class`, `static async` | no TS modifier invented |
| Go | absent | — | function, struct with a field tag |
| Python | supported (decorators only) | repeated `app.route` with raw value, `dataclass`, `abstractmethod`, `functools.cache`, lambda and subscript expressions, `staticmethod` method (`Static`), `async` | module-level `@staticmethod` gives no `Static`; no decorator implies a modifier |
| Java | supported | class annotations (`Deprecated`, array value), repeated `@Override`, `abstract`/`final`, both declarators of one field, `static final`, `final` parameter and local | keyword in a body string |
| C# | supported | `Serializable`, three attribute lists incl. `[return: NotNull]`, `abstract`/`virtual`/`override sealed`, `readonly` fields and struct, `sealed` class, `static async` | `[assembly: …]` not on `Shape`; keyword in body |
| C | absent | — | `static` stays `StaticStorage` only; keyword in a string |
| C++ | supported | `virtual`, `override`, member and class `final`, two `[[…]]` lists, `static` field, attribute through a member `template` | `const` member function not `Const`; `plain` with a keyword string; function-pointer parameter |
| Swift | supported | `@available`/`@objc`, `final` class/property, `override`, `mutating` (also in a protocol), `static`, `async` | `nonmutating`, keyword in a body string |
| Scala | supported | annotation with value, `abstract` class, `final override` method + `@inline`, `final val`, object member `Static` | abstract `def` without keyword, `sealed trait` |
| PHP | supported (modifiers only) | `abstract`/`final` classes and methods, `static` | `#[Attribute]` not read (outside the required set) |
| Perl, Bash, Zsh, Lua | absent | — | keyword-named function, keyword in a string; Perl `:lvalue` sub attribute not read |

Plus `symbols_modifiers::modifiers_serialize_in_one_order_whatever_the_source_order`
(C# `override sealed` vs `sealed override`, `static public async` vs
`async static public` give the same JSON) and
`modifiers_grammar_limits_hold_for_the_pinned_grammars` (the two rows above).

### Revision, cache, goldens

- `EXTRACTOR_REVISION` `"19"` → `"20"`; revision 19 added as a
  `StaleRevision` (`OldKey::Computed`):
  `cache_upgrade::entry_from_extractor_revision_nineteen_is_not_served_after_the_repairs`
  (a revision-19 entry without `modifiers`/`attributes` is not served; the
  repaired value is `[[Const, Unsafe], [[inline, null], [must_use,
  "\"keep\""]]]`, then a warm hit).
- `cache_robustness` gained an `attributes` column (on `static_method`,
  whose `@staticmethod` is the control): absent and `[]` served as no
  attributes (the field's `skip_serializing_if = "Vec::is_empty"`
  encoding), every other shape rejected. That makes twenty-four columns.
  Schema stays 4.1: filling existing fields is not a shape change.
- Goldens regenerated; the diff is `rev.19 → rev.20` plus one `Mut` on
  `grow(&mut self)` in `multi/rust.rs` (the multi tree has no attributes).

### Changed tests (intended)

- `symbols_flags::flags_are_set_exactly_by_each_languages_markers` asserted
  "the attribute facet stays empty", the old contract. It now asserts the
  facet is identical with and without `Capability::Flags` (same parse,
  flags do not change it); the flag assertions themselves are unchanged.

### Requirement → test

| Requirement | Test |
|---|---|
| per-language modifiers and attributes; own tokens only (body, string, type, parameter negatives); wrapper/file attributes not spread; repeats, empty `()`, raw values, `cfg_attr` unexpanded; existing Static/StaticStorage/Async unchanged | `symbols_modifiers::modifiers_matrix_holds_on_complete_and_declarations_only_paths` |
| declarations-only query carries them and runs only the parse pass | same test (selective path per declaration) |
| one serialized order whatever the source order | `symbols_modifiers::modifiers_serialize_in_one_order_whatever_the_source_order` |
| TS/JS decorator grammar limit | `symbols_modifiers::modifiers_grammar_limits_hold_for_the_pinned_grammars` |
| 16-language row guard | `symbols_coverage::every_language_has_one_row_in_each_report_matrix` |
| one attribute parse shared with flags; flags unchanged | `symbols_flags::flags_are_set_exactly_by_each_languages_markers` (facet equal with and without Flags; every flag cell unchanged), all other `symbols_flags` tests unchanged |
| JSON / JSONL / `--json-full`, normal invocation, cache write then warm hit (`parsed 0 files`) | CLI `report::written_modifiers_and_attributes_reach_every_json_format` |
| revision bump invalidates old entries; warm hit after | CLI `cache_upgrade::entry_from_extractor_revision_nineteen_is_not_served_after_the_repairs` |
| cache reader shapes for `attributes` | CLI `cache_robustness::persisted_symbol_fields_are_served_or_recomputed_per_shape` (new column) |
| existing raw-value reader unchanged | `attributes::tests::attributes_keep_their_raw_argument_text` (unchanged) |

Input robustness matrix: no new file-format field. `attributes` and
`modifiers` were already in the record schema; their cache shapes are now
pinned by the `attributes` column and the existing two `modifiers` columns
(absent/`[]` served, `null`, wrong type, one/every wrong element, duplicate
key and trailing content rejected). Smell grep: their `#[serde(default)]`
is the format's own encoding (`skip_serializing_if = "Vec::is_empty"`), so
absent = empty by definition; no `unwrap_or_default`/`.ok()` on the reader.

### Gates

- `just test`: **1041 / 1041 passed** (Phase 3: 1035; +3 lib modifiers
  tests, +1 CLI report test, +1 cache-upgrade test, +1 other — probably the
  concurrent session's).
- `just lint`: green (the Phase 3 note's foreign `type_complexity` /
  `dead_code` errors are gone).
- `just check-tier-coverage tree-hugger`: nothing stranded.
- No rendering change, so `just test-l2` was not run. No OS-specific code
  (no paths, `cfg`, processes), so no cross-check.

### Docs and skill

- `docs/symbol-schema-design.md`: "still empty" paragraph corrected (only
  type relations remain empty), "when computed" row for attributes and
  written modifiers, the Attributes and Modifiers section now states the
  modifier table, attribute rules and examples, unread forms, the TS/JS
  grammar limit, and the adapter loss (attributes and every modifier other
  than Static/StaticStorage/Async).
- `docs/symbol-kinds-and-visibility.md`: record facts row; decorators fill
  `attributes` without marking flags.
- `docs/cache-design.md`: `rev.20`, `attributes` record-field row,
  twenty-four columns.
- `lib/README.md`: schema-record bullets for modifiers and attributes.
- `lib/src/file/query.rs`: `Capability::Declarations` doc lists generics,
  written modifiers and attributes (drift: it predated Phase 3's generics).
- Skill: `SKILL.md` (written-modifiers bullet, attribute facet filled,
  revision 20, stale-revision list, twenty-four columns),
  `api-reference.md` (record facets filled by extraction).

### Comment drift found and fixed

- `symbols_flags.rs` module doc and assertion said the attribute facet
  stays empty → now "the same with or without the flags".
- `attributes.rs::declaration_attributes` doc listed no TS/JS decorators →
  lists them and the file-level forms that never attach.
- `modifiers.rs` module doc said it reads only `async` → names
  `declaration_modifiers`.
- `query.rs` `Capability::Declarations` (above). The `Capability::Metrics`
  doc still says "(`parameters` and `has_docs`)"; that is Phase 5's
  metric and is left for Phase 5 / 9.4.

### Notes

- Files under `features/2026-10-03-symbol-filters-and-reports/` and
  `fixes/_unscheduled/` changed during this session from another process;
  not part of this phase, left alone.
- Skipped: nothing. Pre-existing failures: none.

## Phase 5

Callable body-line metric (`metrics.lines`), extractor revision 22. One
owner; the language groups share one reader (`callable_body_span`) and one
matrix, so they were done in one pass.

### Probe (throwaway, deleted)

An in-crate test (`src/file/tree_file/probe_tmp.rs`, removed with its `mod`
line) and an L1 test (`tests/l1/probe_tmp.rs`, removed) printed every
callable's anchor, declaration, their children and the stored body span
for every primary fixture and one snippet per language. What it settled:

- Every grammar puts a callable's body under the `body` field (Rust,
  TS/JS, Go, Java `constructor_body`, C#, C/C++, Swift `function_body`,
  Scala, PHP, Perl, Bash/Zsh, Lua, Python). The old reader's type-body
  fallbacks (`class_body`, `object_type`, …) are never a callable's body.
- **C/C++ functions had no body span**: their anchor is the
  `function_declarator`; the body is on the declaration
  (`function_definition`). Only C/C++ callables have an anchor different
  from their declaration (31 fixture records). Defaulted/deleted members
  are `function_definition`s with `default_method_clause` /
  `delete_method_clause` and no `body`; a function-try-block's `body` is
  the `try_statement`.
- **C# expression bodies kept `=>`**: the `body` is an
  `arrow_expression_clause` (`=>` + expression), also for local functions
  and constructors.
- JS/TS arrows and Scala `def f = 1` already keep the introducer outside
  the field; a returned arrow (`() => () => 1`) is the outer arrow's whole
  expression body.
- **Python and Lua**: the `block` starts at the first statement; a comment
  before it (Lua: also after it) is a child of the function, beside the
  block. **Lua omits the block of an empty body** (`function f() end` has
  only `parameters` and `end`), so its body span was absent.
- **Trailing whitespace inside the node**: tree-sitter-scala's braceless
  `indented_block` runs over the blank lines up to the dedent
  (`def b(x: Int): Int =\n  val y = x\n  y\n\ndef after = 1` spans to the
  `def after` line); a Perl `block` includes its line break.
- Go `type S struct {…}` and Go func literals, JS/TS function expressions,
  PHP closures/arrow functions, Python lambdas, Rust closures are not
  callable records (unchanged).

### 5.1 Body node reader

- New `classify.rs::callable_body_span(declaration, language, source)`,
  used by `records_from` for Function/Method/Constructor records; other
  kinds keep `body_span_for_declaration(anchor)` unchanged. It reads only
  the declaration's `body` field, so a default value, return type or
  nested/returned callable is never taken; C# `arrow_expression_clause`
  gives its expression; Python/Lua run from the first child after `:` /
  `parameters` (a comment on the header's own line excepted) to the last
  child before `end`; an empty Lua body is the empty span at `end`.
- **Decision: the stored span ends at its last non-whitespace byte**
  (`without_trailing_whitespace`, end location recomputed in byte columns
  as tree-sitter counts them). Otherwise every Scala 3 function followed by
  a blank line would count one extra line, and the span would hold no body
  text there. The spec's "exact same body node" is kept as "exact same
  span": `lines` is computed from the stored `source.body_span`, never
  separately.
- **Decision: an empty Lua body is `Some(0)`, not `None`**. The function
  has an implementation body that happens to be empty; `None` is reserved
  for callables without one (prototypes, abstract/interface methods).
  This is the one form that produces the spec's "empty byte span".
- `body_node_for_declaration` (used for `declaration_text` and
  `signature_text` cuts) is untouched, so declaration and signature text do
  not change (all header/signature matrices unchanged).

### 5.2 `lines` computation

- `metrics.rs::body_lines(kind, body_span)`: `None` unless
  Function/Method/Constructor with a span; `Some(0)` for an empty span;
  else `end.line - start.line + (end.column > 1)`. `symbol_metrics` takes
  the record's body span, so the three metrics still arrive together under
  `Capability::Metrics` and `lines` cannot disagree with the stored span.
- God-files: `god_files/analysis.rs` does not read `source.body_span` (only
  a test comment mentions it) and keeps its declaration-span unit; all
  god-files tests unchanged and green.

### Revision, cache, goldens

- **Concurrent session:** a second session (review 9 of
  `2026-10-03-symbol-filters-and-reports`) is working in this worktree and
  had already moved `EXTRACTOR_REVISION` `"20"` → `"21"` with a
  revision-20 `StaleRevision` before this phase started. This phase moves
  it `"21"` → `"22"` and adds
  `cache_upgrade::entry_from_extractor_revision_twenty_one_is_not_served_after_the_repairs`
  (a revision-21 entry with no `body_span` on a C function and
  `lines: null` is not served; repaired `[2, 3]`, then a warm hit).
- Schema stays 4.1: `lines` already existed; filling it changes no shape.
- `cache_robustness`: the `metrics` column's control and
  `an_absent_metrics_facet_is_not_requested_never_computed_null` pinned the
  old always-`null` value; both now expect `lines: 2` for the Python
  fixture's `instance_method`. The robustness rows themselves are
  unchanged (still twenty-four record columns; `lines` already had its
  column from Phase 2).
- Goldens regenerated (`HUG_UPDATE_GOLDEN=1`); the record-level diff
  against `HEAD` is the revision, Phase 4's `Mut`, and four new
  `body_span`s on the multi tree's C/C++ callables (`point_sum`,
  `reset_origin`, `geometry::Vector::Vector`, `geometry::dot`). No other
  body span moved (checked by comparing every record's span against HEAD).

### Changed tests (intended)

- `symbols_metrics::metric_forms_outside_the_fixtures` compared the whole
  facet with `lines: None`; it now compares `parameters`/`has_docs` only
  (its subject), `lines` being the new matrix's.
- `symbols_query::flags_and_metrics_run_with_the_declarations_only`:
  `old`/`helper` now `lines: Some(1)`.

### Requirement → test

| Requirement | Test |
|---|---|
| every fixture function's `lines` (with `parameters`/`has_docs` unchanged) | `symbols_metrics::metrics_match_each_fixture_function` (new `lines` column in `METRICS_MATRIX`) |
| `lines` agrees with the stored span on every record; non-callables `None` even with a body span | same test (per-record check against the span) |
| per-language body forms: `{}` = 1, multi-line, comments/blank/nested counted, expression bodies without `=>`/`=` (TS/JS arrow, C#, Scala), returned callback / nested callable not the body, defaults not the body (Python lambda, Swift closure, TS arrow), return type not the body (TS object type), abstract/interface/protocol/prototype `None`, C++ defaulted/deleted/pure `None`, C++ try-block, out-of-class C++ member, Python/Lua comments, Lua empty = 0, Scala braceless with trailing blank line, Perl block, class/module/trait `None` | `symbols_metrics::body_lines_are_counted_from_the_callables_own_body` (`BODY_LINES_MATRIX`, one row per language; metrics-only query and complete index) |
| CRLF and LF count the same | same test (every row also run with CRLF) |
| formula incl. span ending at column 1 (spec example 3 → 6:1 = 3) | `file::extract::metrics::tests::body_lines_follow_the_spec_formula` (unit; trimmed record spans never end at column 1) |
| `lines` present inside a facet, `null` where unsupported, facet absent when unrequested | `symbols_metrics::lines_key_is_present_null_or_absent_with_its_facet` |
| 16-language row guard | `symbols_coverage::every_language_has_one_row_in_each_report_matrix` (`BODY_LINES_MATRIX` added) |
| normal invocation; `--json` has no facet; widening an entry without metrics reparses once (`0 hits, 2 partial hits … parsed 2 files`); warm hit (`parsed 0 files`) with equal values | CLI `report::body_lines_reach_the_full_index_across_cache_widening` |
| revision bump invalidates revision-21 entries | CLI `cache_upgrade::entry_from_extractor_revision_twenty_one_is_not_served_after_the_repairs` |
| cache reader shapes of `lines` | CLI `cache_robustness::*` (Phase 2 column, unchanged rows; control value updated) |
| god-files unchanged | existing god-files tests, unchanged |

Input robustness matrix: no new file-format field (`metrics.lines` and its
column exist since Phase 2). Smell grep on the reader is unchanged.

### Gates

- `just test` (tree-hugger): **1052 / 1052 passed**.
- `just lint`: `tree-hugger-cli` green. `tree-hugger` fails on one
  `clippy::type_complexity` in `lib/tests/l1/symbols_shared_context.rs:506`,
  a file the concurrent review-9 session was editing at the time (18:09 →
  18:10); not touched here. With that one lint allowed
  (`cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets
  --all-features -- -D warnings -A clippy::type_complexity`) both crates
  are clean, and no new code here has a complex type.
- `just check-tier-coverage tree-hugger`: nothing stranded. New tests are
  plain L1 names in the existing `l1` and `symbols` binaries.
- No rendering change, so `just test-l2` was not run. No OS-specific code
  (byte spans only; CRLF covered by the matrix), so no cross-check.

### Docs and skill

- `docs/symbol-kinds-and-visibility.md`: `lines` row is current (no longer
  planned); new `### Body lines` section (formula, body forms table,
  trailing whitespace, defaults/nested callables).
- `docs/symbol-schema-design.md`: `lines` paragraph current, example value,
  `body_span` paragraph for callables in the Source Facet section.
- `docs/cache-design.md`: `rev.22`.
- `lib/src/file/query.rs`: `Capability::Metrics` doc now names `lines`
  (the drift Phase 4 left for this phase).
- Skill: `SKILL.md` (body-span/`lines` bullet, revision 22 and stale list),
  `api-reference.md` (the "`None` until the metric lands" note removed).

### Comment drift found and fixed

- `metrics.rs::symbol_metrics` doc said `lines` "is not computed yet and is
  always `None`" → points at `body_lines`.
- `query.rs` `Capability::Metrics` (above); `api-reference.md` (above).
- `symbols_metrics.rs` module doc named the old feature and only two
  metrics → names all three and the new matrix.

### Notes

- Files changed during this phase by the concurrent session (not part of
  this phase): `lib/src/file/extract/qualified_name.rs`,
  `visibility/scala.rs`, `lib/tests/l1/symbols_shared_context.rs`, the new
  `symbols_namespace_bodies.rs` and its `mod` line, the revision-20 stale
  test, and the 2026-10-03 feature files.
- Skipped: `just test-l2` (no rendering change). Pre-existing failures:
  none; the one lint error is the concurrent session's.
- Re-run at close (after the docs/comment edits): the concurrent session
  was mid-edit again. The library unit-test target did not compile
  (`lib/src/analysis/tests.rs`: unresolved `find_container_symbol`, a call
  with 3 of 4 arguments), so `just test` stopped. The targets that compile
  were run directly: `tree-hugger --test l1` 578/579 (the one failure is
  that session's new
  `symbols_shared_context::repeated_names_keep_shared_context_linear_and_results_intact`;
  its one skipped test is its `#[ignore]`d step-count printer) and
  `tree-hugger-cli` 292/292. The earlier full `just test` (1052/1052)
  included this phase's lib unit test
  (`metrics::tests::body_lines_follow_the_spec_formula`).

## Phase 6

Declared type relations (`relations.extends` / `relations.implements`) for
every language except C#, Swift and Scala, plus Rust impl ownership.
Extractor revision 22 → **23** (read right before the bump; the concurrent
session had left it at 22). Schema unchanged (4.1): only existing fields are
filled. Tasks 6.1–6.4 and 6.8 are done. **6.5–6.7 stay open**: ruling 1
(how to classify mixed C#/Swift/Scala base lists) is still unanswered, and
the plan says to leave them open rather than guess.

### Probe (throwaway, deleted)

A `/tmp/th-probe6` crate pinned to the grammar versions in
`lib/Cargo.toml` (deleted at the end of the phase) dumped base-list trees for
TS, JS, Java, PHP, Python, Go, C++ and Rust. The Phase 1 tables held. Facts
the readers depend on:

- TS `extends_clause` = `value:` + separate `type_arguments:`, so the written
  text runs from the value through its arguments. JS `class_heritage` holds
  the expression directly.
- Java: `superclass`, `super_interfaces` → `type_list`, `extends_interfaces`
  → `type_list` (enum and record `interfaces:` too).
- PHP `qualified_name` keeps the leading `\` as `prefix:` tokens, so the root
  anchor is read from the text.
- Python `superclasses: argument_list` entries are `identifier`, `attribute`,
  `subscript`, `call`, `list_splat`, `keyword_argument` and
  `dictionary_splat`.
- Go `interface_type` → `type_elem` (one child for an embed; a union or
  `~` term has several or a `negated_type`), plus `method_elem`.
- C++ `base_class_clause`: an `access_specifier` node, an anonymous
  `'virtual` token, then the type node.
- Rust supertraits: `bounds: trait_bounds` entries (`lifetime`,
  `removed_trait_bound` `?Sized`, `higher_ranked_trait_bound`); a negative
  impl has an anonymous `'!` token; `impl dyn Tr` self type is
  `dynamic_type`.

### 6.1 Per-file declaration index

`tree_file/relations.rs::Declarations` is built once per file at the end of
`records_from`, from **every** record (selected kinds or not): in-file
qualified name → record indexes, plus scope-naming records by declaration and
anchor node id. One `count_shared_context_steps(records.len())` for the build
and one step per scope probed, so `symbols_shared_context` still sees linear
growth. No per-relation or per-symbol rescan.

### 6.2 Resolution rules

`Declarations::resolve` (doc comment carries the contract). The written path
(`extract/relations.rs::BasePath`: segments with generic arguments dropped,
plus an anchor) is joined onto each enclosing scope, innermost first. Rust
`crate::`, `self::` and `super::` and a PHP leading `\` anchor the path.

- The **first** scope at which any declaration has the joined name decides,
  and it resolves only if that is the only declaration and its kind is
  eligible. So a nearer type alias, variable or duplicate shadows an outer
  type instead of letting the lookup fall through.
- A visible type parameter shadows. That means the owner's own parameters
  and those of enclosing declarations (`declared_generics`), or an impl's
  `<T>`.
- Rust relative paths stop at the nearest module, because an item does not
  see its parent module's names.
- Imports are not read. An imported name has no declaration at its scope, so
  it stays unresolved.
- Eligible kinds: base targets Class/Interface/Trait/Type; impl self types
  Type/Class/Enum; impl traits Trait.
- Targets keep the written text and the in-file qualified name, never the
  scan-root prefix (`finish` does not touch relation targets).
- Lists keep source order. Equal text appears once; distinct instantiations
  stay distinct.

### 6.3 Plain-language readers

`extract/relations.rs::declared_bases(name, …)` reads the name's parent node
(the declaration that writes the list, for example a Go `type_spec` or a C++
`class_specifier`), one function per language: TS/JS, Java, PHP, Python, Go,
C++ and Rust supertraits. C, Perl, Bash, Zsh and Lua read nothing; their
matrix rows hold the forms that must not be inferred (`@ISA`, `use parent`,
metatables, a C struct embed).

Decisions recorded here (also in `docs/type-relations.md`):

- Python `*bases` is kept as written and unresolved. `**options` and
  keyword arguments are not bases.
- Go: syntax cannot tell a named type embedded in an interface constraint
  from an embedded interface. Only the predeclared non-interface types
  (`bool`, `int*`, `uint*`, `float*`, `complex*`, `byte`, `rune`, `string`,
  `uintptr`) are excluded; `error`, `any` and `comparable` are kept.
- Rust supertraits drop `?Sized`. A higher-ranked bound is kept as written
  with no path.

### 6.4 Rust impl ownership

- `extract/rust_impl.rs::impl_header` reads the trait as written, negativity,
  the self type's text and path, and the impl's own type parameters.
  `relations::impl_relations` walks the tree once per Rust file. Every
  positive trait impl, empty ones included, adds its trait to the resolved
  self type's `implements` (selected records only, file order, deduplicated by
  text) and to each direct member's.
- The old "exactly one same-named trait in the file" link loop in
  `records_from` (which the concurrent session had just rewritten to an index)
  is gone, replaced by the shared resolver. Its `count_shared_context_steps`
  import went with it.
- Containment: `ContainerLookup::SameFileType` now carries
  `impl_item: Option<Node>` (Rust only), and `definitions()` copies it onto
  the new `ExtractedSymbol::container_impl`. Under `RecordFacts::containment`
  (new field, set from `PassPlan::containment`), every record whose container
  search ended at an impl is pre-linked:
    - resolved self type: `relations.container` and `parent` are that record;
    - unresolved: `relations.container` is the self type as written, with no
      ID and no parent (ruling 6).
- This covers parameters of impl methods too. By existing design (owner
  matrix row `W::rooted::self`) those belong to the impl type. Linking only
  methods would have left `fn by_ref(&self)` in `impl Tr for &Local`
  unlinked while its `self` joined `Local`.
- `analysis::link_containers` keeps pre-linked records and adds them to their
  parent's `members` in record order. `resolve_containers` skips them.
- Persistence (ruling for Phase 7, decided now so the existing equivalence
  tests stay green): no record retains the self-type resolution or the
  written self-type text (`container_name` stays the bare last segment, and
  qualified names are unchanged). So `coverage::record_passes_suffice` now
  takes the language, and Containment on a Rust entry parses once
  (`WidenedCapabilities { parsed: true }`). This is the spec's "reparse once on
  that path" fallback. Pinned by a new `symbols_cache` widening row.
- Qualified names, `identity.container_name`, `identity.container_kind` and
  symbol IDs are unchanged.

### Changed tests (intended; each follows a spec rule)

| Test | Old expectation | New, and why |
|---|---|---|
| `symbols_kinds::rust_trait_impl_members_record_the_implemented_trait` | `Display` | `fmt::Display`: full written text (ruling 4) |
| `symbols_kinds::rust_impl_members_are_members_of_the_implemented_type` | `impl PrivateTrait for String` member has no container | textual container `String`, no ID, no parent (ruling 6) |
| `symbols_owner_identity::a_sole_same_named_record_never_overrides_an_explicit_owner` | container `None` | no container **ID** (textual container allowed, ruling 6) |
| `symbols_owner_identity` OWNER_MATRIX row "unqualified, rooted or imported impl" | `use a::V; impl V` → owned by `a::V` | case removed and the row renamed: an imported name stays unresolved (spec); the case now lives in `symbols_relations::rust_impl_members_join_the_resolved_self_type_only` |
| `symbols_shared_context::check_rust_traits` (the concurrent session's file) | `implements Probe` with no ID (several traits share the name) | ID of the impl's own module's `r9_i::Probe`: resolved by lexical scope, not bare name |
| goldens `symbols.json` / `symbols.jsonl` | — | `extractor_version` rev.23 on every record; `typescript.ts` `Panel` gains `implements Sized` with the same-file ID. No other record changed (compared record by record) |

The owner-identity and shared-context tests belong to the review cycle of
`2026-10-03-symbol-filters-and-reports`. They were edited only where this
spec's rules changed the expectation, with a comment at each site.

### Requirement → test

| Requirement | Test |
|---|---|
| Full written text; access/`virtual` not in C++ text; source order; dedup; instantiations kept | `symbols_relations::relations_matrix_holds_on_complete_and_declarations_only_paths` (rows Rust, TS, JS, Go, Python, Java, C++, PHP) |
| Qualified external target sharing a local short name stays unresolved | same matrix: Rust `other::Base`, TS `other.Root`/`lib.Base`, Java `other.Root`, C++ `other::B`, PHP `\Lib\Base`, Python `mod.Base` |
| Duplicate names in different scopes resolve lexically | matrix: Rust `a::Dup`, Python `Outer.Base`, Java `p.Outer.Base`, C++ `ns::N : B` → `ns::B`; Rust `b::UsesOuter: Base` stays unresolved (module boundary) |
| Ambiguous / alias / type-parameter / dynamic targets stay unresolved | matrix: Rust `Amb`, TS `Alias`, C++ `Alias` and `template <class T> … : T`, Java `class Param<Root> extends Root`, TS `Gen<T> extends T<string>`, JS/TS `mix(Root)`, Python `make()` |
| `crate::`/`self::`/`super::`, PHP `\` anchors | matrix: Rust `a::UsesDup`, PHP `\App\Other` |
| Go named embeds only; struct embeds are composition | matrix Go row (`int`, `~string`, unions excluded; `S` has none) |
| Python `metaclass=`/`**` not bases | matrix Python `Child` |
| Unselected records feed the index | matrix declarations-only path (owner kind only; targets of other kinds still get IDs) |
| C, Perl, Bash, Zsh, Lua: none, with negative forms | matrix Absent rows assert no relation on any record |
| C#, Swift, Scala pending (ruling 1) | matrix Pending rows assert nothing is read yet |
| One row per language | `symbols_coverage::every_language_has_one_row_in_each_report_matrix` (`RELATIONS_MATRIX` added) |
| Multiple and empty Rust impls aggregate in file order; negative adds none; reference/alias/imported/external/type-parameter self types do not attach | `symbols_relations::rust_trait_impls_aggregate_on_the_resolved_self_type` |
| Member keeps its trait relation; method-only query | `symbols_relations::rust_impl_members_keep_their_trait_on_a_method_only_query` |
| Containment aggregation; textual container for unresolved; no membership; qualified names unchanged | `symbols_relations::rust_impl_members_join_the_resolved_self_type_only` |
| Declarations-only runs the parse pass alone; no containment computed | `symbols_relations::relations_need_only_the_parse_pass` |
| Rust containment widening reparses once and equals the uncached result | `symbols_cache::widening_capabilities_runs_only_the_missing_passes` (new Rust Containment row, `parsed: true`) |
| Narrow query = projected cache result for every fixture | existing `symbols_cache::projected_results_equal_uncached_queries` (+ `multi/` twin), green |
| Revision bump invalidates entries | `cache_upgrade::entry_from_extractor_revision_twenty_two_is_not_served_after_the_relations` (Python bases, `OldKey::Computed`) |
| Linear shared context | existing `symbols_shared_context` tests, green |

Input Robustness Matrix: not applicable. No file format or configuration
reader changed; relations reuse the existing `SymbolRef` lists, whose cache
column (`relations.implements`) is already in `cache_robustness`.

### Gates

- `just test` (tree-hugger area): **1061 / 1061 passed**, 0 skipped.
- `just lint`: green (two clippy `collapsible_*` findings in the new files,
  fixed).
- `just check-tier-coverage tree-hugger`: nothing stranded; the five new
  `symbols_relations` tests are L1 in the declared `l1` binary.
- `just test-l2`: not run. No terminal rendering changed (outline output is
  Phase 8). The plain and table goldens are unchanged.
- Cross-OS: not run. The change is pure tree-walking over source text, with
  no paths, processes or OS APIs; CI covers Linux/macOS on the pull request.
- Pre-existing failures: none.

### Docs and skill

- New `docs/type-relations.md`: per-language table, resolution rules with
  examples and a Mermaid flow, the Rust impl table, cache widening, and
  limitations. C#/Swift/Scala are marked **planned**.
- Corrected stale text: `symbol-kinds-and-visibility.md` (records table,
  adapter losses, impl owner rule: `impl V` after `use a::V` no longer links,
  the trait is no longer its last segment); `symbol-schema-design.md` (what is
  still empty, Relation Facet, shared-context table row);
  `cache-design.md` (containment widening rule for Rust, rev.23);
  `lib/README.md`; `README.md` (link to the new page).
- Skill: `SKILL.md` (new relations bullet, doc link, revision list) and
  `api-reference.md` (relations, container under Containment).
- Comments: `rust_impl.rs` module doc rewritten; `ContainerLookup::SameFileType`
  doc; `coverage.rs` module doc and `record_passes_suffice` doc;
  `link_containers` doc; `RecordFacts` field doc.

### Notes

- The concurrent session was editing `analysis/mod.rs`,
  `analysis/tests.rs`, `analysis/counters.rs`, `tree_file.rs`,
  `symbols_shared_context.rs`, `symbol-schema-design.md` and the skill during
  this phase. Edits here were limited to the hunks this phase needed:
  `link_containers`/`resolve_containers` and the `RecordFacts` literal in
  `analysis/mod.rs`; the trait-link block in `tree_file.rs`.
- Containment ties: `find_container_by_identity` still picks the nearest
  preceding owner when several records share a qualified name (Lua repeated
  `local M`). Rust impl members no longer use it, because the resolver
  requires exactly one. Non-impl records are unchanged.
- For Phase 7: the self-type evidence is not persisted (see 6.4); the
  equivalence tests already hold with the reparse. Phase 7.1 should decide
  whether that is final or whether a record field should carry it (a schema
  addition).
- For Phase 8: an unresolved impl member now has a textual
  `relations.container` with no ID. `render::Outline` already heads such a
  member by `container.name` (`[in other::Local]`), which is the self type as
  written rather than the old in-file owner path; no golden covers that case
  yet.

## Phase 7

Capabilities, cache and query/index equivalence. No record changed, so
neither `EXTRACTOR_REVISION` (still 23) nor the schema (4.1) was bumped. The
library gained one public counter field (`WorkCounters::relation_links`) and
doc corrections. Everything else is new tests. Tasks 7.1–7.6 are done; 6.5–6.7
still wait on ruling 1.

### 7.1 Planner and projection: decisions

- **Declarations** already carries generic parameters, constraints,
  modifiers, attributes and `extends`/`implements` (Phases 3, 4 and 6), for
  selected records. Targets resolve against every record of the file,
  selected or not. These facts do not depend on any capability or on kind
  selection, so `coverage::project` keeps them unchanged; no projection rule
  was needed. Its module doc and `Capability::Declarations` /
  `Capability::Containment` docs now say so.
- **Containment** carries the Rust impl links (`RecordFacts::containment`
  in `records_from`) and `link_containers` for everything else; projection
  already drops `parent`/`container`/`members` without it.
- **Metrics** carries `parameters`, `has_docs` and `lines` together.
  Widening parses once, and narrowing drops the whole facet. Both were
  already in place.
- **Self-type evidence (final ruling):** no record retains a Rust impl's
  resolved self type. Widening a Rust entry by Containment parses once
  (`WidenedCapabilities { parsed: true }`), the spec's documented fallback,
  and stays that way. Persisting the evidence was rejected, for two
  reasons. First, a new record field means a schema bump and another cache
  column. Second, storing pre-linked containers on a Declarations-only entry
  would make a stored entry differ from the uncached result of its own
  coverage, which every cache invariant relies on. The cost is one parse,
  only on that one widening path. Already documented in `type-relations.md`
  and `cache-design.md`.
- No new capability. One definitions query, one parse and one relation link
  per analysis (asserted below).

### Library change

- `WorkCounters::relation_links` (`analysis/counters.rs`), incremented once
  per `tree_file/relations.rs::link_relations` call. The linker builds the
  per-file `Declarations` index and, in Rust, walks the impls once, so this
  counter is how a test proves "one index build, one impl traversal per
  file". The CLI only uses `since`, so the new field needed no change there.

### Shared test corpus

`symbols_relations::declaration_fact_sources` returns every source of
`GENERICS_MATRIX`, `MODIFIERS_MATRIX`, `BODY_LINES_MATRIX` (LF and CRLF) and
`RELATIONS_MATRIX`, plus `RUST_IMPLS`. `write_declaration_fact_sources`
writes them to a temp dir. These are the forms built to exercise the new
facts (bounds, `where` clauses, attribute repetitions, body spans, external
and ambiguous targets, multiple, empty and negative impls). The primary
fixtures used by the older equivalence tests hold few of them.

### Requirement → test

| Requirement | Test |
|---|---|
| Narrow query = matching slice of the complete index for generics, constraints, modifiers, attributes, `extends`/`implements` (IDs included), container/parent/members, `metrics`, under Declarations / Containment / Metrics / Flags / Containment+Metrics / all, for every kind, functions, types and methods only; absent where the capability is not asked | `symbols_query::declaration_facts_of_narrow_queries_equal_the_complete_index` (asserts each fact kind occurs in the corpus, textual impl containers included) |
| Declarations-only skips optional passes; one parse, one definitions query, one relation link, no diagnostics (complete index: one diagnostics run, one link) | `symbols_query::declaration_facts_need_one_parse_one_capture_and_one_relation_link` |
| No per-declaration rescan in base/impl resolution; one relation link per file | `symbols_relations::relation_resolution_grows_linearly_with_declarations` (Rust modules with supertraits and local/external impls, TS, Java nested, C++ namespaces with `virtual ext::D`, Python; resolved targets checked at both sizes) |
| Real warm `CacheLookup::Hit` with zero work, cold `Miss(Absent)`, projection to narrower queries equal to uncached (containers and `lines` dropped, relations kept) | `symbols_cache::declaration_fact_sources_are_warm_hits_and_project_to_uncached_queries` |
| Widening declarations → containment (record-only except Rust, which parses once) → metrics (parses once); widened = uncached; entry records the union; each request then a `Hit` with no work, narrowing included | `symbols_cache::declaration_fact_entries_widen_by_containment_and_metrics` |
| Invalidation by schema version | `symbols_cache::a_declaration_fact_entry_of_an_older_record_schema_is_a_miss` (4.0 records on the Rust impl entry), plus the existing `incompatible_record_schema_versions_are_misses_never_served_or_widened` |
| Invalidation by `EXTRACTOR_REVISION` | existing `symbols_cache::each_key_input_and_stored_identity_decides_a_miss` (revision is a fingerprint input) and `cache_upgrade::entry_from_extractor_revision_twenty_two_is_not_served_after_the_relations` (Phase 6, rev.22 → 23); no bump this phase |
| Phase 2 matrix (`metrics.lines` reader shapes) | existing `symbols_adapter::new_record_fields_read_per_shape` and `cli/tests/symbols/cache_robustness.rs`, green |
| Older-model adapters lossy for structured generics, constraints, relations, attributes, `metrics.lines` and new modifiers | `symbols_adapter::record_only_declaration_facts_are_lost_through_the_older_model` (each loss must actually occur, so a fixed adapter flips it) |

Mutation checks, run once by hand and reverted:

- Building the `Declarations` index from selected records only made
  `declaration_facts_of_narrow_queries_equal_the_complete_index` fail
  (`generics Rust: functions with [Containment]`).
- Charging `records.len()` steps per `resolve` call made the linear test
  fail for all five shapes (about 4× the steps at 2× declarations).
- Letting Rust Containment widen from records made
  `declaration_fact_entries_widen_by_containment_and_metrics` fail.

Input Robustness Matrix: not applicable. No reader of a file format or
configuration changed; the record and cache readers are untouched.

### Gates

- `just test` (tree-hugger): **1068 / 1068 passed**, 0 skipped.
- `just lint` (clippy `--all-targets -D warnings`, both packages): green.
- `just check-tier-coverage tree-hugger`: nothing stranded. The seven new
  tests are L1 in the declared `l1` binary, and no name carries a tier marker.
- `just test-l2`: not run. No rendering changed.
- Cross-OS: not run. The new code is a counter and tests over temp files
  with in-memory sources (CRLF variants included). No path comparison,
  process or OS API is involved, and CI proves Linux and macOS on the pull
  request.
- Pre-existing failures: none.

### Docs and skill

- `symbol-schema-design.md`: the adapter list now names `generic_params`,
  `constraints`, `extends`/`implements` and `metrics.lines` and states the
  round trip; the work-counter paragraph adds `relation_links`. Drift fixed:
  the migration step said "currently 4.0"; it is 4.1.
- `symbol-kinds-and-visibility.md`: "Adapter gaps" now covers the record →
  `SymbolInfo` direction.
- `type-relations.md` and `cache-design.md` already described the Rust
  widening fallback and the relation adapter loss; unchanged.
- Skill: `SKILL.md` relations bullet (counter and shared corpus),
  `api-reference.md` work counters.
- Comments: `Capability::Declarations`/`Containment` docs and the
  `coverage.rs` module doc (which facts projection keeps).

### Notes

- To share `SMALL`, `LARGE` and `grows_linearly`, I made them `pub` in
  `symbols_shared_context.rs` (the concurrent session's file). That is a
  one-word change on three lines, so both linear tests use one bound.
- While this phase was running, another process made commit `4be249d1a`
  ("feat(tree-hugger): extract type relations for symbol records", Phase 6).
  It also swept in this phase's edits to `analysis/counters.rs`,
  `analysis/coverage.rs`, `file/query.rs`, `file/tree_file/relations.rs` and
  `tests/l1/symbols_query.rs`, which were in the working tree at the time.
  This phase staged and committed nothing. The remaining Phase 7 changes are
  uncommitted (`symbols_adapter.rs`, `symbols_cache.rs`,
  `symbols_relations.rs`, `symbols_shared_context.rs`, docs, skill, plan,
  log, spec). The frontmatter lists every file this phase touched,
  committed or not.

## Phase 8

### 8.1 Container-row inheritance (`cli/src/render.rs`)

- `Renderer::outline_node` appends `Renderer::declared_bases(record)` after
  the declaration and before the location: ` extends A, B; implements C`,
  each entry's `SymbolRef::name` (written text) through `render::literal`,
  in record order (= source order), empty lists omitted, lists joined by
  `; `. Only `SymbolKind::is_type()` rows get it, so a Rust method's retained
  trait relation and the `[in …]` context headings never show bases, and
  `functions --tree` (no type rows) is unchanged. No row is ever added, so
  selectors are not widened. It goes through the same `hanging_line` Prose,
  so wrapping, plain output and OSC 8 links are unchanged; `classes`, the
  flat listing, the table, `--count` and `--stats` are untouched.
- **`--signatures full` deduplication is a per-source rule, not text
  matching** (`header_writes(language, list)`). Probe over TS, Python, C++,
  Go, Rust, Java, PHP and JS: every full header (`signature_text`) already
  writes its declaration's base list, except two lists the extractor reads
  elsewhere: a Rust type's `implements` (from `impl` blocks) and Go
  interface `extends` (embeds in the body). A substring test was rejected:
  `pub struct W<T: std::fmt::Display>` with `impl Display for W` would hide
  `implements std::fmt::Display` because the bound repeats the text. When
  C#/Swift/Scala (6.5–6.7) land, their lists come from the header, which is
  the rule's default, so no change is needed unless a ruling reads them
  elsewhere (Swift `extension X: P` conformances would be one: such entries
  come from an extension, not `X`'s header, and `header_writes` must then
  return `false` for Swift `implements` or split the rule by origin).
- Short mode output: `class Square extends Base<number>; implements Shape,
  Comparable<Square> [3:1]`; full: `class Square extends Base<number>
  implements Shape, Comparable<Square> [3:1]` (header only); Rust full:
  `pub struct W<T: std::fmt::Display> implements std::fmt::Display, Base`.

### 8.2 Tests (`cli/tests/symbols/outline.rs`, new module of the `symbols` L1 binary)

Requirement → test:

| Requirement | Test |
|---|---|
| bases on `types --tree` and `symbols --tree` rows, source order, including a type with no displayed children; impl on `&Local` adds nothing | `type_rows_show_their_declared_bases_in_source_order` |
| a Rust method's trait relation is not a heading; `[in &Local]` heading for an unresolved impl (the case Phase 7 flagged as uncovered) | same test (exact rows, and exactly four rows carry bases) |
| `functions --tree` gains no type row; a `--name` selector adds no container or target | `functions_tree_adds_no_type_row_and_selectors_add_no_container_or_target` |
| `full` does not repeat header lists but keeps Rust impl / Go body lists (including the bound-overlap case); `none` keeps bases | `signature_modes_keep_bases_without_repeating_the_header` |
| sorting, `--plain` = `--format plain`, styled wrapping at `COLUMNS=40`, `<…>` stays text | `bases_follow_sorting_plain_output_and_wrapping` |
| counts and stats gain no category | `counts_and_stats_gain_no_relation_category` |
| `--json`/`--jsonl`/`--json-full` carry the same relations as the row; `metrics.lines` in the complete index (3, 1, null for a type); listings carry no metrics facet | `json_formats_carry_the_outline_relations_and_body_lines` |

With `declared_bases` disabled, five of the six fail; the counts test is a
guard and passes either way by design.

### 8.3 Goldens

Only `symbols.tree` changed, one row: `public class Panel { size: Size }
implements Sized [9:8]` (multi `typescript.ts`). Reviewed: intended. JSON,
JSONL, table, stats, plain and full-signature goldens are unchanged (no
record changed in this phase; `EXTRACTOR_REVISION` stays 23, schema 4.1).
Full-signature golden is a flat listing, so unaffected.

### 8.4 L2

`level2_declarations::check_outline`: `Shape` row now ends with
`implements Sized`; added style checks (the clause is neither bold nor dim,
the location after it stays dim) and a `COLUMNS=36` case where the bases
wrap under the row's hanging indent. `just test-l2` (tmux, WezTerm; Kitty
not installed here, its entry points return early): 6 passed. The harness
spawns its panes in the background through the Test Toolkit; nothing here
requested focus.

### Verification

- `just test` (tree-hugger): 1074 passed, 0 skipped.
- `just test-l2`: 6 passed.
- `just lint`: clean (one clippy `match_like_matches_macro` fixed).
- `just check-tier-coverage tree-hugger`: nothing stranded; `outline` is an
  L1 module name with no tier marker.
- OS: pure string formatting, no paths or OS calls added; no cross-check run.

### Docs and skill

- `docs/symbol-reports.md`: new "Declared bases on type rows" subsection
  under Outline (what shows, selectors, signature modes, wrapping, counts,
  JSON). `docs/type-relations.md` points at it. `cli/README.md` `--tree`
  bullet. Area/lib READMEs left to 9.2.
- Skill: `SKILL.md` output-modes bullet (`declared_bases`, `header_writes`,
  `outline.rs`).

## Phase 9

Docs, READMEs, skill, comment drift and close. One library change, which is a
doc comment only (`records_from`); no behavior, record, schema (4.1) or
`EXTRACTOR_REVISION` (23) change, so no test was added or changed.

**Ruling 1 is still unanswered** (non-interactive session; no answer in the
spec). Tasks 6.5–6.7 (C#, Swift, Scala base lists) stay open. The docs
describe these languages as **planned** with examples of the forms involved
and say plainly that their records have empty `extends`/`implements` today.
The first item of the definition-of-success checklist therefore holds with
this one open exception; `human_review` stays `true` on the spec.

### 9.1 Current docs

- `docs/symbol-kinds-and-visibility.md`
    - "Grammar limits" listed only the Scala `private enum` form. It now has
      one row per demonstrated limit of this work, each with snippet, pinned
      grammar, observed node shape and what is reported: Python PEP 696
      default (tree-sitter-python 0.25.0), TS `in`/`out` variance
      (tree-sitter-typescript 0.23.2), C++ concept-constrained parameter
      read as `Const` (tree-sitter-cpp 0.23.4), TS/JS non-path decorator
      expression (tree-sitter-typescript 0.23.2, tree-sitter-javascript
      0.25.0). Each row names the test family that pins it.
    - "Current limitations": new bullets for the planned C#/Swift/Scala
      relations and for declarations with no record (Python `type A[T]`, C#
      `delegate`) carrying no generics. The intro now points at the limits
      in the plural.
- `docs/type-relations.md`: new "C#, Swift and Scala" section (C#
  `class Child : Base, IDisposable`, Swift `struct Point: Codable` and
  `extension Point: Equatable`, Scala `extends Base with Logging`), why
  the lists are not read, and what records hold today. The table row and the
  limitation bullet link to it.
- `docs/symbol-schema-design.md`: Relation Facet "planned" sentence links
  to that section.
- Checked current, unchanged: `cache-design.md` (`0.1.0+rev.23`, 4.1,
  `metrics.lines` rows), `symbol-reports.md` (outline bases, schema 4.1,
  `too_many_generic_parameters`), `symbol-schema-design.md` (version table,
  generics table and limits, attributes/modifiers, adapter list). Mermaid
  flows for resolution and body lines were already in place.
- No doc names or links this feature (grep for `2026-10-06` and
  `symbol-type-relations` under `tree-hugger/docs`, READMEs and the skill:
  no hits).

### 9.2 READMEs

- `README.md` (area): new release-notes section "generics, base types,
  modifiers, attributes and body lines" (what each new fact is, the outline
  row example, schema 4.1, one-time re-analysis, adapter loss). The older
  release note that says "schema 4.0" describes its own release and was left.
- `lib/README.md`: `type_info.generic_params`/`constraints` bullet with the
  `Bag<T: Clone = String>` example; `lines` meaning; C#/Swift/Scala
  planned; the record → `SymbolInfo` adapter losses.
- `cli/README.md`: which JSON formats carry the declared facts, and that
  `metrics.lines` appears only in `--json-full`
  (`ReportFormat::capabilities`: `--json`/`--jsonl` ask Flags only).

### 9.3 Skill

- `SKILL.md` "Known Limitations": C#/Swift/Scala relations planned (ruling
  pending, doc anchor); the declaration-fact grammar limits with their test
  family and doc table.
- `api-reference.md`: adapter paragraph now lists the declaration facts the
  older model drops (with the pinning test); relations paragraph says
  C#/Swift/Scala are planned.
- The rest of the skill (relations, generics, modifiers, body lines, outline
  bullets, revision 23, schema 4.1) was already current from Phases 2–8.

### 9.4 Comment drift

Reviewed the module and item docs of every reader this feature changed:
`extract/{attributes,flags,metrics,modifiers,generics,relations,rust_impl,classify}.rs`,
`tree_file.rs::records_from`, `tree_file/relations.rs`,
`shared/schema_v2/facets.rs` (`Attribute`, `Modifier`, `MetricsFacet`,
`GenericParam`), `file/query.rs` (`Capability`), and the CLI renderer's
`declared_bases`/`header_writes`.

| Drift | Resolution (code is correct) |
|---|---|
| `TreeFile::records_from` doc: "generic parameters, implemented trait and the `facts` asked for" — the trait-only link was replaced by declared bases in Phase 6, and attributes and written modifiers were missing | Rewritten: "generic parameters, attributes, written modifiers, declared bases and the `facts` asked for are filled only for kinds `selection` wants. Base targets resolve against every record, wanted or not." |

The two phrases the plan named ("attributes are read only for flags",
"metrics have two members") no longer appear anywhere in `lib/src` or
`cli/src`; earlier phases had already corrected them (Phase 2 attributes,
Phase 2/5 `MetricsFacet` and `Capability::Metrics`).

### 9.5 Rulings, probe, departures and unsupported forms (consolidated)

**Rulings** (Phase 1 table, final state):

| # | Ruling | Final |
|---|---|---|
| 1 | Mixed C#/Swift/Scala base lists | **Open.** Not answered; 6.5–6.7 not implemented; documented as planned |
| 2 | Schema version | Accepted default: `V4_1` |
| 3 | Prerequisite state | Accepted default: proceed; read syntax where kinds are broad |
| 4 | Full written Rust trait text | Accepted default (`fmt::Display`, not `Display`) |
| 5 | Modifier order | Accepted default: one normalizer (`normalize_modifiers`) |
| 6 | Unresolved impl self type | Accepted default: textual container, no ID, no parent |
| 7 | `u16` overflow | Accepted default: `TooManyGenericParameters` |
| 8 | Wider performance measurement | Not planned |
| 9 | Spike scope | Done once in Phase 1 |

**Grammar probe:** the Phase 1 tables (section 1.2) remain correct; Phase 6
re-probed the base-list nodes and they held.

**Departures from the spec/plan** (all already in the docs):

- The older `TypeMetadata::type_parameters` readers stay separate readers,
  not a projection of the new structured reader (Phase 3; plan strand 2 said
  "kept as a projection").
- Rust impl self-type resolution is not persisted; widening a cached Rust
  entry by Containment parses once (the spec's fallback, Phases 6/7).
- `--signatures full` drops a base list the header already writes by an
  exact per-source rule (`header_writes`), not by text matching (Phase 8).
- C#, Swift, Scala relations not implemented (ruling 1 open).

**Unsupported forms:**

| Form | Kind of limit | Where documented |
|---|---|---|
| Python type-parameter default `class C[T = str]` | grammar (python 0.25.0) | `symbol-kinds-and-visibility.md#grammar-limits`, `symbol-schema-design.md` |
| TS variance `interface K<in T>` | grammar (typescript 0.23.2) | same |
| C++ `template <std::integral T>` read as `Const` | grammar (cpp 0.23.4) | same |
| TS/JS decorator that is not a path or call | grammar (typescript 0.23.2, javascript 0.25.0) | same, plus `symbol-schema-design.md` attributes |
| Python `type A[T] = …`, C# `delegate`: no record, so no generics | extractor (no record by design; spec forbids adding records to fill a cell) | `symbol-kinds-and-visibility.md` current limitations |
| C#/Swift/Scala `extends`/`implements` | open ruling | `type-relations.md#c-swift-and-scala` |
| Go: a named type embedded in a constraint vs an embedded interface | syntax cannot tell; predeclared non-interfaces excluded | `type-relations.md` limitations |
| PHP `#[…]`, Perl sub attributes, C `__attribute__` not read as attributes | outside the spec's required set | `symbol-schema-design.md` attributes |

### 9.6 Final verification

- `just test` (tree-hugger): **1074 / 1074 passed**, 0 skipped.
- `just lint`: green (both packages).
- `just test-l2`: **6 / 6 passed** (tmux and WezTerm; the Kitty entry points
  return early because Kitty is not installed here). Panes are spawned in
  the background by the harness; no window took focus.
- `just check-tier-coverage`: not re-run; this phase added and renamed no
  test (Phase 8's run covers the current test set).
- Cross-OS: not run. This phase changed documentation and one doc comment.
- Pre-existing failures: none.

Definition-of-success checklist (plan): every item holds except the first
item's C#/Swift/Scala relation cells, which wait on ruling 1 (logged above,
not accepted as an exception by the author yet). The feature directory stays
active; implementation complete, ready for review once ruling 1 is settled.
