---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: '2026-10-25T02:10:23-07:00'
spec: 2026-10-31-type-aware-evaluation/spec.md
implemented: false
description: 'A **feature** review of `2026-10-31-type-aware-evaluation/spec.md`'
feature: 2026-10-31-type-aware-evaluation/review-2.md
previous: 2026-10-31-type-aware-evaluation/review-1.md
---

# Review 2

The feature is **production ready** within the specification's scope. All five previous findings are verified fixed. There are no new findings, partial repairs, blocked findings, or human decisions required to close this review.

This review read the specification, its declarations design annex and corrections, the first review, the original implementation log, and the repair log's Finding Ledger. The conclusions below come from source inspection and fresh test runs, rather than the ledger's completion flags.

## Previous Finding Verification

Required instances include the clean controls and the additional children recorded during repair. IDs in this table refer to the first review and its repair ledger in [log.md](log.md).

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Deliver guarded editor analysis for Claudine lifecycle expressions | **.1:** `start`, `success`, `blocked`, `failure`, each with `stdout` and `say`. **.2:** shared events using `say`, including the renamed-event controls. **.3:** `loop.gate`. **.4:** false branch. **.5:** nested write. **.6:** all five Markdown events. **.7:** error-global availability. **.8:** all nine other host-only verbs' operands. | **verified fixed** | All cases in [DMLS lifecycle host tests](../../../darkmatter/dmls/tests/l1/lifecycle_host_profile.rs) passed through stdio editor sessions. Claudine's [host catalog tests](../../lib/tests/l1/host_catalog.rs) passed artifact freshness, rebuilt-registry equality, and composition/editor-walk parity. [GuardedStackCalls](../../../darkmatter/dmls/src/diagnostics/frontmatter.rs) now uses the selected host's parser and registry, then Darkmatter's one scoped walk. | None. Under Claudine, `composed` and `failed` are rejected renamed spellings, rather than valid stacks that should narrow. The parser and editor tests verify that distinction. |
| R1-F2 — Stop execution when a runtime warning becomes blocking | **.1:** body → shell. **.2:** frontmatter → shell. **.3:** block condition → shell. **.4:** transclusion condition → child. **.5:** condition → next directive. **.6:** frontmatter shell ternary → body shell. **.7:** child's interpolation → child's shell. **.8:** passive call warning. **.9:** handled absence. **.10:** removed branch. **.11:** lifecycle compose route. **.12:** scalar-context and both assignment recovery denials. **.13:** later expressions/conditions, nested shells, and absorbed child stops. | **verified fixed** | All 26 [runtime blocking tests](../../../darkmatter/lib/tests/l1/runtime_blocking.rs) passed, with effect controls. The CLI report test checks both ordinary and lifecycle routes, including absence of later effect markers. [Pipeline checks](../../../darkmatter/lib/src/markdown/compose/pipeline/mod.rs), expression guards, condition watches, and nested-stop propagation enforce the same blocking policy. Policy tests also passed the earlier-effect/no-rollback control. | None. Preserve the recorded boundaries: final frontmatter declarations decide undeclared-read candidates; already-started parallel work is not rolled back; one expression is judged as a unit. These are settled pipeline choices, not newly missing instances. |
| R1-F3 — Preserve shared diagnostics on lifecycle call rejection | **.1:** action text in all five Markdown events. **.2:** conditions in each event. **.3:** shell operand. **.4:** side-effect argument. **.5:** runtime-set value. **.6:** control reason. **.7:** body/frontmatter controls. **.8:** block/transclusion condition controls. **.9:** Claudine error conversion. | **verified fixed** | The [shared-field editor/composition comparisons](../../../darkmatter/dmls/tests/l1/expression_call_diagnostics.rs) passed for code, severity, message, source, expected type, and actual type. CLI lifecycle rejection passed with no effect. Both Claudine preservation tests passed. [Lifecycle rejection](../../../darkmatter/lib/src/lifecycle/validate.rs) stores the located diagnostic; [report conversion](../../../darkmatter/lib/src/markdown/compose/report_diagnostics.rs) consumes it before generic expression-error classification; Claudine retains it in its error conversion. | None. |
| R1-F4 — Preserve the ordered passive diagnostic history | **.1:** separate body expressions. **.2:** frontmatter leaves. **.3:** calls within one expression. **.4:** block condition before rejection. **.5:** lifecycle action list. **.6:** interpolation before block condition. **.7:** interpolation before transclusion condition. **.8:** later runtime failure. **.9:** parent's warnings before a child rejected during preflight. | **verified fixed** | All [passive history tests](../../../darkmatter/lib/tests/l1/report_diagnostics.rs) passed, including earliest-rejection selection, omitted post-rejection findings, and parent/child history through both composition and preflight. CLI tests passed ordered history and child rejection for ordinary and lifecycle routes. [The passive collector](../../../darkmatter/lib/src/markdown/compose/call_precheck.rs) now keeps and sorts the prefix; lifecycle errors carry earlier warnings; preflight preserves ancestor warnings. | None. Parent passive analysis runs before child composition, so the parent's collected warnings precede the child's records, including parent warnings authored after the directive. The repair records and tests this existing ordering decision. |
| R1-F5 — Do not restrict nullable-parameter suppression by return type | **.1:** runtime body calls. **.2:** runtime frontmatter calls. **.3:** editor body diagnostics. **.4:** nullable-returning host function. **.5:** direct predicates. **.6:** nested-call boundary. **.7:** plain-unknown, optional-non-null, mixed-overload, and all-null-overload controls. | **verified fixed** | [Registration](../../../darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs) now examines parameter nullability and every applicable overload, without a return-type condition. [AbsenceScope](../../../darkmatter/lib/src/markdown/compose/expression/absence.rs) owns the shared call-site distinction. Runtime suppression, frontmatter, host-function, editor, and CLI denial tests all passed. Direct `trim(x)`/`lower(x)` suppress the missing-property warning; `is_empty(trim(x))` retains it. | None. The repair's documented nested-call rule is retained; no new function-name suppression list was introduced. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No recurring finding remains. This review found no unchanged instance, repair regression, or missed sibling of the five earlier defect classes. The extra repair-ledger children are included in the verification table above, rather than reported as new findings.

