---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
  - |-
    Accept or change the counting convention and initial language set. This
    author choice remains pending from the specification and the first review;
    the repairs have not changed the proposed convention.

    Rust, JavaScript, and TypeScript are supported. Each callable starts at 1
    and adds 1 for each conditional branch, conditional loop, short-circuit
    operator, non-default branch arm, guard, and exception handler. Rust treats
    the last match arm as the default and counts `let … else`. JavaScript and
    TypeScript count `??`, `&&=`, `||=`, and `??=`; optional chaining and Rust
    `?` add nothing. Rust async, unsafe, labeled, and inline const blocks count
    in the enclosing callable; nested const/static items do not. Object literal
    values count; nested methods, types, and TypeScript namespaces do not.

    Read the examples in [Cyclomatic Complexity](../../docs/cyclomatic-complexity.md)
    and choose one outcome:

    - Accept these counting rules and the three supported languages.
    - Specify different rules or a different initial language set. Adjust the
      implementation, expected totals, documentation, and extractor revision
      together before release.

    This pending choice is separate from technical readiness. There are no
    findings blocked on it.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-05T21:52:08-07:00
spec: 2026-10-07-cyclomatic-complexity/spec.md
implemented: false
description: "A **feature** review of `2026-10-07-cyclomatic-complexity/spec.md`"
feature: 2026-10-07-cyclomatic-complexity/review-2.md
previous: 2026-10-07-cyclomatic-complexity/review-1.md
---

# Review of Cyclomatic Complexity — Iteration 2

The feature is **production ready under the implemented counting convention**.
Both technical findings from the first review are repaired, including their
sibling sites and regression coverage. No new technical finding was identified.
The author's acceptance of the counting convention remains a separate human
review item, as in the first review; it does not determine this readiness verdict.

## Previous findings and blocked-item check

The first review placed its two unblocked technical findings under `Findings`,
rather than a heading named `Unblocked Findings`. Both were checked first.
It explicitly had no blocked findings, and the repair log records no deferred
finding or intervening author decision. There is therefore no newly unblocked
technical work to assess.

### Type syntax inside callable bodies inflates complexity — resolved

**Defect class checked:** syntax traversal counts value-shaped decisions inside
type regions that should contribute nothing to the enclosing callable.

In the tree-hugger library, [cyclomatic](../../lib/src/file/extract/complexity/mod.rs)
measures a callable's body. Its traversal now excludes type nodes and type
arguments. TypeScript assertions and `satisfies` also exclude the children after
their keyword, covering targets that the grammar represents as ordinary call
or member expressions. The operand still contributes its own decisions.

The [Rust rules](../../lib/src/file/extract/complexity/rust.rs) and
[TypeScript rules](../../lib/src/file/extract/complexity/typescript.rs) enumerate
the exclusions. A grammar-inventory test checks their type kinds against the
pinned grammars, including both TypeScript grammars. The public
[metrics matrix](../../lib/tests/l1/symbols_metrics.rs) checks the original
failing contexts and value-expression controls through selective and complete
queries, with LF and CRLF inputs.

The following sweep includes every site from the earlier finding. TypeScript
rows ran as both `.ts` and `.tsx` unless explicitly restricted to `.ts`.
The observed totals matched expectations in uncached, cold-cache, and warm-cache
CLI reports using JSON, JSON Lines, and full indexes.

