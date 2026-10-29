---
implementation_1: "2026-10-29T01:56:41-07:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-10-29T01:56:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-map-and-filter/darkmatter/features/2026-11-01-map-and-filter-expression/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- Inventory reconciled: one medium finding, seventeen required cases, no blockers or recurrence; informational decisions require no repairs.
- Defect class: nested argument dependency classification loses surrounding lexical bindings and mistakes local reads for document reads.

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Preserve lexical scope in nested-content dependency classification | fixed | All 17 required public preflight cases and 5 additional scope controls passed; focused regression suite recorded below. |
| R1-F1.1 | Body nested-content check, map: Identifier `part => as_markdown(part)`; pending document `part`; expected Accept | fixed | `nested_composition::callback_preflight_scope_01_body_map` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.2 | Body nested-content check, filter: Identifier `part => as_markdown(part)`; same pending property; expected Accept | fixed | `nested_composition::callback_preflight_scope_02_body_filter` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.3 | Body nested-content check, map: `{part} -> as_markdown(part)` over `[{part: 'hello'}]`; expected Accept | fixed | `nested_composition::callback_preflight_scope_03_map_fields` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.4 | Body nested-content check, filter: Same destructuring and second arrow; expected Accept | fixed | `nested_composition::callback_preflight_scope_04_filter_fields` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.5 | Nested callback capture: Outer `part`, inner `n`, inner `as_markdown(part)`; expected Accept | fixed | `nested_composition::callback_preflight_scope_05_outer_capture` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.6 | Frontmatter nested-content check, map: Whole-value `result` containing the same map call; expected Accept | fixed | `nested_composition::callback_preflight_scope_06_frontmatter_map` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.7 | Frontmatter nested-content check, filter: Whole-value `result` containing the same filter call; expected Accept | fixed | `nested_composition::callback_preflight_scope_07_frontmatter_filter` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.8 | Unobserved-property classification: Replace document `part` with `"{{ can_execute('git') }}"`; keep local bare argument; expected Accept | fixed | `nested_composition::callback_preflight_scope_08_unobserved` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.9 | Explicit local control, map: `as_markdown(local.part)` with pending document property; expected Accept | clean | `nested_composition::callback_preflight_scope_09_explicit_map` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.10 | Explicit local control, filter: Same explicit local argument; expected Accept | clean | `nested_composition::callback_preflight_scope_10_explicit_filter` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.11 | Explicit outer-local capture control: Inner callback reads `as_markdown(local.part)`; expected Accept | clean | `nested_composition::callback_preflight_scope_11_explicit_capture` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.12 | Explicit local with unobserved document property: `as_markdown(local.part)` with `can_execute` property; expected Accept | clean | `nested_composition::callback_preflight_scope_12_explicit_unobserved` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.13 | Explicit document control, pending: `as_markdown(fm.part)`; expected Reject | clean | `nested_composition::callback_preflight_scope_13_document_pending` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.14 | Explicit document control, unobserved: `as_markdown(fm.part)` with `can_execute` property; expected Reject | clean | `nested_composition::callback_preflight_scope_14_document_unobserved` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.15 | Nonshadowing callback control: `item => as_markdown(item)`; expected Accept | clean | `nested_composition::callback_preflight_scope_15_nonshadowing` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.16 | Ordinary frontmatter dependency inventory: `result: "{{ map(['hello'], part => part) }}"`, then `{{ result }}`; expected Accept | clean | `nested_composition::callback_preflight_scope_16_frontmatter_inventory` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.17 | Shell-template dependency inventory: `::shell echo {{ as_csv(map(['hello'], part => part)) }}`; expected Accept | clean | `nested_composition::callback_preflight_scope_17_shell_inventory` passed; exact approvals or typed rejection, and no execution. |
| R1-F1.18 | Sibling scope restoration: local scope ends before a later `as_markdown(part)`; reject pending document read | clean | `callback_preflight_scope_18_sibling_restoration` passed with typed frontmatter dependency rejection. |
| R1-F1.19 | Iterable source before binding: `as_markdown(part)` in the iterable still reads the document; reject | clean | `callback_preflight_scope_19_source_before_binding` passed with typed rejection. |
| R1-F1.20 | Bare local member read under a same-named pending document property; discover nested command | fixed | `callback_preflight_scope_20_local_member` failed before repair and passed afterward with the exact approval set. |
| R1-F1.21 | Explicit indexed document read `fm['part']` is never hidden by callback bindings; reject | clean | `callback_preflight_scope_21_document_index` passed with typed frontmatter dependency rejection. |
| R1-F1.22 | Filter local under a same-named unobserved document property; discover nested command | fixed | `callback_preflight_scope_22_unobserved_filter` failed alone before repair and passed afterward with the exact approval set. |
| R1-F1.23 | Literal nested-content consumer must retain authored literals independently of scope | clean | `authored_as_markdown_literals` accepts only `StringLiteral` nodes and ignores scope names; `preflight_discovers_nested_effects_without_composing_them` verifies all authored literal branches. |
| R1-F1.24 | Literal ICMP collection must not reinterpret callback locals as document reads | clean | `authored_icmp_probes` accepts only argument lists converted by `literal_value`; variables yield `None`, so no lexical/document root classification occurs. Visitor order and literal syntax are unchanged. |
| R1-F1.25 | Lazy-root classifier must keep unavailable `current` reads distinct | clean | `lazy_root_read` inspects only reserved `current`, which the binding view forbids as a callback parameter; lexical names cannot shadow it. |
| R1-F1.26 | Unevaluated-call classifier must reject actual probes independently of lexical names | clean | `unevaluated_call` inspects function-call nodes, never variable roots; existing probe-dependent nested-content tests preserve rejection. |

