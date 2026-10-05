---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-06-symbol-type-relations/spec.md"
plan: "tree-hugger/features/2026-10-06-symbol-type-relations/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages: []
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
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
