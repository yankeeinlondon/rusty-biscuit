---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-07-cyclomatic-complexity/spec.md
plan: tree-hugger/features/2026-10-07-cyclomatic-complexity/plan.md
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
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/src/file/extract/complexity/rust.rs
  - tree-hugger/lib/src/file/extract/complexity/tests.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/src/file/extract/complexity/javascript.rs
  - tree-hugger/lib/src/file/extract/complexity/typescript.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_3:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_4:
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/reports.rs
docs_updated_during_phase_4:
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-schema-design.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
source_files_during_phase_5:
  - tree-hugger/cli/tests/symbols/reports.rs
docs_updated_during_phase_5:
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
docs_created_during_phase_5:
  - tree-hugger/docs/cyclomatic-complexity.md
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_code:
  - tree-hugger/lib/src/file/extract/complexity/mod.rs
  - tree-hugger/lib/src/file/extract/complexity/rust.rs
  - tree-hugger/lib/src/file/extract/complexity/javascript.rs
  - tree-hugger/lib/src/file/extract/complexity/typescript.rs
  - tree-hugger/lib/src/file/extract/complexity/tests.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/metrics.rs
  - tree-hugger/lib/src/file/extract/mod.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/query.rs
  - tree-hugger/lib/src/shared/schema_v2/facets.rs
  - tree-hugger/lib/src/shared/schema_v2/types.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/tests.rs
  - tree-hugger/lib/src/analysis/counters.rs
  - tree-hugger/lib/src/cache/mod.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/lib/tests/l1/symbols_metrics.rs
  - tree-hugger/lib/tests/l1/symbols_query.rs
  - tree-hugger/lib/tests/l1/symbols_adapter.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/l1/cache_tests.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/symbols/cache_robustness.rs
  - tree-hugger/cli/tests/symbols/wiring.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/cli/tests/fixtures/golden/symbols.tree
documentation:
  - tree-hugger/docs/cyclomatic-complexity.md
  - tree-hugger/docs/symbol-schema-design.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/docs/symbol-kinds-and-visibility.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/README.md
  - tree-hugger/lib/README.md
  - tree-hugger/cli/README.md
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/tree-hugger/api-reference.md
completed_phase: 5
implemented: true
---

# Implementation Log for 2026-10-07-cyclomatic-complexity (5 phases)

## Phase 1

Phase 1 is rulings, a prerequisite check and the grammar spike. It changes no
source code: the spike ran as a throwaway `lib/examples/` dump against the
pinned grammars, and that file was deleted afterwards.

### Prerequisite check (2026-10-05)

`2026-10-06-symbol-type-relations` **has landed** in this worktree:

- `MetricsFacet` (`lib/src/shared/schema_v2/facets.rs`) has `parameters`,
  `has_docs` and `lines`, each `#[serde(deserialize_with = "nullable")]`.
- Records, indexes and envelopes emit `SchemaVersion::V4_1`;
  `EXTRACTOR_REVISION` is `"25"`.
- That spec is `implemented: true` but still in review. Its review fixes are
  uncommitted in this worktree (`generics.rs`, `relations.rs`,
  `rust_impl.rs`, goldens, `cache_upgrade.rs`), so the revision may move
  again before Phase 2. Read both values from the tree at Phase 2 time.
- The next schema minor for this feature is therefore **V4_2**, and the
  extractor revision is the current one + 1 (26 if nothing else lands).

**Departure from the plan (body reader).** The plan says the engine traverses
"the callable body node already chosen by `body_node_for_declaration`". In
the landed code, `source.body_span` and `lines` come from
`classify.rs::callable_body_span` → `callable_body_extent`.
`body_node_for_declaration` is a separate reader (it also falls back to type
bodies) used for declaration/signature text cuts. The spec requires the
*same* body as `lines`, so Phase 2 must use `callable_body_extent`.

`callable_body_extent` returns a `TextSpan`, not a node:

- For Rust/JS/TS (and every `body`-field language), the body is one node:
  the `body` field, or for a C# `arrow_expression_clause` its expression.
- For Python and Lua, the body is a run of sibling children, not one node.

Recommended Phase 2 shape: have `callable_body_extent` return the body as
nodes (one node, or a first..last sibling run, or empty), and derive the span
from that. Both the span and the traversal then come from one reader, and no
second body discovery path is added.

### Rulings (author acceptance pending; see spec `human_review_items`)

The session is non-interactive, so nobody could accept these during Phase 1.
Recorded below are the plan's defaults (rules 1–4) plus the residual choices
the spike surfaced. Each lives in one rule-table entry and can be flipped
without touching the engine.

1. **Counting convention**: the spec's recommended option. `1 +` the sum of:
   - +1 for each `if` and else-if, conditional loop, ternary, short-circuit
     Boolean operator, non-default arm, separate guard and exception handler;
   - 0 for plain `else`, `finally`, unconditional loops and unconditional
     `return`/`break`/`continue`/`throw`;
   - grouped labels or patterns are one arm.
2. **Initial language set**: Rust, JavaScript and TypeScript. The other 13
   languages are `None` through the engine's no-rules path.
3. **Residual forms** (plan defaults):
   - `??` and logical assignment (`&&=`, `||=`, `??=`) are +1;
   - optional chaining `?.` and Rust `?` are 0 by design;
   - pattern alternatives (`A | B` in one arm) are 0, because grouped
     patterns are one arm.
   - Comprehensions do not exist in these three languages.
4. **Recognized-but-unsupported forms**: none in Rust, JS or TS. Every
   branching construct the pinned grammars produce is classified below. No
   form makes the metric `None`, so for these three languages only a parse
   error in the visited body does.

**New residual choices from the spike** (default recommended; author may flip):

| # | Choice | Recommended default | Alternative |
|---|---|---|---|
| R1 | Rust `match`, which is always exhaustive | arms − 1: the last arm is the default, whatever its pattern | count every arm except a bare `_` arm; an exhaustive match without `_` then counts 1 more than the equivalent `if`/`else` chain |
| R2 | Rust `let … else` | +1, a refutable pattern test with an early exit, like `if let` | 0, treated as an early exit |
| R3 | Rust `async { }` / labeled `'l: { }` / `unsafe { }` / `const { }` blocks inside a body | traversed, contributing to the enclosing function | `async` blocks as boundaries, like closures |
| R4 | Rust item-level `const`/`static` inside a body | boundaries, contributing 0: compile-time items, not per-call code | traversed |
| R5 | JS/TS object literal `{ p: a ? 1 : 2 }` | traversed (its values run inline); its methods, getters and setters are boundaries | the whole literal is a boundary, as an "anonymous owner" |
| R6 | TS `namespace N { }` inside a body | boundary, 0 | traversed |

