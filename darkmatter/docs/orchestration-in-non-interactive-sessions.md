# Orchestration in Non-Interactive Sessions

This document covers how to run multi-phase, orchestrated work using agentic CLIs in non-interactive (headless) mode — specifically focusing on subagent spawning, which is the key mechanism for keeping context windows manageable during large implementations.

## The Problem

When implementing a multi-phase plan (e.g., 3 phases, 14 tasks), running everything in a single `-p` session risks exhausting the context window before completion. Subagents solve this by giving each phase a fresh, isolated context window.

However, subagent support in non-interactive mode requires explicit configuration — it doesn't "just work" like it does in interactive sessions.

## Claude Code

### Flags and Modes

| Flag | Purpose |
|------|---------|
| `-p "prompt"` | Run headless (non-interactive), exit after completion |
| `--agent <name>` | Run as a named agent definition (from `.claude/agents/`) |
| `--agents '<json>'` | Define subagents dynamically via inline JSON |
| `--dangerously-skip-permissions` | Skip all permission prompts (YOLO mode) |
| `--allowedTools "Tool1,Tool2"` | Pre-approve specific tools |
| `--max-turns N` | Limit agentic turns |

### Enabling Subagents in Headless Mode

The Agent tool is available in `-p` mode **if subagents are explicitly defined** via the `--agents` flag:

```bash
claude -p "Implement the plan" \
  --dangerously-skip-permissions \
  --max-turns 30 \
  --agents '{
    "phase-worker": {
      "description": "Implements a single phase of the plan",
      "prompt": "You implement code changes for a specific phase. Read the assigned tasks, write the code, verify with cargo check.",
      "tools": ["Read", "Edit", "Write", "Bash", "Grep", "Glob"],
      "model": "sonnet"
    }
  }'
```

Without `--agents`, the Agent tool may not be available and Claude will skip delegation.

### Agent Definition Files

File-based agent definitions live in:
- `.claude/agents/` (project-scoped, checked into git)
- `~/.claude/agents/` (user-scoped)

Format is Markdown with YAML frontmatter:

```markdown
---
name: phase-worker
description: Implements a single phase of an implementation plan
tools: Read, Edit, Write, Bash, Grep, Glob
model: sonnet
---

You implement code changes for a specific phase of a plan.
Read the assigned tasks, write the code, and verify compilation.
```

These can be referenced via `--agent phase-worker` or spawned as subagents from a parent agent.

### Permission Propagation

- `--dangerously-skip-permissions` propagates to all subagents automatically
- Individual subagent `permissionMode` can be set in the agent definition
- Background subagents that need unapproved tools will fail silently (not block)

### Limitations

- **No nesting**: Subagents cannot spawn their own subagents
- **No inter-agent messaging**: Subagents report back to the parent only
- Each subagent gets its own isolated context window (this is a feature, not a bug)

## Other Claudine-Supported Agents

All 8 agents supported by claudine have some form of subagent support in non-interactive mode:

| Agent | Status | Style | Notes |
|-------|--------|-------|-------|
| Claude Code | Supported | Tool delegation (`--agents`) | Most mature |
| Codex CLI | Experimental | Tool delegation | Requires `[features].multi_agent = true` in config |
| Gemini CLI | Experimental | Automatic spawn | Requires `experimental.enableAgents` setting |
| Goose | Supported | Automatic spawn | Via `auto` permission mode and YAML recipes |
| Kimi Code | Supported | Tool delegation | Agent definitions in `~/.kimi/agents/` |
| OpenCode | Supported | Tool delegation | Task tool with `~/.config/opencode/agents/` |
| Qwen Code CLI | Supported | Tool delegation | Agent definitions in `~/.qwen/agents/` |
| Roo Code | Partial | Orchestrator mode | Uses mode files, not directory-based agents |

## Strategies for the Feature Pipeline

### Strategy 1: Subagent Orchestration (Recommended for Large Plans)

The parent Claude session reads the plan and delegates each phase to a subagent. Each phase gets a fresh context window.

```bash
claude -p "Read plan.md. For each phase, spawn a phase-worker subagent with that phase's tasks." \
  --dangerously-skip-permissions \
  --max-turns 30 \
  --agents '{
    "phase-worker": {
      "description": "Implements one phase of the plan",
      "prompt": "Implement all tasks in the given phase. After each task, run cargo check. Report files changed.",
      "tools": ["Read", "Edit", "Write", "Bash", "Grep", "Glob"]
    }
  }'
```

**Pros**: Each phase gets full context window. Parent orchestrates with minimal context use.
**Cons**: Requires `--agents` configuration. Parent can't see subagent's intermediate work.

### Strategy 2: Justfile Phase Chaining (Recommended for Reliability)

The justfile itself orchestrates phases, calling separate Claude sessions for each:

```just
_feature_implement_phase feat base_dir phase:
    #!/usr/bin/env bash
    claude --dangerously-skip-permissions --max-turns 20 -p \
      "Read {{base_dir}}/plan.md. Implement ONLY Phase {{phase}}. \
       Check {{base_dir}}/log.md for prior phase completions. \
       After completing, append results to {{base_dir}}/log.md."
```

**Pros**: Maximum reliability. Each phase is fully isolated. Easy to retry a single phase. Log file provides handoff context.
**Cons**: No shared session state. Each phase must re-read context from files.

### Strategy 3: Direct Implementation (Current Approach)

Single session implements everything sequentially with per-phase logging:

```bash
claude -p "Implement the full plan directly..." \
  --dangerously-skip-permissions \
  --max-turns 30
```

**Pros**: Simplest. No configuration needed.
**Cons**: Context window exhaustion risk on large plans. If session dies, progress may be lost (mitigated by per-phase log writes).

## Recommendations

1. **For plans with 3+ phases or 10+ tasks**: Use Strategy 1 (subagent orchestration) or Strategy 2 (justfile chaining)
2. **For small plans (1-2 phases, <8 tasks)**: Strategy 3 (direct implementation) is fine
3. **Always write to the log file after each phase** regardless of strategy — this provides recovery state
4. **Always set `implement_complete` frontmatter** to `false` at start and `true` on completion — this provides a reliable completion check for the justfile
5. **Use `cargo check` after each task** in all strategies to catch compilation errors early
