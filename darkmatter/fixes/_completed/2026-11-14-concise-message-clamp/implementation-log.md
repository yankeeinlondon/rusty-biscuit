---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/fixes/2026-11-14-concise-message-clamp/spec.md"
plan: "darkmatter/fixes/2026-11-14-concise-message-clamp/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - "darkmatter/lib/src/lifecycle/envelope.rs"
  - "darkmatter/lib/src/lifecycle/envelope/tests.rs"
  - "darkmatter/lib/src/lifecycle/context.rs"
  - "darkmatter/lib/src/lifecycle/context/tests.rs"
  - "darkmatter/lib/src/lifecycle/cancellation.rs"
  - "darkmatter/lib/src/lifecycle/md_host.rs"
  - "darkmatter/lib/src/lifecycle/md_host/tests.rs"
  - "darkmatter/lib/src/lifecycle/executor/tests.rs"
  - "darkmatter/lib/src/lifecycle/preflight/tests.rs"
  - "darkmatter/lib/src/lifecycle/run_guard.rs"
  - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
  - "darkmatter/lib/tests/l1/global_catalog.rs"
  - "darkmatter/lib/tests/l1/lifecycle_run_protocol.rs"
  - "darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs"
  - "darkmatter/cli/src/commands/compose/lifecycle.rs"
  - "darkmatter/cli/tests/l1/compose_lifecycle_refusals.rs"
  - "claudine/cli/src/commands/wrap/composition/pipeline.rs"
  - "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
  - "claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs"
  - "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
  - "claudine/cli/src/commands/wrap/sequence/container.rs"
  - "claudine/cli/src/commands/wrap/sequence/task_run.rs"
  - "claudine/lib/src/composition/error/mod.rs"
  - "claudine/lib/src/composition/lifecycle/context/tests.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/binding_contract.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/recovery.rs"
  - "claudine/lib/src/composition/lifecycle/report.rs"
  - "claudine/lib/src/composition/looping/engine.rs"
  - "claudine/lib/src/diagnostics/restored.rs"
  - "claudine/lib/src/diagnostics/restored/tests.rs"
  - "claudine/lib/src/diagnostics/snapshot.rs"
  - "claudine/lib/src/diagnostics/snapshot/tests.rs"
  - "claudine/lib/src/reporting/types.rs"
docs_updated_during_phase_1:
  - "darkmatter/docs/topics/lifecycle/error-handling.md"
  - "darkmatter/docs/topics/lifecycle/index.md"
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - ".claude/skills/darkmatter/lifecycle.md"
  - ".claude/skills/claudine/SKILL.md"
packages:
  - "darkmatter"
  - "darkmatter-cli"
  - "claudine"
  - "claudine-cli"
decisions_for_review:
  - |-
    Removed the "second, unclamped copy" of each error message that a recent commit
    added (DiagnosticSnapshot.full_message, LifecycleErrorInfo.full_msg, the
    actionable_msg() accessor, and the helpers concise_and_full and
    single_line_message). The plan did not mention them because it was written
    before they existed; the spec says no second full_message field is needed.
    Once nothing is cut, the copy always equals the normal message, so keeping
    it would leave a field that is always empty and an accessor that just
    returns msg. Alternative: keep them as always-empty compatibility shims,
    which avoids touching about a dozen Claudine call sites now but leaves dead
    API behind. Reversing this means restoring those fields and the accessor in
    darkmatter/lib/src/lifecycle/{envelope,context}.rs and re-pointing the
    Claudine callers (listed in source_files_during_phase_1) back to it.
  - |-
    Made the Claudine-side edits that this removal forces (call sites, test
    fixtures, the restored-diagnostic display, one test) in Phase 1, although
    the plan gives Claudine to Phase 2. They are mechanical consequences of the
    Darkmatter change, so Phase 2 only has to do the work its own plan lists.
    Alternative: leave them to Phase 2, whose agent would then meet compile
    errors the plan does not explain.
  - |-
    The contract audit found a stale comment in a Claudine CLI test
    (lifecycle_mutation_root.rs) saying the status line is "length-clamped",
    so only the root's label could be checked. I rewrote the comment and
    made the test stricter: it now also checks that the mutation root's last
    folder name, followed by ")", appears and that no ellipsis does. I did
    not assert the whole root path, because how Windows spells that path
    (short names, prefixes) may differ from what the test can compute, and
    the Windows host could not run the test (full disk). Alternative: assert
    the full path, which is stronger on macOS and Linux but may fail on
    Windows. To change this, edit that one assertion.
