---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-08-scoped-references/spec.md"
plan: "tree-hugger/features/2026-10-08-scoped-references/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - tree-hugger/docs/references.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/shared/schema_v2/occurrences.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/analysis/scope/rust.rs
  - tree-hugger/lib/src/analysis/scope/ecma.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/queries/rust/references.scm
  - tree-hugger/lib/queries/typescript/references.scm
  - tree-hugger/lib/queries/javascript/references.scm
  - tree-hugger/lib/tests/l1/symbols_occurrences.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
docs_updated_during_phase_2:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/query-system.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/tests/l1/symbols_reference_summaries.rs
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
docs_updated_during_phase_3:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
---

# Implementation Log for 2026-10-08-scoped-references (6 phases)

## Phase 1

Phase 1 changed no production code. It produced the capture inventory, the
scope study, the planned language table
(`tree-hugger/docs/references.md`) and the rulings below.

### Prerequisites

All three `depends-on` features are in the tree:

- `2026-10-03-symbol-improvements` (in `_completed`): `NameIndex` candidate
  index (`lib/src/analysis/mod.rs`, `coverage.rs`), selective `Capability`
  enum (`lib/src/file/query.rs`: Declarations, Containment, References,
  Semantics, StructuredDocs, Diagnostics, Flags, Metrics), and
  `semantic_pass` reporting `is_recursive: Some(Unknown)`.
- `2026-10-03-symbol-filters-and-reports` (in `_completed`): qualified names
  and smart-case `NamePattern`.
- `2026-10-05-deferred-kind-precision` (active directory): its spec has
  `implemented: true`, but `status` still reads `draft-spec`, and the
  directory has `review-1.md`/`review-2.md`. `SymbolKindV2::Constructor` and
  the constructor rules are in the code. This is not a gap for this feature;
  the stale `status` is the author's to close.

**Baseline differs from the plan.** The plan and spec assume schema 4.0 and
`EXTRACTOR_REVISION` "10". The tree is at `SchemaVersion::CURRENT = V4_3`
(`lib/src/shared/schema_v2/types.rs`) and `EXTRACTOR_REVISION = "29"`
(`lib/src/file/mod.rs`). See ruling 3.

### Capture inventory (all 16 languages)

How the effective queries are assembled (`lib/src/queries/mod.rs`): the
`locals` query is the vendored `queries/vendor/<lang>/locals.scm` with its
`; inherits:` chain resolved (TypeScript and JavaScript inherit `ecma`, C++
inherits `c`, PHP inherits `php_only`). JavaScript and TypeScript then add
their own `queries/<lang>/locals.scm` overlays (`; extends`). The
`references` query is the language's own `queries/<lang>/references.scm`
(a separate query, run by `TreeFile::references_from`, which only reads
`@reference` captures). Today nothing reads `@local.scope` for binding
(`bind_references` uses `NameIndex::named(name).first()`).

Counts are pattern counts in the effective text, computed with a throwaway
script outside the repository.

