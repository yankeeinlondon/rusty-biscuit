---
created: 2026-06-03
reviewed: true
status: ready for planning and implementation
area: claudine
kind: fix
depends-on: null
related:
  - claudine/features/2026-06-03-always-harness/spec.md
  - claudine/fixes/2026-06-06-inline-sequence/spec.md
---

# Inline Compose Final Response Capture

## Problem

`inline-compose` rewrites a Markdown document body from the provider response.
Today that body can be sourced from `assistant_text`, which is the accumulated
assistant output for the whole provider session.

That accumulation is correct for summaries and direct `compose`, but it is too
broad for an inline artifact. A provider can emit text-only assistant messages
between tool calls, for example:

1. "Let me read the research documents first."
2. tool call
3. "Now I will write the document."
4. tool call
5. final document body

The first and third messages are process narration. They are not reasoning
blocks, and they are not the requested artifact. If `inline-compose` writes the
full `assistant_text` accumulation, those narration blocks leak into the file
body before the final answer.

Reasoning and thinking events are not the bug. They already use separate
semantic events and must stay excluded from `assistant_text` and inline body
capture.

## Scope

In scope:

- Define a provider-agnostic final-response capture contract for structured
  streams.
- Use that final response as the body source for `inline-compose`.
- Preserve existing `assistant_text` behavior for direct `compose`, summaries,
  logs, metrics, and terminal rendering.
- Preserve the Codex post-hoc last-message behavior for providers that do not
  stream usable `OutputText`.
- Add regression coverage for interstitial narration and for harness vs
  non-harness convergence while both execution paths exist.
- Update user-facing composition documentation to describe the closure
  contract.

Out of scope:

- Changing provider parsers except where they currently misclassify reasoning
  as `OutputText`.
- Changing direct `compose` output or stream summary behavior.
- Changing the inline frontmatter preservation, `last_updated`, atomic-write,
  or cleanup contracts.
- Bringing the legacy non-structured Goose capture path under this contract.
  That path captures provider stdout directly and should be handled by a
  separate structured-Goose or legacy-capture cleanup.
- Implementing the broader always-harness refactor. This fix must still cover
  both current paths until that refactor lands.

## Design Decision

Add a final-response accumulator at the normalized semantic layer, not inside a
provider-specific parser and not inside the file rewrite code.

The accumulator tracks the text emitted after the most recent tool call:

- On `SemanticEvent::OutputText`, append the text chunk.
- On `SemanticEvent::ToolCall`, clear the accumulator.
- Ignore `Reasoning`, `ToolResult`, metadata, status, errors, and provider
  extension events.

At stream end, the accumulator contains the closing assistant response for
providers that emit structured output text. `inline-compose` writes that value
to the document body.

This is intentionally keyed to `ToolCall`, not `ToolResult`. Some providers
emit a tool start event without a reliable matching result, or emit results in
provider-specific shapes. The user-visible boundary that makes earlier text
process narration is the agent's decision to call another tool. Resetting on
`ToolCall` therefore catches the leak at the earliest provider-agnostic point.

## Fallback Behavior

Some providers do not stream the final assistant response as `OutputText`.
Codex is the important existing example: Claudine recovers the final answer
post-hoc through `--output-last-message` and patches `assistant_text`.

For `inline-compose`, the body source must be selected as follows:

1. If the final-response accumulator contains non-whitespace text, write the
   accumulator value.
2. Otherwise, if the provider has a documented post-hoc final-message recovery
   path and `assistant_text` contains non-whitespace text from that recovery,
   write `assistant_text`.
3. Otherwise, treat the response as empty and preserve the existing
   empty-captured-output failure behavior. Do not fall back to pre-tool
   narration merely because the accumulator is empty.

The third rule is important. A structured provider may emit narration, call a
tool, and then fail to emit a final answer. In that case `inline-compose` must
fail rather than writing stale narration into the file.

## Required Implementation Shape

The implementation should store the accumulator with the structured stream
details already shared by the live semantic sink and composition closure code.
The expected shape is:

- Add `final_response: String` to `StructuredSummaryDetails`.
- Add small methods equivalent to `push_final_response(&str)` and
  `reset_final_response()`.
- Update `LiveSemanticSink::on_semantic_event` so `ToolCall` resets the
  accumulator and `OutputText` appends to it.