source_files_during_phase_2:
  - "claudine/lib/src/harness/runtime.rs"
  - "claudine/lib/src/harness/runtime/tests.rs"
  - "claudine/lib/src/diagnostics/snapshot.rs"
  - "claudine/lib/src/diagnostics/snapshot/tests.rs"
  - "claudine/lib/src/stream/task_ledger.rs"
  - "claudine/lib/src/stream/task_ledger/tests.rs"
  - "claudine/lib/src/stream/logs/opencode/classify/mod.rs"
  - "claudine/lib/src/stream/logs/opencode/classify/asset.rs"
  - "claudine/lib/src/stream/logs/opencode/classify/tests.rs"
  - "claudine/lib/src/composition/lifecycle/report.rs"
  - "claudine/lib/src/composition/lifecycle/context/tests.rs"
  - "claudine/lib/src/render/incomplete_subagents.rs"
  - "claudine/cli/tests/l1/lifecycle_refusal_reports.rs"
  - "claudine/cli/tests/l1/wrap_incomplete_subagents.rs"
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - ".claude/skills/claudine/SKILL.md"
source_files_during_phase_3:
  - "claudine/lib/src/diagnostics/snapshot/tests.rs"
  - "claudine/lib/src/composition/lifecycle/tests/audio_emission.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs"
  - "claudine/cli/tests/l1/characterization_error_routes.rs"
  - "claudine/cli/tests/common/incomplete_subagents.rs"
  - "claudine/cli/tests/level2/level2_incomplete_subagents_capture.rs"
  - "darkmatter/cli/tests/l1/compose_lifecycle.rs"
  - "darkmatter/cli/tests/l1/compose_data_output.rs"
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - "claudine/cli/tests/l1/lifecycle_mutation_root.rs"
  - "claudine/lib/src/composition/lifecycle/context.rs"
  - "claudine/lib/src/composition/sequence/task/mod.rs"
docs_updated_during_phase_4:
  - "darkmatter/docs/topics/lifecycle/error-handling.md"
  - "claudine/docs/topics/flow-control/lifecycle.md"
  - "claudine/docs/topics/non-interactive-sessions.md"
  - "claudine/docs/topics/error-architecture.md"
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - "claudine/docs/use-skills/use-claudine/lifecycle.md"
source_code:
  - "darkmatter/lib/src/lifecycle/envelope.rs"
  - "darkmatter/lib/src/lifecycle/envelope/tests.rs"
  - "darkmatter/lib/src/lifecycle/context.rs"
  - "darkmatter/lib/src/lifecycle/context/tests.rs"
  - "darkmatter/lib/src/lifecycle/cancellation.rs"
  - "darkmatter/lib/src/lifecycle/md_host.rs"
  - "darkmatter/lib/src/lifecycle/md_host/tests.rs"
  - "darkmatter/lib/src/lifecycle/executor/tests.rs"
  - "darkmatter/lib/src/lifecycle/preflight/tests.rs"
  - "darkmatter/lib/src/lifecycle/run_guard.rs"
  - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
  - "darkmatter/lib/tests/l1/global_catalog.rs"
  - "darkmatter/lib/tests/l1/lifecycle_run_protocol.rs"
  - "darkmatter/lib/tests/l1/lifecycle_boundary/replay.rs"
  - "darkmatter/cli/src/commands/compose/lifecycle.rs"
  - "darkmatter/cli/tests/l1/compose_lifecycle_refusals.rs"
  - "claudine/cli/src/commands/wrap/composition/pipeline.rs"
  - "claudine/cli/src/commands/wrap/harness_orch/loop_control.rs"
  - "claudine/cli/src/commands/wrap/harness_orch/loop_control/error_routing.rs"
  - "claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs"
  - "claudine/cli/src/commands/wrap/sequence/container.rs"
  - "claudine/cli/src/commands/wrap/sequence/task_run.rs"
  - "claudine/lib/src/composition/error/mod.rs"
  - "claudine/lib/src/composition/lifecycle/context/tests.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/binding_contract.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/conditions_control.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs"
  - "claudine/lib/src/composition/lifecycle/executor/tests/recovery.rs"
  - "claudine/lib/src/composition/lifecycle/report.rs"
  - "claudine/lib/src/composition/looping/engine.rs"
  - "claudine/lib/src/diagnostics/restored.rs"
  - "claudine/lib/src/diagnostics/restored/tests.rs"
  - "claudine/lib/src/diagnostics/snapshot.rs"
  - "claudine/lib/src/diagnostics/snapshot/tests.rs"
  - "claudine/lib/src/reporting/types.rs"
  - "claudine/lib/src/harness/runtime.rs"
  - "claudine/lib/src/harness/runtime/tests.rs"
  - "claudine/lib/src/stream/task_ledger.rs"
  - "claudine/lib/src/stream/task_ledger/tests.rs"
  - "claudine/lib/src/stream/logs/opencode/classify/mod.rs"
  - "claudine/lib/src/stream/logs/opencode/classify/asset.rs"
  - "claudine/lib/src/stream/logs/opencode/classify/tests.rs"
  - "claudine/lib/src/render/incomplete_subagents.rs"
  - "claudine/cli/tests/l1/lifecycle_refusal_reports.rs"
  - "claudine/cli/tests/l1/wrap_incomplete_subagents.rs"
  - "claudine/lib/src/composition/lifecycle/tests/audio_emission.rs"
  - "claudine/cli/tests/l1/characterization_error_routes.rs"
  - "claudine/cli/tests/common/incomplete_subagents.rs"
  - "claudine/cli/tests/level2/level2_incomplete_subagents_capture.rs"
  - "darkmatter/cli/tests/l1/compose_lifecycle.rs"
  - "darkmatter/cli/tests/l1/compose_data_output.rs"
  - "claudine/cli/tests/l1/lifecycle_mutation_root.rs"
  - "claudine/lib/src/composition/lifecycle/context.rs"
  - "claudine/lib/src/composition/sequence/task/mod.rs"
