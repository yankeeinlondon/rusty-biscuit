# Spec: Consistent Rendering for Compose and Inline-Compose

## Core Requirements

- All reporting for `compose` and `inline-compose` during non-interactive
  sessions should be nearly identical and should use the same code.
- Any exceptions where `compose` and `inline-compose` need to be done
  differently must include clear comments in the code that describe why this
  exception is important.
- All calls to `emit_legacy_composition_session_event` should be changed to
  use the modern alternative.
- Once all calls to `emit_legacy_composition_session_event` are removed, then
  remove it from the code base.

## Behavior Parity

The following behaviors must be unified so that both modes share a single
implementation:

- **Codex post-hoc text application** (`codex_output.apply_to_summary`) is
  applied identically in both modes.
- **Stdout rendering of assistant text** is applied identically in both modes,
  gated by the same `had_streamed_assistant` signal so that neither mode
  duplicates text that already streamed live.
- **`had_streamed_assistant` tracking** is performed for both modes and used
  consistently to gate stdout re-rendering.
- **Legacy (non-structured) runs** — currently only reachable for Goose —
  must render the same stderr summary block that structured runs do, built
  from a `StreamExecutionSummary` with the fields that are knowable (exit
  code, `is_error`) populated and the rest left empty. This replaces the
  JSONL-only silence of `emit_legacy_composition_session_event`.

## Behavior Removal

- The empty-text warning (`"the agent did not provide a summarized message on
  their completed work!"`) currently emitted by `inline-compose` must be
  removed. Neither mode should emit this warning. The `SessionEnd` JSONL
  event already records when assistant text is empty; no stderr message is
  needed.

## Intentional Exceptions

These differences between compose and inline-compose are intentional and must
be preserved with comments in the merged code path:

1. **Closure validation and file write** — inline-compose mutates the source
   file; compose does not. Closure extraction, frontmatter merge, and
   darkmatter cleanup are exclusive to inline.
2. **Deferred summary timing** — inline-compose must emit the summary after
   closure validation messages so the user sees file-write status before
   metadata. Compose emits immediately because there is no intermediate
   output.
3. **Interrupted-session body report** — inline-compose reports partial body
   content when interrupted. Compose does not write files so this is
   irrelevant.
4. **Writability pre-check** — inline-compose validates write permission on
   the target file before execution. Compose does not write files.

## Testing

- Unit tests cover the new shared helpers (`run_structured_composition`,
  `emit_composition_summary`, `emit_minimal_composition_summary`).
- A golden-output test runs both `compose` and `inline-compose` against a
  mock provider (with no closure validation work) and asserts their stderr
  summary output is byte-identical, modulo the documented intentional
  exceptions above.

## Delivery

- Multi-phased plan following the phase structure in `tech-design.md`.
- All git work (commits, branches, PRs) is handled externally to the plan;
  phase boundaries are logical checkpoints, not enforced PR boundaries.
