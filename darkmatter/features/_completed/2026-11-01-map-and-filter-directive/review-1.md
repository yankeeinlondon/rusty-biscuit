---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Selection guards lose refinements across destructured and object bindings'
  - 'R1-F2 (high): Runtime diagnostic projections drop authored locations or iteration identity'
  - 'R1-F3 (high): Static collection rejection discards earlier recovery warnings'
  - 'R1-F4 (low): Expression reference retains obsolete lookup and file-guard behavior'
human_review: true
human_review_items:
  - "Confirm after the repair cycle: Register the reserved local root in Phase 2; this was subsequently completed."
  - "Confirm after the repair cycle: Wire budgets, builders, and diagnostic registries in their later implementation phases."
  - "Confirm after the repair cycle: Implement collection analysis with the scanner instead of introducing an empty stub."
  - "Confirm after the repair cycle: Allow the existing render function’s argument count to unblock lint without unrelated refactoring."
  - "Confirm after the repair cycle: Register the collection operation before connecting its executor and CLI control; both subsequently landed."
  - "Confirm after the repair cycle: Parse callback bodies in interpolation mode, where || is the existing fallback operator."
  - "Confirm after the repair cycle: Reserve local as specified, retain fm.local access, reject colliding host globals, and adjust old test data."
  - "Confirm after the repair cycle: Keep existing public page/shell scans compatible and expose collections through their own scanner adapters."
  - "Confirm after the repair cycle: Accept bare asc/desc as well as quoted sort-order values."
  - "Confirm after the repair cycle: Preserve historical unquoted page/shell closer matching while requiring collection container matching."
  - "Confirm after the repair cycle: Defer general expression diagnostic fields and retain collection spans in collection-owned records; remaining gaps are finding R1-F2."
  - "Confirm after the repair cycle: Leave destructured-field and object-member guard identities separate; finding R1-F1 requires repairing this."
  - "Confirm after the repair cycle: Use one traversal for conditional and collection regions when collections are active."
  - "Confirm after the repair cycle: Use an offset-aware lookup to retain iteration locals through later document stages."
  - "Confirm after the repair cycle: Charge materialized bytes once per enclosing collection expansion, including inserted child output."
  - "Confirm after the repair cycle: Leave an expression-lookup budget accessor to the separate expression-function feature; directive nested composition already shares its budget."
  - "Confirm after the repair cycle: Keep ordinary repeated-body errors on existing error variants; finding R1-F2 requires preserving their iteration context."
  - "Confirm after the repair cycle: Rely on body provenance for authored ranges instead of duplicating those ranges in retained scopes."
  - "Confirm after the repair cycle: Introduce a distinct local shell-placeholder timing because lifecycle deferred placeholders require barrier values."
  - "Confirm after the repair cycle: Use the shared body type scopes for ordinary shell sites, shell blocks, and discovery."
  - "Confirm after the repair cycle: Preserve current quoted transclusion recognition and propagate indentation rather than adding active quote-prefixed file directives."
  - "Confirm after the repair cycle: Use a second collection-expanded discovery pass, ignore selection gates there, and evaluate target-producing map callbacks."
  - "Confirm after the repair cycle: Identify repeated child sites by authored line, original item path, and explicit parameters; preserve the existing empty-node exception."
  - "Confirm after the repair cycle: Rewalk repeated child files with different explicit parameters while retaining existing unscoped duplicate handling."
  - "Confirm after the repair cycle: Keep reference validation passive and omit per-item expansion of local-dependent hyperlink destinations."
  - "Confirm after the repair cycle: Include object insertion order in semantic cache and graph identities; keep environment and policy sets order-insensitive."
  - "Confirm after the repair cycle: Measure cached subtree budget use with per-handle ledgers to avoid concurrent counter subtraction."
  - "Confirm after the repair cycle: Deliver typed collection failures directly to consumers waiting for the same child computation."
  - "Confirm after the repair cycle: Omit budget source locations; finding R1-F2 requires adding them."
  - "Confirm after the repair cycle: Add a CLI collection-error presentation wrapper; finding R1-F2 requires library-owned provenance to survive every projection."
  - "Confirm after the repair cycle: Offer visible local names without new object-member completion; use enclosing names for incomplete headers and keep recovery off in editor analysis."
  - "Confirm after the repair cycle: Correct escaped callback diagnostic offsets by projecting decoded spans back to authored bytes."
  - "Confirm after the repair cycle: Update the frozen editor corpus baseline for obsolete map syntax instead of rewriting the frozen document."
  - "Confirm after the repair cycle: Leave earlier recovery warnings lost on static collection failure; finding R1-F3 requires preserving them."
  - "Confirm after the repair cycle: Keep acceptance tables in Rust with a generated test per row and stable executable document fixtures."
  - "Confirm after the repair cycle: Document the shipped iteration_limits getter and actual discovery modules instead of adding redundant getters or a placeholder module."
  - "Confirm after the repair cycle: Report the unrelated opt-in draft-prompt schema failure as informational."
  - "Confirm after the repair cycle: Record WSL2 evidence as unproven after a transport timeout; normal CI evidence remains outside readiness for this review."
  - "Confirm after the repair cycle: Record earlier test omissions for child contexts, root-only overrides, concurrent reuse, and quote-prefixed output; retain existing context and override contracts without expanding quoted directive syntax."
  - "Repair cycle 1 (R1-F1): an identity map such as `p => p` does not carry a guard fact about `p.members` into the result binding; only the mapping expression's own type flows to a map result."
  - "Repair cycle 1 (R1-F2): iteration provenance is one `MarkdownError::InIteration` wrapper (code and recoverability are the inner error's); budget errors are located in place instead of wrapped."
  - "Repair cycle 1 (R1-F2): a page-block gate message keeps its document-body-relative `line N` text; the record's `source`/`span` carry the authored file line."
  - "Repair cycle 1 (R1-F2): a repeated shell-block failure is located at the block opener, not the individual command line."
  - "Repair cycle 1 (R1-F2): in claudine, a shell failure inside a repeated copy rides `ComposeFailed` (code still `composition.shell_expansion`) instead of `ShellExpansionFailed`, so its location and item path survive."
  - "Repair cycle 1 (R1-F3): with a later static rejection, earlier recovery warnings are reported ahead of it, following the precheck's existing rule for earlier warnings, instead of a blocked warning becoming the failure."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-28T04:55:01-07:00
