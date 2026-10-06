---
$schema: feature-review.yaml
ready: false
findings:
  - title: Type syntax inside callable bodies inflates complexity
    priority: high
  - title: Default groups change complexity when their labels are reordered
    priority: medium
human_review: true
human_review_items:
  - |-
    Accept or change the pending counting convention and initial language set.
    This is the existing unresolved author choice recorded in the specification,
    not a new approval requirement introduced by this review.

    The implementation supports Rust, JavaScript, and TypeScript. A callable
    starts at 1 and adds 1 for each conditional branch, conditional loop,
    short-circuit operator, non-default branch arm, guard, and exception handler.
    Rust treats the last match arm as its default and counts `let … else`.
    JavaScript and TypeScript count `??`, `&&=`, `||=`, and `??=`; optional
    chaining and Rust `?` add nothing. Rust async, unsafe, labeled, and inline
    const blocks belong to the enclosing callable; nested const/static items
    do not. Object literal values belong to the enclosing callable, while
    nested methods, types, and TypeScript namespaces do not.

    Review these choices against the examples in
    `tree-hugger/docs/cyclomatic-complexity.md` and choose one outcome:

    - Accept the implemented convention and these three supported languages.
    - Specify different counting rules or a different initial language set;
      update the implementation, expectations, documentation, and extractor
      revision before release.

    Both technical findings below can be repaired before this choice is made.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-05T21:25:32-07:00
spec: 2026-10-07-cyclomatic-complexity/spec.md
implemented: true
next: 2026-10-07-cyclomatic-complexity/review-2.md
implemented_by: claude/opus
log: tree-hugger/features/2026-10-07-cyclomatic-complexity/log.md
description: "A **feature** review of `2026-10-07-cyclomatic-complexity/spec.md`"
feature: 2026-10-07-cyclomatic-complexity/review-1.md
---

# Review of Cyclomatic Complexity — Iteration 1

The feature is **not production ready**. Two counting defects produce incorrect
numbers despite all existing Level 1 tests passing. Type syntax can contribute
decisions to the enclosing callable, and a switch group containing `default`
can contribute differently when its labels are reordered. Neither finding is
blocked on human review.

## Findings

### High — Type syntax inside callable bodies inflates complexity

**Defect class:** traversal excludes declaration headers and named type
declarations but still counts decisions inside type syntax embedded in a
callable's body.

In the tree-hugger library, [cyclomatic](../../lib/src/file/extract/complexity/mod.rs)
computes a callable's complexity with an iterative traversal.
Its `visit` function prunes only the boundary kinds listed in the
[Rust rules](../../lib/src/file/extract/complexity/rust.rs) and
[TypeScript rules](../../lib/src/file/extract/complexity/typescript.rs).
Those lists omit anonymous type syntax and type arguments. Consequently, an
`if_expression` inside a Rust array length or a `ternary_expression` inside a
TypeScript type is counted as a decision in the surrounding function.

The specification excludes type syntax and nested type expressions. The current
[complexity documentation](../../docs/cyclomatic-complexity.md) also says
TypeScript type syntax never adds. A type annotation should not make an otherwise
straight-line callable look more complex. Named aliases are already excluded,
so moving the same type inline currently changes the number.

Minimal Rust reproduction:

```rust
fn inline_type() {
    let _x: [u8; if true { 1 } else { 2 }];
}

fn named_type() {
    type Buffer = [u8; if true { 1 } else { 2 }];
    let _x: Buffer;
}
```

Save this as `probe.rs` and run from its directory:

```sh
hug --json-full functions --no-cache probe.rs
hug --json functions --undocumented --no-cache probe.rs
hug functions --jsonl --undocumented --no-cache probe.rs
```

Observed complexity is **2** for `inline_type` and **1** for `named_type`;
both should be **1**. The Rust sweep source also compiled successfully with
`rustc --edition=2024 --crate-type lib`; this is supported Rust code, not merely
parser recovery.

The same omission affects TypeScript and TSX. For example:

```ts
function inlineType(a: boolean) {
  let x: { [a ? 1 : 2]: string };
}
```

The pinned grammars parse this without syntax errors, but the type's conditional
adds 1 to the function. These TypeScript probes establish syntax-tree behavior;
they do not claim acceptance by the TypeScript compiler.

