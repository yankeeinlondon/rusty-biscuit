## The bug

The inline body capture used result.assistant_text — and assistant_text is the running accumulation of every assistant text block across the
whole session. When an agent narrates between tool calls ("Let me read the research documents…", "Now I'll write…"), each narration block is a
separate text-only message that gets appended to assistant_text. So the body ended up as all the interstitial narration concatenated with the
final summary — exactly the mess you pasted.

(Actual thinking blocks were already excluded — they're emitted as Reasoning, not output text. The leak was the plain-text narration Claude
emits before each tool call.)

## The fix

I added a provider-agnostic "final response" accumulator at the normalized semantic layer, defined precisely as the output text emitted after
the agent's last tool call:

- On every OutputText semantic event → append to the accumulator.
- On every ToolCall → reset it (anything said before a tool call is process narration, not the closing answer).

So after the stream ends, the accumulator holds only the final turn. inline-compose now writes that, not assistant_text. Providers that don't
stream output text (Codex, which recovers its last message post-hoc into assistant_text) yield an empty accumulator and fall back to the prior
behavior. Compose mode and all reporting keep using assistant_text unchanged — the change is confined to inline body capture.

**Files:**

- claudine/cli/src/commands/wrap/policy.rs — final_response field + push_final_response/reset_final_response on StructuredSummaryDetails.
- claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs — drive the accumulator from OutputText/ToolCall.
- claudine/cli/src/commands/wrap/composition/structured.rs — inline capture uses the final response (with assistant_text fallback).
- claudine/docs/topics/composition.md — documented the "final response only" closure contract.

**Tests:**

- Sink unit test final_response_keeps_only_text_after_last_tool_call (narrate→tool→narrate→tool→final ⇒ only final survives).
- CLI integration test inline_compose_writes_only_final_response_not_narration (inline-compose --claude with a stream-json stub; asserts the
body is the final document and neither narration block leaked).

**Verification**

- cargo clippy -p claudine-cli — clean
- claudine-cli binary unit tests — 1034/1034
- wrap_commands — 125/125 · sequence_cli — 24/24

One scope note: this fixes the structured-stream providers (Claude, Gemini, etc.). The legacy non-structured Goose path still captures raw
stdout as the body — that's a separate path and wasn't part of what you hit, but say the word if you want it brought under the same rule.