| Language | `@local.scope` node kinds (effective) | Definition kinds | `locals` `@local.reference` patterns | `references.scm` shape |
|---|---|---|---|---|
| Rust | block, function_item, closure_expression, if/for/while/loop, match_expression, match_arm, struct_item, enum_item, impl_item (one alternation) | enum, enum_variant, field, function, import, macro, method, module, trait, type, type_alias, var | 3 | broad `identifier`, `type_identifier`, field-expression value, scoped-identifier path, macro name, type args (6) |
| TypeScript | statement_block, function_expression, arrow_function, function_declaration, method_definition, for_statement, for_in_statement, catch_clause (from `ecma`) | class, enum, enum_variant, field, function, import, interface, method, namespace, parameter, property, type, var; **plus bare `@local.definition`** for plain `required_parameter`/`optional_parameter` | 2 | broad `identifier`, `type_identifier`, member object, call function, type args, template substitution, shorthand property (7) |
| JavaScript | as TypeScript (`ecma`) | class, field, function, import, method, parameter, var | 3 | as TypeScript minus types, plus computed property name (6) |
| Go | source_file, block, function/method declarations, func_literal, if, for, expression_switch | field, function, import, method, namespace, type, type_alias, var | 8 | broad `identifier`/`type_identifier`, selector operand, call function, qualified-type package, composite literal type (6) |
| Python | module, function_definition, class_definition, comprehensions; **also on class bodies with assignments and on `aliased_import`/`import_statement`/`import_from_statement`** | class, field, function, import, method, parameter, var | 1 | broad `identifier`, attribute object, call function, subscript, type, decorator, for-in clause (7) |
| Perl | block, function_definition | function, module, var | 0 | scalar/array/hash variables, bareword call, package name (5) |
| Lua | chunk, function declaration/definition, do/for/if/repeat/while | associated, function, method, parameter, var | 1 | broad `identifier`, dot/bracket/method index tables, call name (5) |
| Bash | function_definition | function, var | 2 | expansions, `command_name`, **every `word`** (4) |
| Zsh | function_definition | function, var | 2 | as Bash (4) |
| C | translation_unit, function_definition, compound_statement, struct_specifier, for/if/while; **also `preproc_function_def`** | enum, enum_variant, field, function, macro, parameter, type, type_alias, var; **plus bare `@local.definition`** | 4 | broad `identifier`/`type_identifier`, field-expression argument, pointer argument, call function, sizeof type (6) |
| C++ | C's, plus class_specifier, template_declaration, namespace bodies, lambda, try body, catch, requires; **also `function_declarator`, `template_function`, `template_method` (declarator nodes, not bodies)** | C's plus class, method, namespace | 5 | C's plus `namespace_identifier`, qualified scope, template args (7) |
| C# | block only | class, constructor, enum, enum_variant, import, interface, method, namespace, parameter, property, type, var | 1 | broad `identifier`, member object, invocation function, type args, `new` type, base list, attribute, typeof (8) |
| Java | program, block, method/constructor declarations, lambda, for/enhanced-for, if, try, catch | class, constructor, enum, enum_variant, field, import, interface, method, namespace, parameter, type, var | 2 | broad `identifier`/`type_identifier`, invocation object, field-access object, type args, `new` type, annotations (8) |
| Scala | block, function_definition, lambda, template_body, for_expression | class, enum, enum_variant, function, import, module, parameter, trait, type_alias, var | 1 | broad `identifier`/`type_identifier`, field value, call function, type args, instance type, annotation (7) |
| Swift | statements, function/class/protocol/property declarations, do/for/guard/if/repeat/switch/while | constructor, enum_variant, field, function, import, interface, parameter, type, type_alias, var | 0 | broad `simple_identifier`, user types, navigation target, call, type args, attributes (6) |
| PHP | class/interface/trait/enum/method declarations, function_definition, anonymous function **(all on the declaration node)** | class, constant, enum, enum_variant, field, function, import, interface, method, namespace, trait, var | 10 | variable names, broad `name`, qualified names, member object, call function, class constant (6) |

Findings that matter for Phase 2:

- **No `references.scm` separates a call target from any other use.** The
  `Call`/`Usage` split needs new captures in every language. That is why the
  limited languages report `kind` from call syntax where the existing pattern
  shows it (`call_expression function: (identifier)` and equivalents) and
  otherwise `Usage`.
- **Member-call names are missed where the grammar gives them their own
  node kind**: Rust `field_identifier` (`x.f()`), TS/JS `property_identifier`
  (`obj.f()`), Go `field_identifier`. Java, C#, Python, Lua and Swift capture
  member names only through the broad `identifier` pattern, with no receiver
  or call distinction.
- **Scope captures on declaration or import nodes**: Python (import
  statements, assignment-bearing class bodies), C (`preproc_function_def`),
  C++ (`function_declarator`, `template_function`, `template_method`), PHP
  (every scope is a declaration node), Rust (`function_item`, `struct_item`,
  `enum_item`, `impl_item`: the declaration node, so the name and the header
  sit inside the scope they open).
- **Kindless definitions**: TypeScript plain parameters and one C pattern use
  a bare `@local.definition`. The scope tree must keep them as binding facts,
  or plain parameters will not shadow outer names.
- Bash/Zsh `word` captures every command argument, so most of their
  occurrences are not references at all; another reason they stay limited.

### Scope study (Rust and TypeScript)

The study used throwaway code only, at `/tmp/scoped-refs-spike/` (not in the
repository). It is a crate pinned to `tree-sitter` 0.26.3,
`tree-sitter-rust` 0.24.0 and `tree-sitter-typescript` 0.23.2, the same
grammars as `lib/Cargo.toml`. It reads the effective `locals` text for
each language and the repository's `references.scm`. It builds a
scope tree from `@local.scope`, puts each definition in its innermost scope
(or the parent scope when the scope node is the declaration itself or the
pattern sets `definition.*.scope parent`), and compares two bindings for every
`@reference`:

- **first-match**: the first definition of the name in document order.
  This approximates today's `NameIndex::named(name).first()`.
- **scoped**: innermost scope outward. Members (method, field, property,
  variant) are never lexical names. Rust adds a `let`-after-statement order
  rule, the latest preceding `let` wins, and type/value categories are kept
  apart.

Changed-target counts are not correctness evidence. Each class below was
judged by reading samples.

| Corpus | References | Same target | Unbound both ways | Changed target | First-match bound, scoped unbound | Ambiguous in scope | Member calls not captured | `Type::f()` path calls |
|---|---|---|---|---|---|---|---|---|
| Rust fixtures (7 files under `lib/tests/fixtures`) | 152 | 37 | 100 | 9 | 6 (1 member, 4 category, 1 path) | 0 | 2 | 2 |
| Rust `lib/src` (95 files, extra real-code sample) | 31 334 | 7 743 | 12 202 | 7 533 | 3 856 (805 member, 1 436 category, 1 615 order/out of scope) | 0 | 7 034 | 958 |
| TypeScript fixtures (10 files) | 126 | 29 | 83 | 0 | 12 (6 member, 6 kindless-parameter) | 2 | 11 | n/a |

