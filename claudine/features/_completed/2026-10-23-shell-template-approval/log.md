---
implementation_1: "2026-10-14T20:35:33-07:00"
implementation_2: "2026-10-14T21:34:00-07:00"
implementation_3: "2026-10-14T21:53:37-07:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-10-14T20:35:33-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/features/2026-10-23-shell-template-approval/review-1.md'
- this is iteration 1 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | First-task shell sites support either exact or template approval and execute the selected mode | fixed | All required children verified; known first-task holes now retain either-mode graph candidates and settle before execution. |
| R1-F1.1 | First task and first-group member primary shell, setup/teardown shell, setup/teardown shell-valued set accept saved shapes | fixed | Ten first/group native recorder regressions passed, one per required primary/stack/set surface. |
| R1-F1.2 | Scripted approval offers exact and shape choices; default exact preference remains | fixed | Scripted graph test verifies offered exact/shape choices, persisted rule and settled mode; native first-group control proves exact priority with both rules present. |
| R1-F1.3 | Later task five surfaces retain template execution | clean | Five later-task native regressions passed; all preserve the independently expected `probe`, `a b` argument vector. |
| R1-F1.4 | First prompt document and standalone directive, shell-block, frontmatter and lifecycle shell/set siblings remain correct | clean | Ten first-prompt/standalone document sibling regressions passed, one per directive/block/frontmatter/lifecycle shell/set surface. |
| R1-F1.1.1 | first primary: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::first_primary_accepts_shape` passed. |
| R1-F1.1.2 | first setup: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::first_setup_accepts_shape` passed. |
| R1-F1.1.3 | first teardown: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::first_teardown_accepts_shape` passed. |
| R1-F1.1.4 | first setup_set: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::first_setup_set_accepts_shape` passed. |
| R1-F1.1.5 | first teardown_set: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::first_teardown_set_accepts_shape` passed. |
| R1-F1.1.6 | group primary: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::group_primary_accepts_shape` passed. |
| R1-F1.1.7 | group setup: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::group_setup_accepts_shape` passed. |
| R1-F1.1.8 | group teardown: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::group_teardown_accepts_shape` passed. |
| R1-F1.1.9 | group setup_set: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::group_setup_set_accepts_shape` passed. |
| R1-F1.1.10 | group teardown_set: saved shape preserves `a b` as one argument | fixed | `sequence_shell_templates::group_teardown_set_accepts_shape` passed. |
| R1-F1.3.1 | later primary: saved shape preserves `a b` as one argument | clean | `sequence_shell_templates::later_primary_accepts_shape` passed. |
| R1-F1.3.2 | later setup: saved shape preserves `a b` as one argument | clean | `sequence_shell_templates::later_setup_accepts_shape` passed. |
| R1-F1.3.3 | later teardown: saved shape preserves `a b` as one argument | clean | `sequence_shell_templates::later_teardown_accepts_shape` passed. |
| R1-F1.3.4 | later setup_set: saved shape preserves `a b` as one argument | clean | `sequence_shell_templates::later_setup_set_accepts_shape` passed. |
| R1-F1.3.5 | later teardown_set: saved shape preserves `a b` as one argument | clean | `sequence_shell_templates::later_teardown_set_accepts_shape` passed. |
| R1-F1.2.1 | Graph candidate offers exact and shape, persists selected row and settles mode | fixed | `first_task_shell_template_scripted_choices_persist_and_settle` passed in `just test preflight`. |
| R1-F1.2.2 | Exact rules retain preference when both exact and shape rules exist | clean | `a_first_group_offers_resolved_bytes_for_every_member` passed with simultaneous exact and template rules, retaining two resolved words for each member. |
| R1-F1.2.3 | Mode reaches stack stamping, environment reapproval and primary dispatch | fixed | Ten first/group cases preserve one argument; settlement runs before stack parsing and removes byte-only records for switched sites. |
| R1-F1.4.1 | Known-value prompt directive: saved shape preserves one argument | clean | `known_prompt_directive_accepts_shape` passed in final focused run. |
| R1-F1.4.2 | Known-value prompt block: saved shape preserves one argument | clean | `known_prompt_block_accepts_shape` passed in final focused run. |
| R1-F1.4.3 | Known-value prompt frontmatter: saved shape preserves one argument | clean | `known_prompt_frontmatter_accepts_shape` passed in final focused run. |
| R1-F1.4.4 | Known-value prompt shell: saved shape preserves one argument | clean | `known_prompt_shell_accepts_shape` passed in final focused run. |
| R1-F1.4.5 | Known-value prompt set: saved shape preserves one argument | clean | `known_prompt_set_accepts_shape` passed in final focused run. |
| R1-F1.4.6 | Known-value standalone directive: saved shape preserves one argument | clean | `known_standalone_directive_accepts_shape` passed in final focused run. |
| R1-F1.4.7 | Known-value standalone block: saved shape preserves one argument | clean | `known_standalone_block_accepts_shape` passed in final focused run. |
| R1-F1.4.8 | Known-value standalone frontmatter: saved shape preserves one argument | clean | `known_standalone_frontmatter_accepts_shape` passed in final focused run. |
| R1-F1.4.9 | Known-value standalone shell: saved shape preserves one argument | clean | `known_standalone_shell_accepts_shape` passed in final focused run. |
| R1-F1.4.10 | Known-value standalone set: saved shape preserves one argument | clean | `known_standalone_set_accepts_shape` passed in final focused run. |
| R1-F1.5 | Shared task fixture explicitly chooses exact approval for byte-parity/environment controls | fixed | Final full suite exposed stale shape-only fixture; focused task regressions and final rerun recorded below. |
| R1-F2 | Referenced prompt lifecycles retain sequence execution mode and validate deferred values before setup | fixed | All required event/forms, late barriers, lexical scope and sibling consumers verified; shared filler and frozen document snapshot used throughout. |
| R1-F2.1 | Six document event stacks (start, success, finalize, blocked, failure, loop.gate), shell and shell-valued set: prefix/shape preserve arguments, exact-only refused | fixed | Twelve event/form modules each verify prefix, shape and exact-only policy; inactive branches audited condition-blind. |
| R1-F2.2 | Missing and array late values in both forms fail before task setup; group roots remain lexically available in group members | fixed | Twenty-four event/form/value cases leave only prior recorded; lexical group and frozen-late/current-ordinary tests pass. |
| R1-F2.3 | Directive, shell-block, frontmatter and task-stack sibling consumers retain required mode and barrier | clean | Native sibling regressions passed alongside repaired referenced lifecycle cases, with independent argv and missing-value markers. |
| R1-F2.4 | Initialization remains forbidden; on_error remains warning-only | clean | Initialization regression selection passed; executor emits warning text and never dispatches on_error as a command. |
| R1-F2.1.1 | Referenced start shell: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_start_shell_modes` three native tests passed. |
| R1-F2.2.1 | Referenced start shell: missing/array late reads fail before setup | fixed | `referenced_shell_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.2 | Referenced start set: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_start_set_modes` three native tests passed. |
| R1-F2.2.2 | Referenced start set: missing/array late reads fail before setup | fixed | `referenced_set_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.3 | Referenced success shell: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_success_shell_modes` three native tests passed. |
| R1-F2.2.3 | Referenced success shell: missing/array late reads fail before setup | fixed | `referenced_success_shell_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.4 | Referenced success set: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_success_set_modes` three native tests passed. |
| R1-F2.2.4 | Referenced success set: missing/array late reads fail before setup | fixed | `referenced_success_set_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.5 | Referenced finalize shell: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_finalize_shell_modes` three native tests passed. |
| R1-F2.2.5 | Referenced finalize shell: missing/array late reads fail before setup | fixed | `referenced_finalize_shell_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.6 | Referenced finalize set: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_finalize_set_modes` three native tests passed. |
| R1-F2.2.6 | Referenced finalize set: missing/array late reads fail before setup | fixed | `referenced_finalize_set_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.7 | Referenced blocked shell: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_blocked_shell_modes` three native tests passed. |
| R1-F2.2.7 | Referenced blocked shell: missing/array late reads fail before setup | fixed | `referenced_blocked_shell_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.8 | Referenced blocked set: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_blocked_set_modes` three native tests passed. |
| R1-F2.2.8 | Referenced blocked set: missing/array late reads fail before setup | fixed | `referenced_blocked_set_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.9 | Referenced failure shell: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_failure_shell_modes` three native tests passed. |
| R1-F2.2.9 | Referenced failure shell: missing/array late reads fail before setup | fixed | `referenced_failure_shell_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.10 | Referenced failure set: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_failure_set_modes` three native tests passed. |
| R1-F2.2.10 | Referenced failure set: missing/array late reads fail before setup | fixed | `referenced_failure_set_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.11 | Referenced loop shell: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_loop_shell_modes` three native tests passed. |
| R1-F2.2.11 | Referenced loop shell: missing/array late reads fail before setup | fixed | `referenced_loop_shell_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.1.12 | Referenced loop set: prefix/shape keep arguments and exact-only refuses condition-blind before the sequence | fixed | `referenced_loop_set_modes` three native tests passed. |
| R1-F2.2.12 | Referenced loop set: missing/array late reads fail before setup | fixed | `referenced_loop_set_missing_barrier` and corresponding array test passed; recorder contains only prior. |
| R1-F2.2.13 | Group shell/set read lexical group variables; missing member fails before setup | fixed | Four `referenced_group_*` native tests passed, expecting one `a b` argument or no command. |
| R1-F2.2.14 | Both lifecycle forms retain frozen late values while ordinary values remain current | fixed | `sequence_lifecycle_shell_snapshot_survives_later_preparation` passed, explicitly asserting barrier value and current ordinary value in both action forms. |
| R1-F2.2.15 | Derived late frontmatter retains authored timing through graph and JIT; task params override defaults | fixed | `referenced_lifecycle_derived_late_value_retains_its_provenance` and `referenced_lifecycle_parameter_overrides_the_child_default` passed. |
| R1-F2.2.16 | Absent/null known values and compound forbidden err/current reads fail before prior, in both forms | fixed | Eight `referenced_*_known`/`referenced_*_compound_*` native regressions passed. |
| R1-F2.3.1 | Directive/block/frontmatter/transclusion and task-stack siblings keep argument and barrier contracts | clean | Existing 59 sequence template sibling cases passed alongside lifecycle repairs, including native independently expected argv. |
| R1-F2.4.1 | Initialization command/set shells forbidden even in inactive branches | clean | `just test initialize`: 125 passed, including grammar dead-branch and programmatic runtime refusals. One unrelated existing schema CLI test took 5.102s. |
| R1-F2.4.2 | on_error remains warning-only, without a new adapter | clean | Executor branches call `render_shell_text` then `emitter.emit_warn`; neither sends warning text to the shell runner. Corrected stale collector comment claiming execution. |
| R1-F2.2.17 | Outside-group shell/set reads refuse upfront; group metadata stays lexical | fixed | `referenced_shell_group_outside` and `referenced_set_group_outside` passed; `PromptReference.in_group` distinguishes group membership. |
| R1-F2.1.13 | JIT lifecycle execution proves graph template membership, never exact-byte membership | fixed | `sequence_lifecycle_requires_preapproved_shape_membership` passed; exact bytes alone cannot authorize JIT lifecycle template execution. |
| R1-F2.5 | New passive graph lifecycle discovery has a reasoned composition-seam guard entry | fixed | Exact site allowlisted as frontmatter-only discovery under an existing prepared request; no prompt or shell execution. Final full-area gate verifies guard. |
| R1-F3 | Group entry validates all members before any member command runs and retains late snapshots | fixed | All required surfaces and sibling consumers verified; group entry freezes member late values and retains document snapshots/epoch/context without scheduler redesign. |
| R1-F3.1 | Serial and parallel primary/setup/teardown shell and shell-valued set reject missing/array values before alpha | fixed | Twenty task-site matrix cases passed, proving refusal before alpha and beta setup/primary/teardown. |
| R1-F3.2 | Serial and parallel prompt directive, shell-block, frontmatter and lifecycle shell/set reject missing/array values before alpha | fixed | Sixty-four document-site matrix cases passed, including transclusion and all six lifecycle events in both action forms. |
| R1-F3.3 | Ordinary shell/prompt and transcluded document barriers remain correct | clean | Existing ordinary shell/prompt missing/invalid and transcluded-document native barrier regressions passed in sequence_shell_templates. |
| R1-F3.1.1 | Serial/parallel member primary: missing group member and array outputs refuse before alpha | fixed | `group_barrier_primary`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.1.2 | Serial/parallel member setup: missing group member and array outputs refuse before alpha | fixed | `group_barrier_setup`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.1.3 | Serial/parallel member teardown: missing group member and array outputs refuse before alpha | fixed | `group_barrier_teardown`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.1.4 | Serial/parallel member setup_set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_setup_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.1.5 | Serial/parallel member teardown_set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_teardown_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.1 | Serial/parallel document directive: missing group member and array outputs refuse before alpha | fixed | `group_barrier_directive`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.2 | Serial/parallel document block: missing group member and array outputs refuse before alpha | fixed | `group_barrier_block`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.3 | Serial/parallel document frontmatter: missing group member and array outputs refuse before alpha | fixed | `group_barrier_frontmatter`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.4 | Serial/parallel document transcluded: missing group member and array outputs refuse before alpha | fixed | `group_barrier_transcluded`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.5 | Serial/parallel document start.shell: missing group member and array outputs refuse before alpha | fixed | `group_barrier_start_shell`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.6 | Serial/parallel document start.set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_start_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.7 | Serial/parallel document success.shell: missing group member and array outputs refuse before alpha | fixed | `group_barrier_success_shell`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.8 | Serial/parallel document success.set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_success_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.9 | Serial/parallel document finalize.shell: missing group member and array outputs refuse before alpha | fixed | `group_barrier_finalize_shell`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.10 | Serial/parallel document finalize.set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_finalize_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.11 | Serial/parallel document blocked.shell: missing group member and array outputs refuse before alpha | fixed | `group_barrier_blocked_shell`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.12 | Serial/parallel document blocked.set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_blocked_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.13 | Serial/parallel document failure.shell: missing group member and array outputs refuse before alpha | fixed | `group_barrier_failure_shell`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.14 | Serial/parallel document failure.set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_failure_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.15 | Serial/parallel document loop.shell: missing group member and array outputs refuse before alpha | fixed | `group_barrier_loop_shell`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.2.16 | Serial/parallel document loop.set: missing group member and array outputs refuse before alpha | fixed | `group_barrier_loop_set`: all four serial/parallel missing/array native tests passed; only prior recorded. |
| R1-F3.4 | All ten shell surfaces plus transclusion freeze group-entry late outputs while ordinary values observe prior member mutations | fixed | Eleven group_snapshot native controls passed: prior output seed remains frozen despite alpha producing seed\nseed; ordinary out reads after. |
| R1-F3.5 | Group own setup cannot run: schema forbids setup/teardown on group tasks | clean | `tts_prewarm::a_group_container_with_setup_is_rejected_before_any_preparation` passed in group_ selection; no container surface added. |
| R1-F3.6 | Serial shell-free members retain JIT Git capture; parallel members retain separate provider sessions and coherent epochs | fixed | `just test group_`: 204 passed, including serial Git staging, serial/parallel environment captures, and unchanged exact epoch/provider-session assertion. |
| R1-F3.7 | Prompt dispatch rebinds parallel private runtime and stream while retaining validated snapshot/request identity | fixed | `group_snapshot_parallel_prompt_mutations_stay_private_until_merge` passed: writer sees after, reader before, next step after; late inputs seed in both legs. |
| R1-O1 | Nonblocking pre-existing graph discovery omission for referenced lifecycle approvals | fixed | Same shared-request passive lifecycle discovery now contributes bytes and templates to the graph audit; all twelve event/form exact-only tests refuse before prior. |
| R1-O2 | Future nested containers and group terminal events have no execution surface | deferred | Review explicitly says not to implement containers solely for this review. |

- starting the work on 'R1-F1: first-task either-mode approval' at 20:35:33
        - Defect class: byte-eligible task shell sites discard their typed approval candidate and never settle shape membership into execution mode.
        - Sweep traced primary commands, both task stacks, shell-valued set, first-group members, graph authorization, stack stamping, reapproval, and primary dispatch. Later task and document siblings use existing shared candidates and template dispatch.

        - `just test sequence_shell_templates`: 49 passed, including all ten repaired first-task/group surfaces and five later-task siblings; no recorder argument split.
        - `just test preflight`: 160 passed, including scripted exact/shape offers, persistence, and mode settlement. Informational macOS compact-unwind linker warning only.
        - Added ten first-prompt and standalone document sibling regressions and strengthened the native first-group exact-preference control; final focused run in progress.

        - Final focused `cd claudine && just test sequence_shell_templates`: **59 passed** (including ten document siblings and exact-priority control); every test finished below 2 seconds. The canonical L1 target compiles all added cases; no new tier markers or feature gates were added.
        - `git diff --check`: passed. Current first-task behavior replaced obsolete planned caveats in both topic pages and the Claudine skill; no new dependencies or platform-specific APIs.
        - Changed for R1-F1: `claudine/lib/src/composition/sequence/preflight/mod.rs`, `claudine/lib/src/composition/sequence/preflight/tests.rs`, `claudine/lib/src/composition/sequence/task/mod.rs`, `claudine/lib/src/composition/preflight.rs`, `claudine/cli/tests/l1/sequence_shell_templates.rs`, `claudine/docs/topics/pre-flight-checks.md`, `claudine/docs/topics/flow-control/sequences.md`, `.claude/skills/claudine/SKILL.md`, and this log.
- work completed for 'R1-F1: first-task either-mode approval' at 20:40:42

- starting the work on 'R1-F2: referenced prompt lifecycle templates' at 20:43:00
        - Defect class: lifecycle adapters lose the enclosing task's static mode and deferred-value snapshot, allowing byte execution and validation after setup.
        - Sweep includes all six event stacks and both action forms, directive/frontmatter siblings, task stacks, initialization and warning-only on_error.

        - Native sequence regressions passed: `just test sequence_shell_templates`, **133 passed**, including all six events in both forms under prefix, shape and exact-only rules, all missing/array event barriers, group lexical scope, known absent/null and forbidden compounds, derived late provenance and task parameter precedence. Each new split case stayed below two seconds.
        - Graph discovery now also inventories referenced lifecycle commands using passive frontmatter-only composition under the existing request, closing observation R1-O1. No shell runs to discover a placeholder.

        - `just test preflight`: **160 passed**; snapshot-retention focused test: **1 passed**; `just test initialize`: **125 passed**, including initialization shell/set refusals. The initialization run reported an existing unrelated schema CLI test at 5.102s; retained F2 cases remain under two seconds.

        - Final focused verification after membership and lexical-scope repairs: `just test sequence_shell_templates` **135 passed**, `just test sequence_lifecycle` **2 passed**, and `git diff --check` passed. No new test exceeds two seconds; all compile in existing L1 targets and no tier markers/features were added.
        - Changed for R1-F2: `claudine/lib/src/composition/preflight.rs`, `prepare.rs`, `prepare/tests.rs`, `error/tests.rs`, `lifecycle/shell_template.rs`, `lifecycle/validate.rs`, `sequence/preflight/mod.rs`, `sequence/task/mod.rs`; `claudine/cli/src/commands/compose/prep.rs`, `wrap/sequence/{jit.rs,jit/tests.rs,task_run.rs,phase1c.rs,iterate.rs,mod.rs}`; `claudine/cli/tests/l1/sequence_shell_templates.rs`; `darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs`; `claudine/docs/topics/pre-flight-checks.md`, `.claude/skills/claudine/SKILL.md`, and this log.
        - Handoff for R1-F3: `PromptTaskRunner::validate` is passive and now covers lifecycle stacks as well as document directives. Preserve the request's `document_shell_values`, captured epoch/context, and `lifecycle_group` when validating before group dispatch and later executing members. `PromptReference.in_group` records lexical membership for graph discovery. No group scheduler behavior was changed in R1-F2.
- work completed for 'R1-F2: referenced prompt lifecycle templates' at 20:54:00

- starting the work on 'R1-F3: group-wide late-value barrier' at 20:55:00
        - Defect class: a composite step begins effects before every member's late shell inputs are checked, and subsequent member entry can replace the validated snapshot.
        - Sweep covers primary commands, both task stacks and set shells, referenced document directives/block/frontmatter and lifecycle shells, both schedulers, group setup, and ordinary/transcluded task barriers.

        - Both schedulers passed all **84** missing/array barrier cases, including all six lifecycle events in shell and whole-value set forms and transcluded directives. Eleven native positive controls separately proved frozen late outputs and live ordinary mutations.
        - Broader group checks exposed serial context-capture timing and a redundant shell-free lifecycle probe. Shell-free serial prompts now resume their existing JIT capture; structural traversal skips a second document preparation when no lifecycle shell/set site exists. Late-shell readers retain the barrier epoch/context without recapture. No scheduler or provider-session model changed.

        - Final broad focused group check: `cd claudine && just test group_`, **204 passed**, preserving the existing environment, staging, session, exact epoch, scheduler and mutation-isolation assertions. Earlier context/environment failures and the invalid group-setup fixture were corrected; no checks were weakened.
        - Structural shell-free lifecycle detection checks both action variants across all event branches and loop gate, including layered caller/proxy overrides. It avoids a redundant passive body preparation while keeping literal or dynamic shell actions on the shared validation path.

        - Final focused `cd claudine && just test sequence_shell_templates`: **231 passed** after all production edits, including ordinary shell/prompt/transclusion sibling barriers and parallel prompt private-runtime rebinding. `just test serial_groups`: **12 passed**, independently preserving ordinary serial task params/mutations/outputs. `git diff --check`: passed. All new cases compile in the existing L1 target with no new feature or tier gates and finished below three seconds in the final run.
        - Changed for R1-F3: `claudine/lib/src/composition/sequence/task/{mod.rs,group.rs}`, `claudine/lib/src/composition/prepare.rs`, `claudine/cli/src/commands/wrap/sequence/task_run.rs`, `claudine/cli/tests/l1/sequence_shell_templates.rs`, `darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs` (snapshot emptiness accessor), `claudine/docs/topics/pre-flight-checks.md`, `claudine/docs/topics/flow-control/sequences.md`, `.claude/skills/claudine/SKILL.md`, and this log. Existing comment drift claiming serial shell late values read predecessor outputs was corrected to describe the frozen group barrier and live ordinary state.
        - R1-F3 disposition: **fixed**, with every required instance and downstream request/runtime/context consumer verified. No open child rows, deferred performance work, new dependencies, platform-specific APIs, formatter runs, or commits. Final full-area verification remains the orchestrator's closing step.
- work completed for 'R1-F3: group-wide late-value barrier' at 21:04:19

- starting the work on 'R1-F1: final-suite fixture authorization reconciliation' at 21:05:43
        - Full `cd claudine && just test` stopped at 1,608 of 9,464 tests: 1,603 passed and five failed. The byte-parity/document-environment fixture granted every candidate a shape, which now selected template mode for new either-mode first-task candidates.
        - Defect class: a test authorization fixture conflates a site's available approval modes with the mode its byte-parity assertions intend to exercise.
        - Sweep found one shared fixture loop; it now records exact membership for either-mode candidates and shape membership for template-required candidates, preserving all existing behavior assertions.
        - Focused `cd claudine && just test-library composition::sequence::task::tests`: **136 passed**, including all five unchanged byte-parity/environment assertions. An initial invocation from the repository root selected no recipe; it was corrected to the package area before testing.
- work completed for 'R1-F1: final-suite fixture authorization reconciliation' at 21:07:25

- Final verification scope: Claudine library/CLI and Darkmatter's additive shared snapshot API. Sniff dependency inspection confirms Claudine's downstream contract/CLI are included by its area recipe; Darkmatter's other consumers retain unchanged signatures and behavior. No OS-specific code or process runner changed; portable native recorder cases run locally on macOS, and remote OS execution is not required for these state/approval transport repairs. No terminal or browser tier is required by the reviewed behavior.

- Final full-suite integration: the second Claudine run stopped on `composition_seams::compose_with_allowlist_holds_the_line`, after 5,750 passing tests. The new graph audit's passive frontmatter-only read needed an exact allowlist entry with a reason; added that entry without changing the guard or its assertions.
        - Several unchanged tests crossed five seconds during the full 16-worker run, including existing schema and removed-grammar CLI cases. Final verification uses `NEXTEST_TEST_THREADS=4` to reduce local contention; test filters, coverage and deadlines remain unchanged. Sniff CPU/memory observation reports 16 cores and substantial memory/swap use; a bounded filesystem usage query exhausted its discovery budget and establishes no process attribution.
        - The four-worker retry was initially launched from the repository root by mistake. It was stopped during workspace compilation (exit 143), with only its identified process tree terminated, then relaunched from `claudine/`. No workspace-wide test result is claimed. Process inspection revealed an independent `just test` invocation, which was left untouched; this corroborates concurrent host load without attributing its workload.

- Final `cd claudine && NEXTEST_TEST_THREADS=4 just test`: **9,464 passed**, nine existing skips, 413.695 seconds. All 231 sequence shell-template cases passed; their maximum duration was 2.506 seconds. No repair test exceeded five seconds.
        - Two unchanged agent-value tests took 5.531 and 8.788 seconds under the full concurrent suite. Focused `NEXTEST_TEST_THREADS=1 just test-cli agent_text_is_data`: **28 passed**, 22.698 seconds; those two took 1.676 and 2.998 seconds and no test exceeded five seconds. The independent run resolves the timing concern as contention; no required performance measurement is deferred.

- Final `cd claudine && just lint`: **passed**, including transport/doc guards and all five area crates' Clippy checks with warnings denied. No formatter was invoked. Darkmatter full-area L1 and lint are next, followed by sequential tier audits.

- Final `cd darkmatter && NEXTEST_TEST_THREADS=4 just test`: **10,113 passed**, ten existing skips, 107.922 seconds, no tests above five seconds. Covers the shared shell template/parser/filler/policy regressions and all default area crates including DMLS.

- Final `cd darkmatter && just lint`: **passed**, including all area Clippy checks and the Zed extension's `wasm32-wasip2` compile check. No formatter or warning suppression was introduced.

- Final sequential tier audits: `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter` both **passed**, each reporting zero stranded tests. `git diff --check` **passed**. All required ledger IDs are unique, have verification evidence, and have a final disposition.

### Completion Report

- **Outcome: complete**. Finished at 2026-10-14T21:26:38-07:00; elapsed **0 h 51 min 5 s** from the requested start timestamp. Implementation complete, ready for review.
- Parent findings: **total 3 = fixed 3 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0**. Every required instance and downstream consumer is fixed or independently checked clean; no required row remains open.
- Verified repairs:
        - R1-F1: known first-task and first-group sites retain either-mode candidates, expose both prompt choices, and execute the selected mode; exact preference and fixed-byte environment checks remain covered.
        - R1-F2: referenced lifecycle shell/set commands retain sequence position, lexical group availability, authored/derived timing, approved membership and late snapshots. Every event is audited before the sequence and validated before setup. Observation R1-O1 is also fixed by passive graph discovery.
        - R1-F3: both schedulers validate every member's late inputs before any member dispatch. Native effect markers, snapshot/live-value controls, context/environment/Git observations, provider epochs, and private parallel mutations all pass without weakening existing assertions.
- Remaining required finding/child IDs: **none**. Nonblocking **R1-O2** remains **deferred**: nested sequence containers and group terminal events have no current runtime surface, and the review expressly forbids implementing those containers solely for this cycle. Next action: implement and test those surfaces when the separately scoped container feature is authorized; this observation does not prevent this cycle from being complete.
- No required performance measurement is deferred. The two unchanged full-suite timing outliers both passed below the five-second budget in the isolated canonical run. No formatter, commit, CI cell, dependency, real provider, or focus-taking window was introduced.
- Review metadata now records this log, `implemented: true`, and `implemented_by: codex/gpt-6.1-sol`; the reviewer's readiness, findings and body remain intact. Log metadata records the requested `implementation_1` timestamp.

| Final verification command | Result | Findings verified |
| --- | --- | --- |
| `cd claudine && NEXTEST_TEST_THREADS=4 just test` | 9,464 passed; nine existing skips; all shell-template cases below three seconds | R1-F1, R1-F2, R1-F3 and existing behavior |
| `cd claudine && NEXTEST_TEST_THREADS=1 just test-cli agent_text_is_data` | 28 passed; two unchanged timing outliers verified at 1.676/2.998 seconds | Full-suite timing concern, unrelated to required repair instances |
| `cd claudine && just lint` | Passed, including transport/doc guards and all five crates | R1-F1, R1-F2, R1-F3 |
| `cd darkmatter && NEXTEST_TEST_THREADS=4 just test` | 10,113 passed; ten existing skips; no slow tests | R1-F2/R1-F3 shared snapshot API and unchanged shared shell behavior |
| `cd darkmatter && just lint` | Passed, including Zed WASM compile | R1-F2/R1-F3 shared snapshot API |
| `just check-tier-coverage claudine` | Passed, zero stranded tests | All added regression cases |
| `just check-tier-coverage darkmatter` | Passed, zero stranded tests | Shared template regression suite |
| `git diff --check` | Passed | Combined changes |
| `md get` review/log keys | Parsed and read back successfully | Completion metadata |

### Changed Files

| File | Findings |
| --- | --- |
| `.claude/skills/claudine/SKILL.md` | R1-F1, R1-F2, R1-F3 |
| `claudine/cli/src/commands/compose/prep.rs` | R1-F2 |
| `claudine/cli/src/commands/wrap/sequence/iterate.rs` | R1-F2 |
| `claudine/cli/src/commands/wrap/sequence/jit.rs` | R1-F2 |
| `claudine/cli/src/commands/wrap/sequence/jit/tests.rs` | R1-F2 |
| `claudine/cli/src/commands/wrap/sequence/mod.rs` | R1-F2 |
| `claudine/cli/src/commands/wrap/sequence/phase1c.rs` | R1-F2 |
| `claudine/cli/src/commands/wrap/sequence/task_run.rs` | R1-F2, R1-F3 |
| `claudine/cli/tests/l1/composition_seams.rs` | R1-F2 |
| `claudine/cli/tests/l1/sequence_shell_templates.rs` | R1-F1, R1-F2, R1-F3 |
| `claudine/docs/topics/flow-control/sequences.md` | R1-F1, R1-F2, R1-F3 |
| `claudine/docs/topics/pre-flight-checks.md` | R1-F1, R1-F2, R1-F3 |
| `claudine/features/2026-10-23-shell-template-approval/log.md` | R1-F1, R1-F2, R1-F3 |
| `claudine/features/2026-10-23-shell-template-approval/review-1.md` | R1-F1, R1-F2, R1-F3 |
| `claudine/lib/src/composition/error/tests.rs` | R1-F2 |
| `claudine/lib/src/composition/lifecycle/shell_template.rs` | R1-F2 |
| `claudine/lib/src/composition/lifecycle/validate.rs` | R1-F2 |
| `claudine/lib/src/composition/preflight.rs` | R1-F2 |
| `claudine/lib/src/composition/prepare.rs` | R1-F2, R1-F3 |
| `claudine/lib/src/composition/prepare/tests.rs` | R1-F2 |
| `claudine/lib/src/composition/sequence/preflight/mod.rs` | R1-F1, R1-F2 |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | R1-F1 |
| `claudine/lib/src/composition/sequence/task/group.rs` | R1-F3 |
| `claudine/lib/src/composition/sequence/task/mod.rs` | R1-F1, R1-F2, R1-F3 |
| `claudine/lib/src/composition/sequence/task/tests.rs` | R1-F1 |
| `darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs` | R1-F2, R1-F3 |


## Implementation of Review Findings #2

> **started at:** 2026-10-14T21:34:00-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/features/2026-10-23-shell-template-approval/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- Defect class: lifecycle adapters impose template syntax before deciding whether resolved-byte execution is eligible.
- The review has no explicit finding IDs; this ledger assigns R2-F1 to its single high finding and retains that title. Both observations are nonblocking existing limitations.

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | First-task prompt lifecycles incorrectly require template grammar; preserve resolved-byte eligibility | fixed | Bounded mode-order repair; all 97 new cases passed in final 9,561-test area run; lint, tier audit, and diff checks passed. |
| R2-F1.1.1 | first prompt start shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_start::first_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.1.2 | first-group prompt member start shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_start::group_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.2.1 | first prompt success shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_success::first_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.2.2 | first-group prompt member success shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_success::group_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.3.1 | first prompt finalize shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_finalize::first_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.3.2 | first-group prompt member finalize shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_finalize::group_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.4.1 | first prompt failure shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_failure::first_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.4.2 | first-group prompt member failure shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_failure::group_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.5.1 | first prompt blocked shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_blocked::first_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.5.2 | first-group prompt member blocked shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_blocked::group_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.6.1 | first prompt loop.gate shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_loop::first_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.6.2 | first-group prompt member loop.gate shell accepts known executable interpolation via exact approval | fixed | `byte_grammar_loop::group_short` passes with exact approval; the long-form counterpart also passes. Active stacks record `[probe]`; inactive stacks remain audited without execution. |
| R2-F1.7 | Long-form shell.command preserves byte eligibility in first prompt and first-group member | fixed | `byte_grammar_{start,success,finalize,failure,blocked,loop}::{first_long,group_long}`: all 12 pass with exact rules; shared boundary covers both forms. |
| R2-F1.8 | First prompt known argument plus semicolon executes approved chain | fixed | `byte_grammar_first_chain` and `byte_grammar_group_chain` pass: independently expected recorder vectors `[dist]`, then `[tail]`. POSIX uses semicolon; native Windows uses its system-shell `&` separator. |
| R2-F1.9 | Standalone six stacks and long form preserve executable interpolation and semicolon behavior | clean | All six events × standalone short/long controls plus standalone chain pass; active events record `[probe]`, inactive failure/blocked record nothing; standalone loop gate records once. |
| R2-F1.10 | Later prompt six stacks reject invalid template grammar before predecessor effects | clean | All six events × later short/long controls pass: invalid template grammar refusal precedes the literal predecessor (no recorder calls). |
| R2-F1.11 | Ordinary first task and first-group primary/setup/teardown preserve resolved-byte behavior | clean | Six `byte_grammar_{first,group}_{primary,setup,teardown}` controls pass, asserting exact `[probe]` and primary ordering. |
| R2-F1.12 | Whole-value lifecycle set and ordinary setup/teardown set preserve existing interpolated-executable refusals | clean | All six events × first/group lifecycle set, standalone start set, and four ordinary first/group setup/teardown set controls pass; executable diagnostics and zero native calls preserve the existing restriction. |
| R2-F1.13 | Document directive/block/frontmatter preserve existing interpolated-executable refusals | clean | Six `byte_grammar_{first,standalone}_{directive,block,frontmatter}` controls pass: each refuses before any recorder call; existing executable insertion checks precede mode selection. |
| R2-F1.14 | Late reads still require templates; mode decision reaches upfront audit and JIT preparation | fixed | Shared preparation tests cover both passive audit and JIT for all six stacks and both forms; direct/derived late and forbidden reads refuse. Four native first-task late/forbidden controls pass. |
| R2-F1.14.1 | Passive graph audit retains first-task resolved bytes independent of template grammar | fixed | Twelve `sequence_lifecycle_byte_grammar_*` tests call `validate_document_shell_late_values` and assert exact known command bytes with no template. |
| R2-F1.14.2 | JIT preparation preserves those known bytes for every signal and shell form | fixed | The same twelve tests independently call `prepare_sequence_probe` and assert resolved bytes and no template. |
| R2-F1.14.3 | First-task direct and derived late reads still require valid template grammar | clean | Shared late/forbidden matrix and native `byte_grammar_first_late_{program,chain}` controls pass, refusing before effects. |
| R2-F1.14.4 | Forbidden roots remain refused even with late reads and invalid template grammar | clean | Native first-task error/current controls plus shared matrix retain forbidden-root diagnostics ahead of grammar validation. |
| R2-F1.15 | Lifecycle on_error text has no command execution or template-grammar decision | clean | `LifecycleExecutor::run_shell_action` renders on_error only as warning text; it cannot introduce another instance of this command-mode defect. |
| R2-F1.7.1.1 | first prompt start long shell.command accepts approved known executable | fixed | `byte_grammar_start::first_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.7.1.2 | group prompt start long shell.command accepts approved known executable | fixed | `byte_grammar_start::group_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.9.1.1 | Standalone start short preserves byte execution | clean | `byte_grammar_start::standalone_short` passed; independently expected `[probe]`. |
| R2-F1.10.1.1 | Later prompt start short refuses before predecessor | clean | `byte_grammar_start::later_short_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.9.1.2 | Standalone start long preserves byte execution | clean | `byte_grammar_start::standalone_long` passed; independently expected `[probe]`. |
| R2-F1.10.1.2 | Later prompt start long refuses before predecessor | clean | `byte_grammar_start::later_long_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.12.1.1 | first prompt start whole-value set preserves executable refusal | clean | `byte_grammar_start::first_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.1.2 | group prompt start whole-value set preserves executable refusal | clean | `byte_grammar_start::group_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.7.2.1 | first prompt success long shell.command accepts approved known executable | fixed | `byte_grammar_success::first_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.7.2.2 | group prompt success long shell.command accepts approved known executable | fixed | `byte_grammar_success::group_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.9.2.1 | Standalone success short preserves byte execution | clean | `byte_grammar_success::standalone_short` passed; independently expected `[probe]`. |
| R2-F1.10.2.1 | Later prompt success short refuses before predecessor | clean | `byte_grammar_success::later_short_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.9.2.2 | Standalone success long preserves byte execution | clean | `byte_grammar_success::standalone_long` passed; independently expected `[probe]`. |
| R2-F1.10.2.2 | Later prompt success long refuses before predecessor | clean | `byte_grammar_success::later_long_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.12.2.1 | first prompt success whole-value set preserves executable refusal | clean | `byte_grammar_success::first_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.2.2 | group prompt success whole-value set preserves executable refusal | clean | `byte_grammar_success::group_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.7.3.1 | first prompt finalize long shell.command accepts approved known executable | fixed | `byte_grammar_finalize::first_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.7.3.2 | group prompt finalize long shell.command accepts approved known executable | fixed | `byte_grammar_finalize::group_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.9.3.1 | Standalone finalize short preserves byte execution | clean | `byte_grammar_finalize::standalone_short` passed; independently expected `[probe]`. |
| R2-F1.10.3.1 | Later prompt finalize short refuses before predecessor | clean | `byte_grammar_finalize::later_short_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.9.3.2 | Standalone finalize long preserves byte execution | clean | `byte_grammar_finalize::standalone_long` passed; independently expected `[probe]`. |
| R2-F1.10.3.2 | Later prompt finalize long refuses before predecessor | clean | `byte_grammar_finalize::later_long_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.12.3.1 | first prompt finalize whole-value set preserves executable refusal | clean | `byte_grammar_finalize::first_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.3.2 | group prompt finalize whole-value set preserves executable refusal | clean | `byte_grammar_finalize::group_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.7.4.1 | first prompt failure long shell.command accepts approved known executable | fixed | `byte_grammar_failure::first_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.7.4.2 | group prompt failure long shell.command accepts approved known executable | fixed | `byte_grammar_failure::group_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.9.4.1 | Standalone failure short preserves byte execution | clean | `byte_grammar_failure::standalone_short` passed; independently expected no calls. |
| R2-F1.10.4.1 | Later prompt failure short refuses before predecessor | clean | `byte_grammar_failure::later_short_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.9.4.2 | Standalone failure long preserves byte execution | clean | `byte_grammar_failure::standalone_long` passed; independently expected no calls. |
| R2-F1.10.4.2 | Later prompt failure long refuses before predecessor | clean | `byte_grammar_failure::later_long_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.12.4.1 | first prompt failure whole-value set preserves executable refusal | clean | `byte_grammar_failure::first_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.4.2 | group prompt failure whole-value set preserves executable refusal | clean | `byte_grammar_failure::group_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.7.5.1 | first prompt blocked long shell.command accepts approved known executable | fixed | `byte_grammar_blocked::first_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.7.5.2 | group prompt blocked long shell.command accepts approved known executable | fixed | `byte_grammar_blocked::group_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.9.5.1 | Standalone blocked short preserves byte execution | clean | `byte_grammar_blocked::standalone_short` passed; independently expected no calls. |
| R2-F1.10.5.1 | Later prompt blocked short refuses before predecessor | clean | `byte_grammar_blocked::later_short_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.9.5.2 | Standalone blocked long preserves byte execution | clean | `byte_grammar_blocked::standalone_long` passed; independently expected no calls. |
| R2-F1.10.5.2 | Later prompt blocked long refuses before predecessor | clean | `byte_grammar_blocked::later_long_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.12.5.1 | first prompt blocked whole-value set preserves executable refusal | clean | `byte_grammar_blocked::first_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.5.2 | group prompt blocked whole-value set preserves executable refusal | clean | `byte_grammar_blocked::group_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.7.6.1 | first prompt loop long shell.command accepts approved known executable | fixed | `byte_grammar_loop::first_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.7.6.2 | group prompt loop long shell.command accepts approved known executable | fixed | `byte_grammar_loop::group_long` passed; argv is `[probe]` for active stacks, empty for inactive stacks. |
| R2-F1.9.6.1 | Standalone loop short preserves byte execution | clean | `byte_grammar_loop::standalone_short` passed; independently expected `[probe]`. |
| R2-F1.10.6.1 | Later prompt loop short refuses before predecessor | clean | `byte_grammar_loop::later_short_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.9.6.2 | Standalone loop long preserves byte execution | clean | `byte_grammar_loop::standalone_long` passed; independently expected `[probe]`. |
| R2-F1.10.6.2 | Later prompt loop long refuses before predecessor | clean | `byte_grammar_loop::later_long_refused` passed; grammar diagnostic and zero recorder calls. |
| R2-F1.12.6.1 | first prompt loop whole-value set preserves executable refusal | clean | `byte_grammar_loop::first_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.6.2 | group prompt loop whole-value set preserves executable refusal | clean | `byte_grammar_loop::group_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.11.1.1 | first ordinary task primary preserves exact executable approval | clean | `byte_grammar_first_primary` passed; exact argv and ordering asserted. |
| R2-F1.11.1.2 | first ordinary task setup preserves exact executable approval | clean | `byte_grammar_first_setup` passed; exact argv and ordering asserted. |
| R2-F1.11.1.3 | first ordinary task teardown preserves exact executable approval | clean | `byte_grammar_first_teardown` passed; exact argv and ordering asserted. |
| R2-F1.12.7.1.1 | first ordinary task setup_set preserves executable refusal | clean | `byte_grammar_first_setup_set` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.7.1.2 | first ordinary task teardown_set preserves executable refusal | clean | `byte_grammar_first_teardown_set` passed; executable diagnostic and zero recorder calls. |
| R2-F1.11.2.1 | group ordinary task primary preserves exact executable approval | clean | `byte_grammar_group_primary` passed; exact argv and ordering asserted. |
| R2-F1.11.2.2 | group ordinary task setup preserves exact executable approval | clean | `byte_grammar_group_setup` passed; exact argv and ordering asserted. |
| R2-F1.11.2.3 | group ordinary task teardown preserves exact executable approval | clean | `byte_grammar_group_teardown` passed; exact argv and ordering asserted. |
| R2-F1.12.7.2.1 | group ordinary task setup_set preserves executable refusal | clean | `byte_grammar_group_setup_set` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.7.2.2 | group ordinary task teardown_set preserves executable refusal | clean | `byte_grammar_group_teardown_set` passed; executable diagnostic and zero recorder calls. |
| R2-F1.12.8 | Standalone start whole-value set preserves executable refusal | clean | `byte_grammar_standalone_set_refused` passed; executable diagnostic and zero recorder calls. |
| R2-F1.13.1.1 | first document directive preserves inserted-executable refusal | clean | `byte_grammar_first_directive` passed; failure and zero native calls, tied to existing executable-insertion contract. |
| R2-F1.13.1.2 | first document block preserves inserted-executable refusal | clean | `byte_grammar_first_block` passed; failure and zero native calls, tied to existing executable-insertion contract. |
| R2-F1.13.1.3 | first document frontmatter preserves inserted-executable refusal | clean | `byte_grammar_first_frontmatter` passed; failure and zero native calls, tied to existing executable-insertion contract. |
| R2-F1.13.2.1 | standalone document directive preserves inserted-executable refusal | clean | `byte_grammar_standalone_directive` passed; failure and zero native calls, tied to existing executable-insertion contract. |
| R2-F1.13.2.2 | standalone document block preserves inserted-executable refusal | clean | `byte_grammar_standalone_block` passed; failure and zero native calls, tied to existing executable-insertion contract. |
| R2-F1.13.2.3 | standalone document frontmatter preserves inserted-executable refusal | clean | `byte_grammar_standalone_frontmatter` passed; failure and zero native calls, tied to existing executable-insertion contract. |
| R2-F1.9.7 | Standalone authored chain preserves exact shell execution | clean | `byte_grammar_standalone_chain` passed; exactly `[dist]`, then `[tail]`. |
| R2-O1 | Existing document and set restrictions on interpolated executables | deferred | Nonblocking observation; review explicitly requires no broader implementation. Preserve and verify restrictions under R2-F1.12/13. |
| R2-O2 | Future containers have no execution surface | deferred | Nonblocking observation; implementing containers is not required by this repair. |

- starting the work on 'R2-F1: First-task prompt lifecycles incorrectly require template grammar' at 21:35:09
        - Defect class: execution adapters validate template grammar before deciding whether known first-task commands may use resolved bytes.
        - Sibling sweep: `resolve_lifecycle_shells` sends every signal and both shell action forms through the same helper; upfront passive discovery and JIT preparation share that boundary. Ordinary sequence preparation already scans expressions independently on failed template parsing. Whole-value set and document directive/block/frontmatter executable checks precede mode selection and remain required existing restrictions.
        - Moved first-task/no-late-read eligibility before typed template parsing, using the existing expression scanner and derived late-root analysis. Forbidden-root diagnostics still run first; later-task grammar and audited membership checks remain in place.
        - Added 84 native CLI cases and 13 shared preparation cases. Native controls prove both required recorder output and absence of extra effects; preparation cases assert independently expected command bytes separately for passive audit and JIT preparation.
        - Updated the current pre-flight documentation with configured-executable and authored-chain examples and the existing document/set limitations. No dependencies or runtime architecture changed.
        - Focused verification: `cd claudine && just test-cli byte_grammar` passed 84/84 in 3.989 seconds; `cd claudine && just test-library sequence_lifecycle` passed 15/15 in 0.703 seconds (13 new boundary cases plus existing snapshot and approval-membership controls). All selected individual tests stayed below five seconds. `git diff --check` passed.
        - Initial diagnostic attempts caught a private enum import compile error and incorrect standalone-loop expected results; both test issues were corrected before the passing final selections. The existing macOS compact-unwind linker warning remained informational.
        - All new cases are in already declared L1 targets: the CLI `l1/main.rs` includes `sequence_shell_templates`, and library preparation tests are in the existing unit-test module. CLI cases run with the recipe-enabled `test-fixtures` feature. No terminal, provider service, formatter, commit, or external OS command was used. Native Windows chain tests use its existing `cmd` separator; the production correction is platform-independent.
        - Changed files for R2-F1: `claudine/lib/src/composition/preflight.rs`, `claudine/lib/src/composition/prepare/tests.rs`, `claudine/cli/tests/l1/sequence_shell_templates.rs`, `claudine/docs/topics/pre-flight-checks.md`, and this log.
- work completed for 'R2-F1: First-task prompt lifecycles incorrectly require template grammar' at 21:39:58

- starting the work on 'final reconciliation and area verification' at 21:40:34
        - Independently inspected mode-order repair, forbidden-root validation, ordinary-task sibling behavior, test assertions, declared L1 target, and local/CI fixture features. Expanded grouped ledger evidence into individual required event/form/consumer rows.
        - Final scope: Claudine library and CLI; no public API or Darkmatter code changed during this cycle. Full-area `NEXTEST_TEST_THREADS=4 just test` started; lint and tier audit will follow sequentially.
        - Cross-platform consideration: decision logic uses portable Rust with no OS path or process change; native chain regression uses the existing system shell separator per platform. This cycle runs locally on macOS; remote OS behavior is not claimed. L1 is sufficient for the reviewed contract.

- Full-area L1 progress: more than 6,000 tests passed with no failures or tests above five seconds observed. Test/lint/tier commands remain sequential to preserve the fixture-enabled CLI binary.

- Final `cd claudine && NEXTEST_TEST_THREADS=4 just test`: **9,561 passed**, nine existing skips, 294.504 seconds. All 97 new byte-grammar cases were selected and passed; no test exceeded five seconds. Final area lint started after test completion.

- Final `cd claudine && just lint`: **passed**. Transport guard (nine tests), lifecycle doc guard, and Clippy for all five area crates passed. The existing macOS compact-unwind linker warning is informational.
- Final `just check-tier-coverage claudine`: **passed**, zero stranded tests. `git diff --check`: **passed**. Ledger audit: 94 unique IDs, no pending rows, every required child fixed or verified clean.
- Input robustness matrix: not applicable; this change reads expression/template text and does not change a configuration/file-format reader.
- work completed for 'final reconciliation and area verification' at 21:47:58

### Completion Report

- **Outcome: complete**. Finished at 2026-10-14T21:47:58-07:00; elapsed **0 h 13 min 58 s** from the requested start timestamp. Implementation complete, ready for review.
- Parent findings: **total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0**. Observation rows are excluded from parent finding counts.
- R2-F1: shared referenced-lifecycle preparation determines resolved-byte eligibility and direct/derived late reads before imposing template grammar. Known first-task and first-group commands retain exact-byte approval. Invalid templates have no shape candidate. Later-task and late-read commands retain grammar refusals; forbidden roots remain refused before that decision.
- Independently reconciled every row against tests and code. All six lifecycle stacks, both shell forms, native chain execution, upfront audit, JIT preparation, ordinary task siblings, standalone controls, set/directive restrictions, and no-predecessor-effect refusal controls are verified. All 97 new tests run in the declared L1 targets and fixture features; none exceeded 0.627 seconds in the final area run. No area test exceeded five seconds.
- Remaining required finding/child IDs: **none**. Nonblocking observations remain **deferred**:
        - **R2-O1**: interpolated executables remain unsupported by existing document directive/frontmatter and shell-valued set parsers. Next action: address only through a separately authorized behavior change; this review explicitly excludes expanding them. Their current refusals are verified by R2-F1.12/13.
        - **R2-O2**: nested sequence containers and group terminal events still have no execution surface. Next action: implement and verify them with the separately scoped container work; this review expressly does not require that implementation.
- No required performance measurement was deferred; the specification requires no performance spike. Local macOS verification completed; remote Linux/native Windows/WSL execution was not performed or claimed. No OS-specific production code changed.
- Completion metadata records the requested log link, `implemented: true`, `implemented_by: codex/gpt-6.1-sol`, and `implementation_2` timestamp. The reviewer’s readiness judgment, findings, and body are preserved. No formatter, commit, dependency, CI cell, real provider, or focus-taking window was introduced.

| Final verification command | Result | Findings verified |
| --- | --- | --- |
| `cd claudine && just test-cli byte_grammar` | 84 passed; 3.989 seconds | R2-F1 and all native CLI instances/sibling controls |
| `cd claudine && just test-library sequence_lifecycle` | 15 passed; includes 13 new audit/JIT/late/forbidden cases and two existing snapshot/membership cases | R2-F1.14 and every shared lifecycle preparation event/form |
| `cd claudine && NEXTEST_TEST_THREADS=4 just test` | 9,561 passed, nine existing skips; 294.504 seconds; all 97 new cases selected | R2-F1 and all children; existing area behavior |
| `cd claudine && just lint` | Passed | R2-F1; area transport/doc/Clippy checks |
| `just check-tier-coverage claudine` | Passed, zero stranded tests | Test placement and live tier selection |
| `git diff --check` | Passed | Combined working tree |
| `md get` review/log completion keys | Parsed and read back successfully | Completion metadata |

### Changed Files

| File changed during this cycle | Finding or purpose |
| --- | --- |
| `claudine/lib/src/composition/preflight.rs` | R2-F1 mode decision before template parsing |
| `claudine/lib/src/composition/prepare/tests.rs` | R2-F1.14 shared audit/JIT and late/forbidden controls |
| `claudine/cli/tests/l1/sequence_shell_templates.rs` | R2-F1 native instances and sibling controls |
| `claudine/docs/topics/pre-flight-checks.md` | R2-F1 exact-byte grammar contract and compact examples |
| `claudine/features/2026-10-23-shell-template-approval/log.md` | Inventory, progress, reconciled evidence and completion report |
| `claudine/features/2026-10-23-shell-template-approval/review-2.md` | Requested completion metadata only |


## Implementation of Review Findings #3

> **started at:** 2026-10-14T21:53:37-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/features/2026-10-23-shell-template-approval/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- Defect class: resolved-byte approval discards authored program-placeholder restrictions and can offer or persist unrestricted program approval.
- The review has no explicit IDs; this cycle assigns R3-F1 to its single high finding and R3-O1/R3-O2 to its nonblocking observations. Spec sections 5 and 8 and acceptance criterion 8 authorize the bounded metadata repair.

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | Executable placeholders must not offer or accept unrestricted program approval | fixed | All required children independently reconciled; focused regressions, both full-area L1/lint runs, and both tier audits passed. |
| R3-F1.1 | Shared candidate collection and choice projection retain program-hole restrictions, including invalid template grammar | fixed | Authored candidate text survives template errors; Darkmatter authored-program choice matrix plus shared constructor compilation. |
| R3-F1.2 | Claudine byte fallback hides prefix and rejects unoffered prefix decisions without policy writes | fixed | 96 public adapter tests reject both unoffered prefix and shape grants with no saved grant. |
| R3-F1.3 | First ordinary primary/setup/teardown, both shell forms and both chain program positions | fixed | 12 first-task public adapter tests pass all six decisions. |
| R3-F1.4 | First-group primary/setup/teardown, both shell forms and both program positions | fixed | 12 first-group task public adapter tests pass all six decisions. |
| R3-F1.5 | First prompt all six lifecycle stacks, both shell forms and both program positions | fixed | 24 first-prompt public adapter tests pass all six decisions. |
| R3-F1.6 | First-group prompt all six lifecycle stacks, both shell forms and both program positions | fixed | 24 group-prompt public adapter tests pass all six decisions. |
| R3-F1.7 | Standalone all six lifecycle stacks, both shell forms and both program positions | fixed | 24 standalone public adapter tests pass all six decisions, including repeated resolution. |
| R3-F1.8 | Later prompt all six lifecycle stacks and both forms retain grammar refusal | clean | 12 native later-prompt grammar refusals pass; no predecessor recorder calls. |
| R3-F1.9 | Late executable or unsupported late argument retains pre-execution refusal | clean | Four native first-late/forbidden tests pass with no recorder calls. |
| R3-F1.10 | Directive/block/frontmatter standalone and first-prompt executable rejection | clean | Six native directive/block/frontmatter controls pass with no recorder calls. |
| R3-F1.11 | Whole-value set six prompt stacks and task setup/teardown including groups retain rejection | clean | All six events, task stages and group/standalone native set-refusal controls pass. |
| R3-F1.12 | Fixed program argument placeholders retain prefix approval/persistence standalone, first and later | fixed | Existing standalone/later choice matrices pass; first-task scripted matrix now also persists prefix git. |
| R3-F1.13 | Exact/once/deny/exact-blacklist remain available and valid for executable placeholders | fixed | Every new adapter test proves exact membership or exact-blacklist persistence, once success and deny failure; all six handler decisions tested. |
| R3-F1.3.1.1 | first_primary: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_primary::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.1.2 | first_primary: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_primary::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.1.3 | first_primary: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_primary::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.1.4 | first_primary: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_primary::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.2.1 | first_setup: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_setup::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.2.2 | first_setup: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_setup::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.2.3 | first_setup: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_setup::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.2.4 | first_setup: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_setup::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.3.1 | first_teardown: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_teardown::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.3.2 | first_teardown: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_teardown::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.3.3 | first_teardown: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_teardown::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.3.3.4 | first_teardown: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::first_teardown::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.1.1 | group_primary: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_primary::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.1.2 | group_primary: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_primary::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.1.3 | group_primary: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_primary::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.1.4 | group_primary: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_primary::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.2.1 | group_setup: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_setup::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.2.2 | group_setup: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_setup::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.2.3 | group_setup: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_setup::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.2.4 | group_setup: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_setup::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.3.1 | group_teardown: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_teardown::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.3.2 | group_teardown: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_teardown::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.3.3 | group_teardown: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_teardown::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.4.3.4 | group_teardown: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_teardown::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.1.1 | prompt_start: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_start::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.1.2 | prompt_start: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_start::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.1.3 | prompt_start: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_start::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.1.4 | prompt_start: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_start::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.2.1 | prompt_success: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_success::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.2.2 | prompt_success: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_success::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.2.3 | prompt_success: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_success::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.2.4 | prompt_success: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_success::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.3.1 | prompt_finalize: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_finalize::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.3.2 | prompt_finalize: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_finalize::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.3.3 | prompt_finalize: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_finalize::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.3.4 | prompt_finalize: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_finalize::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.4.1 | prompt_failure: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_failure::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.4.2 | prompt_failure: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_failure::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.4.3 | prompt_failure: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_failure::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.4.4 | prompt_failure: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_failure::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.5.1 | prompt_blocked: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_blocked::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.5.2 | prompt_blocked: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_blocked::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.5.3 | prompt_blocked: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_blocked::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.5.4 | prompt_blocked: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_blocked::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.6.1 | prompt_loop: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_loop::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.6.2 | prompt_loop: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_loop::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.6.3 | prompt_loop: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_loop::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.5.6.4 | prompt_loop: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::prompt_loop::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.1.1 | group_prompt_start: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_start::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.1.2 | group_prompt_start: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_start::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.1.3 | group_prompt_start: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_start::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.1.4 | group_prompt_start: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_start::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.2.1 | group_prompt_success: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_success::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.2.2 | group_prompt_success: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_success::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.2.3 | group_prompt_success: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_success::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.2.4 | group_prompt_success: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_success::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.3.1 | group_prompt_finalize: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_finalize::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.3.2 | group_prompt_finalize: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_finalize::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.3.3 | group_prompt_finalize: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_finalize::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.3.4 | group_prompt_finalize: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_finalize::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.4.1 | group_prompt_failure: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_failure::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.4.2 | group_prompt_failure: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_failure::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.4.3 | group_prompt_failure: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_failure::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.4.4 | group_prompt_failure: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_failure::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.5.1 | group_prompt_blocked: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_blocked::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.5.2 | group_prompt_blocked: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_blocked::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.5.3 | group_prompt_blocked: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_blocked::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.5.4 | group_prompt_blocked: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_blocked::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.6.1 | group_prompt_loop: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_loop::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.6.2 | group_prompt_loop: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_loop::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.6.3 | group_prompt_loop: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_loop::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.6.6.4 | group_prompt_loop: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::group_prompt_loop::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.1.1 | standalone_start: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_start::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.1.2 | standalone_start: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_start::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.1.3 | standalone_start: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_start::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.1.4 | standalone_start: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_start::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.2.1 | standalone_success: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_success::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.2.2 | standalone_success: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_success::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.2.3 | standalone_success: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_success::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.2.4 | standalone_success: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_success::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.3.1 | standalone_finalize: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_finalize::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.3.2 | standalone_finalize: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_finalize::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.3.3 | standalone_finalize: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_finalize::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.3.4 | standalone_finalize: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_finalize::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.4.1 | standalone_failure: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_failure::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.4.2 | standalone_failure: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_failure::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.4.3 | standalone_failure: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_failure::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.4.4 | standalone_failure: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_failure::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.5.1 | standalone_blocked: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_blocked::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.5.2 | standalone_blocked: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_blocked::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.5.3 | standalone_blocked: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_blocked::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.5.4 | standalone_blocked: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_blocked::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.6.1 | standalone_loop: short form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_loop::short_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.6.2 | standalone_loop: long form, first program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_loop::long_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.6.3 | standalone_loop: short form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_loop::short_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.7.6.4 | standalone_loop: long form, second chain program; exact/once/deny/blacklist valid, prefix denied without grant | fixed | `shell_template_program_approval::standalone_loop::long_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.1.1 | Later prompt start short retains grammar refusal before predecessor effects | clean | `byte_grammar_start::later_short_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.1.2 | Later prompt start long retains grammar refusal before predecessor effects | clean | `byte_grammar_start::later_long_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.2.1 | Later prompt success short retains grammar refusal before predecessor effects | clean | `byte_grammar_success::later_short_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.2.2 | Later prompt success long retains grammar refusal before predecessor effects | clean | `byte_grammar_success::later_long_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.3.1 | Later prompt finalize short retains grammar refusal before predecessor effects | clean | `byte_grammar_finalize::later_short_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.3.2 | Later prompt finalize long retains grammar refusal before predecessor effects | clean | `byte_grammar_finalize::later_long_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.4.1 | Later prompt failure short retains grammar refusal before predecessor effects | clean | `byte_grammar_failure::later_short_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.4.2 | Later prompt failure long retains grammar refusal before predecessor effects | clean | `byte_grammar_failure::later_long_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.5.1 | Later prompt blocked short retains grammar refusal before predecessor effects | clean | `byte_grammar_blocked::later_short_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.5.2 | Later prompt blocked long retains grammar refusal before predecessor effects | clean | `byte_grammar_blocked::later_long_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.6.1 | Later prompt loop short retains grammar refusal before predecessor effects | clean | `byte_grammar_loop::later_short_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.8.6.2 | Later prompt loop long retains grammar refusal before predecessor effects | clean | `byte_grammar_loop::later_long_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.9.1 | byte_grammar_first_late_program: reject before any native effect | clean | `byte_grammar_first_late_program` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.9.2 | byte_grammar_first_late_chain: reject before any native effect | clean | `byte_grammar_first_late_chain` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.9.3 | byte_grammar_first_forbidden_error: reject before any native effect | clean | `byte_grammar_first_forbidden_error` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.9.4 | byte_grammar_first_forbidden_current: reject before any native effect | clean | `byte_grammar_first_forbidden_current` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.10.1.1 | first directive rejects inserted executable before execution | clean | `byte_grammar_first_directive` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.10.1.2 | first block rejects inserted executable before execution | clean | `byte_grammar_first_block` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.10.1.3 | first frontmatter rejects inserted executable before execution | clean | `byte_grammar_first_frontmatter` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.10.2.1 | standalone directive rejects inserted executable before execution | clean | `byte_grammar_standalone_directive` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.10.2.2 | standalone block rejects inserted executable before execution | clean | `byte_grammar_standalone_block` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.10.2.3 | standalone frontmatter rejects inserted executable before execution | clean | `byte_grammar_standalone_frontmatter` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.1.1 | first prompt start whole-value set rejects inserted executable | clean | `byte_grammar_start::first_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.1.2 | group prompt start whole-value set rejects inserted executable | clean | `byte_grammar_start::group_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.2.1 | first prompt success whole-value set rejects inserted executable | clean | `byte_grammar_success::first_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.2.2 | group prompt success whole-value set rejects inserted executable | clean | `byte_grammar_success::group_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.3.1 | first prompt finalize whole-value set rejects inserted executable | clean | `byte_grammar_finalize::first_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.3.2 | group prompt finalize whole-value set rejects inserted executable | clean | `byte_grammar_finalize::group_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.4.1 | first prompt failure whole-value set rejects inserted executable | clean | `byte_grammar_failure::first_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.4.2 | group prompt failure whole-value set rejects inserted executable | clean | `byte_grammar_failure::group_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.5.1 | first prompt blocked whole-value set rejects inserted executable | clean | `byte_grammar_blocked::first_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.5.2 | group prompt blocked whole-value set rejects inserted executable | clean | `byte_grammar_blocked::group_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.6.1 | first prompt loop whole-value set rejects inserted executable | clean | `byte_grammar_loop::first_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.6.2 | group prompt loop whole-value set rejects inserted executable | clean | `byte_grammar_loop::group_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.7.1.1 | first task setup whole-value set rejects inserted executable | clean | `byte_grammar_first_setup_set` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.7.1.2 | first task teardown whole-value set rejects inserted executable | clean | `byte_grammar_first_teardown_set` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.7.2.1 | group task setup whole-value set rejects inserted executable | clean | `byte_grammar_group_setup_set` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.7.2.2 | group task teardown whole-value set rejects inserted executable | clean | `byte_grammar_group_teardown_set` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.11.8 | Standalone whole-value set retains executable rejection | clean | `byte_grammar_standalone_set_refused` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.12.1 | Fixed program argument placeholder standalone retains prefix choice and persisted program rule | clean | `each_choice_at_a_resolved_site_saves_its_row` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.12.2 | Fixed program argument placeholder first task retains prefix choice and persisted program rule | fixed | `first_task_shell_template_scripted_choices_persist_and_settle` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.12.3 | Fixed program argument placeholder later task retains prefix choice and persisted program rule | clean | `each_choice_at_a_template_site_saves_its_row` Passed; scripted/native result assertions verify independently expected grants, refusals and effects. |
| R3-F1.14 | Candidate settlement and runtime membership preserve resolved-byte mode for invalid templates | clean | `shape()` returns None for a failed template; `settle_task_shell_modes` cannot select shape mode. Native byte-grammar first/group/lifecycle controls prove required output and ordering. |
| R3-F1.15 | Policy/cache and downstream byte pass cannot re-offer prefix after restricted site decisions | fixed | Each adapter asserts one scripted request and the independently expected approved-byte membership; rejected choices persist no grant. |
| R3-F1.16 | Repeated lifecycle resolution preserves authored program-hole eligibility | fixed | All 24 standalone adapter cases resolve twice before public approval; all six handler decisions retain restricted choices. |
| R3-F1.17.1 | Leading curated redirects do not hide executable holes; fixed-program argument holes retain prefix | fixed | Candidate authored-program table and public `leading_curated_redirects_withdraw_program_choice` passed; exact stays offered and unoffered prefix persists no grant. |
| R3-F1.17.2 | LF/CRLF legacy native-shell second programs containing holes do not offer prefix; quoted line breaks remain argument data | fixed | Candidate authored-program table and public `native_shell_line_boundaries_withdraw_program_choice` passed; exact and template choices stay offered and unoffered prefix persists no grant. |
| R3-F1.17.1.1 | Leading stdout null redirect: executable hole withdraws prefix; fixed program argument hole retains prefix | fixed | Candidate authored-program table and public `leading_curated_redirects_withdraw_program_choice` passed; exact stays offered and unoffered prefix persists no grant. |
| R3-F1.17.1.2 | Leading stderr null redirect: executable hole withdraws prefix; fixed program argument hole retains prefix | fixed | Candidate authored-program table and public `leading_curated_redirects_withdraw_program_choice` passed; exact stays offered and unoffered prefix persists no grant. |
| R3-F1.17.1.3 | Leading stdout-to-stderr redirect: executable hole withdraws prefix; fixed program argument hole retains prefix | fixed | Candidate authored-program table and public `leading_curated_redirects_withdraw_program_choice` passed; exact stays offered and unoffered prefix persists no grant. |
| R3-F1.17.1.4 | Leading stderr-to-stdout redirect: executable hole withdraws prefix; fixed program argument hole retains prefix | fixed | Candidate authored-program table and public `leading_curated_redirects_withdraw_program_choice` passed; exact stays offered and unoffered prefix persists no grant. |
| R3-F1.17.2.1 | LF executable boundary: variable second native-shell program withdraws prefix and cannot persist an unoffered grant | fixed | Candidate authored-program table and public `native_shell_line_boundaries_withdraw_program_choice` passed; exact and template choices stay offered and unoffered prefix persists no grant. |
| R3-F1.17.2.2 | CRLF executable boundary: variable second native-shell program withdraws prefix and cannot persist an unoffered grant | fixed | Candidate authored-program table and public `native_shell_line_boundaries_withdraw_program_choice` passed; exact and template choices stay offered and unoffered prefix persists no grant. |
| R3-F1.17.2.3 | LF plus stdout-null redirect: variable second native-shell program withdraws prefix and cannot persist an unoffered grant | fixed | Candidate authored-program table and public `native_shell_line_boundaries_withdraw_program_choice` passed; exact and template choices stay offered and unoffered prefix persists no grant. |
| R3-F1.17.2.4 | CRLF plus stderr-null redirect: variable second native-shell program withdraws prefix and cannot persist an unoffered grant | fixed | Candidate authored-program table and public `native_shell_line_boundaries_withdraw_program_choice` passed; exact and template choices stay offered and unoffered prefix persists no grant. |
| R3-F1.17.2.5 | LF plus stdout-to-stderr redirect: variable second native-shell program withdraws prefix and cannot persist an unoffered grant | fixed | Candidate authored-program table and public `native_shell_line_boundaries_withdraw_program_choice` passed; exact and template choices stay offered and unoffered prefix persists no grant. |
| R3-F1.17.2.6 | CRLF plus stderr-to-stdout redirect: variable second native-shell program withdraws prefix and cannot persist an unoffered grant | fixed | Candidate authored-program table and public `native_shell_line_boundaries_withdraw_program_choice` passed; exact and template choices stay offered and unoffered prefix persists no grant. |
| R3-F1.17.2.7 | Quoted LF/CRLF and fixed-program argument holes after boundaries retain prefix | clean | Candidate authored-program table passed with independently expected prefix=true for quoted LF/CRLF and every fixed-program redirect/boundary control. |
| R3-F1.17.3 | Restricted newline-plus-redirect candidates must reach site approval even when strict byte tokenization fails | fixed | Public lifecycle test covers all four redirect combinations and rejects unoffered prefix without a policy write. |
| R3-O1 | Existing document and set executable interpolation restrictions | deferred | Nonblocking observation; expansion is separate work; current behavior checked under R3-F1.10/11. |
| R3-O2 | Future nested containers and terminal group events | deferred | Nonblocking observation; spec leaves container expansion to later work; no runtime surface to repair. |

