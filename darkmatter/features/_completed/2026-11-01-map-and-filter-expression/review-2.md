---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-29T02:10:59-07:00
spec: 2026-11-01-map-and-filter-expression/spec.md
implemented: false
description: "A **feature** review of `2026-11-01-map-and-filter-expression/spec.md`"
feature: 2026-11-01-map-and-filter-expression/review-2.md
previous: 2026-11-01-map-and-filter-expression/review-1.md
---

# Review 2

The feature is production ready under the specification's contract. The previous finding is verified fixed, including every required instance and the repair ledger's additional consumers. No new findings or decisions requiring author input remain.

## Previous Finding Verification

Read `review-1.md`, the repair's `### Finding Ledger` in `log.md`, and the original implementation log before checking the code. The previous review contains one finding and no human rulings. Its informational decisions remain accepted and are not reopened.

Evidence below comes from source inspection and fresh L1 execution, not completion flags. Test names belong to the **darkmatter** package's [nested-composition suite](../../lib/tests/l1/nested_composition.rs). All 22 `callback_preflight_scope_*` cases call public `Markdown::compose_preflight` against temporary documents. Successful cases assert the exact command approval set; rejected cases assert a typed dependency error naming `frontmatter.part`. Every case checks that the execution marker was not created.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Preflight mistakes shadowing callback locals for pending frontmatter | R1-F1.1–26, individually checked below | verified fixed | The call visitor carries enclosing bindings into nested arguments; the shared read inventory removes lexical reads while preserving explicit document reads. All 22 public cases passed; four other consumers satisfy the inspected invariants below. | Nothing |
| R1-F1.1: Body map binding | Bare identifier with pending document property | verified fixed | `callback_preflight_scope_01_body_map` accepts and discovers the local command. | Nothing |
| R1-F1.2: Body filter binding | Bare identifier with pending document property | verified fixed | `callback_preflight_scope_02_body_filter` accepts with exact approvals. | Nothing |
| R1-F1.3: Map destructuring | `{part} -> as_markdown(part)` | verified fixed | `callback_preflight_scope_03_map_fields` accepts with exact approvals. | Nothing |
| R1-F1.4: Filter destructuring | Same fields and second arrow | verified fixed | `callback_preflight_scope_04_filter_fields` accepts with exact approvals. | Nothing |
| R1-F1.5: Outer capture | Nested callback reads outer bare local | verified fixed | `callback_preflight_scope_05_outer_capture` discovers the captured command. | Nothing |
| R1-F1.6: Frontmatter map | Whole-value result with shadowing callback | verified fixed | `callback_preflight_scope_06_frontmatter_map` accepts with exact approvals. | Nothing |
| R1-F1.7: Frontmatter filter | Whole-value result with shadowing callback | verified fixed | `callback_preflight_scope_07_frontmatter_filter` accepts with exact approvals. | Nothing |
| R1-F1.8: Unobserved property | Bare local shadows `can_execute` result | verified fixed | `callback_preflight_scope_08_unobserved` discovers only the local and marker commands. | Nothing |
| R1-F1.9: Explicit local map control | `as_markdown(local.part)` | verified fixed | `callback_preflight_scope_09_explicit_map` still accepts. | Nothing |
| R1-F1.10: Explicit local filter control | Same explicit-local argument | verified fixed | `callback_preflight_scope_10_explicit_filter` still accepts. | Nothing |
| R1-F1.11: Explicit outer capture control | Nested `local.part` | verified fixed | `callback_preflight_scope_11_explicit_capture` still discovers the captured command. | Nothing |
| R1-F1.12: Explicit local/unobserved control | Local shadows unobserved property | verified fixed | `callback_preflight_scope_12_explicit_unobserved` still accepts. | Nothing |
| R1-F1.13: Pending document control | `as_markdown(fm.part)` | verified fixed | `callback_preflight_scope_13_document_pending` still rejects with the typed dependency. | Nothing |
| R1-F1.14: Unobserved document control | Explicit `fm.part` from `can_execute` | verified fixed | `callback_preflight_scope_14_document_unobserved` still rejects. | Nothing |
| R1-F1.15: Nonshadowing control | Callback parameter named `item` | verified fixed | `callback_preflight_scope_15_nonshadowing` still accepts. | Nothing |
| R1-F1.16: Frontmatter inventory | Callback result consumed by body | verified fixed | `callback_preflight_scope_16_frontmatter_inventory` accepts without false dependencies. | Nothing |
| R1-F1.17: Shell-template inventory | Callback in shell template | verified fixed | `callback_preflight_scope_17_shell_inventory` approves exactly `echo hello` plus authored frontmatter commands. | Nothing |
| R1-F1.18: Sibling restoration | Document read after callback exit | verified fixed | `callback_preflight_scope_18_sibling_restoration` rejects the pending document read. | Nothing |
| R1-F1.19: Iterable before binding | Document read in iterable expression | verified fixed | `callback_preflight_scope_19_source_before_binding` rejects; parameter scope excludes the iterable. | Nothing |
| R1-F1.20: Local member | `as_markdown(part.content)` | verified fixed | `callback_preflight_scope_20_local_member` discovers the member's command. | Nothing |
| R1-F1.21: Indexed document | `as_markdown(fm['part'])` | verified fixed | `callback_preflight_scope_21_document_index` retains typed rejection. | Nothing |
| R1-F1.22: Filter/unobserved sibling | Filter local shadows `can_execute` result | verified fixed | `callback_preflight_scope_22_unobserved_filter` accepts with exact approvals. | Nothing |
| R1-F1.23: Literal nested-content consumer | Authored literals, including unreached branches | verified fixed | `authored_as_markdown_literals` matches only `StringLiteral`; traversal still includes every branch and callback body. `preflight_discovers_nested_effects_without_composing_them` passed. | Nothing |
| R1-F1.24: Literal ICMP consumer | Literal versus computed probe arguments | verified fixed | `authored_icmp_probes` uses `literal_value`: string/number literals are admitted; variables return `None`. No lexical/document classification occurs and traversal order is preserved. | Nothing |
| R1-F1.25: Lazy-root classifier | Reserved `current` dependency | verified fixed | `lazy_root_read` only classifies `current`; protected-name validation forbids callbacks binding that root. The repair preserves this invariant. | Nothing |
| R1-F1.26: Unevaluated-call classifier | Actual discovery-unsupported calls | verified fixed | `unevaluated_call` inspects function-call nodes, never variable names. Existing nested dependency-rejection tests passed. | Nothing |

