# Topics Closeout Log

## 2026-07-02 — Permissions Checkpoint Package

- Started the ratified first topic, `permissions`, using `agent-permissions/` as the
  merged home.
- Kept the old `permissions/` docs in place as validation baselines.
- Extracted legacy coverage from the six old docs through focused subagents. The main
  gaps versus the current typed topic are rule grammar, permission entities, sandboxing,
  folder/project trust, managed policy, MCP-specific filters, non-interactive approval
  behavior, approval persistence, protected paths, and tool visibility versus approval.
- Widened `agent-permissions/_schema.yaml` with optional fields for those gaps so the
  existing nine docs remain valid until an approved update-mode fleet populates them.
- Modernized `agent-permissions/_agent-permissions.md` with the standard host config
  inspection grant note, the merged-topic scope, explicit PolicyEngine consumer framing,
  and frontmatter capture instructions for the widened schema.
- Revised `config_files` from a single `{ user, repo }` object to OS-scoped records
  (`os: macos|linux|windows|all`) so Windows-specific paths can be captured without
  prose-only exceptions. Existing docs were migrated mechanically with `os: all`.
- Replaced prose-only `precedence` with an ordered structured list
  (`source`, free-form `scope[]`, `merge_strategy`, `notes`). Existing docs were
  migrated conservatively with the prior prose preserved in `notes`; their broad
  `source` strings intentionally still contain the old ordering summary. The refresh
  fleet should split those broad records into one ordered record per source.
- Stopped before the checkpoint-gated pilot/fleet run. Next step is Ken review of the
  widened schema and prompt.
