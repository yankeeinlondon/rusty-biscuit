---
$schema: feature-review.yaml
ready: false
findings:
  - title: Parameter defaults incorrectly see body-local var bindings
    priority: high
  - title: Pattern capitalization invents or omits Rust bindings
    priority: high
  - title: Transparent call wrappers incorrectly limit coverage
    priority: medium
  - title: Cache validation accepts nonexistent evidence identities and missing callable regions
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-06T15:09:44-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-08-scoped-references/log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-2.md
previous: 2026-10-08-scoped-references/review-1.md
next: 2026-10-08-scoped-references/review-3.md
---

# Review: Scoped References, iteration 2

The feature is **not production ready**. The previously reported examples have regression coverage and pass, but the broader sweep found incorrect bindings, false direct recursion, unnecessary uncertainty for wrapped calls, and remaining cache inconsistencies. None requires a human decision. This review changes only review documents and specification metadata.

## Previous findings and blocked work

The previous review uses `## Findings`, rather than separate unblocked and blocked sections. All four findings were unblocked; there were no blocked findings to reconsider.

| Previous finding | Repair verified | Remaining result of the class sweep |
|---|---|---|
| Lexical lookup crosses boundaries that do not establish a binding | Rust module/item boundaries, conditional branches, closure controls, ECMA `with` and direct `eval` have regression rows and pass | Parameter defaults still see body-local `var`; Rust pattern identity still depends on capitalization |
| Named function expressions bind their names to outer declarations | Named ordinary and generator expressions in TypeScript/JavaScript exclude their declaration names and prevent binding to the outer function; CLI regression passes | Original finding repaired |
| Invocation classification loses calls across supported source forms | All-language invocation matrix, nested member wrappers, negative controls and CLI callers/callees tests pass | The coverage readers still recognize selected direct callee shapes only |
| Cache validation accepts inconsistent reference evidence | Wrong retained target, malformed ID, lost ID, duplicate region and missing Rust/ECMA callable region cases now recompute; mutation matrix passes | Validly formatted nonexistent IDs remain accepted, and Python/Lua callable-region checks are bypassed |

The implementation log records a separate deferral for consistency of containment and declared-base relationships. Those relationships are not derived from reference occurrences and are outside this review's repair list. That deferral does not block any finding below.

## Unblocked Findings

### High — Parameter defaults incorrectly see body-local var bindings

**Defect class:** function-scope binding placement conflates the parameter environment with the function body environment, establishing references to declarations unavailable during default evaluation.

In the tree-hugger library, [ECMA binding_sites](../../lib/src/analysis/scope/ecma.rs) assigns `var` to `Placement::FunctionScope`, and [ScopeTree::placement](../../lib/src/analysis/scope/mod.rs) puts it in the same scope that parameter defaults search. The body block correctly isolates `function`, `let` and `const`, but `var` bypasses that block.

The minimal reproduction is:

```javascript
function f() {}
function g(x = f()) { var f = () => {}; }
```

`hug symbols --json-full --no-cache a.js` reports the default's call as `Scoped` to the body-local `f`. It must bind to the outer function. A Node execution with logging in each function confirms the default calls the outer function. This affects incoming references to both declarations and the calls reported for `g`.

The same source was adapted to every callable form below and run through the shipped CLI. Every defective row was tested in both `.ts` and `.js`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Function declaration, TS and JS | `function g(x=f()) { var f=()=>{}; }` | Default binds to body-local `f` | Bind to outer `f` |
| Arrow, TS and JS | `const g=(x=f())=>{ var f=()=>{}; };` | Same incorrect target | Outer `f` |
| Function expression, TS and JS | `const g=function(x=f()) { var f=()=>{}; };` | Same incorrect target | Outer `f` |
| Generator declaration, TS and JS | `function* g(x=f()) { var f=()=>{}; }` | Same incorrect target | Outer `f` |
| Generator expression, TS and JS | `const g=function*(x=f()) { var f=()=>{}; };` | Same incorrect target | Outer `f` |
| Method, TS and JS | `class C { g(x=f()) { var f=()=>{}; } }` | Same incorrect target | Outer `f` |
| Constructor, TS and JS | `class C { constructor(x=f()) { var f=()=>{}; } }` | Same incorrect target | Outer `f` |
| Function declaration, TS control | Replace `var` with `let`, `const`, or a body `function f() {}` | Default binds to outer `f` | Clean |
| Default-argument controls in previous repair | TS/JS body `function f() {}` | Regression tests pass | Clean |

**Required repair:** distinguish parameter-default lookup from body-local `var` lookup while preserving body hoisting and parameter bindings. Add Level-1 matrix rows for all seven callable forms in both languages, with public occurrence IDs and incoming/outgoing CLI assertions. Review the shared scope reader's consumers, including declared-relation lookup, for the same environment distinction. Update the parameter/hoisting contract in `docs/references.md`.