documentation:
  - "darkmatter/docs/topics/lifecycle/error-handling.md"
  - "darkmatter/docs/topics/lifecycle/index.md"
  - ".claude/skills/darkmatter/lifecycle.md"
  - ".claude/skills/claudine/SKILL.md"
  - "claudine/docs/topics/flow-control/lifecycle.md"
  - "claudine/docs/topics/non-interactive-sessions.md"
  - "claudine/docs/topics/error-architecture.md"
  - "claudine/docs/use-skills/use-claudine/lifecycle.md"
completed_phase: 4
implemented: true
---
---

# Implementation Log for 2026-11-14-concise-message-clamp (4 phases)

## Phase 1

### What changed

- `darkmatter/lib/src/lifecycle/envelope.rs`: one private `normalize` (strip
  escapes with `biscuit-terminal`'s `strip_ansi_codes`, then fold to one line)
  backs both `concise_message` and `concise_message_with_suffix`; the suffix
  is normalized together with the base. The fold now also splits on U+2028 and
  U+2029. `CONCISE_MESSAGE_MAX_CHARS` and `clamp_chars` are deleted. Field and
  function docs say "complete, single-line, escape-free; no length limit".
- **Departure from the plan:** the plan predates commit `0d307aef2`, which
  added a second, unclamped copy of the message beside the clamped one
  (`DiagnosticSnapshot::full_message`, `LifecycleErrorInfo::full_msg`,
  `LifecycleErrorInfo::actionable_msg()`, `concise_and_full`,
  `single_line_message`). With no clamp, that copy is always identical to
  `msg`, and the spec says no second `full_message` field is needed. All five
  are removed; every reader of `actionable_msg()` now reads `msg` (darkmatter:
  `report_diagnostics.rs`, `run_guard.rs`, `cli/src/commands/compose/lifecycle.rs`;
  claudine: about a dozen call sites, mechanical). `cancellation.rs` no longer
  maintains `full_msg`. See `decisions_for_review`.
- Claudine was touched only for that ripple: removing `full_msg: None` /
  `full_message: None` fields from test fixtures, `actionable_msg()` →
  `msg`, `diagnostics/snapshot.rs` back to `concise_message(...)` (the
  pre-`0d307aef2` form), `diagnostics/restored.rs` displays
  `snapshot.message`, and the claudine test
  `a_long_edit_refusal_keeps_its_complete_text_in_err_msg` (renamed) now
  asserts `err.msg` and the serialized snapshot carry the complete text.
  After Phase 1 claudine fails to compile only at
  `claudine/lib/src/harness/runtime.rs:101` (`FAILURE_MESSAGE_MAX_CHARS`),
  which Phase 2 removes, as the plan expects.
- Comments: `md_host.rs` edit-failure comment no longer says the message is
  clamped (reason-first order kept because it reads better);
  `executor/tests.rs` helper comment corrected; `compose_lifecycle_refusals.rs`
  module docs talk about the "old" 240-character limit.
- Docs updated because the removed mechanism was described there:
  `darkmatter/docs/topics/lifecycle/error-handling.md` (the `edit` failure
  paragraph and the `msg` row of Fields), `darkmatter/docs/topics/lifecycle/index.md`
  (mutation-root refusal paragraph), `.claude/skills/darkmatter/lifecycle.md`,
  `.claude/skills/claudine/SKILL.md` (`actionable_text`).

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| no ellipsis, suffix intact on a 600-char base | `lifecycle::envelope::tests::a_long_base_and_suffix_stay_intact` |
| lengths 239/240/241 and a 2,000-char ASCII+CJK+emoji message unchanged | `...::lengths_around_the_old_limit_are_unchanged` |
| hygiene: SGR, OSC-8 with BEL, LF, CRLF, tab, stray BEL, U+2028, U+2029, U+0085, empty, blank, interior double spaces | `...::hygiene_table` |
| suffix gets hygiene too | `...::the_suffix_is_normalized_too` |
| idempotent across the whole table | `...::concise_message_is_idempotent` |
| `err.msg` of a real long containment refusal (effect, frontmatter, edit) is complete, projected unchanged in `err.*`, and is the `md compose` diagnostic text | `lifecycle::md_host::tests::a_long_root_*` (rewritten `assert_full_refusal`) |
| end-to-end through `md compose` (terminal and JSON) keeps whole target and root | `darkmatter-cli` `compose_lifecycle_refusals` (unchanged assertions) |
| action-failure normalization | `lifecycle::context::tests::an_action_failure_message_is_normalized` (renamed) |

The input-robustness matrix does not apply: no file format or configuration
reader changed.

### Gates

- `cd darkmatter && just lint`: clean.
- `cd darkmatter && just test`: 17042 passed, 10 skipped; "none at or above 5.0s".
  The only warning is the macOS linker `__eh_frame` notice, which predates
  this change.
- I did not run the scope-skip check. Source files changed, so both gates
  were required.
- Dependent packages: only `claudine`/`claudine-cli` use the changed API. As
  the plan expects, they do not compile until Phase 2 removes
  `FAILURE_MESSAGE_MAX_CHARS` (`claudine/lib/src/harness/runtime.rs:101`).
  `cargo check -p claudine -p claudine-cli --tests` reports only that one error,
  so the ripple edits made here compile. Claudine tests are left to Phase 2.
- I ran no cross-OS check. The change is pure string handling with no paths,
  processes or OS APIs, so it carries no OS-specific risk.
- I ran no L2 or L3 tests; the spec requires no live or focus-changing test.

## Phase 2

### What changed

- `claudine/lib/src/harness/runtime.rs`: the local `concise_message`,
  `sanitize_message`, `LinePick`, and `headline` are gone. `concise_message`
  is now `pub use darkmatter::lifecycle::envelope::concise_message`, so
  `harness/mod.rs` and `diagnostics/snapshot.rs` compile unchanged.
  `failure_message` calls `concise_message_with_suffix` directly. An explicit
  `error_message` is normalized whole (every line kept, R11); one that
  normalizes to empty falls through to guard, timeout, stderr, and exit-code
  sources in the existing order. Stderr uses `last_useful_line` (R12). The
  test-only `FAILURE_MESSAGE_MAX_CHARS` and its "headline, not the archive"
  doc are deleted; the `failure_message` doc now says the message is the
  complete selected explanation and the stderr log is not appended.
- `diagnostics/snapshot.rs`: `from_code` stores `concise_message(&message)`,
  and its trait doc says so.
- `stream/task_ledger.rs`: `PRIOR_FAILURE_CLAUSE_MAX_CHARS` and `clamp_chars`
  are deleted. `prior_failure_clause` normalizes kind and message before the
  blank check and returns the complete clause. `compose_message` and
  `apply_to_summary` are unchanged; docs updated.
- OpenCode `classify/mod.rs::error_headline` (300-character cap) and
  `classify/asset.rs::summarize_error_json` (500-byte raw-tag cap) return the
  whole value; the stack/`cause=` cut stays. Only R15's
  `bridge/format.rs` preview survives the `take(` audit in `opencode/`.
- Phase 1 handoff: `report.rs::actionable_text` inlined (both reporters read
  `error.msg`), `.claude/skills/claudine/SKILL.md` updated to match, and the
  `lifecycle_refusal_reports.rs` comments reworded.
- **Work pulled forward from Phase 3**, because Phase 2 made it red or wrong:
  `cli/tests/l1/wrap_incomplete_subagents.rs::display_truncation_never_reaches_the_machine_record`
  asserted the headline was ≤ 240 characters and had truncated, which fails once
  the ledger stops clamping. It is rewritten as Phase 3 specifies
  (`the_headline_and_the_machine_record_both_name_every_task`, with the
  `ManyLongNamedStops` doc). In `composition/lifecycle/context/tests.rs`, two
  `<= 240` assertions became red once Claudine compiled against Phase 1:
  `err_msg_is_hygiened_for_a_hostile_action_failure` now asserts the exact
  complete text, and `every_err_msg_producer_is_single_line_escape_free_and_capped`
  is renamed `every_err_msg_producer_is_single_line_and_escape_free` without the
  cap. The "budget-reserved" comment Phase 3 lists there is reworded. The doc on
  `render/incomplete_subagents.rs` said the headline is clamped to 240; it now
  describes the two views without that claim.

### Requirement-to-test mapping

| Requirement | Test (claudine unless noted) |
|---|---|
| explicit error keeps every line, escapes stripped (original failing input) | `harness::runtime::tests::an_explicit_error_keeps_every_line` |
| escape-only error falls through to stderr | `...::an_escape_only_error_falls_through_to_stderr` |
| blank error falls through to exit-code message, suffix at attempt 2 | `...::a_blank_error_falls_through_to_the_exit_code_message` |
| guard pattern kept whole, suffix intact | `...::a_long_guard_pattern_is_kept_whole` |
| long message plus `(attempt 2)` suffix, no `…` | `...::a_long_message_keeps_its_whole_text_and_the_suffix` |
| stderr last line kept whole, earlier lines dropped | `...::a_long_last_stderr_line_is_kept_whole` |
| cascade precedence and suffix on every source | `...::cascade_and_suffix_matrix` (unchanged) |
| 40-task headline names the last task; facts complete | `...::the_incomplete_headline_names_every_task` |
| `from_code` normalizes its message | `diagnostics::snapshot::tests::from_code_normalizes_its_message` |
| typed diagnostic display kept whole | `...::snapshot_message_keeps_an_overlong_display_whole` |
| ledger keeps whole displaced failure and all 30 tasks | `stream::task_ledger::tests::the_composed_message_still_honors_the_concise_message_contract` |
| message-only clause complete, tasks in declaration order | `...::a_long_message_only_failure_keeps_the_clause_and_every_task` |
| `incomplete_message` names all 40; equals the summary's message | `...::the_headline_names_tasks_while_the_machine_list_stays_complete` |
| escape-only displaced message leaves no dangling clause | `...::an_escape_only_displaced_message_leaves_no_dangling_clause` |
| OpenCode non-JSON tag kept whole | `stream::logs::opencode::classify::tests::non_json_error_tag_is_kept_whole` |
| OpenCode headline kept whole (400 emoji) | `...::error_headline_keeps_a_long_multibyte_payload_whole` |
| end to end through `claudine compose`: `err.msg`, JSONL facts, and terminal all name 12 long tasks in order | `claudine-cli` `wrap_incomplete_subagents::the_headline_and_the_machine_record_both_name_every_task` |

The input-robustness matrix does not apply: no file format or configuration
reader changed. The OpenCode classifiers read log lines, but only their output
length changed, not how any field is parsed.

### Gates

- `cd claudine && just lint`: clean (only the pre-existing macOS linker
  `__eh_frame` notice).
- `cd claudine && just test`: 12243 passed, 15 skipped, none at or above 5.0s.
- `cd darkmatter && just lint`: clean. `cd darkmatter && just test`: 17042
  passed, none at or above 5.0s. No darkmatter source changed in this phase.
- Dependents: only `claudine-cli` depends on `claudine`, and it is in the area
  run above. I did not run the scope-skip check because source files changed.
- I ran no cross-OS check. The changes are pure string handling with no
  paths, processes, or OS APIs.
- I ran no L2 or L3 tests; the spec requires no live or focus-changing test.
- The working tree also shows the author's moves of
  `2026-11-14-lifecycle-mutation-root` into `_completed` and a deleted Claudine
  spec. They are not part of this phase and I left them untouched.

## Phase 3

All Phase 3 work is test-only, plus three comment edits. No production code
changed. I did it in one session without subagents, because the four task
groups were small and serial runs kept host load down.

- **Projection parity** (`claudine/lib/src/diagnostics/snapshot/tests.rs`):
  `every_projection_route_keeps_the_complete_normalized_message` runs one
  ANSI-colored, multi-line, tabbed message of more than 241 characters, each
  with a unique last word, through four routes:
  - typed selection via `DiagnosticSnapshot::select` over a semantic
    `IngestError` that wraps a registered `ClaudineError::ConfigValidation`
    cause;
  - `from_code("cap.rate_limit", …)` and the matching `from_action_failure("rate_limit", …)`
    label route;
  - a facet-less `from_action_failure("shell", …)`;
  - a catalog miss through `from_error_or_action` with an unregistered prose
    error.

  It asserts the exact normalized text, `snapshot.cause.message` complete,
  `info.msg == snapshot.message`, and a `to_value`/`from_value` round trip
  of the message and cause.
- **md compose command paths**: `compose_lifecycle::a_long_composed_error_reaches_the_diagnostic_and_err_msg_whole`
  sends a 600-character `error:` ending in `TAIL-MARKER`. The finalize line
  holds the whole text, and a separate stderr diagnostic line holds the tail.
  `compose_data_output::a_lifecycle_run_writes_only_the_envelope_to_stdout`
  gained a long multi-line YAML block-scalar leg: stdout is exactly one JSON
  document, `diagnostics[0].message` equals the lines joined with spaces, and
  stderr is empty.
- **Claudine command paths**: Phase 2 had already rewritten
  `wrap_incomplete_subagents.rs`. I added
  `characterization_error_routes::an_initialize_error_reaches_finalize_err_msg_whole`:
  a 503-character `initialize` `error:`, with finalize appending
  `{{ err.msg }}` to `events.log`. It asserts that the single recorded line
  equals the full reason. I also updated the module doc's `err.msg` bullet
  ("concise message projection" is gone; it names this test as the one
  length property pinned). R17's comment edits are in
  `tests/common/incomplete_subagents.rs` and
  `level2/level2_incomplete_subagents_capture.rs`; the L2 test was not run.
- **Delivery isolation**: `audio_emission::executor_publishes_a_long_speech_text_whole`
  (a 1,006-character `say` reaches the recording emitter unchanged). In
  `event_time_interpolation.rs`, the `<= 240` assertion and its "~240-char
  cap" comment became a retention assertion (`msg` still names
  `no_such_function`). The sibling test
  `a_long_err_msg_reaches_speech_whole_and_unchanged` runs a 615-character
  `err.msg` through `say` and then `message`: both receive the full text, and
  `err` equals its clone after the event. The `context/tests.rs`
  "budget-reserved" comment had already been reworded in Phase 2.