spec: 2026-11-01-map-and-filter-directive/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-map-and-filter/darkmatter/features/2026-11-01-map-and-filter-directive/log.md
description: 'A **feature** review of `2026-11-01-map-and-filter-directive/spec.md`'
feature: 2026-11-01-map-and-filter-directive/review-1.md
next: 2026-11-01-map-and-filter-directive/review-2.md
---

# Review 1 — Map and Filter Directives

The feature is **not production ready**. Four findings need repair. None requires a human decision before repair: the specification supplies a safe direction for each. This is the first implementation review, so there are no carried findings.

## Findings

### R1-F1 (high): Selection guards lose refinements across destructured and object bindings

**Authority — spec, R3:** “Apply selection facts to original-item bindings, using their binding identity even when two callbacks use different names.” R2 also permits a source whose iterable shape is established by shared guard facts. A valid retained object stops being provably iterable when one callback destructures its field and another accesses that same field through an object parameter.

The defect class is inconsistent identity for two ways of naming the same original item field. In the darkmatter library, [ItemFrame](../../lib/src/markdown/compose/expression/scope/iterable.rs) assigns a separate local ID to a destructured field, while [Subject](../../lib/src/markdown/compose/expression/scope/projection.rs) represents an object member as the whole-item ID plus a member path. Consequently the shared narrowing engine cannot connect them.

This realistic document is rejected with `dm.collection.untyped_map_source` at `members`, although the selection proves its iterable type:

```md
---
$schema:
  data: "{ members: any(required) }[](required)"
data: [{members: [a, b]}]
---
::filter data "{members} => true" filter="p => is_array(p.members)"
::map members
- {{i}}
::end-map
::end-filter
```

Expected output is `- a` and `- b`, on separate lines. Changing only the guard pattern to `{members} => is_array(members)` succeeds.

The sweep used the shipped `md compose --output report-json` binary in a temporary directory. Each row was run with `filter=`, `while=`, and `until=`; `until` used the negated predicate to retain the same item. Original-field proofs are required here; no facts are requested to carry automatically onto a new map result.