- starting the work on 'R3-F1: Executable placeholders still offer unrestricted program approval' at 21:54:00
        - Sibling sweep found metadata loss at candidate choice projection, ordinary sequence candidate collection (invalid shapes were omitted), and lifecycle resolution/collection (authored bytes were discarded when template grammar failed). Both primary and setup/teardown/group tasks share the first boundary; all six lifecycle stacks and both shell forms share the second. Existing document/set executable-insertion rejection precedes these adapters.
        - Retaining authored program evidence through these boundaries and extending site prompts to byte-eligible candidates without valid template shapes. Added public approval/persistence adapter tests for all event/form/program-position combinations; these also exercise exact, once, deny and exact-blacklist decisions.
        - The public matrix caught the existing chain-as-one-byte-command adapter: site prompting must use the same approval key as the legacy byte validator. Preserved that representation and execution behavior while carrying authored restrictions into the prompt; no quoting or dispatch layer was added.
        - Expanded the ledger into individual adapter/event/form/program-position rows and refusal-control instances. A small authored-word scan recognizes expression spans before shell separators, so an unsupported argument construct does not erase evidence about a later program or suppress fixed-program eligibility.
        - Initial regression iterations corrected fixture-only group and loop syntax mistakes, caught by positive discovery assertions; all ordinary and first-group task cases and all referenced lifecycle events subsequently reached the expected scripted approval path.
        - Retained authored lifecycle text across repeated resolution; all 24 standalone adapter cases now resolve twice and still offer restricted choices. Candidate settlement cannot choose a shape for invalid template grammar, so byte dispatch and membership remain unchanged.
        - Focused final checks: `cd claudine && NEXTEST_TEST_THREADS=4 just test shell_template` passed **440/440** in 36.360 seconds, including all native refusal/output controls; `just test-library preflight` passed **200/200** in 1.453 seconds; final `just test-library shell_template_program_approval` passed **96/96** in 0.967 seconds after the repeated-resolution and unoffered-shape controls. Each new adapter case tests all six handler decisions. `cd darkmatter && NEXTEST_TEST_THREADS=4 just test shell_template` passed **75/75** in 1.472 seconds, including the new authored-program eligibility table.
        - Preserved fixed-program prefix approval: standalone and template-required choice matrices passed, and the first-task scripted choice matrix now explicitly checks saving `prefix git`. Exact/once/deny/exact-blacklist cases assert independently expected approved-byte membership and policy effects; unoffered prefix/shape decisions write no grant.
        - Updated both areas' current approval documentation and skill guidance to explain authored-program restrictions and invalid-template candidate retention. No new dependency, configuration/file-format parser, formatter, commit, real provider, terminal window or OS-specific production branch was added; the input robustness matrix is not applicable to authored shell text.
        - Files changed for R3-F1: `.claude/skills/claudine/SKILL.md`, `.claude/skills/darkmatter/compose.md`, `claudine/docs/topics/pre-flight-checks.md`, `claudine/lib/src/composition/lifecycle/{action_shape.rs,actions.rs,shell_template.rs}`, `claudine/lib/src/composition/preflight.rs`, `claudine/lib/src/composition/sequence/preflight/{mod.rs,tests.rs}`, `claudine/lib/src/harness/shell.rs`, `darkmatter/docs/inline/preflight-checks.md`, `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs`, `darkmatter/lib/src/markdown/compose/shell_expansion/site.rs`, `darkmatter/lib/src/markdown/compose/shell_expansion/template/{candidate.rs,tests.rs}`, and this log.
        - `git diff --check` passed. Ledger audit found **157 unique IDs**, no pending rows; required rows are fixed or verified clean. No blockers or required performance deferrals remain. Final full-area test/lint/tier checks and completion metadata remain the orchestrator's responsibility.
