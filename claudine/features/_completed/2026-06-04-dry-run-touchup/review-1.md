---
ready: false
agent: codex
model: ""
---

# Review: 2026-06-04 dry-run touchup

## Verdict

Not ready for production.

The implementation makes good progress on the shared `AgentResolutionState` model and adds L1/L2 coverage for several dry-run rendering states. However, there are still spec-level gaps in the end-to-end dry-run state prediction, list classification, live no-TTY messaging, and live picker verification.

## Findings

### High: dry-run auto-select list states render as a plain selected provider

Spec requirement: a frontmatter `agent` list with exactly one valid and installed agent must render the auto-select header and explanatory line in the dry-run `Agent` cell.

Implementation issue: `eagerly_resolve_target(..., dry_run = true)` returns `Some(ResolvedExecutionTarget)` for `AgentResolutionState::ListOneInstalled` at `claudine/cli/src/commands/wrap/composition/mod.rs:480`. Later, `DryRunRender::from_request` treats any `resolved_target` as `AgentResolutionState::Selected` at `claudine/cli/src/commands/wrap/composition/dry_run.rs:76`, so the list-specific state is lost before the table renders.

Impact: the dry-run table says only `Claude`/`Codex`/etc. instead of the required:

- `✓ the <agent> will be used without the need for interactive prompting`
- `the Markdown document suggested multiple agents but only <agent> is installed on this host`
- invalid suggestions, when present

Test level: the direct renderer unit test for `ListOneInstalled` is L1, but the end-to-end dry-run request path is not covered. The strongest relevant test is at the wrong surface: it does not verify the user-observable CLI behavior.

### High: one-item all-invalid agent lists are misclassified as single-invalid

Spec requirement: any list-valued `agent` hint resolving to zero installed agents, including all-invalid lists, must use the dedicated zero-installed-list state.

Implementation issue: `ParsedAgentHint` tracks `is_list` internally, but `EffectiveSelectionHints` stores only `agent: Option<AgentHint>` and `agent_invalid: Vec<String>`. When a list has no valid providers, `to_agent_hint()` returns `None` at `claudine/lib/src/composition/prepare.rs:300`, losing whether the original value was a list. `classify_agent_resolution` then treats `agent: None` plus one invalid value as `SingleInvalid` at `claudine/lib/src/composition/select.rs:60`.

Impact: `agent: ["not-real"]` renders and aborts as a single invalid agent, not as the zero-installed-list state specified for list-valued hints. The existing classifier test covers two invalid list entries, but misses the one-item list edge case.

Test level: missing L1 for single-entry all-invalid lists in both dry-run rendering and live resolution.

### High: no-TTY live abort messages do not consistently match the TTY/dry-run messages

Spec requirement: for every prompting state, no-TTY execution must emit the same styled message the TTY path would show to stderr, then abort with a structured non-zero exit.

Implementation issue: the no-TTY renderer in `render_agent_resolution_failed_body` uses different copy for several states. For example, `ListMultipleInstalled` says “Multiple suggested agents are installed; the interactive picker would ask...” at `claudine/lib/src/composition/error.rs:1001`, while the dry-run/TTY message says “caller will be asked to choose interactively between suggested Agents”. The zero-installed-list body also changes the message and adds “the current session is not interactive” at `claudine/lib/src/composition/error.rs:1038`, while the TTY message is built separately at `claudine/cli/src/commands/wrap/composition/mod.rs:622`.

Impact: the dry-run table is not a faithful prediction of what the live no-TTY path emits, which is a central scope requirement of the spec.

Test level: the L1 no-TTY tests mostly downcast to `CompositionError::AgentResolutionFailed` and assert the state. They do not assert stderr content for each prompting state, nor that `--silent` leaves the styled agent-resolution report intact.

### High: live TTY picker behavior is not verified at the required L1 surface

Spec requirement: live TTY behavior must be verified for no-agent, invalid, not-installed, multi-installed list scoped to suggested installed agents, and zero-installed-list scoped to all installed agents.

Implementation status: the code routes TTY states through `prompt_for_agent_state` and `build_scoped_picker_plan`, but the added tests at `claudine/cli/src/commands/wrap/composition/mod.rs:2743` and following cover no-TTY aborts and auto-select states only. There is no L1 test proving the TTY arm calls the picker with the right scope, emits the required pre-prompt messages, or keeps `--silent` from changing agent resolution/reporting.

Impact: user-observable key behavior in the live path is not production-ready under the review's test rigor rules. This is not an L2/L3 issue; these can be L1 tests with an injected picker/planner seam or a pure planner helper.

## Notes

The L2 additions cover red/yellow/dim styling and some structural formatting through tmux. That is useful, but the spec also requires `hr` rendering asserted in a real terminal. The current L2 test comments say tmux cannot assert the horizontal rule and falls back to L1. If no alternate real-terminal backend is used for that requirement, this remains an L2 acceptance gap.

## Verification Performed

Ran:

```text
cargo test --color=never -p claudine-cli agent_list_one_installed_renders_auto_select_header
```

Result: passed. This only verifies the direct renderer unit test, not the end-to-end dry-run CLI path.
