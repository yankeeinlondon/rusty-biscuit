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
source_files_during_phase_4:
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_4:
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/references.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/terminal_scope.rs
  - tree-hugger/cli/tests/support/terminal_report.rs
  - tree-hugger/cli/tests/level2_declarations.rs
docs_updated_during_phase_5:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_6:
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/cli/src/render.rs
docs_updated_during_phase_6:
  - tree-hugger/docs/references.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/query-system.md
source_code:
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
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/analysis/coverage.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/tests/l1/symbols_reference_summaries.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/symbols/terminal_scope.rs
  - tree-hugger/cli/tests/support/terminal_report.rs
  - tree-hugger/cli/tests/level2_declarations.rs
documentation:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
completed_phase: 6
implemented: true
implementation_7: "2026-10-07T11:55:32-07:00"
deferred_perf_measurement: false
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

## Phase 4

Phase 4 moved the record schema to 5.0, bumped the extractor revision, and
made the cache read the reference evidence strictly: a damaged or
contradictory entry is now a miss and gets recomputed, never served.

### What was built

- **Versions.** `SchemaVersion::V5_0` is now `CURRENT`, and 4.3 is documented
  as "no longer emitted". `EXTRACTOR_REVISION` went from 30 to 31 (ruling 3:
  bumped from the value in the tree, not "10" → "11"). `REPORT_SCHEMA_VERSION`
  stays 1 (ruling 4). The goldens `symbols.json` and `symbols.jsonl` were
  regenerated with `HUG_UPDATE_GOLDEN=1`. Their diff is only
  `schema_version` (4.3 → 5.0, 190 sites) and `extractor_version` (rev.30 →
  rev.31, 95 sites). Every record ID is unchanged, so declaration IDs stay
  stable.
- **Strict evidence form** (new `lib/src/cache/stored_evidence.rs`). The
  entry writes `occurrences` and `reference_coverage` through
  `StoredOccurrence`/`StoredCoverage`/`StoredRef`/`StoredSpan`/`StoredPoint`.
  Every key is written (`null` owner or ID, `[]` candidates or reasons), and
  the reader types are `deny_unknown_fields` and require every key (a
  `nullable` deserializer, so an absent key is an error and never `None`).
  The public `Occurrence`/`ReferenceCoverage` serde defaults are never on
  the cache path. `symbol_cache.rs` uses these through
  `serialize_with`/`deserialize_with`, and its `nullable_string` became the
  generic `stored_evidence::nullable`.
- **`validate_evidence`** (called from `StoredEntry::validate`, error →
  `CacheMiss::Inconsistent`). It absorbs Phase 3's two capability checks and
  adds:
  - the first region is the whole file with no owner (`whole_file_span`,
    now `pub(crate)`);
  - every span is checked against the source: in range, on character
    boundaries, start ≤ end, and its points equal the one-based line and
    byte column of its offsets (`LineStarts`). An occurrence span is
    non-empty and lies inside its owner span;
  - `target.name` is the source text at the span;
  - `(owner ID, owner span)` is a coverage region, the occurrence's owner
    equals that region's owner, and an owner or candidate that names a
    stored record repeats its name and qualified name;
  - confidence consistency: `Scoped` has no candidates; `NameOnly` has no
    target ID and has distinct candidates, each with an ID; `Unresolved`
    has neither;
  - occurrences are strictly in canonical order (`occurrence_order`, now
    `pub(crate)`) with distinct spans. Coverage is in outer-first order.
    Reasons are ascending and distinct, and empty exactly when the region
    is `Complete`;
  - cross-check against the records: every in-file `references` edge (a
    reference that is not a `dependencies` import) and every
    `referenced_by` edge has the `Scoped` occurrence that made it, and
    `is_recursive: Yes` has a `Scoped` self-`Call`.
- **Projection** needed no change. Phase 3 already did what the plan task
  asks: `record_passes_suffice` widens `Semantics` without a parse only from
  stored `Occurrences`, and `project` clears the evidence without the
  capability. The existing `symbols_cache` projection and widening rows
  pass.

### Departures and decisions

- **The `[]`-occurrences cell.** The plan's matrix says `[]` is accepted
  only when the file truly has none. Without a parse, the reader can tell
  that only through derived facts: an emptied list is refused when any record
  keeps an edge an occurrence made (or a recursion `Yes`). An occurrence
  removed while nothing derived from it remains (a call with no in-file
  target) cannot be detected without parsing. That residual is documented in
  `docs/cache-design.md#reference-evidence`. The entry key includes the
  source hash, so an entry is only ever read for its own text.
- **`target.name` vs records is not checked.** A target's name is the
  written token, which can differ from the declaration's name (a TS
  `new Box()` can bind a constructor record). Owners and candidates are
  copies of `symbol_ref(record)`, so those are checked.
- **Test helpers keep forged entries consistent.**
  `cache_upgrade::rename_record` (the sentinel used by every warm-hit test)
  now renames the evidence owners and candidates that name the record. A
  new `set_qualified_name` does the same for the revision 6 and 20 rows,
  which forge a qualified name. Without this, the stricter reader correctly
  refused their control payloads.
- `assert_schema_bump_entry_is_not_served` now takes a `SchemaBump` row
  (what the old shape lacked, and how to recognize the current shape), so
  the revision-30 row can say "no `occurrences`/`reference_coverage` keys".
- Hard-coded "future" versions in tests (`5.0` in two incompatible-version
  lists, and `{5,1}` as the robustness matrix's incompatible minor) were
  moved: the lists now use 4.3, 5.1 and 6.0.
- **Concurrent edits in this worktree.** While this phase ran, another
  session edited the same worktree for the `2026-10-06-symbol-type-relations`
  review: `lib/src/file/extract/relations.rs`,
  `lib/src/file/tree_file/relations.rs`, `lib/tests/l1/symbols_relations.rs`,
  `cli/tests/symbols/outline.rs`, `docs/type-relations.md`,
  `docs/symbol-kinds-and-visibility.md`, `docs/symbol-reports.md`, that
  feature's `log.md`, and the area/lib/cli READMEs. It also made some of
  the same 4.3 → 5.0 text substitutions in `docs/cache-design.md`,
  `docs/symbol-schema-design.md`, the READMEs and the skill before my doc
  pass. **Those files are not this phase's work** (except where listed in
  the frontmatter). Whoever commits this phase must separate them.

### Input Robustness Matrix (cache entry, reference evidence)

One test walks the matrix from a real `hug --json-full` entry for a
TypeScript fixture. The fixture has a `Scoped` call with an edge, a
`NameOnly` member call, an `Unresolved` call, a `Limited` region, direct
recursion and a non-ASCII character. Each cell makes one edit, and the
outcome is asserted through `hug`'s output (served = the sentinel is
present; rejected = output equals an uncached run):
`cache_robustness::reference_evidence_is_served_only_when_consistent`.

| Shape | `occurrences` | `reference_coverage` | `confidence` | `target.id` | `candidates` | `span` | `owner` | coverage `reasons` |
|---|---|---|---|---|---|---|---|---|
| control | served | served | served | served | served | served | served | served |
| absent | miss | miss | miss | miss | miss | miss | miss | miss |
| `null` | miss | miss | miss | miss (the edge has no occurrence) | miss | miss | miss (no such region) | miss |
| wrong type | miss | miss | miss | miss | miss | miss | miss | miss |
| one bad element | miss | miss | miss | miss | miss | miss | miss (qualified name ≠ region owner's) | miss |
| every element bad | miss | miss | miss | miss | miss | miss | miss | miss |
| empty | miss (edge without occurrence); **served** for a quiet file | miss | miss | miss | miss (`NameOnly` needs candidates) | miss | miss | miss (`Limited` needs a reason) |
| duplicate key | miss | miss | miss | miss | miss | miss | miss | miss |
| trailing content | miss | miss | miss | miss | miss | miss | miss | miss |

Plus 24 consistency edits, each of which must be a miss: zero-length, inverted,
past-the-end and off-boundary spans; points that disagree with offsets; a
target name that is not the source text; `Scoped` with candidates;
`NameOnly` with an ID or a repeated candidate; `Unresolved` with an ID or
candidates; a repeated occurrence; reordered occurrences; an owner from
another region; a region owner or candidate whose names differ from the
record's; a `Complete` region with a reason; repeated reasons; no file
region; reordered coverage; evidence without the capability; a removed
edge-making occurrence; a forged `referenced_by`; and a forged
`is_recursive: Yes`. The capability claim column (`completed_capabilities`)
is the existing Python-matrix entry column. Code-smell grep on the cache
read path: no `#[serde(default)]`, `unwrap_or_default`, `.ok()` or parsing
`filter_map` in `stored_evidence.rs` (its one `filter_map` reads IDs off
already-parsed relation refs).

The first matrix run found a real gap. An edited owner `qualified_name` was
served. That led to the owner-equals-region-owner and names-its-record checks.

### Requirement → test mapping

| Requirement | Test |
|---|---|
| Schema 5.0 on records, envelopes, entries | `wiring` (TypeScript schema 5.0), `report.rs`/`reports.rs` envelope prefixes, `tree_file::emits_symbol_records_v2`/`emits_symbol_index_v2_with_completed_passes`, goldens |
| Declaration IDs unchanged | golden diff (version fields only) + `golden::json_symbols_over_multi_match_the_golden` |
| Old version invalidated (revision 30, schema 4.3, no evidence keys) under both keys; rewritten entry current | `cache_upgrade::entry_from_extractor_revision_thirty_is_not_served_after_the_scoped_references_bump` |
| 4.3 entry/record versions are misses | `symbols_cache::incompatible_record_schema_versions_are_misses_never_served_or_widened` (4.3 added), `each_key_input_and_stored_identity_decides_a_miss` (entry 4.3 added), `cache_upgrade::warm_entry_from_the_previous_extractor_…` |
| Malformed or inconsistent evidence is a miss, never an empty result | `cache_robustness::reference_evidence_is_served_only_when_consistent` |
| Genuinely empty evidence is served | the same test (quiet file) |
| Persisted round trip (write → read → rewrite → read), every fixture | `symbols_cache::stored_reference_evidence_round_trips_through_rewrites` (new) |
| Passive corpus: every shipped fixture's evidence passes the strict reader (`Hit`, no work) | `symbols_cache::every_fixture_is_a_complete_warm_hit_equal_to_an_uncached_run` |
| Cold/warm/widened/narrowed equal uncached; real `Hit`; counters | existing `symbols_cache` projection/widening tests (Phase 3 `Occurrences` rows), `cache_robustness::the_cache_entry_records_the_coverage_it_holds` (now also asserts the written `null` owner) |
| Span points vs offsets (CRLF, non-ASCII, end of source) | `cache::stored_evidence::tests::points_are_one_based_lines_and_byte_columns` |

### Gates

- Scope calculator (repository root, after all edits): `"packages":
  ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`, `"change_class":
  "package"`, so both gates ran. The input list includes the concurrent
  session's files.
- `just test` (tree-hugger + tree-hugger-cli): **1193 of 1193 passed**.
  An earlier full run (under load from the concurrent session's builds) had
  1192 pass and 1 time out. My output filter did not capture that test's
  name, and the next full run was all green.
- `just lint`: passes.
- Not run: `just test-l2`, because no terminal rendering changed, and
  cross-OS checks. The new code reads no paths. Spans are validated against
  the source text with byte arithmetic, `\r` counts as a column exactly as
  tree-sitter does (unit-tested), and the tests use `tempfile` and `Scratch`.

## Phase 5

Phase 5 added `hug refs`, which reports one declaration's references (or its
callers or callees) from the reference evidence, in text and `--json`.

### What was built

- **Command** (`cli/src/refs.rs`, new; wired in `cli/src/main.rs` through
  `run_refs`, handled before the declaration commands' output-mode
  resolution). `hug refs [<symbol> | --id <id>] [file-globs...]`: the first
  positional is the name, the rest are file filters; `--id` makes every
  positional a filter. With neither, a clap usage error (exit 2).
  `--callers` conflicts with `--callees` (clap). `--json-full` is rejected in
  `run_refs`; `--jsonl`, `--format`, `--count`, `--stats` and every
  declaration selector are rejected by clap because `RefsArgs` does not carry
  `ReportArgs`/`SelectionArgs`.
- **Shared discovery flags.** `--exclude-files`, `--language` and `--no-cache`
  moved from `ScanOptions` into a new `DiscoveryArgs`, flattened into
  `ScanOptions` and into `RefsArgs`, so `refs` reuses them without accepting
  the listing-only flags (`--exclude-symbols`, `--comments`, `--group-by-*`).
  The persistent-cache constructor became `main::symbol_cache`, shared by both
  paths.
- **Selection.** Every file is analyzed with `SymbolQuery::new()
  .capabilities([Occurrences]).scan_root(..)`: every kind and every scope, so
  locals and parameters are selectable. A name is `NamePattern::exact`,
  matched through the library's `SymbolQuery::matches` (smart case,
  language separators, `file#` prefixes, trailing `!`), and `--id` compares
  `record.id` exactly. Matches are collected across the whole scan before
  anything is written: more than one → stderr listing (location, label,
  qualified name, ID), exit 1, empty stdout; none → empty report, exit 0.
