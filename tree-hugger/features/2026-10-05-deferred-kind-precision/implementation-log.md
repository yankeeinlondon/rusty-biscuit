---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-05-deferred-kind-precision/spec.md
plan: tree-hugger/features/2026-10-05-deferred-kind-precision/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/header/mod.rs
  - tree-hugger/lib/src/file/extract/type_metadata.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/symbol.rs
  - tree-hugger/lib/tests/l1/symbols_field_lists.rs
  - tree-hugger/lib/tests/l1/symbols_header.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/lib/tests/l1/symbols_qualified_names.rs
  - tree-hugger/lib/tests/l1/symbols_query_filters.rs
  - tree-hugger/lib/tests/l1/symbols_scope.rs
  - tree-hugger/cli/tests/symbols/order.rs
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/queries/typescript/locals.scm
  - tree-hugger/lib/queries/vendor/c/locals.scm
  - tree-hugger/lib/queries/vendor/c_sharp/locals.scm
  - tree-hugger/lib/queries/vendor/go/locals.scm
  - tree-hugger/lib/queries/vendor/java/locals.scm
  - tree-hugger/lib/queries/vendor/php_only/locals.scm
  - tree-hugger/lib/queries/vendor/scala/locals.scm
  - tree-hugger/lib/queries/vendor/swift/locals.scm
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/header/keyword.rs
  - tree-hugger/lib/src/file/extract/header/mod.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/type_metadata.rs
  - tree-hugger/lib/src/file/extract/visibility/csharp.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/file/extract/visibility/java.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/queries/inventory.rs
  - tree-hugger/lib/src/queries/provenance.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_declarator_wrappers.rs
  - tree-hugger/lib/tests/l1/symbols_deferred_metadata.rs
  - tree-hugger/lib/tests/l1/symbols_field_lists.rs
  - tree-hugger/lib/tests/l1/symbols_function_locals.rs
  - tree-hugger/lib/tests/l1/symbols_kind_coverage.rs
  - tree-hugger/lib/tests/l1/symbols_kinds.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols-signatures-full.plain
  - tree-hugger/cli/tests/fixtures/golden/symbols-stats.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/tests/fixtures/golden/symbols.plain
  - tree-hugger/cli/tests/fixtures/golden/symbols.stats
  - tree-hugger/cli/tests/fixtures/golden/symbols.table
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
---

# Implementation Log for 2026-10-05-deferred-kind-precision (6 phases)

## Phase 1

Phase 1 changes no source, docs or skill files. It records the baseline,
the rulings, the capture path and the grammar spike (S1/S2) that Phases 2
and 3 build on. `packages` lists the packages whose baseline was run; neither
was edited.

### Baseline (2026-10-05, macOS host)

- Working tree at start: `feat/symbols` at `9266bc82e`. The only dirty paths
  were `tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md`
  (modified) and this log (untracked, empty).
- **Departure from the plan's expectation:** the plan says the tree holds
  uncommitted sibling-feature edits under `lib/src/file/extract/`,
  `lib/tests/l1/`, CLI goldens and `symbols_owner_identity.rs`. None are
  uncommitted any more; they appear to have been committed before this
  phase started. The baseline is therefore the committed tree plus the
  sibling's `log.md` edit, which no build reads.
- `just test` in `tree-hugger/`: **949 tests run, 949 passed (8 slow), 0
  skipped**, across 5 binaries (`tree-hugger` and `tree-hugger-cli`).
- `just lint` in `tree-hugger/`: clippy clean for `tree-hugger` and
  `tree-hugger-cli` (exit 0).
- Baseline is green, so Phase 1 continued.

**Concurrent edits during this phase.** While Phase 1 ran, another session
working the sibling feature `2026-10-03-symbol-filters-and-reports` changed
this worktree. This phase made none of these changes.

- **Uncommitted edits to:**
  - `lib/src/analysis/{mod,tests}.rs`
  - `lib/src/file/mod.rs`
  - `lib/src/file/tree_file/definitions.rs`
  - `lib/tests/l1/symbols_owner_identity.rs`
  - `cli/src/{declarations,render}.rs`
  - `cli/tests/symbols/{cache_upgrade,classes,reports}.rs`
  - the `symbols.json` / `symbols.jsonl` goldens
  - the three `docs/symbol-*.md` pages
  - `.claude/skills/tree-hugger/SKILL.md`
- **Feature moves:** `2026-10-03-symbol-improvements` moved to
  `features/_completed/`, and the sibling `log.md` was staged.
- **`EXTRACTOR_REVISION` is now `"11"`** (`lib/src/file/mod.rs`), not the
  `"10"` the plan's Phase 4 names. Phase 4 must bump from whatever value is
  current then, and add the matching `StaleRevision` in
  `cli/tests/symbols/cache_upgrade.rs`.

**Re-check on the tree with those edits:**

- `just test`: 954 run, 954 passed (61 slow), 0 skipped.
- `just lint`: clean.

Phase 2 starts from a green tree either way. Do not attribute later diffs
in those files to this feature without checking `git log` and `git diff`
first.

### Rulings R1–R7

The plan's defaults are applied. Each one either restates the spec's own
recommendation or is confirmed by the code, and none widens the required
scope. Under the spec, Open Questions recommendations do not expand scope
until the author accepts them, so these defaults are the spec's required
scope. The author can still amend any of them before Phase 3.

| Ruling | Outcome | Evidence |
|---|---|---|
| R1 Go/Swift owners | Default kept: Go struct/interface and Swift class/enum/struct stay `Type`. Go `Field` containers and Swift `Constructor` owners report container kind `Type`. | Spike: `S` (Go struct) is `Type`; Swift `C`, `S`, `En` are `Type`. Swift `enum En` carries `kind_data=Unknown` (no variant list), unlike enums that are `Enum`. |
| R2 Constructor breadth | Default kept: explicit instance constructors only. Java compact record constructors, C# static/primary constructors and destructors, and C++ out-of-class definitions keep current behavior. | Spike: Java `compact_constructor_declaration` is not captured today; C# `static C() {}` is the same `constructor_declaration` node with a `modifier` `static`, so the new capture must **exclude** it to keep current behavior (absent). C# `destructor_declaration` is absent today. C++ `C::C() {}` is `Function`, qn `C`, no container. |
| R3 Variant payload | Confirmed: a standalone `EnumVariant` carries `kind_data = Unknown`; the owning enum's `EnumData.variants` holds the variant list. | Spike: existing Rust `A`/`B`/`C` and Swift `a` EnumVariant records already have `kind_data=Unknown`; Rust `R` and C `E` hold `EnumData { variants: [...] }`. See finding F5 for the `discriminant` gap. |
| R4 Query file | Confirmed with the code's resolution order (detail below). | `src/queries/mod.rs::resolve_locals_query`, `src/queries/provenance.rs::query_provenance`. |
| R5 Python/PHP owner rule | Default kept. | Spike: function-local `__init__` is already `Function` (qn `f.__init__`), and `__init__` in a class nested in a function is `Method` with container `Inner`. The hook in `classify_symbol_kind` runs before `is_function_local_member` demotion in `refine_symbol_kind`, so the order the plan asks for already exists. |
| R6 Enum-member access/static | Default kept. On records `static` is the presence of `Modifier::Static` (`modifiers_from_symbol`), so "absent" and "false" are the same state: enum members get no `Static` modifier. | `src/shared/schema_v2/mod.rs::modifiers_from_symbol`. |
| R7 Wider measurement | Not scheduled. Nothing in S1 needs a new full-file scan or extra parse; every form is a node in the existing single parse. | S1 below. |