The sweep substituted one decision into each type context, retaining ordinary
value expressions as controls. Every TypeScript row below ran through both
`.ts` and `.tsx`, except angle-bracket assertions, which apply only to `.ts`.
All rows parsed without syntax errors.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust local array annotation | `let _x: [u8; if true { 1 } else { 2 }];` | 2 | 1 |
| Rust reference annotation | `let _x: &[u8; if true { 1 } else { 2 }];` | 2 | 1 |
| Rust tuple annotation | `let _x: ([u8; if true { 1 } else { 2 }],);` | 2 | 1 |
| Rust pointer annotation | `let _x: *const [u8; if true { 1 } else { 2 }];` | 2 | 1 |
| Rust function-pointer annotation | `let _x: fn([u8; if true { 1 } else { 2 }]);` | 2 | 1 |
| Rust generic type argument | `let _x: Buf<{ if true { 1 } else { 2 } }> = Buf;` with `struct Buf<const N: usize>;` | 2 | 1 |
| Rust explicit generic arguments on a call | `make::<{ if true { 1 } else { 2 } }>();` with `fn make<const N: usize>() {}` | 2 | 1 |
| Rust explicit generic arguments on a value | `let _x = Buf::<{ if true { 1 } else { 2 } }>;` | 2 | 1 |
| Rust cast target | `std::ptr::null::<u8>() as *const [u8; if true { 1 } else { 2 }]` | 2 | 1 |
| Rust nested type alias / struct | Decision in the alias target / tuple-struct field type | 1 in each | 1; clean |
| Rust return / parameter type | The same array type in a declaration header | 1 in each | 1; clean |
| Rust nested const item | `const X: usize = if true { 1 } else { 2 };` | 1 | 1; clean |
| Rust inline const block | `let _x = const { if true { 1 } else { 2 } };` | 2 | 2 under the documented convention; clean |
| Rust array-repeat value / ordinary initializer | `[0; if true { 1 } else { 2 }]` / `if true { 1 } else { 2 }` | 2 in each | 2; clean value-expression controls |
| TypeScript / TSX local object-type annotation | `let x: { [a ? 1 : 2]: string };` | 2 | 1 |
| TypeScript / TSX array / tuple annotation | The same object type inside `…[]` / `[…]` | 2 in each | 1 |
| TypeScript / TSX assertion target | `{} as { [a ? 1 : 2]: string }` | 2 | 1 |
| TypeScript / TSX satisfies target | `{} satisfies { [a ? 1 : 2]: string }` | 2 | 1 |
| TypeScript angle-bracket assertion | `<{ [a ? 1 : 2]: string }>{}` | 2 | 1 |
| TypeScript / TSX generic argument | `f<{ [a ? 1 : 2]: string }>()` | 2 | 1 |
| TypeScript / TSX type query | `let x: typeof import(a ? 'a' : 'b');` | 2 | 1 |
| TypeScript / TSX return / parameter type | The same object type in a declaration header | 1 in each | 1; clean |
| TypeScript / TSX local alias / interface | The same object members in a named type declaration | 1 in each | 1; clean |
| TypeScript / TSX conditional type | `let x: T extends string ? number : boolean;` | 1 | 1; clean |
| TypeScript / TSX nested class | `class C { x = a ? 1 : 2; }` | 1 | 1; clean |
| TypeScript / TSX object value | `return { p: a ? 1 : 2 };` | 2 | 2; clean value-expression control |
| JavaScript | Anonymous class initializer / object value | 1 / 2 | 1 / 2; clean; no type-annotation syntax |
| Other 13 languages | Existing explicit unsupported rows in the metrics matrix | `null` | `null`; no complexity traversal |

The tree-hugger CLI's JSON, JSON Lines, and full-index projections all preserve
these results. Fresh cache writes and actual warm hits preserve them too; each
warm full-index run reported **one hit and zero parses**. Thus this is an
extraction defect, not a report-format or cache-projection defect.

**Required change:** exclude type subtrees and type-argument regions by grammar
kind or field before counting their descendants. Preserve decisions in ordinary
value expressions and the explicitly documented inline-const convention. Add
the failing contexts and clean controls to the public metrics matrix, including
both TypeScript grammars, and verify selective/complete queries and warm hits.
Update the relevant rule inventory and documentation, and bump the extractor
revision so cached incorrect totals are recomputed.

