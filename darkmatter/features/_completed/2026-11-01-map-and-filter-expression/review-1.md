---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (medium): Preflight mistakes shadowing callback locals for pending frontmatter"
human_review: true
human_review_items:
  - "Informational: Ordinary calls retain their existing syntax-tree node; calls containing callbacks use a separate node. No action is required."
  - "Informational: Expressions reuse the directive parser, scopes, types, limits, and file predicates; the originally deferred runtime phase is complete. No action is required."
  - "Informational: A failing root test-wrapper invocation was replaced with equivalent feature-enabled package recipes; wrapper repair remains outside this feature. No action is required."
  - "Informational: Host-specific protected names are checked at lifecycle execution, when all host globals are known. No earlier preparation guarantee was added. No action is required."
  - "Informational: Unrelated opt-in prompt-schema and review-runner failures were recorded rather than repaired here; fail-fast left some cases unrun. No action is required for this feature."
  - "Informational: Invalid callback syntax and placement share the existing diagnostic identity. No action is required."
  - "Informational: Callback declaration locations are retained in immutable type frames without requiring spans from existing callers. No action is required."
  - "Informational: Editor hover preserves known literal types rather than widening them. No action is required."
  - "Informational: Editor completion gives local bindings priority only when locals are available. No action is required."
  - "Informational: Callback names and arrows use existing variable and operator highlighting categories. No action is required."
  - "Informational: Completion documentation promises binding names and their types; member types are available through hover. No action is required."
  - "Informational: Downstream verification targeted affected expression consumers and small dependent suites; unrelated large suites were omitted. No action is required."
  - "Informational: An outdated opt-in schema guard was updated for existing recursive schema types without changing schema behavior. No action is required."
  - "Informational: Existing ignored tests were retained; no new exclusions were introduced. No action is required."
  - "Informational: Local verification claims macOS results; cross-platform evidence is left to CI. No action is required."
  - "Informational: Concrete limit builders, nullable signatures, and diagnostic context follow existing shared machinery, as the spec permits. No action is required."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-29T01:48:38-07:00
spec: 2026-11-01-map-and-filter-expression/spec.md
implemented: true
description: "A **feature** review of `2026-11-01-map-and-filter-expression/spec.md`"
feature: 2026-11-01-map-and-filter-expression/review-1.md
log: '/Volumes/coding/wt/rusty-biscuit/feat-map-and-filter/darkmatter/features/2026-11-01-map-and-filter-expression/log.md'
implemented_by: codex/gpt-6.1-sol
next: 2026-11-01-map-and-filter-expression/review-2.md
---

# Review 1

The feature is not production ready because preflight can reject a valid callback that shadows frontmatter. One medium finding needs repair. No finding needs an author decision before repair.

## Findings

### R1-F1 (medium): Preflight mistakes shadowing callback locals for pending frontmatter

**Authority — spec:** Acceptance criterion 4 requires “Explicit `local.*`, explicit `fm.*`, bare reads, null shadowing, nested reuse, and scope restoration agree.” The scope contract also says locals may shadow frontmatter, and the file-discovery contract permits targets computed from discovery-supported expressions. A callback's literal input is available during discovery even when an unrelated document property with the same name awaits execution.

**Defect class:** Dependency classification loses the enclosing lexical scope when inspecting a nested function argument, so it classifies a local read as a document-property read.