| Site | Shape tested | Observed | Expected |
|---|---|---|---|
| Rust local array annotation | `let _x: [u8; if true { 1 } else { 2 }];` | 1 | 1 |
| Rust reference annotation | The same array type inside `&…` | 1 | 1 |
| Rust tuple annotation | The same array type inside `(…, )` | 1 | 1 |
| Rust pointer annotation | The same array type inside `*const …` | 1 | 1 |
| Rust function-pointer annotation | The same array type inside `fn(…)` | 1 | 1 |
| Rust generic type argument | `Buf<{ if true { 1 } else { 2 } }>` | 1 | 1 |
| Rust call turbofish | `make::<{ if true { 1 } else { 2 } }>()` | 1 | 1 |
| Rust value turbofish | `Buf::<{ if true { 1 } else { 2 } }>` | 1 | 1 |
| Rust cast target | Cast to `*const [u8; if true { 1 } else { 2 }]` | 1 | 1 |
| Rust qualified path type | `<[u8; if true { 1 } else { 2 }] as Default>::default()` | 1 | 1 |
| Rust nested alias and struct | The same array type in a local alias or tuple-struct field | 1 | 1; retained clean exclusions |
| Rust declaration header | The same array type in parameter and return positions | 1 | 1; retained clean exclusion |
| Rust nested const/static items | Decisions inside local item initializers | 1 | 1; retained clean exclusions |
| Rust value controls | Conditional cast operand, call argument, inline const block, array-repeat length, or ordinary initializer | 2 each | 2; value decisions retained |
| Rust generic struct value | Conditional const argument plus conditional field initializer | 2 | 2; only the field initializer counts |
| TypeScript object annotation | `let x: { [a ? 1 : 2]: string };` | 1 | 1 |
| TypeScript array and tuple annotations | The same object type inside `…[]` and `[…]` | 1 | 1 |
| TypeScript `as` target | `{} as { [a ? 1 : 2]: string }` | 1 | 1 |
| TypeScript value-shaped `as` target | `{} as import(a ? 'a' : 'b')` | 1 | 1 |
| TypeScript `satisfies` target | Object type or `import(a ? 'a' : 'b')` target | 1 each | 1 |
| TypeScript angle-bracket assertion, `.ts` only | `<{ [a ? 1 : 2]: string }>{}` | 1 | 1 |
| TypeScript call type argument | `f<{ [a ? 1 : 2]: string }>()` | 1 | 1 |
| TypeScript constructor type argument | `new C<{ [a ? 1 : 2]: string }>(a ? 1 : 2)` | 2 | 2; argument value retained |
| TypeScript instantiation expression | `f<{ [a ? 1 : 2]: string }>` | 1 | 1 |
| TypeScript type query | `let x: typeof import(a ? 'a' : 'b');` | 1 | 1 |
| TypeScript chained assertion targets | Object-type assertion followed by an import-type assertion | 1 | 1 |
| TypeScript declaration header | Object type with a conditional computed key in parameter/return positions | 1 | 1; retained clean exclusion |
| TypeScript named types and namespaces | Local alias, interface, enum, abstract class, or namespace containing decisions | 1 for enclosing callable | 1; retained clean exclusions |
| TypeScript conditional type | `let x: T extends string ? number : boolean;` | 1 | 1; retained clean exclusion |
| TypeScript nested class | Conditional field initializer | 1 for enclosing callable | 1; retained clean exclusion |
| TypeScript value controls | Conditional assertion operand, conditional call argument, or object property value | 2 each | 2; value decisions retained |
| TypeScript chained assertion operand | `((a ? 1 : 2) as number satisfies number) || b` | 3 | 3; ternary and short-circuit retained |
| JavaScript | Anonymous class initializer versus object property value | 1 / 2 | 1 / 2; clean, no type-annotation grammar |
| Other 13 languages | Public metrics matrix's explicit unsupported rows | `null` | `null`; no complexity rules enabled |

Some TypeScript type-context probes are accepted by the pinned syntax grammars
without asserting that the TypeScript compiler accepts them. Their purpose is
to verify how the public syntax-analysis API handles those parsed regions.

### Default groups change complexity when their labels are reordered — resolved

**Defect class checked:** grouped branch counting decides an arm from one label
instead of checking every label sharing the body.

In the tree-hugger library, [Counts::Arm](../../lib/src/file/extract/complexity/mod.rs)
now checks the consecutive labels preceding the group's final label and excludes
the arm if any is `default`. A label with its own body ends a group; comments
and other grammar extras do not. JavaScript, TypeScript, and TSX use the same
decision table. The public `SWITCH_GROUP_CYCLOMATIC` matrix exercises every
ordering from the first review in all three grammars, both query paths, and
both line endings.