### Medium — Default groups change complexity when their labels are reordered

**Defect class:** grouped branch counting checks the group's final label without
recognizing a default label earlier in the same group.

In the tree-hugger library, [Counts::Arm](../../lib/src/file/extract/complexity/mod.rs)
decides whether a JavaScript switch case contributes a decision. It sees whether
that case has a body, or is the last label, but does not check whether the group
also contains `default`. The [JavaScript decision table](../../lib/src/file/extract/complexity/javascript.rs)
uses this rule; TypeScript and TSX share it.

The documented convention says labels sharing a body are one arm and labels
grouped with `default` add nothing. It does not restrict `default` to the end
of a group. The implementation currently applies that exclusion only when
`default` is the final label.

Minimal reproduction, valid in JavaScript, TypeScript, and TSX:

```js
function defaultFirst(x) {
  switch (x) { default: case 1: break; }
}
function defaultLast(x) {
  switch (x) { case 1: default: break; }
}
```

Run the same three CLI commands above with this source saved as `probe.js`,
`probe.ts`, or `probe.tsx`. The first function reports **2**, the second **1**;
both should report **1** under the documented convention.

The complete switch-label sweep ran every row below in all three grammars.
Rust's separate match-arm rule was checked as a sibling branch-counting site.
The switch sources had no syntax diagnostics.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| JavaScript / TypeScript / TSX switch group | `default: case 1: break;` | 2 | 1 |
| JavaScript / TypeScript / TSX switch group | `case 1: default: case 2: break;` | 2 | 1 |
| JavaScript / TypeScript / TSX group with comments | `default: /* c */ case 1: /* d */ case 2: break;` | 2 | 1 |
| JavaScript / TypeScript / TSX final empty group | `default: case 1:` | 2 | 1 |
| JavaScript / TypeScript / TSX switch group | `case 1: default: break;` | 1 | 1; clean |
| JavaScript / TypeScript / TSX final empty group | `case 1: default:` | 1 | 1; clean |
| JavaScript / TypeScript / TSX default only | `default: break;` | 1 | 1; clean |
| JavaScript / TypeScript / TSX distinct default body | `default: x++; case 1: break;` | 2 | 2; clean |
| JavaScript / TypeScript / TSX ordinary group | `case 1: case 2: break;` | 2 | 2; clean |
| JavaScript / TypeScript / TSX final empty case | `case 1: break; case 2:` | 3 | 3; clean |
| JavaScript / TypeScript / TSX separate case bodies | `case 1: x++; case 2: x--; break; default: break;` | 3 | 3; clean |
| Rust match | `true => 1, false => 2` | 2 | 2; clean last-arm convention |
| Rust grouped pattern | `1 \| 2 => 1, _ => 2` | 2 | 2; clean, grouped patterns remain one arm |
| Other 13 languages | Existing unsupported-language metrics rows | `null` | `null`; no arm-counting rule enabled |

JSON, JSON Lines, full-index, uncached, cold-cache, and warm-cache checks all
agreed. There is no independent renderer or cache repair needed.

**Required change:** determine the whole consecutive label group before deciding
whether it contributes; a default anywhere in a group excludes that arm.
Preserve distinct bodies, final empty cases, and comment-insensitive grouping.
Add permanent expectations for every ordering above in JavaScript, TypeScript,
and TSX, and retain Rust controls. Bump the extractor revision and align the
rule-table comments with the group-based behavior.

## Verification and requirement coverage

Reviewed the specification, plan, implementation log, counting engine, all
enabled rule tables, shared body reader, metric construction, schema decoding,
cache invalidation/projection, CLI output, current documentation, and test
placement. No production code or permanent tests were changed during review.

- `just test` in `tree-hugger/`: **1,099 passed, zero skipped**.
- `just lint` in `tree-hugger/`: passed for the library and CLI.
- `just check-tier-coverage tree-hugger`: **zero stranded tests**.
- Supplemental CLI sweeps checked the source contexts listed in both findings,
  with fresh caches, actual warm hits, and all three JSON report formats.
