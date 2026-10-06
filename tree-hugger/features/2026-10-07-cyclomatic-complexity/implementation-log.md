---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-07-cyclomatic-complexity/spec.md
plan: tree-hugger/features/2026-10-07-cyclomatic-complexity/plan.md
implemented_by: claude/opus
started_phase: 1
packages: []
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
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