| Site | Shape tested | Observed | Expected |
|---|---|---|---|
| JavaScript / TypeScript / TSX group | `default: case 1: break;` | 1 | 1 |
| JavaScript / TypeScript / TSX group | `case 1: default: case 2: break;` | 1 | 1 |
| JavaScript / TypeScript / TSX group | `case 1: default: break;` | 1 | 1 |
| JavaScript / TypeScript / TSX commented group | Block or line comments between default and ordinary labels | 1 | 1 |
| JavaScript / TypeScript / TSX final empty group | `default: case 1:` or `case 1: default:` | 1 each | 1 |
| JavaScript / TypeScript / TSX default alone | `default: break;` | 1 | 1; retained clean case |
| JavaScript / TypeScript / TSX distinct default body | `default: x++; case 1: break;` | 2 | 2; retained clean case |
| JavaScript / TypeScript / TSX ordinary group | `case 1: case 2: break;` | 2 | 2; retained clean case |
| JavaScript / TypeScript / TSX final empty case | `case 1: break; case 2:` | 3 | 3; retained clean case |
| JavaScript / TypeScript / TSX separate bodies | `case 1: x++; case 2: x--; break; default: break;` | 3 | 3; retained clean case |
| JavaScript / TypeScript / TSX multiple groups | Ordinary group, group containing default, final empty case | 3 | 3; default does not cross a body boundary |
| JavaScript nested switch | Outer ordinary arm contains an inner default-first group | 2 | 2; inner default does not suppress the outer arm |
| JavaScript label-value control | Default-group label has a ternary; two later ordinary arms have `;` / `{}` bodies | 4 | 4; label expression and separate bodies count |
| Rust match | `true => 1, false => 2` | 2 | 2; unchanged last-arm convention |
| Rust grouped pattern | `1 \| 2 => 1, _ => 2` | 2 | 2; one arm node |
| Rust wildcard and guard | `_ if x > 1 => 1, 1 \| 2 => 2, _ => 3` | 4 | 4; guard and non-last arms count |
| Rust nested match | Two-arm match inside the first arm of another two-arm match | 3 | 3; each match has its own last arm |
| Other 13 languages | Explicit unsupported metrics rows | `null` | `null`; no branch-counting rules enabled |

Both repairs use extractor revision **29**, replacing revision 28. The
[CLI cache-upgrade test](../../cli/tests/symbols/cache_upgrade.rs) verifies that
old totals are recomputed for Rust types, TypeScript types, and JavaScript
default-first groups. The record shape remains schema **4.3**. The golden-file
diffs change the extractor provenance, and the current documentation and skill
describe the repaired rules.

## Findings

None. No additional functional, test-placement, or material performance defect
was identified within this feature's scope.

## Recurrence

No finding recurs. The earlier two defect classes were swept again, including
the clean controls and adjacent forms above; neither remains unresolved.

## Verification and requirement coverage

Reviewed the specification, plan, both implementation logs, previous review,
counting engine, enabled language tables, shared body reader, metric construction,
schema decoding, cache projection and invalidation, CLI projections, current
documentation, and relevant test declarations. Production code and permanent
tests were not changed during this review.

- `just test` in `tree-hugger/`: **1,102 passed, zero skipped**.
- `just lint` in `tree-hugger/`: passed for the library and CLI.
- `just check-tier-coverage tree-hugger`: **zero stranded tests**.
- Supplemental shipped-CLI sweep: **198 expected totals across 90 source files**,
  checked in JSON, JSON Lines, and full-index output. Each format used isolated
  scratch inputs, home/config directories, and a fresh cache: cold runs reported
  90 misses and 90 parses; warm runs reported 90 hits and zero parses.

The CLI sweep replayed the permanent Rust, JavaScript, TypeScript, TSX, and
switch-group matrices, then added the adjacent forms listed above. Its malformed
type controls returned `null` in Rust, TypeScript, and TSX. Computed method names,
anonymous class heritage expressions, nested generator bodies, and their class
initializers remained excluded from the enclosing callable, as required by the
ownership convention. A Rust multi-`let` chain with three `&&` operators scored
5: the base value, the `if`, and the three operators.

For a small reproduction, save one source example from the tables as `probe.rs`,
`probe.js`, `probe.ts`, or `probe.tsx`, and run in its directory:

```sh
hug --json-full functions --no-cache probe.ts
hug --json functions --undocumented --no-cache probe.ts
hug functions --jsonl --undocumented --no-cache probe.ts
```

Use the matching extension. Omit `--no-cache` and add `--verbose` for cold and
warm checks; a passing warm check must actually report a hit and zero parses.