- work completed for 'R3-F1: Executable placeholders still offer unrestricted program approval' at 22:03:38

- starting the work on 'R3-F1 reconciliation: redirect and line-boundary program evidence' at 22:05:30
        - Defect class: authored program classification can mistake a leading redirection for a program and treat native-shell command line boundaries as argument whitespace.
        - Swept the four curated redirect forms and LF/CRLF boundaries, including fixed-program argument-hole controls. Existing template parsing already proves program holes behind curated redirects; native-shell consumers additionally require line-boundary evidence because template parsing treats lines as argument whitespace.



- Reconciliation repair and verification:
        - Reused `ProgramWordHole` as decisive parser evidence and successful parsing for commands without line boundaries. Native-shell line scanning reuses the same parser on each next physical line so leading curated redirects cannot conceal the next program; quoted LF/CRLF remain data. No grammar expansion or new shell protection layer.
        - The combined newline/redirect sweep also caught a downstream strict-tokenizer fallback that returned before site prompting. Restricted-prefix candidates now retain their existing native-shell approval key through that fallback.
        - Verification after the final behavior change: `cd claudine && NEXTEST_TEST_THREADS=4 just test-library shell_template_program_approval` passed **98/98** in 1.237 seconds; `cd darkmatter && NEXTEST_TEST_THREADS=4 just test shell_template_program_choices` passed **1/1** in 0.075 seconds. Tests ran as compiled L1 unit tests. The Claudine linker emitted its existing `__eh_frame` size warning; no test failed. `git diff --check` passed.
        - Follow-up changed files: `darkmatter/lib/src/markdown/compose/shell_expansion/template/{candidate.rs,tests.rs}`, `claudine/lib/src/harness/shell.rs`, `claudine/lib/src/composition/sequence/preflight/tests.rs`, and this log. No completion metadata was changed; full-area final gates remain with the orchestrator.