### High — Pattern capitalization invents or omits Rust bindings

**Defect class:** identifier spelling substitutes for binding identity in patterns, suppressing legitimate capitalized locals and inventing locals for lowercase constants.

In the tree-hugger library, [Rust pattern_names](../../lib/src/analysis/scope/rust.rs) treats every capitalized identifier in a refutable pattern as a constant or variant. Rust naming conventions are warnings, not name-resolution rules. The fallback in [bind_all](../../lib/src/analysis/occurrences.rs) sometimes masks this for query-captured tuple bindings, but bare pattern bindings have no such record.

This valid source falsely reports `is_recursive: Yes`:

```rust
fn F(x: fn()) {
    if let F = x { F(); }
}
```

The pattern token itself becomes a `Usage` of function `F`, and the call targets the function's own ID. It actually calls the parameter's function-pointer value through a new binding. `rustc --crate-type lib --edition 2024` accepted this source and the `while`, `match`, `mut` and `ref` variants below. The correct recursion result is `Unknown`, with indirect-dispatch coverage; the declaration token must be excluded from occurrences.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `if let` | Above source | Pattern is a function usage; call binds to enclosing function; recursion `Yes` | Exclude pattern name; bind local; recursion `Unknown` |
| `while let` | `while let F=x { F(); }` in the same function | Same false `Yes` | Local binding and `Unknown` |
| Match arm | `match x { F => F() }` | Same false `Yes` | Local binding and `Unknown` |
| Mutable match binding | `match x { mut F => F() }` | Same false `Yes` | Local binding and `Unknown` |
| Reference match binding | `match x { ref F => F() }` | Same false `Yes` | Local binding and `Unknown` |
| Constant in match pattern | `const f:u8=1; … match x { f => { f; }, _ => {} }` | Pattern disappears; body `f` binds without an ID | Both are references to the constant ID |
| Constant in `if let` / `while let` | `const f:u8=1; … if let f=x { f; }`, then `while let` | Same invented no-ID binding | Both names reference the constant |
| Constant in match guard | `const f:u8=1; … match x { f if f==1 => {}, _ => {} }` | Pattern disappears; guard binds without an ID | Constant references |
| Uppercase constant, control | `const F:u8=1; … match x { F => { F; }, _ => {} }` | Both references bind to constant | Clean |
| Explicit constant path, control | `m::f` in pattern and arm | Both remain qualified, uncertain usages | Clean conservative path evidence |
| Captured tuple bindings, controls | `Some(F)` in `if let`, `while let`, and `match` | Inner calls bind to captured local; following calls bind outer | Clean, but fallback masks the heuristic |
| Struct shorthand, controls | `S { F }` in conditional and match | Inner call binds without an outer ID | Clean |
| Irrefutable bindings, controls | `let F=x`, parameter `F`, `for F in xs` | Local calls bind to their local records | Clean |

**Required repair:** decide constant/variant identity from supported lexical evidence, and recognize explicit binding syntax such as `mut` and `ref`. When identity cannot be established, retain uncertainty instead of inventing a proven outer call. Sweep the shared pattern walker and capture fallback together. Add the entire table to Level-1 binding/recursion tests and assert CLI callers exclude the false self-calls. Correct the capitalization rule in the module comments and `docs/references.md`; that documented rule is itself incorrect.

### Medium — Transparent call wrappers incorrectly limit coverage

**Defect class:** invocation classification and coverage classification use different callee-shape rules, so syntax recognized as a direct established call still adds an indirect-dispatch limitation.

The tree-hugger library's [is_call_target and wrapper_operand](../../lib/src/analysis/occurrences.rs) recognize transparent wrappers, but `Walk::visit_rust` and `Walk::visit_ecma` in the same file still inspect the outer callee node directly. For `fn f() {} fn g() { (f)(); }`, the call correctly targets `f`, yet `g` has `Limited(indirect_dispatch)` coverage and recursion `Unknown`. The unwrapped version has complete coverage and recursion `No`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust call coverage | `(f)()` | Established call; limited coverage; `g: Unknown` | Complete coverage; `g: No` |
| Rust generic call coverage | `(f::<u8>)()` | Same unnecessary limitation | Complete / `No` |
| Rust explicit type-path coverage | `(S::f)()` with one inherent declaration | Same unnecessary limitation | Complete / `No` |
| TS and JS call coverage | `(f)()` | Same unnecessary limitation | Complete / `No` |
| TS assertion coverage | `(f as (()=>void))()`, `(f satisfies (()=>void))()`, `(<(()=>void)>f)()`, `f!()` | Each is an established call, but limited / `Unknown` | Complete / `No` |
| TS and JS constructor coverage | `new (F)()` with local class `F` | Established class target, but limited / `Unknown` | Same completeness as `new F()` |
| Direct-call controls, Rust/TS/JS | `f()` | Complete / `No` | Clean |
| Wrapped self-call controls, Rust/TS/JS | Existing regression matrix | `Yes` | Clean: positive self-call overrides the unnecessary limit |
| Member, variable, sequence, conditional and dereference controls | Existing invocation, negative and recursion matrices | Remain uncertain/indirect as appropriate | Preserve these limits |
| Other 13 language readers | Direct and wrapped invocation matrix | Explicit unsupported-language coverage | Clean conservative contract; no resolved coverage reader to repair |