- **Report model.** Resolution is file-local, so only the matching file's
  index is kept. Incoming (default, `--callers` = `Call` only): `Scoped`
  occurrences whose `target.id` is the declaration's ID, plus a separate
  `name_only_suggestions` group of `NameOnly` occurrences whose candidates
  include it. `Unresolved` same-name occurrences are never attached. Callees:
  every `Call` whose `owner.id` is the declaration's ID, whatever its
  confidence, each with an `absent_id` explanation when the target has no
  ID (`import`, `binding_without_record`, `name_only`, `unresolved`).
  Owner and candidate `qualified_name`s are replaced by the records'
  displayed (prefixed) names for presentation only; nothing stored changes.
- **JSON envelope** (`report_schema_version` 1, ruling 4): `schema_version`,
  `root_dir`, `boundary`, `selector`, `direction`, `declaration` (trimmed like
  `--json` records, `null` when nothing matched), `files` (the declaration's
  file: `file`, `language`, `completed_capabilities`, `occurrences`,
  `name_only_suggestions`, `coverage`, `limited_regions`), then `errors`.
- **Text** through the declaration renderer (`Renderer::{line, list, link,
  declaration}` and `render::label` became `pub(crate)`): heading with the
  declaration and its linked location, the boundary on every report, groups
  by owning region (owner name, `file-level code [span]` or `code without a
  declaration owner [span]`), one bullet per call site with a linked dim
  location, and a note counting `Limited` regions.

### Departures and decisions

- **How `import` is told apart from other `Scoped` calls without an ID.** The
  occurrence does not store which import bound it, so the CLI reads the
  owner record's `relations.dependencies`. That list is built from exactly
  the bound imports (`bind_references`), keyed by the import's local name,
  which is the written target name. So `import` is decided from the
  library's own evidence, not guessed; a `Scoped` call with no ID that is
  not listed there is `binding_without_record` (a TS `for…of`/`catch`
  binding, for instance). No schema change was needed.
