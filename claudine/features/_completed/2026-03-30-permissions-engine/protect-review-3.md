# Protect Review 3

Checks run:

- `cargo test -p claudine --lib protect -- --nocapture`
- `cargo test -p claudine --lib adapters::tests -- --nocapture`
- `cargo test -p claudine --lib dispatch::tests -- --nocapture`

All of the focused Protect, adapter, and dispatch tests above passed.

## Findings

### P1: Wrapper `AGENT_PARAMS` decoding is incompatible with the wrapper contract, so effective policy resolution is likely broken in real wrapped sessions

The wrapper writes `AGENT_PARAMS` as a JSON string array in `claudine/cli/src/commands/wrap/env.rs:56-58,84`, but dispatch reconstructs argv with `split_whitespace()` in `claudine/lib/src/dispatch/mod.rs:300-309`. The provider parsers then expect real argv tokens such as `--sandbox`, `--approval-mode`, or `--permission-mode` (`claudine/lib/src/permissions/providers/codex.rs:188-223`, `claudine/lib/src/permissions/providers/claude.rs:180-227`, `claudine/lib/src/permissions/providers/gemini.rs:253-277`). A JSON blob like `["--sandbox","workspace-write"]` does not survive that split as usable flags, so wrapped sessions can silently fall back to configured-only policy even though the design depends on effective policy whenever wrapper argv exists.

Suggested fix: parse `AGENT_PARAMS` with `serde_json::from_str::<Vec<String>>()` first, and only fall back to shell splitting for legacy/non-JSON values. Add a dispatch integration test that proves wrapper-supplied argv changes the resolved policy and yields `ProtectPolicyMode::Effective`.

### P1: `AfterModel` / `McpResponse` never receives the post-action Protect pass, so redaction/blocking for that phase is currently dead

Protect models `AgenticEvent::AfterModel` as `ProtectPhase::McpResponse` in `claudine/lib/src/services/protect/service.rs:326-335`, and the default observer explicitly supports `AfterModel` in `claudine/lib/src/services/protect/observe.rs:41-50`. But dispatch only runs post-action Protect for `AfterTool`, `TurnComplete`, and `SubagentStop` in `claudine/lib/src/dispatch/mod.rs:224-229`. The only place redaction is actually applied is the `protect_post` branch in `claudine/lib/src/dispatch/mod.rs:249-267`, so any `AfterModel` evaluation can produce a plan that is never used.

Suggested fix: include `AgenticEvent::AfterModel` in the post-action Protect path, or switch the gate from a hardcoded event whitelist to phase/capability-based logic. Add an integration test that drives an `AfterModel` event with a redactable payload and verifies the modified response reaches `adapter.format_response(...)`.

### P2: `ModifyProviderConfig` exists in the decision model, but no observer emits it, so the designed config-mutation guard is non-functional

`ProtectIntent::ModifyProviderConfig` is part of the public intent surface in `claudine/lib/src/services/protect/intent.rs:15,46`, and the evaluator treats it as elevated risk in `claudine/lib/src/services/protect/evaluate.rs:264-275,537-545`. However, the default observer in `claudine/lib/src/services/protect/observe.rs:37-127` never emits this intent, and the provider-specific observers only rebuild command/path/MCP/mode intents (`claudine/lib/src/adapters/claude.rs:92-149`, `claudine/lib/src/adapters/codex.rs:105-178`, `claudine/lib/src/adapters/gemini.rs:79-146`, `claudine/lib/src/adapters/roo.rs:71-158`). As implemented, provider config edits are treated as ordinary writes, so the final design's explicit provider-config escalation never actually fires.

Suggested fix: teach observers to recognize provider config targets and config-mutating CLI operations, for example edits under provider config locations or tool calls that mutate settings files. Add provider fixtures that prove these operations emit `ModifyProviderConfig` and escalate according to posture.

### P2: Completion output scanning appears to be wired only for synthetic tests, not for the real event flow

The completion scan only inspects `observation.payload` in `claudine/lib/src/services/protect/evaluate.rs:694-753`. The default observation code only fills `payload` from `meta.tool_response` in `claudine/lib/src/services/protect/observe.rs:111-119`. `EventMeta` has `tool_response` and `notification_message`, but no canonical field for model completion text in `claudine/lib/src/events/event_meta.rs:44-46,64-70`. On top of that, dispatch currently runs the post-action Protect pass on `TurnComplete`, not `AfterModel` (`claudine/lib/src/dispatch/mod.rs:224-229`), which is where provider output is more naturally surfaced. The current unit tests pass because they manually inject `ProtectPayload`, but I did not find an adapter/dispatch path that would do this for a real completion.

Suggested fix: define a canonical completion-output payload path, populate it from stream/adapters, and cover it with an end-to-end test that proves a real provider completion containing a secret or instruction payload is caught by Protect.

## Coverage Gaps

- There is still no dispatch integration test for the wrapper JSON `AGENT_PARAMS` path. The dispatch tests stop at trust/session construction (`claudine/lib/src/dispatch/mod.rs:742-876`) and do not prove that wrapped argv produces an effective snapshot.
- Adapter observation tests only assert specific semantic extraction for Claude, Codex, Gemini, and Roo in `claudine/lib/src/adapters/mod.rs:423-607`. There are no targeted fixtures for Qwen, OpenCode, Kimi, or Goose, and no fixtures for `SpawnSubagent`, `ModifyProviderConfig`, or `TraversePath`.
- There is no integration test proving that `AfterModel` / `McpResponse` redaction is applied before provider formatting.
- There is no end-to-end test proving completion scanning sees actual provider output instead of a hand-constructed `ProtectPayload`.

## Ergonomics / Performance

- Snapshot caching from the final design is not implemented. `ProtectService` has no cache field in `claudine/lib/src/services/protect/service.rs:28-34`, and `evaluate_structured()` always resolves through `evaluate_request()` in `claudine/lib/src/services/protect/service.rs:129` even though `evaluate_with_snapshot()` is explicitly positioned for cached reuse in `claudine/lib/src/services/protect/evaluate.rs:122-127`. Pre/post evaluations in the same hook process currently pay the full snapshot resolution cost twice.
- Regex-heavy paths recompile patterns on every evaluation: MCP redaction recompiles each pattern in `claudine/lib/src/services/protect/redact.rs:54-64`, and completion scanning recompiles secret and command patterns in `claudine/lib/src/services/protect/evaluate.rs:717-744`. Precompiling validated regexes once inside the effective Protect config would reduce repeated overhead and simplify hot-path code.