**Corrected** (the scoped target is right and first-match was wrong):

- Rust `var -> var` (6 392 of the `lib/src` changes): a local or
  parameter reference that first-match bound to another function's local of
  the same name. Every sampled row was correct, for example
  `dead_code.rs:457` `tree` moved from L406 to L442.
- Rust `field -> var` (847): shorthand `Self { size }` and parameters named
  like a struct field. A field is never a lexical name.
- Rust `function -> function` (28), including the nested
  `fn find_macro` repeated in two functions of `dead_code.rs`. Its self-call
  `find_macro(child)` moved from the *other* nested function to its own
  declaration. This is exactly the shape honest recursion depends on.
- Rust `function/module -> var` and `var -> function`: a local named like
  a free function or module (`definitions`, `named`, `stats`), or the
  reverse (`reference(...)` in `analysis/tests.rs` bound to a `let reference`
  in another test).
- Rust member/category unbinds: a bare name that first-match tied to a
  method, field or variant, or a value name tied to a type.

**Still uncertain, or wrong in the naive study** (Phase 2 must handle these):

- `impl fmt::Display for X { fn fmt(..) }`: the impl header and the method
  name both sit inside the `impl_item` scope, so the naive tree bound the
  `fmt` qualifier to the method (`symbols/rust.rs:129-130`,
  `rule_registry.rs:621-622`, `shared/symbol.rs:757`). Associated items must
  stay out of lexical tables, and impl/trait headers must resolve in the
  enclosing scope. These rows count as "changed", which shows why changed
  counts are not evidence.
- `Widget::new(..)` (`symbols/multi/rust.rs:26`) and 958 `Type::f()` calls in
  `lib/src`: first-match bound the name `new` by coincidence. Scoped lookup
  unbinds it until the path's type resolves to one in-file type with exactly
  one inherent `f` (ruling 2).
- 7 034 Rust and 11 TS member calls (`x.f()`): the name is not captured at
  all today. Under ruling 2 they become `NameOnly` candidates, which needs new
  `references.scm` patterns.
- TS `import { readFile as read }`: `import_specifier (identifier)` captures
  both `readFile` and `read`. The original name is wrongly bound, which caused
  the two "ambiguous" rows (`imports.ts:21`). Phase 2 must bind only the
  alias (the `alias` field when present).
- TS plain parameters (`function greet(name = "World")`): the bare
  `@local.definition` was ignored by the study, so `name` fell through to an
  arrow parameter at L18. Phase 2 must keep kindless definitions.
- TS interface/class members sit in the file-root scope (there is no
  `class_body`/`interface_body` scope), so an interface `greet()` signature
  competed with `function greet` (`sample.ts:18`). Member kinds must be
  excluded from lexical tables, as in Rust.
- Rust scopes missing from the vendored query: `source_file` (the study used
  a synthetic root), inline `mod_item` bodies (items of `mod x { … }` leak
  into the file scope) and `trait_item`.

**Sizing for Phase 2.** The scope tree and lookup are small: one scope
vector with parent links, plus a `(scope, category, name)` index. Most of the
work is language rules and query edits:

1. Rust: add `source_file`, `mod_item` body and `trait_item` scopes. Exclude
   associated items, resolve headers outside their impl, add the `let` order
   rule, keep type/value/macro categories, add path calls through
   `Self::`/`Type::`, and capture `field_identifier` as a method-call
   `Call` (receiver `Usage`).
2. TS/JS: bind kindless parameter definitions, bind only the import alias,
   exclude members, add `var` vs `let`/`const` scoping and the TDZ, capture
   `property_identifier` in `call_expression function: member_expression`
   as a `Call`, and keep type/value categories for TS.
3. All languages: separate call-target captures, so `kind` is `Call` vs
   `Usage`, and conservative `Limited` occurrences for the other 13.

Rust lookups per file grow with references, not references × declarations.
The largest file in `lib/src` ran in well under a second in the throwaway
crate. This is not a benchmark, and no timing was recorded (ruling 9).

### Rulings (accepted for implementation; `yolo: true`)

1. **First-release breadth: option 2.** Rust, TypeScript and JavaScript
   resolve the forms marked **Resolved** in `docs/references.md`.
   JavaScript ships with TypeScript, because the `ecma` query and the lookup
   rules are shared. The other 13 languages emit conservative occurrences
   (never `Scoped`) and `Limited(unsupported_language)` coverage. Limited rows
   never count as resolved forms.