**R4 detail.** Locals queries resolve like this:

- `resolve_locals_query` loads the vendored `queries/vendor/{lang}/locals.scm`,
  following its `; inherits:` lines.
- Only `javascript` and `typescript` have an overlay `queries/{lang}/locals.scm`.
  Both are `; extends` overlays appended after the vendored query.
- Provenance for `QueryKind::Locals` prefers the vendored record
  (`vendor_query_provenance`) and carries per-language notes in
  `vendored_locals_changes`. That note list is the "capture inventory" to
  update.

So the edit locations are:

- **TypeScript:** `queries/typescript/locals.scm` (overlay, no provenance
  change). JavaScript loads its own overlay and does not inherit the
  TypeScript one, so TypeScript-only captures cannot leak into `.js`.
- **PHP:** `queries/vendor/php/locals.scm` is only `; inherits: php_only`, so
  PHP captures go in `queries/vendor/php_only/locals.scm`. Add a note to the
  `ProgrammingLanguage::Php` arm of `vendored_locals_changes`, beside the
  existing `const_declaration` note.
- **Go, Python, Java, C#, C, C++, Swift, Scala:** the vendored
  `queries/vendor/{lang}/locals.scm`, with a note added to that language's
  `vendored_locals_changes` arm. Go and Scala currently have **no** notes,
  so their first note flips `translation_status` from `CapturesRenamed` to
  `MultipleChanges`. Any test pinning that status must change with it.
- **C/C++:** `queries/vendor/cpp/locals.scm` starts with `; inherits: c`, so
  a capture added to the C query reaches C++. In the spike both already
  capture enumerators as `Variable`, so this is a classifier change, not a
  query change.

### Capture path

A definitions capture becomes a kind in three steps, all in
`lib/src/file/extract/classify.rs`:

1. `symbol_kind_from_capture` maps `@local.definition.<suffix>`.
2. `resolve_declaration_node` (`declaration.rs`) finds the declaration.
3. `refine_symbol_kind` → `classify_symbol_kind` overrides by declaration
   node, then demotes function-local members (`is_function_local_member`).

Every capture suffix the new kinds need already maps:

| Kind | Capture name | Mapped |
|---|---|---|
| Constructor | `@local.definition.constructor` | yes |
| EnumVariant | `@local.definition.enum_variant` | yes |
| TypeAlias | `@local.definition.type_alias` | yes |
| Field | `@local.definition.field` | yes |
| Namespace | `@local.definition.namespace` | yes |

No new capture name is needed. A classifier-only cell (Python, PHP and C++
constructors; C/C++ enumerators; Scala fields) can also be done with an arm
in `classify_symbol_kind` keyed on the declaration node, with no query edit.

### S1: Grammar node inventory

**Method.** A throwaway program, outside the repository (`/tmp/th-spike`,
not committed), depends on the `tree-hugger` library by path. It parses each
snippet with the grammar `ProgrammingLanguage::grammar_for_path` returns, so
the pinned crates from `lib/Cargo.toml` are used. For each snippet it prints:

- the tree, with field names and anonymous keyword tokens;
- `has_error`;
- today's `TreeFile::symbols()` and `symbol_index_v2()` kind, container,
  qualified name, access and `kind_data`.

Parse results do not depend on the platform, so the spike ran on one host.

**Pinned grammars:**

| Language | Crate |
|---|---|
| TypeScript | tree-sitter-typescript 0.23.2 |
| Go | tree-sitter-go 0.25.0 |
| Python | tree-sitter-python 0.25.0 |
| Java | tree-sitter-java 0.23.5 |
| C# | tree-sitter-c-sharp 0.23.1 |
| C | tree-sitter-c 0.24.1 |
| C++ | tree-sitter-cpp 0.23.4 |
| Swift | tree-sitter-swift 0.7.1 |
| Scala | tree-sitter-scala 0.24.0 |
| PHP | tree-sitter-php 0.24.2 |

The runtime is tree-sitter 0.26.3.

**Columns.** "Captured today" is what the extractor reports now. Verdicts:

- **capture gap**: no record exists today.
- **classifier gap**: a record exists with the wrong kind.
- **grammar limit**: the pinned grammar cannot represent the form.
- **negative ✓**: the counterexample already keeps its expected kind.

Unless a row says otherwise, every snippet parsed with `has_error=false`.

#### TypeScript

