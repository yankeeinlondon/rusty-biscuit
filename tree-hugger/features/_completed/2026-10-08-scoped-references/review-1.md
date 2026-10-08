---
$schema: feature-review.yaml
ready: false
findings:
  - title: Lexical lookup crosses boundaries that do not establish a binding
    priority: high
  - title: Named function expressions bind their names to outer declarations
    priority: high
  - title: Invocation classification loses calls across supported source forms
    priority: medium
  - title: Cache validation accepts inconsistent reference evidence
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-06T13:06:44-07:00
spec: 2026-10-08-scoped-references/spec.md
implemented: true
next: 2026-10-08-scoped-references/review-2.md
implemented_by: claude/opus
log: tree-hugger/features/2026-10-08-scoped-references/log.md
description: "A **feature** review of `2026-10-08-scoped-references/spec.md`"
feature: 2026-10-08-scoped-references/review-1.md
---

# Review: Scoped References

The feature is **not production ready**. Its overall analysis, selective-query integration, conservative treatment of unsupported languages, and report structure are implemented. However, valid source can produce established references to the wrong declarations, and a JavaScript `with` statement can produce false proof of recursion. Calls are also omitted from callers/callees reports for several source forms. The cache accepts some contradictory or incomplete evidence instead of recomputing it.

This is implementation review iteration 1. No earlier implementation review exists for this feature; recurrence is false. None of the findings requires a human decision. Implementation files were left unchanged.

## Findings

### High — Lexical lookup crosses boundaries that do not establish a binding

**Defect class:** lexical lookup treats every enclosing syntax scope as accessible, including scopes that prohibit capture or introduce dynamic lookup, and gives conditional bindings visibility in branches where they do not exist.

In the tree-hugger package, [ScopeTree::lookup_from](../../lib/src/analysis/scope/mod.rs) performs outward name lookup, and [Rust binding_sites](../../lib/src/analysis/scope/rust.rs) places conditional bindings. The traversal needs language-specific boundaries, not just parent links. [Walk::visit_ecma](../../lib/src/analysis/occurrences.rs) marks `with` as limited but does not prevent [Resolver::resolve_supported](../../lib/src/analysis/occurrences.rs) from claiming an established binding inside it.

Each row below was exercised through `hug symbols --json-full --no-cache <file>`. “Outer” means the declaration outside the tested boundary; IDs were compared with the declaration records in that report.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust inline module | `fn f() {} mod m { pub fn g() { f(); } }` | The call in `m::g` is `Scoped` to root `f`; its coverage is complete | An unqualified name cannot cross the module boundary without an import; no established target ID |
| Rust nested named function | `fn f() { let x = \|\| {}; fn g() { x(); } }` | `g` has an established reference to enclosing local `x` | Named items cannot capture enclosing locals; no established target ID |
| Rust nested constant | `fn f() { let x = \|\| {}; const Y: () = x(); }` | The ownerless constant region binds to enclosing local `x` | No established reference to an enclosing runtime local |
| Rust nested static | Replace `const Y` with `static Y` in the preceding source | Same established reference to enclosing local `x` | No established reference to an enclosing runtime local |
| Rust closure, control | `fn f() { let x = \|\| {}; let g = \|\| x(); }` | The closure binds to local `x` | Clean: closures may capture locals |
| Rust `if let` | `fn f() {} fn g(x: Option<fn()>) { if let Some(f) = x { f(); } else { f(); } f(); }` | Both the success branch and `else` call bind to the pattern's `f`; the final call binds to outer `f` | Only the success branch uses the pattern binding; `else` and the following call use outer `f` |
| Rust conditional chain | Add `&& true` after `if let Some(f) = x` | The same pattern binding incorrectly reaches the `else` call | The `else` call uses outer `f` |
| Rust `while let`, control | Replace the conditional with `while let Some(f) = x { f(); } f();` | The loop call uses the pattern binding; the following call uses outer `f` | Clean |
| Rust `for`, control | `fn f() {} fn g(xs: Vec<fn()>) { for f in xs { f(); } f(); }` | The loop call uses its binding; the following call uses outer `f` | Clean |
| TypeScript `with` | `function f(obj) { with (obj) { f(obj); } }` | The call is `Scoped` to `f` and recursion is `Yes`, despite limited coverage | `obj.f` can intercept lookup; the call must remain uncertain and recursion must be `Unknown` |
| JavaScript `with` | Same source in `.js` | Same false established self-call and `Yes` | Same uncertainty requirement |
| TypeScript/JavaScript default argument, control | `function f() {} function g(x = f()) { function f() {} }` | The default calls outer `f`, rather than the function declared inside the body | Clean |

