---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-08-scoped-references/spec.md"
plan: "tree-hugger/features/2026-10-08-scoped-references/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
  - tree-hugger
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - tree-hugger/docs/references.md
skills_files_updated_during_phase_1: []
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
