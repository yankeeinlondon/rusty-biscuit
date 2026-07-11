# D-1.5 Checkpoint Package (2026-07-05)

> **Status (2026-07-05):** ratified. Part A was ratified by Ken exactly as proposed
> (2026-07-05) and every disposition has been applied to `summary-triage.md`.
> Part B was ruled (Ken, 2026-07-05): **B1** — retire the field; **B2** — ruling
> superseded by Ken's memory-system context, the recommendation is being reworked;
> **B3** — adopt the research vocabulary, as a future-facing observational field.
> B1/B3 + the hooks-report `unmapped_native_events` field were EXECUTED 2026-07-05
> (registry 42 = 10 roster / 10 research / 22 facts; gen check clean). B2's
> reworked recommendation (memory research topic, field unchanged) is recorded in
> its section below. Section D (schema-v2 items 1c/1d) was re-presented in
> digestible form 2026-07-05; Ken's decision pending.

## A. Disposition proposals (untriaged `summary-triage.md` items)

Legend: `[I]` implement now · `[S]` scheduled (named work item) · `[W]` won't-do.
Items already marked in the file are not repeated. Where several items share one
natural work item, the proposal names the bundle — piecemeal edits would churn the
same module repeatedly.

### MCP

| # | Item (abbrev.) | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Goose/Kimi/Qwen native MCP, no Claudine path | [S] | Real feature work → "MCP rollout round 2" bundle; orthogonal to the metadata pipeline |
| 2 | Claude `--mcp-config` runtime injector | [S] | Same MCP bundle; per-run injection is cleaner than shadow-home for claude |
| 3 | Plugin-declared MCP servers as 4th discovery source | [S] | Behind plugin-extraction work; import candidates with credential stripping, catalog stays authoritative |
| 4 | ACP `session/new` mcpServers injection | [S] | Behind the ACP adoption track; capability-gate per provider |

### Agent Logging

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Provider-log evidence adapters (federated `claudine logs`) | [S] | Own track; large, well-specced by the summary |
| 3 | Shared path/volatility inventory | [S] | Fold INTO the evidence-adapter item — one inventory, consumers: shadow-home skip list + adapters |

### Agent Permissions

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Goose wrapper default posture (`GOOSE_MODE=auto` ≈ YOLO) | [S] | Posture flip interacts with approval transport; batch with the permissions/PolicyEngine six-axis round rather than a risky standalone flip |
| 2 | OpenCode `--auto` = auto-reply-once | [S] | Same permissions round; low-cost model correction, batch it |
| 3 | Kimi precise approval transport is ACP-only | [S] | Recorded as input to the ACP adoption disposition |

### System Prompt

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Kimi `--rsp` (Kimi Code has no per-session replace) | [S] | "System-prompt facts truth-up" bundle: version-scoped `SystemPromptSpec` corrections + remediation-bearing wrapper errors |
| 2 | Goose `--rsp` (no clean per-launch replace) | [S] | Same bundle |
| 3 | Goose `--asp` (`goose run --system` only) | [S] | Same bundle |
| 4 | `OPENCODE_CONFIG_CONTENT` merge contract | [I]✓ | **Already implemented in Phase C**: `wrapper_mcp.rs` merge + `wrapper_stages.rs` stage 10.5 YOLO-overlay merge; user-supplied value preserved. Mark done with pointer |
| 5 | Qwen/Gemini inline-only flags; `*_SYSTEM_MD` missing-file FATAL | [S] | Same truth-up bundle |
| 6 | Kimi legacy Jinja2 `StrictUndefined` | [W] | Legacy Python kimi-cli (1.x) only; Claudine wraps Kimi Code (0.x) — version-scoped out by the product split |
| 7 | Docs: `--rsp` = "replace provider base prompt where possible" | [I] | Cheap docs fix; execute in the truth-up bundle's doc pass |

