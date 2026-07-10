# Spike: Local Runners Research Fleet

> 2026-07-02. Executed `local-runner-plan.md`: schema-enforced research on five
> local model runners (Ollama, LM Studio, oMLX, Llama.cpp, vLLM) plus a
> `local-runners` agent skill. All fleet runs on OpenCode +
> `kimi-for-coding/k2p7`.

## What was built

- `docs/local-runners.yaml` — runner roster (same shape as `providers.yaml`,
  plus `binary` / `default_port` template fields).
- `docs/research/local_runners/_schema.yaml` — sidecar contract; validated
  positive AND negative before first use (negative fixture caught all five
  deliberate violations with line numbers).
- `docs/research/local_runners/_fleet.md` — sequence prompt carrying
  the full established pattern (fixed `update:`, `initialize` same-day skip,
  `success` verification stack, `--yolo` grant note, on-host evidence
  requirement).
- Five schema-valid research documents; `local-runners` skill (Phase 5).

## Run history

1. **Pilot** (Ollama, oMLX, vLLM; temp-roster technique) — 3/3 succeeded,
   schema-valid on first pass, ~8 min/doc. Evaluators: 3× pass_with_issues.
2. **Contract v2** from pilot verdicts, then **full fleet** (pilot docs
   deleted first — prompt changed materially) — 5/5 succeeded, schema-valid.
   Evaluators: omlx/lmstudio/llamacpp pass_with_issues, **ollama and vllm
   fail** (see failure taxonomy).
3. **Contract v3** targeting the failure taxonomy, then a **targeted re-run**
   of the four problem docs (omlx kept). First launch aborted correctly
   (model substitution → capped GLM → fail-fast); relaunch 4/4, all
   schema-valid. Grep-verified every must-fix defect closed except two
   single-line items, hand-corrected (see below).

## Schema/prompt iterations (what the verdicts changed)