- R16: Messenger was not touched or tested.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| snapshot, cause, and `err.msg` carry complete normalized text on every projection route; JSON round trip | `claudine` `diagnostics::snapshot::tests::every_projection_route_keeps_the_complete_normalized_message` |
| `md compose` terminal diagnostic and finalize `err.msg` keep a 600-character error | `darkmatter-cli` `compose_lifecycle::a_long_composed_error_reaches_the_diagnostic_and_err_msg_whole` |
| `md compose` data envelope carries the full folded multi-line message; one JSON doc; empty stderr | `darkmatter-cli` `compose_data_output::a_lifecycle_run_writes_only_the_envelope_to_stdout` |
| `claudine compose` initialize error reaches finalize `err.msg` whole | `claudine-cli` `characterization_error_routes::an_initialize_error_reaches_finalize_err_msg_whole` |
| incomplete-task headline and machine record name every task | `claudine-cli` `wrap_incomplete_subagents::the_headline_and_the_machine_record_both_name_every_task` (Phase 2) |
| speech emitter receives long text unshortened | `claudine` `composition::lifecycle::tests::audio_emission::executor_publishes_a_long_speech_text_whole` |
| delivery does not shorten or mutate `err.msg` | `claudine` `...::event_time_interpolation::a_long_err_msg_reaches_speech_whole_and_unchanged` |