### Non-Interactive Sessions

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Kimi Wire peer obligations (`--afk` posture) | [I] | Largely closed by the step-0 wire work (all four request types answered by policy in `wiring/dispatch.rs`); remaining: a posture verification note + fixtures in the kimi wire follow-up |
| 2 | OpenCode promoted stderr is CONTRACT | [S] | Lands with Phase E record grammar (promoted-structured vs diagnostic stderr is a named Phase E deliverable) |
| 3 | Goose invocation hardening (`--quiet`, `--name`, taint) | [S] | Goose-hardening work item; the error-then-`complete` taint rule itself lands in the Phase E migration map |
| 4 | Qwen init variants + exit codes 53/55/130 | [S] | Phase E declarative-record candidates (already named in plan §E.4) |
| 5 | Codex exec JSONL launch-metadata gap | [S] | Fold into item 6 |
| 6 | Cross-provider wrapper-metadata capture layer | [S] | Own work item — one systematic capability, not eight patches |

### Hooks

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 2 | `after_compact` canonical event | [S] | Gates Codex-hooks `[S]` phase 2; catalog-types enum + event_mapping regen; schedule immediately before that work |
| 3 | Tool-selection phase unmappable | [W] | Fires before a concrete tool call exists; no honest canonical mapping — document as unmapped in the hooks topic |
| 4 | OpenCode blocking authority = plugin hooks, not event bus | [S] | Verify + bridge in the hooks round (OpenCode registration writer) |
| 5 | Goose SubagentStart/Stop not emitted | [I] | Cheap facts/event_mapping correction: never register/claim them |
| 6 | Goose hook registration = Open Plugin authoring | [S] | Goose registration-writer work item (Windows Git Bash/MSYS2 caveat noted) |
| 7 | Kimi wire-hook channel last mile | [I]✓ | **Largely implemented**: wire mode already dispatches `HookRequest` through the canonical pipeline (`wiring/dispatch.rs::dispatch_hook_request`); remaining gap is registration/docs only |
| 8 | Registration-health kill-switch detection | [S] | Already scheduled inside Codex-hooks phase 2; extend cross-provider afterward |
| 9 | `human_in_the_loop` dispatchability review | [S] | Small analysis in the hooks round |

### Session Resumption

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 2 | Handle capture + persistence | [S] | Anchor item of a "resume round 2" bundle — THE prerequisite |
| 3 | Resume doesn't restore launch env | [S] | Resume round 2 (record + reapply launch metadata) |
| 4 | Serialize resume attempts per session id | [S] | Resume round 2 |
| 5 | Kimi `KIMI_CODE_HOME` version-scope facts | [I] | Fold into this session's kimi version-scoping pass (step 0 already establishes the axis) |
| 6 | Goose `--resume` is global | [I]✓ | **Already implemented** by the post-C resume ruling: `goose.rs::build_resume_args` pins explicit `--session-id`. Mark done with pointer |
| 7 | Record non-resumable launch conditions | [S] | Resume round 2 |
| 8 | Two-step API resume paths | [S] | Durable-HITL track after resume round 2 (couples with ACP/server surfaces) |
| 9 | Claude durable HITL `PreToolUse` defer | [S] | Resume round 2 — but reserve `human_input_requested`/`session_resumable` in the Phase E taxonomy NOW (cheap while designing `SignalEvent`) |

### ACP

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Adoption disposition (hybrid) | [S] | Ratify the summary's hybrid as standing direction: wrappers stay baseline; ACP provider-by-provider as typed streaming + permission backend; capability-gate reverse-request handlers; record streams into reporting before ACP replaces execution |
| 2 | Enforcement-plane per delegation TIER | [I] | Absorbed into ruling B3's field-semantics notes + facts |
| 3 | Adapter dependency drift (npx adapters; Pi two lines) | [S] | With ACP adoption — track adapter identity/version in facts when integration lands |
| 4 | Gemini `--experimental-acp` deprecated | [I] | Cheap: purge stale references at next touch |
| 5 | `session/update` as signal `source` | [I] | Lands in Phase E record grammar (plan §E.4 already mandates `source: acp`) |

