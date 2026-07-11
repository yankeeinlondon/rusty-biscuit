# Plan: Research Topics Closeout

> **Goal:** every research topic in the provider-metadata workstream is schema-enforced,
> roster-complete, evaluated, and (where it earns one) distilled into a skill — closing
> the gap the spec describes between "topics today" and "typed contracts feeding codegen".
>
> **Builds on:** the proven pattern from `local-runner-plan.md` and
> `model-config-plan.md` (both executed 2026-07-02). This plan industrializes that
> pattern across the remaining topics; it does not reinvent it.

## Current state (inventoried 2026-07-02; scope corrections from Ken applied)

| Topic | Schema | Prompt | Docs | State |
| --- | --- | --- | --- | --- |
| `agent-logging` | yes | current | 10 | **done** (schema-enforced) |
| `agent-models` | yes | current | 9 | **done** |
| `model-config` | yes | current (frozen) | 9 | **done** (refreshed, skill exists) |
| `local_runners` | yes | current (frozen) | 5 | **done** (skill exists) |
| `permissions` *(merged)* | NO | **none** | — | `agent-permissions/` (9 docs, schema) and legacy `permissions/` (6 docs) merge into ONE topic; **no sequence defined yet** — no prompt, no schema closure |
| `usage` | NO | old pattern | **0** | prompt only — full authoring run |
| `agent-cli` | NO | old pattern | 8 | old-generation docs (`claude-code.md`, `gemini-cli.md`, `roo-code.md`); missing pi/kilo |
| `non-interactive-sessions` | NO | old (`_fleet.md`) | 8+2 | old-generation docs (`qwen.md`); missing pi/kilo; 2 non-roster extras (`codex-gotchas.md`, `codex-tools-and-events.md`) |
| `system-prompt` | yes | current | 9 legacy | **fleet topic**; legacy docs exist, with two stale resume-prompt misfires to treat as evaluator inputs only |
| `acp` | yes | current | 6 provider docs + reference docs | **fleet topic**; legacy provider docs and protocol/library docs exist |
| `env-vars` | — | legacy notes only | 1 | **removed as standalone topic**; domain topics own their own env vars |
| `hooks` | NO | none | 8 legacy | **fleet topic** (per prior discussion); legacy per-provider docs exist (old-roster names) |
| `skills` | NO | none | — | **fleet topic**; legacy input: `cross-referencing/` (8 docs) + `skillsets/` (3 docs) |
| `slash-commands` | NO | none | — | **fleet topic**; legacy input: `cross-referencing/` |
| `subagents` | NO | none | — | **fleet topic**; legacy input: `cross-referencing/` |
| *(new)* `resume` | — | — | legacy dir (9 docs, no prompt) | planned per spec |
| *(new)* `mcp` | — | — | legacy dir (6 docs, no prompt) | planned per spec ("config/security/events") |
| `streaming` | — | — | — | **removed as standalone topic**; structured stream contracts belong to `non-interactive-sessions` |

**Old research is a validation asset, not a migration burden.** Old-generation files
(`claude-code.md`, `gemini-cli.md`, `qwen.md`, the legacy `hooks/`, `cross-referencing/`,
`permissions/`, `resume/`, `mcp/` docs) are kept in place, unrenamed. New fleet runs
write fresh roster-named docs (`claude.md`, `gemini.md`, `qwen-cli.md`, …); evaluation
subagents then cross-check the new doc against the old one — divergences are either
genuine platform drift (belongs in the new doc) or a regression in the new research
(fix it). Fresh mode means `changes: []` and no changelog requirement on first runs.

Remaining legacy one-off dirs (`protect`, `rendezvous`,
`agent-designs`, `hook-designs`, `separating-phases`) are design research, not
fleet topics; they stay as-is unless Checkpoint 0 says otherwise.

## Context the executing agent needs

- **Use the `claudine` agent skill** (load first). Pattern authority: parent spec
  `spec.md` ("Research topics as typed contracts", "Research pipeline") and the two
  executed sibling plans. Do not invent a parallel format.
- **Run invocation:** `claudine sequence <topic prompt> --yolo`, in the background;
  steps take ~5–15 min; a 9-step fleet is ~1–2.5 h wall-clock. This plan carries
  **~12 fleets** — expect multi-day elapsed time; the pipeline discipline below is what
  keeps it moving.
- **Model directive:** `agent: opencode`, `model: kimi-for-coding/k2p7`. If capped
  (`Weekly/Monthly Limit Exhausted`), switch the sequence frontmatter to
  `model: minimax/MiniMax-M3` and resume (initialize skip makes re-runs clean). Verify
  the first `llm_call_start` names the requested model; kill and relaunch if OpenCode
  substituted its config default.
- **Sequence frontmatter must carry** the standard stacks (copy from
  `model-config/_fleet.md`): `update:` gate, initialize same-day-skip,
  `success` last_updated verification, `$schema: ./_schema.yaml` instruction, evidence
  requirement (`{{state.user_dir}}` inspection), `::file @prompts/make-it-markdown.md`
  output, `md schema validate` exit criterion.