| Child ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Filter body after additional selection / while / until | Destructured body; guard reads `p.members` | Nested `::map members` rejected in all three slots | Render both members |
| R1-F1.2 | Filter body after additional selection / while / until | Object body; guard destructures `members` | Nested `::map p.members` rejected in all three slots | Render both members |
| R1-F1.3 | Map callback and its result type after selection / while / until | Mapping `{members} => {members}`; guard reads `p.members` | Nested `::map members` rejected in all three slots | Mapping expression can use the proven input field type; render both members |
| R1-F1.4 | Filter body, clean control | Object parameters `p` and `q`; guard reads `q.members` | Both members render in all three slots | Both members render |
| R1-F1.5 | Both directive bodies, clean control | Both callbacks destructure `members` | Both members render in all three slots | Both members render |
| R1-F1.6 | Later sort / while / until callbacks and map expression | A file field checked through an object parameter, then read through a destructured parameter with `frontmatter(path, 'title')` | Successful composition, including with uncertain-call warnings denied | Successful composition; no additional runtime failure observed |
| R1-F1.7 | Editor and passive call-check scope consumers | Both consume the same `BodyScopes` / `ItemFrame` projection | Code inspection confirms the same identity mismatch reaches their shared scope | Use one canonical original-field identity throughout |

The positional filter predicate uses the same selection helper as the additional predicate; the identity defect is in the shared frame, not predicate evaluation. Repair the shared subject representation so destructured fields alias the corresponding original-item member path. Preserve distinct identities for mapped results and shadowed bindings. Add public analysis/composition cases for both crossing directions, including selection and stopping callbacks; the existing same-pattern cases do not protect this boundary.

### R1-F2 (high): Runtime diagnostic projections drop authored locations or iteration identity

**Authority — spec, R9:** “Library-owned typed diagnostics preserve the underlying error, authored file, line/column, expression span, and nested iteration path.” The diagnostics/editor acceptance row also requires original positions and library/CLI/report agreement. Several normal runtime failures retain only part of that information.

The defect class is loss of provenance when a scoped failure is converted into a budget, ordinary body, or reporting error. In the darkmatter library, [CollectionError::Budget](../../lib/src/markdown/compose/collection_blocks/error.rs) has no site, [structural conditions](../../lib/src/markdown/compose/collection_blocks/engine.rs) return ordinary page-block errors without attaching the current path, and the later-stage errors do not carry the retained scope’s iteration. The CLI wrapper can add a path only when the library error still owns it.

For budget reproductions, use a declared `data: string[]`, body `x`, and two items with `--max-iteration-visits 1`. For runtime expression reproductions, use `data: ["./ok.md", "./missing.md"]`, create `ok.md` with `title: good`, and evaluate `frontmatter(i, 'title')` in each site. The second item fails. Every table row was exercised through `md compose --output report-json` for **both** map and filter, except the explicitly identified library projection inspection.

| Child ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Item-visit reservation | Two items, visit limit 1 | Correct `data[1]`, but `source: null` | Authored collection location and original item path |
| R1-F2.2 | Authored-body copy | One 51-byte body, byte limit 1 | Correct `data[0]`, but `source: null` | Authored copy location and path |
| R1-F2.3 | Interpolation insertion | One 120-byte value; `{{i}}` body; byte limit 10 | Expansion failure has `source: null` | Authored interpolation span and path |
| R1-F2.4 | Ordinary shell output insertion | Approved `echo {{i}}`, 120-byte value, byte limit 30 | Expansion failure has `source: null` | Authored shell site and path |
| R1-F2.5 | Shell-block output insertion | Approved `echo {{i}}` block, byte limit 50 | Expansion failure has `source: null` | Authored shell-block site and path |
| R1-F2.6 | Child output insertion | `::file ./child.md`, 120-byte child, byte limit 30 | Expansion failure has `source: null` | Authored transclusion site and path |
| R1-F2.7 | Cached child budget replay | Same child included twice; child visits one item; request visit limit 1 | Failure has neither source nor iteration path | Identify the replaying child site and preserve applicable original iteration identity |
| R1-F2.8 | Ordinary repeated-body interpolation | `{{frontmatter(i, 'title')}}` | Correct authored line 7, no `data[1]` | Both location and original item path |
| R1-F2.9 | Repeated page-block gate | `::block when="frontmatter(i, 'title')"` | `source: null`, body-relative “line 2,” no item path | Authored file line 7 and `data[1]` |
| R1-F2.10 | Nested collection gate | Inner `::map ... when="frontmatter(i, 'title')"` | Same missing location/path as page gate | Authored gate location and enclosing `data[1]` |
| R1-F2.11 | Repeated transclusion target expression | `::file {{frontmatter(i, 'title')}}` | Authored line 7, no item path | Both authored target span and `data[1]` |
| R1-F2.12 | Repeated transclusion condition | `::file ./ok.md when="frontmatter(i, 'title')"` | `source: null`, body-relative “line 2,” no item path | Authored condition location and `data[1]` |
| R1-F2.13 | Shell argument validation | Approved `echo {{i}}` with an actual null item | Missing-value failure names a line, but source is null and item path absent | Authored site plus `data[0]` |
| R1-F2.14 | Shell-block argument validation | Same null argument inside a repeated shell block | Source is null and item path absent | Authored command site plus `data[0]` |
| R1-F2.15 | Callback failure, clean CLI control | `i => frontmatter(i, 'title')` | CLI reports authored location and `data[1]` | Retain both |
| R1-F2.16 | Library diagnostic projection of that callback | `report_diagnostics::error_diagnostics` reads the typed collection error | Source is projected, but the generic diagnostic has no span/path fields and its message uses the path-free error display; CLI repairs the message separately | Preserve typed provenance in the library record for every host |