The relevant **darkmatter** code is [the scope-aware call visitor and dependency classifier](../../lib/src/markdown/compose/preflight/collect.rs) and [the shared access-read inventory](../../lib/src/markdown/compose/expression/absence.rs). Callback bodies extend enclosing names; iterable arguments and sibling expressions retain their original scope. Both frontmatter and body checks consume the repaired classifier. Pending shell values and unobserved values use the same filtered roots; explicit `fm` reads remain dependencies.

The repair also optimized a differential test in [the type-scope suite](../../lib/tests/l1/expression_type_scope.rs). It moves unchanged sample filtering and branch projections outside the value-pair loop. Inspection confirms that every admitted pair and branch assertion remains. All 12 selected differential cases passed; the slowest took 1.839 seconds.

## Unblocked Findings

None.

## Blocked Findings

None. No prerequisite or author decision blocks this review.

## Specification and verification coverage

Reviewed the reconciled specification, original implementation record, repair diff, shared parser and binding/read machinery, callback evaluator, call-site type projection, catalog, topic documentation, and relevant public composition, CLI, editor, and Claudine fixtures. Expression arrays retain the intended differences from directive rendering; passive analysis remains distinct from discovery and execution.

Every requirement needs **L1** verification: values and types are checked in process, CLI behavior through hermetic subprocesses, and editor behavior through language-server protocol requests. This spec adds no terminal rendering or OS keyboard behavior requiring L2 or L3. Semantic-token categories and ranges are protocol results.