- starting the work on 'R1-F1: scope-aware nested-content dependencies' at 01:58:58
        - Sweep: both body and frontmatter use `dynamic_nested_dependency`; pending and unobserved property checks share its isolated argument reads.
        - Clean sibling invariants: literal-content/ICMP visitors inspect syntax only; unevaluated calls inspect function nodes; `current` cannot be a callback parameter. Whole-expression frontmatter and shell inventories already exclude lexical reads through the shared inventory.
        - Add per-case public preflight regressions before repair; assert complete command sets and absence of execution, including explicit document rejection and lexical scope restoration.
        - Reproduction: `cd darkmatter && BISCUIT_TEST_NARROW='test(callback_preflight_scope)' just test` ran 20/22 cases: 9 reproduced failures, 11 controls passed, 2 not run after fail-fast. Separate `test(callback_preflight_scope_22_unobserved_filter)` reproduced the additional filter/unobserved case.
        - Repair: nested-call traversal retains each callback pattern only for its body; shared lexical-read filtering classifies both pending and unobserved dependencies. Iterable arguments and sibling calls retain their enclosing scope.
        - Added current-behavior documentation for local discovery versus explicit `fm` dependencies. No parser/configuration loader changes; the input robustness matrix does not apply.
        - Focused verification: `cd darkmatter && BISCUIT_TEST_NARROW='test(callback_preflight_scope)' just test` passed all 22 cases (0.147 seconds execution).
        - Broader affected checks: `cd darkmatter && BISCUIT_TEST_NARROW='test(nested_composition) | test(expression::absence) | test(callback)' just test` passed all 158 tests (0.830 seconds execution), including existing nested discovery, strict dependency failures, shared read inventory, CLI callbacks, and DMLS callbacks.
        - Target/tier: `tests/l1/main.rs` declares `mod nested_composition`; new names carry no excluded tier marker. No extra features are needed beyond the area recipe.
        - Reviewed changed symbol comments and documentation; the call visitor contract now describes retained bindings. No OS-specific code, path handling change, performance measurement, or author decision was needed.
- work completed for 'R1-F1: scope-aware nested-content dependencies' at 02:01:32

- Final reconciliation: inspected the combined implementation and every ledger row; shared lexical filtering retains explicit document reads and source/sibling scope boundaries. No review authority was disputed or author decision required.
- Initial full gates: `cd darkmatter && just test` passed 13,609 tests with 10 existing skips; `just lint` passed all four packages and the Zed WASM check; `just check-tier-coverage darkmatter` reported zero stranded tests.
        - Eleven existing tests exceeded five seconds during the full run. Each was rerun alone: ten completed in 1.158–2.344 seconds and are load-sensitive; the scalar-union differential test took 12.479 seconds.
        - Gate maintenance: hoisted unchanged sample filtering and branch type projection out of the differential helper's pair loop, retaining every sample pair and assertion. This addresses the observed L1 timing violation without changing production behavior.
        - Focused timing validation: `cd darkmatter && BISCUIT_TEST_NARROW='test(expression_type_scope::every_guard_side)' just test` passed all 12 cases, maximum 1.788 seconds.
        - The full L1 rerun after optimization passed 13,609 tests in 77.158 seconds with no slow tests and 10 pre-existing skips. Clippy then caught a redundant borrow in the optimized assertion; removed it and reran `just lint` successfully. Final L1 is being repeated after that last source edit.

### Completion Report

- **Outcome: complete** — implementation complete, ready for review. Finished at 2026-10-29T02:10:53-07:00; elapsed 14 minutes 12 seconds from the requested start timestamp.
- Parent findings: **1 total = 1 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed**. All 26 child rows reconciled: 10 fixed and 16 clean. Remaining finding/child IDs: **none**.
- Verified R1-F1 repair: nested-call discovery preserves callback bindings for pending and unobserved dependencies, including map/filter, both arrows, destructuring, nested captures, and body/frontmatter surfaces. Exact command sets prove required discoveries and exclude extra commands; typed errors preserve genuine document dependencies. Preflight executes nothing.
- Final verification on macOS after the final source edit:
        - `cd darkmatter && just test`: **13,609 passed**, 10 pre-existing skips, 88.434 seconds; no test exceeded five seconds. Covers R1-F1 and every required instance/consumer.
        - `cd darkmatter && just lint`: **passed** for darkmatter, darkmatter-cli, dmls, zed-dmls-cli, and Zed WASM compilation. The intermediate redundant-borrow lint is resolved.
        - `just check-tier-coverage darkmatter`: **passed**, zero stranded tests; both changed test modules are declared by the existing L1 target.
        - `git diff --check`: **passed**.
- Required behavior uses L1 only; no terminal/browser resources or notable cross-OS risks were introduced. No performance measurement is deferred. No new dependency, architecture change, disputed authority, or author decision was needed. No formatting, commits, branch changes, or lifecycle moves were performed.
- Changed files:
        - `darkmatter/lib/src/markdown/compose/expression/absence.rs`
        - `darkmatter/lib/src/markdown/compose/expression/mod.rs`
        - `darkmatter/lib/src/markdown/compose/preflight/collect.rs`
        - `darkmatter/lib/tests/l1/nested_composition.rs`
        - `darkmatter/lib/tests/l1/expression_type_scope.rs`
        - `darkmatter/docs/inline/preflight-checks.md`
        - `darkmatter/features/2026-11-01-map-and-filter-expression/log.md`
        - `darkmatter/features/2026-11-01-map-and-filter-expression/review-1.md`
- The pre-existing specification edit was preserved; this repair did not change the specification. Review readiness, findings, human-review items, and body were preserved; only requested completion metadata changed.