2. **Receiver inference: option 1, narrowed.** Only explicit local paths and
   receiver forms that the language rules prove resolve. **Departure from the
   plan default:** Rust `self.f()` is **dropped** from the resolved forms and
   stays `NameOnly`, because auto-deref and trait methods mean the in-file
   inherent method is not proven. Rust `Self::f()`/`Type::f()` resolves only
   when the path's type is one in-file type and exactly one inherent impl of
   it declares `f`. TS/JS `this.f()`/`obj.f()` stay `NameOnly`. So
   recursion through `self.f()` reports `Unknown`, never `Yes`.
3. **Schema number: 5.0, from the actual 4.3 baseline.** No other active
   feature (`2026-10-09`…`2026-10-11`) advances the record schema; each of
   their specs keeps it. **Departure from the plan text:** the extractor
   revision bump is from the current value (29 today) to the next one at
   Phase 4 time, not "10" → "11". A `V5_0` constant follows the
   `SchemaVersion` doc pattern. The cache-upgrade tests need a
   `StaleRevision` row for the old revision, and an
   `assert_schema_bump_entry_is_not_served` row for 4.3, as earlier bumps
   did (see the tree-hugger skill).
4. **Report envelope: `REPORT_SCHEMA_VERSION` stays 1.** **Departure from
   the plan default (bump to 2):** the declaration envelopes keep their keys.
   The `--json-full` indexes gain fields that the record `schema_version`
   (5.0) already announces. The spec says to bump "where envelope shapes
   change", and `2026-10-10-stats-package-grouping` keeps 1 on the same
   reasoning. `hug refs` gets its own envelope shape at
   `report_schema_version: 1`, told apart by its keys, as `files` vs
   `indexes` already are. If Phase 5 finds that its envelope must share a key
   with a different meaning, it bumps the constant then.
5. **Order and hoisting: encode exactly the rows of the table.** Rust `let`
   after its statement (the initializer sees the outer name) and items
   order-free. TS/JS `let`/`const`/`class` with the TDZ (the whole block),
   and `function`/`var` hoisted (`var` to the function). Python's
   local-by-assignment is only asserted as `Limited`, never a wrong binding.
   Anything else is `Limited(unsupported_binding_form)`.
6. **Ownerless regions. Departure from the plan default for TS/JS
   parameter defaults:** a default is evaluated by the callee at call time
   in its parameter scope, so the function owns it, not the enclosing scope.
   Decorators are owned by the scope enclosing the class. TS/JS class field
   initializers and static blocks: owner absent, region
   `Limited(ownership_unknown)`, lexical binding still applies. Rust has no
   default arguments. A Rust `const`/`static` initializer inside a function
   has no owner, and its `owner_span` is the item, so it never counts as a
   call of the function. File-level code has no owner, and its `owner_span`
   is the whole file.
7. **Anonymous callables:** owner absent, `owner_span` is the callable,
   coverage regions keyed by span, no synthetic IDs.
8. **Reason codes (serialized enum, additive):** `unsupported_language`,
   `unsupported_binding_form`, `missing_call_capture`, `indirect_dispatch`,
   `ownership_unknown`, `parse_error`, `empty_reference_query`, **plus
   `unexpanded_macro`**, added because the spec names unknown macro expansion
   as a coverage limit (Rust `name!(…)`).
9. **Performance:** no measurement is required. If a timing sample is taken
   in Phase 6, it uses one host. Wider measurement (other hosts, large
   repositories, statistics) is listed here for the author and **not run**.

### Gates

Skipped under the phase rule. After all Phase 1 edits I ran, from the
repository root:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `"packages": []`, `"change_class": "documentation"`. The input list
also included `.claude/skills/tree-hugger/SKILL.md` and two
`2026-10-06-symbol-type-relations` files, which other work had already
modified. This phase did not touch them. `just test` and `just lint` were
not run. No source file is listed in `source_files_during_phase_1`.

No test was added, because Phase 1 changed no behavior. The rulings and the
table are inputs for the Phase 2 L1 tests (`symbols_occurrences.rs`). The
throwaway spike in `/tmp/scoped-refs-spike/` is not repository code.

## Phase 2

Phase 2 built the occurrence model, the lexical scope tree and lookup, and
coverage, reachable through the new library method
`TreeFile::reference_evidence()`. Nothing consumers already see changed:
`TreeFile::query`, the cache, the bind pass, diagnostics and the CLI are
untouched, and the two new `FileSymbolIndex` fields are omitted when empty,
so every serialized index is byte-identical (the goldens pass unchanged).

### What was built

- **Types** (`lib/src/shared/schema_v2/occurrences.rs`): `Occurrence`,
  `OccurrenceKind` (`Call`/`Usage`), `Confidence`
  (`Scoped`/`NameOnly`/`Unresolved`), `ReferenceCoverage`, `CoverageState`,
  `LimitReason` (snake_case, the nine codes of ruling 8) and
  `ReferenceEvidence`. `FileSymbolIndex` gained `occurrences` and
  `reference_coverage` (`serde(default, skip_serializing_if = "Vec::is_empty")`),
  and `SymbolRef` gained `PartialEq, Eq`.