| Form | Nodes (declaration ← name) | Captured today | Verdict |
|---|---|---|---|
| `enum E { Red }` bare member | `enum_body > property_identifier` (**no wrapper node**; the name's parent is `enum_body`) | no | capture gap |
| `Blue = 2` | `enum_body > enum_assignment(name: property_identifier, value: number)` | no | capture gap |
| `"quoted" = 1` (string-named member) | `enum_assignment(name: string > string_fragment)` | no | capture gap (not in the required table; decide in Phase 3 whether the name is `quoted`) |
| `export enum Ex { Green }` | `export_statement > enum_declaration`; members as above | enum yes (`Enum`), members no | capture gap (access inheritance case) |
| `const enum CE {…}` | `enum_declaration` (the `const` token is anonymous) | enum yes, members no | capture gap |
| `namespace Space {}` | `expression_statement > internal_module(name: identifier, body: statement_block)` | no record; members already qualified `Space.inside`, container `None` | capture gap |
| nested `namespace Inner {}` in `Space` | `statement_block > expression_statement > internal_module` | no; `deep` qn `Space.Inner.deep` | capture gap |
| `export namespace A.B {}` | `export_statement > internal_module(name: nested_identifier(object: identifier, property: property_identifier))` | no; `v` qn `A.B.v` | capture gap (dotted name) |
| `declare namespace D {}` | `ambient_declaration > internal_module` | no (its `function_signature f` is not captured either) | capture gap |
| `module Mod {}` | `module(name: identifier)` | no | capture gap, **with hazard F1** |
| `export module M2 {}` | `export_statement > module` | no | capture gap (F1) |
| negative: `class K { Red = 1 }` | `public_field_definition` | `Property` `K.Red` | negative ✓ |
| negative: `declare module "x" {}` | `ambient_declaration > module(name: string)` | no record; acts as container `"x"` (`Module`) for `y` | negative ✓ (keep: no named-namespace record) |
| negative: `export * as Star from "./x"` | `export_statement > namespace_export` | no | negative ✓ |
| negative: `import * as Imp from "./x"` | `import_statement > import_clause > namespace_import` | no definition | negative ✓ |
| JavaScript fallthrough: `class J { constructor(a) {} }` | `method_definition(name: property_identifier "constructor")` | `Constructor` | regression ✓ (existing arm) |

#### Go

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `type Alias = int` | `type_declaration > type_alias(name: type_identifier, '=', type: type_identifier)` | **no record at all** | capture gap |
| grouped `type ( Grouped = string; GroupedDefined int )` | one `type_declaration` with children `type_alias` and `type_spec`; **no `type_spec` wrapper around the alias** | alias no; `GroupedDefined` `Type` | capture gap |
| generic `type GenAlias[T any] = []T` | `type_alias(type_parameters: type_parameter_list, type: slice_type)` | no | capture gap (parses cleanly) |
| `type S struct { X, Y int }` | `type_spec > struct_type > field_declaration_list > field_declaration(name: field_identifier, name: field_identifier, type)` | no | capture gap (two `name` fields on one declaration) |
| `z string` | `field_declaration(name, type)` | no | capture gap |
| `anon struct{ Inner int }` | `field_declaration(name: field_identifier "anon", type: struct_type)` | no | capture gap for `anon` (a named field of `S`); `Inner` sits in an anonymous struct type and keeps current behavior (absent) |
| negative: `type Defined int` | `type_spec` | `Type` | negative ✓ |
| negative: embedded `Embedded` | `field_declaration(type: type_identifier)`, **no `name` field** | no | negative ✓ (keep absent) |
| negative: `type I interface{ M() }` | `interface_type > method_elem(name: field_identifier)` | `I` `Type`; `M` absent | negative ✓ (must never become `Field`) |
| negative: receiver `r`, parameter `p` | `parameter_declaration` | `Parameter` | negative ✓ |

#### Python

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `def __init__(self, value=1)` in `class C` | `class_definition > block > function_definition` | `Method` container `C` | classifier gap |
| decorated `__init__` in `class D` | `block > decorated_definition > function_definition` | `Method` container `D` | classifier gap |
| `__init__` in a class nested in a function | `function_definition > block > class_definition > block > function_definition` | `Method` container `Inner`, qn `f.Inner.__init__` | classifier gap (R5: becomes `Constructor`) |
| negative: top-level `def __init__()` | `module > function_definition` | `Function` | negative ✓ |
| negative: function-local `__init__` | `function_definition > block > function_definition` | `Function` qn `f.__init__` | negative ✓ |
| negative: `__new__`, `@classmethod make` | `function_definition` / `decorated_definition` | `Method` | negative ✓ |

#### Java

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `C(int x) {}`, `public C() {}` | `class_body > constructor_declaration(modifiers?, name: identifier, parameters, body: constructor_body)` | **no record**; parameters already qualified `C.C.x` | capture gap |
| enum constructor `Planet(double m) {}` | `enum_body > enum_body_declarations > constructor_declaration` | no | capture gap |
| `enum E { RED, BLUE }` | `enum_body > enum_constant(name: identifier)` | no | capture gap |
| `MERCURY(1.0)` | `enum_constant(name, arguments: argument_list)` | no | capture gap |
| `BLUE { void inBody() {} }` | `enum_constant(name, body: class_body > method_declaration)` | constant no; `inBody` `Method`, container `None`, qn `inBody` | capture gap for `BLUE`; the unnamed-owner boundary already holds for `inBody` |
| negative: `void C() {}` | `method_declaration(type: void_type)` | `Method` | negative ✓ |
| negative: initializer `{ }`, `static { }` | `block`, `static_initializer` | no | negative ✓ |
| out of scope (R2): `record R(int x) { R { } }` | `compact_constructor_declaration(name: identifier, body: block)` | no | keep absent |

#### C\#

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `public C(int x) {}` | `declaration_list > constructor_declaration(modifier, name, parameters, body: block)` | **no record** | capture gap |
| struct `public S(int x) => X = x;` | `constructor_declaration(body: arrow_expression_clause)` | no | capture gap |
| record `public Rec(int a, int b) : this(a) {}` | `constructor_declaration(constructor_initializer, body: block)` | no | capture gap |
| `enum E { Red, Blue = 2 }` | `enum_member_declaration_list > enum_member_declaration(name, value?)` | no | capture gap |
| negative: `public int C() {…}` | `method_declaration(returns: predefined_type)` | `Method` | negative ✓ |
| R2 keep: `static C() {}` | `constructor_declaration(modifier "static")`, same node kind as an instance constructor | no | keep absent; exclude it from the new capture (F4) |
| R2 keep: `~C() {}` | `destructor_declaration` | no | keep absent |
| R2 keep: primary `class P(int x)` | `class_declaration > parameter_list > parameter` | `x` `Parameter` | keep |

#### C

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `enum E { RED, BLUE = 2 };` | `enum_specifier > enumerator_list > enumerator(name: identifier, value?)` | `Variable` container `E` (`Enum`) | classifier gap |
| anonymous `enum { ANON_A, ANON_B };` | `enum_specifier` (no `name`) > `enumerator_list > enumerator` | `Variable`, container `None` | classifier gap (container stays `None`) |
| `typedef enum { T_A } TEnum;` | `type_definition > enum_specifier > enumerator_list > enumerator` | `T_A` `Variable`, container `None` | classifier gap |
| negative: `int x = RED;` | `declaration > init_declarator(value: identifier)` | only `x` `Variable` | negative ✓ |

#### C++

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `C();` | `field_declaration_list > declaration > function_declarator(declarator: identifier)`. A constructor has **no `type` field**, and its declarator is `identifier`, not `field_identifier`. | `Method` container `C` | classifier gap |
| `C(int x) {}` | `function_definition > function_declarator(identifier)` | `Method` | classifier gap |
| `C(const C&) = default;` | `function_definition(default_method_clause)` | `Method` | classifier gap (parses cleanly) |
| `C(C&&) = delete;` | `function_definition(delete_method_clause)` | `Method` | classifier gap (parses cleanly) |
| `template <typename T> C(T t) {}` | `template_declaration > function_definition` | `Method` | classifier gap (`template_declaration` is already a transparent wrapper) |
| `explicit C(double d);` | `declaration(explicit_function_specifier, declarator: function_declarator)` | `Method` | classifier gap |
| `TC();` in `template <…> class TC` | `template_declaration > class_specifier > … > declaration` | `Method` container `TC` | classifier gap |
| member-template constructor in `template <…> struct TS` | `field_declaration_list > template_declaration > declaration` | `Method` container `TS` | classifier gap |
| `K();` in `namespace N { class K }` | `declaration` | `Method` qn `N::K::K` | classifier gap |
| `struct S { S() = default; }` | `function_definition(default_method_clause)` | `Method` container `S` (`Type`) | classifier gap |
| unscoped `enum Unscoped { U_A, U_B = 2 }` | `enum_specifier > enumerator_list > enumerator` | `Variable` container `Unscoped` | classifier gap |
| scoped `enum class Scoped { S_A }` | `enum_specifier("enum" "class") > enumerator_list > enumerator` | `Variable` container `Scoped` | classifier gap |
| in-class `enum Inner { IA };` | `field_declaration(type: enum_specifier)` | `IA` `Variable` container `Inner` | classifier gap |
| negative: `~C();` | `function_declarator(declarator: destructor_name)` | absent | negative ✓ (keep) |
| negative: `operator bool() const;` | `declaration(declarator: operator_cast)` | absent | negative ✓ (keep) |
| negative: `void method();` | `field_declaration(type: primitive_type, declarator: function_declarator(field_identifier))` | `Method` | negative ✓ |
| R2 keep: out-of-class `C::C() {}` | `function_definition > function_declarator(declarator: qualified_identifier(scope, name))` | `Function`, qn `C`, no container | keep |

#### Swift

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `init() {}` in class | `class_body > init_declaration(name: "init", body)`. **The `name` field is the anonymous keyword token `init`.** | **no record**; parameters already qualified `C.init.x` | capture gap (F3) |
| `init?(x: Int)` | `init_declaration(name: "init", '?', parameter, body)` | no | capture gap (parses cleanly) |
| `init!(y: Int)` | `init_declaration(name: "init", bang, parameter, body)` | no | capture gap (parses cleanly) |
| `init` in struct, in extension, in enum | `class_declaration(declaration_kind: struct / extension / enum) > class_body` or `enum_class_body` > `init_declaration` | no | capture gap |
| `typealias Alias = Int` | `typealias_declaration(name: type_identifier, '=', name: user_type)`. **The target is also under field `name`** (second occurrence). | no | capture gap (F3) |
| `typealias G<T> = Array<T>` | `typealias_declaration(name, type_parameters, '=', name: user_type)` | no | capture gap |
| `typealias Raw = Int` in an enum | `enum_class_body > typealias_declaration` | no | capture gap |
| negative: `deinit {}` | `deinit_declaration` | absent | negative ✓ |
| negative: `static func make()` | `function_declaration` | `Method` | negative ✓ |
| negative: `associatedtype Element` | `protocol_body > associatedtype_declaration(name)` | absent | negative ✓ |
| negative: `associatedtype E2 = Int` (a default, not an alias) | `associatedtype_declaration(name, '=', name: user_type)` | absent | negative ✓ (must stay non-alias) |

#### Scala

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `case Red, Blue` | `enum_body > enum_case_definitions > simple_enum_case(name)` ×2 | no | capture gap |
| `case Value(x: Int)` | `enum_case_definitions > full_enum_case(name, class_parameters)` | case no; `x` `Parameter` container `Color` | capture gap |
| `type Alias = Int` | `type_definition(name: type_identifier, '=', type)` | no | capture gap |
| generic `type Gen[A] = List[A]` | `type_definition(name, type_parameters, type: generic_type)` | no | capture gap |
| object `val a = 1`, `var b: Int = 2` | `template_body > val_definition(pattern: identifier)` / `var_definition(pattern, type)` | `Variable` container `O` (`Module`) | classifier gap |
| trait `val c: Int`, `var d: Int` | `template_body > val_declaration(name)` / `var_declaration(name)` | `Variable` container `T` (`Trait`) | classifier gap |
| class `val e = 1` | `val_definition` | `Variable` container `K` | classifier gap |
| negative: abstract `type Element` | `type_definition(name)`, **no `type` field** | absent | negative ✓ (keep absent) |
| negative: method-local `val local` | `function_definition > block > val_definition` | `Variable` qn `K.m.local` | negative ✓ |
| negative: class parameter `p` | `class_parameters > class_parameter` | `Parameter`; listed in `K`'s field list | negative ✓ |
| out of scope: `val (d1, d2) = (1, 2)` | `val_definition(pattern: tuple_pattern)` | absent | keep |
| **`private enum Hidden { case H }`** (top level, or in `object W`) | `ERROR(access_modifier "private")` followed by a separate `enum_definition`. The same happens for `protected`. | enum `Hidden` `Enum` access **Public**; case absent | **grammar limit** (G1) |

#### PHP

| Form | Nodes | Captured today | Verdict |
|---|---|---|---|
| `public function __construct($x = 1)` | `declaration_list > method_declaration(visibility_modifier, name: name)` | `Method` container `C` | classifier gap |
| `function __Construct()` (mixed case) | `method_declaration(name: name "__Construct")` | `Method` | classifier gap (keep source spelling) |
| `enum Suit { case Hearts; }` | `enum_declaration_list > enum_case(name)` | no | capture gap |
| backed `case H = 'h';` | `enum_case(name, value: string)` | no | capture gap |
| `namespace App;` | `namespace_definition(name: namespace_name)`, **no body**; the members are following siblings | no record; `inside` qn `App\inside`, container **None** | capture gap (F2) |
| `namespace App\Braced { … }` | `namespace_definition(name: namespace_name(name, name), body: compound_statement)` | no record; `inside` qn `App\Braced\inside`, container **`App\Braced` (`Namespace`)** | capture gap |
| negative: top-level `function __construct()` | `function_definition` | `Function` | negative ✓ |
| negative: legacy `function Legacy()` in `class Legacy` | `method_declaration` | `Method` | negative ✓ |
| negative: `use Foo\Bar;` | `namespace_use_declaration` | no definition | negative ✓ |
| negative: global `namespace { … }` | `namespace_definition` with **no `name` field** | no record; `global_fn` container `None` | negative ✓ (keep: no invented name) |

#### Forms the plan flagged as likely limits

| Flagged form | Result |
|---|---|
| Swift `init?` / `init!` | Parse cleanly (`'?'` token; `bang` node). Not a limit. |
| C++ `C() = default`, `= delete`, template-wrapped | Parse cleanly and are already captured, as `Method`. Not a limit. |
| Go grouped `type ( A = B )` | Parses cleanly; `type_alias` sits directly in `type_declaration`. Not a limit. |
| Scala 3 `enum` cases, `case Value(x: Int)` | Parse cleanly (`simple_enum_case`, `full_enum_case`). Not a limit. |
| TypeScript `module Space {}` | Parses cleanly as `module` (F1). Not a limit. |
| PHP braced global `namespace {}` | Parses cleanly; there is no `name` field. Negative case, not a limit. |
| Java enum constants with class bodies | Parse cleanly (`enum_constant(body: class_body)`). Not a limit. |

### S2: Verdict gate

**One grammar limit, G1.** In tree-sitter-scala 0.24.0, an access modifier
on a Scala 3 `enum` (`private enum Hidden { case H }`, also `protected`, at
the top level or in a template body) does not parse.

- **Snippet:** `object W { private enum Hidden { case H } }`
- **Grammar:** tree-sitter-scala 0.24.0, on the tree-sitter 0.26.3 runtime.
- **Observed node:** `ERROR(access_modifier)` followed by a sibling
  `enum_definition`; `has_error = true`.
- **Remaining behavior:**
  - The enum is extracted as `Enum`, with access **Public**: the modifier is
    lost.
  - Its cases will extract once the Scala enum-case capture lands. They
    inherit the enum's recorded access, which is Public.
- **Scope of the limit:** only the "inherits a non-default access" form of
  the Scala EnumVariant cell. Default-access enum cases are a capture gap
  and land normally.
- **Phase 3 Scala task:**
  - Keep the access-modified form as an `unsupported` test case with this
    reason.
  - Assert the inheritance rule on a default-access enum.
  - Do not file this as "language has no construct".

Every other required form is a capture gap or classifier gap and proceeds
to Phase 2/3. Every negative case keeps its expected kind today.

### Findings for Phases 2 and 3

- **F1: `module` is a scope boundary.**
  - TypeScript `module Mod {}` produces a node of kind `module`.
  - `is_scope_boundary` (`declaration.rs`) lists `module` because it is
    Python's root node, so `resolve_declaration_node` stops on it and falls
    back to the name.
  - The TypeScript Namespace work must make that rule language-aware: for
    example, treat `module` as a boundary only when it has no parent. Do not
    capture around it.
- **F2: PHP namespace containers are asymmetric today.**
  - Members of a braced `namespace App\Braced { … }` already get container
    `App\Braced` (`Namespace`).
  - Members of a semicolon `namespace App;` get container `None`.
  - Both keep the qualified name `App\…`.
  - Adding the semicolon Namespace record raises one question: should its
    following siblings gain it as container, through the sibling-scoped
    container path that C# file-scoped namespaces use? Phase 2 must decide
    that.
  - The spec says qualified names and IDs of unrelated records must not
    change. Container is not an ID input: IDs come from
    `stable_symbol_key(language, file, kind, qualified_or_name, sibling_ordinal)` in
    `src/shared/schema_v2/mod.rs`. A container change alone therefore keeps
    IDs, but it does change `container` in reports and goldens.
- **F3: two Swift name traps.**
  - `init_declaration`'s `name` field is the anonymous keyword token `init`,
    so the query needs `name: "init" @local.definition.constructor` or a
    classifier fallback. Verify that the extractor accepts an anonymous
    captured name node.
  - `typealias_declaration` uses field `name` for both the alias name and
    the target type. The alias target must be the `name` child after `=`,
    not `child_by_field_name("name")`, which returns the first one.
- **F4: C# static constructors share the instance-constructor node.**
  `static C() {}` is a `constructor_declaration` with a `modifier` `static`.
  Under R2 it stays absent, so filter it in the query (`#not-match?` on
  modifiers is awkward) or in the classifier. If it would otherwise be
  captured, drop it rather than reclassify it.
- **F5: enum discriminants are not recorded today.**
  - `EnumVariantData.discriminant` is `None` for `B = 2` (Rust) and
    `BLUE = 2` (C), even though the text is right there.
  - The spec wants initializer and discriminant text retained "that the
    existing enum metadata can express", and `discriminant` is such a
    field.
  - Phase 2 reconciliation should fill it from the variant's `value` node,
    or the docs should state the gap. Filling it changes the existing
    Rust/C `kind_data` output (goldens), so record the decision.
- **F6: declaration nodes `resolve_declaration_node` does not recognize
  today** (no `_declaration` / `_definition` / … suffix, and not in
  `EXACT_DECLARATION_KINDS`):
  - Go `type_alias`
  - PHP `enum_case`
  - Scala `simple_enum_case` and `full_enum_case`
  - TypeScript `enum_assignment`, `internal_module` and `module` (F1)

  A TypeScript bare enum member has **no wrapper at all**: its parent is
  `enum_body`. Phase 2's "owner and wrapper rules" task should add these,
  and decide what the declaration node of a bare TypeScript member is (the
  name node itself).
- **F7: C++ constructors can be recognized syntactically.** In a class body,
  a `declaration` or `function_definition` with no `type` field whose
  `function_declarator` declares an `identifier` (not `field_identifier`,
  not `destructor_name`, not `operator_cast`) is a constructor. That holds
  for every form in S1, with no name comparison, which matches the spec's
  "never resolve class ownership by name alone".
- **F8: Go fields need two names per declaration.** `X, Y int` is one
  `field_declaration` with two `name` fields, and the query must emit one
  record per name. Embedded fields have no `name` field, so a capture on
  `name:` excludes them naturally.
- **F9: Go aliases are not in today's output at all.** Their arrival adds
  records to the Go goldens and fixtures. Any Go golden diff in Phase 3
  should consist of only new `TypeAlias`/`Field` records.
- **F10: facts from the `tree-hugger` skill that Phase 2/3 must respect.**
  - `phase6_neovim_query_reuse::every_locals_definition_capture_is_inventoried`
    fails when a new `@local.definition.*` capture is missing from the
    inventory, so every query edit updates the inventory in the same change.
  - `extract/qualified_name.rs` already uses a TypeScript `internal_module`
    as a naming prefix even though no record exists. That is why
    `Space.inside` already works. Once `internal_module` becomes a Namespace
    record, the prefix must come from the record, not from both, or members
    get `Space.Space.inside`.
  - `stable_symbol_key` hashes the qualified name, so any prefix change
    changes IDs.
  - `align_field_metadata` already adds standalone `Field` records to a
    type's field list. Go and Scala fields should reuse it rather than add a
    new reconciliation.

## Phase 2

Phase 2 extends the shared extraction rules so the per-language work in
Phase 3 only adds query captures and classifier arms. No kind changed: every
record of every fixture keeps its kind, and the CLI goldens have no diff
from this phase. Two record changes are deliberate and described under
"Behavior changes" below.

### Gates (2026-10-05, macOS host)

- `just test` in `tree-hugger/`: **976 run, 976 passed**, 0 skipped.
- `just lint` in `tree-hugger/`: clippy clean for `tree-hugger` and
  `tree-hugger-cli`.
- One intermediate run had one timeout,
  `file_identity::god_files_paths_are_relative_to_any_spelling_of_the_root`
  (30 s), at a host load average of about 200. It passed alone (19 s) and in
  the next two full runs. That test is outside this phase's code.
- No cross-OS run. Every change is tree-sitter node logic with no path,
  process or terminal handling, and the one CLI test change only changes the
  source text the test writes.

### Concurrent edits during this phase

A session working on `2026-10-03-symbol-filters-and-reports` kept editing
this worktree throughout Phase 2. Its changes are not part of this phase:

- `lib/src/file/query.rs`, `lib/src/file/extract/{doc_comments,flags}.rs`,
  `lib/src/file/mod.rs`, `lib/tests/l1/symbols_flags.rs`,
  `cli/tests/symbols/{cache_upgrade,filters}.rs`, `lib/README.md`,
  `docs/{cache-design,symbol-kinds-and-visibility}.md`, the sibling
  feature's files, and parts of `docs/symbol-schema-design.md`,
  `SKILL.md` and `api-reference.md`.
- **It bumped `EXTRACTOR_REVISION` to `"12"`** and regenerated
  `symbols.json`/`symbols.jsonl`. That golden diff is only
  `extractor_version` `rev.11` → `rev.12`. Phase 4 bumps from whatever value
  is current then.
- In the three shared docs, this phase changed only the lines named under
  "Docs and skill" below, with in-place edits.

### Requirement-to-test mapping

| Plan task | Behavior | Test (all L1) | Before this phase |
|---|---|---|---|
| Kind settlement | A narrow `Constructor`/`Method`/`Parameter` query equals the same kinds of a full query with the same capabilities (Declarations, Containment, all), class expressions included; `Method` excludes constructors | `symbols_query_filters::constructor_kind_is_settled_before_kind_selection` | passed (see departure D1) |
| Owner and wrapper rules | Declarations of the new forms resolve from their names: TS bare member → the name, `enum_assignment`, `internal_module`, `module` (F1); Go `type_alias`; PHP `enum_case`; Scala `simple_enum_case`/`full_enum_case`; a Scala case parameter stays `class_parameter` | `extract::classify::tests::deferred_declarations_resolve_from_their_names` | TS `module` and the F6 kinds failed |
| Owner and wrapper rules | Go field containers: ungrouped `S` at its `type_declaration`, grouped `A`/`B` at their own `type_spec`; anonymous `struct{}` fields (field type, alias target, composite literal) have none; the type is extracted at that same node | `extract::classify::tests::go_fields_are_contained_by_the_type_that_declares_them` | no Go type container existed |
| Owner and wrapper rules | Java members after enum constants are direct members | `extract::classify::tests::java_enum_body_declarations_hold_direct_members` | failed |
| Owner and wrapper rules (Go grouped) | Each grouped Go type lists only its own fields | `symbols_field_lists::grouped_go_types_list_only_their_own_fields` | failed (`B` listed `A`'s fields) |
| Owner and wrapper rules (Go grouped) | Each grouped Go type has its own declaration text, header, span; keyword `type` | `symbols_header::grouped_go_types_have_their_own_headers_and_spans` | failed (shared span, header `None`) |
| Constructor hook | JS/TS rule moved into the hook unchanged: class body and class expression → Constructor, object literal → Method, class in a function → Constructor | `extract::classify::tests::ecma_constructors_are_class_body_members_named_constructor`, `symbols_kinds::deferred_kinds_hold_through_both_paths` | passed (regression guard) |
| Alias target | Go plain, ungrouped `type_declaration`, grouped, generic; Swift plain and generic; Scala plain and generic; `None` for Scala abstract, Go defined type and a C typedef | `extract::classify::tests::alias_targets_are_the_written_right_hand_side` | Go/Swift/Scala failed |
| Variant reconciliation | Paired variants keep reader data and fill only gaps; missed standalone variants are added in source order; no duplicates; repeated names pair in order; a list is built from standalone variants alone | `tree_file::definitions::tests::variant_lists_merge_standalone_variants_without_duplicates`, `…variant_lists_are_built_from_standalone_variants_alone` | new code |
| Variant reconciliation (F5) | `discriminant` text on `symbols()`, the record and the round trip, for Rust, C, C++ scoped, C#, TS (number and string), PHP backed; Java constant arguments stay `None`; variants never enter the field list | `symbols_field_lists::enum_variant_lists_keep_discriminant_text` | failed (always `None`) |
| Namespace rule | A namespace record qualifies members once: TS `namespace`/`module`, PHP semicolon and braced; the member's name is identical with and without the record | `extract::classify::tests::namespace_records_qualify_their_members_once` | passed (the builder already reuses a record as owner) |
| Namespace rule | TS namespaces contain their members (nested included), never themselves; `module Mod {}` is a `Namespace` container; `declare module "x"` stays `Module` | `extract::classify::tests::typescript_namespaces_contain_their_members`, new `KIND_MATRIX` row `namespaceFunction`, updated rows in `symbols_scope` and `symbols_qualified_names` | failed (no container) |
| Scaffolding | Table-driven positive (`DEFERRED_KIND_ROWS`) and negative (`DEFERRED_KIND_NEGATIVES`) rows through both public paths, one section per language | `symbols_kinds::deferred_kinds_hold_through_both_paths`, `symbols_kinds::deferred_kind_counterexamples_keep_their_kinds` | new |
| Adapter agreement | v1 and v2 agree after reconciliation (caught the Swift `Type` enum case, see D4) | existing `symbols_adapter::every_fixture_symbol_round_trips_through_the_record` | — |

Every new test is in a declared target: the library's unit tests, or
modules `mod`-declared in `lib/tests/l1/main.rs` / `cli/tests/symbols/`. No
test name or module has a tier marker, so all run in L1. No new test reads a
repository file.

### What changed, by plan task

**Kind settlement.** `TreeFile::definitions` calls `refine_symbol_kind`
for every capture before `MetadataSelection` is consulted, so a light
(unselected) record's kind is already final. No code change was needed. The
new test is a regression guard.

**Owner and wrapper rules** (`declaration.rs`, `classify.rs`):

- `EXACT_DECLARATION_KINDS` gains `enum_case`, `simple_enum_case`,
  `full_enum_case`, `enum_assignment`, `type_alias`, `internal_module` and
  `module` (F6).
- **F1 fixed:** `module` is no longer a scope boundary by kind; every root
  is one already (`parent().is_none()`), which covers Python's `module`
  root.
- A bare TypeScript enum member has no wrapper. Its declaration is the name
  node itself (the existing fallback), and the container search finds the
  enum through `enum_body`.
- `is_member_body_kind` adds Java's `enum_body_declarations`.
- **Go grouped types**: `context_for_name` (`declaration.rs`) replaces a
  grouped `type ( … )` context by the name's own `type_spec`/`type_alias`,
  so declaration, anchor, span, header, type metadata and naming-owner
  identity are per type. An ungrouped `type S …` keeps its
  `type_declaration`, so its header stays `type S struct` and the goldens are
  unchanged. `is_grouped_go_type_declaration` is the one predicate.
- `ancestor_container` has a Go arm (`go_type_container`): a `type_spec` is
  its type's container, at the node the type's symbol is extracted at. The
  generic `("type_declaration", Type)` entry was removed; Go was the only
  grammar with that node, and it never resolved a name.
- `is_unnamed_type_owner`: a Go `struct_type` not directly under a
  `type_spec` (field type, alias target, composite literal) is an unnamed
  owner, so its fields get no invented container.
- C++ `template_declaration` was already a transparent wrapper; nothing was
  needed.

**Constructor hook** (`classify.rs::is_constructor_declaration`): the one
place a `Function`/`Method` becomes `Constructor`, called before the
`Function` → `Method` promotion. `refine_symbol_kind` still demotes a
function-local constructor afterward. Today it holds only the JS/TS rule,
moved verbatim from its old arm. It is restricted to `Function`/`Method`
captures, where the old arm accepted any capture kind; no test or fixture
changed. Phase 3 adds arms gated on `is_direct_member`.

**Static and return type check:** the Python, Swift, PHP and C++ static
readers decide by keyword or decorator only (`@staticmethod`/`@classmethod`,
`static`/`class`, `static_modifier`, `storage_class_specifier`), never by a
missing receiver, so a reclassified constructor is not made static.
`extract_signature` takes no return type from the owner.

**Alias target:** `extract_type_alias_target(node, language, source)`
reads Rust `type_item`, Go `type_alias` (and an ungrouped
`type_declaration`'s alias child), Scala `type_definition`'s `type` field,
the node after `=` in Swift `typealias_declaration` (F3), and the TS form as
before. The language parameter keeps C's `type_definition` (a typedef) out:
its `type` field is only the declarator's base type.

**Variant and field reconciliation** (`definitions.rs`):

- `align_variant_metadata` runs after `align_field_metadata`. For an
  `Enum` owner, it merges the reader's variant list with the standalone
  `EnumVariant` records the owner contains (pure `reconcile_variants`). It
  creates metadata when the reader read none; Scala `Color` has none today,
  so its list will come from the standalone cases.
- `MetadataSelection::for_kinds` adds `EnumVariant` when `Enum` is
  selected, so a narrow `Enum` query reads the same variant doc comments as
  a full one.
- Fields: `align_field_metadata` needed no change. Go `Field` records
  (Phase 3) will pair by name with the Go reader's entries, because their
  container node is the type's own node, and the reader's `type_text` is
  kept.
- **F5 decided: fill `discriminant`.** `VariantInfo` gains
  `discriminant: Option<String>` (serde: skipped when `None`, defaulted on
  read). `variant_discriminant` reads the `value` field in the Rust, C/C++,
  C#, TS and PHP readers. Both schema conversions carry it. No golden
  fixture has an explicit enum value, so the goldens are unchanged.

**Namespace rule:**

- `qualified_names` already treats an extracted `Namespace` as a naming
  owner, so a record at `internal_module`, `module` or a braced PHP
  `namespace_definition` replaces the unextracted-name prefix and produces
  the same text. A semicolon PHP namespace is a sibling, never an owner, and
  its members keep `App\inside` from `file_namespace_before`.
- The namespace's own name is unchanged by the record:
  `is_namespace_node_kind` is untouched, so TS names stay un-rooted.
- **Containers, F2 decided:** TS `internal_module` and identifier-named
  `module` are `Namespace` containers, matching C++, C# and braced PHP.
  Members of a semicolon PHP `namespace App;` keep no container, as members
  of a Java `package` or Go `package` clause have none. Changing that would
  change the container of every record in every such PHP file. Container is
  not an ID input, so it can be revisited.

### Behavior changes (intended, no golden diff)

1. **Grouped Go types are separate declarations.** Each type in
   `type ( … )` now has its own span, declaration text, header
   (`Second int`, previously `None`) and field list (previously the first
   spec's).
2. **TypeScript namespace members have their namespace as container.** For
   example, `namespaceFunction` → container `ExportedNamespace`, container
   kind `Namespace`. IDs are unchanged.
3. **Enum variant lists carry `discriminant`** where the source states a
   value.

### Departures from the plan

- **D1:** the plan asks for a failing narrow-vs-complete test first. It
  passed on its first run: kinds are already settled before selection.
  It stays as a regression guard.
- **D2:** the Phase 2 checkpoint expects no golden diff. That holds for
  this phase. The goldens do show a diff, but it is the other session's
  revision bump (above).
- **D3:** `cli/tests/symbols/order.rs`
  `equal_declaration_spans_fall_through_to_the_id_tie_breaker` used grouped
  Go types as its source of records that tie on span and kind. Behavior
  change 1 removes that tie, so the test now uses C
  `typedef int Alpha, Beta, …;`, whose aliases still tie (verified: every
  alias's declaration span is the whole typedef).
- **D4:** variant reconciliation is limited to `Enum` owners. A Swift enum
  is `Type` (R1), and its record has no variant list, so completing its v1
  metadata made `symbols_adapter::every_fixture_symbol_round_trips_through_the_record`
  fail.

### Docs and skill

- `docs/symbol-schema-design.md`: the header-omission rule no longer lists
  grouped Go types, and a bullet states their per-spec header.
- `.claude/skills/tree-hugger/api-reference.md`: same correction.
- `.claude/skills/tree-hugger/SKILL.md`: removed "Go grouped `type (…)`
  specs share one span" from the ordering note.
- **Drift found:** `header/mod.rs::signature_text`'s doc and its
  `is_grouped_go_type` rule described grouped types the code no longer
  produces. The function was removed and the doc rewritten. The doc of
  `is_unnamed_type_owner` now lists the Go case. Other docs that describe
  Phase 2 plumbing are left to Phase 5.

### Findings for Phase 3 and later

- **F11: constructors lose the `semantics` facet.**
  `analysis::semantic_pass` and `coverage::stamp_parse_pass` act on
  `Function | Method` only. A method reclassified as `Constructor` (Python,
  PHP, C++) will lose `semantics` and `parse_pass: semantic`, as TS/JS
  constructors already do. Recommended: include `Constructor` in both, in
  Phase 4 with the golden update (TS/JS constructors gain the facet).
- **F12:** a Scala `case Value(x: Int)` parameter will be qualified
  `Color.Value.x` once `Value` is extracted, because `EnumVariant` is a
  naming kind. Its ID changes. The spec allows this ("affected
  descendants"); Phase 4's ID tests should expect it.
- **F13:** `symbols_field_lists::php_go_and_scala_controls_keep_their_lists`
  pins Go struct fields as metadata-only (`Standalone::No`). The Phase 3 Go
  task flips them to `Yes`.

## Phase 3

Progress notes are appended as each wave closes; the summary, mapping and
departures are completed at the end of the phase.

### Wave 1 (TypeScript, Go, Python, C), checkpoint 3a green

- `just test`: 981 run, 981 passed. `just lint`: clean.
- CLI goldens regenerated (`HUG_UPDATE_GOLDEN=1`); the only record changes
  are the Go field `Widget.Size` (new) and Python `Gadget.__init__`
  (Method → Constructor). Every other line of the JSON diff is the
  placeholder `<id-N>` renumbering after the inserted record.

### Wave 2 (Java, C#, C++), checkpoint 3b green

- `just test`: 982 run, 982 passed. `just lint`: clean.
- Golden record changes: C++ `geometry::Vector::Vector` (Method →
  Constructor) and Java `Counter.Counter` (new Constructor). The Java
  constructor's parameter `start` keeps its name and ID.

### Wave 3 (Swift, Scala, PHP), checkpoint 3c green

- `just test`: 985 run, 985 passed. `just lint`: clean.
- No golden change in this wave: the `multi/` fixtures declare no Swift
  initializer or alias, no PHP constructor, case or namespace, and only a
  Scala class parameter (which stays a Parameter).
- Every cell of the spec's deferred table is `landed` in
  `symbols_kind_coverage.rs`; all 16 `unsupported` lists are empty. The one
  grammar limit, G1, is a demonstrated `GRAMMAR_LIMITS` entry (below).

### Rows and forms delivered

| Language | Kind | Forms that land | How |
|---|---|---|---|
| TypeScript | EnumVariant | bare, `= value`, quoted (`"x y"`, name without quotes), in `const enum`, `export enum`, a function-local enum | overlay `queries/typescript/locals.scm` |
| TypeScript | Namespace | `namespace A`, dotted `namespace A.B` (one record, full name), `module A`, `declare namespace`, nested | overlay query; keyword `namespace`/`module` |
| Go | TypeAlias | `type A = B`, grouped, generic | vendored query (`type_alias`) |
| Go | Field | each named field of a named struct (`X, Y int` gives two), grouped and generic structs, function-local named struct | vendored query (fields under a `type_spec`'s own `struct_type`) |
| Python | Constructor | direct and decorated `__init__`, in a class nested in a function or an `if` | classifier arm |
| Java | Constructor | explicit constructors (class, enum, record non-compact, local class), overloads | vendored query |
| Java | EnumVariant | constants, with arguments, with a class body | vendored query |
| C# | Constructor | instance constructors with a block or expression body, `: this(…)`, struct and record; `static` excluded in the query | vendored query (`#not-eq? @_modifier "static"`) |
| C# | EnumVariant | members, with values, attributed | vendored query |
| C / C++ | EnumVariant | named, anonymous (no container), typedef'd, in a struct, function-local; C++ scoped/unscoped, in a class or namespace | C query recaptured as `enum_variant` (C++ inherits) |
| C++ | Constructor | `C();`, `C(int) {}`, `= default`, `= delete`, `explicit`, template-wrapped, in class templates, namespaces and local structs | classifier arm, syntax only (F7) |
| Swift | Constructor | `init`, `init?`, `init!`, convenience/required, in classes, structs, enums, extensions and protocols; overloads | vendored query (anonymous `init` token captured as the name) |
| Swift | TypeAlias | top level, generic, in a type | vendored query |
| Scala | EnumVariant | `case A, B`, `case V(x: Int)`, braceless `enum E:` | vendored query |
| Scala | TypeAlias | `type A = B`, generic; abstract `type E` excluded | vendored query (`type:` field required) |
| Scala | Field | direct `val`/`var` definitions and declarations, `lazy val`, in classes, traits, objects (static), enums | classifier arm |
| PHP | Constructor | `__construct` in any case spelling, in classes and traits | classifier arm (`eq_ignore_ascii_case`) |
| PHP | EnumVariant | unit and backed cases | `php_only` query |
| PHP | Namespace | semicolon and braced, full name; global `namespace {}` has none | `php_only` query |

Metadata decided in this phase:

- **Access.** TS enum members take the enum's level (new `enum_body` arm
  in `visibility/ecma.rs`); a TS `namespace` is a top-level declaration (its
  `expression_statement` wrapper is transparent). Java constants and C#
  members are Public. A Java enum constructor without a keyword is Private
  (JLS 8.9.2). C/C++ enumerators stay Unknown. Scala cases already inherit
  the default Public; PHP cases are Public through the existing reader.
- **Static.** No constructor or variant gets `Static`; Scala object fields
  are static and class/trait fields are not, by the existing member rule.
- **Keywords.** New keyword sites: TS `internal_module`/`module`, Swift
  `init_declaration`/`typealias_declaration`, Scala `type_definition` and
  enum cases (`case`, read from `enum_case_definitions`), PHP `enum_case`
  and `namespace_definition`. Java, C# and C++ constructors, TS, Java,
  C#, C and C++ enum members and Go fields have no keyword token and report
  none; a Python constructor keeps `def` and a PHP one `function`.
- **Headers.** An alias's target is its initializer and is cut, so an
  ungrouped Go `type A = B` now gives `type A` like the grouped form and
  every other language (`type_alias` added to the declarator kinds).
- **Qualified names.** An ungrouped Go struct's fields were `S.S.X`: the
  prefix walk met the `type_spec` (named by the Go container rule) and then
  the `type_declaration` holding `S`'s record. `qualified_name.rs` now
  reuses the record when a container node is itself an extracted symbol, so
  they are `S.X`. Only the Go `type_spec` case reaches that branch.
- **Field lists.** Scala's type-metadata reader now lists body `val`/`var`
  members with their written types, so `align_field_metadata` pairs them by
  name rather than adding them untyped. A paired entry also takes the
  standalone member's doc comment when the reader read none.

### Requirement-to-test mapping

All tests are L1, in the declared `l1` binary (`lib/tests/l1/main.rs`
declares the new `symbols_deferred_metadata` module); no name segment has a
tier marker. None reads a repository file.

| Requirement | Test |
|---|---|
| Each form's kind and container on both paths | `symbols_kinds::deferred_kinds_hold_through_both_paths` (`DEFERRED_KIND_ROWS`, one section per language) |
| Negative cases keep their kinds | `symbols_kinds::deferred_kind_counterexamples_keep_their_kinds` (`DEFERRED_KIND_NEGATIVES`, extended per language) |
| Scope of the new records | `symbols_kinds::deferred_kinds_get_their_scope` (new) |
| Qualified name, access, static, keyword, header, parameters (defaults included, no invented return type) and alias target | `symbols_deferred_metadata::deferred_declarations_report_their_metadata` (new, `DEFERRED_METADATA_ROWS`) |
| Overloaded constructors are separate records with distinct IDs | `symbols_deferred_metadata::overloaded_constructors_are_separate_records` (Java, C#, C++, Swift) |
| Cells landed; language-wide guards kept | `symbols_kind_coverage::deferred_constructs_are_unsupported_extraction`, `kinds_a_language_lacks_never_appear_in_its_fixture` |
| Grammar limit G1 demonstrated | `symbols_kind_coverage::grammar_limits_are_demonstrated` (new) |
| Go fields pair with the field list, typed; embedded and anonymous-struct fields and interface method elements are not fields | `symbols_field_lists::go_struct_fields_pair_with_the_field_list_and_keep_their_types` (new), `grouped_go_types_list_only_their_own_fields` and `php_go_and_scala_controls_keep_their_lists` (flipped to standalone) |
| Scala body fields pair with the field list, typed, with docs; class parameters stay metadata only | `symbols_field_lists::scala_body_fields_pair_with_the_field_list` (new), `php_go_and_scala_controls_keep_their_lists` |
| Scala cases are variants of their own enum | `symbols_function_locals::scala_enum_given_and_extension_members_belong_to_their_owner` (updated) |
| C++ fixture constructor | `symbols_kinds::functions_declared_directly_in_a_type_body_are_methods` (updated) |
| Query provenance and capture inventory | `phase6_neovim_query_reuse::every_locals_definition_capture_is_inventoried`, `queries::provenance::tests::changed_vendor_queries_record_their_changes` (Go and Scala added; the unchanged control is now Lua) |
| CLI output | the eight `multi/` goldens (`golden::*`), regenerated and reviewed per wave |

### Departures from the plan

- **D5: no per-wave worktrees.** Phase 2's work is uncommitted, so a
  worktree created from `HEAD` would lack the plumbing every Phase 3 task
  builds on. The three waves ran sequentially in this worktree, each closed
  by its own checkpoint.
- **D6: metadata rows live in one new module.** The access, static, header
  and qualified-name matrices assert the primary fixtures. Rather than
  adding the deferred constructs to ten fixtures (which every
  fixture-wide matrix would then have to absorb), each deferred
  declaration's metadata is one row of `symbols_deferred_metadata.rs`,
  asserted on both paths with the existing `assert_level` and
  `assert_static` helpers. The fixture-wide guards (adapter round trip,
  unique IDs, contiguous headers, no Unknown kind) still cover the
  fixtures' existing enums, namespaces and constructors.
- **D7: Swift headers and parameter defaults.** tree-sitter-swift places a
  parameter's `= default` directly in the function node. The header cut at
  that `=` (`func f(x: Int, y: Int`) and the signature reader looked for the
  default inside the parameter, so no Swift default was ever read. Both are
  fixed for `function_declaration`, `init_declaration` and
  `protocol_function_declaration`, which changes existing Swift function
  headers and parameters. No golden or fixture test changed.
- **D8: the attributed-enumerator test was rewritten.**
  `symbols_declarator_wrappers::c_and_cpp_attributed_enumerators_match_plain_ones`
  compared `RED [[deprecated]]` with `GREEN`. Neither pinned grammar parses
  an attribute on an enumerator, so `GREEN` is a top-level declarator after
  error recovery; the test passed only while both were `Variable`. It is now
  `c_and_cpp_attributed_enumerators_do_not_parse`: `RED` is a variant of
  `Color`, and `GREEN` is never parsed as one. Not a required form.
- **D9: goldens updated in Phase 3.** Phase 4 owns the golden update, but
  each Phase 3 checkpoint requires a green `just test`, so the goldens were
  regenerated per wave and reviewed (above). Phase 4 regenerates them again
  after the revision bump.
- **D10: G1 is a `GRAMMAR_LIMITS` entry, not `unsupported`.** The
  `unsupported` check requires a clean parse, and G1's whole point is that
  the snippet does not parse. The new entry records the snippet, grammar,
  reason and remaining records (`Hidden` Enum Public, `H` EnumVariant
  Public), and its test fails once the grammar parses it.
- **D11: forms beyond the table that landed with the required ones:** TS
  quoted enum members; Swift initializer requirements in a protocol
  (`Constructor` in an `Interface`); Java constructors in an enum (Private).
- **D12: provenance and inventory notes** were added with each query edit
  (ruling R4), ahead of Phase 5's provenance task. The inventory gained
  `local.definition.constructor`.

### Findings for Phases 4 and 5

- **F11 still applies**: Python, Java, C#, C++, Swift and PHP constructors
  are now `Constructor`, so `semantic_pass`/`stamp_parse_pass` (Function |
  Method only) skip them.
- **F14 (doc drift, code is right):** the skill says Go interface method
  elements are listed as fields. `extract_go_interface_methods` matches
  `method_spec`, which tree-sitter-go 0.25 no longer produces
  (`method_elem`), so no Go interface lists anything. The new Go field-list
  test pins that. Phase 5 should correct the skill and docs, or the author
  may choose to fix the reader.
- **F15 (pre-existing, not changed):** a C++ `= default;` / `= delete;`
  member's header keeps the clause (`C(const C&) = default;`).
- **F16 (pre-existing adapter loss, all languages):** a standalone `Field`
  record's `FieldData` has no `type_text`, and `is_mutable`/`is_readonly` are
  fixed at true/false. The owner's field list carries the type.
- **F17:** a TS `declare namespace D {}` reports Unknown access, like every
  other `declare` declaration (`ambient_declaration` is not a transparent
  wrapper in `top_level_visibility`).
- **Docs to update in Phase 5:** `docs/symbol-kinds-and-visibility.md`
  (deferred cells, "`namespace` is not yet reported", G1), the skill's
  "Go fields are deferred" line, `docs/symbol-schema-design.md` (alias
  headers, field-list doc fill, Swift parameter defaults).

### Cross-platform

Every change is tree-sitter node logic, query text or test tables; nothing
touches paths, processes or terminals, and the new tests use the existing
`extract_inline` temp-file helper. No cross-OS run was made; CI covers
Windows and WSL2 after merge.
