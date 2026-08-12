# Plan: Fix and Complete the model_config Research

> **Goal:** correct the `model_config` topic's misrepresentation of local-model (and
> cross-cloud) support, re-establish the prompt as high quality, and end with
> consistent, schema-valid research across all 9 providers.
>
> **Depends on:** the local-runners research (see `local-runner-plan.md`) — it is the
> ground truth this plan validates against. Do not start Phases 2+ before that fleet is
> complete and approved.

## The defect being fixed

The 2026-07-02 model-config fleet (9/9 "succeeded", docs in
`claudine/docs/research/model-config/`) frames local-runner support as a property of the
*provider* having "native support" — e.g. the Claude Code doc claims **no native support
for local model runners**. That framing is wrong. Support is a property of **API-standard
bridging**: most runners expose an OpenAI-compatible endpoint (some also
Anthropic-compatible — oMLX does; Ollama does), so any provider that allows a base-URL
override can use them: Claude Code via `ANTHROPIC_BASE_URL`-style env, Codex via
`model_providers` in `config.toml`, OpenCode via `provider.<id>.npm` + `baseURL`, etc.
The same mechanism also covers using *other cloud vendors'* models on vendor platforms
(e.g. a non-OpenAI model on Codex) — the current prompt never asks about that either.

## Context the executing agent needs

- **Use the `claudine` agent skill** (load it first) — it documents the composition/
  sequence machinery, lifecycle stacks, and CLI conventions this plan assumes.
- Parent spec: `claudine/features/2026-07-02-provider-metadata/spec.md` ("Out-of-box vs
  user-configured models", "Model identity grammar"). Current topic files:
  `claudine/docs/research/model-config/{_fleet.md,_schema.yaml}` and 9 provider
  docs produced by the flawed prompt.
- Ground truth input: `claudine/docs/research/local_runners/*.md` frontmatter (ports,
  endpoint paths, API-standard support per runner) once that fleet lands.
- The research-pattern conventions, run invocation (`--yolo`), background execution, and
  **model directive** are identical to `local-runner-plan.md` — including: research runs
  use `agent: opencode`, `model: kimi-for-coding/k2p7`; if capped
  (`Weekly/Monthly Limit Exhausted`), **switch immediately to
  `model: minimax/MiniMax-M3`**; and verify the first `llm_call_start` names the
  requested model (OpenCode has transiently substituted the config default — kill and
  relaunch if so).
- Context discipline: evaluate documents with parallel subagents returning structured
  verdicts; never read all 9 research bodies into the orchestrator's context.

## Phase 1 — schema + prompt revision

1. **Schema (`_schema.yaml`) changes:**
   - Replace `local_runners[].supported: enum(native,openai_compatible,unsupported)`
     with a bridging-aware enum, e.g.
     `integration: enum(first_class, base_url_override, proxy_required, unsupported)` —
     `first_class` = the provider ships explicit runner integration;
     `base_url_override` = works by pointing the provider's OpenAI/Anthropic-compatible
     client at the runner; `proxy_required` = needs a translation shim.
   - Add a `cloud_bridge` section: can the user route the provider at a *different cloud
     vendor's* API via the same override mechanisms? (`supported: boolean`,
     `mechanism: string`, `example: string`.)
   - Keep everything else (config_files, api_standards, metadata_overrides,
     merge_semantics, env_vars, default_model_site).