- **Capture collection.** `TreeFile::definitions_with_scopes` returns the
  `@local.scope` nodes from the same locals run as the definitions
  (`definitions` itself does not collect them, so declaration reports do no
  extra work). `TreeFile::reference_captures` runs the references query once
  and keeps one capture per node span; `references_from` now filters its
  `@reference` captures, with unchanged order and exclusions.
- **Reference queries.** Rust, TypeScript and JavaScript `references.scm`
  gained `@reference.member` patterns for method-call names (Rust
  `field_identifier`, including turbofish calls) and `call`/`new` member names
  (TS/JS `property_identifier`/`private_property_identifier`). Only
  occurrences read that capture name, so diagnostics and the old bind pass see
  exactly what they saw before. `references.scm` files are Tree Hugger
  originals, so no provenance note applies. **No vendored `locals.scm` was
  edited**: the missing scopes that the scope study found (Rust `source_file`,
  inline `mod` body, `trait_item`, generic items; TS generators, classes,
  signatures, `switch_body`) are language rules in code
  (`LanguageRules::extra_scope`). That keeps the vendored text, its
  provenance and the definition extraction unchanged.
- **Scope tree** (`lib/src/analysis/scope/{mod,rust,ecma}.rs`): the scopes are
  the `@local.scope` nodes, the file root and the extra kinds, sorted and
  deduplicated, each with a parent link. Bindings live in a
  `(scope, name)` table and carry category bits (`TYPE`/`VALUE`/`MACRO`;
  imports carry all three), `visible_from`, `ordered` (Rust `let`: the
  latest visible wins) and `callable`. Lookup climbs parent links, and a
  Rust glob ends the search. No lookup scans every declaration.
- **Binding facts.** One cursor walk over the tree collects binding sites per
  language rule, and those are the truth: Rust items (associated items
  excluded), `let`/parameter/closure/`for`/`match`/`if let` pattern names,
  generics and `use` names; TS/JS declarations, `var` (function scoped),
  parameters and patterns, `for…of`, `catch`, type parameters and import local
  names (alias only). A record the walk did not find binds by its kind,
  unless it is a false capture (see Departures). Bindings that have no record
  (`for`/`catch`/destructured names, generic parameters) still shadow, and
  resolve `Scoped` with no ID.
- **Resolution.** Plain names use lexical lookup. Rust `Self::f`/`Type::f`
  resolves through an inherent impl of one in-file struct or enum (or one of
  its enum variants). Member calls are `NameOnly` over member functions,
  with region reason `indirect_dispatch`. Any other qualified name keeps its
  written path and is uncertain. A `Scoped` call to a binding that is not
  `callable` (a variable or parameter) adds `indirect_dispatch`.
- **Ownership and coverage.** Regions are the whole file (owner absent, span
  computed from the source, so leading trivia is included), every
  function-kind record with a `body`, anonymous callables (per-language kind
  list), Rust `const`/`static` inside a function, and TS/JS field
  initializers and static blocks (`ownership_unknown`). Reasons come from the
  walk: macros, call shapes, `with`/`eval`, and `ERROR`/`MISSING` nodes. Each
  reason goes to the innermost region. Limited languages get
  `unsupported_language` on every region, and an empty references query adds
  `empty_reference_query`.
- **Determinism.** Occurrences are deduplicated by span and sorted by span,
  owner span, kind, written target and confidence. Candidates are sorted by
  name position, then ID, and deduplicated by ID. Coverage is sorted
  outer-first. The order never depends on capture or hash-map order.

### Departures and decisions (spec and plan unchanged)

- **`target.qualified_name` is the written path or receiver expression,
  including the name** (`Self::new`, `other.method`, `std::mem::swap`). It
  is never the declaration's qualified name. This is the spec's "written
  name and qualifier" in the existing `SymbolRef` shape.
- **Fallback candidates for supported languages are `Item`-scope records
  only.** The planned doc diagram said "any same-named declaration". Another
  function's locals can never be visible, and a bare name cannot reach a
  member. Limited languages keep every same-named record as a candidate.
- **False captures are ignored for binding.** The vendored queries capture
  the Rust pattern path `Some` in `Some(x)` (through
  `tuple_struct_pattern (identifier)` and `let_condition (_ (identifier))`)
  and a TS parameter's *default value* `a` in `b = a` (through
  `required_parameter (identifier) @local.definition`) as variables. These
  records still exist (no extraction change in this phase). They neither bind
  nor hide a reference, so `b = a` resolves `a` to the earlier parameter.
  The record oddity is a pre-existing extraction issue, left for the author.
- **Refutable Rust patterns:** a bare capitalized identifier in a `match` arm
  or `if let` binds nothing (it is read as a constant or unit variant).