### Skills / Slash Commands / Plugins

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1–7, 9, 10 | Generic-root projection · precedence/shadowing · Linked-But-Degraded · inventory-only classes · parse-and-emit conversion · no-native-surface presentation · grammar rewrite map · plugin extraction w/ provenance · plugin-bundle distribution | [S] | ONE consolidated **"linker v2" round** — all touch the linking module + the `resource_support` graduation; nine piecemeal passes would churn the same code |
| 11 | Runtime-executable plugin surfaces as linking exclusions | [I] | Cheap safety rule: codify the exclusion list now |

### Subagents

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Observation adapters (normalized child record) | [S] | Couples with the logging evidence-adapter track (same stores/readers) |
| 2 | Observability-strength gating | [S] | Same item — the gating enum is its design core |
| 3 | Kimi subagent linking version-sensitive | [I] | Version-scoping note in this session's kimi facts pass; linking exclusion for the legacy YAML surface |
| 4 | Compatibility-origin tagging | [S] | Linker v2 round |
| 5 | Codex agent files = config layers | [S] | Linker v2 round (translation, never equivalence) |
| 6 | Goose frontmatter `model` metadata-only | [S] | Linker v2 round (don't project model intent) |

### Local Runners / Model Config

| # | Item | Proposal | Rationale |
| --- | --- | --- | --- |
| 1 | Typed runner detection (sniff) | [S] | Sniff-side work item (separate package area); confidence ladder + identify-by-response-marker |
| 2 | Bridge-config generation | [S] | Own feature, after runner detection |
| 3 | Partial-compatibility warnings | [S] | With bridge-config |
| 4 | Gemini cannot bridge without proxy | [I] | Cheap doc note: exclude from bridge scope |
| 5 | OpenAI dialect split (chat vs responses) | [S] | With bridge-config (proxy-required detection) |
| 6 | Codex replacement-shaped catalog + reserved ids | [S] | With bridge-config (generation constraints) |
| 7 | Stale-manual-block detection | [S] | Phase F drift channel (plan already points there) |
| 8 | Qwen project-scope REPLACE trap | [I] | Cheap: note in model-config facts/docs as a config-generation guard |

**Net new scheduled bundles this table creates:** MCP rollout round 2 · evidence-adapter track (absorbs subagent observation) · permissions posture round (rides the six-axis work) · system-prompt facts truth-up · wrapper-metadata capture layer · hooks round (goose/opencode writers + reviews) · resume round 2 · ACP adoption track · linker v2 · local-runner bridge track (sniff detection → bridge generation).

## B. The three parked rulings

### B1. `session_locations` — what does the field mean?

Evidence (verified this session):

- **No runtime consumer.** The field appears only in the describe serialization and
  its test (`lib/src/provider/tests.rs:737`). Nothing reads it operationally.
- **The population is incoherent:** claude = the *grandparent directory* of its
  transcript surface; codex = an app log + shell-snapshots dir; goose = three log
  dirs/globs at a base path that **contradicts research** (facts
  `~/.local/state/goose/logs/` vs observed macOS
  `~/Library/Application Support/Block/goose/state/logs/`); qwen = orphaned
  relative `logs/openai` appearing in no research surface; gemini/kimi/opencode =
  `[]` despite rich surfaces.
- No single agent-logging `role` filter reproduces any provider's list.

Options:

- **(a) Retire the field** (shape change: ProviderInfo field delete + registry +
  emit.rs + regen all 7 + catalog.json in one change). `session_log_paths`
  (research-fed, clean `role == session_transcript` rule) already carries the
  machine-grade answer; the evidence-adapter backlog item owns the full inventory.
- **(b) Redefine as human-curated "session data roots"** documentation facts;
  hand-fix the six incoherent values; permanent facts.
- **(c) Derive from agent-logging surfaces** via a multi-role filter +
  directory-truncation coercion; research-fed.

**Recommendation: (a).** No consumer + incoherent population = a speculative field
whose two real jobs are owned elsewhere. (b) is the fallback if a human-facing
"where does it keep stuff" hint should survive in describe output.

### B2. `memory_files` — extraction rule

Evidence:

- **Load-bearing consumer:** `cli/src/commands/wrap/harness_orch/prompt.rs::find_wrapper_harness_source`
  walks `memory_files` (repo-relative entries) to locate the wrapper harness seed.
  Over-selection is a behavior change, not a data cleanup.
- The candidate rule (`mode == append ∧ format == markdown` over system-prompt
  `config_sources`) reproduces the current list for **zero** providers — it is a
  strict superset everywhere, pathological for kimi (**20 selected vs 1 current**;
  research marks every skills dir `mode: append`). `mode == append` alone is worse
  (opencode's jsonc records join).
- One genuine conflict: facts say `.claude/CLAUDE.local.md`; research says
  `./CLAUDE.local.md` (repo root).

Options:

- **(a) Keep facts permanently, curated** (the `allowed_env_keys` pattern);
  `config_sources` is review evidence at generate time.
- **(b) Schema v2 addition:** give `config_sources` records a
  `role: memory | skills | config | other` key; the rule becomes `role == memory`;
  graduates at the system-prompt v2 refresh (folds into approvals item **4a**,
  already recommended "approve, sequence last").
- **(c) Adopt the mode/format rule + per-provider excludes** — rejected; kimi shows
  it cannot converge.

**Recommendation: (a) now, (b) as the graduation trigger** rolled into schema-v2
item 4a. Separately: the `CLAUDE.local.md` location conflict needs a human check
(which path does Claude Code actually read?) — drift finding, not auto-corrected.

> **Reworked recommendation (2026-07-05), after Ken's memory-system context.**
> Ken's framing: Claudine will add its own memory system; fleet research should
> capture what each provider already offers, as design input — whether any of it
> maps into codegen is unknowable until Claudine's memory design is formalized.
> That framing decouples three things the original question tangled:
>
> 1. **The catalog field `memory_files` stays facts-fed and curated, unchanged.**
>    It answers a narrow *wrapper* question — "which Markdown context files does
>    the provider auto-load" — and is load-bearing (harness-seed discovery). It is
>    not a memory model and should never be derived from system-prompt research.
>    (Optional future rename to `context_files` to stop overloading "memory";
>    deferred — churn without payoff today.)
> 2. **Provider memory-system knowledge becomes a dedicated `memory` research
>    topic** (standard sidecar + `_fleet.md` pattern) — the landscape survey the
>    memory design process will consume. Schema sketch: memory kinds offered
>    (model-written auto-memory vs user-curated files vs session-scoped), storage
>    locations + formats (Claude MEMORY.md dirs, Codex `memories_1.sqlite`, Goose
>    memory extension, …), write triggers (who writes, when), load timing, scope
>    (global/project/session), user controls (enable/disable/edit/clear), system-
>    prompt interaction, portability. Deliberately NOT wired to codegen.
> 3. **Timing:** authoring the sidecar/prompt is cheap and can happen anytime;
>    the fleet RUN is scheduled by the topics-closeout track as the named
>    prerequisite of the memory design process — not before. After the design
>    formalizes, revisit which keys (if any) graduate into the catalog.
>
> The old extraction-rule question (mode==append ∧ format==markdown) dissolves —
> nothing derives `memory_files` from research. The `CLAUDE.local.md` path
> conflict remains a one-line human check at the next facts touch.

### B3. `acp` — vocabulary reconciliation

Evidence (three-way comparison, 7 catalog providers):

| Provider | constant `server_mode` | sidecar `support` | summary | Verdict |
| --- | --- | --- | --- | --- |
| claude | `not_supported` | `adapter` | Adapter | MISMATCH |
| codex | `not_supported` | `adapter` | Adapter | MISMATCH |
| gemini | `not_supported` | `native` | Native | MISMATCH |
| goose | `native` | `native` | Native | match |
| kimi | `available_via_wire_proxy` | `native` | Native | MISMATCH |
| opencode | `not_supported` | `native` | Native | MISMATCH |
| qwen | `not_supported` | `native` | Native | MISMATCH |

Root cause: the constant answers *"does Claudine have an ACP path today"* while
research answers *"does the provider speak ACP"*. Kimi's
`available_via_wire_proxy` is a Claudine integration detail — kimi has genuine
native `kimi acp`. Also noted: `events_via_acp` naming drift (goose canonical
`request_permission` vs kimi wire-envelope `approval_request`).

Options:

- **(a) Field means PROVIDER capability.** Adopt the sidecar vocabulary
  (`native / adapter / partial / none / unknown`) for `server_mode`; graduate
  research-fed (mechanical re-point + regen); retire `available_via_wire_proxy`
  (the wire-proxy integration fact already lives in `event_mapping` support
  kinds); `client_supported` / `events_via_acp` stay Claudine-owned facts.
- **(b) Two fields:** keep the constant as Claudine-integration status (facts) and
  add a separate research-fed provider-capability field.
- **(c) Status quo.**

**Recommendation: (a).** One honest field, unblocks the mechanical graduation,
matches the hybrid-adoption framing. Note this is a **shape change** (`AcpSupport`
enum in `lib/src/provider/acp.rs` + emit.rs + regen in one change).

## D. Permissions enum derivation (schema-v2 items 1c/1d)

Corpus: all 9 `agent-permissions` docs (fleet of 2026-07-03). 106 `env_vars`
records, 74 `precedence` records carrying 93 distinct scope tokens. Full
inventories with verbatim values and per-record mappings are in the session
transcript; the ratification-relevant summary:

### 1c. `env_vars[].effect` → ADD `effect_category`, KEEP the prose

Proposed 15-variant enum (counts): `sandbox_control` 22 · `none` 17 ·
`state_home_relocation` 10 · `config_path_override` 10 · `tool_surface` 8 ·
`security_hardening` 7 · `customization_lockdown` 6 · `credential` 6 ·
`threat_detection` 5 · `policy_overlay` 3 · `config_injection` 3 ·
`config_source_toggle` 3 · `approval_mode` 2 · `network_control` 2 ·
`workspace_trust` 1 · plus `other` (2 unmappable: `CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST`,
`GEMINI_CLI` outbound marker).

- Optional collapse to 11 variants: merge `state_home_relocation`→`config_path_override`,
  `policy_overlay`→`config_injection`, `threat_detection`→`security_hardening`.
- **Keep prose**: it carries value sets, version gates, precedence interactions,
  platform constraints, and hardening *direction* (`GOOSE_DISABLE_KEYRING` /
  `QWEN_TLS_INSECURE` *weaken*) that a category cannot.
- Negative finding: **no provider exposes a YOLO/disable-enforcement env var** —
  do not add a speculative variant.
- Keep the 17 `none` records (verified negative results).

### 1d. `precedence[].scope` → REPLACE free-form with an enum

Values are already bare tokens (zero prose payload); five tokens (`mcp`, `rules`,
`sandbox`, `tool_visibility`, `approval_mode`) cover half of all occurrences.
Proposed 18-variant enum: `rules`, `mcp`, `approval_mode`, `sandbox`,
`tool_visibility`, `general_config`, `config_loading`, `agents`, `extensions`,
`hooks`, `skills`, `slash_commands`, `customization_resources`,
`security_controls`, `trust`, `workspace`, `provider_model`, `other`.

- All 93 tokens map; deliberately aligned with the ratified `permission_entities`
  vocabulary so the two enums stay translatable.
- One token collision resolved per-record: claude/cli `tools` (a visibility flag →
  `tool_visibility`) vs kilo/config_directories `tools` (tool definitions →
  `customization_resources`).
- Pi's `extension_hooks` record (runtime interception points) is the argument for
  keeping `other` + `notes`.
- Migration: tightening `scope` invalidates all 74 records until a mechanical
  normalization pass rewrites them; adding `effect_category` should land
  **optional-until-next-refresh** (the established widening pattern).