**v2 (post-pilot):** `metadata_endpoints.auth_gated` (detection needs the
ungated probe set — oMLX's `/api/status` rejects unauthenticated requests),
`integration_hooks[]` (`omlx launch codex`, `ollama launch claude` had no
schema home and were silently dropped), `traps[]`, `auth_notes`, operational
definition of `platforms.support: native` (released artifact, else
`separate_project` — vLLM-Metal), since_version must be exact-or-unknown,
OpenAPI `/openapi.json` counts as `formal` schema + endpoint enumeration
source, served-vs-import model-format split, all-id-forms grammar rule,
negative probes (observed 404s) as evidence.

**v3 (post-fleet):** since_version **verification protocol** (open the tag's
release notes and find the feature named there, else `unknown` — a
confidently wrong tag is worse than unknown), exact `opencode_example` JSON
skeleton in the prompt, cross-runner contamination warning (only record
endpoints verified against THIS runner), never drop experimental endpoints
(mark them), legacy-migrated-host caution (record fresh-install default AND
observed legacy paths; never infer another OS's paths from this host).

## k2p7 failure-mode taxonomy (from 8 evaluator verdicts)

1. **Version fabrication** (3×): exact-but-wrong `since_version` tags, once
   with a citation that didn't support the claim (vLLM "v0.17.0"). v3's
   verification protocol fixed all instances (v0.14.0 / v0.11.1 / b7187
   correct on re-run).
2. **OpenCode block malformed** (3×): `npm` set to the runner's JS client
   (`"ollama"`, `"vllm"`, `"openai"`), models as arrays, flattened keys. The
   v3 skeleton fixed all three.
3. **Cross-runner contamination** (2×): llama.cpp doc claimed Ollama's
   `/api/tags` (once even as "observed on this host" — a fabricated
   observation) and named oMLX's default port as 8080. Reduced but **not
   eliminated** by v3 — one `/api/tags` claim survived and was hand-corrected
   against `server-http.cpp`.
4. **Host-state generalization** (1×): LM Studio's legacy-migrated
   `~/.cache/lm-studio` presented as the fresh-install default; Windows paths
   invented from macOS conventions (`%LOCALAPPDATA%`). Fixed by v3.

## Hand-corrections (overrides-layer specimens)

Two single-line corrections were applied by hand after the targeted re-run
rather than burning another fleet pass; both are exactly the class of durable
correction the spec's `docs/providers/overrides/` layer is designed for
(these edits will be overwritten by the next research regeneration):

- `llamacpp.md` model_list notes: removed the fabricated `/api/tags` claim
  (verified absent from `tools/server/server-http.cpp`'s public allowlist).
- `vllm.md` openai_compatible `since_version`: `"v0.0.1"` → `"unknown"`
  (v0.0.1 is PyPI's pre-launch placeholder; the tag's notes do not name the
  feature).

## Live specimens for existing backlog items

- **Model-mismatch guard (4th occurrence):** the first targeted-re-run launch
  resolved `zai-coding-plan/glm-5.2` despite frontmatter `kimi-for-coding/
  k2p7`. Because GLM was usage-capped, the stream failed and claudine
  aborted fail-fast (visible failure). Had GLM been healthy, four documents
  would have been silently researched on the wrong model. The
  `llm_call_start`-vs-requested comparison remains the highest-value wrapper
  guard on the backlog.
- **Per-item sequence selector:** the temp-roster + temp-sequence-copy
  technique was needed twice (pilot, targeted re-run). A `--only <name>`
  flag would remove real friction.
- **`grant:` frontmatter still a no-op** — both fleets ran `--yolo`.

## Notable research findings

- **Anthropic Messages API is now universal** across all five runners, each
  added 2025–2026 explicitly for Claude Code compat (Ollama v0.14.0,
  LM Studio 0.4.1, llama.cpp b7187, vLLM v0.11.1, oMLX v0.1.0). Anthropic
  base URLs exclude `/v1`; OpenAI base URLs include it.
- **Port alone cannot identify a runner** (vLLM and oMLX both default 8000);
  the `detection[]` records carry response markers (`GET /` → "Ollama is
  running", `/props` build_info, oMLX `/health` default_model).
- llama.cpp has a new official homepage: **https://llama.app**.
- `ollama launch` now wires 16 agentic-CLI integrations; oMLX's `omlx launch`
  wires 8 — runner-native integration hooks are a real, growing surface.
- Drift flag for the `model-citizen` skill: it documents only LM Studio's
  legacy model dirs; the current default is `~/.lmstudio/models` (pointer
  file `~/.lmstudio-home-pointer` relocates it).

## Codex verification wave (2026-07-02, post-fleet)

Ken directed a final full-content evaluation of the four re-run docs plus the
omlx fixes, all executed by **Codex** (`codex exec
--dangerously-bypass-approvals-and-sandbox`, prompt via stdin). Verdicts:
ollama pass_with_issues, lmstudio pass_with_issues, **llamacpp fail, vllm
fail** — confirming that targeted-re-run docs need their own adversarial pass
(every fresh k2p7 generation introduced new errors). All flagged defects were
then fixed by per-doc Codex correction jobs (live-probe-verified; llamacpp's
job started a temporary router-mode `llama-server` on port 18080 to observe
auth gating), with skill-file syncs applied by hand. All five docs and the
skill are now consistent and schema-valid.

Notable truths established by the wave:

- **vLLM Anthropic versions are split**: `/v1/messages` v0.11.1 but
  `/v1/messages/count_tokens` v0.17.0 (PR #35588) — both earlier "wrong"
  version claims were each half-right.
- **llamacpp contamination recurred in a new form** (`/api/show`, `/api/chat`
  — PR #22165 removed all `/api` routes); cross-runner contamination survives
  targeted prompt instructions and needs an explicit verification pass.
- The hand-written allowlist note from the previous round was itself wrong
  (`/props`/`/metrics` are NOT in server-http.cpp's public set) — corrections
  need the same adversarial verification as generated content.
- Ollama's OpenAI deviations list was stale against current docs
  (`tool_choice`/`logit_bias`/`user`/`n`/image URLs now supported; verified
  live); `/v1/responses` first named in v0.14.2 release notes.
- LM Studio fresh-install default is `~/.lmstudio`; `~/.cache/lm-studio` is
  legacy migration only; `/openapi.json` returns an error body, not a schema.

### Tooling bugs found while running the wave

1. **claudine compose→codex non-interactive hang.** `prompt_delivery` in
   `cli/src/commands/wrap/profile/codex.rs` returns `Stdin` for
   non-interactive runs (comment: "Codex exec reads from stdin"), but the
   spawned argv carries no `exec` subcommand — bare `codex` opens the
   interactive TUI, which without a TTY spawns `$EDITOR` (observed: `nvim
   ~/.claudine/.codex/instructions.md`) and blocks forever. Neither
   `timeout` nor `step_timeout` fires (hang precedes streaming). Reproduced
   with frontmatter `agent: codex` and with the `--codex` switch. Workaround
   used: raw `codex exec` with stdin prompt.
2. **Stale codex on PATH**: `/usr/local/bin/codex` (old npm install,
   unauthenticated, crashes under no-TTY) shadows the real
   `~/.bun/bin/codex`.

## Remaining known gaps

- `vllm.md`: `/ping` (SageMaker alias) still unrecorded; `VLLM_HOST_IP`
  companion trap mentioned only alongside `VLLM_PORT`.
- `lmstudio.md`: `llmster` daemon captured only in `alt_binaries`;
  Linux/Windows home-pointer relocation marked inferred (no official source).
- Judged minor by evaluators; next-regeneration fodder, not
  catalog-poisoning errors.