- **Not occurrences:** the Rust `Self` keyword, lifetimes and loop labels,
  attribute contents, and everything inside `use`/`extern crate`/`import`
  statements (import records cover them). For supported languages the old
  `is_definition_like_reference` filter is not used, because the binding
  sites decide what a definition is (it dropped parameter defaults).
- **TS `const f = () => …`** is `callable`, so calling it is not an indirect
  call. Calling a `let`/`var`/parameter is.
- **Glob and ambiguous bare calls carry no coverage reason**; the occurrence
  is `NameOnly`/`Unresolved`. Phase 3 decides recursion from the
  occurrences (a `NameOnly` call whose candidates include the function's
  own ID is the "potentially recursive" case).

### Requirement → test mapping

L1 `lib/tests/l1/symbols_occurrences.rs` (declared in `tests/l1/main.rs`, no
tier marker, so `just test` runs it), plus one crate unit test:

| Requirement | Test |
|---|---|
| Rust `let f = f();` reads the outer `f`; inner block `let` order | `rust_let_initializer_reads_the_outer_binding_and_later_uses_the_let` |
| TS/JS block `let` hides the outer name before its declaration (TDZ) | `ecma_block_let_hides_the_outer_name_before_its_declaration` |
| `var` hoisting to the function | `ecma_var_is_function_scoped_and_hoisted` |
| Python local-by-assignment is `Limited`, never a wrong binding | `python_local_by_assignment_is_limited_never_a_wrong_binding` |
| Uncaptured bindings shadow | `hidden_bindings_block_outer_matches_even_without_a_record` |
| Type/value categories | `type_and_value_names_are_separate_categories` |
| Same-named methods on two types stay `NameOnly`; `Type::f` path resolves | `same_named_methods_on_two_types_stay_name_only` |
| `other.method()`/`self.method()` never bind to the caller | `a_receiver_call_never_binds_to_the_caller` |
| Unrelated/dynamic receivers uncertain | `ecma_receivers_and_dynamic_access_stay_uncertain` |
| Overload ambiguity has no target ID | `overloads_never_get_a_target_id` |
| Written qualifier kept on unresolved targets | `unresolved_qualified_targets_keep_their_written_qualifier` |
| Spans in non-ASCII source; repeated calls on one line | `spans_cover_the_name_token_in_non_ascii_source` |
| Overlapping captures dedupe | `overlapping_captures_make_one_occurrence` |
| Constructors are calls | `constructors_are_calls` |
| File-level code and initializers | `file_level_code_and_initializers_keep_their_occurrences` |
| Anonymous callables have distinct `owner_span` | `anonymous_callables_are_distinct_owner_absent_regions` |
| Nested named functions | `nested_named_functions_own_their_calls` |
| Named bodies have coverage without occurrences | `named_bodies_have_coverage_even_without_occurrences` |
| Declaration names, comments and strings excluded | `declaration_names_comments_and_strings_are_not_occurrences` |
| Macros are usages that limit coverage | `macros_are_usages_that_limit_their_region` |
| Parse errors limit their region | `parse_errors_limit_the_region_that_holds_them` |
| Ruling 6: defaults owned by the function, decorators by the enclosing scope | `defaults_belong_to_the_function_and_decorators_to_the_enclosing_scope` |
| `with`/`eval` limit coverage | `ecma_with_and_eval_limit_their_region` |
| Alias import: `Scoped`, no ID, import record keeps source and original | `alias_imports_bind_the_alias_locally_without_an_id` (TS and Rust) |
| All 16 languages: explicit state, never empty-and-passing | `every_language_reports_occurrences_with_an_explicit_state` |
| IDs are the complete index's record IDs | `owner_target_and_candidate_ids_are_the_complete_index_ids` |
| Canonical order, distinct candidates, reasons iff `Limited` | `occurrences_and_coverage_are_in_canonical_order` |
| Shuffled capture order gives identical output | `analysis::occurrences::tests::capture_order_never_changes_the_evidence` (lib unit test: reversed, rotated and repeated scope and reference captures, for Rust, TS and Python) |

The Input Robustness Matrix does not apply to this phase. No file format or
configuration reader changed; the cache reader changes in Phase 4. Old summary
behavior is covered by the existing suite, which passes unchanged
(`symbols_query`, `symbols_cache`, `lint_diagnostics`, goldens).

### Gates

- Scope calculator (repository root, after all edits):
  `"packages": ["tree-hugger", "test-toolkit"]`, `"change_class": "package"`,
  so both gates ran.
