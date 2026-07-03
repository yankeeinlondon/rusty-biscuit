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
- Modernized `agent-permissions/_fleet.md` with the standard host config
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
- Added optional `cli_zero_permissions` metadata and prompt questions for a future
  Claudine wrapper feature that starts a provider run with no permissions/tools via
  CLI-only session flags, then explicitly adds requested permissions without mutating
  the user's provider config.
- Left `env_vars[].effect` as prose for first-pass research, with a follow-up action
  after the refreshed fleet to inventory observed effects and introduce a typed
  effect category where the data supports it.
- Removed the standalone `env-vars` sidecar schema after review. Environment variables
  are now owned by the domain topics they affect, with `agent-cli` retaining only
  general CLI/runtime variables that do not belong to a narrower topic. The legacy
  `env-vars` notes remain as design input, not as a fleet target.
- Removed the old `env-vars/_env-vars.md` sequence stub so there is no standalone
  env-vars fleet driver left behind.
- Promoted `system-prompt` from legacy design research into a schema-enforced fleet
  topic feeding `SystemPromptSpec`. Added `_schema.yaml` and `_fleet.md` using
  the original Claude Code prompt as the seed, with explicit fields for append/replace
  support, config sources, prompt layers, agent/subagent prompt isolation, format
  recommendations, and Claudine delivery strategy.
- Promoted `acp` from legacy design/protocol research into a schema-enforced fleet
  topic feeding future Claudine ACP client/adapter work. Added `_schema.yaml` and
  `_fleet.md` using the original Claude Code ACP prompt as the seed, with explicit
  fields for launch modes, protocol versions, capabilities, reverse requests,
  permission/filesystem/terminal models, streaming, Rust client guidance, and
  compatibility quirks.
- Reworked the `mcp` topic from config-file inventory into a protocol-aware provider
  comparison. The schema and prompt now capture transports, tools/resources/prompts,
  roots/sampling/elicitation, authorization, tool exposure, resource exposure, prompt
  exposure, runtime injection, sync behavior, and security posture so provider
  variances are not collapsed into generic "MCP server config" notes.
- Reworked the `resume` topic from a thin session-ID prompt into a session-continuation
  research model. The schema and prompt now distinguish continue-latest, explicit
  handle resume, interactive pickers, non-interactive follow-up prompts, transcript
  replay, server/live-process continuation, branch/checkpoint behavior, restored state,
  lookup scope, interruption recovery, and human-in-the-loop continuation.
- Reworked the `non-interactive-sessions` prompt and schema from an older prose-heavy
  structured-output survey into a wrapper/parser metadata contract. The topic now asks
  for entry points, output formats, schema sources, stdio/framing contracts, stream
  discriminators and ordering, session metadata, event families, tool visibility,
  completion semantics, blocking behavior, subagent visibility, normalized signal
  detectability, headless constraints, and a recommended Claudine strategy. The prompt
  now includes concrete calibration patterns from existing Claudine parser knowledge
  (Claude/Codex/Gemini/Qwen/Kimi/OpenCode/Goose/Roo) so fleet answers must discuss
  real commands, stream envelopes, event fields, parser footguns, and blocking behavior
  instead of generic "JSON output" support. Clarified the research output model:
  provider docs should be written as explanatory prose first, with frontmatter serving
  as a distilled machine-readable index of key operational facts rather than replacing
  the narrative analysis.
- Standardized schema-backed fleet sequence drivers on `_fleet.md` next to `_schema.yaml`
  in every topic directory. Each fleet driver now defines a `file` frontmatter property
  with the full target research filepath, uses `file_exists(file)` rather than
  `state.file` path checks, and emits positive lifecycle progress on successful
  same-day updates in addition to stale-success failure checks.
- Stopped before the checkpoint-gated pilot/fleet run. Next step is Ken review of the
  widened schema and prompt.