**Required repair:** reuse the transparent-wrapper operand rules when deciding call and constructor coverage. Keep dynamic/receiver/value calls limited. Add negative-recursion rows for calls to a different established declaration; the existing wrapped self-call tests cannot detect this defect because a proven self-call wins over coverage limitations.

### Medium — Cache validation accepts nonexistent evidence identities and missing callable regions

**Defect class:** cache validation treats references missing from retained records as potentially omitted even in every-kind entries, and exempts whole languages from required callable-region checks.

In the tree-hugger library, [validate_evidence](../../lib/src/cache/stored_evidence.rs) allows an absent retained record through `is_none_or`. This is appropriate for declarations omitted by a narrowed entry, but an every-kind entry has no omitted declaration kinds. ID formatting does not prove a declaration exists. Separately, required-region validation skips Python and Lua outright.

Reproduction used copies of all 16 primary symbol fixtures under isolated temporary directories. For each file, the shipped CLI wrote an entry with `hug symbols --json-full --verbose`; the unedited second run proved one hit and zero parses. Each edit restored that original entry. A phantom ID is 64 zero characters, satisfying the generated-ID syntax. An extra region copies the last region's span/state/reasons and assigns a new phantom owner, keeping order intact.

`Hit` below means the malformed entry was served with zero parses and changed public output. `Miss` means one parse and output equal to the unedited control. Every defective cell should be a miss.

| Site | Phantom coverage owner | Phantom candidate ID | Phantom ownerless established target ID | Remove an empty named callable's region |
|---|---|---|---|---|
| Rust | Hit | Hit | Hit | Miss, clean |
| TypeScript | Hit | Hit in member-call reproduction | Hit | Miss, clean |
| JavaScript | Hit | Hit in member-call reproduction | Hit | Miss, clean |
| Go | Hit | Miss, clean | Not applicable: no established targets | Miss, clean |
| Python | Hit | Miss, clean | Not applicable | Hit |
| Java | Hit | Miss, clean | Not applicable | Miss, clean |
| C# | Hit | Miss, clean | Not applicable | Miss, clean |
| C | Hit | Miss, clean | Not applicable | Miss, clean |
| C++ | Hit | Miss, clean | Not applicable | Miss, clean |
| Swift | Hit | Miss, clean | Not applicable | Miss, clean |
| Scala | Hit | Miss, clean | Not applicable | Primary fixture has no empty named region |
| PHP | Hit | Miss, clean | Not applicable | Miss, clean |
| Perl | Hit | Miss, clean | Not applicable | Miss, clean |
| Bash | Hit | Miss, clean | Not applicable | Miss, clean |
| Zsh | Hit | Miss, clean | Not applicable | Miss, clean |
| Lua | Hit | Miss, clean | Not applicable | Hit |

Additional controls: a TypeScript named-owner target changed to the phantom ID is rejected through its contradictory summary; an ownerless malformed `"bogus"` ID is rejected in Rust and TypeScript. The passing narrowed-cache tests prove that IDs for genuinely omitted declarations must remain accepted. Existing tests also reject inconsistent `references`, `referenced_by`, `dependencies`, and recomputed semantic facets; they do not close the every-kind identity hole.

The Python/Lua defect is reproducible against their real primary fixtures by removing an already-present empty named region. It does not require guessing whether every declaration in those languages has a body. Validate against the callable/body facts actually retained, extending those facts if needed. A blanket language exemption cannot enforce the contract.

**Required repair:** require every non-null occurrence target, candidate, occurrence owner, coverage owner and reference-summary identity to name a retained declaration in every-kind entries, with matching record identity where applicable. Preserve narrowed-entry behavior and legitimate import/unreported-binding null IDs. Enforce empty callable regions for Python and Lua without assuming unsupported body shapes. Add all language rows above to the public cache mutation matrix, including named-owner and narrowed controls.

#### Input robustness matrix