- `just lint` (clippy `--all-targets -D warnings`, both crates): passes.
- Final `just test --no-fail-fast` (after every edit): 1168 of 1169
  passed. The one red test is the timeout
  `symbols_relations::generic_boundary::cpp::overflows`. An earlier run
  (1139 of 1141) had the 2 red tests
  `symbols_relations::generic_boundary::csharp` (fail) and `::cpp`
  (30 s timeout). An earlier run named the same test
  `generic_shadowing_holds_on_both_sides_of_the_u16_position_boundary`.
  **Neither is from this phase.** Another session was editing this
  worktree concurrently, for the `2026-10-06-symbol-type-relations` review-3
  fixes. Its uncommitted edits are in `lib/tests/l1/symbols_relations.rs`
  (+281 lines, the new tests), `lib/src/file/extract/{generics,relations}.rs`,
  `lib/src/file/tree_file/relations.rs` and that feature's spec, log and
  review files. This phase did not touch any of those files. All 27
  `symbols_occurrences` tests and the unit test pass.
- Not run: `just test-l2` (no terminal output changed in this phase) and
  cross-OS checks. The new code reads no paths or filesystem state: spans are
  byte offsets, CRLF only shifts columns, and the tests use `tempfile`.
  The `os` skill's Windows path traps do not apply. CI covers Linux and
  macOS on the pull request.

## Phase 3

Phase 3 wired the occurrence core into the one analysis path: the reference
summaries and `semantics.is_recursive` are now read from the occurrences, and
`Capability::Occurrences` keeps the evidence in a result.

### What was built

- **`Capability::Occurrences`** (`lib/src/file/query.rs`), between
  `References` and `Semantics` in canonical order. `Occurrences` requires
  `References`; `Semantics` requires `Occurrences`. `complete_index()` (so
  `--json-full`) includes it through `Capability::ALL`.
- **One path** (`analysis::run_query`). With `References` the locals query
  runs once through `definitions_with_scopes`, the references query runs
  once (`reference_captures`, shared with diagnostics through the new
  `TreeFile::references_from_captures`), `OccurrenceSyntax` is built before
  `records_from` consumes the definitions, and `resolve(&index.symbols)` runs
  after containment and before unselected records are dropped. Without
  `Occurrences` the evidence is cleared after the edges and recursion are
  derived. `TreeFile::reference_evidence()` is now an `Occurrences` query
  that selects no kinds (the separate Phase 2 sequence is gone).
- **Summaries** (`bind_references`). Only `Scoped` occurrences with an owner
  make edges: an in-file target ID gives `references` plus the target's
  `referenced_by`; an occurrence bound to an import gives `references` plus
  `dependencies`. The import is identified by `ResolvedOccurrences::bound_imports`,
  aligned with the sorted occurrences: the resolver maps the import binding's
  name start to the import record (the record whose name node it is, else the
  same-named import whose statement holds it). `RefSet` dedupe and
  first-occurrence order are kept. Export binding and containment are
  unchanged. `OwnerSymbolIndex` (the old smallest-enclosing-record owner
  search) and its unit test were removed as dead code.
- **Recursion** (`semantic_pass`, `direct_recursion`, `may_call_itself`). It
  reads only `index.occurrences`, `reference_coverage` and `imports`, so the
  cache can widen a stored `Occurrences` entry by `Semantics` without a parse
  and get the uncached result. `Yes`: a `Scoped` call to the record's own ID
  (wins over any uncertainty). `Unknown`: no owned region (bodyless), a
  `Limited` region, or a call that might be to itself: `NameOnly` with the
  record among the candidates, `Unresolved` with its name, `Scoped` through an
  import whose original last segment is its name, or, for a constructor, a
  call of its own class. `No`: otherwise. Constructors are now evaluated
  (they were skipped before); `stamp_parse_pass` follows.
- **Projection and widening** (`analysis/coverage.rs`). Without
  `Occurrences`, `project` clears both lists. `record_passes_suffice`:
  `Semantics` needs stored `Occurrences` (not `References`); `Occurrences`
  needs a parse.
- **Counters** (`analysis/counters.rs`): `reference_captures` (references
  query runs) and `occurrence_resolutions`.

### Departures and decisions

- **Cache storage pulled forward from Phase 4 (minimal).** Because
  `Semantics` now implies `Occurrences`, an entry claiming `semantics` would
  have been served with empty evidence. The entry therefore stores
  `occurrences` and `reference_coverage` now: both keys are required (an old
  entry without them is a `Malformed` miss and is recomputed), they must be
  empty without the `occurrences` capability, and the coverage must be
  non-empty with it (`Inconsistent` otherwise). Phase 4 still owns the schema
  5.0 bump, the extractor revision bump, the strict per-field validation and
  the Input Robustness Matrix (`cache_robustness` columns for the two keys).
