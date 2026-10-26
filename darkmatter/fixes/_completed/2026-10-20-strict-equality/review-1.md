---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-20T16:56:36-07:00"
spec: 2026-10-20-strict-equality/spec.md
fix: 2026-10-20-strict-equality/review-1.md
implemented: true
implemented_by: claude/opus
description: "A **fix** review of `2026-10-20-strict-equality/spec.md`"
ready: true
findings: []
observations:
    - Opt-in prompt test expects an obsolete null default
human_in_the_loop: null
human-in-the-loop: false
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
recurrence: false
---

# Strict equality review — iteration 1

The fix is ready for production within the specification's scope. No blocking
findings were identified. This is the first review; there are no previous
findings to reconcile.

## Decisions and deferred scope

Two independent agents reviewed the specification/plan and implementation log
before the implementation review began. Neither found an unresolved design
decision or deferred implementation requirement needing human input.

The implementation uses one typed comparator for runtime evaluation and static
guard analysis. It preserves existing number parsing, including rounding and
integer saturation, and retains a broader number type when a narrower schema
literal could reject a numerically equal value. These choices follow the spec.
For example, `5 == 5.0` is true, while `5 == '5'` is false.

More precise guard types change shell approval shapes for some comparisons
against literals. Previously stored approvals for those shapes need renewed
approval; the implementation deliberately adds no migration in this repository
with no existing users. Frozen prompt copies were retained where their
comparisons did not change behavior. The package-list condition in
`prompts/code-comment-quality.md` now checks `ctx.current_packages`; its former
condition referenced the nonexistent `ctx.package`. None of these choices
requires a new product decision.

## Findings

None.

## Verification

Reviewed the implementation commits after `9919a6928` through `455eb05e4`,
together with the current uncommitted documentation and skill updates. Used
the darkmatter and rust-testing skills. Checked the shared comparator, runtime
operator dispatch, literal conversion, finite-value and container guard
analysis, affected consumers, migration inventory, documentation, and test
registration.

| Requirement | Strongest applicable verification reviewed |
| --- | --- |
| Strict scalar, null, exact numeric, and recursive container equality | Level 1 public `parse`/`evaluate` tests in `expression_strict_equality`, with independent expected results, typed large integers, both operand orders, reflexivity, and complementary operators |
| Guard facts admit every live runtime value | Level 1 expected-type assertions and differential tests in `expression_type_scope`, including finite nullable operands, unknown values, numeric storage, container families, and existing write invalidation |
| Presence checks reach shell preflight, targets, warnings, and editor results | Level 1 shared consumer cases, `shell_template_body_guards`, `directive_target_analysis`, `unknown_identifier_warning`, Claudine `scope_site_conformance`, and DMLS protocol tests in `expression_scopes` |
| Ordinary composition and downstream conditions | Level 1 `strict_equality_composition`, Claudine lifecycle `conditions_control`, and CLI `sequence_grammar_matrix`; these cover interpolation, ternaries, block selection, guarded transclusion, operand errors, and skipped branches |
| Commit-message routing remains safe | Level 1 CLI `implement_plan_commit_message` and `compose_initialize_acceptance`; frozen prompts pass preflight, null/empty/absent messages use the generated route, supplied messages stay one argument, and clean trees execute neither commit route |
| Authored-expression migration and current documentation | Recorded corpus inventory, reviewed rewritten and retained comparisons, existing opt-in corpus checks, and the implementation log's normal-invocation composition checks |
| Required implementation gates | Implementation log records final `just test` and `just lint` passes in both areas, plus opt-in prompt checks with existing failures disclosed below |

All requirements concern semantic evaluation, passive analysis, or CLI/protocol
wiring. Level 1 is appropriate; none requires terminal rendering or keyboard
encoding evidence. The input robustness matrix does not apply: this change
modifies expression semantics, not a file-format or configuration loader.

Fresh review verification:

- Darkmatter and DMLS focused nextest run: **96 passed**, 9,536 unselected.
- Claudine library and CLI focused nextest run: **349 passed**, 9,935 unselected.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine`:
  **zero stranded tests**. New modules are declared by their consolidated
  Level-1 binaries and selected by the normal tier.
- `git diff --check`: passed.

The `just` wrappers rejected compound nextest filter expressions during shell
argument forwarding, so the focused runs used `cargo nextest run` directly.
No full-suite or lint rerun was performed during this review; those results
are implementation-log evidence. The compiler emitted a macOS linker warning
about compact unwind table size; both test runs completed successfully.
Cross-OS evidence remains CI's responsibility and does not block this review.

## Observations

### Opt-in prompt test expects an obsolete null default

In the claudine package,
[`shipped_implement_plan_prepares_with_unset_optional_commit_message`](../../../claudine/lib/src/composition/schema/tests.rs:169)
prepares the live implementation prompt without a supplied message and asserts
that the resulting `commit_message` is null. The
[prompt](../../../prompts/_implement/implement-plan.md:15) explicitly defaults
that property to `""`, so the assertion fails even though preparation preserves
the declared default correctly. The implementation log reports this opt-in
failure; source inspection confirms the mismatch and that the empty default
already existed at the implementation base. This review did not rerun that
known failing opt-in test. The smallest repair is to update its expectation
and explanatory text to the empty default; keep absent/null behavior in the
dedicated fixture tests. This is an existing test defect outside the fix's
changed code and does not block readiness. The other recorded prompt-check
failures arise while validating templates without their required arguments;
they do not establish an equality regression.