### Grammar spike results

Pinned grammars: tree-sitter-rust 0.24.0, tree-sitter-javascript 0.25.0,
tree-sitter-typescript 0.23.2 (`LANGUAGE_TYPESCRIPT`; TSX shares the node
kinds). One fixture per language covered every inventory category; each
parsed with `has_error() == false`.

Answers to the spike's questions:

- **Operators**: every short-circuit operator is an anonymous token in the
  `operator` field of `binary_expression` (Rust/JS/TS) or
  `augmented_assignment_expression` (JS/TS), with one exception: Rust
  `let_chain` holds its `&&` as an unfielded anonymous child between
  `let_condition`s.
- **Else-if**: it has no node of its own; it is `else_clause` → nested
  `if_expression`/`if_statement`, so counting each `if` node once counts
  else-if correctly and plain `else` is 0.
- **Guards**: a Rust guard is the `condition` field of `match_pattern`. JS
  and TS have no guards.
- **Grouped labels**: JS/TS `case 1: case 2:` are separate `switch_case`
  nodes, and only the last one of a group has a `body` field. A Rust or-pattern is
  one `match_arm` with an `or_pattern`.
- **Unconditional loops**: Rust `loop_expression`; JS/TS `for (;;)` has
  `condition: empty_statement`.
- **Optional chaining**: `?.` is an `optional_chain` node, not an operator.
- **TS type-level conditionals**: `T extends U ? X : Y` is `conditional_type`,
  a different kind from `ternary_expression`, so it never matches.
- **Errors**: an unclosed delimiter inside a body shows as `ERROR`/`MISSING`
  in that body only. A malformed nested `fn inner` leaves the outer function
  and its siblings error-free in the visited region; a malformed JS function
  leaves the next function intact. An unclosed Rust function brace can also
  swallow the declaration into a root `ERROR`, so no record exists to measure.
- **Macros**: Rust `macro_invocation` holds a `token_tree` whose `if`, `&&`
  and similar items are plain tokens. The tree is opaque, so the walk prunes
  it and the macro contributes 0.

### Rule table: Rust (tree-sitter-rust 0.24.0)

Traversal starts at the `body` field of `function_item`, which is a `block`.

| Category | Match (kind / field / token) | Contribution |
|---|---|---|
| `if`, else-if, `if let` | `if_expression` (else-if is `else_clause` → `if_expression`) | +1 each |
| plain `else` | `else_clause` | 0 |
| let chain `if let A = x && b` | `&&` token child of `let_chain` | +1 per `&&` |
| `let … else` | `let_declaration` with an `alternative` field | +1 (R2) |
| `while`, `while let` | `while_expression` | +1 |
| `for` | `for_expression` | +1 |
| `loop` | `loop_expression` | 0 |
| `&&`, `\|\|` | `binary_expression` whose `operator` is `&&` or `\|\|` | +1 each |
| `&=`, `\|=`, `&`, `\|` | non-short-circuit operators | 0 |
| `match` arms | `match_arm` in `match_block` | +1 per arm except the last (R1) |
| or-pattern in an arm | `or_pattern` | 0, grouped patterns are one arm |
| guard | `match_pattern` with a `condition` field | +1 |
| `?` | `try_expression` | 0 by design |
| `return`, `break`, `continue`, labeled `break` | `*_expression` | 0 |
| `async`/labeled/`unsafe`/`const` blocks | `async_block`, `block`, `unsafe_block`, `const_block` | traversed (R3) |
| macro calls | `macro_invocation` (its `token_tree`) | pruned, 0 (opaque) |
| closures | `closure_expression`, including an expression body | boundary, pruned |
| nested items | `function_item`, `function_signature_item`, `struct_item`, `enum_item`, `union_item`, `trait_item`, `impl_item`, `mod_item`, `type_item`, `macro_definition`, `foreign_mod_item` | boundary, pruned |
| item `const`/`static` | `const_item`, `static_item` | boundary, pruned (R4) |
| exception handling | none in Rust | n/a |
| comprehensions, ternary, null-coalescing | none in Rust | n/a |

Examples with expected totals (each body parses cleanly):

| Source body | Total |
|---|---|
| `fn f() {}` | 1 |
| `fn f(a: bool) { if a {} else if !a {} else {} }` | 3 |
| `fn f(a: bool, b: bool) -> bool { a && b \|\| !a }` | 3 |
| `fn f(x: Option<i32>) -> i32 { match x { Some(1) \| Some(2) => 1, Some(n) if n > 3 => 2, _ => 3 } }` | 1 + 2 arms + 1 guard = 4 |
| `fn f(x: Option<i32>) { let Some(y) = x else { return }; }` | 2 |
| `fn f(x: Option<i32>, a: bool) { if let Some(v) = x && a {} }` | 3 |
| `fn f() { loop { break; } }` | 1 |
| `fn f(a: bool) { while a {} for _ in 0..3 {} }` | 3 |
| `fn f() -> Result<(), E> { g()?; Ok(()) }` | 1 |
| `fn f(a: bool) { let c = \|k: bool\| if k { 1 } else { 2 }; fn inner(a: bool) { if a {} if a {} } }` | outer 1, `inner` 3 |
| `fn f() { m!(if a {}); /* if a && b */ let s = "if a"; }` | 1 |
| `fn f(a: bool) { struct S([u8; if true { 1 } else { 2 }]); }` | 1 |

### Rule table: JavaScript (tree-sitter-javascript 0.25.0)

Traversal starts at the `body` field of the callable: a `statement_block`, or
an arrow function's expression.