- work completed for 'R3-F1 reconciliation: redirect and line-boundary program evidence' at 22:08:02

- starting the work on 'final independent reconciliation and area verification' at 22:08:43
        - Inspected candidate collection, optional-shape prompting, exact/session-cache stopping conditions, persistence guards, lifecycle repeated-resolution evidence, and task mode settlement. Added a narrow serial follow-up for supported leading redirects and native-shell LF/CRLF boundaries; its public regressions now pass, including combinations and fixed-program controls.
        - Corrected the graph candidate field documentation: the collection now retains commands with placeholders even when no template shape exists. No execution grammar was added.
        - Final affected areas are Claudine and Darkmatter. Full-area L1, lint, and tier coverage run sequentially; L1 covers passive discovery, scripted prompt decisions, persistence, and existing native execution/refusal controls. No higher tier resource is required.
        - Cross-platform consideration: production edits are portable metadata/decision logic with no new OS branch, path comparison, or process dispatch. Tests use fixture-owned contexts and policy files. This cycle verifies macOS locally; remote Linux, native Windows and WSL evidence is not claimed.

- Final `cd claudine && NEXTEST_TEST_THREADS=4 just test`: **9,659 passed**, nine existing skips, **341.441 seconds**; all 98 new public approval regressions selected. No test exceeded five seconds. Area lint started after completion.