| User-facing requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Exact per-language totals and zero-contribution controls | Level 1 public metrics matrices, LF/CRLF, clean-parse assertions; shipped-CLI sweep | Passing, including both repaired classes |
| Nested callable/type ownership, even when the inner record is omitted | Level 1 public query tests and nested callable/type controls | Passing |
| Empty and expression bodies, bodyless/non-callable records, unsupported languages, other metrics retained | Level 1 public metrics and 16-language coverage matrices | Passing |
| Counted-body errors and missing nodes; nested-body and sibling isolation | Level 1 engine tests and public malformed-body matrix; supplemental malformed type controls | Passing for tested forms |
| Deep traversal and checked arithmetic | Level 1 20,000-level walk on a 256 KiB stack and accumulator-overflow test | Passing |
| No complexity work without Metrics and no unwanted optional passes | Level 1 work-counter/query assertions | Passing |
| Narrow/complete agreement and unchanged symbol identity | Level 1 public query comparisons | Passing |
| Widen once; cache unavailable values; warm hits without parsing; remove unrequested facet | Level 1 cache tests asserting actual lookup states, result equality, and work counters | Passing |
| JSON / JSONL / full-index exposure only where Metrics is requested; unchanged statistics | Level 1 hermetic CLI report tests and supplemental format sweep | Passing |
| Required nullable key and malformed-cache handling | Level 1 public record and edited-cache CLI matrices | Passing |
| Previous schema/revision entries recomputed | Level 1 CLI cache-upgrade tests, including revision 28's incorrect totals | Passing |
| Older-model adapters remain explicitly lossy | Level 1 adapter tests | Passing |
| God-files size/risk policy remains independent of complexity | Existing Level 1 god-files tests and inspection of its scoring inputs | Passing; no complexity input introduced |

All new behavior concerns syntax analysis, public API results, or machine-readable
reports. **Level 1 is the appropriate verification level.** This feature introduces
no rendered terminal view, input handling, keybinding, or OS keyboard behavior
requiring Level 2 or Level 3 evidence. Existing terminal tests were not rerun;
there is no verification-level mismatch in the new requirements. Cross-OS
evidence is left to the existing CI process and does not affect this verdict.

The library declares its consolidated `tests/l1/main.rs` target and includes
the metrics, query, cache, and adapter modules. The CLI's live `symbols` target
includes the report, cache-robustness, and cache-upgrade modules. The relevant
tests have ordinary Level 1 names and ran in `just test`; the tier audit confirms
none are stranded.

### Input robustness matrix

The feature's new load-bearing input field is **`metrics.cyclomatic`**, a
required nullable `u32` in JSON. The public record decoder and records nested
inside `FileSymbolIndex` share the same metric decoder. JSON Lines and full-index
reports are output projections, not additional input-format parsers.

The current run passed the public record shape tests and the persisted-cache
matrix. The latter edits entries written by `hug`, marks a stored record to
distinguish serving from recomputation, and asserts the public CLI result.
Additional numeric-shape checks cover the limits of `u32`.

| Shape | Public record JSON: `metrics.cyclomatic` | Persisted cache JSON: `metrics.cyclomatic` → public CLI |
|---|---|---|
| Numeric control | Value preserved | Stored value served and preserved |
| Absent | Rejected | Recomputed; no fabricated `null` |
| Explicit `null` | Preserved as unavailable | Served as `null` |
| Wrong whole-field type: string, Boolean, array | Rejected | Recomputed |
| Wrong whole-field type: object | Empty object rejected; object cannot decode as `u32` | Recomputed |
| Wrong type, one array element | Array cannot decode as a scalar `u32` | Recomputed |
| Wrong type, every array element | Array cannot decode as a scalar `u32` | Recomputed |
| Empty array / object | Rejected | Recomputed |
| Negative / fractional / greater than `u32::MAX` | Rejected | Recomputed |
| `u32::MAX` | Preserved | Served and preserved |
| Duplicate key | Rejected | Recomputed |
| Trailing invalid content | Rejected | Recomputed |
| Previous schema or extractor revision | Separate version checks apply | Recomputed |

Array-element rows are exercised through the cache matrix; they are not a
collection field with elements that can be silently discarded. The metric uses
the required-nullable decoder, with no permissive default or filtering conversion.
No input-robustness finding was identified.

## Closure

The review is marked `ready: true`, with no unblocked or blocked technical
findings. The previous review links forward to this review and is marked
implemented. The specification records two review iterations and `completed: true`
as requested for a passing review. Its pending author choice is preserved in
`human_review_items`; these metadata updates do not record author acceptance or
move the feature to a completed lifecycle directory.