2. **Prompt (`_fleet.md`) changes:**
   - Reframe the local-models section: the question is never "does {{state.name}}
     support Ollama" but "which API standards can {{state.name}}'s model client speak,
     and how is its base URL redirected"; runner-side facts (ports, paths, standards)
     come from the local-runners research — instruct the agent to READ the relevant
     `docs/research/local_runners/*.md` frontmatter as ground truth rather than
     re-researching runners.
   - Add the cloud-bridge questions (other vendors' models on vendor platforms).
   - Add an explicit anti-pattern instruction: "do not describe absence of first-class
     runner integration as 'no support' when a base-URL override path exists".
3. Validate the revised sidecar positive/negative (`md schema validate` on temp docs);
   `claudine sequence ... --dry-run` must pass ×9.
4. **Checkpoint with Ken** on the revised schema + prompt before running.

## Phase 2 — prove the prompt on a pilot

1. Pilot on **Claude Code** (the doc with the known misrepresentation) plus **Codex**
   (vendor platform with TOML `model_providers`) using the temp-pilot-roster technique.
   The existing docs are from a prior day, so the initialize same-day skip will not
   block; the `update` gate will run them in update mode (changelog entries expected).
2. Evaluate with subagents against concrete checks: does the Claude doc now document the
   base-URL override path with a working-looking example? does `local_runners[]` use the
   new integration enum with runner facts consistent with the local-runners ground
   truth? does `cloud_bridge` exist and answer concretely? schema-valid?
3. Adjust prompt/schema as needed; re-pilot only if changes were material.

## Phase 3 — fleet completion + refresh decision

1. Run the full fleet with the finalized prompt. All 9 docs exist, so every step runs in
   update mode. **Skip-gate note:** any doc already updated *today* (the pilots) will be
   skipped by the initialize stack — that is correct behavior; do not delete files to
   force re-runs unless the prompt changed after their run.
2. For each provider doc, an evaluation subagent renders one of two verdicts, and the
   orchestrating agent makes the call per doc:
   - **Acceptable via targeted edit** — the re-run produced good research but small
     inconsistencies remain (e.g. stale phrasing carried from the old body): fix
     directly with surgical edits, keeping frontmatter schema-valid.
   - **Re-run required** — the doc still misframes support or is internally
     inconsistent: re-run that provider (temp pilot roster), after prompt adjustment if
     the failure implicates the prompt.
3. Exit criteria: all 9 docs schema-valid; no doc claims "no local model support" where
   a base-URL path exists; `local_runners[].integration` and `cloud_bridge` populated
   for all providers; spot-check consistency against local-runners ground truth passes;
   changelog entries present (update-mode runs must record what changed).

## Phase 4 — the `model-config` agent skill

1. Create `.claude/skills/model-config/` with `SKILL.md` (<200 lines; activation
   `description` written around: configuring models in agentic CLIs, adding
   local/cloud models to Claude Code / Codex / OpenCode / etc., base-URL overrides,
   OpenAI/Anthropic-compatible endpoints, per-provider config file shapes) plus
   supporting reference docs distilled **from the research frontmatter first**
   (per-provider tables: config file + format, api_standards, integration paths,
   merge semantics, env overrides) with prose only where it adds judgment.
2. The skill must answer, context-efficiently: "how do I add model X (cloud or local)
   to provider Y", "which API standards can provider Y's client speak and how is the
   base URL redirected", "whose metadata wins when a user block and the built-in
   catalog overlap". Cross-link the `local-runners` skill for runner-side facts and
   cite `claudine/docs/research/model-config/<provider>.md` for depth.
3. **Update the `claudine` skill** (`.claude/skills/claudine/SKILL.md`, research
   section): add a short summary of the model-config research area and a pointer
   recommending the `model-config` skill for configuration depth (alongside the
   `local-runners` pointer added by the sibling plan). Keep it to a few lines — the
   claudine skill carries the map, not the territory. If the skill file declares a
   `hash:` frontmatter property, regenerate it with `md hash <file>` after editing.

## Done when

The prompt is frozen (record the iteration history briefly in
`claudine/features/2026-07-02-provider-metadata/spike-logging/findings.md`'s style — a short
`model-config-refresh.md` note in the feature dir), the 9 provider docs are
high-quality, consistent, and correctly represent local-runner and cross-cloud
bridging, the `model-config` skill exists and is consistent with the research, and the
`claudine` skill carries the summary + pointer.