| Acceptance criterion | Verification boundary and evidence | Assessment |
| --- | --- | --- |
| 1: interpolation, conditions, lifecycle, typed values and provenance | L1 `collection_expression_surfaces`, CLI surface tests, and Claudine lifecycle fixtures; produced expression text remains data. | Satisfied |
| 2: hermetic dependency lifecycle fixture | L1 `collection_host_compat::expression_lifecycle`: normal preparation/execution accepts implemented/empty dependencies and rejects unimplemented, absent, or null implementation status. | Satisfied |
| 3: arrows, patterns, nesting and located rejection | L1 shared parser/call-check tests, CLI invalid-callback tests, and editor escaped/multiline diagnostic projection. | Satisfied |
| 4–5: protected names, shadowing, lookup, restoration and undeclared reads | L1 runtime/read-inventory tests and 22 public preflight cases; host protection also checked on empty inputs. | Satisfied; R1-F1 repaired |
| 6: first-error stop, original positions, nested paths and causes | L1 counted runtime fake, typed callback errors, nested composition, and CLI report-json assertions. | Satisfied |
| 7: arrays, tuples, ordered objects, nulls and binder failures | L1 callback runtime, schema-tuple/YAML, catalog example and call-check tests; object filters retain entry arrays. | Satisfied |
| 8: retained frames and nested capture | L1 lexical frame/session tests, type-frame spans surviving AST drop, and runtime capture/restoration cases. | Satisfied |
| 9: catalog, examples, lifecycle exclusion and docs | Catalog registration/signature/example tests and inspection of current expression topic documentation and skill guidance. | Satisfied |
| 10: passive editor scopes, hover, completion, tokens and diagnostics | L1 `collection_editor`, cursor/recovery tests, and provider no-effects tests, including exact token/range assertions. | Satisfied |
| 11: passive checks, known types and file predicates | L1 call-check cases cover dead bodies, patterns, tuple/object projection, both file predicates, and isolation of shadowed facts. Shared predicate/fact implementation inspected. | Satisfied |
| 12: counted evaluation and shared budgets | L1 runtime/session and collection-execution cases cover one source evaluation, sequential visits, first failure, mixed directive/expression limits, child exhaustion and fallback fatality. Expansion accounting remains shared; expression arrays do not charge rendered bytes. | Satisfied |

Test placement was checked alongside execution. The **darkmatter** library sets `autotests = false`, declares the L1 target, and that target includes `nested_composition`, `expression_type_scope`, and `collection_expression_surfaces`. Regression names carry no excluded tier marker. Fresh execution also selected CLI/editor and Claudine targets; no added feature gate prevents these cases from running.

The input robustness matrix does not apply: this feature extends an expression text grammar and its consumers, not a file/configuration loader. Catalog declarations use the existing loader. No persisted format is added.

## Verification performed

- In `darkmatter`, `BISCUIT_TEST_NARROW='test(callback) | test(collection_expression) | test(collection_host) | test(collection_execution) | test(collection_editor) | test(nested_composition) | test(expression_type_scope::every_guard_side)' just test`: **207 passed**, 13,412 excluded by selection; 2.150 seconds test execution.
- In `claudine`, `BISCUIT_TEST_NARROW='test(collection_host_compat)' just test`: **9 passed**, 11,727 excluded by selection; 0.586 seconds test execution.
- `just check-tier-coverage darkmatter`: **passed**, zero stranded tests.

Both builds emitted macOS linker warnings about a large debug unwind section and completed successfully. No selected test exceeded five seconds.

The repair log records full Darkmatter L1 and lint passes after its final edit. Those are historical implementation evidence; this review reran the affected tests and does not claim fresh full-suite or lint results. Cross-OS evidence remains CI's responsibility and does not block readiness. Previously disclosed opt-in prompt/review-runner failures are not counted as passing evidence or reopened as feature requirements.

## Recurrence

No recurring finding. R1-F1 is fully repaired, including additional sibling cases and consumers in its repair ledger. No partial repair, regression, newly unblocked work, or newly discovered sibling remains.