| Category | Match | Contribution |
|---|---|---|
| `if`, else-if | `if_statement` | +1 each |
| plain `else` | `else_clause` | 0 |
| `for (…; cond; …)` | `for_statement` whose `condition` is not `empty_statement` | +1 |
| `for (;;)` | `for_statement` with `condition: empty_statement` | 0 |
| `for…in`, `for…of` | `for_in_statement` | +1 |
| `while`, `do…while` | `while_statement`, `do_statement` | +1 |
| ternary | `ternary_expression` | +1 |
| `&&`, `\|\|`, `??` | `binary_expression` `operator` | +1 each |
| `&&=`, `\|\|=`, `??=` | `augmented_assignment_expression` `operator` | +1 each |
| other compound assignment | `+=` and similar | 0 |
| `?.` | `optional_chain` | 0 by design |
| `switch` arms | `switch_case` that has a `body`, or that is the last child before `}` | +1 |
| grouped labels `case 1: case 2:` | a bodyless `switch_case` followed by another case or `default` | 0 (one arm) |
| `default` | `switch_default` | 0 |
| `catch` | `catch_clause` | +1 |
| `finally` | `finally_clause` | 0 |
| `return`, `break`, `continue`, `throw`, labels | statements | 0 |
| template substitutions | `template_substitution` | traversed |
| object literal values | `object`, `pair` | traversed (R5) |
| nested callables | `function_declaration`, `generator_function_declaration`, `function_expression`, `generator_function`, `arrow_function`, `method_definition` | boundary, pruned |
| nested types | `class_declaration`, `class` (expression) | boundary, pruned (field initializers excluded) |
| comprehensions, guards, pattern alternatives | none | n/a |

Examples with expected totals:

| Source | Total |
|---|---|
| `function f() {}` | 1 |
| `function f(a, b) { if (a) {} else if (b) {} else {} }` | 3 |
| `function f(a, b, c, d) { return (a && b \|\| c) ?? d; }` | 4 (unparenthesized `??` mixed with `\|\|` is a JS syntax error that tree-sitter accepts; fixtures parenthesize) |
| `function f(a, b) { a &&= b; a \|\|= b; a ??= b; a += 1; }` | 4 |
| `function f(o) { return o?.x?.y; }` | 1 |
| `function f(a) { switch (a) { case 1: case 2: break; case 3: return; default: break; } }` | 3 |
| `function f() { try { g(); } catch (e) {} finally {} }` | 2 |
| `function f() { for (;;) { break; } }` | 1 |
| `function f(o) { for (const k in o) {} for (const v of o) {} while (o) {} do {} while (o); }` | 5 |
| `const f = (k) => k ? 1 : 2;` | 2 (expression body) |
| `function f(a, b = a ? 1 : 2) { /* if (a) */ return "a ? b : c"; }` | 1 |
| `function f(a) { function inner() { if (a) {} } class C { x = a ? 1 : 2; m() { if (a) {} } } }` | outer 1, `inner` 2, `m` 2 |
| `function f(a) { return { p: a ? 1 : 2, q() { if (a) {} } }; }` | 2 (R5), `q` is pruned |

### Rule table: TypeScript (tree-sitter-typescript 0.23.2)

Every JavaScript rule above applies unchanged. TypeScript adds:

| Category | Match | Contribution |
|---|---|---|
| type-level conditional `T extends U ? X : Y` | `conditional_type` | 0, a type and not a ternary |
| `as`, `satisfies`, non-null `!` | `as_expression`, `satisfies_expression`, `non_null_expression` | 0, traversed |
| type annotations in a body | `type_annotation` | 0, traversed (no decision kinds occur there) |
| nested types | `interface_declaration`, `type_alias_declaration`, `enum_declaration`, `abstract_class_declaration`, `class_declaration`, `class` | boundary, pruned |
| nested namespace | `internal_module`, `module` | boundary, pruned (R6) |
| bodyless overloads and signatures | `function_signature`, `method_signature`, `abstract_method_signature` | boundary; the record itself is bodyless, so `None` |
| parameter properties `constructor(private a)` | in `formal_parameters`, outside the body | not visited |

Examples with expected totals:

| Source | Total |
|---|---|
| `function f<T>(a: boolean): void { let x: T extends string ? 1 : 2; }` | 1 |
| `function f(a: any, b: number) { return (a as number) ?? b!; }` | 2 |
| `const f = async (k: number): Promise<number> => k > 1 ? 1 : 2;` | 2 |
| `function f(a: boolean) { interface I { m(): void } enum E { A = 1 } abstract class K { n() { if (a) {} } } }` | outer 1, `n` 2 |
| `function over(a: string): void;` | `None` (bodyless) |
| `class C { constructor(private a: boolean) { if (a) {} } }` | constructor 2 |
| `function f(b: number) { switch (b) { case 1: default: break; } }` | 1 (`case 1` grouped with `default`) |

### Robustness matrix (deserialization of `metrics.cyclomatic`)

The plan defines the matrix and Phase 4 implements it. No code reads the key
yet, so no test was added in Phase 1.

### Test mapping for Phase 1

Phase 1 changes no behavior, so it adds no tests. The executable
expectations for the tables above are written in Phase 2 (Rust) and Phase 3
(JS/TS), from the example rows here.

### Gates run (2026-10-05)

- `just lint` (tree-hugger): passes.
- `just test` (tree-hugger), then re-run with `--no-fail-fast`: 1080 tests,
  1078 passed, **2 failed**:
  `tree-hugger-cli::symbols golden::json_symbols_over_multi_match_the_golden`
  and `golden::jsonl_symbols_over_multi_match_the_golden`.
- **These failures predate Phase 1.** The golden files
  (`cli/tests/fixtures/golden/symbols.json{,l}`) and the extractor sources
  carry uncommitted review fixes for `2026-10-06-symbol-type-relations`, and
  another session is still editing this worktree (an untracked
  `lib/examples/zz_relations_probe.rs` appeared during this phase). Phase 1
  changed no source, test or golden file, so it neither caused these failures
  nor was the place to fix them. They must be green before Phase 2 regenerates
  the goldens for the new key.
- No OS-specific risk: Phase 1 changed no code, and grammar shape does not
  depend on the OS, so no cross-check was run.
- Skill: no change needed. No behavior changed, and the rule tables are not
  current behavior until Phase 2/3 implement them.

## Phase 2

