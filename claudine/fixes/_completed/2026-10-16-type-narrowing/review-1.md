---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-20T13:03:18-07:00
spec: 2026-10-16-type-narrowing/spec.md
fix: 2026-10-16-type-narrowing/review-1.md
implemented: true
description: "A **fix** review of `2026-10-16-type-narrowing/spec.md`"
ready: true
findings: []
observations:
    - Future-case labels imply decisions the spec has not made
    - Earlier exact-approval handling of an empty argument remains an author item
human_in_the_loop: null
human-in-the-loop: false
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
recurrence: false
implemented_by: claude/opus
---

# Review 1: type narrowing

The fix is ready for production within the specification's current scope. No high, medium, or low findings were identified. This is the first implementation review, so there are no previous findings to reconcile.

The review covered the implementation changes beginning after `fd64a311b`, the final documentation changes in the working tree, the specification, plan, and implementation log. Production code was not changed during review.

## Decisions and deferred work

Two independent reviewers first inspected the implementation log and specification/plan for decisions, unresolved rules, and deferred work. Neither identified a decision requiring a human review before proceeding.

The implementation follows the existing evaluator where one specification-table entry conflicts with its behavior: `x != ''` excludes null because null compares as empty text against a non-null value. This preserves the specification's explicit requirement to keep equality semantics unchanged. The log records this departure. Missing-value warnings also now respect each branch separately: `x ? 'some' : x` can warn about the final read instead of treating both branches as protected by `x`.

Guarded values that will exist only later are checked when the selected command is reached. Checking them unconditionally at task start would reject a branch that should be skipped. Ordinary late values retain their task-start checks. Already-present values remain eligible for exact approval, which authorizes fixed command bytes; template execution checks the original declaration and applicable guard refinement against the value it dispatches. These choices preserve the stated approval and validation boundaries.

The editor uses the shared analysis for body blocks and branches within expressions. It does not apply lifecycle `when` guards to `then`/`else` actions, or task guards to task bodies. The specification qualifies editor agreement as applying at supported sites, and these existing editor capability limits are documented and explicitly represented in the conformance inventory. Nested-union projection, fuller host types, and function-argument integration remain future work. Function-result narrowing is outside this fix's scope.

The unresolved `file_exists` question concerns a future argument binder that could convert a number into a path string. A successful check of that converted string would not prove that the original value was a string. Today's handler requires an original string, and the specification explicitly permits this fix to ship before that future policy is resolved. No human decision is required for the implementation reviewed here.

## Findings

None.

## Verification

The requirements concern static analysis, passive editor behavior, and process arguments. Level 1 is appropriate: no requirement depends on a terminal emulator's rendering or keyboard encoding. CLI tests launch recording executables or fake providers in isolated fixtures; the motivating prompt uses a frozen copy and never creates a real commit.

| Specification requirement | Strongest verification reviewed and rerun | Result |
| --- | --- | --- |
| Guarded commit message passes preflight, false branch launches nothing, selected branch receives one argument; unguarded missing value fails (acceptance 1–2) | Level 1 CLI: `implement_plan_commit_message` and `guarded_shell_values`, including the frozen prompt | Passed |
| A selected command rejects a value cleared by `set`, preserving earlier action effects (acceptance 3) | Level 1 CLI and library: guarded-shell execution tests | Passed |
| Applicable body, lifecycle, inline-guard, whole-value `set`, task-shell, and task-`with` sites share guard rules (acceptance 4) | Level 1 public composition/preflight conformance tests in Darkmatter and Claudine | Passed |
| Structural literals/unions and narrowed shell approval types (acceptance 5–6) | Level 1 public schema/scope/template APIs: `schemas_core_type_values`, `expression_type_scope`, `shell_template_guards` | Passed |
| Editor/library agreement at supported sites (acceptance 7) | Level 1 language-server session tests: `expression_scopes`, including hover and diagnostics | Passed |
| Competing classifiers and unreviewed unguarded construction fail the ownership checks (acceptance 8) | Level 1 ownership guards and their negative snippet tests in all three affected libraries | Passed |
| Presence is specific to the guarded expression; actual null is not excused by a declaration (acceptance 9) | Level 1 CLI, public template API, and site conformance tests | Passed |
| False guarded branches bypass conditional late-value checks; ordinary values retain the barrier (acceptance 10) | Level 1 CLI group/task tests and library barrier tests | Passed |
| Equality edge cases and predicate aliases agree with the evaluator (acceptance 11) | Level 1 public scope tests, including differential comparisons against the evaluator | Passed |
| Alias, ancestor, descendant, and branch-join writes invalidate facts; runtime checks enforce the approved refinement (acceptance 12) | Level 1 CLI mutation tests plus public scope/template tests | Passed |
| Passive `file_exists` analysis and unavailable-binding behavior (acceptance 13) | Level 1 scope, composition, and language-server tests | Passed |
| Existing approval modes, blacklists, argument boundaries, missing-value refusal, retries, and resumes (acceptance 14) | Level 1 CLI `lifecycle_shell_templates`, `sequence_shell_templates`, `shell_missing_values`; public template execution/policy regressions | Passed |

The focused reruns used the package-area `just test` recipes:

- Darkmatter: `expression_type_scope::` — 32 passed.
- Darkmatter: conformance, ownership, editor scopes, core types, warning handling, and guarded templates — 119 passed.
- Claudine: conformance, ownership, guarded shells, frozen commit prompt, and task guards — 132 passed.
- Darkmatter: guarded body commands, template validation/execution/policy, and directive targets — 54 passed.
- Claudine: lifecycle/sequence shell approvals and missing values — 334 passed.
- Darkmatter: shipped-schema corpus checks after writing the review metadata — 9 passed.

These are focused test executions, with overlap between filters; they are not a claim that the entire package suites were rerun. All passed, and no reported test exceeded the five-second L1 budget. The macOS linker emitted an existing compact-unwind size warning while building Claudine; linking and tests succeeded.

New integration modules are declared by their consolidated L1 binaries, and fixture features are enabled by the area's test configuration. `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine` both passed with zero stranded tests. The ownership checks also exercised their source inventories. No additional terminal/browser tier or cross-OS evidence is required by this specification; CI supplies the environment coverage.

The input robustness matrix does not apply: this fix changes expression analysis and template handling, not configuration loaders, file-format deserializers, or their field validators.

## Observations

**Future-case labels imply decisions the spec has not made.** The conformance inventory labels function-result narrowing as “planned,” although the specification explicitly excludes it. It also describes future conversion-aware `file_exists` as proving a string regardless of the original argument, although that rule remains unresolved and the spec recommends weaker original-value evidence. These labels do not affect execution: their cells are explicitly not run. Treat function-result narrowing as out of scope and the conversion rule as undecided when maintaining the inventory. See [the shared case descriptions](../../../darkmatter/lib/tests/common/type_scope_cases.rs) and [the conformance explanations](../../../darkmatter/lib/tests/l1/scope_site_conformance.rs).

**Earlier exact-approval handling of an empty argument remains an author item.** The [implementation log](implementation-log.md#findings-recorded-not-fixed) reports that resolving `git commit -m "{{commit_message}}"` with an empty message drops the empty argument from its exact-approval key. The log attributes this to the earlier shell-template implementation; the motivating guard is false for an empty message. This review did not independently reproduce that earlier defect and does not make it a repair requirement for this fix. Its owner can investigate it separately.
