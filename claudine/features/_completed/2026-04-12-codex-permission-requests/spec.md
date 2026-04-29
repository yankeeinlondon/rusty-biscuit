## What Codex already gives us (via typed protocol):

- PermissionRequest — a prompt to allow/deny a specific action (shell command, file write, etc.)
- ApprovalRequest — a structured approval gate (e.g. sandboxed vs. privileged execution)
- UserInputRequest — interactive questions blocking progress

These are parsed into typed variants in stream/protocol/codex.rs and flow through permission_meta() / on_permission_request()
handlers, but they currently don't materialize in StreamExecutionSummary or the JSONL summary event.

## The narrower feature idea:

Surface Codex permission activity in the end-of-run summary — without claiming cross-provider normalization. Concretely:

1. Counters in the Codex parser — increment per-session counts as PermissionRequest / ApprovalRequest / UserInputRequest events
arrive.
2. New optional summary fields — something like permission_prompts: Option<u32> and, if resolution events exist,
permission_denials: Option<u32>. Left as None for other providers; skip_serializing_if = "Option::is_none" keeps JSONL clean.
3. Summary prose line — one-liner in the wrapper stderr summary ("3 permission prompts, 1 denied") using the same Prose + badge
styling as session badges.
4. JSONL summary event — counts serialized alongside existing fields; no schema break.

## Why this is worthwhile as a standalone step:

- Proves the field shape (count vs. structured list vs. both) without prematurely committing to a normalized cross-provider contract.
- Gives immediate user value for Codex sessions, where permission prompts are a real operational signal.
- Creates a working reference that future providers (Kimi Wire, OpenCode) can mimic once they expose equivalent signals.

### Open questions worth answering before implementing:

- Does Codex emit a resolution event after a PermissionRequest (approved vs denied), or does Claudine only see the ask? If only the ask, the field should be named permission_prompts — calling it "denials" would be dishonest.
- Should `UserInputRequest` roll into the same counter or stay separate (it's a different UX concept: "waiting on me" vs. "am I allowed").

    - DECISION:
        - We want Claudine to be able to see as much as is possible; this is really a question of whether this is feasible or not.
        - Keep `UserInputRequest` separate

- Naming discipline: use neutral names (permission_prompts) so the field stays honest when Kimi/OpenCode light up, rather than Codex-specific names that would need renaming later.
    - DECISION: use neutral names

> **Scope:** small — 1–2 days. New feature file under _unscheduled/ would be appropriate since it's distinct from the still-blocked normalized schema expansion.


