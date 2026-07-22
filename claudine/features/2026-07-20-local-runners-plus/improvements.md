# Local Runners Fleet — Improvement Report

Produced 2026-07-19 by a three-pass orchestrated review of the local-runners
research fleet (`_fleet.md` + `local-runners.yaml` + `_schema.yaml`), followed
by a coherence pass across the three task outcomes.

> **Addendum 2026-07-19 (applied):** an independent research pass verified the
> claims here against live sources and local probes, corrected the facts
> marked ✎ below, added the port-identity strategy ([Task 4](#task-4--port-identity-strategy-added-2026-07-19)),
> and landed the sequencing step-1 `_fleet.md` defect fixes plus the
> `identity_probes` schema addition (all five existing docs updated and
> re-validated).

- **Task 1** — roster evaluation: which runners belong in the fleet
- **Task 2** — frontmatter facts: schema structure and missing facts
- **Task 3** — lifecycle hooks: challenging success, recovering from failure
- **Task 4** — port identity: identifying which runner listens on a shared port

A consolidated, ordered adoption plan is at the end ([Recommended
Sequencing](#recommended-sequencing)); a Claudine/Darkmatter feature wishlist
distilled from blocked improvements is just before it.

---

## Task 1 — Runner Roster Evaluation

Twenty candidates were evaluated against the roster criteria (local model
*server* with an HTTP API, OpenAI-compatible preferred; real coding-agent
adoption; released artifact on at least one of macOS/Linux/Windows; distinct
detection-catalog value). All popularity/status claims were verified against
live sources on 2026-07-19.

### Verdict summary

| Candidate | Verdict | One-line rationale |
|---|---|---|
| LocalAI | **add now** | Very active (v4.3, May 2026); OpenAI + Anthropic + Ollama drop-in APIs; distinct `local-ai` binary + systemd service |
| SGLang | **add now** | HF's officially recommended TGI successor alongside vLLM; distinct port 30000; launched via `python -m sglang.launch_server` ✎ |
| Docker Model Runner | **add now** | Docker-official, huge install base via Docker Desktop 4.40+; distinct port 12434; OpenAI + Anthropic + Ollama APIs |
| Jan | **add now** | Active desktop app with a built-in OpenAI-compatible server on distinct port 1337 (Anthropic endpoint claimed in the original review is unverified — see ✎ below) |
| Lemonade Server | **add now** | AMD-sponsored official local server (GPU + NPU); distinct port 13305; OpenAI/Ollama/Anthropic compatible |
| KoboldCpp | **add now** | Active single-binary llama.cpp derivative with its own API family, port 5001, and router mode |
| mistral.rs | watchlist | Capable Rust engine (OpenAI + Anthropic endpoints) but moderate adoption; port 1234 collides with LM Studio |
| llamafile | watchlist | Revived by Mozilla.ai (v0.10.0, 2026-03-19) but momentum still recovering; no stable binary name to detect |
| RamaLama | watchlist | Red Hat/containers-org backed; serves llama.cpp/vLLM in containers on 8080; adoption modest |
| Xinference | watchlist | Active (v3.0.0, 2026-07-19); distinct port 9997; adoption centered on Dify/RAG stacks, not coding agents |
| text-generation-webui | watchlist | Still active with portable builds + OpenAI/Anthropic API on 5000, but coding-agent mindshare has moved on |
| TabbyAPI | watchlist | Official ExLlamaV3 server (port 5000) but self-described hobby project; NVIDIA-exl3 niche |
| Nexa SDK | watchlist | Qualcomm-backed `nexa serve`; unique NPU angle but adoption early, default port unstable |
| MLX `mlx_lm.server` | watchlist | Official Apple server but macOS-only, "not recommended for production"; oMLX owns the MLX slot |
| llama-swap | watchlist | Popular, but an orchestrating proxy over other local servers; partially obsoleted by llama.cpp's built-in router |
| GPT4All | exclude | Dormant — last release v3.10.0 (Feb 2025), no commits since; "Is GPT4all dead?" issues unanswered |
| Hugging Face TGI | exclude | Maintenance mode 2025-12-11; repo archived read-only 2026-03-21; HF points users to vLLM/SGLang |
| Cortex (Jan server) | exclude | Deprecated by Jan in favor of direct llama.cpp; repo archived 2025-07-04 |
| LiteLLM | exclude | Proxy/router — never loads model weights; belongs in the `model-config` topic as a wiring pattern |
| Msty | exclude | Closed-source studio whose "Local AI service" is a bundled Ollama — detection would correctly find Ollama |

If fleet slots are constrained, priority order for the adds:
**Docker Model Runner ≥ LocalAI ≥ SGLang ≥ Jan > Lemonade > KoboldCpp**
(KoboldCpp's adoption skews roleplay over coding agents — demote it first).

### Proposed roster entries

```yaml
- name: LocalAI
  file: localai.md
  binary: local-ai
  default_port: 8080
  user_dir: ""
  site: https://localai.io
  docs: https://localai.io/docs/
  repo: https://github.com/mudler/LocalAI
- name: SGLang
  file: sglang.md
  binary: sglang
  default_port: 30000
  user_dir: ""
  site: https://docs.sglang.ai
  docs: https://docs.sglang.ai
  repo: https://github.com/sgl-project/sglang
- name: Docker Model Runner
  file: docker-model-runner.md
  binary: docker-model
  default_port: 12434
  user_dir: ""
  site: https://docs.docker.com/ai/model-runner/
  docs: https://docs.docker.com/ai/model-runner/api-reference/
  repo: https://github.com/docker/model-runner
- name: Jan
  file: jan.md
  binary: jan
  default_port: 1337
  user_dir: "~/Library/Application Support/Jan"
  site: https://jan.ai
  docs: https://www.jan.ai/docs
  repo: https://github.com/janhq/jan
- name: Lemonade Server
  file: lemonade.md
  binary: lemonade-server
  default_port: 13305
  user_dir: "~/.cache/lemonade"
  site: https://lemonade-server.ai
  docs: https://lemonade-server.ai/docs/
  repo: https://github.com/lemonade-sdk/lemonade
- name: KoboldCpp
  file: koboldcpp.md
  binary: koboldcpp
  default_port: 5001
  user_dir: ""
  site: https://github.com/LostRuins/koboldcpp
  docs: https://github.com/LostRuins/koboldcpp/wiki
  repo: https://github.com/LostRuins/koboldcpp
```

Entry-specific cautions the research docs must carry:

- **LocalAI** — port 8080 collides with llama-server; disambiguate via WebUI
  root, `/readyz`, and the backend-gallery API. No fixed per-user dir
  (`MODELS_PATH` / `--models-path`).
- **Docker Model Runner** — no PATH binary in the classic sense: CLI plugin at
  `~/.docker/cli-plugins/docker-model`; base URL is
  `http://localhost:12434/engines/v1`; host TCP port opt-in on Docker Desktop,
  default-on for the Linux package. Model store lives inside Docker-managed
  storage (a VM on macOS/Windows), so `user_dir` is empty.
- **Jan** — detection is app-bundle + port-1337 probe, not a PATH binary.
  Data folder differs per OS (macOS `~/Library/Application Support/Jan`, Linux
  `~/.config/jan`, Windows `%APPDATA%\jan`; legacy `~/jan` / `~/.jan`). A
  separate self-hosted "Jan Server" Docker stack exists on port 8000 — the
  roster entry targets the desktop app's server (the LM Studio precedent).
- **Lemonade** — default port changed: current docs use **13305**; older
  guides (and LiteLLM docs) still show 8000 — record both. Linux splits into
  `lemond` daemon + `lemonade` CLI + `lemonade-tray`; Debian package registers
  systemd service `lemonade-server`; apt installs cache under
  `/var/lib/lemonade/.cache`.
- **KoboldCpp** — deliberately emulates other runners' APIs (Ollama,
  A1111/Forge, ComfyUI, Whisper, XTTS — Ollama emulation confirmed via the
  README feature list ✎). Community reports that users run it on 11434 to
  impersonate Ollama, and that its API key is accepted but not validated, are
  plausible but were **not verifiable** against official sources on
  2026-07-19 ✎ — treat both as reported, not established. Its
  `GET /api/extra/version` endpoint returns `{"result":"KoboldCpp",...}`, a
  definitive identity marker even when it impersonates Ollama's port.

### Existing-entry audit

No dead projects, renamed binaries, changed ports, or wrong URLs among the
existing five. Specifically verified: oMLX is alive and healthy at
github.com/jundot/omlx (17k+ stars, Apache-2.0, recent releases, omlx.ai
live). Two research-doc (not roster) notes:

1. **llama-server now ships a built-in router mode** (`--models-dir`,
   `POST /models/load` / `/models/unload`) for multi-model hotswap — its
   metadata-endpoint surface changed and its research doc should be refreshed.
2. **Port collisions are worsening** — after the adds: 8000 (vLLM, oMLX),
   8080 (llama-server, LocalAI, mlx_lm.server), 5000 (TabbyAPI,
   text-generation-webui), 1234 (LM Studio, mistral.rs). The `detection[].expect`
   response-marker field is doing real work; stress it in each new doc.

### Landscape note

Anthropic-Messages-API support has become table stakes in 2026: oMLX,
LocalAI, Docker Model Runner, Lemonade, llamafile, mistral.rs, and
text-generation-webui all expose one. (The original review included Jan in
this list ✎ — Jan's API-server docs describe only an OpenAI-compatible
server powered by llama.cpp; no Anthropic endpoint was verifiable on
2026-07-19, so the Jan research doc must establish this fresh rather than
inherit the claim.) The `## Agentic CLI Integration`
section's direct-Claude-Code wiring (`ANTHROPIC_BASE_URL`) is now relevant for
most of the fleet, not an exception.

### Fact corrections (2026-07-19 verification pass)

Claims in the original review that did not survive re-verification against
official sources:

1. **Jan has an Anthropic-compatible server** — unverified; Jan's docs
   document OpenAI-compatible only. Corrected in the verdict table and
   landscape note.
2. **SGLang's CLI is `sglang serve`** — unverified; the documented launcher
   is `python -m sglang.launch_server`. The proposed roster entry's
   `binary: sglang` should be re-established by the research doc (a `sglang`
   console script exists, but its `serve` subcommand alias was not evidenced).
3. **KoboldCpp on 11434 / unvalidated API key** — community-reported, not
   established by official sources; downgraded to "reported" in the cautions.

Confirmed during the same pass: LocalAI (8080, `/readyz`, OpenAI + Anthropic
+ Ollama route registrations in `core/http/app.go`), Docker Model Runner
(12434, `/engines/v1`, host-TCP opt-in split, `~/.docker/cli-plugins/`), Jan
(1337), Lemonade (**port changed 8000 → 13305 in v10.1.0, 2026-04-06** — the
review's 13305 is correct, and the 8000-collision set shrank accordingly),
KoboldCpp (5001, OllamaApi emulation), llama.cpp router mode
(`--models-dir`, `/models/load`, `GET /props` → `"role":"router"`).

---

## Task 2 — Frontmatter Facts & Schema Structure

### Verified SimplifiedSchema capabilities (grounding)

Verified against the Darkmatter source
(`darkmatter/lib/src/markdown/schemas/simplified/`), not the schema file's own
comments:

- Primitive types incl. `literal`/`expression`; arrays via postfix `[]`;
  property-level and root-level unions; inline object literals.
- **YAML-native nested mappings ARE accepted** and lower to inline objects
  (`parse_property_def` → `parse_schema_shape`, `simplified/mod.rs:240`).
  The comment at `_schema.yaml:14-16` claiming they are rejected is **stale**
  — fix the comment (and optionally migrate the quoted-string object literals
  to native mappings for readability).
- Pattern keys (`<string>`, `<starting::…>`, `<ending::…>`, `<pattern::…>`),
  cross-file imports (`Name@fileref`).
- Per-property constraints: `required`, `default(...)`, `min/max`, `integer`,
  `minLen/maxLen`, `notEmpty`, **`pattern(regex)`**, `suggest(...)`,
  `members(...)`; object-level `$constraints`; array-level constraints.

The availability of `pattern(regex)` matters beyond Task 2 — it is the interim
enforcement path for Task 3's `since_version` problem (see below).

### (a) Structural proposals (ranked by value/cost)

**1. Widen `api_standards.standard` with `ollama_compatible` — high value.**
The schema edit is additive (existing documents stay valid); the recommended
*data* migration is small but real: Ollama's `/api/*` records re-classify from
`native` to `ollama_compatible`, and llama.cpp's Ollama-alias endpoints join
them. This is reinforced by Task 1: four of the six proposed runners
(LocalAI, Docker Model Runner, Lemonade, KoboldCpp) expose Ollama-compatible
endpoints, so without the enum member the ambiguity multiplies.

```yaml
standard: enum(openai_compatible,anthropic_compatible,ollama_compatible,native; required)
```

**2. Add `container` to `platforms.support` — medium value, additive.**
Docker Model Runner is container-native; LocalAI and SGLang are
container-first. Forcing them into `native` or `unsupported` is a semantic
mismatch.

```yaml
support: enum(native,wsl,container,separate_project,unsupported; required)
```

**3. Convert `opencode_example` from JSON-string to a structured object —
medium value, breaking.** The JSON-as-string smell: validation cannot check
structure, consumers must parse, malformed JSON surfaces at runtime. Pattern
keys model the provider-id and model-id dictionaries:

```yaml
opencode_example: "{ provider: { <string>: { npm: string(required), name: string(required), options: { baseURL: url(required) }, models: { <string>: { name: string(required) } } } } }"
```

**4. Add `env` (and `xml`) to `config_files.format` — low value, additive.**
`.env` files are neither `ini` nor `text`.

**5. Extend optional `confidence` beyond `detection` — low value, additive.**
Add `confidence: enum(source_code,observed,documented,inferred)` to
`metadata_endpoints`, `env_vars`, and `model_store_paths` — an `observed`
default port is stronger evidence than a `documented` one, and negative
probes ("404 observed") deserve the same grading the detection records get.

**6. No schema change — instruction clarification.** Usage of top-level
`api_reference_url` vs `api_standards[].docs_url` is inconsistent across the
existing five documents (Ollama stuffs API docs into the per-standard field).
Clarify in `_fleet.md`: top-level `api_reference_url` is the authoritative API
docs; per-standard `docs_url` is for standard-specific deviations/guides.

**7. Tighten `since_version` with a pattern constraint — high value,
additive (coherence-pass addition).** The fleet prompt's verification protocol
("exact tag or the literal `unknown`, never a hedge") is enforced by prose
only. A `pattern(...)` constraint rejecting empty strings and hedge phrases
(e.g. require `^\S+$` at minimum, disallowing spaces — which kills "v0.13.3 or
later") makes `validate_schema()` (Task 3, P1) enforce it mechanically. This
is the ratified interim for the expression engine's lack of array predicates.

### (b) Missing facts (accepted)

Each tied to a stated consumer; encyclopedia candidates were rejected.

| Field | Type (SimplifiedSchema) | Consumer / justification |
|---|---|---|
| `license` | `enum(mit,apache_2,gpl_3,proprietary,mixed; required)` | model-config: legal permissions are distinct from `open_source` artifact availability (full-open + GPL ≠ full-open + Apache) |
| `version_probe` | `"{ os: enum(macos,linux,windows,all; required), method: enum(cli,bundle,package,http; required), command: string(required), pattern: string, confidence: enum(source_code,observed,documented,inferred; required), notes: string }[]"` | sniff: binary detection is incomplete without the `--version` invocation and a parse regex; flag and output format vary per runner (and sometimes per OS — hence a record list). **Adopted and landed 2026-07-19** (schema + all five docs; shape extended with `method`/`confidence` — see below) |
| `structured_output` | `"{ supported: enum(yes,no,conditional; required), mechanism: enum(json_mode,json_schema,grammar,tool_use,none), notes: string }"` | model-config: agentic CLIs lean on JSON-mode / schema-constrained / grammar generation; support varies widely |
| `vision` | `"{ supported: enum(yes,no,conditional; required), modalities: string[], notes: string }"` | model-config: which runners serve multimodal models (agentic CLIs send base64 images) |
| `context_control` | `"{ mechanism: enum(flag,env_var,model_default,config_file,unsupported; required), site: string, notes: string }"` | user guidance: agentic sessions hit 32k-128k; how to raise the limit (`--ctx-size`, `OLLAMA_CONTEXT_LENGTH`, …) |
| `max_concurrent_models` | `"{ supported: boolean(required), mechanism: enum(flag,env_var,auto,unsupported), site: string, default: number, notes: string }"` | user guidance: multi-model capacity limits (e.g. `OLLAMA_MAX_LOADED_MODELS`) |
| `model_ttl` | `"{ supported: boolean(required), mechanism: enum(flag,env_var,config,none), site: string, default: string, notes: string }"` | user guidance: idle auto-unload behavior (Ollama `5m` default vs vLLM never) |
| `telemetry` | `"{ enabled_by_default: boolean(required), opt_out_mechanism: enum(flag,env_var,config,none), site: string, notes: string }"` | privacy/compliance: phone-home behavior and how to disable it |
| `min_hardware` | `"{ ram_gb: number, vram_gb: number, cpu: string, notes: string }"` | sniff/user guidance: "this host probably can't run X" inference (oMLX needs Apple Silicon; vLLM needs real GPU memory) |
| `port_conflict` | `"{ default_port: number(required), configurable: boolean(required), mechanism: enum(flag,env_var,config,none), site: string, notes: string }"` | sniff: Task 1's collision map (8000, 8080, 1234, 5000) makes reconfigurability a detection-relevant fact |
| `identity_probes` | `"{ rank: number(required), request: string(required), match_in: enum(body,json_field,header,status; required), field: string, marker: string(required), uniqueness: enum(unique,strong,weak; required), zero_model_ok: boolean, auth_gated: boolean, confidence: enum(source_code,observed,documented,inferred; required), notes: string }[]"` | sniff: the ranked, machine-consumable answer to "which runner is listening on this port?" — see [Task 4](#task-4--port-identity-strategy-added-2026-07-19). **Adopted and landed 2026-07-19** (schema + all five docs) |

**Rejected** (with reason): release cadence (too volatile), VRAM sizing
guidance (model-specific, not runner-specific), API-key provisioning (covered
by `auth`/`auth_notes`/`config_files`), CORS defaults (covered by `env_vars`),
systemd/launchd unit names (covered by `platforms[].service`), container image
names (covered by `platforms[].install`).

---

## Task 3 — Lifecycle Hook Improvements

All proposals verified against `.claude/skills/claudine/lifecycle.md`, the
Darkmatter expression/side-effect docs, and the implementation
(`composition/lifecycle/control.rs`, `runtime.rs`,
`wrap/harness_orch/loop_control/*`). Anything not currently expressible is in
the [feature wishlist](#claudine--darkmatter-feature-wishlist) instead of being proposed with imaginary functions.

### Key semantics the proposals rest on

- **`validate_schema(file)` exists** as a read-side expression function and is
  usable in any lifecycle `when:` guard.
- **Flow control is universal** — `resume`/`retry`/`proxy` dispatch from
  `success` as well as `failure`. (The "resume is valid only in `failure`"
  line in lifecycle.md and the `actions.rs` rustdoc is stale drift; code wins.)
- **`resume` re-enters the same provider session** (OpenCode:
  `opencode run --session <id>`), and the resumed attempt's terminal event
  re-runs the full `success` stack — resumed work gets re-verified.
- **Budgets are shared slots per step**: all `resume` items (across success
  and failure stacks) share one attempt ceiling fixed at first firing; `retry`
  has a separate slot.
- **Success-side budget exhaustion quietly lets success stand** — so
  `resume`-from-`success` is a *soft* challenge only. Hard invariants must use
  `error:` (which routes through the `failure` event, carrying the reason in
  `err.msg` — facet-less, so match tokens with `contains(err.msg, 'TOKEN')`).
- **`err` is parse-rejected in `success`** — success-side checks read file
  state only.
- **`finalize`** is the "finally" stage; `initialize`-phase recovery is
  unsupported.

### Defects in `_fleet.md` as written (fix regardless of anything else)

1. **`{{err.message}}` is not a field** — the `failure.warn` renders
   `(err: )`. Use `{{err.code}}: {{err.msg}}`.
2. **Redundant complement guard** — success item 2's
   `when: "frontmatter(file,'last_updated') == ctx.today"` can never be false
   when reached (item 1's `error:` terminates the stack). Drop the guard.
   Same simplification applies to the mirrored `initialize` guards (put the
   guarded `skip` item first, make the message item unguarded).
3. **Missing `fail_fast: false`** — the default `true` halts the whole fleet
   on the first runner failure (e.g. the known OpenCode auto-reject). A
   research sweep wants the other runners to complete; the freshness gate
   already makes re-runs cheap.
4. **Stale schema comment** — `_schema.yaml:14-16` (nested mappings) per
   Task 2.

### P1 — Enforce schema validation in `success` (highest value)

The exit criteria *ask the agent* to run `md schema validate`; nothing
enforces it. A pure guard does, with no shell and no provider trust:

```yaml
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "NOT_STAMPED: research file was not updated"
        - when: "!validate_schema(file)"
          action:
              - stderr: "<b>{{file}}</b> fails <code>md schema validate</code> — the agent's exit-criteria claim was wrong."
              - error: "SCHEMA_INVALID: research frontmatter fails schema validation"
```

This is also the enforcement point for every Task 2 schema tightening —
notably the `since_version` pattern constraint (structural proposal 7).

### P2 — Detect-in-`success`, recover-in-`failure` (the architecture)

`success` detects and demotes with tokenized `error:` reasons; `failure`
recovers with bounded `retry`/`resume`. After the failure-side resume budget
exhausts, the failure stays terminal — unlike success-side exhaustion — so
hard invariants genuinely fail the step.

```yaml
failure:
    message: "💥 the Local Runners research on **{{state.name}}** failed to complete!"
    stack:
        - when: "err.is_transient || err.is_throttled"
          action:
              - warn: "Transient/throttled failure for **{{state.name}}** ({{err.code}}) — retrying"
              - action: retry
                max_attempts: 2
                delay: "60s"
                backoff: exponential
        - when: "err.category == 'timeout'"
          action:
              - resume: "You were stopped by a timeout. Continue from where you left off: finish the research document {{file}}, stamp `last_updated: {{ctx.today}}`, and re-run `md schema validate '{{file}}'` until it returns true."
        - when: "contains(err.msg, 'NOT_STAMPED') || contains(err.msg, 'SCHEMA_INVALID') || contains(err.msg, 'NO_CHANGELOG')"
          action:
              - resume: "Your run ended but verification failed: {{err.msg}}. If you were blocked reading {{state.user_dir}} by a permission denial, do NOT stop — record those probes as `confidence: documented` with the gap noted, and finish. Complete the document {{file}}, set `last_updated: {{ctx.today}}`, and keep fixing frontmatter until `md schema validate '{{file}}'` returns true."
        - action:
              - warn: "The Local Runners research on **{{state.name}}** failed: {{err.code}}: {{err.msg}}"
```

**Retry-vs-resume policy:** `resume` when the session has salvageable context
(verification miss, timeout); `retry` when the attempt was poisoned
(transient infra, throttle); fall through to a terminal warn otherwise.

### P3 — Soft spot-checks of surprising results (`resume` from `success`)

For *suspicion* rather than *violation*: challenge once; if the agent
re-verifies and the value persists, exhaustion lets success stand — which is
correct for a spot-check. All resume items share one budget, so fold every
suspicion into **one** item whose message enumerates only the triggered arms
(ternaries render at event time):

```yaml
success:
    stack:
        # …P1 hard checks first…
        - when: "(frontmatter(file,'requires_claudine_update') && is_empty(frontmatter(file,'reason'))) || (!update && !is_empty(frontmatter(file,'changes'))) || (update && is_empty(frontmatter(file,'changes'))) || is_empty(frontmatter(file,'traps'))"
          action:
              - info: "Challenging surprising results in {{file}} before accepting them"
              - resume: >-
                    Before this research is accepted, re-verify these suspicious results in {{file}}:
                    {{ frontmatter(file,'requires_claudine_update') && is_empty(frontmatter(file,'reason')) ? " (1) you set requires_claudine_update: true but left reason empty — state the reason or flip it to false." : "" }}
                    {{ !update && !is_empty(frontmatter(file,'changes')) ? " (2) this was a FRESH run yet changes is non-empty — changes must be [] on first research." : "" }}
                    {{ update && is_empty(frontmatter(file,'changes')) ? " (3) this was an UPDATE run yet changes is empty — either enumerate what changed since the last research, or add a Changelog entry stating explicitly that you re-verified and found no changes." : "" }}
                    {{ is_empty(frontmatter(file,'traps')) ? " (4) traps is [] — that is only valid after actively hunting for misleading knobs; name one place you looked, or fill the array." : "" }}
                    Fix what is wrong, leave what you have verified, restamp last_updated, and ensure `md schema validate '{{file}}'` still returns true.
        - action:
              - info: "The **Local Runners** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Local Runners** research on **{{state.name}}** completed successfully"
```

Cost caveat: a legitimately empty `traps` burns one bounded resume round-trip
— the price of the spot-check.

`since_version` hedge-detection **cannot** be expressed here (no array
projection/predicates in the expression engine) — it is handled by the schema
pattern constraint + P1 instead.

### P4 — Verify the Changelog grew on update runs

No expression function reads body sections, so pair a `shell` action with a
one-line prompt amendment (update-gated task block): *"the new `## Changelog`
entry's heading must begin with `### {{ctx.today}}`"* — then:

```yaml
success:
    stack:
        # …after P1, before P3…
        - when: "update"
          action:
              action: shell
              command: "grep -q '^### {{ctx.today}}' '{{file}}'"
              on_error: "NO_CHANGELOG: update run added no dated Changelog entry"
```

A failing grep routes to failure and P2's `NO_CHANGELOG` resume lane picks it
up. Clean non-shell version needs a body-section read function (wishlist #5).

### P5 — Stale-but-restamped guard

Catch an agent that bumps `last_updated` without doing research:

- **Free tier (available now):** P3's `update && is_empty(changes)` arm plus
  P4's dated-changelog grep already force an update run to either enumerate
  changes or write a dated "no changes" entry — a restamp with an untouched
  body fails P4 outright.
- **Strong tier (available but fragile):** capture the pre-run body hash via
  `$()` frontmatter expansion (`md hash` body segment), compare in a success
  `shell` action. Hazard: the pre-flight resolution ordering of a lifecycle
  `shell` referencing a `$()`-computed frontmatter key is not documented —
  validate with `--dry-run` before trusting. The clean version is a
  `body_hash(file)` expression function (wishlist #3).

### P6 — Durable failure telemetry for prompt-mining

`append_line` with event-time interpolation is the available channel (`err`
is in scope in `failure`/`finalize`; a structured `append_jsonl` record is
blocked — wishlist #8). Log **before** any control action ends the stack:

```yaml
failure:
    stack:
        - action:
              action: append_line
              file: "{{ctx.repo_root}}/claudine/docs/research/local_runners/_fleet-failures.tsv"
              line: "{{ctx.today}}\t{{state.name}}\t{{env.AGENT}}\t{{env.MODEL || 'default'}}\t{{err.code}}\t{{err.disposition}}\t{{err.msg}}"
              no_error: true
        # …then the P2 recovery lanes…
finalize:
    stack:
        - when: "err"
          action:
              action: append_line
              file: "{{ctx.repo_root}}/claudine/docs/research/local_runners/_fleet-failures.tsv"
              line: "{{ctx.today}}\t{{state.name}}\tFINAL\t{{timing.total_ms}}ms\t{{err.code}}\t{{err.msg}}"
              no_error: true
```

Failure rows without a matching FINAL row = "failed then recovered"; with one
= "terminally failed". That split is exactly what you mine to decide whether a
resume message or the base prompt needs strengthening.

### P7 — The `grant:`/OpenCode auto-reject failure mode

The documented real failure (`grant:` unimplemented → OpenCode auto-rejects
`external_directory` reads non-interactively → agent stops early with
success-ish output) has **no lifecycle-visible signature of its own** — the
step exits 0 and the expression engine cannot read the transcript. P1 converts
the premature stop into a tokenized failure (unfinished file), and P2's resume
message tells the resumed session how to proceed under a denial (degrade to
`confidence: documented`, don't stop). The real fix is `grant:` (wishlist #1);
until then the `--yolo` comment should stay loudly in place.

### Placement summary

- **`initialize`** — freshness gate only (recovery is unsupported there).
- **`success`** — verification: hard `error:` demotions first, then the single
  soft `resume` spot-check, then celebration. Never `err`.
- **`failure`** — first-position telemetry, then all recovery.
- **`finalize`** — terminal telemetry, guarded on `err`.
- **Sequence level** — `fail_fast: false`.

---

## Task 4 — Port-identity strategy (added 2026-07-19)

Motivation: many runners share default ports (8000: vLLM/oMLX; 8080:
llama.cpp/LocalAI; 1234: LM Studio/mistral.rs; 5000: TabbyAPI/
text-generation-webui), so a listening port never identifies the runner. The
fleet already carried the ingredients (`detection[].expect` markers, `traps`
warnings), but the *strategy* — which probes to run, in what order, with what
confidence — lived only in prose. This task makes it a first-class structured
fact.

### Findings (verified against live servers on this host + upstream source)

The single best identity probe per fleet runner, ranked with fallbacks:

| Runner | Port | Rank-1 probe (definitive) | Marker | Fallbacks |
|---|---|---|---|---|
| Ollama | 11434 | `GET /` | exact body `Ollama is running` | `GET /api/version` (Ollama-only path); `/api/tags` with real sha256 digests; no `Server` header (Go) |
| llama.cpp | 8080 | header check on any request | `Server: llama.cpp` (set on every response, even 503-loading; no opt-out) | `GET /props` → `build_info`; `/v1/models` → `"owned_by":"llamacpp"`; `/api/tags` empty-digest reverse-tell |
| LM Studio | 1234 | `GET /api/v0/models` | `/api/v0/*` namespace + `compatibility_type`/`state` fields | `/api/v1/models` (`models` root key); `/v1/models` → `"owned_by":"organization_owner"`; `X-Powered-By: Express` (weak) |
| oMLX | 8000 | `GET /health` | `engine_pool` object (ungated even with API key; works with zero models) | `/openapi.json` → `info.title: "oMLX API"`; `/api/status` 401 envelope (namespace is oMLX-only) |
| vLLM | 8000 | `GET /version` | `{"version":"0.x.x"}` on a vLLM-only path | `/openapi.json` path fingerprint (`/tokenize`, `/pooling`, `/rerank`); `/metrics` `vllm:` families |

Anti-patterns the research docs must call out explicitly:

- `GET /health` returning `{"status":"ok"}` (llama.cpp) or an empty 200
  (vLLM) — **liveness, not identity**. oMLX's JSON-bodied `/health` is the
  exception that proves the rule: its shape identifies, its status does not.
- `GET /v1/models` — the path is universal; only per-entry markers
  (`owned_by`) discriminate, and only some runners set distinctive values.
- `server: uvicorn` — shared by every FastAPI runner (vLLM, oMLX, LocalAI,
  SGLang, Lemonade); useless for disambiguation. Conversely llama.cpp's
  `Server: llama.cpp` and LM Studio's `X-Powered-By: Express` show headers
  *can* be first-class probes.
- Runner mimicry cuts both ways: llama.cpp mounts `/api/tags` (Ollama
  alias) but with empty `digest`/`modified_at` — a reverse-tell that
  *distinguishes* it from real Ollama when both probes are run.

Best probes for the six proposed roster additions (from the 2026-07-19
verification pass; the research docs must confirm each):

| Candidate | Port | Best probe | Marker |
|---|---|---|---|
| LocalAI | 8080 | `GET /readyz` (or `/healthz`) | auth-exempt k8s-style path + `/v1/models` OpenAI shape |
| SGLang | 30000 | `GET /get_model_info` | SGLang-specific keys (`model_path`, `is_generation`, `weight_version`) |
| Docker Model Runner | 12434 | `GET /engines/v1/models` | the `/engines/` path prefix itself |
| Jan | 1337 | `GET /v1/models` | weak — no documented app-specific endpoint; research must find one |
| Lemonade | 13305 | `GET /api/v1/health` (fallback `/live`) | Lemonade-only `/api/v1` prefix |
| KoboldCpp | 5001 | `GET /api/extra/version` | `{"result":"KoboldCpp","version":...}` — defeats its own Ollama impersonation |

### Representation (landed)

Structured in frontmatter as the `identity_probes` record list (shape in the
Task 2 table; full semantics documented in `_schema.yaml`): ranked probes with
`match_in` (`body`/`json_field`/`header`/`status`), the `field`/`marker`
pair, a `uniqueness` grade (`unique`/`strong`/`weak`), and the two
detector-critical booleans `zero_model_ok` and `auth_gated`. Prose mirrors it
in each doc's `## Detection` → `### Port identity` subsection. Both are now
required by the schema, so the fleet prompt captures them on the next run and
`validate_schema()` enforces them mechanically.

### Installed-version determination (landed 2026-07-19)

The companion question — "is it installed, and which version?" — was only
implicit in the fleet (version output lived in `detection[].expect` prose).
It is now structured as `version_probe` (shape in the Task 2 table). Live
verification on this host surfaced three traps the docs now record:

- **LM Studio:** `lms --version` prints `CLI commit: 0b2a176` — the CLI's git
  commit, **not** the app version (observed app version `0.4.12+1`, available
  only from the app-bundle plist on macOS). There is no running-server
  version endpoint at all.
- **llama.cpp:** `llama-server --version` prints `version: 8168 (723c71064)`
  — a build number + commit sha, not a semver — and emits Metal/backend init
  noise *before* the version line, so a parser must match the `version:`
  line anywhere in the output.
- **Ollama:** the CLI (`ollama version is 0.32.1`) and the long-lived server
  daemon can drift; the running server's version is an `identity_probes`
  concern (`GET /api/version`), the installed version a `version_probe` one
  — the two channels are recorded separately.

---

## Claudine / Darkmatter Feature Wishlist

Improvements blocked on missing capability, in rough priority order:

1. **`grant:` frontmatter** — scoped read permission for `{{state.user_dir}}`
   so the fleet stops requiring `--yolo`. The only real fix for the OpenCode
   auto-reject failure mode (P7).
2. **Author-supplied error facets on `error:`** — e.g.
   `error: { code: "research.not_stamped", message: "…" }`. Success-side
   demotions are currently facet-less, forcing failure guards to match tokens
   in `err.msg` prose — exactly what the faceted `err` contract exists to
   avoid.
3. **`body_hash(file)` expression function** (Darkmatter-hash based) — makes
   the stale-restamp guard (P5) a pure `when:` guard.
4. **Array projection/predicates** (`pluck` / `any` / `all`) — without them,
   per-record frontmatter rules are unverifiable from lifecycle guards.
   Interim: schema `pattern(...)` constraints + `validate_schema()` (adopted
   above for `since_version`).
5. **Body-section read function** — `has_heading(file, text)` or
   `markdown_section(file, name)` — replaces P4's grep and prompt amendment.
6. **Per-item / named resume budgets** — the soft spot-check resume and the
   failure-recovery resume currently share one slot whose ceiling is fixed by
   whichever fires first.
7. **Late-binding object channel for `append_jsonl`** — object arguments
   interpolate at compose time, where `err` does not exist; structured error
   telemetry is impossible today.
8. **`defer` backend** (rendezvous scheduler) — "site down / rate-limited →
   `defer: "2h"`" instead of burning retry budget.
9. **(Docs drift, not a feature)** — "resume is valid only in `failure`" in
   both lifecycle.md copies and the `actions.rs` `Resume` rustdoc contradicts
   the implemented universal flow-control contract; code wins, docs need
   fixing.

---

## Recommended Sequencing

The three task outcomes interlock; this order avoids re-running the fleet
twice:

1. **Fix `_fleet.md` defects now** (no design decisions needed):
   `err.message` → `err.msg`, drop the redundant success/initialize complement
   guards, add `fail_fast: false`, fix the stale `_schema.yaml` nested-mapping
   comment, add the changelog-heading prompt amendment (P4's prose half).
   **Status: ✅ landed 2026-07-19** — along with P1's `validate_schema()`
   success guard, the step-4 initialize schema gate, and the `identity_probes`
   schema fact + five-doc update (Task 4).
2. **Land the schema changes in one batch** (Task 2): additive proposals
   (2, 4, 5, 7) plus the two breaking ones (1's data migration,
   3's `opencode_example` restructure) plus the ten new facts. Batching
   matters because breaking changes force a document refresh — do it once.
3. **Land the lifecycle hardening** (Task 3 P1-P6). P1's `validate_schema()`
   guard is what makes step 2's tightened schema *enforced* rather than
   aspirational.
4. **Bridge the freshness gate to the schema** (coherence-pass addition):
   after a breaking schema change, existing docs are invalid but the 14-day
   freshness skip would still bypass them. Add `!validate_schema(file)` to the
   initialize "needs research" condition (and its complement to the skip arm)
   so schema-invalidated documents automatically re-enter the research pool:

   ```yaml
   - when: "!file_exists(file) || !validate_schema(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
   ```

   **Status: ✅ landed 2026-07-19** (the initialize skip arm was simplified
   to an unguarded fallback instead of a mirrored complement guard).

5. **Expand the roster** (Task 1's six adds) and run the fleet: the six new
   runners get researched once, under the final schema, with the hardened
   lifecycle; the five existing docs re-enter via step 4's gate.
6. **File the feature wishlist** items as Claudine `_unscheduled` fixes/
   features (notably `grant:`, error facets, `body_hash`).

### Coherence-pass findings (contradictions resolved)

- Task 2 internally labeled the `ollama_compatible` proposal both "Additive"
  and "Breaking" — resolved above: the schema edit is additive; the
  recommended re-classification of existing Ollama/llama.cpp records is a
  small data migration (absorbed into step 2's batch).
- Task 2's `version_probe` was shaped as a single record but justified by
  per-OS variance — corrected to a record list.
- Task 3 flagged `since_version` per-record rules as unverifiable from
  lifecycle guards; Task 2 independently verified `pattern(regex)` constraints
  exist. The two compose: constrain in `_schema.yaml`, enforce via P1. Adopted
  as structural proposal 7.
- Task 1's port-collision map and Ollama-API-emulation findings independently
  motivate Task 2's `port_conflict` fact and `ollama_compatible` enum member —
  mutually reinforcing, no conflicts.
- No contradictions between Task 3's lifecycle proposals and the fleet's
  documented constraints (`grant:` unimplemented, `defer` unimplemented,
  `err` unavailable in `success`) — proposals were verified against
  implementation semantics before inclusion.

## Sources

Task 1's full per-candidate source list (all fetched 2026-07-19):

- oMLX: https://github.com/jundot/omlx · https://omlx.ai/ · https://github.com/jundot/omlx/releases
- LocalAI: https://github.com/mudler/LocalAI · https://localai.io/basics/news/index.html · https://localai.io/installation/linux/
- Jan: https://www.jan.ai/docs/desktop/api-server · https://www.jan.ai/docs/desktop/troubleshooting
- Docker Model Runner: https://docs.docker.com/ai/model-runner/api-reference/ · https://www.docker.com/blog/how-we-designed-model-runner-and-whats-next/ · https://github.com/docker/model-runner
- SGLang: https://sgl-project-sglang-93.mintlify.app/backend/openai-compatible-api · https://qwen.readthedocs.io/en/latest/deployment/sglang.html
- Lemonade: https://github.com/lemonade-sdk/lemonade · https://lemonade-server.ai/docs/guide/configuration/ · https://news.ycombinator.com/item?id=47612724
- KoboldCpp: https://github.com/LostRuins/koboldcpp · https://github.com/LostRuins/koboldcpp/releases · https://docs.sillytavern.app/usage/api-connections/koboldcpp/
- llamafile revival: https://blog.mozilla.ai/llamafile-returns/ · https://github.com/mozilla-ai/llamafile/releases
- RamaLama: https://github.com/containers/ramalama · https://ramalama.ai/docs/commands/ramalama/serve/
- mistral.rs: https://github.com/ericlbuehler/mistral.rs
- text-generation-webui: https://github.com/oobabooga/text-generation-webui/wiki/12-%E2%80%90-OpenAI-API
- TGI archival: https://github.com/huggingface/text-generation-inference
- GPT4All dormancy: https://github.com/nomic-ai/gpt4all/issues/3558 · https://github.com/nomic-ai/gpt4all/issues/3605
- Cortex deprecation: https://github.com/janhq/cortex.cpp · https://www.jan.ai/changelog/2025-07-31-llamacpp-tutorials
- Xinference: https://github.com/xorbitsai/inference
- TabbyAPI: https://github.com/theroyallab/tabbyAPI · https://github.com/turboderp-org/exllamav3
- Nexa SDK: https://github.com/NexaAI/nexa-sdk
- mlx-lm server: https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/SERVER.md
- llama-swap: https://github.com/mostlygeek/llama-swap
- Msty: https://docs.msty.app/how-to-guides/get-the-latest-version-of-local-ai-service

Tasks 2 and 3 were grounded in repo sources: `darkmatter/lib/src/markdown/schemas/simplified/`, `.claude/skills/claudine/lifecycle.md`, `claudine/lib/src/composition/lifecycle/`, `claudine/cli/src/commands/wrap/harness_orch/loop_control/`, and the five existing research documents in this directory.

Task 4 (2026-07-19) was grounded in live probes on this host (Ollama 0.32.1
on :11434, oMLX 0.5.1 on :8000, LM Studio started briefly for `/api/v0`/
`/api/v1` probes then restored), the local llama.cpp checkout
(`tools/server/server.cpp` route table, `server-http.cpp:75` `Server`
header), and official docs/source: vLLM `api_server.py` +
docs.vllm.ai/en/latest/serving, LocalAI `core/http/app.go`, docs.sglang.ai,
docs.docker.com/ai/model-runner, jan.ai/docs/desktop/api-server,
lemonade-server.ai/docs + lemonade-sdk/lemonade releases (v10.1.0 port
change), github.com/LostRuins/koboldcpp + lite.koboldai.net/koboldcpp_api.
