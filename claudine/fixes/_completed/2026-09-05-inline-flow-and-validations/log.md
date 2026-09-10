---
fix: 2026-09-05-inline-flow-and-validations
implementation_2: 2026-09-07T15:40:46-07:00
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
        - confirmed the leak: `MODEL` outranks frontmatter in model resolution, so a host exporting `MODEL=…` replaced the fixture model `llamacpp/ac9-probe-model` and hard-failed `level2_lifecycle_equivalence_ac9_context_facets_match_direct_run` rather than skipping
        - scrubbed the variable at all three layers the host could reach:
                - `common::CliProcessFixture` calls `.env_remove("MODEL")` on the child env, with a `//` recording *why* (model resolution consults the generic `MODEL` var)
                - the L2 helper gained `staged_env_prefix` / `staged_env_prefix_augmented`, which prepend `MODEL=''` to every staged tmux invocation, and every call site was routed through them
                - the in-process target-launch tests route through shadow `rebuild_launch_identity` / `rebuild_target_launch` wrappers holding a `without_ambient_model()` guard for the rebuild duration
        - work landed in commit `d9e806ed1` together with Findings 1 and 2
- work completed for 'Finding 3 — ambient MODEL leaks into L2 equivalence tests' at 14:54:57-07:00
- session interrupted after Finding 3; **resumed at 15:40:46-07:00** to finish the remaining findings
        - reconciled the tree against the review before resuming: Findings 1, 2 and 3 are committed in `d9e806ed1`, and Finding 5's independent-axes follow-through is committed in `30c71f8d5`
        - the uncommitted `darkmatter/dmls/tests/lsp_session.rs` and `darkmatter/lib/tests/schemas_required_count_matrix.rs` work in the worktree belongs to the concurrent `darkmatter/fixes/2026-09-07-required-vs-eager` cycle, not to this review; left untouched
- starting the work on 'Finding 5 — uncommitted independent-axes follow-through' at 15:41:10-07:00
        - verified already landed in `30c71f8d5`: `def_is_required_at_completion` (`lib/src/composition/inline_prompt.rs:84-95`) now matches only `Constraint::Required`, `Eager` no longer contributes to the completion presence column
        - the same commit carries the four-cell axis matrix (both array placements plus a union arm), the renamed `eager_without_required_is_optional_at_completion` inversion, the `prepare_inline` `///` correction, and the two shared-resources doc updates
        - no further work required; the committed tree is no longer left between the two rulings
- work completed for 'Finding 5 — uncommitted independent-axes follow-through' at 15:41:10-07:00
- starting the work on 'Finding 4 — nested-anchoring name-collision regression test' at 15:41:40-07:00
        - the review pointed at `darkmatter/dmls/tests/lsp_session.rs`, but the nearest test — `nested_schema_conversion_error_points_to_the_nested_value` — is actually a unit test in the `mod tests` of `darkmatter/dmls/src/diagnostics/frontmatter.rs`. The new test was added there, beside it, which also kept the change entirely clear of the concurrently-edited `lsp_session.rs`
        - confirmed both halves of the mechanism:
                - `convert.rs` (`object_body_from_shape`) qualifies a nested failure as `format!("{context}.{prop_name}")`, bare `prop_name` only at `<root>`
                - `frontmatter.rs` resolves that string by trying `["$schema", property]` as one literal key, then `["$schema"] + property.split('.')`, then the `$schema` block span
        - **the two halves fail differently in the collision case**, which is why one fixture is enough: the dotted qualification is what stops the anchor landing on the valid top-level sibling, and the split-path fallback is what stops it degrading to the whole `$schema` block
        - added `nested_conversion_error_does_not_anchor_on_a_same_named_top_level_property`: a `$schema` with a valid top-level `prompt: string(required)`, an invalid nested `meta.prompt: string(integer)`, and a satisfying `prompt: hello` instance value so the valid sibling is genuinely clean
        - the test pins the `dm.schema.invalid_type_definition` range to the nested line *and* asserts every emitted diagnostic is confined to that line — one assertion that rules out both the valid top-level `prompt` and the `$schema` block fallback
        - no hover assertion: the neighbouring tests in this module assert on diagnostics only, and hover isolation is already covered at the `lsp_session.rs` tier
        - proved non-vacuity with two temporary source mutations, each reverted:
                - `convert.rs` — collapsed `qualified_name` to the bare `prop_name` → FAIL, the diagnostic moved onto the valid top-level `prompt` (exactly the review-1 #10 defect)
                - `frontmatter.rs` — deleted the `split('.')` fallback → FAIL, the range degraded to the `$schema` block
                - the pre-existing `nested_schema_conversion_error_points_to_the_nested_value` survives mutation 1, since it has no same-named sibling — precisely the gap this closes
        - verification in `darkmatter/` (that area's recipes cover DMLS): `just lint` → exit 0, clean across `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli` and the wasm32-wasip2 leg; `just test` → 7709 passed, 0 failed, 51 skipped (84.4s)
- work completed for 'Finding 4 — nested-anchoring name-collision regression test' at 15:49:52-07:00

### Successful Completion

The implementation of review cycle 2 has completed successfully in 2h 4m
(13:46:22-07:00 to 15:50:51-07:00, spanning one interruption and resume at
15:40:46-07:00). During this implementation all 5 review findings were
evaluated to see if they could be fixed as a part of this implementation
cycle: 5 were fixed, 0 were deferred.

Finding 1 (High) — the blocker that held `ready: false` — is closed: the
interrupt messaging now carries L1 stderr assertions, and the past-tense
"restored" that could contradict a failed rollback was reworded
prospectively. Findings 2, 3 and 4 are closed by a deleted scratch test, an
ambient-`MODEL` scrub at all three layers the host could reach, and the
nested-anchoring collision fixture. Finding 5 required no new work — the
independent-axes follow-through it flagged as uncommitted had already landed.

The files changed across this cycle:

- `claudine/cli/src/commands/wrap/inline.rs` — prospective interrupt wording, collapsed branches, `///` recording the seam ordering
- `claudine/cli/tests/inline_completion_lifecycle.rs` — stderr assertions on the exit-130 restore path
- `claudine/cli/tests/common/mod.rs`, `claudine/cli/tests/level2_lifecycle_control.rs`, `claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs` — ambient `MODEL` scrub
- `claudine/lib/src/composition/schema/tests.rs` — scratch test deleted, multi-element partial pinned
- `claudine/lib/src/composition/inline_prompt.rs`, `claudine/lib/src/composition/prepare.rs`, `claudine/lib/src/composition/prepare/tests.rs` — independent-axes completion column (Finding 5, landed in `30c71f8d5`)
- `darkmatter/dmls/src/diagnostics/frontmatter.rs` — the collision regression test

Findings 1, 2 and 3 are committed in `d9e806ed1`; Finding 5 in `30c71f8d5`.
The Finding 4 test is green in the worktree and awaits a commit, which is a
separate operation.