The Rust examples that cannot capture a local deliberately test invalid bindings in otherwise parseable source: this tool accepts source that has not passed a compiler, and must not manufacture a proven edge for it. The `if let` example is valid source and demonstrates incorrect results without that condition.

**Why this matters:** established IDs feed both relationship summaries and `hug refs`. Merely marking a region limited cannot retract a wrong established edge, and a falsely established self-call overrides limitations to produce `Yes` recursion.

**Required repair:** enforce Rust item/module capture boundaries while preserving legal closure capture and accessible item names; restrict conditional bindings to their actual visibility regions; prevent dynamic `with` lookup from establishing lexical targets. Add Level-1 rows to the occurrence and recursion matrices and assert the resulting summaries and CLI reports, including the nested static and conditional-chain siblings above.

### High — Named function expressions bind their names to outer declarations

**Defect class:** expression-local declaration names are omitted from the binding inventory and occurrence exclusions, so declarations themselves and their internal calls resolve to a same-named outer function.

In the tree-hugger package, [ECMA binding_sites](../../lib/src/analysis/scope/ecma.rs) handles function declarations and class expressions but has no named function-expression or named generator-expression case. [ScopeTree::names_itself](../../lib/src/analysis/scope/mod.rs) also needs to keep every expression-local name inside the expression.

The shared reproduction is `function f() {} const g = function f() { f(); }; f();`, with `function*` substituted for the generator rows. Each source was run through the same complete-index CLI command as the preceding finding.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| TypeScript function expression | Named `function f()` assigned to `g` | Its declaration-name token is a `Usage` of outer `f`; its body call is `Scoped` to outer `f` | Exclude the declaration name; bind the body call to the expression-local name, with no outer ID |
| JavaScript function expression | Same shape in `.js` | Same incorrect declaration occurrence and outer call target | Same expression-local binding |
| TypeScript generator expression | Named `function* f()` assigned to `g` | Same incorrect declaration occurrence and outer call target | Same expression-local binding |
| JavaScript generator expression | Same shape in `.js` | Same incorrect declaration occurrence and outer call target | Same expression-local binding |
| TypeScript/JavaScript unnamed function and generator expressions, controls | Remove the expression's name from each reproduction | The inner and final calls both bind to outer `f` | Clean: no expression-local name shadows it |
| TypeScript/JavaScript class expression, controls | `class F {} const G = class F { m() { return F; } }; F;` | Inner `F` binds without an ID; final `F` binds to the outer class; expression name is excluded | Clean |
| TypeScript/JavaScript arrow parameter, controls | Outer `f`, then an arrow with parameter `f`, then an outer `f()` | The arrow call uses its parameter; final call uses outer `f` | Clean |

The final outer call is the positive control in every function/generator row: it correctly retains the outer ID. The expression's inner call must not appear alongside it as an established caller of that outer declaration.

**Required repair:** inventory and exclude named function/generator expression bindings using their actual pinned grammar node kinds. A binding without a declaration record may legitimately be `Scoped` with no ID; it must still block the outer declaration. Add Level-1 tests for both languages, ordinary functions and generators, including `hug refs f --callers` to prove only the final outer call is established.

### Medium — Invocation classification loses calls across supported source forms

**Defect class:** the invocation detector recognizes only selected direct parent shapes, treating transparent invocation wrappers and several languages' ordinary call syntax as value uses.

In the tree-hugger package, [is_call_target](../../lib/src/analysis/occurrences.rs) determines which occurrences callers/callees may list. It crosses qualified paths and Rust generic-function nodes, but not parentheses or TypeScript assertion wrappers. It also lacks the ordinary Perl and shell invocation shapes.

