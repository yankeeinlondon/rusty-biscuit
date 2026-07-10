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
- Removed `streaming` as a standalone provider-research topic. The standalone prompt
  duplicated `non-interactive-sessions` and was too generic to produce useful parser
  facts. Structured stream selection, framing, event families, correlation metadata,
  terminal events, and parser caveats now belong to `non-interactive-sessions`; any
  future stream-specific topic should be fixture/test-case oriented rather than another
  general provider fleet.
- Stopped before the checkpoint-gated pilot/fleet run. Next step is Ken review of the
  widened schema and prompt.

## 2026-07-05 — Status correction (recorded by the provider-metadata track)

- The entry above is stale as a description of current state: the widened-schema
  permissions fleet **already ran on 2026-07-03** and is committed (`d0702ca93`,
  "refresh research fleets and topic docs for 7-provider lineup"). Evidence: all 9
  `agent-permissions/*.md` docs carry `last_updated: 2026-07-03` with the widened
  fields populated (`rule_model`, `permission_entities`, `sandbox`, `trust_and_admin`,
  `mcp_permissions`, `cli_zero_permissions`, …), and `precedence` was split into
  ordered per-source records as scheduled. The cross-provider permissions summary was
  generated the same day.
- Still-open follow-ups from the 2026-07-02 entry: derive a typed `env_vars.effect`
  category enum from the landed fleet data (effect is still prose, as designed for the
  first pass), and tighten `precedence.scope` to enums now that the fleet has shown
  the vocabulary.
- Downstream note: the provider-metadata track's Phase D consumed this topic's landed
  data through its 2026-07-04/05 checkpoint; the remaining permissions-fed catalog
  graduations (`yolo` typed switch sites, `cli_sensitive_axes` six-axis booleans)
  need a **schema v2** addition beyond this widening — proposals tracked in that
  track's docs.

## 2026-07-05 — `memory` topic authored (Ken's B2 ruling)

Authored the new schema-enforced `memory` research topic
(`docs/research/memory/{_schema.yaml,_fleet.md}`) per Ken's B2 ruling: a landscape
survey of what each provider already ships as "memory", as design input for a future
Claudine-owned memory system — deliberately NOT wired to catalog codegen. The existing
catalog field `memory_files` (context-file auto-loading — wrapper mechanics) is out of
scope and unchanged; that surface belongs to the system-prompt topic, and the fleet
driver instructs researchers to reference rather than duplicate it. Schema follows the
freeze-risk pattern (free-form strings where vocabulary is uncertain, enums only for
closed sets: `memory_kinds[].kind`, `storage[].os`/`scope`, `write_model[].writer`)
with prose fields for load model, user controls, system-prompt interaction, limits,
portability, and speculative `claudine_notes`. The fleet run is deferred to the
closeout track, scheduled when the memory design process starts.

## 2026-07-05 — agent-permissions schema-v2 items 1c/1d executed (provider-metadata track)

- Executed the ratified 1c/1d increments (approved by Ken, 2026-07-05) as a docs-only
  pass over `docs/research/agent-permissions/`. Item 1c: `effect_category` added to the
  sidecar's `env_vars` shape as an **optional** 16-variant enum beside the kept `effect`
  prose (prose stays authoritative for specifics; the category is the queryable index)
  and backfilled across all **106** env_vars records in the 9 provider docs. Item 1d:
  `precedence[].scope` tightened from free-form strings to the ratified 18-variant enum
  and every scope token normalized in place across all **74** precedence records
  (deduped per record, first-occurrence order preserved; the per-record `tools`
  collision resolved as claude/cli → `tool_visibility` and kilo/config_directories →
  `customization_resources`).
- `_schema.yaml` comments updated (the "keep free-form for the first pass" note is
  retired) and `_fleet.md` capture instructions now teach both vocabularies (`none` is
  a first-class `effect_category` for verified non-permission vars; `other` only when
  nothing fits; no invented scope tokens — overflow goes in `notes`).
- All 9 provider docs validate clean via `md schema validate` against the tightened
  sidecar. Refresh fleets are unaffected: the 1c addition is optional-until-refresh and
  the 1d values were normalized in place, so no fleet run is forced by this change.

## 2026-07-07 — Closeout-fleet completion reconciled (log↔reality drift)

This log recorded only the `permissions` fleet (2026-07-03) and the authored-but-unrun
`memory` topic, which read as "closeout barely started" — but that is a **stale log**,
not the real state. Verified against the filesystem + git today:

- **Every closeout topic in the ratified roster has run and is committed.** Each of
  `agent-cli`, `non-interactive-sessions`, `usage`, `permissions` (`agent-permissions`),
  `hooks`, `skills`, `slash-commands`, `subagents`, `system-prompt`, `acp`, `resume`,
  and `mcp` (plus `plugins` and `signals`) carries `_schema.yaml` + `_fleet.md` and is
  **roster-complete** — 9 provider docs including `pi.md` + `kilo.md` — all
  `last_updated: 2026-07-03` (signals refreshed 2026-07-06). Thirteen topic dirs carry a
  2026-07-03 `pi.md`. `local_runners` is complete at its own 5-runner roster (not a
  provider roster).
- Landed across the committed fleet-refresh commits (`d0702ca93` "refresh research
  fleets and topic docs for 7-provider lineup", plus the per-topic expansions
  `692e49e50` hooks kilo/pi/qwen-cli, `10361d86b`/`27bd41151` mcp, `3b0b98216`/`8703a1c6e`
  skills, `b35528168` hooks, …). The 9-provider research roster (pi + kilo included) ran
  ahead of the compiled enum exactly as designed; M-Kilo/M-Pi code graduation followed.
- **Only `memory` remains unrun**, by design (Ken's B2 ruling: authored as design input,
  deliberately not wired to codegen; deferred to when the memory design process starts).
- **Why the log looked empty:** the plan's Phase-3 step 3 ("append a per-topic outcome
  note after each topic") was not kept during the 2026-07-03 batch. This entry is the
  reconciliation; per-topic prompt-iteration/verdict notes were not captured at run time
  and are not reconstructed here.
- **Not a Checkpoint 3.** Phase 4's closeout review (present the per-topic outcome log +
  updated topic table; decide follow-on codegen work) is Ken-gated and remains open. This
  entry records that the **fleets** are done — the gating fact for M-Antigravity (H3),
  whose roster entry lands only after the closeout fleets finish — not that the closeout
  is formally signed off. Still-open follow-ups from earlier entries (typed
  `env_vars.effect` enum; `precedence.scope` already tightened in 1d) carry forward.