- **Summaries changed meaning, as the spec says.** (1) The 13 languages
  without resolver rules now have **no** `references`/`dependencies`/
  `referenced_by` edges (before: first-same-name edges). (2) Owners are
  callables only, so a struct, field or constant no longer collects
  references (e.g. a field's type), and code in a closure or at file level
  makes no edge for the enclosing function. (3) A parameter's type is an
  occurrence the function owns, so `fn go(x: A)` references `A`. The old
  Python test `distinct_unresolved_references_are_all_kept` was rewritten in
  TypeScript (`distinct_import_references_are_all_kept`); the Python case
  now asserts no edges and kept occurrences.
- **Import alias of the function itself** (Phase 2's warning): decided by
  name, from public data (import local name equals the call's written name
  and the import's original last segment equals the callable's name), so
  fresh and cache-widened results agree. `use std::fmt::format` inside a
  function `format` is therefore `Unknown`; a conservative false `Unknown`,
  never a false `No`.
- **`missing_call_capture` is still never emitted.** A call whose target
  no capture names (`(pick())()`) is `indirect_dispatch` (Phase 2), which
  gives the same `Unknown`. The matrix row uses that form.
- **`may_panic` is unchanged code** but reads the new `references`; a
  Rust `panic!` was never an in-file record or import, so it stays `Unknown`
  as before.
- **`--json-full` now includes `occurrences` and `reference_coverage`** in
  each index (through `complete_index()`), with in-file names.
  `REPORT_SCHEMA_VERSION` stays 1 (ruling 4). Goldens are `--json`/plain and
  did not change.
- The legacy v1 adapter has no recursion field; `FunctionData.is_recursive`
  stays the constant `false`. Nothing changed there.

### Requirement → test mapping

New L1 file `lib/tests/l1/symbols_reference_summaries.rs` (declared in
`tests/l1/main.rs`, no tier marker, so `just test` runs it):

| Requirement | Test |
|---|---|
| Recursion: direct self-call `Yes`; self-value `No`; parameter/variable call `Unknown`; proven self-call wins; receiver call, unresolved same name, computed call, macro, parse error `Unknown`; closure and nested function cannot prove the enclosing one; mutual recursion `No`; bodyless `Unknown`; import alias of itself `Unknown` (control: alias of another name `No`); TS TDZ shadow `Unknown`; `this.m()` `Unknown`; constructor of its own class `Unknown` (control `No`); JS `Yes`; Python/Go `Unknown`; each on the complete index **and** a functions-only query | `recursion_matrix_holds_on_complete_and_narrowed_queries` (`RECURSION_MATRIX`, 25 rows) |
| `None` without semantics (`References`, `Occurrences`), only callables get it | `is_recursive_is_absent_without_semantics_and_only_on_callables` |
| Edges only for established targets, first-occurrence order, dedupe; shadowed local, receiver call, file-level initializer and closure make no edge; uncertain occurrences kept | `summaries_hold_only_established_targets_in_first_occurrence_order` |
| Bound (aliased) imports are dependencies of their owners only | `bound_imports_are_dependencies_of_their_owners_only` |
| Limited languages: no edges, occurrences kept, none `Scoped` | `limited_languages_bind_no_edges_but_keep_their_occurrences` |
| Whole-file evidence under a kind-narrowed query (16 primary fixtures); narrowed records equal the same-capability wide query; `References` alone keeps edges and drops evidence; declaration/containment queries do no reference work; one references run and one resolution per analysis | `occurrences_describe_the_whole_file_whatever_kinds_are_selected` |
| Diagnostics unchanged by the shared references run (every fixture) | `diagnostics_are_unchanged_by_the_shared_references_run` |
| Usages feed edges, only calls decide recursion | `usages_feed_summaries_but_not_recursion` |

Updated tests:

- `symbols_query::passes_run_once_after_their_prerequisites`: capability
  closure with `Occurrences`; `reference_captures` and
  `occurrence_resolutions` counted per pass plan.
- `symbols_query::distinct_import_references_are_all_kept` (was the Python
  `distinct_unresolved_references_are_all_kept`); the old
  `is_recursive_is_unknown_when_semantics_run_and_absent_otherwise` was
  replaced by the matrix.
- `symbols_cache::widening_capabilities_runs_only_the_missing_passes`:
  `References` → `Semantics` now parses; new rows `Occurrences` →
  `Semantics` (no parse) and `References` → `Occurrences` (parse), each
  followed by real `CacheLookup::Hit`s equal to uncached results.
- `symbols_cache::assert_projections_equal_uncached_queries`: new cells
  `Occurrences` and methods-only `Semantics`; asserts some projection
  stripped occurrences.
- CLI `cache_robustness::the_cache_entry_records_the_coverage_it_holds`:
  the entry lists `occurrences` and stores both lists; `--json-full` cold and
  warm runs report the stored coverage (end-to-end through `hug`).

The Input Robustness Matrix for the new cache keys is Phase 4's (plan
table); this phase added only the presence and capability-consistency
checks described above.

### Gates

- `just test` (tree-hugger + tree-hugger-cli): 1185 passed, 0 failed.
- `just lint`: passes.
- Not run: `just test-l2` (no terminal rendering changed) and cross-OS
  checks. No path, filesystem or OS-specific code changed; spans are byte
  offsets.