The sweep used a function named `f` with one call to itself in each language, first direct and then parenthesized where applicable, through the complete-index CLI. Unsupported binding resolution is allowed to remain `NameOnly`; the syntactic call still needs `kind: Call`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust | `fn f() { f(); }` → `fn f() { (f)(); }` | Direct: `Call`, recursion `Yes`; wrapped: `Usage`, recursion `Unknown` | Both are calls; preserve an established direct self-call through transparent parentheses |
| TypeScript | `function f() { f(); }` → `(f)()` | Direct: `Call`/`Yes`; wrapped: `Usage`/`Unknown` | Both are calls |
| JavaScript | Same direct/wrapped forms | Same loss | Both are calls |
| TypeScript assertion | `(f as (() => void))()` | `Usage` | `Call` |
| TypeScript non-null expression | `f!()` | `Usage` | `Call` |
| Python, Go, C, C++, C#, Swift, Scala, PHP, Lua | Direct `f()` and parenthesized `(f)()` in a callable body | Direct form: `Call`; wrapped form: `Usage` in every listed language | `Call` in both forms; binding confidence can remain limited |
| Java, control | Ordinary `f()` in a method body | `Call` | Clean; no parenthesized method-name control was used because that is not Java invocation syntax |
| Perl | `sub f { f(); }` | `Usage` | `Call` |
| Bash | `f() { f; }` | `Usage` | `Call` |
| Zsh | `f() { f; }` | `Usage` | `Call` |

**Why this matters:** these occurrences disappear from both `--callers` and `--callees`, which filter out `Usage`. Binding limitations in 13 languages do not excuse losing the call-site classification promised for all languages.

**Required repair:** add language-aware invocation forms and traverse transparent wrappers without treating the receiver or a call argument as the invocation target. Extend the all-language occurrence matrix with direct and wrapped controls and add CLI assertions for established and name-only call groups. Level 1 is sufficient for this classification requirement.

### Medium — Cache validation accepts inconsistent reference evidence

**Defect class:** structural JSON validation does not fully validate the semantic consistency of occurrence references and coverage, allowing a cache hit to change the public reference report.

In the tree-hugger package, [validate_evidence](../../lib/src/cache/stored_evidence.rs) checks candidate and owner references against retained records but never applies the corresponding check to a `Scoped` target. Its summary checks can catch some wrong targets with named owners, but ownerless references create no reverse edges and bypass that protection. Coverage validation also accepts duplicate regions and omission of a retained callable's empty region.

Reproduction: create `a.ts` containing `function f() {} function g() {} f();`, run `hug refs f a.ts --json --verbose` with an isolated `TMPDIR`, and edit the generated `tree-hugger/cache/hug/**/*.json` entry. Restore the unedited tool-produced entry before every mutation. The unedited entry reports one cache hit, no parses, and one incoming occurrence. Each mutation below changes one field or list element.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Ownerless `Scoped` target | Replace the call's `target.id` with the retained record ID of `g`, leaving written name `f` | Cache hit; incoming references to `f` drop from one to zero | Miss and recompute: the ID names a retained declaration with a different name |
| Ownerless `Scoped` target | Replace its ID with `"bogus"` | Cache hit; incoming reference disappears | Reject an ID that cannot be a generated declaration ID |
| Ownerless `Scoped` target | Replace its ID with `null` | Cache hit; incoming reference disappears | For this all-kind entry, no matching import or unreported binding supports losing the known target ID; recompute |
| Named-owner `Scoped` target, control | Rust `fn f() { f(); } fn g() {}`; replace self-call ID with `g` or `"bogus"` | Miss because the stored reference edge has no supporting occurrence | Clean rejection, although indirect and dependent on summary contents |
| Target name, control | Replace written `f` with `g`, keeping the source span | Miss | Clean: name must match source text |
| Coverage region | Duplicate one named callable's coverage row | Cache hit | Miss: duplicate region identity is not valid evidence |
| Coverage region | Remove empty `g`'s named coverage row, leaving the whole-file row | Cache hit | Miss: callable coverage is required even without occurrences |

Do not reject every ID absent from retained records: narrowed entries legitimately refer to omitted declarations. Validate what the entry can prove, including retained record identity, valid ID representation, duplicate region identities, and required coverage for retained body-bearing callables. Preserve legitimate no-ID import and unreported-binding cases.

#### Input robustness matrix sweep