These sites cover structural reservation, later inserted bytes, cached replay, scoped expression/condition consumers, and library/CLI projections. Static errors are correctly site-based and have no exposing iteration; they should remain reported once. Nested callback paths are already covered by the existing `nested_failures_report_every_enclosing_position` test and should remain intact.

The smallest coherent repair is library-owned contextual failure data populated at the authored site, with typed span and iteration fields carried into the common diagnostic projection. Reuse `BodyOrigin` and `ScopeRanges`; do not infer identity from rendered messages. Budget counters can remain context-free: enrich their returned failure at the charge caller. Preserve ordinary body recovery rules while attaching context. Tighten the existing budget and body-error tests to assert provenance, then add representative library/CLI parity checks. This requires no new permission model or human ruling.

### R1-F3 (high): Static collection rejection discards earlier recovery warnings

**Authority — spec, R9:** “Preserve warnings collected before failure in the compose report.” In the darkmatter library, [analyze_collection_blocks](../../lib/src/markdown/compose/collection_blocks/analysis.rs) collects recovered sources but returns only the later error; [the call precheck](../../lib/src/markdown/compose/call_precheck.rs) receives recovered sources only on success. An earlier authored recovery therefore vanishes if any later collection has a static typing failure.

The defect class is dropping the diagnostic prefix when a batch validator returns an error. A user composing a document with one waived undeclared source and a separate invalid scalar source receives only the scalar error, although fixing that error immediately reveals the earlier warning again.

Reproduce with `unknown: [1]`, declared `scalar: number(required)` set to `2`, an initial `::map unknown` region, and a later `::filter scalar` region. Run `md compose doc.md --allow untyped-map-variables --output report-json`. The report contains only `dm.collection.invalid_source`; it must also retain the earlier `dm.collection.untyped_map_source` warning.

| Child ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Recovered map source before static source rejection | Waive untyped map, then iterate known scalar | Only static error | Earlier map warning plus static error |
| R1-F3.2 | Recovered filter source before static source rejection | Waive untyped filter, then iterate known scalar | Only static error | Earlier filter warning plus static error |
| R1-F3.3 | Recovered map source before runtime pattern failure, clean control | Later `any[]` scalar item destructured as `{x}` | Warning and runtime error both retained | Retain both |
| R1-F3.4 | Recovered filter source before runtime pattern failure, clean control | Same later runtime failure | Warning and runtime error both retained | Retain both |
| R1-F3.5 | Recovery without later failure, clean control | Same waived unknown source alone | One warning and successful document | One warning and successful document |
| R1-F3.6 | Pattern, sort-key, and protected-name static rejection branches | Earlier recovered map/filter followed by, respectively, `{missing}` on a number item, `sort="i => true"`, or a protected `ctx` parameter | All six CLI cases omit the earlier warning and return only the corresponding pattern, sort, or callback error | Earlier warning plus the selected static error |
| R1-F3.7 | Call-precheck prefix filtering, clean existing mechanism | Earlier advisories before a chosen rejection | Existing code filters by authored order and carries `earlier` warnings | Feed collection recoveries into that mechanism rather than bypassing it |