The tests cross the CLI boundary: the `md` and `claudine` binaries are spawned
with filesystem fixtures, and stdout/stderr and an events file are asserted
(L1, F3). They also cover the in-process crate API (F1/F2). No live speech,
notification, or focus-changing backend is used. The input-robustness matrix
does not apply, because no reader of a file format or configuration changed.

### Gates

- Targeted: the seven tests above pass. I measured one fixture wrong
  (596 < 600 characters), so the first run failed its own precondition
  assertion; I fixed the repeat count.
- `cd darkmatter && just test`: 17043 passed, 10 skipped, 0 at or above 5.0s.
  `just lint`: exit 0.
- `cd claudine && just test`: 12247 passed, 15 skipped, none at or above 5.0s.
  `just lint`: exit 0.
- Dependents: `claudine` and `claudine-cli` are the dependents of Darkmatter,
  and both ran in the area suite above.
- I ran no L2/L3 tests, as the spec requires. I ran no cross-OS check: the
  changes are test-only string assertions over existing fixtures, with no
  paths or OS APIs.

## Phase 4

Docs, skills and the contract audit. No production behavior changed; the
source edits are doc comments and one test assertion.

### What changed

- **Darkmatter docs.** Phase 1 had already rewritten the `edit` failure
  paragraph, the `msg` field row in `error-handling.md`, and the repo skill
  `.claude/skills/darkmatter/lifecycle.md`, so only the missing piece was
  added: a `failed:` example with fixed notification text, and the sentence
  that submitting the full message does not guarantee a remote service shows
  all of it. A grep of `darkmatter/lib/src` and `darkmatter/cli/src` for
  "cut", "reason first" and "clamp" found no remaining message-length wording
  (the `clamp` hits are unrelated numeric clamps).