- **Coverage in JSON** is the regions owning a listed row (plus the
  declaration's own region for `--callees`), not every region of the file;
  `limited_regions` counts the file's `Limited` regions for incoming views.
- **Exit status for ambiguity is 1**, like a failed file; clap usage errors
  stay 2.
- **Shared L2 helper fix.** `terminal_report::Report::has_logical_row` joined
  the padded last piece of a WezTerm soft-wrapped row onto the next row (the
  capture pads that piece to the pane width), so an 81-column `refs` heading
  could never match. It now measures rows without trailing spaces, as its doc
  already implied ("the space the capture trimmed"). The regression test
  `terminal_scope::a_padded_last_piece_ends_its_logical_row` fails against
  the old helper.
- An all-lowercase qualified name is case-insensitive under smart case, so
  `names.ts#widget` matches both `widget` and `Widget`. That is the existing
  rule, and the test uses `--id` from the ambiguity listing instead.

### Requirement → test mapping

All L1 tests are in `cli/tests/symbols/refs.rs` (the `symbols` binary, L1):

| Requirement | Test |
|---|---|
| Incoming rows: every `Scoped` call/usage bound to the ID, call sites kept, owners prefixed, coverage for each row | `references_list_every_established_call_and_usage_in_the_declarations_file` |
| `--callers` excludes `Usage`, keeps repeated calls | `callers_exclude_usages_and_keep_repeated_calls` |
| `--callees`: owned calls only, all confidences, absent IDs explained (TS bindings, Rust import, unresolved), own region in coverage | `callees_list_owned_calls_and_explain_absent_ids` |
| Uncertain group separate; `--id` cannot upgrade; non-candidate not suggested | `name_only_suggestions_stay_separate_and_an_id_cannot_upgrade_them` |
| Duplicate names across files: exit 1, no stdout (JSON and text), locations + IDs; file prefix, `--id`, `--exclude-files` select one | `a_name_declared_in_two_files_is_an_error_listing_both` |
| Exact smart-case names, case-sensitive IDs, local targets | `names_are_exact_smart_case_and_ids_case_sensitive` |
| Empty reports (unknown name, unknown ID, unreferenced declaration) succeed and state the boundary | `empty_reports_succeed_and_state_the_boundary` |
| Cross-file same-name call never resolved | `a_cross_file_same_name_call_is_never_a_resolved_reference` |
| Versioned envelope, key set, `errors` last, trimmed record, failed file → `errors` + exit 1 | `the_json_envelope_is_versioned_and_ends_with_errors` |
| Text grouping, ownerless labels, suggestion group, callee explanations | `the_text_report_groups_rows_by_owner_and_labels_ownerless_code` |
| Cache options reused; warm run is a real `Hit` with no parse and identical output | `a_warm_run_hits_the_cache_with_the_same_report` |
| Missing selector, `--callers`+`--callees`, `--json-full`, `--jsonl`, `--format`, `--count`, `--kind`, `--exclude-symbols` rejected (exit 2, no stdout) | `missing_selectors_conflicts_and_unsupported_formats_are_rejected` |
| Real-terminal styling, OSC 8 links, boundary on empty report, `hug`'s wrapping at `COLUMNS=60` (L2, tmux/WezTerm/Kitty) | `level2_declarations::check_refs` |
| Padded WezTerm wrap piece ends its logical row | `terminal_scope::a_padded_last_piece_ends_its_logical_row` |

No parser, schema or configuration reader changed in this phase (the CLI
reads evidence that Phase 4's strict cache reader already validates), so no
input-robustness matrix applies.

### Gates

- `just test`: **1206 of 1206 passed** (Phase 4: 1193, plus 12 `refs` tests
  and 1 `terminal_scope` test).
- `just test-l2`: **6 of 6 passed**. tmux and WezTerm ran every check,
  `check_refs` included. Kitty is not installed on this host, so its entry
  points skipped cleanly (0.01s). The first run failed on the
  `has_logical_row` defect above.
- `just lint`: passes.
- Cross-OS: `just cross-check tree-hugger-cli --os windows refs:: terminal_scope::`
  on native Windows: 22 of 22 passed. A filterset with parentheses
  (`-E 'test(/…/)'`) fails in the recipe's remote shell quoting; plain
  substring filters work. Linux and WSL were not run: the new code reuses
  the existing portable-path helpers (`portable_relative`, `display_path`,
  `file_url`) and adds no filesystem or platform code.
- Scope calculator (repository root, after every Phase 5 edit, frontmatter
  included): `git status --porcelain --untracked-files=all | awk '{print $NF}'
  | xargs python3 scripts/ci/affected_scope.py` gave `"packages":
  ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`, `"change_class":
  "package"`, so both gates were required. The working tree still holds the
  uncommitted Phase 4 changes, so the input is wider than this phase.
- Final run on the finished tree (after the last `refs.rs` cleanup):
  `just lint` passes; `just test` 1206 of 1206 passed.

## Phase 6

Phase 6 finished the documentation, the skill, the comment drift pass and
the cross-OS review. Its only source changes are five doc comments; there is
no behavior change.

### What was done

- **Docs.** `docs/references.md` gained "Upgrading from schema 4.x": what
  5.0 changes in meaning (not layout), a before/after table for
  `references`, `referenced_by` and `is_recursive`, and how the cache
  recomputes old entries. Every row was checked against `hug symbols
  --json-full` on the example before it was written. The first draft said
  4.x reported `is_recursive: Yes` by name and that `run.references` became
  `[]`. Both were wrong: 4.x always said `Unknown`, and the edge now points at
  the local `let parse` record. The draft was corrected before it was kept.
  The area `README.md` lists `refs` and has a new release-notes section,
  "scope-aware references and `hug refs`". `lib/README.md` shows
  `reference_evidence()` and has a schema-records bullet for the summaries
  and `Capability::Occurrences`. Already current and left alone:
  `cli/README.md`, `docs/symbol-reports.md`, `docs/cache-design.md` and
  `docs/symbol-schema-design.md` (all updated in Phases 2–5), plus
  `docs/symbol-kinds-and-visibility.md` (only its schema number is
  involved, and it already says 5.0). `docs/symbol-schema.md` is a general
  research note, not a contract, so it was not touched. `references.md`
  already had the recursion Mermaid diagram, the language table and the
  boundary section, and it has no **planned** markers. No `docs/` page names
  this feature.
- **Concurrent commit.** While this phase ran, another session committed the
  `references.md` upgrade section as `6e4ad0334` ("docs(tree-hugger): add
  schema 5.0 upgrade notes to references"). The committed text is this
  phase's final, corrected version. That file is therefore no longer in the
  uncommitted tree, but it is listed below as a Phase 6 doc.
- **Skill.**
  - `SKILL.md`: removed the duplicated `refs` table row and the duplicated
    `hug refs` architecture bullet (left over from Phase 5); added a
    Detailed Documentation link to `docs/references.md`.
  - `api-reference.md`: added `reference_evidence()` and the
    `Occurrence`/`ReferenceCoverage`/`ReferenceEvidence` types.
  - `query-system.md`: fixed drift (see below).
- **Drift pass** (code is correct; the comments were fixed):
  1. `cache/symbol_cache.rs` `CacheLookup::WidenedCapabilities` claimed
     semantics always widens without a parse. It does so only from stored
     occurrences, and containment never does for Rust or Swift.
  2. `cache/mod.rs` `CacheUnit::References`: "References are not yet stored
     in FileSymbolIndex" is now false.
  3. `analysis/scope/mod.rs` `ScopeTree::table` said "in name position
     order". Bindings are stored in binding order, and `visible_in` picks
     the latest `ordered` binding by position.
  4. `file/query.rs` `SymbolQueryResult::index` did not mention
     occurrences or coverage.
  5. `cli/src/render.rs` module doc did not name `refs` as a user.
  6. Skill `query-system.md` called the bind pass "name-based" and said a
     `@reference.member` capture "never changes the bind pass". The bind pass
     now reads occurrences, so it can.

  Checked and accurate: `bind_references`, `semantic_pass`, the
  `direct_recursion`/`may_call_itself` docs, `Capability` docs and order,
  `SchemaVersion`, `stored_evidence`, the scope rules, `refs.rs`,
  `RefsArgs`/`DiscoveryArgs`, and the three `references.scm` headers.
- **Pre-existing limit found, not fixed.** `semantic_pass` sets
  `may_panic: Yes` only when `relations.references` names `panic`/`panic!`.
  That list holds only `Scoped` in-file targets and bound imports, and
  `panic!` is `Unresolved`, so in practice it is never `Yes`. `main`'s
  first-match `bind_references` also pushed only in-file targets and
  imports, so this feature did not cause it. It is out of scope; a follow-up
  could read the occurrences instead.
- **OS review.** No new path or prefix code was added in this phase. Phase 5's
  `refs` reuses `portable_relative`/`display_path`/`file_url`, which were
  already proven on native Windows (Phase 5: 22/22 `refs::` and
  `terminal_scope::` on `build-win-native`). Linux had not been run for this
  feature, so this phase ran it:
  - `just cross-check tree-hugger --os linux symbols_occurrences
    symbols_reference_summaries stored_evidence`: 36/36 passed.
  - `just cross-check tree-hugger-cli --os linux refs::`: 12/12 passed.

  WSL was not run: there is no platform-specific code, and the nightly runs
  WSL2. No CI environments or gates were added.
- **Timing note** (one host, macOS dev machine, one sample each, release
  build): `hug refs run_query 'lib/src/**' --json` over `tree-hugger/lib/src`
  (103 files, 39,051 lines) took 0.36 s with `--no-cache`, then 0.04 s and
  0.03 s warm. The debug build took 1.86 s cold. The author should take a
  wider measurement (several hosts, a monorepo-sized tree) per ruling 9.

### Departures and decisions

- No spec departure in this phase. Spec `status` is set to `implemented`.
  The feature directory was not moved and `just complete` was not run.
- Feature-wide departures and rulings are recorded in the Phase 1–5
  sections above and are not repeated here.

### Requirement → test mapping

This phase changed no behavior: five comments, Markdown docs and skill
files. No test was added. The docs' worked examples were verified by running
the real binary (`hug symbols --json-full`, `hug refs`), not by a test. No
parser, schema or configuration reader changed, so the input-robustness
matrix does not apply.

### Gates

- Scope calculator (repository root, after every edit including
  frontmatter): see the final entry below.
- `just lint`: passes.
- `just test`: **1229 of 1229 passed**. That is 1206 at Phase 5 plus tests
  another session committed in this worktree.
- `just test-l2`: **6 of 6 passed** (tmux, WezTerm; Kitty is not installed,
  so its entry points skip).
- Linux cross-check: 36/36 lib and 12/12 CLI, as above.
- Scope calculator, final (repository root, after every Phase 6 edit,
  frontmatter included): `git status --porcelain --untracked-files=all |
  awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gave
  `"packages": ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`,
  `"change_class": "package"`, so the gates could not be skipped and were
  run. The input includes the other session's uncommitted
  `2026-10-06-symbol-type-relations` files; `test-toolkit` is a reverse
  dependency, as in Phase 5.

## Implementation of Review Findings #7

> **started at:** 2026-10-07T11:55:32-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-08-scoped-references/review-7.md'
- this is iteration 7 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|---|---|---|---|
| R7-F1 | R7-F1 — Medium — Syntax-marker exclusions still admit Java import-triggered and Swift precedence tokens | fixed | All required children and consumers independently verified; production fixes preexist this pass. Expanded regression coverage in occurrences/cache/CLI; no unresolved defect found in the class. |
| R7-F1.01 | Java local; Static field/method import; var v=id(xs); var Usage/NameOnly; Exclude marker; retain id Call/xs Usage; Full-index CLI 5:9; no-import control excludes it | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.02 | Java final local; final var w=id(v); Same false suggestion; Exclude var; retain id/v; Same sweep 6:15 | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.03 | Java enhanced for; for(var y:xs); Same; Exclude var; retain xs/body y; Same 7:14 | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.04 | Java resource; try(var r=open(xs)); Same; Exclude var; retain open/xs/r; Same 8:14 | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.05 | Java lambda; (var z)->z; Same, anonymous owner; Exclude var; keep body z and region; Same 9:58 | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.06 | Java record pattern; o instanceof P(var a); Same; Exclude var; retain P/o/body a; Same 10:28 | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.07 | Java for initializer; for(var i=0;i<var;i++); Both tokens Usage; Exclude marker 11:14; keep real parameter read 11:29; Same: eight var suggestions instead of one across .01–.07 | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.08a | static field import; import static p.Tools.var; Activates false legacy-type flag; A value import is not evidence of a type named var; Independent import edit, pinned static token | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.08b | static method import; import static p.Methods.var; var x=var(); Marker Usage plus real Call; Exclude marker; retain Call; Independent CLI method-call control | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.09 | import controls; None; ordinary/static wildcard; static import ending in another name; var only in intermediate qualifier; Inference markers excluded; Clean; intermediate spelling does not establish a type; Eight-import sweep; actual qualifier var retained | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Legacy annotation and in-file group references also pass marker_exclusions_keep_legacy_reads_group_references_and_anonymous_regions. |
| R7-F1.10 | legacy controls; Class/interface/enum/annotation type/type parameter named var; ordinary single-type import; Real var type use retained; Preserve provable legacy reads; Independent sources and current legacy matrix. Annotation declaration-token leak is R7-F2.01 | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Legacy annotation and in-file group references also pass marker_exclusions_keep_legacy_reads_group_references_and_anonymous_regions. |
| R7-F1.11a | Swift associativity directive; associativity: left/right/none; Directive Usage; Exclude directive name; Error-free precedence_group_attribute tree; same-spelled local and later read | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.11b | Swift assignment directive; assignment: true; assignment Usage; Exclude directive; true is already not captured; Same | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.11c | Swift higherThan directive; higherThan: P; higherThan Usage; Exclude directive; retain P; Same | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.11d | Swift lowerThan directive; lowerThan: Q; lowerThan Usage; Exclude directive; retain Q; Same | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.12a | Swift left value; associativity: left; Usage/NameOnly to local left; Exclude built-in value; keep ordinary left reads; 9:18 extra versus 16:57 required | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.12b | Swift right value; associativity: right; Same; Same role-based exclusion; Three-group CLI sweep; later right read retained | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.12c | Swift none value; associativity: none; Same; Same role-based exclusion; Same, later none read retained | already fixed | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. |
| R7-F1.13 | Swift group references; higherThan P, lowerThan Q, operator’s group P; Usages retained; Clean; preserve real group names; Exact attribute/operator roles and independent CLI | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Legacy annotation and in-file group references also pass marker_exclusions_keep_legacy_reads_group_references_and_anonymous_regions. |
| R7-F1.14.lua | Lua attributes; const/close attribute identifiers versus initializer/later reads; Attribute token excluded; actual reads retained; Clean for these roles; preserve both excluded markers and retained reads; Role-based Lua attribute arm; exact MARKER_EXCLUSION_MATRIX and independent CLI | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. attribute identifiers excluded only in attribute nodes; initializer/value/member reads remain. |
| R7-F1.14.php | PHP relative operands; self/static/parent type/new/instanceof versus actual method names; Relative marker excluded; actual method Calls retained; Clean for these roles; preserve both excluded markers and retained reads; Role-based PHP class operand arm; exact matrix/cache/CLI controls | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Class operands reserved case-insensitively; genuine same-spelled method Calls remain. |
| R7-F1.14.bash | Bash options; declare -i, local -- versus expansions/ordinary commands; Options excluded; actual command/read tokens retained; Clean for these roles; preserve both excluded markers and retained reads; declaration_command-only option rule; shell exact-offset rows | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Options excluded only in declaration_command through --; ordinary grep argument remains. |
| R7-F1.14.zsh | Zsh options; typeset +x, integer operands versus expansions/commands; Options/declared operands excluded; expansions retained; Clean for these roles; preserve both excluded markers and retained reads; Same local option rule; exact-offset rows and independent CLI | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Same option invariant plus integer declaration/expansion controls. |
| R7-F1.14.rs | Rust modifiers; pub/mut/impl/dyn in modifier/operator roles; No marker occurrences; ordinary same-spelled permitted names keep binding behavior; Clean for these roles; preserve both excluded markers and retained reads; Reference query captures identifier/type/field names; these modifier roles are anonymous keywords, never candidate names | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.ts | TypeScript modifiers; visibility/static/async/type operators; No modifier occurrences; real identifiers retained; Clean for these roles; preserve both excluded markers and retained reads; Grammar keyword roles versus identifier captures; existing supported-binding/exclusion controls | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.js | JavaScript modifiers; static/async/await operator roles; No modifier occurrences; real identifiers retained; Clean for these roles; preserve both excluded markers and retained reads; Same ECMA role distinction, current supported-binding controls | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.py | Python keywords; async/await and match/type in keyword roles; No keyword occurrences; usable match/type names retained; Clean for these roles; preserve both excluded markers and retained reads; Keyword node roles versus identifier captures; current Python declaration/read controls | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.go | Go keywords; go/defer, actual any/comparable names; Keywords absent; type-name reads retained; Clean for these roles; preserve both excluded markers and retained reads; Keywords are not identifier nodes; any/comparable remain names, current all-language rows | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.c | C storage modifiers; static/extern/typedef roles; No storage-keyword occurrences; real type/value names retained; Clean for these roles; preserve both excluded markers and retained reads; Anonymous keyword tokens versus identifier/type_identifier query captures | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.cpp | C++ modifiers; static/constexpr/auto/decltype roles; No tested modifier occurrences; actual declared type/value reads retained; Clean for these roles; preserve both excluded markers and retained reads; Built-in/modifier grammar roles versus named reference captures; current C++ matrices | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.cs | C# modifiers; record/readonly/async/var/await; usable dynamic/nameof controls; Modifiers absent; genuine named-type/name reads retained; Clean for these roles; preserve both excluded markers and retained reads; var is implicit_type, keywords are not identifier captures; current C# matrices | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.scala | Scala modifiers; given/using/inline/lazy roles; No modifier occurrence; actual bounds/body reads retained; Clean for these roles; preserve both excluded markers and retained reads; Keyword roles and explicit given-name exclusion; independent given/abstract-member controls | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.14.perl | Perl declaration keywords; my/our/state/local versus actual variables/calls; Keywords absent; true variable/call reads retained; Clean for these roles; preserve both excluded markers and retained reads; Reference query captures variables/call names/package identifiers, not declaration keywords; current exact Perl controls | clean | contextual_syntax_markers_are_never_occurrences_and_reads_remain: exact MARKER_EXCLUSION_MATRIX offsets and kinds passed; both excluded tokens and independently expected genuine reads verified. Added explicit modifier/read control; anonymous keyword/modifier roles cannot match identifier reference captures, while the named type/value positions independently remain reads. |
| R7-F1.C1 | Collection and marker gate; Distinguish static value imports from established legacy type evidence; add every Swift directive/value above by role. Preserve R6-F3.06 legacy reads. | already fixed | Inspected java_declared_type_name: static token rejects type evidence; ordinary single imports and declarations retain legacy mode. swift_markers excludes directive plus associativity value only. Exact marker matrix passed. |
| R7-F1.C2 | Exclusion/category/candidates/lookup; Correct excluded_names before fallback. Marker tokens must never reach NameOnly lookup; real reads keep their conservative IDs/confidence. | already fixed | Walk::markers feeds excluded_names before Resolver::occurrence/candidate selection; exact marker matrix rejects all excluded offsets while retaining independently listed Call/Usage offsets, uncertain IDs/candidates. |
| R7-F1.C3 | Ownership/summaries/coverage/recursion; Keep the Java lambda’s anonymous boundary after excluding its marker. Limited languages retain Limited coverage, empty established summaries and Unknown recursion. | already fixed | assert_exclusion_rows asserts uncertainty, Limited unsupported_language coverage, empty references/referenced_by and Unknown recursion for every limited-language row. Anonymous lambda owner/region test passed. |
| R7-F1.C4 | Filtering/projection/cache/widening; Public API reproduces extra rows through narrow Miss → Hit → semantic widening without a parse → kind widening → complete Hit → projected Hit, for Java and Swift. Pin required absences and real-read offsets. | fixed | contextual_syntax_markers_stay_excluded_through_the_cache passed: six actual lookup stages (Miss, Hit, unparsed semantic widening, kind widening, complete Hit, projected Hit). Added EveryInference.java verifies all var offsets; EveryPrecedence.swift verifies all directive/value absences and exact retained reads/candidates. Occurrence matrix separately verifies other Java reads. |
| R7-F1.C5 | Default and Call reports; Cold/warm ID-selected JSON/text list false suggestions with actual warm Hits. Call-only filters already reject these Usages. Preserve real var Calls and ordinary Swift reads; repairing Call reports alone is insufficient. | fixed | contextual_syntax_markers_are_never_suggestions passed: ID-selected cold Miss/warm Hit JSON/text, all seven Swift directive/value spellings, real static-method var Call, exact suggestions, zero established occurrences; caller filters reject Usages and preserve Calls. |
| R7-F1.C6 | Tests/docs/comments/invalidation; Extend declared marker/cache/CLI matrices with the complete inventory, correct legacy-import and precedence documentation/comments, and invalidate old extractor output. No resolver/version integration needed. | fixed | Revision 46 and its stale-revision-45 test independently verified (1 passed). Current references.md and tree-hugger skill correctly describe static imports, precedence roles and S1–S6. Expanded existing declared L1 matrices and matching docblock; no production repair needed. |
| R7-F1.S1 | Swift legacy operator body words excluded; ordinary same-spelled reads remain | already fixed | Exact S1–S4 occurrence row passed. Availability/configuration six-stage cache row passed; exclusion uses grammar roles, not a global name blacklist. |
| R7-F1.S2 | Swift availability platforms excluded; ordinary macOS/iOS reads remain | already fixed | Exact S1–S4 occurrence row passed. Availability/configuration six-stage cache row passed; exclusion uses grammar roles, not a global name blacklist. |
| R7-F1.S3 | Swift available attribute arguments excluded; non-marker wrapper arguments remain | already fixed | Exact S1–S4 occurrence row passed. Availability/configuration six-stage cache row passed; exclusion uses grammar roles, not a global name blacklist. |
| R7-F1.S4 | Swift conditional-compilation operands excluded; ordinary DEBUG/UIKit reads remain | already fixed | Exact S1–S4 occurrence row passed. Availability/configuration six-stage cache row passed; exclusion uses grammar roles, not a global name blacklist. |
| R7-F1.S5 | Named-argument and tuple labels never read bindings; true values/assignments/map keys retained | already fixed | All seven language rows and three designator/table-key controls passed; Python/Swift cache stages and CLI controls passed. |
| R7-F1.S5.py | keyword_argument name excluded; value retained | already fixed | Exact corresponding MARKER_EXCLUSION_MATRIX S5 row passed; expected offsets/kinds independently listed. |
| R7-F1.S5.cs | argument and attribute x: labels excluded; member x=value assignments retained | already fixed | Exact corresponding MARKER_EXCLUSION_MATRIX S5 row passed; expected offsets/kinds independently listed. |
| R7-F1.S5.php | argument labels excluded; value Call retained | already fixed | Exact corresponding MARKER_EXCLUSION_MATRIX S5 row passed; expected offsets/kinds independently listed. |
| R7-F1.S5.swift | call/function-reference/tuple/type/pattern labels excluded; values/dictionary keys retained | already fixed | Exact corresponding MARKER_EXCLUSION_MATRIX S5 row passed; expected offsets/kinds independently listed. |
| R7-F1.S5.scala | direct argument assignment label excluded; parenthesized assignment and values retained | already fixed | Exact corresponding MARKER_EXCLUSION_MATRIX S5 row passed; expected offsets/kinds independently listed. |
| R7-F1.S5.perl | fat-comma word quoted/excluded; value reads and genuine Call retained | already fixed | Exact corresponding MARKER_EXCLUSION_MATRIX S5 row passed; expected offsets/kinds independently listed. |
| R7-F1.S5.go | literal struct field keys excluded; map and named-type keys retained | already fixed | Exact corresponding MARKER_EXCLUSION_MATRIX S5 row passed; expected offsets/kinds independently listed. |
| R7-F1.S5.c | Designator/table keys do not become occurrences; actual initializer values remain | clean | Exact S5 control row passed. Reference captures omit designators/table-name keys, while value identifier offsets are retained. |
| R7-F1.S5.cpp | Designator/table keys do not become occurrences; actual initializer values remain | clean | Exact S5 control row passed. Reference captures omit designators/table-name keys, while value identifier offsets are retained. |
| R7-F1.S5.lua | Designator/table keys do not become occurrences; actual initializer values remain | clean | Exact S5 control row passed. Reference captures omit designators/table-name keys, while value identifier offsets are retained. |
| R7-F1.S6 | Swift built-in attribute names/keyword arguments excluded; wrappers, global actor, Sendable, SPI groups and derivative target retained | already fixed | Exact S6 matrix passed with positive and negative offsets; attributes.swift six cache stages and objc CLI cold/warm JSON/text passed. Pinned built-in spelling list leaves real type roles intact. |
| R7-F1.S7 | C# global namespace alias excluded; extern alias qualifier and path names retained | clean | Existing global/extern-alias marker matrix rows passed. global is excluded only when alias child token is global; named extern aliases remain usages. Import-specific ledger belongs to R7-F3.S2. |
| R7-F2 | R7-F2 — High — Declaration and import-name exclusions omit unsupported and compound name forms | fixed | All required source repairs preexist; repaired missing per-shape cache and eight-selector CLI coverage. Every required/recurrence child verified; no open defect. |
| R7-F2.01 | Java annotation types; Top-level/nested @interface A; elements/defaults and actual @A; Declaration A Usage; element name excluded, real default/use kept; Exclude annotation-type name; retain genuine reads; CLI/tree: nested 2:22 extra vs 2:59 required; top-level 1:12 extra | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.02a | simple Java modules; module m, open module n; Declared names Usage; No declaration-name occurrence; CLI/tree: m 1:8, n 2:13 extra | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.02b | qualified Java modules; module a.b; Currently hidden by R7-F3’s false imports; Exclude name tokens after repairing imports; keep directive reads; Grammar module_declaration.name scoped_identifier; masked case, not a clean exclusion | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.03 | Java packages; package a.b, simple control, later a/b types; Compound declaration a/b Usage; Exclude declaration components; keep later types; 1:9/1:11 extra vs 1:46/1:51 required | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.04 | C# static constructors; static G(), ordinary constructor, later G type; Static name Usage with Class/Constructor candidates; Exclude static declaration; retain type read; 1:18 extra vs 1:42 required; static constructor has no record by design | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.05a | C# custom events; event Action E {add{} remove{}} vs field-like event; Accessor-event E Usage; field-like name excluded; Exclude event name, retain type and later E read; 2:31 extra vs 2:54 required | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.05b | explicit-interface events; event Action IFace.E {add{} remove{}}; E declaration Usage; IFace/type reads kept; Exclude E only; preserve IFace/type; Error-free event_declaration.name tree | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.06 | C# external aliases; extern alias A/B; later A::T/B::U; Declaration aliases Usage; Exclude binders; retain real qualified usages; 1:14/1:30 extra vs 1:63/1:71 required | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.07 | C# block namespace; namespace A.B {...}; Both declaration components Usage; Exclude both; retain real A/B reads; 1:11/1:13 extra vs 1:47/1:52 required | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.08 | C# file namespace; namespace A.B;; Same extra components; Same exclusion; simple-name control remains clean; Independent CLI/tree and one compound Namespace record | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.09a | C++ nested namespace; namespace A::B, C::D::E; All declaration components Usage; Exclude every component; keep actual A/B method Calls; Two-depth CLI: 1:47/1:50 extra vs Calls 1:72/1:79 | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.09b | C++ inline namespace; namespace A::inline B; Both components Usage; Exclude A/B despite inline token; Error-free pinned tree, 1:11/1:21 extra | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.10a | Scala qualified package; package a.b; later a/b type reads; Declaration a/b Usage; Exclude declared path; keep types; 1:9/1:11 extra vs 4:17/4:32 required | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.10b | Scala braced package; package a.b {class G}; Same extra components; Same token exclusion; Independent CLI, error-free tree | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.10c | Scala stacked/plain packages; package a; package b on consecutive lines; Both simple names Usage; Exclude each package declaration; Independent CLI: 1:9/2:9 extra | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.11a | Perl qualified package; package A::B; later A()/B(); Both package segments Usage with sub candidates; Exclude declaration; retain Calls; 1:9/1:12 extra vs Calls 1:31/1:36 | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.11b | Perl versioned/braced package; package A::B v1.0 {sub f{}}; Same extra segments; Same exclusion; simple-package name remains excluded; Error-free independent CLI/tree | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.12a | TypeScript dotted namespaces; A.B and A.B.C, plain-name control, later A type; Declaration prefix A Scoped to class A; Exclude declaration prefix; retain true type read and lexical context; 1:22 extra vs 1:58 required; public cache stages preserve false ID | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.12b | TypeScript named-owner consumer; class A {}; function g(){ namespace A.B{} }; Declaration A is Scoped to class A, g.references includes A and A.referenced_by includes g; Exclude declaration token and both fabricated summary edges; Independent full-index CLI; this is parseable unfinished source, not compiler-valid namespace placement. Exclusion applies to declaration syntax regardless of compiler validation | already fixed | typescript_namespace_prefixes_never_bind_to_a_same_named_class: exact class ID for true type read, no declaration-prefix occurrence, no fabricated g→A/reverse edge; new cache sweep checks summary absence at all six stages. |
| R7-F2.13 | Swift macros; macro foo()=...; same-spelled function and actual foo(); Macro declaration foo Usage; Exclude macro name; preserve real Call and signature reads; macro_declaration(simple_identifier); 2:7 extra vs Call 3:1 | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.14 | Swift precedence groups; precedencegroup P; operator group P; Declaration P Usage with type candidate; Exclude declaration P; keep group reference; 2:17 extra vs 3:21 required; attribute markers are R7-F1 | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.15.{typealias,class,enum,protocol,struct,func,var,let} | Swift selective imports; Each of eight import selectors with Foo.Name, and later Name reads; Final imported binding Name Usage; simple-import control excluded; Exclude the binding token, retain actual later reads; no global name filter; Eight error-free selector probes; struct Bar at 2:19 extra vs real Bar 3:8 | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.16 | other named declarations; Java ordinary types/elements/generics/constructors; C# instance constructors/delegates/destructors/properties/patterns/simple namespaces; C++ unions/aliases/concepts/templates; Scala abstract members/givens/extensions; Swift protocol members; Perl prototypes; Tested declaration names excluded, actual type/default/body reads retained; Clean for these forms; Independent CLI controls, grammar-name inventory and current DECLARATION_EXCLUSION_MATRIX. Accessor keywords are not identifier captures | clean | Expanded language child inventory below; all prior declaration matrix rows now also pass six cache stages. Exact range exclusion never filters a spelling globally; genuine type/default/body controls retained. |
| R7-F2.17 | remaining language sites; Rust/JS binding sites; Python alias/named-expression/pattern names; Go package/binders; C names; PHP trait/alias declarations; Lua locals/table names; shell operands/options; Current required exclusion/read rows pass; Clean for earlier inventory; preserve independently expected real reads; Full all-language declaration/initializer/parameter matrices plus Go/Python/PHP/shell CLI controls | clean | Expanded language child inventory below; all prior declaration matrix rows now also pass six cache stages. Exact range exclusion never filters a spelling globally; genuine type/default/body controls retained. |
| R7-F2.C1 | Collection/name spans/exclusion; Add every missing declaration site without synthetic records; exclude actual child name-token spans for compound declarations. TypeScript must use its binding/exclusion truth. | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.C2 | Binding/categories/candidates; Suppress the TypeScript declaration prefix before it establishes the class ID, and limited-language tokens before NameOnly lookup. Preserve alias/category and genuine-read evidence. | already fixed | Existing production repair independently verified by declaration_tokens_are_never_occurrences_and_reads_remain and the full exclusion matrix through six actual cache stages; exact required reads and declaration-token absence. |
| R7-F2.C3 | Ownership/summaries/coverage/recursion; Independent .12b produces a g-to-A summary and its reverse edge. Shared exclusion must remove both as well as the occurrence. Preserve named/anonymous regions, recursion controls and limited-language uncertainty. | already fixed | typescript_namespace_prefixes_never_bind_to_a_same_named_class: exact class ID for true type read, no declaration-prefix occurrence, no fabricated g→A/reverse edge; new cache sweep checks summary absence at all six stages. |
| R7-F2.C4 | Filtering/projection/cache/widening; Independent six-stage API probes reproduce Java annotation, C# event/static constructor, TS namespace, Swift macro/selective-import errors. Assert exact declared-token absence and real-read presence at actual Hits, including omitted kinds. | fixed | every_declaration_exclusion_shape_holds_through_six_cache_stages passed: every matrix shape, independently expected offsets/kinds, actual Miss/Hit/record-only widening/kind widening/complete Hit/projected Hit; TS omitted-kind ID retained. |
| R7-F2.C5 | Default/Call reports and selection; Cold/warm ID-selected JSON/text show the extra rows; TypeScript shows an established reference. Remove declaration Usages while preserving real Calls/Usages, name/ID selection and ambiguity. Call-only directions already reject these Usages. | fixed | unreported_and_compound_declaration_names_are_never_references passed cold/warm ID-selected Java/TS JSON/text; every_swift_selective_import_selector_excludes_only_its_bound_token passed all eight selectors plus imports/callers controls; prior selection/ambiguity tests unchanged. |
| R7-F2.C6 | Import repair dependency; R7-F3 currently masks qualified Java module declarations. Repair both decisions together: removing false imports must restore annotation/directive reads without exposing module declaration tokens. This is actionable implementation work, not a human blocker. | already fixed | Qualified Java module .02b excludes a/b while requires c/d remain exact reads after structural Java import repair; occurrence and all six cache stages pass. |
| R7-F2.C7 | Tests/docs/comments/revision; Extend declared L1 exclusion/cache/CLI matrices with every row and positive/negative control, update current docs/affected comments and invalidate stale extractor results. Keep IDs/counts unless intentionally repairing a false declaration. | fixed | Extended existing declared L1 symbols_occurrences/symbols_cache/CLI symbols modules; refs docs declared-path/selective-import table and symbol-kind elaborated-use examples match source; extractor revision 46 invalidates revision 45. |
| R7-F2.15.typealias | Swift typealias import excludes final binding token, retains module and later read | already fixed | Exact .15.typealias declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.15.class | Swift class import excludes final binding token, retains module and later read | already fixed | Exact .15.class declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.15.enum | Swift enum import excludes final binding token, retains module and later read | already fixed | Exact .15.enum declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.15.protocol | Swift protocol import excludes final binding token, retains module and later read | already fixed | Exact .15.protocol declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.15.struct | Swift struct import excludes final binding token, retains module and later read | already fixed | Exact .15.struct declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.15.func | Swift func import excludes final binding token, retains module and later read | already fixed | Exact .15.func declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.15.var | Swift var import excludes final binding token, retains module and later read | already fixed | Exact .15.var declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.15.let | Swift let import excludes final binding token, retains module and later read | already fixed | Exact .15.let declaration row and six cache stages; eight-selector CLI test verifies cold/warm JSON/text, bound import name/source and empty callers. |
| R7-F2.S1 | Swift whole-module import path tokens | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.S2 | PHP compound namespace components | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.S3 | PHP promoted/by-reference and variadic parameter binders | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.S4 | Scala package object binder | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.S5 | Scala renamed import alias binders; original name reads | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.S6 | Scala type-lambda/context-bound/named-tuple binders | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.S7 | Python future feature/alias binders; real feature reads | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.S8 | C/C++ standalone/qualified type names; qualifiers and specialization reads | already fixed | Exact same-ID DECLARATION_EXCLUSION_MATRIX row(s) passed directly and at all six cache stages; both exclusion and genuine reads asserted. |
| R7-F2.X1 | Elaborated C/C++ type usages remain reads and produce no false Type record | already fixed | elaborated_type_uses_are_reads_not_declarations: exact type-record list, Usage/path/candidate controls for C and C++; definition-only shapes retain records. |
| R7-F2.16.java | Preserve clean java named declarations and genuine reads | clean | R5-F2.01/.02/.03/.S1/.S2: generics, compact constructors, labels, annotation elements and patterns. Ordinary type/member names enter shared definition_ranges from records; inventory supplements unreported binders only. Direct exclusion matrix and six-stage cache verification passed. |
| R7-F2.16.cs | Preserve clean cs named declarations and genuine reads | clean | R5-F2.22/.23/.24/.S12/.S13: delegates/destructors, lambda/query/tuple/pattern/local-function names. Instance constructor .04 and simple namespace .08 controls; recorded property names use exact record name spans. Direct exclusion matrix and six-stage cache verification passed. |
| R7-F2.16.cpp | Preserve clean cpp named declarations and genuine reads | clean | R5-F2.17–.21/.S10/.S11 and R7-F2.S8: template/concept/alias/union/destructor names. Record name spans exclude recorded concepts and aliases, while only final qualified declaration token is excluded and qualifier reads stay. Direct exclusion matrix and six-stage cache verification passed. |
| R7-F2.16.scala | Preserve clean scala named declarations and genuine reads | clean | R5-F2.04–.09 plus R7-F2.S4–.S6: abstract type members, generics, package objects, contextual/type-lambda names. Recorded methods/givens/extensions use definition_ranges; type/bound/body positions are distinct reads. Direct exclusion matrix and six-stage cache verification passed. |
| R7-F2.16.swift | Preserve clean swift named declarations and genuine reads | clean | R5-F2.25–.28/.S14/.S15: protocol associatedtype/init/subscript labels, parameter packs, optional/case/capture/setter binders; bounds/body reads retained. Direct exclusion matrix and six-stage cache verification passed. |
| R7-F2.16.perl | Preserve clean perl named declarations and genuine reads | clean | R5-F2.31/.S17: signature/loop names; subroutine records exclude function/prototype name by exact name span, genuine Call .11a remains. Direct exclusion matrix and six-stage cache verification passed. |
| R7-F2.17.rust | Preserve clean rust exclusion and read decisions | clean | R5-F2.33/.S20; Rust binding-site inventory excludes binders, constant patterns read; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.js | Preserve clean js exclusion and read decisions | clean | R5-F2.35; named-expression/default/pattern binding scopes use ECMA truth; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.python | Preserve clean python exclusion and read decisions | clean | R5-F2.11–.16/.S8/.S9; aliases/PEP695/lambda/pattern targets excluded, defaults/stable/assignment reads retained; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.go | Preserve clean go exclusion and read decisions | clean | R5-F2.10/.S6/.S7/.S20; package record name excluded by definition_ranges, generics/range/select/receiver binders checked; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.c | Preserve clean c exclusion and read decisions | clean | R5-F2.36 and R7-F2.S8 control; parameter/local/tag declarations and initializer reads; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.php | Preserve clean php exclusion and read decisions | clean | R5-F2.29/.30/.S16 plus R7-F2.S2/.S3; recorded trait/alias names have exact record exclusion, key/member/initializer reads retained; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.lua | Preserve clean lua exclusion and read decisions | clean | R5-F2.32/.S18/.S19; locals/table names versus computed keys/member/body reads; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.bash | Preserve clean bash exclusion and read decisions | clean | R5-F2.37; declaration operands and option markers use declaration_command-only roles, expansions read; direct all-language matrix and six-stage cache check passed. |
| R7-F2.17.zsh | Preserve clean zsh exclusion and read decisions | clean | R5-F2.38; same shell inventory and independent Zsh row; direct all-language matrix and six-stage cache check passed. |
| R7-F3 | R7-F3 — Medium — Java qualified references are misclassified as imports and disappear | fixed | All required children and prior S1–S7/X1–X3 verified; expanded missing consumer coverage for all Java directive roles, named arguments, deep paths, candidate identities and both CLI call directions. No remaining scoped-reference defect. |
| R7-F3.01 | marker annotation; @A → @p.q.A; A Usage disappears; p/q/A become imports; Retain qualified annotation target and genuine qualifier evidence; no imports; One-edit CLI control, public referenced_symbols, default cold/warm reports | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.02 | annotation arguments; @p.q.A(value=external); Name lost; value/external retained; Keep annotation target independently of real arguments; CLI/error-free annotation.name tree | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.03 | requires directive; requires a.b; simple control; Qualified names disappear into imports; Retain reference operands/path; no import bindings; Full-directive CLI/tree inventory | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.04 | exports package; exports p.q; Operands become imports; Retain referenced package; not a declaration; Same | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.05 | exports target modules; exports p.q to x.y,z.w; Both module paths become imports; Retain every target-module operand/path; Same, both comma-separated elements | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.06 | opens package; opens r.s; Operands become imports; Retain package references; Same | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.07 | opens target modules; opens r.s to t.u; Module path becomes imports; Retain target-module references; Same | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.08 | uses service; uses C → uses a.b.C; C disappears; Retain C Usage/path and real qualifier reads; One-edit control and public diagnostics-reference API | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.09 | provides service; provides C with D → qualified service; Provided type becomes imports; Retain provided-service target/path; Same control/edit | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.10 | provider list; d.e.D,f.g.F; Every provider disappears as imports; Retain every distinct provider; Full-directive source, both elements | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.11 | nested paths; Three-plus segments in every applicable slot; Each segment can become a phantom import; Enforce context at every nesting depth; Grammar inventory: annotations/imports/packages/modules/directives plus recursive scoped_identifier.scope | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.12 | genuine imports; Ordinary/static single and on-demand clauses; Imports reported from actual import nodes; Preserve real imports and existing final-component filtering; Eight-import sweep, fixture/public import controls | clean | java_imports_report_only_their_final_segment and Java matrix .12: exact ordinary/static single/on-demand imports, qualifiers and deep paths retained; no false import tokens become occurrences. |
| R7-F3.13 | declarations/other paths; package a.b, module a.b, regular qualified type paths, simple annotations/directives; Package imports skipped; module declaration falsely imported; scoped_type_identifier unaffected; No declaration imports; R7-F2 excludes declaration tokens. Real type and simple-reference controls retained; CLI/tree distinguishes name-node categories; module case currently masks missing exclusion | already fixed | java_qualified_names_outside_an_import_are_reads_not_imports: exact hand-stated offsets, paths, Usage/confidence, candidate IDs and imports; all Java inventory rows also verified through six cache stages and public referenced_symbols. |
| R7-F3.14.rs | Rust import reader; extern/use/alias/list captures; Captures structurally inside import syntax; Clean for false-import-context class; preserve aliases; [Effective Rust import patterns](../../lib/queries/vendor/rust/locals.scm:1); current public alias/binding rows | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.ecma | TS and JS readers; import_specifier and namespace_import; Structurally import-scoped; Clean; keep original-versus-alias decisions; [Shared ECMA import patterns](../../lib/queries/vendor/ecma/locals.scm:28); current imported-binding/default rows | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.go | Go reader; import_spec/import_spec_list; Structurally import-scoped; Clean; preserve aliases/path controls; [Go import patterns](../../lib/queries/vendor/go/locals.scm:64), public imports and all-language rows | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.py | Python reader; aliased_import, import_statement, import_from_statement; Structurally import-scoped; Clean; preserve actual names/read controls; [Python import patterns](../../lib/queries/vendor/python/locals.scm:23) plus current fixture/alias tests | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.cs | C# reader; using_directive captures; Structurally import-scoped; Clean for false ancestry; external alias exclusion is R7-F2; [C# import patterns](../../lib/queries/vendor/c_sharp/locals.scm:82) and namespace/alias probes | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.scala | Scala reader; import_declaration; Structurally import-scoped; Clean for ancestry; package declarations are R7-F2; [Scala import pattern](../../lib/queries/vendor/scala/locals.scm:93); package/import role comparison | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.swift | Swift reader; import_declaration; Structurally import-scoped; Clean for ancestry; final child exclusion is R7-F2; [Swift import pattern](../../lib/queries/vendor/swift/locals.scm:1) and eight selective-import probes | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.php | PHP reader; namespace_use_clause; Structurally import-scoped; Clean for ancestry; retain genuine trait/member reads under existing contract; [PHP import pattern](../../lib/queries/vendor/php_only/locals.scm:75) and current import/marker controls | clean | Effective locals/imports query patterns are structurally inside import syntax; no_query_carries_a_predicate_the_runtime_does_not_enforce compiles every query/language and rejects unsupported predicates. Import binding/read controls passed in just test -j 4 import (57 tests); prior metadata siblings verified separately as S1–S7/X1–X3. |
| R7-F3.14.c | C reader; Effective import-definition patterns; No import-definition capture; Clean for this overcapture: the query cannot invent an import; this does not imply broader import support; Effective c locals query inventory and current occurrence fixtures | clean | Effective locals queries have no local.definition.import capture: imported_symbols only accepts that capture, so these readers cannot invent imports. All-language query predicate guard passes; no broader import support claimed. |
| R7-F3.14.cpp | C++ reader; Effective import-definition patterns; No import-definition capture; Clean for this overcapture: the query cannot invent an import; this does not imply broader import support; Effective cpp locals query inventory and current occurrence fixtures | clean | Effective locals queries have no local.definition.import capture: imported_symbols only accepts that capture, so these readers cannot invent imports. All-language query predicate guard passes; no broader import support claimed. |
| R7-F3.14.lua | Lua reader; Effective import-definition patterns; No import-definition capture; Clean for this overcapture: the query cannot invent an import; this does not imply broader import support; Effective lua locals query inventory and current occurrence fixtures | clean | Effective locals queries have no local.definition.import capture: imported_symbols only accepts that capture, so these readers cannot invent imports. All-language query predicate guard passes; no broader import support claimed. |
| R7-F3.14.perl | Perl reader; Effective import-definition patterns; No import-definition capture; Clean for this overcapture: the query cannot invent an import; this does not imply broader import support; Effective perl locals query inventory and current occurrence fixtures | clean | Effective locals queries have no local.definition.import capture: imported_symbols only accepts that capture, so these readers cannot invent imports. All-language query predicate guard passes; no broader import support claimed. |
| R7-F3.14.bash | Bash reader; Effective import-definition patterns; No import-definition capture; Clean for this overcapture: the query cannot invent an import; this does not imply broader import support; Effective bash locals query inventory and current occurrence fixtures | clean | Effective locals queries have no local.definition.import capture: imported_symbols only accepts that capture, so these readers cannot invent imports. All-language query predicate guard passes; no broader import support claimed. |
| R7-F3.14.zsh | Zsh reader; Effective import-definition patterns; No import-definition capture; Clean for this overcapture: the query cannot invent an import; this does not imply broader import support; Effective zsh locals query inventory and current occurrence fixtures | clean | Effective locals queries have no local.definition.import capture: imported_symbols only accepts that capture, so these readers cannot invent imports. All-language query predicate guard passes; no broader import support claimed. |
| R7-F3.C1 | Import collection/context/provenance; Enforce actual import ancestry or structurally scope the query. Sweep nested paths, package exclusion and true-import intermediate filtering together. Update provenance for query edits. | already fixed | Java import_declaration-scoped query, all-language predicate guard (1 passed), query provenance notes and source metadata readers independently inspected; other import readers have structural ancestry. |
| R7-F3.C2 | Definitions/exclusions/occurrences; Correct collection before definition_ranges removes real reads. Restore every required Usage/path before ownership/candidates. Keep explicit declaration exclusions from R7-F2. | already fixed | Shared imported_symbols input precedes definition_ranges in occurrences and referenced_symbols; full Java matrix reads/exclusions and public inventory passed; F2 retains package/module declaration exclusion. |
| R7-F3.C3 | Candidates/summaries/coverage/recursion; Restored Java reads can have NameOnly candidates, never invented IDs. Keep Limited coverage, empty established summaries and Unknown recursion. No resolver expansion. | fixed | assert_import_rows now compares exact candidate IDs; six-stage Java cache independently asserts Usage/confidence/candidate names/no IDs, Limited unsupported-language coverage, empty summaries and Unknown recursion. |
| R7-F3.C4 | Selective filtering/cache; Both missing-reference sources remain wrong through all six public cache stages, including semantic widening without a parse and actual projected Hits. Imports and evidence must both become correct; validation cannot fix a collector’s false source facts. | fixed | qualified_reads_and_imports_hold_through_the_cache reuses every Java inventory row and sibling sources with independent expected reads/imports through Miss, Hit, no-parse semantic widening, kind widening, complete Hit and projected Hit (passed). |
| R7-F3.C5 | Public reference/diagnostics consumer; referenced_symbols independently loses qualified annotation/service references but retains unqualified controls. Repair its shared import input too; do not change severity/confidence/default enablement. | fixed | New java_import_context_inventory_keeps_public_referenced_symbols verifies all Java operand roles and genuine import controls via public diagnostics references; passed in import (57). Severity/default enablement source remains unchanged. |
| R7-F3.C6 | CLI refs/imports; imports invents annotation/directive entries; cold/warm ID-selected default text/JSON omit real suggestions. Assert restored offsets/paths and no imports in files without import clauses. These Usages remain outside callers/callees. | fixed | Expanded qualified_java_annotations_and_services_are_suggestions_not_imports asserts every directive operand, provider list, deep annotation and named args in actual cold/warm ID-selected JSON/text; empty callers+callees; imports JSON/plain reports no phantom entries (passed). |
| R7-F3.C7 | Tests/docs/comments/invalidation; Add all rows to declared L1 matrices, including diagnostics-reference API, import report and cache stages; update current docs/reader comments and invalidate old extractor evidence. | fixed | Declared library l1 and CLI symbols targets compile/run additions; current references/report docs, tree-hugger skill, provenance and rev46 invalidation independently inspected; only test behavior changed in this pass, no new source revision needed. |
| R7-F3.02.named | Named annotation arguments preserve target, key and value separately | fixed | New exact matrix row p/q/A/value/external also runs through public referenced_symbols and six cache stages; CLI restores deep annotation target beside real arguments. |
| R7-F3.S1 | C# qualified/static/global/alias using directives preserve binding and qualifier reads | already fixed | just test -j 4 csharp_using: 3 passed; C# source rows, exact public metadata, CLI imports; six-stage cache U.cs also passed. |
| R7-F3.S2 | C# global alias is a marker; extern alias and path segments remain reads | already fixed | F1 marker controls and F2 extern-alias inventory verified; marker reader narrows only alias_qualified_name global token, not E::T. |
| R7-F3.S3 | PHP single, grouped and aliased use clauses preserve bound names | already fixed | just test -j 4 php_use_clauses: 2 passed; exact public import metadata/read rows; cache i.php and CLI imports_list_only_the_names_each_import_binds passed. |
| R7-F3.S4 | Python dotted/from/future/on-demand imports preserve binding and real path reads | already fixed | Python matrix S4.01–.03, public python_imports_report_only_their_bound_names, cache i.py and multi-language CLI import report passed in import (57). |
| R7-F3.S5 | Rust use/self/list/glob/extern alias forms preserve bindings and dependencies | already fixed | imports_bind_exactly_the_names_they_bring_in asserts exact dependency set and no IDs for bound imports; public Rust import metadata/cache i.rs/CLI all passed. |
| R7-F3.S6 | Scala qualifiers/renames/selectors/on-demand imports preserve binding and reads | already fixed | SCALA_IMPORT_MATRIX S6.01–.03 with exact candidates/paths/imports; public Scala metadata/cache I.scala/CLI imports passed. |
| R7-F3.S7 | Swift selective imports bind their final segment, source remains path | already fixed | F2 all eight selector matrix, public swift_selective_imports_report_their_bound_name and CLI every_swift_selective_import_selector_excludes_only_its_bound_token passed (import filter). |
| R7-F3.X1 | TS/JS default/aliased/namespace/require/equal imports preserve original and alias | already fixed | IMPORT_BINDING_MATRIX Rust/TS/JS rows independently assert bindings/dependencies; public ECMA metadata, i.ts six-stage cache and CLI import report passed. |
| R7-F3.X2 | TS/JS export aliases/re-export names are excluded, local exports remain reads | already fixed | just test -j 4 export_clause: 2 passed; exact whole-file export matrix/IDs/summaries/public diagnostics references and cold/warm CLI reports; exports.ts cache passed. |
| R7-F3.X3 | Go blank/dot/raw import paths preserve metadata without phantom binding | already fixed | GO_IMPORT_MATRIX, public go_blank_and_dot_imports_report_what_they_bind/raw metadata, cache r.go and CLI raw imports passed (import filter). |
| R7-OBS.1 | Nonblocking separate type-relation contract: Python future-import reader | deferred | Occurrence resolution is intentionally unsupported for Python; future import metadata and declaration exclusion pass S4.02. Spec preserves containment/export contracts. Next action: add relation-tree future-import arm and owner-resolution tests in that separate resolver work. |
| R7-OBS.2 | Nonblocking separate imports display contract: plain-text reconstructed syntax | deferred | Spec requires refs text/JSON, both verified; import-binding facts here use exact public records/JSON. Cosmetic imports syntax rendering is separate. Next action: improve plain imports rendering with dedicated output tests. |

- Nonblocking carried grammar limitations: R6-F1.17b and R5-F1.S12 retain their accepted pinned-grammar prerequisites; no required R7 repair is blocked. No performance measurement is requested.

- Verification scope: source facts and reports are confined to the tree-hugger area (library `tree-hugger`, consumer `tree-hugger-cli`). `sniff repo package-dependencies tree-hugger tree-hugger-cli --json` reports only the CLI as the library's reverse dependency. GitNexus MCP lists stale indexes for other worktrees, not this checkout; repository-wide text searches confirm executable consumers rather than treating those indexes as evidence. Final gates will run the area's build, L1, lint, live L2 rendering check, and tier-coverage audit.

- starting the work on 'R7-F1 syntax-marker exclusions' at 11:58:06
        - Defect class: identifier-shaped compiler directives and inference markers reach reference lookup when contextual category evidence is too broad or a marker role is missing.
        - Sweep: Java inference slots and legacy-type/import gates; Swift precedence, legacy operator bodies, availability/configuration/attribute roles; argument/tuple labels in all matching grammars; Lua attributes, PHP relative operands, C# global aliases, shell options; reference capture roles in all 16 languages.
        - Existing production repairs are present; independently verify them and expand sampled cache/CLI controls to cover all Java inference slots, every Swift precedence directive/value, and the genuine static-method Call.

        - Verified every F1 required row plus prior S1–S6 siblings; added separate S5 consumer-language rows and S7 C# global/extern-alias control. New exact modifier/read controls cover Rust, TS, JS, Python, Go, C, C++, C#, Scala and Perl.
        - Focused verification: `just test -j 4 contextual_syntax` passed 3 tests; `just test -j 4 marker_exclusions_keep` passed 1 test; `just test -j 4 entry_from_extractor_revision_forty_five` passed 1 test. These are declared library l1 and CLI symbols targets selected by L1.
        - An initial new Perl control expected unsigiled x; corrected to the public $x name and byte position, then reran successfully. Initial root recipe and parenthesis-containing filter attempts failed before tests; canonical package-area commands above are the final evidence.
        - Changed in this pass: `lib/tests/l1/symbols_occurrences.rs`, `lib/tests/l1/symbols_cache.rs`, `cli/tests/symbols/refs.rs`; production repairs/docs/revision were already present and independently verified. No blocker or performance measurement applies.
- work completed for 'R7-F1 syntax-marker exclusions' at 12:01:57

- Orchestrator reconciliation for R7-F1: inspected the source gate, marker roles, exact-offset matrices and added cache/CLI assertions. Production instances were already repaired, but required verification coverage needed expansion; parent R7-F1 and consumers C4–C6 are therefore **fixed** this cycle rather than claiming no repair was needed. Final whole-area gates remain required.

- starting the work on 'R7-F2 declaration exclusions' at 12:03:59
        - Defect class: declaration exclusion misses unreported binders and child name tokens of compound declarations, exposing false suggestions or established references.
        - Sweep: shared declared-name expansion, record definition ranges, Rust/ECMA binding sites, every rule-less declaration reader, import-bound names, exclusion-before-candidates, summary consumers, cache projection and CLI reports. Production repairs and prior S1–S8/X1 fixes already exist; expand consumer verification beyond sampled sources.

        - Verified all source rows and recurrence S1–S8/X1. Reused the hand-stated declaration matrix across all six cache stages, preserving independent expected results; expanded all eight Swift selector CLI default/import/callers consumers. TS exact identity and forward/reverse summary controls pass.
        - Focused checks passed: `just test -j 4 declaration_exclusion` (1), `just test -j 4 selective_import` (3), `just test -j 4 declaration_tokens` (3), `just test -j 4 namespace_prefixes` (1), `just test -j 4 compound_declaration` (1), `just test -j 4 elaborated_type` (1). Existing declared L1 targets compile and select all additions.
        - Corrected test-only setup failures: initial root recipe/path mistakes, an incorrect Confidence type name, and decoding omitted public empty occurrence lists; final runs above pass. Stored strict cache reader remains unchanged; only absent public skip-empty lists map to empty, malformed present data still fails.
        - Changed this pass: `lib/tests/l1/symbols_occurrences.rs` (shared matrix visibility), `lib/tests/l1/symbols_cache.rs` (every-shape six-stage coverage), `cli/tests/symbols/refs.rs` (eight-selector CLI coverage). No source/doc/revision repair needed beyond preexisting changes; no blocker or performance measurement.
- work completed for 'R7-F2 declaration exclusions' at 12:10:26

- Orchestrator reconciliation for R7-F2: inspected the shared declaration-token expansion and its record/inventory consumers, exact TypeScript class-ID and bidirectional edge checks, every-shape cache sweep, and eight-selector CLI test. Required review IDs are retained, selector leaves and recurrence rows are explicit, and no F2 row remains open. Final whole-area gates will independently rerun these declared L1 tests.

- starting the work on 'R7-F3 import source-role classification' at 12:12:38
        - Defect class: broad import captures or mismatched binding projections classify real reads as imports or lose required import bindings before exclusion and lookup.
        - Sweep: all effective import query patterns and general predicates, import metadata readers, shared definition ranges and diagnostics references, candidate/summary/coverage/recursion consumers, all six cache stages and CLI default/import/call directions; recurrence and prior S1–S7/X1–X3 included.
        - Existing production repairs are present. Required consumer coverage sampled annotation/services while omitting other Java module operands; reuse the independently stated Java source matrix through cache and public diagnostics references, and expand CLI operand checks.

        - All review source rows and prior S1–S7/X1–X3 siblings verified. Added every Java source shape to six-cache-stage and diagnostics-reference checks, named annotation-argument row, exact candidate-ID assertions, and every directive operand/deep-path CLI suggestions with both callers and callees empty.
        - Focused final checks passed: `just test -j 4 qualified_` (23), `just test -j 4 qualified_reads_and_imports` after candidate/coverage strengthening (1), `just test -j 4 import` after final changes (57), `just test -j 4 export_clause` (2), `just test -j 4 no_query_carries` (1), `just test -j 4 php_use_clauses` (2), `just test -j 4 csharp_using` (3). These execute declared library l1 and CLI symbols targets.
        - Initial new test setup failures (matrix label used as extensionless filename; two hand-stated CLI provider columns) were corrected and rerun; no production defect or parser/OS blocker was found. An incidental path search used the wrong current directory and did not affect verification.
        - Changed this pass: `lib/tests/l1/symbols_occurrences.rs`, `lib/tests/l1/symbols_cache.rs`, `lib/tests/l1/tree_file.rs`, `cli/tests/symbols/refs.rs`. Source repairs, docs, query provenance and extractor rev46 already exist and were independently inspected. No performance measurement required.
        - R7-OBS.1/.2 reconciled as explicit nonblocking separate-contract observations, outside parent finding counts; accepted grammar limitations R6-F1.17b/R5-F1.S12 remain unchanged and do not block these repaired forms.
- work completed for 'R7-F3 import source-role classification' at 12:18:00

- Orchestrator reconciliation for R7-F3: inspected the import-scoped Java query and unenforced-predicate guard, full Java public diagnostics/import inventory, exact cache paths/candidate IDs, and CLI operand lists with both call directions. Ledger audit: every review ID retained, no duplicate IDs, no pending required row. There are 3 fixed parents and 164 required/sibling child rows (12 fixed, 99 already fixed, 53 clean); 2 separately inventoried nonblocking observations remain deferred.
- Final verification started after the last test change: `just test -j 4` in tree-hugger; 1,462 tests selected. No file-format/configuration reader changed; the existing strict cache input-robustness matrices are included in this full run. No new robustness matrix or performance measurement is required.

### Completion Report

**Outcome: complete — implementation complete, ready for review.** Elapsed: 31 min 8 sec (2026-10-07T11:55:32-07:00 → 2026-10-07T12:26:40-07:00).

- Parent findings: **total 3 = fixed 3 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0**. R7-F1, R7-F2 and R7-F3 are fixed; production repairs were already present, and this cycle repaired required regression/consumer coverage gaps and independently verified the source decisions.
- Child ledger: **164** required/sibling rows = **12 fixed + 99 already fixed + 53 clean**. Every original review ID is retained, named selector/language leaves and discovered consumers are explicit, and no required row is pending or open. Two nonblocking observations are separately inventoried and excluded from parent counts.
- Verified repairs:
        - R7-F1: all Java inference slots under static imports, every Swift precedence directive/value, actual reads/Calls and anonymous boundaries; ten language modifier controls; six cache stages and ID-selected cold/warm JSON/text with caller filtering.
        - R7-F2: every declaration-exclusion shape through six cache stages; exact TypeScript class identity and absent fabricated forward/reverse edges; all eight Swift selective-import selectors through refs/import JSON/text and caller filtering; recurrence S1–S8/X1 and language controls.
        - R7-F3: every Java qualified-name role through public diagnostics references/imports and six cache stages, exact paths/candidate identities, all module operands and deep paths in CLI reports, both call directions; recurrence S1–S7/X1–X3 and the all-query unenforced-predicate guard.
- Open required finding/child IDs: **none**. Nonblocking carryovers remain:
        - **R6-F1.17b**: adjacent Swift repeated assertions require a pinned-grammar upgrade; supported grouped forms remain verified. Next action, if broader syntax is desired: upgrade the grammar and add the source forms to invocation matrices.
        - **R5-F1.S12**: Swift specialized explicit initializer paths require a pinned-grammar upgrade. Next action: upgrade the grammar and verify qualified path/classification/coverage evidence.
        - **R7-OBS.1** (deferred, separate contract): Python future-import handling in the type-owner relation tree. The occurrence/import facts are verified; this feature preserves that separate resolver contract. Next action: add the relation-tree arm with a type-owner regression when implementing that resolver repair.
        - **R7-OBS.2** (deferred, separate contract): plain import reports reconstruct selected import syntax imperfectly. Binding records and required refs text/JSON are verified. Next action: extend import rendering with language-specific syntax and golden controls.
- Changed files in this pass (preexisting staged production/docs/query changes preserved):
        - `tree-hugger/lib/tests/l1/symbols_occurrences.rs`: shared matrix access, Java annotation argument row, and exact modifier/read controls (R7-F1/F2/F3).
        - `tree-hugger/lib/tests/l1/symbols_cache.rs`: complete marker inventory, every declaration shape and every Java qualified-name role through actual six-stage lookups (R7-F1/F2/F3).
        - `tree-hugger/lib/tests/l1/tree_file.rs`: complete Java public diagnostics-reference/import inventory (R7-F3.C5).
        - `tree-hugger/cli/tests/symbols/refs.rs`: complete Swift marker/selective-import and Java module-operand reports and call filtering (R7-F1.C5, R7-F2.C5, R7-F3.C6).
        - `tree-hugger/features/2026-10-08-scoped-references/implementation-log.md`: inventory, progress, reconciliation and completion metadata.
        - `tree-hugger/features/2026-10-08-scoped-references/review-7.md`: implementation metadata only; readiness judgment, findings and body preserved.
- Final verification after the last test edit:
        - `just test -j 4` in tree-hugger: **1,462 passed, 0 skipped**, 234.087 seconds; verifies all three findings, strict cache robustness, scope/summary/recursion and existing consumers.
        - `just lint` in tree-hugger: **passed**, library and CLI all-target Clippy with warnings denied; no formatter runs in this recipe.
        - `just build` in tree-hugger: **passed**, library and CLI.
        - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_declarations_in_tmux` in tree-hugger: **1 selected test passed**, 5.562 seconds; 391 unrelated tests filtered out. Backend proof: **tmux run=1, skip=0, panic=0**. Live refs rendering verifies R7-F1.C5/R7-F2.C5/R7-F3.C6 without taking window focus.
        - `just check-tier-coverage tree-hugger` at root: **0 stranded tests**; the new tests compile in declared library l1/CLI symbols binaries and have unmarked L1 names.
        - Metadata/ledger audit: no duplicate IDs, no missing review IDs, no pending required rows; review body and `ready: false` preserved.
- OS assessment: this cycle changes portable grammar-source regression tests and metadata only; no platform conditionals, path-comparison logic, process implementation or filesystem semantics changed. Final evidence is macOS; no cross-OS execution was needed for this repair, and no cross-OS pass is claimed.
- No production format/configuration reader changed; existing strict cache robustness matrices passed in L1. No performance measurement was requested or deferred; `deferred_perf_measurement: false`. No formatter, commit, feature completion move, or `just complete` was run. All subagents and verification commands have finished.
