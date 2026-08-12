# Plan: Local Runners Research Fleet

> **Goal:** build authoritative, schema-enforced research on local model runners
> (LM Studio, Ollama, oMLX, Llama.cpp, vLLM), then distill it into a `local-runners`
> agent skill. Local runners are model **servers**, not agentic CLIs — but the same
> research machinery applies.

## Context the executing agent needs

- **Use the `claudine` agent skill** (load it first) — it documents the composition/
  sequence machinery, lifecycle stacks, and CLI conventions this plan assumes.
- **Parent spec:** `claudine/features/2026-07-02-provider-metadata/spec.md` (read the "Model Ground
  Truth" and "Out-of-box vs user-configured models" sections). Worked example of the
  whole research pattern: `claudine/features/2026-07-02-provider-metadata/spike-logging/findings.md`.
- **The established research pattern** (copy it, don't reinvent): a roster YAML drives
  `claudine sequence` over a topic prompt; structured facts go to frontmatter validated
  by a `_schema.yaml` sidecar (`$schema: ./_schema.yaml`; the sidecar MUST wrap its
  properties under a root `$schema:` key; nested shapes MUST be quoted inline-object
  literals). Reference implementation: `claudine/docs/research/model-config/`
  (`_fleet.md` + `_schema.yaml`).
- **Sequence file must carry** (all present in the reference implementation):
  `update: "{{file_exists(file) && !markdown_body_empty(file)}}"`, the `initialize`
  same-day-skip stack, the `success` verification stack (gate on
  `frontmatter(file, 'last_updated') != ctx.today`), and exit criteria requiring
  `md schema validate` to pass.
- **Run invocation:** `claudine sequence <file> --yolo` (the `grant:` frontmatter is not
  implemented; without `--yolo` OpenCode auto-rejects `external_directory` and the agent
  stops early). Run fleets in the background; steps take ~5–15 min each.
- **Model directive:** `agent: opencode`, `model: kimi-for-coding/k2p7`. It is possible
  (not expected) that this plan gets capped mid-run (`Weekly/Monthly Limit Exhausted`).
  If that happens, **immediately switch the sequence frontmatter to
  `model: minimax/MiniMax-M3`** (still OpenCode) and resume — the initialize skip makes
  re-runs resume cleanly. Also verify the first `llm_call_start` line in the run output
  names the requested model: OpenCode has been observed transiently substituting the
  config-default model; if the wrong model appears, kill and relaunch.
- **Context-window discipline:** the orchestrating agent must NOT read full research
  bodies or long web pages into its own context. Fan out subagents for discovery and
  evaluation; keep only their structured summaries. Research execution itself happens in
  the `claudine sequence` child processes, which cost the orchestrator nothing.

## Phase 1 — attribute discovery (orchestrated fan-out)

Spawn **one research subagent per runner** (parallel, web-enabled), each returning a
compact structured summary (NOT prose dumps) answering: what attributes describe this
runner that a machine catalog should capture? Seed each subagent with the known-required
attribute list below and ask it to confirm/extend:

1. URLs — homepage, docs, API schema/reference, repo
2. Summary description
3. OS platforms — which OSes; binary name(s) to check for per OS; install methods
   (brew, installer app, pip, docker); daemon vs foreground; service management
   (launchd/systemd)
4. API: default listening port(s)
5. API: Anthropic Messages API support — supported? URL path, base_url shape, auth
   variance
6. API: OpenAI API support — supported? URL path, base_url shape, auth variance,
   known compatibility deviations/gotchas
7. API: metadata endpoints — health check, model list, currently-loaded model(s)
8. Configuration file(s) and format

Candidate extensions the subagents should evaluate (drop what proves irrelevant):
model acquisition (registry pull vs HuggingFace vs manual), supported model formats /
quantization (GGUF, MLX, safetensors), hardware acceleration (Metal/CUDA/ROCm/CPU),
multi-model & concurrent serving, streaming (SSE) support, tool/function-calling
support, embeddings endpoint, auth options (none/api-key), env vars, and a
**per-runner example config block for OpenCode** (the `provider.<id>.npm` +
`baseURL` + `models` shape — see spec "Out-of-box vs user-configured models").

**Synthesis (main context):** merge the five summaries into an attribute inventory and a
draft `_schema.yaml`. Two purposes to keep in mind: (a) the schema feeds the
`model_config` topic's ground truth (see `model-config-plan.md`); (b) the platform/binary
/port/health-url attributes are a future `sniff` detection surface — capture them in
detection-friendly form (exact binary names per OS, exact URL paths).

## Phase 2 — roster + baseline prompt (CHECKPOINT)

1. Write `claudine/docs/local-runners.yaml` in the same format as
   `claudine/docs/providers.yaml` (`kind: sequence`, a `template.desc`, and a `list:`
   with one entry per runner). Entries: **LM Studio, Ollama, oMLX, Llama.cpp, vLLM**
   with `name`, `file` (e.g. `ollama.md`), `site`, `repo`, and any useful
   template-visible fields discovered in Phase 1 (e.g. `binary`).
2. Write `claudine/docs/research/local_runners/_schema.yaml` (from the Phase-1 draft;
   validate positive AND negative with `md schema validate` using temp docs, then delete
   the temp docs).
3. Write `claudine/docs/research/local_runners/_fleet.md` — the sequence prompt,
   following the model-config reference structure: Skills section (`claudine` skill),
   Scope, Document Structure (H2 sections mirroring the schema groups), Task steps,
   `$schema: ./_schema.yaml` instruction, per-field capture instructions, an on-host
   **evidence requirement** (some runners are installed on this host — `which ollama`,
   check default ports, read real config files; prefer `observed` facts over
   documentation), Output (`::file @prompts/make-it-markdown.md`), Exit Criteria.
4. **STOP and checkpoint with Ken** — present the attribute inventory, the schema, and
   the prompt for brainstorm/approval before any fleet run.

## Phase 3 — pilot (2–3 runners) + evaluation

1. After approval: pilot on **Ollama, oMLX, and vLLM** (diversity: the most popular, the
   Apple-Silicon-native one, and a server-class one). `claudine sequence` has no
   per-item selector — use the pilot technique from the logging spike: a temporary
   pilot roster YAML containing only those entries + a temp copy of the sequence file
   pointing at it (both in the same directory so relative references resolve); delete
   both afterward.
2. Evaluate with **parallel subagents** (one per produced doc, returning structured
   verdicts): schema-valid? every API claim carries a URL or observed evidence?
   base_url/auth answered concretely (not "varies")? OpenCode example config present and
   plausible? gaps or hand-waving flagged.
3. Adjust prompt/schema from the verdicts. Schema changes are contract changes — reflect
   them in both `_schema.yaml` and the capture instructions.

## Phase 4 — complete + finalize

1. Run the full fleet (`claudine sequence .../_fleet.md --yolo`). Already-fresh
   pilot docs re-run only if the prompt changed materially — if so, delete the pilot
   docs first (the initialize skip would otherwise bypass them same-day).
2. Re-run the Phase-3 evaluation subagents across all five docs; confirm improvement;
   make final prompt adjustments and (only if needed) one last targeted re-run.
3. Record outcomes (what the prompt iterations changed, remaining gaps) in a short
   `claudine/features/2026-07-02-provider-metadata/spike-local-runners.md`.

## Phase 5 — the `local-runners` agent skill

1. Create `.claude/skills/local-runners/` with `SKILL.md` (<200 lines; frontmatter
   `description` written for activation: "local model runners / Ollama / LM Studio /
   oMLX / llama.cpp / vLLM / OpenAI-compatible local endpoints…") plus per-runner
   reference docs distilled **from the research frontmatter first** (tables of ports,
   endpoints, binaries, config paths) with prose only where it adds judgment.
2. The skill must answer, context-efficiently: "which runner is installed / how do I
   detect it", "what base URL + path do I use for OpenAI/Anthropic-style calls", "how do
   I list/load models", "how do I wire it into an agentic CLI".
3. Cross-link: research docs are the source of truth; the skill cites
   `claudine/docs/research/local_runners/<runner>.md` for depth.
4. **Update the `claudine` skill** (`.claude/skills/claudine/SKILL.md`, research
   section): add a short summary of the local-runners research area (what it covers,
   where the docs live) and a pointer recommending the `local-runners` skill for
   runner-specific depth. Keep it to a few lines — the claudine skill carries the map,
   not the territory. If the skill file declares a `hash:` frontmatter property,
   regenerate it with `md hash <file>` after editing.

## Done when

- `local-runners.yaml` roster exists; 5 schema-valid research docs; evaluation verdicts
  green; prompt frozen; `local-runners` skill created and consistent with the research;
  `claudine` skill updated with the summary + pointer.
