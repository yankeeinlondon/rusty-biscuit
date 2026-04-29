---
fixed: 2026-04-21
agent: "1"
---

# Fix: OpenCode step_timeout kills subagents mid-execution

## Root Cause

Two interacting problems caused `just commit` to fail with `step_timeout`:

1. **`detect_step_timeout` killed the process while subagents were in-flight.** The OpenCode stream parser correctly tracks subagents via `SubagentStart`/`SubagentStop` events, and `detect_opencode_hang_termination` already deferred when `in_flight_subagents` was non-empty. However, `detect_step_timeout` did NOT check for in-flight subagents — it was a hard kill on silence regardless. When the commit prompt spawned Task subagents via OpenCode, those subagents don't emit events on the parent stream while running. So after the parent saw the last tool result, the silence timer started counting, and 7 minutes later the entire process group was SIGTERM'd — including the still-running subagents that hadn't finished committing.

2. **The commit prompt instructed orchestrators to run subagents concurrently.** The prompt said "concurrently execute a subagent for every semantic group", which (a) risks race conditions on the shared git staging area (already documented in lessons learned), and (b) means multiple subagents are in-flight simultaneously, increasing the window for the step_timeout to fire during their execution.

## Changes

### Code: `claudine/cli/src/commands/wrap/exec.rs`

- `detect_step_timeout` now checks `state.in_flight_subagents` before firing. When subagents are tracked as in-flight, the kill is deferred with a debug log message. Tool calls that are in-flight are NOT exempted — only subagents (which run as separate processes with their own streams) receive this grace.
- New test: `detect_step_timeout_defers_when_subagents_in_flight` verifies the deferral and confirms the kill fires after subagents complete.

### Prompt: `claudine/prompts/commit.md`

- Changed from "concurrently execute a subagent for every semantic group" to "ONE AT A TIME, spawn a subagent for each semantic group"
- Added explicit "CRITICAL RULES about subagent usage" section emphasizing sequential execution and shared-index race conditions
- Added instruction to verify each commit succeeded before starting the next subagent
- Changed `sniff git commits` to `git log --oneline -20` (the `sniff git commits` command does not exist per lessons learned)
- Removed the "the subagent, if it ran into any problems while trying to commit" dangling sentence

### Docs: `claudine/docs/topics/composition.md` and `claudine/docs/topics/validations-and-handlers.md`

- Updated `step_timeout` description to note the subagent-in-flight deferral behavior.
