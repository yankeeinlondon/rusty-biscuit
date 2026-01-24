# OpenCode Agent Delegation

This document describes how to delegate pipeline steps to the OpenCode CLI. The `OpenCodeDelegation` primitive mirrors the Claude Code contract: a prompt plus serialized state goes in, a structured response comes out.

## Overview

OpenCode delegation wraps the `opencode` CLI in two modes:

- **Interactive (TUI)**: Launches the OpenCode terminal UI so a human can collaborate.
- **Headless**: Runs a single `opencode run` call and parses JSON events for output.

Both modes embed a serialized JSON state payload and (optional) JSON schemas into the prompt so the agent can read and update pipeline state.

## CLI Invocation

### Interactive mode

Use the OpenCode TUI for human-in-the-loop workflows, then capture structured output with a follow-up headless call:

```bash
opencode --prompt "${PROMPT_WITH_STATE}"

# After the user exits, capture structured output
opencode run --continue --format json "Return the final output as JSON matching the output schema."
```

The follow-up uses `--continue` to resume the most recent session. Use `--session <id>` if you want deterministic session selection.

### Headless mode

Use the run subcommand when no human interaction is required:

```bash
opencode run --format json "${PROMPT_WITH_STATE}"
```

OpenCode emits a JSON event stream. The delegation step extracts assistant text from `message.part.updated` events.

## State and Schema Injection

Because OpenCode does not expose a `--json-schema` flag, schema guidance is provided directly in the prompt:

```
<user prompt>

Pipeline State (JSON):
{ ... }

State Schema (JSON):
{ ... }

Output Schema (JSON):
{ ... }

Instructions:
Use the provided state JSON and schema. Return the final output as JSON matching the output schema. Output JSON only.
```

The implementation validates that the assistant output is valid JSON when an output schema is supplied.

## Rust Usage

```rust
use ai_pipeline::primitives::atomic::agent_delegation::{
    OpenCodeDelegation, OpenCodeMode, OpenCodeSession,
};
use ai_pipeline::primitives::state::{PipelineState, StateKey};
use serde_json::json;

const AGENT_STATE: StateKey<serde_json::Value> = StateKey::new("agent_state");
const SESSION_ID: StateKey<String> = StateKey::new("opencode_session");

let mut state = PipelineState::new();
state.set(AGENT_STATE, json!({"task": "Summarize the README"}));

let delegation = OpenCodeDelegation::new("Summarize the docs")
    .with_state_key(AGENT_STATE)
    .with_output_schema(json!({
        "type": "object",
        "properties": {
            "summary": { "type": "string" }
        },
        "required": ["summary"]
    }))
    .with_session_key(SESSION_ID)
    .with_session(OpenCodeSession::ContinueLast)
    .headless();

let output = delegation.execute(&mut state)?;
```

## Output Parsing Notes

- `--format json` emits a stream of JSON events, not a single JSON object.
- The delegation step accumulates assistant output from `message.part.updated` events.
- Session IDs are captured from `session.created` events and can be stored in state for later use.