The strict cache reader was tested through the shipped CLI using a tool-written Rust entry from `fn f() { f(); }\nfn g() {}\n`. Each cell restored the original entry and made one edit; the control was an actual hit. `M` means a miss with parsing/recomputation; `H` means a hit. Scalar/object fields have no element-type cell. Duplicate tests duplicated the field at its containing object, not merely a duplicate list value.

| Shape | Capabilities | Occurrences | Coverage | Confidence | Target ID | Candidates | Occurrence span | Owner | Owner span | Coverage state | Reasons | Coverage span |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Absent | M | M | M | M | M | M | M | M | M | M | M | M |
| Null | M | M | M | M | M | M | M | M | M | M | M | M |
| Wrong whole type | M | M | M | M | M | M | M | M | M | M | M | M |
| One wrong element | M | M | M | — | — | M | — | — | — | — | M | — |
| Every element wrong | M | M | M | — | — | M | — | — | — | — | M | — |
| Empty | M | M | M | M | M | H | M | M | M | M | H | M |
| Duplicate key | M | M | M | M | M | M | M | M | M | M | M | M |
| Trailing garbage | M | M | M | M | M | M | M | M | M | M | M | M |

The two empty-list hits are correct controls: a `Scoped` occurrence has no candidates, and complete coverage has no reasons. Null owners/IDs are legitimate in other contexts; this row tests a named-owner established self-call, whose null edits contradict the remaining evidence. Trailing garbage is a document-level mutation, repeated with each field's control, rather than a distinct property of those fields.

These structural cells are clean. The inconsistent-but-well-typed cells in the preceding table are the defect. Existing [symbols_cache tests](../../lib/tests/l1/symbols_cache.rs) cover round trips, projection, capability widening, corrupt documents, and general entry fields, but do not contain the planned occurrence-evidence mutation matrix asserting these public outcomes. Add that matrix permanently, including ownerless targets, duplicate/omitted coverage, nullable controls, candidate consistency and narrowed-entry controls. The review's command-line experiments are not regression coverage.

## Requirement verification

| Requirement | Strongest relevant verification inspected/run | Assessment |
|---|---|---|
| Language visibility, shadowing, aliases, categories, overloads, receiver uncertainty | Level 1: occurrence and relationship matrices; CLI reproductions above | Gaps in the first two findings |
| Name spans, repeated sites, exclusion of declarations/comments/strings, anonymous/named ownership | Level 1: occurrence tests | Existing cases pass; named expressions and wrapped invocations need regression rows |
| Direct recursion, limited coverage, imports, nested bodies | Level 1: recursion matrix and narrow-query equivalence | Existing cases pass; false `with` self-call and parenthesized-call gaps remain |
| Whole-file evidence, filtering, projection, cold/warm/widened cache equality and skipped work | Level 1: query/reference tests and cache tests | Existing cases pass; semantic cache validation gap remains |
| Exact/smart-case names, IDs, ambiguity, empty reports, local targets, conflicting flags, file-local boundary, JSON | Level 1: CLI `refs` tests | Existing cases pass |
| Terminal headings, grouped rows, styles, hyperlinks and wrapping | Level 2: real-terminal declaration tests exercise `refs` through tmux/WezTerm | Appropriate level present and passing; Kitty entry points skip on this host |
| Keyboard/input encoding | Not a feature requirement | Level 3 is not required |

Commands run:

- `just test symbols_occurrences symbols_reference_summaries stored_evidence refs::`: 48 passed.
- `just test symbols_cache`: 36 passed.
- `just test-l2`: six entry points passed; tmux and WezTerm exercised real terminals, while the two Kitty entry points skipped for an unavailable backend.
- `just check-tier-coverage tree-hugger`: no stranded tests.

The reference tests are compiled by the declared library `l1` and CLI `symbols` test targets. The real-terminal tests have declared targets, the `terminal-tests` feature, a live `test-l2` recipe, and CI metadata enabling that feature. No tests or implementation changes were added by this review. Lint was not rerun because only the requested review and spec metadata were edited. Cross-OS proof is left to CI and is not a readiness finding.

The specification's schema major bump, occurrence capability prerequisites, shared extraction path, conservative language support and file-local reporting boundary are present. The findings concern incorrect results within those implemented contracts, rather than a request to expand into compiler-backed or cross-file resolution.
