---
fix: 2026-09-05-inline-flow-and-validations
implementation_2: 2026-09-07T13:46:22-07:00
---

# Log: Inline Flow and Validations

## Implementation of Review Findings #2

> **started at:** 2026-09-07T13:46:22-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-05-inline-flow-and-validations/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review carries five findings, worked serially by subagent:
        - **Finding 1 (High)** — the rewritten interrupt messaging in `report_inline_agent_status` has no test at any level, and the past-tense "restored" is printed before the rollback seam runs
        - **Finding 2 (Low)** — `scratch_dump_file_array_problems` is an assertion-free debug test left in the L1 suite
        - **Finding 3 (Low)** — the AC9 L2 equivalence test hard-fails on any host exporting `MODEL`
        - **Finding 4 (Low)** — no collision regression test for the nested-anchoring fix (nested leaf name colliding with a valid top-level property)
        - **Finding 5 (Note)** — the independent-axes Claudine-side change is uncommitted
- starting the work on 'Finding 1 — interrupt messaging verification' at 13:47:35-07:00
        - confirmed the ordering defect: `report_inline_agent_status` fires at the end of `run_attempt` (`harness_orch/attempt.rs`), while `rollback_inline_document` runs later in `classify_attempt_phase` (`harness_orch/loop_control.rs`)
        - **decision — reworded prospectively rather than moving the print.** Moving it is not surgical: the notice's trigger (`Interrupted || exit 130 || exit 143`) is strictly broader than the rollback branch's trigger (`termination == Interrupted`), and `rollback_inline_document` has six call sites. Relocating the print would silently drop the notice for a child that exits 130 without the wrapper observing a `SIGINT` — the exact case the AC8 stub exercises — so the message now reads "Claudine will restore `<doc>` to its pre-run state" and cannot contradict a later rollback-failure diagnostic
        - collapsed the interrupt block's two identical branches into one notice plus a conditional `Agent summary:` — the empty/non-empty arms printed byte-identical text, so keeping them apart was pure drift surface
        - added a `///` on `report_inline_agent_status` recording *why* the tense is prospective (the seam ordering), per the comment-quality positive criterion B
        - extended `a_provider_exit_130_restores_the_captured_baseline` (`cli/tests/inline_completion_lifecycle.rs`, L1) with stderr assertions: the CTRL+C attribution, the restore notice scoped to its own line (`doc.md` also appears in the unrelated file-reference check, so a whole-stderr `contains` would have passed vacuously), the `Agent summary:` label, and a summary sentinel
        - proved non-vacuity with two temporary source mutations, each reverted: past-tense "Claudine restored" → FAIL, and `Agent summary:` → `Summary:` → FAIL
        - verification: `just test` in `claudine/` → 6810 passed, 0 failed, 11 skipped (43.7s); `just lint` → exit 0, no clippy or fmt output
- work completed for 'Finding 1 — interrupt messaging verification' at 13:52:53-07:00
- starting the work on 'Finding 2 — assertion-free scratch test' at 13:54:22-07:00
        - confirmed the finding: `scratch_dump_file_array_problems` (`lib/src/composition/schema/tests.rs`) was three `eprintln!` loops over `report.problems` and the `attachments` atom shape, with zero assertions, running in the default L1 filter set
        - triaged its three cases against the neighbouring tests: `scalar-string` is already pinned end-to-end by `provided_file_scalar_for_array_property_match_partial_reports_unresolved_file_reference`, and `array-of-one` by `provided_file_array_match_partial_reports_unresolved_file_reference`. Both assert the typed `UnresolvedFileReference` with `is_array: true` — strictly stronger than the dump
        - **decision — delete, then re-pin the one genuinely uncovered case.** Converting the dump verbatim would pin `report.problems` message text, which is Darkmatter's validator output, not a Claudine contract — drift surface with no owner. The `array-of-two` case, however, had no coverage at the `pre_validate_schema` level: the helper test `provided_partial_value_handles_scalar_and_array_for_file_array` pins first-non-empty selection in isolation, but nothing proved a multi-element partial still reaches the typed error rather than collapsing to a generic `SchemaValidation`
        - deleted the scratch test and added `provided_file_array_multi_element_partial_reports_first_element` beside its siblings, asserting `property`/`provided`/`patterns`/`is_array` for `attachments: ["everywhere", "here"]`
        - verified `build_effective_instance` and `atom_for_property` (the scratch test's only callers in this file) remain used by `schema/mod.rs` and `schema/classify.rs`, so the deletion leaves no dead code
        - proved non-vacuity with two temporary mutations, each reverted: `provided` `"everywhere"` → `"here"` → FAIL (the first-element claim is load-bearing), and `is_array` → `!is_array` → FAIL
        - net test count unchanged at 6810 (one deleted, one added)
        - verification: `just lint` in `claudine/` → exit 0, no clippy or fmt output; `just test` → 6810 passed, 0 failed, 11 skipped (42.2s)
- work completed for 'Finding 2 — assertion-free scratch test' at 13:58:44-07:00
- starting the work on 'Finding 3 — ambient MODEL leaks into L2 equivalence tests' at 13:59:38-07:00