- Final `cd claudine && just lint`: **passed**, including transport/lifecycle-document guards and Clippy for all area crates. `just check-tier-coverage claudine`: **passed**, zero stranded tests.

- Final `cd darkmatter && NEXTEST_TEST_THREADS=4 just test`: **10,114 passed**, ten existing skips, **108.559 seconds**. New authored-program classification and existing policy, discovery, persistence and execution controls all selected; no test exceeded five seconds. Area lint started after completion.

- Final `cd darkmatter && just lint`: **passed**, including Clippy for the library, CLI, language server and Zed CLI, plus the Zed extension `wasm32-wasip2` compile check. `just check-tier-coverage darkmatter`: **passed**, zero stranded tests.
- Independently reconciled **171 unique ledger IDs**: 122 fixed, 47 clean, two nonblocking observations deferred. No pending or open required row. Input robustness matrix is not applicable: this repair changes authored shell text and approval metadata, not a configuration/file-format reader.
- work completed for 'final independent reconciliation and area verification' at 22:21:52

### Completion Report

- **Outcome: complete**. Finished at 2026-10-14T22:21:52-07:00; elapsed **0 h 28 min 15 s** from the requested start timestamp. Implementation complete, ready for review.
- Parent findings: **total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0**. Instance rows and observations are excluded from parent counts.
- **R3-F1 fixed:** authored executable-placeholder evidence survives invalid template grammar, ordinary first-task/group candidate collection, standalone/referenced lifecycle preparation and repeated resolution. Restricted prompts hide prefix approval and reject unoffered prefix/shape decisions without policy grants. Exact, once, deny and exact-blacklist remain usable; fixed-program argument placeholders retain prefix persistence.
- Verified the entire recurrence scope: primary/setup/teardown, first groups, all six standalone/referenced lifecycle stacks, both shell forms and both chain program positions. Existing later/late, directive/block/frontmatter and shell-valued set refusals remain covered. Additional redirect/LF/CRLF combinations and positive fixed-program controls pass. Candidate settlement, policy/cache stopping conditions and downstream byte approval are reconciled against independently expected grants and refusals.
- Remaining required finding/child IDs: **none**. Nonblocking observations remain **deferred**:
        - **R3-O1:** expanding executable interpolation in document directives, blocks, frontmatter and whole-value set is separate work. Existing rejection is verified; next action is a separately authorized behavior change if wanted.
        - **R3-O2:** nested containers and terminal group events have no current execution surface. Next action is the separately scoped container implementation and its verification.
