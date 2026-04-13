# Improved Feedback: Blocked Items

Items gated on external decisions, new architecture, upstream changes, or sequencing dependencies.

## Blocked Suggestions

| Suggestion | What | Gated On | Effort | Value |
| --- | --- | --- | --- | --- |
| Kimi Wire-mode integration | Move Kimi's high-fidelity integration path from print-mode `stream-json` to bidirectional `--wire`, including JSON-RPC envelopes, capability negotiation, approvals, structured questions, plan displays, and subagent events. | A deliberate wrapper/runtime design for bidirectional stdio, `initialize` negotiation, and request/response handling. This should not be folded into a parser-only change. | High | Unlocks the richest Kimi supervision surface and removes many of the current blind spots around human-in-the-loop, planning, and subagent activity. |
| Roo stream support | Add a real Roo NDJSON parser and wrapper parity for current Roo CLI event types such as `thinking`, `tool_use`, `tool_result`, `cost`, and `final_result`. | Confirmation that Roo is an actively supported Claudine target and a decision on the desired support level for Roo-specific fields. | Medium-High | Closes a provider parity gap and turns Roo from adapter-only support into a real wrapped stream participant. |
| Gemini per-model attribution and cache reporting | Surface `result.stats.models`, cache efficiency, and related Gemini-specific session metrics in a clearer end-of-run summary once the core parser-correctness work is done. | Agreement on how much provider-specific summary detail Claudine should show in the default stderr flow without making the common case noisy. | Low-Medium | Makes Gemini's multi-model routing and cache behavior much easier to understand in unattended runs. |
| Unified live checklist / plan UI | Introduce a normalized rendering model for checklists, plan displays, and progress snapshots that multiple providers can feed: Codex `todo_list`, Kimi `PlanDisplay`, Claude task/progress signals, and future Qwen/Kimi control-plane events. | Agreement on a cross-provider UI contract, not just parser outputs. Some of this work also depends on Kimi Wire adoption to be fully worthwhile. | Medium | Gives users a more legible sense of progress during long-running sessions instead of relying mostly on tool lines and warnings. |
| Hook plus stream fusion | Correlate stdout stream events with richer hook or side-channel events for providers where stdout is intentionally filtered, especially OpenCode and possibly Qwen/Gemini. | A correlation strategy for deduping multiple event feeds and a wrapper/runtime design that can safely consume both channels together. | High | Makes Claudine substantially better at permission, question, and model-routing visibility without waiting for upstream stdout changes. |
| Pricing-aware cost estimation | Estimate cost for providers that expose token usage but not stable cost fields by combining model identity with maintained pricing metadata. | A maintained pricing source, reliable model identity, and a policy decision on how much approximation is acceptable. | Medium-High | Improves budget awareness, especially for providers where the stream is strong on token counts but weak on spend. |
| Buffered-mode telemetry supplementation | Optionally combine streaming runs with a buffered final artifact when the buffered mode exposes richer end-of-run metadata than the stream, such as Qwen's richer `json` result stats. | A wrapper strategy that can gather richer summaries without deadlocking the subprocess contract or sacrificing live progress. | Medium | Lets Claudine keep live feedback while still capturing provider-specific final telemetry that is absent from the streaming path. |
| Rich subagent correlation | Build a first-class parent/child session model for providers with subagent features, including Codex collab calls, Goose subagent notifications, OpenCode `task`, Kimi subagents, and future Roo child sessions. | Reliable parent-child correlation keys and a normalized reporting story for nested sessions. Some providers also need richer upstream signals. | Medium-High | Makes multi-agent runs understandable instead of flattening them into generic tool traffic. |
| Cross-provider reporting schema expansion | Add new normalized summary fields only after multiple providers prove the concept is common, such as explicit permission-denial summaries or normalized subagent summaries. | Evidence that the concept is truly cross-provider and worth the API surface expansion. | Medium | Keeps reporting queryable and coherent without turning `StreamExecutionSummary` into an unbounded provider-specific bag of fields. |

## Blocker Reasons

- **Kimi Wire-mode** — requires new bidirectional stdio runtime architecture
- **Roo stream** — requires product decision on target support level
- **Gemini attribution** — gated on parser-correctness completion and verbosity agreement
- **Unified checklist** — requires cross-provider UI contract; also depends on Kimi Wire
- **Hook + stream fusion** — requires new correlation strategy and dual-channel wrapper design
- **Pricing-aware cost** — requires external maintained pricing data source
- **Buffered telemetry** — requires wrapper strategy that avoids subprocess deadlocks
- **Rich subagent** — requires upstream correlation keys; some providers lack signals
- **Cross-provider reporting** — requires evidence across multiple providers first