- **Skip-gate reality check:** the same-day skip only matters for same-day re-runs
  after a prompt fix — force those by backdating `last_updated` (never by deleting;
  deletion loses update-mode changelogs on refresh runs).

### Orchestration rules (context-window discipline)

The orchestrating agent's context is the scarce resource. Hard rules, proven in the
model-config execution:

1. **Never read research bodies or long web pages into the orchestrator.** Research
   happens inside `claudine sequence` children (free to the orchestrator); evaluation
   happens in parallel subagents returning structured verdicts (~1–2k tokens each).
2. **Frontmatter-only extraction** in the orchestrator via `awk`/`yq` one-liners when
   compact facts are needed (as done for the model-config skill tables).
3. **One evaluation subagent per produced doc**, given: the doc path, the legacy
   counterpart path (when one exists) for cross-validation, the six-check template
   (schema-valid; concrete answers not "varies"; evidence/URL per claim;
   body/frontmatter agreement; no misframing for the topic's known failure class;
   old-vs-new divergences classified as drift or regression), and the fixed verdict
   shape (`acceptable | acceptable_via_targeted_edit | re_run_required` + suggested
   edits). The orchestrator applies surgical edits itself; only `re_run_required`
   triggers a temp-pilot-roster re-run.
4. **Schema drafting fans out too:** one discovery subagent per legacy doc (or per
   question cluster for greenfield topics), returning attribute inventories — the
   orchestrator only merges.
5. **Pipeline topics, don't parallelize fleets.** One `claudine sequence` fleet at a
   time (they share the OpenCode/kimi quota); while topic N's fleet runs in the
   background, author topic N+1's schema + prompt. A capped quota mid-plan is the
   expected failure mode — the model-switch directive above is the recovery.

## Phase 0 — scope ratification — **CLOSED (approved by Ken, 2026-07-02)**

Ratified resolutions:

- **(a) Ordering** — the default below stands.
- **(b) MCP** — one sectioned topic (`mcp/`), not a three-way split.
- **(c) Merged `permissions` home** — lives in `agent-permissions/` (widened schema,
  existing 9 docs run in update mode); assumed default, revisable at the permissions
  Checkpoint 2.x if the schema widening argues otherwise.
- **(d) Design-research dirs** (`protect`, `rendezvous`, `agent-designs`,
  `hook-designs`, `separating-phases`) — left as-is.
- **(e) Skills** — topics with a direct claudine consumer earn one:
  permissions/PolicyEngine, hooks/adapters, skills+slash-commands+subagents/linking,
  mcp/mcp module, non-interactive-sessions/stream parser contract,
  resume/lifecycle resume action.

Ratified ordering (consumer value first; retrofit before greenfield):
`permissions` (merged) → `hooks` → `agent-cli` → `non-interactive-sessions` →
`skills` → `slash-commands` → `subagents` → `usage` → `system-prompt` → `acp` → `resume` →
`mcp`.

## Phase 1 — retrofit trio (repeat per topic: agent-cli, non-interactive-sessions, usage)

1. **Bootstrap the schema** from existing docs: `md schema detect` over the topic's
   old-generation provider docs (spec Phase 0 explicitly plans this), then
   hand-tighten — enums instead of free strings, `required` markers, quoted
   inline-object literals, root `$schema:` key. Where the topic feeds catalog fields,
   design backwards from those fields (spec: "Research topics as typed contracts").
   `usage` has no docs — draft its schema from the prompt's question list instead,
   via a schema-design subagent.