- **Claudine docs.** `flow-control/lifecycle.md`: the `err.msg` table row, the
  `#### err.msg` section (rewritten: complete, one line, never shortened;
  delivery channels fit it; the example now uses fixed `say` text and
  `err.code`, per the spec), the mutation-root report sentence that still
  said `err.msg` is "cut to its 240-character limit", the cascade wording
  (explicit error kept whole, then guard, timeout, last useful stderr line,
  exit code; this matches `harness::failure_message`), and the planned
  `ClaudineCompletionDelayed` note. `non-interactive-sessions.md`: the
  incomplete-task paragraph no longer mentions a 240-character budget or a
  reservation. `error-architecture.md`: one sentence that the snapshot's
  `message` is complete and normalized.
- **Rust doc comments.** `claudine/lib/src/composition/lifecycle/context.rs`
  (`from_action_failure`: "budget-reserved" removed) and
  `claudine/lib/src/composition/sequence/task/mod.rs` (`message()`: "concise"
  becomes "normalized, unshortened").
- **Caller skill.** `claudine/docs/use-skills/use-claudine/lifecycle.md` has a
  new paragraph and YAML example, placed after the sentence
  "`claudine errors` lists every `err.code`" rather than directly under the
  table, because the paragraph after the table continues it. The prose
  names `err.msg`/`err.code` without braces, and the example sits in a
  fenced block, which composition leaves verbatim, so no triple-brace escape
  was needed. `cd claudine && just publish-skills` ran; the installed
  `~/.claude/skills/use-claudine/lifecycle.md` contains the paragraph and the
  example with `{{ err.code }}` intact.

