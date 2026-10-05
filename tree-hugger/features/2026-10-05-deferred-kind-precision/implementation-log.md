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
