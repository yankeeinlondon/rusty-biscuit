# CLI Output Layout

The claudine CLI produces structured output in a fixed section order.
Each section is visually separated from the next by a blank line.
New reporting code **must** place its output within the correct section
and preserve blank-line separators between sections.

## Section Order

```text
┌─────────────────────────────────────────────────────────┐
│ 1. Execution line                                       │
│    Claudine › Claude  YOLO  Non-Interactive  'prompt…'  │
│                                                         │
│ 2. Environment Variables                                │
│    • ANTHROPIC_API_KEY                                   │
│    • AGENT_PARAMS=[...]                                  │
│    • ...                                                 │
│                                                         │
│ 3. Info / Warning messages                              │
│    - Info: potentially dangerous ENV variables removed…  │
│    ⚠ Warning: …                                         │
│                                                         │
│ 4. Validation checkpoints  (frontmatter-prompt only)    │
│    ✓ validated that agent has read and write permissions │
│    ✓ resolved the file reference to path/to/file.md     │
│    Prompt:                                              │
│    │ The prompt text …                                  │
│                                                         │
│ 5. ── separator (blank line) ──                         │
│                                                         │
│ 6. Session ID                                           │
│    - Claude session ID abc123def4 · model-name          │
│                                                         │
│ 7. Execution output (streamed thoughts, tool calls)     │
│    Let me first find …                                  │
│    …                                                    │
│                                                         │
│ 8. Post-execution metadata                              │
│    ✓ Claude agent completed successfully                │
│    ✓ 250s · 100 input tokens · …                        │
└─────────────────────────────────────────────────────────┘
```

## Rules

1. **Nothing prints before the execution line.** The execution line is
   the first thing the user sees. Validation steps that must run early
   (e.g., file permission checks) should fail-fast on error but defer
   their success messages to the reporting section.

2. **Blank lines separate sections.** Every major transition — env vars
   to info, info to validation, session ID to execution output — must
   have a blank line. Never place two sections back-to-back without
   visual separation.

3. **Group related items.** Validation checkpoints, file resolution, and
   the prompt blockquote are a single logical group. Info and warning
   messages are another group. Do not interleave items from different
   groups.

4. **Session ID gets its own breathing room.** The session ID line marks
   the start of live execution. A blank line after it separates the
   preamble from streamed output.

## Where This Lives in Code

| Section                | File                                  | Function / block              |
|------------------------|---------------------------------------|-------------------------------|
| Execution line         | `cli/src/commands/wrap/mod.rs`        | `log_wrapper_header()`        |
| Environment Variables  | `cli/src/output.rs`                   | `log_wrapper_env_details()`   |
| Info / Warning         | `cli/src/commands/wrap/mod.rs`        | info/warning loop (~line 984) |
| Validation + Prompt    | `cli/src/commands/wrap/mod.rs`        | frontmatter block (~line 1000)|
| Session ID             | `cli/src/commands/wrap/mod.rs`        | `emit_start_summary()`        |
| Post-execution         | `cli/src/commands/wrap/mod.rs`        | post-run reporting            |