- Both affected package areas passed final local macOS L1, lint and tier coverage. **19,773 tests passed** across both area runs; 19 existing skips, no test above five seconds. No required check failed. The existing macOS linker warning is informational. Remote Linux, native Windows and WSL runs were not performed or claimed; no OS-specific production branch or dispatch behavior changed.
- No required performance measurement was deferred; the specification requires no performance spike. No formatter, commit, dependency or CI-scope change was made. Existing staged and working-tree implementation was preserved.
- Completion metadata sets the requested log link, `implemented: true`, `implemented_by: codex/gpt-6.1-sol`, and `implementation_3` timestamp. The reviewer's readiness judgment, findings and body are preserved.

| Final verification command | Result | Findings verified |
| --- | --- | --- |
| `cd claudine && NEXTEST_TEST_THREADS=4 just test` | 9,659 passed; nine existing skips; 341.441 seconds | R3-F1, all children and native refusal/execution controls |
| `cd claudine && just lint` | Passed: transport/doc guards and area Clippy | R3-F1 and affected consumers |
| `just check-tier-coverage claudine` | Passed; zero stranded tests | Declared targets and live tier selection |
| `cd darkmatter && NEXTEST_TEST_THREADS=4 just test` | 10,114 passed; ten existing skips; 108.559 seconds | R3-F1 shared choices, policy, discovery, persistence and execution |
| `cd darkmatter && just lint` | Passed; includes Zed WASM compile check | R3-F1 shared API and area consumers |
| `just check-tier-coverage darkmatter` | Passed; zero stranded tests | Declared targets and live tier selection |
| `git diff --check` | Passed | Combined working tree |