In the **darkmatter** package, [dynamic_nested_dependency](../../lib/src/markdown/compose/preflight/collect.rs#L2162) receives each `as_markdown` argument from [visit_calls](../../lib/src/markdown/compose/preflight/collect.rs#L1974), without the callback bindings surrounding that call. It then checks the isolated argument's root against pending shell properties and properties whose values discovery cannot observe. The earlier whole-expression read inventory correctly excludes callback locals; inspecting the extracted argument loses that information. Both the body and frontmatter branches of the preflight shape checker consume this mistaken classification and reject the document.

Minimal reproduction, adapted from the existing hermetic nested-composition fixture:

```markdown
---
part: "$(echo pending)"
---
{{ map(['hello'], part => as_markdown(part)) }}
```

Load this document from a temporary directory and call the public `Markdown::compose_preflight` with a request anchored there. It returns a dynamic-command-shape error saying the callback depends on `frontmatter.part`, which frontmatter shell expansion resolves. Nothing executes. Changing only the argument to `as_markdown(local.part)` succeeds. Changing the callback parameter to `item` also succeeds. The bare read refers to the literal `hello`, so it should discover the same nested content as either control.

The combination of nested composition and a same-named pending property is uncommon, which makes this medium priority. It is valid authoring syntax, and changing unrelated frontmatter should not break it.

#### Class sweep

The same public preflight reproduction was run with one change per case. “Accept” means discovery succeeds without executing a shell command; “reject” means the appropriate unknown-shape error. Each failing row needs a regression assertion in the repair. Clean rows protect the distinction between local and document reads.

| Child ID | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Body nested-content check, map | Identifier `part => as_markdown(part)`; pending document `part` | Reject | Accept |
| R1-F1.2 | Body nested-content check, filter | Identifier `part => as_markdown(part)`; same pending property | Reject | Accept |
| R1-F1.3 | Body nested-content check, map | `{part} -> as_markdown(part)` over `[{part: 'hello'}]` | Reject | Accept |
| R1-F1.4 | Body nested-content check, filter | Same destructuring and second arrow | Reject | Accept |
| R1-F1.5 | Nested callback capture | Outer `part`, inner `n`, inner `as_markdown(part)` | Reject | Accept |
| R1-F1.6 | Frontmatter nested-content check, map | Whole-value `result` containing the same map call | Reject at `frontmatter.result` | Accept |
| R1-F1.7 | Frontmatter nested-content check, filter | Whole-value `result` containing the same filter call | Reject at `frontmatter.result` | Accept |
| R1-F1.8 | Unobserved-property classification | Replace document `part` with `"{{ can_execute('git') }}"`; keep local bare argument | Reject as an unobserved document dependency | Accept |
| R1-F1.9 | Explicit local control, map | `as_markdown(local.part)` with pending document property | Accept | Accept |
| R1-F1.10 | Explicit local control, filter | Same explicit local argument | Accept | Accept |
| R1-F1.11 | Explicit outer-local capture control | Inner callback reads `as_markdown(local.part)` | Accept | Accept |
| R1-F1.12 | Explicit local with unobserved document property | `as_markdown(local.part)` with `can_execute` property | Accept | Accept |
| R1-F1.13 | Explicit document control, pending | `as_markdown(fm.part)` | Reject | Reject |
| R1-F1.14 | Explicit document control, unobserved | `as_markdown(fm.part)` with `can_execute` property | Reject | Reject |
| R1-F1.15 | Nonshadowing callback control | `item => as_markdown(item)` | Accept | Accept |
| R1-F1.16 | Ordinary frontmatter dependency inventory | `result: "{{ map(['hello'], part => part) }}"`, then `{{ result }}` | Accept | Accept |
| R1-F1.17 | Shell-template dependency inventory | `::shell echo {{ as_csv(map(['hello'], part => part)) }}` | Accept | Accept |

The other nested-argument classifiers were inspected too. `lazy_root_read` checks `current`, which the binding view protects against callback parameter binding. `unevaluated_call` checks actual function calls rather than variable names. Neither creates this shadowing error. The changed frontmatter dependency collector and shell-template collector use the whole expression's shared free-read inventory; rows R1-F1.16–17 exercise those clean sibling consumers. Literal nested-content collection does not classify variable roots and is outside this defect class.

**Recommended repair:** Preserve the enclosing lexical bindings when visiting nested calls and classify each argument with those bindings in scope, using shared read/binding machinery. Apply that to both the pending-property and unobserved-property checks, for body and frontmatter expressions. Keep explicit `fm.part` rejection and genuine execution-dependent targets unchanged. Add public in-process preflight regression assertions to the existing nested-composition suite; no new escaping layer, syntax, or approval policy is needed. `local.part` is a temporary workaround, not a reason to weaken the bare-name contract.

## Implementation decisions (informational)

Two independent agents read the implementation log and reconciled spec in parallel before comparison. The following choices are accepted within the current contract; none requires a human to stop the repair cycle.

| Decision or omitted work | Assessment |
| --- | --- |
| Keep ordinary value calls in their existing syntax-tree node and add a distinct callback-call node | Reasonable compatibility choice inside the crate; callback syntax cannot become a JSON value. |
| Extend directive callback parsing, frames, iterable types, budgets, and predicates | Required shared behavior is reused. Runtime execution was deferred only between implementation phases and is complete. |
| Use package test recipes with explicit features after a root-wrapper failure | Equivalent verification was recorded. Repairing root-wrapper forwarding remains outside this feature. This review's normal area recipes succeeded. |
| Protect host-specific globals at lifecycle execution | The complete binding view is available there. Empty-input callbacks are rejected too; the spec does not require earlier generic preparation to know those globals. |
| Record unrelated opt-in prompt/schema and review-runner failures | No callback acceptance behavior was deferred. The log identifies three review/fix rehearsal failures and standalone prompt-schema failures, and explicitly records cases not run after fail-fast. These were not counted as passing evidence. |
| Reuse `dm.expression.invalid_callback` for syntax and placement | Consistent with the existing registry and permitted diagnostic discretion. |
| Retain declaration locations in immutable type frames while supporting existing unspanned callers | Fits retained-scope and provenance requirements without forcing unrelated caller changes. |
| Preserve precise literal types in editor hover | Consistent with retaining known input information. |
| Prioritize completion candidates only when locals are present | Preserves nearest-local usefulness without changing unrelated completion ordering. |
| Use existing variable/operator token categories for bindings/arrows | Meets the highlighting requirement through existing protocol categories. |
| Describe bare/local binding completion and member-type hover precisely | The spec's local namespace completion is implemented; it does not require general object-member-name completion. |
| Limit downstream checks to affected consumers and small dependent suites | Appropriate for the changed interfaces; no new whole-workspace gate is needed. |
| Update an outdated opt-in schema guard for recursive types | Test maintenance reflecting existing schema behavior, rather than a product scope change. |
| Retain existing ignored tests | The log lists the pre-existing exclusions, including editor alias/block-scalar projection and provider cases. No new exclusion hides callback work. |
| Leave cross-platform execution to CI | This review does not make missing cross-OS evidence a readiness finding. |
| Select conventional option/builder names, nullable signatures, and diagnostic context shapes | The spec explicitly authorizes these choices through the existing shared machinery. |

Historical proposals about one arrow, identifier-only callbacks, array-only sources, a shadow advisory, unavailable prerequisites, and stack-only frames were superseded by the author's reconciliation. They are not outstanding decisions and need no reaffirmation. All five implementation phases are recorded as complete; the current finding is a newly verified defect, not a deferred phase.

## Requirements and verification

All requirements in this feature need **L1** verification: expression values, file discovery, hermetic lifecycle execution, CLI reports, and language-server protocol results. None requires terminal-emulator rendering or OS keyboard injection. DMLS highlighting is checked through semantic tokens, not terminal colors.

| Acceptance criteria | Strongest relevant evidence reviewed | Result |
| --- | --- | --- |
| 1–2: typed interpolation, conditions, lifecycle and Claudine dependency fixture | L1 document/CLI surface tests and normal Claudine preparation/event execution fixtures | Passing |
| 3: arrows, patterns, nesting, spans and invalid placement | L1 shared parser tests, CLI diagnostic tests and editor protocol cases | Passing |
| 4–5: shadowing, local/fm reads, restoration and missing-property warnings | L1 runtime, passive read inventory and surface tests; additional public preflight reproductions above | Gap R1-F1 |
| 6: first failure, original positions, typed causes and reports | L1 callback runtime tests, nested composition and CLI report-json assertions | Passing |
| 7–8: iterable adaptation, nulls, ordering and retained frames | L1 binder/runtime cases, typed tuple/YAML cases and independent frame-lifetime tests | Passing |
| 9: catalog, examples, verb exclusion and docs | L1 catalog examples and signature tests; source/document inspection | Implemented |
| 10: passive editor analysis, completion, hover, highlighting and locations | L1 DMLS protocol tests, escaped/multiline diagnostics and shared scope checks | Passing |
| 11: passive call checks and known types, file predicates and fact isolation | L1 callback call-check/type cases and existing registered narrowing tests | Implemented; no additional defect found |
| 12: ordered evaluation and shared visit/output accounting | L1 counted lookup/session tests and composition cases across directives and children | Passing |

Review execution:

- `cd darkmatter && BISCUIT_TEST_NARROW='test(callback) | test(collection_expression) | test(collection_host) | test(collection_execution) | test(collection_editor)' just test`: **153 passed**, 13,444 unrelated tests filtered out; 1.186 seconds execution.
- `cd claudine && BISCUIT_TEST_NARROW='test(collection_host_compat)' just test`: **9 passed**, 11,727 unrelated tests filtered out; 0.580 seconds execution.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. The surface-test module is declared in the library L1 target; the existing CLI, editor and Claudine modules are also declared and selected.
- A temporary diagnostic case copied the existing nested-composition fixture, loaded hermetic files, and called public preflight for the class sweep. It ran through `just test --success-output immediate`, took 0.080 seconds, and printed the observed outcomes above. The temporary case was removed; no implementation or permanent test file is changed by this review.

The implementation log also records full area tests and lint passing. Those historical results were read as implementation evidence, not presented as fresh review runs. Full tests/lint were not repeated for this review-only change. The opt-in failures remain disclosed rather than treated as a passing suite.

The input robustness matrix does not apply: this feature extends an expression text grammar and its consumers, not a file/configuration loader or field validator. The function catalog adds declarations through the existing loader. No new persisted format is introduced.

This is the first review; there are no previous findings or recurring classes to reconcile.