- A temporary public-API probe checked all 17 numeric-field shapes below through
  both `SymbolRecord` and `FileSymbolIndex` decoding; it passed and was removed.
- The Rust type-context sweep compiled successfully with `rustc`.

All new requirements concern syntax analysis or machine-readable output.
Level 1 public-API and hermetic CLI tests are the appropriate verification
level. This feature adds no rendered terminal view, input event, or keyboard
behavior requiring Level 2 or Level 3 evidence. Existing terminal tests were
not rerun for this review. Cross-OS evidence does not affect this verdict.

| Requirement | Strongest relevant verification inspected | Assessment |
|---|---|---|
| Exact per-language decision totals and zero-contribution controls | Level 1 public metrics matrices with clean-parse assertions and LF/CRLF variants | Findings above expose omitted contexts |
| Bodyless and non-callable records; unsupported languages; other metrics preserved | Level 1 16-language metrics matrix and malformed-body tests | Passing |
| Nested callable/type ownership, including an omitted inner callable | Level 1 public queries plus supplemental CLI controls | Existing named/callable boundaries pass; anonymous type syntax is incomplete |
| Error/missing-node isolation and deeply nested input | Level 1 public malformed-body matrix and engine tests, including a 20,000-level tree on a small stack | Passing for tested forms |
| Checked arithmetic | Level 1 accumulator overflow test | Passing |
| Metrics absent when unrequested; no unwanted traversal or optional passes | Level 1 work-counter and query tests | Passing |
| Widen once, cache unavailable values, warm hits without parsing, projection equivalence | Level 1 public cache tests asserting `CacheLookup::Hit` and work counters | Passing; caches also preserve the two incorrect totals |
| JSON / JSONL / full-index exposure and unchanged statistics | Level 1 hermetic CLI tests plus supplemental sweeps | Passing transport behavior |
| Required nullable field and older schema/revision invalidation | Level 1 record-decoding, cache-robustness, and CLI cache-upgrade tests | Passing |
| Symbol identity and older-model adapter behavior | Level 1 query identity comparison and explicit lossy-adapter tests | Passing |
| God-files scoring remains independent of complexity | Existing Level 1 god-files tests and inspection of the scoring path | No use of the new metric found |

The library uses a declared `tests/l1/main.rs` target with the metrics, query,
cache, and adapter modules included. CLI checks are included by the live
`tests/symbols/main.rs` target. The relevant tests have ordinary Level 1 names
and are selected by `just test`.

### Input robustness matrix

The only newly read load-bearing field is `metrics.cyclomatic`, a required
nullable `u32` in JSON. JSON Lines and full-index output contain the same
record shape; they add no separate input parser. The record and file-index
deserializers share the metric deserializer, and the persisted symbol-cache
reader decodes those records before validating capability/version consistency.

The existing public record and CLI edited-cache tests passed, and a supplemental
public-API probe confirmed the record and file-index outcomes in the following
matrix. The cache tests edit an actual written entry and use a renamed record
to distinguish a served entry from recomputation; they assert public output.

| Shape of `metrics.cyclomatic` | Public record / file-index decoding | Persisted-cache public outcome |
|---|---|---|
| Control, numeric value | Preserved | Served and preserved |
| Absent | Rejected | Recomputed; no fabricated `null` |
| Explicit `null` | Preserved as unavailable | Served as `null` |
| Wrong whole-field type: string, Boolean, array, object | Rejected | Recomputed |
| Array with one or every element of the wrong type | Rejected: the field is a scalar | Recomputed |
| Empty array / object | Rejected | Recomputed |
| Negative, fractional, or greater than `u32::MAX` | Rejected | Recomputed |
| `u32::MAX` | Preserved | Served and preserved |
| Duplicate key | Rejected | Recomputed |
| Trailing invalid content | Rejected | Recomputed |
| Previous schema or extractor revision | Covered by separate upgrade checks | Recomputed |

The new field uses the existing required-nullable deserializer; it has no
permissive default or filtering conversion. No robustness finding was identified.

## Human review and closure

The specification still records author acceptance of the convention and initial
language set as pending. That choice is carried in `human_review_items`; it is
not counted as a technical finding or as the reason for `ready: false`.
Repair both findings and their regression coverage before the next review.
The feature remains active for the author to close after the review cycle.