2. **Modernize the prompt** to the current pattern (sequence frontmatter stacks,
   `$schema` instruction, per-field capture instructions mirroring the schema, evidence
   requirement, Sources section). Keep the sequence driver named `_fleet.md`.
   Fold lessons from the model-config defect: ask capability questions
   ("which standards / mechanisms exist"), never presence questions ("does X support
   Y"), and add topic-specific anti-pattern lines where the old docs show a failure
   class (an inspection subagent per topic reports candidate failure classes from the
   old docs' frontmatter and section headings — not full-body reads).
3. **Leave old-generation files in place** (`claude-code.md`, `gemini-cli.md`,
   `qwen.md`, `roo-code.md`, and the non-roster extras). The fleet writes fresh
   roster-named docs alongside them; evaluators use the old files as cross-validation
   baselines (rule 3 above). No `git mv`, no deletion.
4. **Validate:** sidecar positive + negative via `md schema validate` on temp docs
   (scratchpad, `--schema` flag); `claudine sequence --dry-run` ×9.
5. > **HITL CHECKPOINT 1.x (Ken):** per-topic schema + prompt review before any fleet
   > run — schema changes are catalog-affecting contract changes (spec discipline).
   > Batch: present all three retrofit schemas in one checkpoint if they are ready
   > together; otherwise one checkpoint per topic is fine.
6. **Pilot two providers** (one vendor platform + one open provider — the
   model-config pilot showed the two families fail differently) via the temp-pilot
   roster technique; evaluate with subagents; adjust; then **full fleet** (9 steps,
   background). Pilots re-run in the fleet only if the prompt materially changed.
7. **Evaluate all docs** (parallel subagents, verdict shape above); apply targeted
   edits; re-run only where required. Exit per topic: 9/9 schema-valid, verdicts
   green (including old-vs-new divergence classification).

## Phase 2 — merged + fleet-ified + new topics

Repeat per topic: `permissions` (merged), `hooks`, `skills`, `slash-commands`,
`subagents`, `system-prompt`, `acp`, `resume`, `mcp`.

1. **Attribute discovery fan-out.** Inputs differ by topic:
   - `permissions`: the existing `agent-permissions/_schema.yaml` + its 9 docs'
     frontmatter (already typed) PLUS one subagent per legacy `permissions/` doc
     extracting the questions the current schema does not cover. The merged schema is
     a widening of the proven one, not a restart.
   - `hooks`: one subagent per legacy `hooks/` doc (payloads, return types, event
     names) — these feed claudine's `adapters`/`events`/hook registration, so design
     the schema backwards from those consumers.
   - `skills` / `slash-commands` / `subagents`: one subagent per legacy
     `cross-referencing/` doc (plus `skillsets/` for skills), splitting its combined
     coverage into the three topic-specific attribute sets — these feed the `linking`
     module's portability classification.
   - `system-prompt`: existing `system-prompt/` docs seed the questions, but
     evaluator subagents must flag the two stale resume-prompt misfires as legacy
     defects rather than evidence.
   - `acp`: existing provider docs seed provider-specific questions; `what-is-acp`,
     `json-rpc`, `rust-crates`, `typescript-libraries`, and `who-supports-acp` remain
     protocol/library reference inputs, not per-provider fleet outputs.
   - `resume` / `mcp`: one subagent per legacy doc where dirs exist;
     web-enabled question-cluster subagents for gaps.
   Synthesize the attribute inventory + draft `_schema.yaml` in the orchestrator.
2. **Author the prompt** (`_<topic>.md`) following the reference pattern; state the
   claudine consumer explicitly in the Scope section (PolicyEngine, adapters, linking,
   SystemPromptSpec, ACP client/adapter integration, mcp module, lifecycle resume
   capability matrix). Do not recreate a standalone streaming topic; structured
   stream protocol details are part of `non-interactive-sessions`.
3. Validate as in Phase 1 step 4.
4. > **HITL CHECKPOINT 2.x (Ken):** per-topic attribute inventory + schema + prompt
   > approval before the fleet run. For `permissions`, this checkpoint also ratifies
   > the schema widening against the existing agent-permissions contract.
5. Pilot → evaluate → fleet → evaluate → targeted edits, exactly as Phase 1 steps 6–7.
   For `permissions`, existing agent-permissions docs run in **update mode** (changelog
   entries expected); all other topics produce fresh docs validated against their
   legacy counterparts.

## Phase 3 — skills + map maintenance (after each topic, not at the end)

1. Where Checkpoint 0 approved a skill: create `.claude/skills/<topic>/` (SKILL.md
   <200 lines, activation-focused description, cross-provider comparison tables
   distilled from frontmatter; reference doc(s) for per-provider depth). Stamp
   `hash:` via `md hash --save`.
2. Update the `claudine` skill research section: move the topic into the live list
   with a 2–4 line summary + skill pointer; regenerate its hash. The skill's "Hooks
   Research" and "Cross-referencing Research" link sections get superseded-by notes
   once the corresponding fleet topics land. Keep the map/territory split — the
   claudine skill carries pointers only.
3. Append a short per-topic outcome note (prompt iterations, verdicts, old-vs-new
   divergences worth remembering, remaining gaps) to
   `claudine/features/2026-07-02-provider-metadata/topics-closeout-log.md` — one
   running log for this plan, in the style of `model-config-refresh.md`.

## Phase 4 — closeout

1. Final sweep (orchestrator, cheap): every topic dir has `_schema.yaml` +
   `_<topic>.md`; `md schema validate` green across all topic docs; claudine skill
   research section lists every live topic; log complete.
2. > **HITL CHECKPOINT 3 (Ken):** closeout review — present the per-topic outcome log
   > and the updated topic table; decide follow-on work (e.g. the spec's Phase-0
   > generator work now that every topic is a typed contract).

## Done when

Every topic in the ratified roster — the retrofit trio, the merged `permissions`, the
four resource topics (`hooks`, `skills`, `slash-commands`, `subagents`), and the new
set (`system-prompt`, `acp`, `resume`, `mcp`) — is schema-enforced (`_schema.yaml` +
`$schema:` references), roster-complete (9 providers, current filenames, old research
retained as validation baselines), fleet-run with green evaluation verdicts,
distilled into skills where approved, indexed in the `claudine` skill, and logged in
`topics-closeout-log.md` — leaving the spec's "typed contracts" precondition fully
satisfied for the codegen phase.