Phase 2 adds the counting engine, the Rust rule table, the `cyclomatic` key,
schema 4.3 and extractor revision 27. The drafted Phase 1 rulings (rules 1–4,
R1–R4) are implemented as written; the author's acceptance is still pending
(spec `human_review`), and each ruling is one entry in
`complexity/rust.rs`.

### Starting state (2026-10-05)

- The prerequisite moved again after Phase 1: HEAD emitted **V4_2** with
  `EXTRACTOR_REVISION` **26**. This feature therefore uses **V4_3** and
  **27** (not V4_2/26 as Phase 1 projected).
- **Another session was editing this worktree concurrently** (review fixes for
  `2026-10-06-symbol-type-relations`: `relations.rs`, `tree_file/relations.rs`,
  `render.rs`, `symbols_relations.rs`, `cache_robustness.rs`,
  `cache_upgrade.rs`, the skill). Its half-written `symbols_relations.rs` kept
  the L1 binary from compiling for part of this phase. I made only surgical,
  in-place edits to the files we both touched and did not change its code.

### What changed

- **Body reader, one path** (`classify.rs`). `callable_body_extent` →
  `callable_body`, which returns `CallableBody` (`Node`, a Python/Lua
  sibling `Run { first, last }`, or Lua's `EmptyAt(end)`).
  `CallableBody::span` derives the stored span (same trimming as before), and
  `callable_body_span` was removed. `records_from` reads the body once and
  hands the same value to `symbol_metrics`, so `source.body_span`, `lines`
  and `cyclomatic` cannot read different bodies. This follows the Phase 1
  departure note (the plan named `body_node_for_declaration`, but that
  reader is for declaration-text cuts).
- **Engine** (`extract/complexity/mod.rs`). A `Rules` table has
  `boundaries` (pruned, never error-checked), `opaque` (pruned but
  error-checked with `has_error`: an error in a macro's token tree is still
  in the counted body) and `decisions` by node kind with `Counts::{Always,
  WithField, Operator, Tokens, UnlessLast}`. The walk uses one `TreeCursor`
  and a depth counter, with no recursion. The tally uses `checked_add`, so an
  overflow gives `None`. An `is_error()`/`is_missing()` node in the visited
  region also gives `None`. A language with no table gives `None`
  (`rules_for`), with no fallback. A `Run` body walks each sibling from
  `first` to `last`.
- **Rust table** (`extract/complexity/rust.rs`) matches the Phase 1 table.
  It counts `if_expression`, `while_expression`, `for_expression`,
  `binary_expression` `&&`/`||`, `let_chain` `&&` tokens,
  `let_declaration` with `alternative` (R2), `match_arm` unless last (R1)
  and `match_pattern` with `condition`. Boundaries are closures, nested
  items, and `const`/`static` items (R4). `token_tree` is opaque. `async`,
  `unsafe`, labeled and `const` blocks are traversed (R3).
- **Facet** (`facets.rs`): `cyclomatic: Option<u32>` with
  `deserialize_with = "nullable"` (required, nullable). Its doc says `None`
  means unavailable and is never zero.
- **Wire-up** (`metrics.rs`): `symbol_metrics` takes the
  `Option<CallableBody>` and computes `lines` from its span and
  `cyclomatic` from its nodes, only for Function/Method/Constructor.
  `symbol_metrics` runs only under `Capability::Metrics`, so a
  declarations-only query never traverses. `analysis/coverage.rs` still
  drops the whole facet when Metrics is not requested (checked; unchanged).
- **Schema 4.3** (`types.rs`): added `V4_3` and **`SchemaVersion::CURRENT`**
  (= `V4_3`). Every emitter and validator that hard-coded `V4_2` (cache
  entry/record checks, analysis, god-files, CLI envelopes, JSONL, adapters)
  now uses `CURRENT`, so the next bump changes one line. This is a small
  scope addition beyond the plan, made to avoid repeating the ~25-site edit
  on every bump. Tests that pin the wire value keep JSON literals
  (`{"major":4,"minor":3}`).
- **`EXTRACTOR_REVISION` = "27"** (`file/mod.rs`).

### Comment and doc drift found and fixed

- `docs/symbol-schema-design.md` still named 4.1 as current, and its version
  history had no 4.2 row (drift left by the prerequisite's 4.2 bump). Fixed:
  added 4.2 and 4.3 rows, the status line, the facet key list, the `null`
  meaning and the JSON examples.
- `docs/cache-design.md` said `V4_1`, `rev.24` and "exactly 4.1", and listed
  three metric keys. Changed to `CURRENT` (4.3), `rev.27` and four keys, with
  the `cyclomatic` u32 shapes.
- `cli/tests/symbols/reports.rs` and `report.rs` pinned `"minor":1`; they
  were already failing at HEAD after the 4.2 bump. Updated to 4.3.
- `lib/tests/l1/symbols_cache.rs` listed `{4,2}` as both current and
  incompatible (left half-updated). Current is now `{4,3}`, and the
  incompatible list is 2.0, 3.0, 4.0, 4.1, 4.2, 4.4, 5.0.
- `Capability::Metrics` doc (`query.rs`) and the `extract/mod.rs` module
  doc (re-exports) were updated for the new key and module.
- Skill: `callable_body_span` no longer exists. Updated SKILL.md (the body
  reader, the engine, the test tables, the revision list) and
  `api-reference.md` (`MetricsFacet`, schema 4.3).

### Goldens

`symbols.json`, `symbols.jsonl` and `symbols.tree` were regenerated
(`HUG_UPDATE_GOLDEN=1`). Review of the diff:

- `schema_version` minor 1 → 3 (286 lines) and `extractor_version`
  `rev.24` → `rev.27` (190 lines). The goldens were already stale at HEAD
  (the two golden failures recorded in Phase 1).
- **Not from this phase:** one record gains
  `"relations": {"implements": [{"name": "Named", …}]}`, and the tree golden
  shows `public class Tag { name: String } implements Named`. Both come from
  the concurrent session's uncommitted mixed-base-list relation work. Whoever
  commits should keep those golden lines with that work.
- No `cyclomatic` line appears, which is expected: `--json symbols` and
  `--jsonl` do not request Metrics. The full-index (`--json-full`) checks are
  Phase 4.

### Requirement → test mapping

| Requirement | Test |
|---|---|
| Rust rule table, exact totals per rule (if/else-if, `&&`/`\|\|`, bitwise 0, arms − 1, two-arm `match` = `if`/`else`, guard, guard + `&&`, `let … else`, let chain, `loop` 0, `while`/`for`/`while let`, `?` 0) | `symbols_metrics::rust_cyclomatic_follows_the_counting_rules` (`RUST_CYCLOMATIC`) |
| Zero-contribution look-alikes: macro tokens, comments, strings, attribute args, return-type const expr, items (`struct`/`const`/`static`) in the body | same test, rows `look_alikes`, `return_type`, `items` |
| Nested callable/closure pruned; nested fn has its own total | same test, row `outer`/`inner` |
| `async`/`unsafe`/labeled blocks traversed (R3) | same test, row `blocks` |
| Methods and constructors measured; bodyless trait method `None`; non-callables `None` | same test, last row plus the per-row non-callable sweep |
| Narrow (metrics-only) and complete index agree; LF = CRLF; fixtures parse cleanly | same test (both query paths × both endings, `syntax_diagnostics` empty) |
| Malformed counted body → `None`; malformed nested body leaves outer and sibling measured; other metrics intact | `symbols_metrics::rust_cyclomatic_is_unavailable_only_for_a_malformed_counted_body`; engine level `complexity::tests::only_errors_in_the_counted_body_make_the_total_unavailable` (adds opaque macro error → `None`, closure error ignored) |
| Key present (number / `null`) inside the facet, absent without Metrics | `symbols_metrics::cyclomatic_key_is_present_null_or_absent_with_its_facet`; `lines_key_is_present_null_or_absent_with_its_facet` (Java: `null`) |
| Unsupported languages `None` for every record, non-callables `None` in all 16 languages | `symbols_metrics::metrics_match_each_fixture_function` (`CYCLOMATIC_LANGUAGES`); `complexity::tests::languages_without_reviewed_rules_have_no_value` |
| No recursion per syntax node | `complexity::tests::deep_nesting_is_counted_without_recursion` (20 000 nested `if`s on a 256 KiB stack → 20 001) |
| Checked arithmetic, no wrap | `complexity::tests::a_total_beyond_u32_is_unavailable_not_wrapped` |
| Python/Lua sibling-run body walked first → last only | `complexity::tests::a_sibling_run_body_is_walked_from_first_to_last` (test-only Python rules) |
| Declarations-only query does no metrics work; metrics query has value | `symbols_query::flags_and_metrics_run_with_the_declarations_only` (now asserts `cyclomatic: Some(1)`) |
| Record reader robustness matrix for `metrics.cyclomatic` (control, absent, null, `"3"`, `[3]`, `true`, `-1`, `1.5`, `4294967296`, `{}`, `[]`, duplicate key, trailing content, `u32::MAX` reads) | `symbols_adapter::new_record_fields_read_per_shape` |
| Older schema entries (4.2 without the key) never served, recomputed under both keys | `cache_upgrade::entry_from_extractor_revision_twenty_six_is_not_served_after_the_schema_bump` (shares `assert_schema_bump_entry_is_not_served` with the revision-17 case) |
| Cache refuses other schema minors (4.1, 4.2, 4.4) | `symbols_cache::incompatible_record_schema_versions_are_misses_never_served_or_widened` |
| Envelopes carry 4.3 | `wiring::json_symbols_carry_visibility_and_current_schema`, `report::the_json_envelope_…`, `reports::json_lines_…`, `cache_robustness` record `schema_version` column |

A mutation check confirmed the Rust table test bites: switching `match_arm`
to `Always` failed `rust_cyclomatic_follows_the_counting_rules` (`arms`:
5 ≠ 4). The rule was then restored.

All new tests are L1. Their names carry no tier marker. The unit tests are
in the lib target, and `symbols_metrics` is in the declared `tests/l1`
binary. No feature gate is involved.

Left for later phases, as planned: the cache-level `cyclomatic` column in
`cache_robustness` (Phase 4; its fixture is Python, which has no rules, so
it needs a Rust target), the 16-row `cyclomatic` matrix column (Phase 3),
work-counter tests and `--json-full` CLI checks (Phase 4), and the
counting-rules doc (Phase 5).

### Gates run (2026-10-05)

- `cargo nextest run -p tree-hugger -p tree-hugger-cli --no-fail-fast`:
  1092/1092 passed.
- `just test` (tree-hugger): 1092 tests, 1092 passed.
- `just lint` (tree-hugger): passes.
- An intermediate run had 2 failures in the concurrent session's new
  `cache_upgrade` revision-24/25 tests: the sentinel was the probed record.
  That session fixed them before the final runs. They were not caused by
  this phase.
- OS: no cross-check run. The engine reads only the parse tree. It has no
  paths, I/O or platform APIs. Grammar shape does not depend on the OS. The
  only platform-sensitive item is the deep-nesting test's explicit 256 KiB
  thread stack, which `std::thread::Builder` supports on every target. CI
  covers Linux and macOS on the pull request, plus Windows on `main`.

## Phase 3

Phase 3 adds the JavaScript and TypeScript rule tables, the 16-row
`cyclomatic` matrix column and extractor revision 28. The author has still
not accepted the convention or R1–R6 (spec `human_review`). This phase
implements the drafted Phase 1 tables exactly as written, the option the
review item recommends. Each choice is one table entry, so it can still
be flipped.

### Starting state (2026-10-05)

- Phase 2 is **not committed**: HEAD is still the Phase 1 close, and the
  Phase 2 changes plus the concurrent relations-review edits are in the
  working tree. Phase 3 edits sit on top of them.
- At the start the tree emitted schema 4.3 and revision 27, as Phase 2
  left it.

### What changed

- **Engine** (`complexity/mod.rs`): two new `Counts` variants, as the Phase 2
  handoff asked, rather than special cases in `visit`:
  - `UnlessFieldKind { field, kind }`: +1 unless the field holds that kind.
    JS/TS `for (;;)` and `for (init;; step)` both have
    `condition: empty_statement`.
  - `Arm(field)`: +1 for an arm with that field, or for one that no
    non-comment named sibling follows. Grouped `case 1: case 2:` is one arm,
    `case 1: default:` is 0, and a bodyless last `case 2: }` is one arm.
  - `rules_for` gains JavaScript and TypeScript.
- **JavaScript** (`complexity/javascript.rs`) matches the Phase 1 table:
  - Counted: `if_statement`, `for_statement` (unless `for (;;)`),
    `for_in_statement` (for…in and for…of), `while`/`do`, ternary,
    `&&`/`||`/`??`, `&&=`/`||=`/`??=`, `switch_case` arms and `catch_clause`.
  - Boundaries: function declarations and expressions, generators, arrows,
    `method_definition` (so object literal methods are pruned while object
    literal values are counted, R5), and `class_declaration`/`class` (which
    excludes field initializers).
  - JSX `{a && <b/>}` counts through the ordinary `binary_expression` rule.
- **TypeScript** (`complexity/typescript.rs`) reuses
  `javascript::DECISIONS`. Its boundary list adds `function_signature`,
  `method_signature`, `abstract_method_signature`,
  `abstract_class_declaration`, `interface_declaration`,
  `type_alias_declaration`, `enum_declaration`, `internal_module` and
  `module` (R6). `conditional_type`, `as`, `satisfies` and `!` match no
  rule, so they contribute 0. This is one file per language, with the
  decision list shared rather than duplicated.
- **`EXTRACTOR_REVISION` = "28".** **Departure from plan rule 7** ("not per
  language in this plan's own commits"): the spec (line 315) says any later
  change to supported rules bumps the revision. The repo rule (`file/mod.rs`
  doc, skill) says repairs on an unreleased branch bump too. A local cache
  written at revision 27 would otherwise keep serving `cyclomatic: null` for
  JS/TS functions, and the new cache-upgrade test proves that payload is
  servable under the current key. The Phase 2 handoff asked for the same
  bump.

### Requirement → test mapping

| Requirement | Test |
|---|---|
| JS rule table, exact totals per rule: if/else-if, `&&`/`\|\|`/`??`, `&&=`/`\|\|=`/`??=`, other compound and bitwise ops 0, `?.` 0, switch arms (grouped, with `default`, bodyless last, comment between labels), `catch` with and without a binding, `finally` 0, `for(;;)` and `for(init;;step)` 0, counted `for`, for-in/of, `while`, `do`, labeled loop + `continue` | `symbols_metrics::javascript_cyclomatic_follows_the_counting_rules` (`JAVASCRIPT_CYCLOMATIC`) |
| Empty, block and expression bodies; `async` arrow; arrow returning arrow (outer 1) | same, rows `ternary`/`constant`/`block`/`curried`, `empty` |
| Nested named and anonymous callables, nested class with a field initializer, object-literal values counted and methods pruned (R5), constructors/getters/static methods measured | same, rows `outer`, `literal`, `K` |
| Zero-contribution look-alikes: comments, strings, template text, parameter defaults; a template substitution counts | same, rows `lookAlikes`, `templated` |
| JSX `&&` | same, row `view` |
| TS: everything JS has plus `conditional_type` (in the body and the return type), `as`/`satisfies`/`!` 0; interface/alias/enum/abstract class/namespace boundaries (R6); parameter-property constructor; bodyless abstract method `None` | `symbols_metrics::typescript_cyclomatic_follows_the_counting_rules` (`TYPESCRIPT_CYCLOMATIC`) |
| Each row parses cleanly; LF = CRLF; metrics-only = complete index; non-callables `None` | `assert_cyclomatic_rows` (shared by the three language tests) |
| Malformed counted body → `None` with the other metrics intact; malformed nested body leaves outer and sibling measured | `symbols_metrics::cyclomatic_is_unavailable_only_for_a_malformed_counted_body` (`MALFORMED_BODIES`: Rust, JS, TS; renamed from the Rust-only test) |
| Inner callable left out by the query: the outer total is unchanged | `symbols_metrics::an_outer_total_does_not_depend_on_selecting_its_nested_callables` (Rust, JS, TS; `scopes([Item])` against the complete index) |
| 16-row matrix: `cyclomatic` column, Measured for Rust/JS/TS with fixture values, `NO_CYCLOMATIC` ("no reviewed cyclomatic counting rules") for the other 13; measured ⇔ callable with a body in a reviewed language | `symbols_metrics::metrics_match_each_fixture_function` (`METRICS_MATRIX`), `symbols_coverage::every_language_has_one_row_in_each_report_matrix` |
| Unsupported languages go through the no-rules path, with no fallback | `complexity::tests::languages_without_reviewed_rules_have_no_value` (Go), plus the matrix sweep |
| Revision-27 entry (JS/TS `cyclomatic: null`) is not served after the bump; the same payload under the current key is servable (control) | `cache_upgrade::entry_from_extractor_revision_twenty_seven_is_not_served_after_the_ecmascript_rules` (JS and TS) |

**Mutation checks** (each was reverted afterwards). Every one failed the
expected test:

- `Arm` → `Always`: JS and TS fail.
- `for_statement` → `Always`: JS and TS fail.
- `??` dropped: JS and TS fail.
- `catch_clause` dropped: JS and TS fail.
- `class_declaration` dropped from the JS boundaries: JS fails.
- `method_definition` dropped from the JS boundaries: JS fails.
- `internal_module`, `enum_declaration` or `arrow_function` dropped from the
  TS boundaries: TS fails.

**Extractor limits found while writing rows.** These are not counting
issues, and the rows were removed. JS generator declarations
(`function* g`), function expressions bound to a `const`, and TS
`declare function` produce no Function record, so they cannot be rows.
That is recorded in the skill.

All new tests are L1, in the declared `tests/l1` binary (`symbols_metrics`)
and the CLI `symbols` binary (`cache_upgrade`). No name carries a tier
marker, and no feature gate is involved.

### Goldens

`symbols.json` and `symbols.jsonl` were regenerated (`HUG_UPDATE_GOLDEN=1`).
I compared them with `sed 's/rev\.27/rev.28/g'` of the previous files: the
only differences are the revision strings. No `cyclomatic` key appears,
because those reports do not request Metrics.

### Docs and comment drift

- `docs/symbol-schema-design.md`: said "every language but Rust today" and
  listed only Rust decisions. It now lists the JS/TS decisions and names all
  three languages. The full counting-rules doc is Phase 5.
- `docs/cache-design.md`: `rev.27` → `rev.28`.
- `symbols_metrics.rs` module doc: it named only `RUST_CYCLOMATIC` and three
  metrics. Updated.
- Skill: the table list, the `Counts` variants, the "only Rust has one"
  statement, the test names and the revision list (27 is now stale, 28 is
  current).

### Gates run (2026-10-05)

- `just test` (tree-hugger): 1096 tests, 1096 passed.
- `just lint` (tree-hugger): passes.
- OS: no cross-check was run. Phase 3 adds only rule tables over the parse
  tree and tests that write temp files through existing helpers (CRLF is
  covered explicitly). No path, I/O or platform API is new. CI covers Linux
  and macOS on the pull request and Windows on `main`.


## Phase 4

Phase 4 adds query, cache, adapter and CLI validation for `cyclomatic`, plus
one focused work counter. It changes no counting rule, schema or output, so
schema stays 4.3 and `EXTRACTOR_REVISION` stays "28" (no bump). The author
has still not accepted the convention or R1–R6 (spec `human_review`).

### Starting state (2026-10-05)

The worktree was clean. Phases 2 and 3 and the relations-review work are now
committed (`83bf8089d`, `3a67217b6`), so this phase's diff is only its own
work.

### What changed

- **`WorkCounters::complexity_walks`** (`analysis/counters.rs`), incremented
  in `complexity::cyclomatic` after `rules_for` succeeds. The plan allows a
  focused counter "only if the existing counters cannot observe it". They
  cannot: under Metrics, a body walk adds no parse, capture or link. It
  counts one walk per callable body of a language with rules. So it is 0
  without Metrics and 0 for a language without rules.
- **No production behavior change otherwise.** The CLI requests Metrics only
  for `--json-full` (complete index) and `--stats`, as before. `god_files`
  does not read metrics (grep: no `metrics`/`cyclomatic` in
  `lib/src/god_files`), so its scores and exit codes cannot change.

### Requirement → test mapping

| Requirement | Test |
|---|---|
| Narrow = complete query; LF = CRLF; outer total with inner callables omitted; malformed bodies | Already covered in Phases 2–3 (`assert_cyclomatic_rows`, `an_outer_total_does_not_depend_on_selecting_its_nested_callables`, `cyclomatic_is_unavailable_only_for_a_malformed_counted_body`); not duplicated |
| Metrics absent when unrequested; no complexity walk for declarations-only, Flags, or every capability but Metrics; one walk per measured callable under Metrics and the complete index; none for Go (no rules); bodyless declarations `None`; other metrics intact when `cyclomatic` is `None` (Go: `parameters`/`has_docs`/`lines` exact) | **new** `symbols_query::complexity_is_walked_only_for_metrics_once_per_measured_callable` (Rust, JS, TS, Go via `COMPLEXITY_SOURCES`) |
| Symbol IDs and qualified names are unchanged by whether complexity ran | same test (identical `(id, qualified_name)` lists across all five queries) |
| Widening a declarations-only entry to Metrics parses once and walks each measured callable once; the entry stores each value, `null` included; a warm Metrics request (and the narrower declarations-only one) is a `CacheLookup::Hit` with zero work; a language without rules caches `null` (no perpetual recompute) | **new** `symbols_cache::complexity_is_stored_once_and_served_on_warm_hits` |
| Generic widening/projection/warm-hit by Metrics across every fixture | existing `widening_capabilities_runs_only_the_missing_passes` (Go row = unavailable language), `projected_results_*`, `every_fixture_is_a_complete_warm_hit_*` (their `WorkCounters::default()` checks now also cover `complexity_walks`) |
| Cache robustness matrix for `metrics.cyclomatic` (control, absent, null, `"123"`, one/every wrong element, empty `[]`, duplicate key, trailing content) | **new column** `metrics.cyclomatic` in `cache_robustness::persisted_symbol_fields_are_served_or_recomputed_per_shape` |
| Wrong numbers and types beyond the matrix: `3.5`, `-1`, `4294967296`, `true`, `{}` rejected; `4294967295` served (for both `lines` and `cyclomatic`) | `cache_robustness::metrics_counts_serve_only_a_u32` (renamed from `metrics_lines_serves_only_a_u32_count` and extended) |
| Record-reader matrix (library) | existing `symbols_adapter::new_record_fields_read_per_shape` (Phase 2) |
| Previous schema (4.2) and revision (26, 27) entries recomputed, never migrated | existing `cache_upgrade` tests (Phases 2–3); no new revision this phase |
| The older-model adapter stays lossy and never invents complexity | `symbols_adapter::record_only_declaration_facts_are_lost_through_the_older_model` now also requires that some source *had* a `cyclomatic` value that the round trip dropped |
| CLI: `--json-full` carries the key (number / `null`, never `0`) cold = warm = uncached; `--json` and `--jsonl` listings compute no metrics; `--stats` (terminal and JSON) adds no complexity statistic | **new** `reports::cyclomatic_is_reported_only_where_metrics_already_are` |
| Goldens limited to key and schema | Not regenerated: `golden::*` pass unchanged. No schema or revision change this phase, and `--json`/`--jsonl` do not request Metrics, so no key appears |

**Mutation check.** Moving `count_complexity_walk()` before `rules_for(..)?`
(which counts a walk for a language without rules) failed both new library
tests on their Go cells. The change was reverted.

### Docs and skill

- `docs/cache-design.md`: the `u32` sentence now lists `true`, `{}`, `[]`
  and the served `4294967295`, and the Tests table has a row for the new
  cache test.
- `docs/symbol-schema-design.md`: the work-counter paragraph names
  complexity walks.
- Skill: `api-reference.md` (Work Counters) and SKILL.md (new tests, the
  counter and CLI exposure).
- The old test name appears only in the closed
  `2026-10-06-symbol-type-relations` log. That is a snapshot, so it was left
  as is.

### Placement

All new tests are L1, and none carries a tier marker in its path.
`symbols_query`, `symbols_cache` and `symbols_adapter` are in the declared
`lib/tests/l1` binary. `reports` and `cache_robustness` are in the CLI
`symbols` binary. No feature gate is involved.

### Gates run (2026-10-05)

- `just test` (tree-hugger): 1099 tests, 1099 passed.
- `just test-l2` (tree-hugger): 6 passed (tmux, WezTerm and Kitty for
  declarations and god-files).
- `just lint` (tree-hugger): passes after the test's tuple type was given an
  alias (`ComplexitySource`) for `clippy::type_complexity`.
- OS: no cross-check was run. The new counter is an atomic, and the new tests
  use temp files and the existing `Scratch` harness. Nothing touches paths or
  platform APIs. CI covers Linux and macOS on the pull request and Windows on
  `main`.

## Phase 5

Phase 5 publishes the counting rules and brings the docs, READMEs and skill
up to current behavior. It changes no counting rule, schema or production
code. Schema stays 4.3 and `EXTRACTOR_REVISION` stays "28".

### Starting state (2026-10-05)

Phase 4's work was still uncommitted in the worktree (the `M` files in
`git status`). Phase 5 edits were made on top of it.

### What changed

- **New page `tree-hugger/docs/cyclomatic-complexity.md`**: written for a
  developer new to the repository. It covers how to get the value (`hug
  --json-full` with a `jq` line, and `Capability::Metrics`), how to read a
  number, `null` or an absent facet, and a Mermaid flow of the walk. It has
  the shared rules table, then one table per language (Rust, JavaScript,
  TypeScript). Each row shows its contribution and a source → total
  example. All examples come from the `*_CYCLOMATIC` rows in
  `symbols_metrics.rs`, plus one end-to-end `hug` run on a TS sample
  (`classify` → 4). The page also covers nested callables and types, a
  `null` table, and the syntax-only limits: no macro expansion, no
  conditional compilation, no reachability, no callees, embedded languages
  treated as text, and nested type initializers counted in no function. It
  does not name or link this feature.
- `docs/symbol-schema-design.md`, `docs/symbol-kinds-and-visibility.md`
  (Metrics table: a `cyclomatic` row), `docs/symbol-reports.md`
  (`--json-full` section), `docs/cache-design.md` (a counting-rule change
  bumps the revision): all now link the new page.
- `README.md` (release notes bullet), `lib/README.md` (metrics facet),
  `cli/README.md` (where metrics appear).
- Skill `SKILL.md`: the CLI-exposure sentence was corrected (see drift
  below), the new page was added to "Detailed Documentation", and the
  metrics bullet now names it as the user contract.
  `api-reference.md` was already current.

### Comment and doc drift found and fixed

- `docs/symbol-schema-design.md`: the `MetricsFacet` code block listed only
  `parameters`, `has_docs` and `lines`. `cyclomatic` was added.
- **CLI exposure was misdescribed.** `cli/README.md` said the `metrics`
  facet "is only in `--json-full`". The skill and the Phase 4
  `message_to_agent` said `--json`/`--jsonl` "carry no complexity". Running
  `hug --json functions --undocumented` and `hug functions --jsonl
  --min-params 1` showed otherwise. These selectors already request
  Metrics, so their records carry the facet, `cyclomatic` included. This
  matches the spec ("wherever they already request Metrics"), so the code
  is right and the prose was wrong. The docs, README and skill now say so.
  The `reports` test now pins this (see below).
- Code comments on the changed symbols were reviewed: `MetricsFacet`,
  `symbol_metrics`, `body_lines`, `callable_body`, `complexity/*`,
  `WorkCounters::complexity_walks`, `SchemaVersion::V4_3`/`CURRENT`,
  `EXTRACTOR_REVISION` and `Capability::Metrics`. No drift was found. No
  comment states a metric count that is now wrong.

### Requirement → test mapping

| Requirement | Test |
|---|---|
| JSON/JSONL expose the key wherever Metrics is already requested, with the same values as the full index | **extended** `reports::cyclomatic_is_reported_only_where_metrics_already_are`: `--json functions --undocumented` and `functions --jsonl --min-params 1` give 7 records, each carrying the `cyclomatic` key (presence asserted, so an absent key cannot pass as `null`) equal to the `--json-full` value (3, 3, `null` for Go, `null` for the bodyless trait method) |
| Doc claims (every example total) | each example is a row in `RUST_CYCLOMATIC`/`JAVASCRIPT_CYCLOMATIC`/`TYPESCRIPT_CYCLOMATIC`, `MALFORMED_BODIES`, or `complexity::tests` (macro token-tree error → `None`) |

The new assertion fails if a selector-driven listing drops or changes the
key. The doc pages are not read by any compiled code, so they need no
corpus test.

### Accepted open-question choices and departures (close-out)

- Convention, language set (Rust, JavaScript, TypeScript) and R1–R6 are
  built as drafted in Phase 1. **Author acceptance is still pending.** The
  session is non-interactive, so the spec keeps `human_review: true` with
  its existing item, and the published doc describes the rules as built.
- Departures recorded earlier are unchanged:
  - the body comes from `callable_body` (the reader `lines` uses), not from
    `body_node_for_declaration` (Phase 1);
  - the schema is 4.3 and the revision 27 → 28, not 4.2/26 (Phase 2/3);
  - a focused `complexity_walks` counter was added (Phase 4).
- No closed prerequisite spec was edited.

### Final verification

- Every rule has an executable expectation (`*_CYCLOMATIC` rows), and all
  16 `METRICS_MATRIX` rows have an explicit `cyclomatic` cell (13 are
  `NO_CYCLOMATIC`).
- Narrow, complete, cached and uncached results agree (Phase 4 tests, still
  green).
- The docs explain how to read a number versus `null`
  (`docs/cyclomatic-complexity.md` "Reading the value" and "When the value
  is `null`").
- The feature is left active. Status: implementation complete, ready for
  review once the author accepts the rulings.

### Gates run (2026-10-05)

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' |
  xargs python3 scripts/ci/affected_scope.py` → `packages: [tree-hugger,
  tree-hugger-cli, test-toolkit]`, `change_class: package`. The gates were
  therefore required and run.
- `just test` (tree-hugger): 1099 tests, 1099 passed.
- `just lint` (tree-hugger): passes (exit 0).
- `just test-l2` was not re-run. No renderer or terminal output changed,
  and Phase 4 ran it green.
- OS: no cross-check. The only code change is an L1 CLI test assertion
  that uses the existing `Scratch` harness. CI covers Linux and macOS on the
  pull request and Windows on `main`.