Return the analysis prefix with its failure, or provide an analysis result containing diagnostics and the chosen rejection, then reuse the existing authored-order filtering. Avoid re-evaluating expressions. Add in-process public failure-report tests for both recovery families; the current runtime-only preservation test does not cover static rejection.

### R1-F4 (low): Expression reference retains obsolete lookup and file-guard behavior

**Authority — spec, Delivery and Documentation Requirements, item 4:** synchronize expression/local-scope topic documentation wherever delivered behavior changes its contract. R4 establishes nearest-local bare lookup, and R2 explicitly makes both file predicates establish file suitability.

The current [expression reference](../../docs/topics/darkmatter-expressions.md) still says “An unprefixed name is a document property and nothing else” and describes `file_exists(x)` as proving only a present string, including the example in its predicate-registration table. The implementation now reserves `local`, resolves bare lexical bindings first, and registers `file_exists` with the same true-side file suitability as `is_file_type`; its false side proves nothing. Readers using this reference get contradictory guidance about whether a guarded file call is valid and what a bare name reads inside an iteration.

Update those lookup and guard sections, including the registration-table example and reserved-root table, to describe the delivered behavior. Keep the existing explanation that checking a freshly evaluated read does not prove a separate later read. This is a documentation repair, not a request for stronger runtime guarantees.

## Verification and scope

- Ran `just test collection` in the darkmatter area: **395 passed**, 13,057 outside-filter tests skipped. The run included library, CLI, and DMLS collection tests. The linker emitted its existing large unwind-table warning; there were no test failures.
- Ran `just check-tier-coverage darkmatter`: **0 stranded tests**. The collection modules are declared by their consolidated L1 binaries; the normal area recipe enables the relevant instrumentation and CLI fixture features.
- Ran additional shipped-CLI reproductions in isolated temporary directories for the findings and their clean controls, using `--output report-json`. These checks did not change production source or persistent shell approvals.
- Reviewed the shared parser, original-item iterator, type frames, structural executor, scope lookup, budget charge sites, child discovery/cache integration, reporting, editor adapter, test corpus, and changed documentation. This is a focused review, not a claim that every existing test was rerun.

| Requirement family | Strongest relevant evidence reviewed | Appropriate level |
| --- | --- | --- |
| Header grammar, arrows, patterns, source typing, locals, original filter values | In-process parser, analysis, composition, and shared-corpus tests | L1 |
| Stable sorting, lazy selection, stopping, document-wide callback order | In-process composition with controlled effect handlers/counters | L1 |
| Data provenance, nested scopes, children, dynamic discovery and authorization | Public composition/preflight tests plus hermetic CLI cases | L1 |
| Shell argument boundaries, typed templates, existing cache controls | Native argument-recorder CLI tests and public shell/preflight tests | L1 |
| Shared limits, nesting, copied output, cache replay | Public budget boundary tests and CLI checks; diagnostic omissions remain R1-F2 | L1 |
| Editor diagnostics, hover, visible locals, folding and tokens | In-process LSP requests and analysis effect tripwire | L1 |
| Shipped documentation grammar and normal command wiring | Passive documentation corpus, stable executable fixtures, CLI collection tests | L1 |

There is no terminal key, emulator rendering, or browser interaction requirement in this feature that calls for L2/L3 evidence. Cross-OS evidence is left to the normal CI workflow and does not block readiness here. The implementation log’s unreachable WSL2 host is informational.

The input robustness matrix does not apply: these changes add a text grammar and typed expression consumers, not a new configuration/document loader. Existing frontmatter loading is not redefined by this feature. CLI positive-limit parsing has its explicit zero, negative, overflowing, and unlimited-value rejection tests.

Both `docs/inline/map.md` and `docs/inline/filter.md` have `kind: canonical-documentation`, contain no Planned notice, and describe their different body values consistently. Their syntax examples pass the passive corpus. Their broad diagnostic promises will become accurate when R1-F2 is repaired; do not weaken the spec’s provenance contract to match the omissions.

## Implementation decisions (informational)

