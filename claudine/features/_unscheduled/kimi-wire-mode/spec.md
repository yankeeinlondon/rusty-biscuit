# Kimi Wire Mode

Move Claudine's Kimi integration from print-mode `stream-json` to bidirectional `--wire` (JSON-RPC 2.0 over stdin/stdout) as a full replacement. Wire mode becomes the only Kimi integration path; the existing print-mode stream-json parser, adapter, and documentation are retired in the same change.

This unlocks the richest Kimi supervision surface and removes the current blind spots around human-in-the-loop, planning, and subagent activity.

## Goals

- Replace print-mode `stream-json` with `--wire` as the sole Kimi path.
- Normalize Kimi Wire events onto Claudine's existing 16-event lifecycle model; add new canonical events only when no existing event fits, so capabilities added here benefit future providers too.
- Keep the wrapper UX unchanged: `claudine kimi` transparently launches Wire mode, with no new user-facing flags.
- Retire the old code and docs in the same PR — no compatibility shims, no dead parser code.

## In Scope

Initial implementation covers the full Wire surface needed for production supervision:

- **Transport**: JSON-RPC 2.0 envelope handling over stdin/stdout, including `initialize`, capability negotiation, request/response correlation, and notification handling.
- **Approvals / permissions**: tool-use and edit approval prompts surfaced through Wire, routed through Claudine's existing permission and dispatch path.
- **Structured questions**: Kimi's structured question prompts (AskUserQuestion-style) rendered and answered via Wire.
- **Plan displays**: plan-mode content surfaced through Claudine's normalized events.
- **Subagent events**: subagent lifecycle (start, tool use, completion) mapped into normalized events.

## Out of Scope

- Session replay from `~/.kimi/sessions/<hash>/<id>/wire.jsonl` (Wire `replay` command). May be revisited alongside the broader `resume` feature.
- MCP runtime injection for Kimi (tracked separately; Kimi is still on "no MCP support yet" per [MCP support](../../../docs/mcp-support.md)).
- Changes to other providers' stream adapters.

## User-Facing Surface

- `claudine kimi` automatically runs the provider in `--wire` mode. No new flags, no config opt-in, no fallback path.
- If the installed Kimi binary does not support `--wire`, the wrapper surfaces a clear preflight error directing the user to upgrade. It does **not** silently fall back to print mode.

## Integration Strategy

- Extend the normalized 16-event model first: map Wire events (tool use, tool result, session start/end, usage, message, error, etc.) onto the existing canonical events.
- Introduce new canonical events **only** for lifecycle points that have no existing match (candidates: approval request, structured question, plan display, subagent start/stop). Any new events must be designed to be reusable by other providers that gain similar capabilities.
- Add a new Wire adapter under `stream::adapters` (and corresponding typed protocol module under `stream::protocol/kimi.rs` if schema differs meaningfully from the retired print-mode types), following the two-pass dispatch pattern used by the other provider parsers.
- Remove the print-mode Kimi parser, its protocol types, and its documentation references.

## Success Criteria

- `claudine kimi` launches in Wire mode and round-trips `initialize` + capability negotiation against a real Kimi binary.
- Approval prompts, structured questions, plan displays, and subagent events from Kimi flow through Claudine's dispatch pipeline and surface to hooks/actions equivalently to other providers.
- No references to Kimi print-mode `stream-json` remain in code, tests, docs, or skills.
- Stream parser test suite covers each new Wire event variant and the `unknown_event_type_fails_typed` contract.

## Benefit

Unlocks Claudine's richest Kimi supervision surface — permissions, planning, structured Q&A, and subagent visibility — closing gaps that the print-mode path cannot address.