- Update structured `inline-compose` closure selection so inline writes the
  final-response value first, with only the documented post-hoc fallback.
- Ensure the harness inline closure path uses the same final-response source as
  the non-harness structured path.

Do not change `StreamExecutionSummary::assistant_text` semantics. It remains
the full accumulated assistant text for the whole session.

## Provider Contracts

Structured providers that emit assistant text as `OutputText` are covered by
the accumulator. This includes Claude, Gemini, OpenCode, Kimi wire mode, Qwen,
and any future provider parser that uses the normalized semantic stream.

Codex remains valid through its post-hoc last-message recovery path. The
implementation must make that fallback explicit enough that future providers
cannot accidentally use full-session `assistant_text` unless they intentionally
declare equivalent final-message recovery behavior.

Legacy Goose is not fixed here. The spec should not claim provider-agnostic
coverage for non-structured stdout capture.

## Side Effects and Mitigation

The design intentionally changes only the artifact body source for
`inline-compose`. It must not change:

- live terminal output;
- direct `compose` stdout;
- summary text, token accounting, tool lists, metadata, or logs;
- handler input other than the inline replacement body;
- provider execution arguments; or
- prompt composition and schema validation.

The main side effect is that text emitted before the last tool call disappears
from inline artifacts. That is the desired behavior for process narration. If a
provider emits real user-requested content before a later tool call, the tool
call means the agent continued working and the earlier content is not the final
artifact. Users who need multi-part artifacts should have the agent include all
parts in the closing response.

## Acceptance Criteria

1. A structured-provider stream with `OutputText`, `ToolCall`, `OutputText`,
   `ToolCall`, and final `OutputText` writes only the final `OutputText` to the
   inline document body.
2. The same scenario does not write any interstitial narration into the body.
3. Reasoning or thinking events never contribute to the final-response
   accumulator.
4. `compose` and stream summaries continue to use full `assistant_text`.
5. Codex inline-compose still writes the post-hoc recovered last message when
   the final-response accumulator is empty.
6. A structured provider with only pre-tool narration and no post-tool final
   response fails with the existing empty-output behavior instead of writing
   narration.
7. Inline-compose with a `harness:` block and inline-compose without a
   `harness:` block produce identical replacement bodies for the same
   structured stream.
8. The source document's frontmatter preservation, new-frontmatter merge,
   `last_updated`, atomic write, and markdown cleanup behavior remain
   unchanged.
9. Legacy Goose behavior is either unchanged or explicitly covered by a
   separate follow-up; this fix must not silently broaden the legacy stdout
   capture contract.
10. User-facing composition documentation states that inline closure writes the
    final response only and explains the Codex-style post-hoc fallback without
    implying full coverage for legacy non-structured providers.

## Tests

Required tests:

- Unit test the semantic sink accumulator:
  `OutputText("narration") -> ToolCall -> OutputText("more narration") ->
  ToolCall -> OutputText("final")` leaves `final_response == "final"`.
- Unit test that `Reasoning` events do not append to `final_response`.
- Integration test `inline-compose --claude` or an equivalent structured stub
  where narration appears between tool calls and only the final document body is
  written.
- Integration test that Codex post-hoc final-message recovery remains accepted
  when no structured final response is present.
- Integration test that an empty final response after a tool call fails without
  mutating the source file.
- Convergence test for harness and non-harness inline-compose paths while both
  paths still exist.

Use focused package tests rather than the full workspace unless a touched
shared contract requires broader coverage. Do not run `cargo fmt` unless the
implementation task explicitly calls for it.

## Documentation

Update `claudine/docs/topics/composition.md` under Inline Composition closure
behavior. The docs should say:

- the replacement body is the agent's final response only;
- "final response" means output text emitted after the agent's last tool call;
- interstitial narration is dropped;
- providers with post-hoc final-message recovery can supply that final message
  directly; and
- legacy non-structured stdout capture is not part of this guarantee.

## Open Questions

None. The review decision is to implement the semantic-layer accumulator and to
keep legacy Goose out of scope.

## Definition of Done

This fix is complete when the acceptance criteria pass, the composition docs are
updated, and existing inline-compose, compose, and sequence tests covering the
shared structured pipeline remain passing.