The two requested readers examined the implementation log and the spec/plan in parallel. The spec’s product rulings are settled; bounded autonomy covers diagnostic identities, API spelling, and internal organization. Every item below can be accepted as recommended, or repaired within the existing contract. None requires stopping for a human. The author may confirm or reverse these choices after the repair cycle; the frontmatter contains one-line counterparts.

1. Register the reserved local root in Phase 2; this was subsequently completed.
2. Wire budgets, builders, and diagnostic registries in their later implementation phases.
3. Implement collection analysis with the scanner instead of introducing an empty stub.
4. Allow the existing render function’s argument count to unblock lint without unrelated refactoring.
5. Register the collection operation before connecting its executor and CLI control; both subsequently landed.
6. Parse callback bodies in interpolation mode, where || is the existing fallback operator.
7. Reserve local as specified, retain fm.local access, reject colliding host globals, and adjust old test data.
8. Keep existing public page/shell scans compatible and expose collections through their own scanner adapters.
9. Accept bare asc/desc as well as quoted sort-order values.
10. Preserve historical unquoted page/shell closer matching while requiring collection container matching.
11. Defer general expression diagnostic fields and retain collection spans in collection-owned records; remaining gaps are finding R1-F2.
12. Leave destructured-field and object-member guard identities separate; finding R1-F1 requires repairing this.
13. Use one traversal for conditional and collection regions when collections are active.
14. Use an offset-aware lookup to retain iteration locals through later document stages.
15. Charge materialized bytes once per enclosing collection expansion, including inserted child output.
16. Leave an expression-lookup budget accessor to the separate expression-function feature; directive nested composition already shares its budget.
17. Keep ordinary repeated-body errors on existing error variants; finding R1-F2 requires preserving their iteration context.
18. Rely on body provenance for authored ranges instead of duplicating those ranges in retained scopes.
19. Introduce a distinct local shell-placeholder timing because lifecycle deferred placeholders require barrier values.
20. Use the shared body type scopes for ordinary shell sites, shell blocks, and discovery.
21. Preserve current quoted transclusion recognition and propagate indentation rather than adding active quote-prefixed file directives.
22. Use a second collection-expanded discovery pass, ignore selection gates there, and evaluate target-producing map callbacks.
23. Identify repeated child sites by authored line, original item path, and explicit parameters; preserve the existing empty-node exception.
24. Rewalk repeated child files with different explicit parameters while retaining existing unscoped duplicate handling.
25. Keep reference validation passive and omit per-item expansion of local-dependent hyperlink destinations.
26. Include object insertion order in semantic cache and graph identities; keep environment and policy sets order-insensitive.
27. Measure cached subtree budget use with per-handle ledgers to avoid concurrent counter subtraction.
28. Deliver typed collection failures directly to consumers waiting for the same child computation.
29. Omit budget source locations; finding R1-F2 requires adding them.
30. Add a CLI collection-error presentation wrapper; finding R1-F2 requires library-owned provenance to survive every projection.
31. Offer visible local names without new object-member completion; use enclosing names for incomplete headers and keep recovery off in editor analysis.
32. Correct escaped callback diagnostic offsets by projecting decoded spans back to authored bytes.
33. Update the frozen editor corpus baseline for obsolete map syntax instead of rewriting the frozen document.
34. Leave earlier recovery warnings lost on static collection failure; finding R1-F3 requires preserving them.
35. Keep acceptance tables in Rust with a generated test per row and stable executable document fixtures.
36. Document the shipped iteration_limits getter and actual discovery modules instead of adding redundant getters or a placeholder module.
37. Report the unrelated opt-in draft-prompt schema failure as informational.
38. Record WSL2 evidence as unproven after a transport timeout; normal CI evidence remains outside readiness for this review.
39. Record earlier test omissions for child contexts, root-only overrides, concurrent reuse, and quote-prefixed output; retain existing context and override contracts without expanding quoted directive syntax.

The timing-only deferrals in the early phases are closed by the later implementation. Remaining omissions that violate the contract are explicitly covered by the findings above, rather than reserved for the author. The omitted per-item hyperlink-validation inventory and local member completion are not made new blocking requirements: the spec does not separately promise those additional behaviors. Preserving historical quoted `::file` examples likewise does not require adding a new quoted transclusion grammar. The implementation log’s earlier test omissions are recorded as limitations; they do not by themselves establish a defect or a verification-level mismatch.