### Contract audit

- Deleted symbols grep (`CONCISE_MESSAGE_MAX_CHARS|clamp_chars|...|sanitize_message`
  over `*.rs` in `darkmatter` and `claudine`): no matches.
- Wording grep (`notification-safe|budget-reserved|240-char|240 char|length-clamped|~240`):
  the remaining matches are tests and fixture docs that describe the *old* or
  *removed* limit as history or fixture size (including `ManyLongNamedStops`,
  kept as the Phase 3 agent suggested), Kimi research notes about Kimi's own
  240-character description fallback (unrelated), and a `.claude/skills/claudine/timeline.md`
  entry (historical). One live defect: `claudine/cli/tests/l1/lifecycle_mutation_root.rs:141`
  said "The status line is length-clamped". Fixed: comment rewritten and the
  test strengthened (see `decisions_for_review`).
- `take(` / `chars().count() >` in `claudine/lib/src/stream/logs/opencode/`
  and the Darkmatter shell executor: the only length bounds left are R15's
  two (`bridge/format.rs:162` `truncate_for_inline`, and the executor's
  200-character panic preview); the other matches are `Option::take`,
  `mem::take`, and `take(1)` over directive actions.
- Published caller skill: `grep -n "240\|clamp" ~/.claude/skills/use-claudine/*.md`
  finds nothing.

