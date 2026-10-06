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