The feature's changed file-format reader is the JSON persistent cache. The existing `cache_robustness` public-CLI tests now walk the structural matrix, restoring a tool-written fixture for each edit. The selected run passed these tests. There is no new manifest/configuration format to review.

| Shape | Occurrences | Coverage | Confidence | Target ID, owned | Target ID, ownerless | Candidates | Occurrence span | Owner | Owner span | Coverage owner | Coverage state | Reasons | Coverage span |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Unedited control | Hit | Hit | Hit | Hit | Hit | Hit | Hit | Hit | Hit | Hit | Hit | Hit | Hit |
| Absent | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss |
| Null | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss |
| Wrong whole type | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss |
| One/every wrong element | Miss | Miss | — | — | — | Miss | — | — | — | — | — | Miss | — |
| Empty | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss |
| Duplicate key | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss |
| Trailing invalid content | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss | Miss |

These columns select populated, load-bearing fields, so null/empty mutations contradict their controls. Null import/unreported IDs and ownerless regions remain legitimate in their own control contexts. Empty candidates for `Scoped` and empty reasons for `Complete` remain legitimate too. Capabilities have an additional entry-level matrix, which passed. The new failures are well-typed identity and region mutations, not permissive structural parsing; they must be added alongside this matrix rather than replacing it.

## Blocked Findings

None.

## Recurrence

- **Parameter environments and Rust patterns:** the previous review's “Lexical lookup crosses boundaries that do not establish a binding” concerns the same class of establishing names across unsupported visibility/binding rules. Its repair should have swept parameter defaults against every body binding placement, and bare/modified/refutable patterns against actual declaration identity. It tested body function declarations and lowercase tuple bindings, which did not expose these siblings.
- **Wrapper coverage:** the previous “Invocation classification loses calls across supported source forms” repaired selected callee shapes in invocation detection but left the sibling Rust/ECMA call and constructor coverage readers using direct shapes. The complete list is the wrapper table above.
- **Cache evidence:** the previous “Cache validation accepts inconsistent reference evidence” fixed malformed IDs and mismatched retained targets, but did not sweep validly formatted missing IDs across targets, candidates and owners in every-kind entries. Its missing-region repair also excluded Python/Lua. The all-language table above carries those remaining sites.

The named-expression defect itself did not recur. `recurrence: true` records the repeated classes, not a claim that the original regression tests still fail.

## Requirement verification

| User-facing requirement | Strongest verification inspected/run | Assessment |
|---|---|---|
| Language scope, order, patterns, aliases, categories and conservative receiver resolution | Level 1: binding/occurrence matrices and CLI reproductions | Parameter and pattern findings block readiness |
| Call/usage classification, spans, repeated sites, declaration exclusions and ownership | Level 1: all-language invocation and ownership tests, CLI tests | Original invocation/name-expression regressions pass; capitalized patterns add false usages |
| Honest direct recursion and coverage | Level 1: recursion matrix and additional valid Rust/ECMA fixtures | False Rust `Yes` and unnecessary wrapped-call `Unknown` remain |
| Whole-file evidence, filtered/projection/cache equivalence and skipped work | Level 1: reference-summary, selective-query and cache tests | Existing tests pass; cache identity/region findings remain |
| Exact/smart-case names, IDs, ambiguity, locals, empty results, rejected flags, file-local boundary and JSON | Level 1: `refs` CLI suite | Passed; new binding defects require additional CLI regressions |
| Terminal grouping, styling, locations and wrapping | Level 2: `level2_declarations::check_refs` through tmux and WezTerm | Appropriate verification present and passed |
| Keyboard encoding or hotkeys | No such feature requirement | Level 3 is unnecessary |

Verification performed:

- `just test symbols_occurrences symbols_reference_summaries symbols_cache cache_robustness refs:: -j4`: **118 passed**.
- `just test symbols_query stored_evidence -j4`: **40 passed**.
- `just test-l2`: **six entry points passed**; tmux and WezTerm ran actual terminal checks, while Kitty entry points returned through their unavailable-backend gate.
- `just check-tier-coverage tree-hugger`: **zero stranded tests**.
- Additional shipped-CLI sweeps exercised the source and cache tables above; valid Rust pattern reproductions were also compiled, and the JavaScript default behavior was checked with Node.

The library's declared `l1` target includes the occurrence, summary, query and cache modules. The CLI `symbols` target includes `refs` and cache robustness. Real-terminal targets declare `required-features = ["terminal-tests"]`, the area has a live `test-l2` recipe, and CI metadata enables that feature. Tests use static fixtures copied to isolated directories. No implementation or tests were changed by this review, so lint was not rerun. Cross-OS proof is left to CI and is not a readiness finding.