### Requirement-to-test mapping

Phase 4 changes no behavior, so it adds no behavior test. Its documentation
claims are backed by the Phase 1-3 tests listed above (complete `err.msg`,
snapshot message, incomplete-task headline, delivery isolation). The one
test edit, `claudine-cli` `lifecycle_mutation_root::a_relative_write_from_a_prompt_in_another_repo_is_refused_and_creates_nothing`,
now asserts the refusal keeps the root's last component, with no ellipsis
(L1, spawned `claudine` binary, F3). The input-robustness matrix does not
apply: no reader of a file format or configuration changed.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `packages: [claudine, claudine-cli, test-toolkit, darkmatter]`,
  `change_class: package`, so the gates were not skipped. `test-toolkit`
  comes only from its lint-only `archive-path-guard` companion, which scans
  Rust source; `just archive-path-guard` in `tools/test-toolkit`: 2 passed.
- `cd darkmatter && just test`: 17043 passed, 10 skipped, none at or above
  5.0s (load average about 8 to 16). `just lint`: exit 0.
- `cd claudine && just test`: 12247 passed, 15 skipped, none at or above 5.0s.
  `just lint`: exit 0 (the only warnings are the existing macOS linker
  `__eh_frame` notice).
- After that run, the `lifecycle_mutation_root` assertion was narrowed for
  Windows (see the decision). Rerun: the `lifecycle_mutation_root` module
  (13 passed) and `claudine` `just lint` (exit 0).
- `just slow-tests fresh`: exit 0.
- Dependents: Darkmatter's dependents are `claudine` and `claudine-cli`,
  covered above. Phase 4 changed no public symbol, so nothing downstream of
  Claudine needed a run.
- L2 and L3 were not run: the spec forbids live speech, notification, or
  focus-changing tests, and no behavior in this phase needs a real terminal.

### Cross-OS

The edited assertion reads a path, so I tried
`just cross-check claudine-cli --os windows <test name>`. Two findings:

1. With a `-E 'test(...)'` filter the recipe fails with a shell
   `syntax error near unexpected token '('`, both locally and in the
   remote `_expected_manifest` step. The filter's parentheses are not
   quoted when forwarded. A plain-name nextest filter avoids this.
2. With a plain-name filter the Windows host built and archived the test,
   then stopped at Cargo storage preflight: 36.9 GiB free of the 50 GiB
   required; the automatic sweep freed 0 GiB. I did not override the storage
   guard. No Windows result was produced, and the assertion was narrowed to
   an OS-neutral form instead. `os_failing` is not set because no test failed.

Also seen in the Windows build output, and not from this fix: `unused imports: Duration and Instant`
at `claudine/cli/tests/l1/lifecycle_edit_action.rs:23` (Windows only).

### Unrelated working-tree changes

`claudine/cli/src/argv/rule4_help_hoist.rs`, `claudine/cli/tests/l1/argv_normalization.rs`,
`claudine/docs/topics/argv-normalization.md`, `claudine/docs/topics/cli-pre-parsing.md`
and `claudine/fixes/2026-11-14-output-flag-not-implemented/implementation-log.md`
were already modified when this phase began. They belong to other work, were
not touched, and are not listed in this phase's frontmatter. The Claudine
suite above ran with them in the tree.

Implementation complete, ready for review.