## Observations

None.

## Requirement and Verification Levels

This feature promises expression analysis, composition outcomes, diagnostic data, and command behavior. Level 1 in-process APIs, hermetic subprocesses, and actual stdio editor sessions exercise those boundaries. It introduces no terminal-rendering, keyboard-encoding, mouse, paste, or focus behavior requiring Level 2 or Level 3 verification.

| User-facing requirement | Strongest relevant verification inspected and run | Assessment |
| --- | --- | --- |
| Complete declared types, nested and recursive paths, root unions, globals, optionality, and passive analysis — criteria 1–8, 31–32 | Level 1 public composition/type APIs in `variable_type_projection`; ownership guards; Claudine lifecycle checking | Appropriate. Cases cover effective schema layers, raw schemas, shadowed bare versus `fm` access, unavailable globals, indices, recursive paths, and unchanged stored values. |
| Guarded calls, predicate facts, impossible branches, and write/event boundaries — criteria 9–12, 33–34 | Level 1 library checks, stdio editor diagnostics, shell-template typing, and rebuilt Claudine host-profile parity | Appropriate. The repaired host-event and host-operand cases now exercise the same library walk as runtime checking. |
| Pure file classification, malformed-reference handling, aliases, and guarded file arguments — criteria 13–18, 35 | Level 1 file-predicate APIs, effect instrumentation, composition, CLI wiring, and stdio editor diagnostics | Appropriate. The first review's accepted valid-remote behavior decision is unchanged. |
| Shared diagnostics, nullable arguments, privacy, and host registration/artifact delivery — criteria 19–22, 36–37 | Level 1 exact diagnostic comparisons, public host registration/analysis APIs, handler/network counters, and editor sessions | Appropriate. Rejection tests now cover lifecycle surfaces and Claudine conversion, alongside warning tests. |
| Policy precedence, recoveries, removed options, disabled checks, and stopping — criteria 23–28, 38 | Level 1 library effect controls and CLI subprocess tests | Appropriate. Tests verify outcomes and marker absence/presence, including failures before effects and retention of earlier effects. |
| One report object, retained diagnostics, authored locations, privacy, and failure publication — criteria 29, 39 | Level 1 public report APIs and CLI subprocess tests | Appropriate. Repaired history tests include preflight, lifecycle rejection, and transcluded-child failures. |
| Before/after performance evidence — criterion 30 | Recorded representative-document composition and editor measurements in `implementation-log.md`, under “Performance after” | Present. The log explains the editor-analysis increase separately from schema work and records conservative depth-bound behavior. No stronger benchmark contract is imposed here. |

## Input Robustness Review

The function-facts JSON reader and the repair's lifecycle-profile JSON reader share strict JSON loading and required-value validators. The default invalid-input outcome is a reason naming the companion artifact and incomplete editor analysis; no host handler executes. Missing required fields are errors, explicit null is accepted only in documented nullable slots, wrong collection elements are rejected rather than filtered, and duplicate keys and trailing content are rejected by the shared loader.

The exceptions are explicit data: lifecycle `core`, `position`, `role`, and `outcome_pair` use null for “none,” with registry validation enforcing which event shapes allow that; empty action/effect/global lists describe empty selections where the registry permits them. Absence does not silently become those empty selections.

The public host-function matrix passed its control and malformed field/document cases. Lifecycle-profile tests passed round trips plus absent, stale, unsupported-version, missing-event, unknown-field, duplicate-key, wrong-boolean, and missing-field cases. Stdio editor tests passed valid, absent, stale, and incomplete lifecycle artifacts. The source sweep of both readers found no load-bearing `serde(default)`, silently filtered element, swallowed parse error, or absent/null conflation. The raw-schema projection consumes schema values supplied by the existing schema loader; report JSON is output rather than another configuration reader. No additional robustness defect was established.

## Verification and Limits

Fresh focused runs through the canonical `just test` recipe passed:

- **Darkmatter area:** 148 tests covering previous findings, report/CLI wiring, and editor scopes; 67 covering projection, narrowing, file predicates, ownership, and lifecycle profiles; 40 covering policy, host-function delivery, and blocking.
- **Claudine area:** 60 tests covering host catalogs, artifact freshness/parity, lifecycle call checking, and error conversion.

These are **315 passing test executions**, with overlap between focused selections. No executed test exceeded five seconds. The consolidated test targets declare the relevant modules, and their names select Level 1. Tier coverage audits for both areas reported zero stranded tests.

No implementation source was edited for this review. Full suites, lint, and cross-OS runs were not repeated; the repair log records those broader checks, while this review independently reran the affected behavior. Cross-OS proof and a pending human review are not readiness blockers under this review's contract. The feature directory remains active for the author's review-cycle closure.