### Changed Files

| File changed during this cycle | Finding or purpose |
| --- | --- |
| `.claude/skills/claudine/SKILL.md` | R3-F1 approval metadata guidance |
| `.claude/skills/darkmatter/compose.md` | R3-F1 shared candidate guidance |
| `claudine/docs/topics/pre-flight-checks.md` | R3-F1 public choice restriction |
| `claudine/lib/src/composition/lifecycle/action_shape.rs` | Initialize retained authored metadata |
| `claudine/lib/src/composition/lifecycle/actions.rs` | Carry authored lifecycle command |
| `claudine/lib/src/composition/lifecycle/shell_template.rs` | Carry authored command into candidates |
| `claudine/lib/src/composition/preflight.rs` | Retain and collect invalid-template lifecycle candidates |
| `claudine/lib/src/composition/sequence/preflight/mod.rs` | Retain invalid-template task candidates and correct field documentation |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | Scripted adapter/persistence, repeated-resolution and boundary regressions |
| `claudine/lib/src/harness/shell.rs` | Restricted byte-site prompting and unoffered-decision rejection |
| `darkmatter/docs/inline/preflight-checks.md` | Current public authored-program choice rule |
| `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs` | Supply retained authored candidate field |
| `darkmatter/lib/src/markdown/compose/shell_expansion/site.rs` | Supply retained authored candidate field |
| `darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs` | Shared authored-program eligibility projection |
| `darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs` | Independently expected variable/fixed program choice controls |
| `claudine/features/2026-10-23-shell-template-approval/log.md` | Inventory, progress, evidence and completion metadata |
| `claudine/features/2026-10-23-shell-template-approval/review-3.md` | Requested completion metadata only |

- Completion metadata verification: `md get` read back review `log`, `implemented`, and `implemented_by`, and log `implementation_3` successfully; no duplicate frontmatter keys or parse errors. Final `git diff --check` passed after the completion records. All started commands and both serial subagents finished.
