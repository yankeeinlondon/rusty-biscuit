# Schema v2 Approvals — Detailed Review Package

> **Status:** awaiting Ken's per-item decisions (2026-07-05). Approving an item
> here means: the topic's `_schema.yaml` sidecar and `_fleet.md` prompt get the
> described addition, and the topic joins the closeout track's next
> **update-mode refresh fleet**. Nothing in this document changes Claudine code
> by itself — the payoff arrives in two steps: (1) the refreshed research docs
> carry new typed frontmatter, then (2) a Phase D round graduates the matching
> catalog fields and deletes override pins, with the usual regenerate-and-review
> diff.
>
> Execution note: refresh fleets are run by the **topics-closeout track**
> (`claudine sequence` over each topic's `_fleet.md`), not by the
> provider-metadata sessions. The permissions widening of 2026-07-02 → fleet run
> of 2026-07-03 is the precedent: schema additions land as **optional fields**,
> so existing docs stay valid until the fleet populates them.

## Glossary (the jargon, in one place)

- **Topic** — one research subject directory under `docs/research/<topic>/`
  (e.g. `agent-models`), holding one Markdown doc per provider.
- **Sidecar / `_schema.yaml`** — the machine-validated contract for each doc's
  YAML frontmatter. `md schema validate` enforces it; the generator's
  compatibility gate reads it.
- **Fleet / `_fleet.md`** — the `claudine sequence` driver that re-researches
  the topic across all roster providers. A **refresh** runs it in update mode:
  the agent re-verifies everything against current docs/source/behavior and
  records deltas in a changelog.
- **Graduation** — switching a catalog field's declared source from
  hand-maintained **facts** (`docs/providers/facts/<slug>.yaml`) to
  **research** (topic frontmatter). Graduated fields become self-updating:
  every future refresh re-derives them and regeneration surfaces drift as a
  reviewable diff. Facts fields, by contrast, rot silently until a human
  notices.
- **Coercion** — the named generator rule that turns a research value into the
  catalog shape (e.g. `CliFlagSitesToFlag` picks the model flag out of
  `model_selection` records).
- **Skip-loudly** — a coercion that cannot use a record (annotated prose,
  compound value) must report it in the generate output rather than silently
  dropping it (Checkpoint A ruling 4).
- **Override pin** — a field-keyed `{value, reason}` entry in
  `docs/providers/overrides/<slug>.yaml` that wins over the declared source.
  Pins are debt: each one exists because the source data can't express the
  truth yet. Several proposals below exist specifically to retire pins.
- **Freeze risk** — the cost of typing a vocabulary too early: once an enum is
  in the sidecar, every provider doc and the generator gate depend on it, so a
  wrong guess is expensive to walk back. The mitigation pattern (used by the
  permissions widening) is: free-form string on the first pass → derive the
  enum from observed fleet data → tighten.

## Shared cost model

Every approved topic costs **one refresh fleet run** (9 providers, agentic
research sessions — real tokens and wall-clock). Costs amortize: all additions
to one topic ride the same refresh. The matrix's standing rule applies —
*graduate when the topic gains a typed key at its next scheduled refresh; do
not force a refresh for one field alone.* That is why this package bundles
per-topic rather than per-field.

The generic tradeoff for every item is the same triangle:

- **facts (status quo):** zero fleet cost, human-owned, rots silently.
- **typed research key:** self-updating after each refresh, diff-reviewed; costs
  a refresh + carries freeze risk on any enum it introduces.
- **override pin:** stopgap where research exists but can't express the truth;
  the honest thing is to fix the schema so the pin can die.

---

## 1. agent-permissions v2

The widened schema (2026-07-02) already landed and its fleet already ran
(2026-07-03) — this section is the *next* increment on top of that data.

### 1a. Typed YOLO switch sites

- **What exists today.** The sidecar's `yolo` record carries
  `{has_interactive_yolo, has_non_interactive_yolo, mechanism}` where
  `mechanism` is prose — e.g. claude's is effectively "`--permission-mode
  bypassPermissions` or `--dangerously-skip-permissions`; …". The catalog's
  `yolo: YoloSupport` field (variants like direct-flag / env-var /
  non-interactive-only / none) is **facts-fed** because prose can't drive a
  typed coercion.
- **Proposed addition.** A `yolo_switches` record array:
  `{ kind: enum(cli_flag, env_var, config_key, mode_value), site: string,
  scope: enum(interactive, non_interactive, both), notes: string }[]` — one
  record per switch, `site` restricted to the bare token (mirroring the
  bare-flag discipline that made `model_cli_flag` coercible).
- **What it unlocks.** `yolo` graduates research-fed: a provider adding or
  renaming its YOLO flag (this happens — approval-mode surfaces churn) shows up
  as a regeneration diff instead of a stale wrapper behavior.
- **Tradeoffs.** The `kind` enum is small and stable (low freeze risk). The
  main cost is coercion design: `YoloSupport` must be derivable from the
  records (direct_flag = any cli_flag record; non_interactive_only = no
  interactive-scoped record; etc.). Alternative: stay facts — defensible, YOLO
  mechanisms churn slower than model lists.
- **Recommendation: approve.** Medium priority; clean unlock, small enum.

### 1b. Six-axis classification → `cli_sensitive_axes`

- **What exists today.** The catalog's `cli_sensitive_axes` is 10 booleans over
  *Claudine's own PolicyEngine axis taxonomy* (read_path, write_path,
  traverse_path, execute_command, access_domain, …) — facts-fed. The permissions
  summary proposed a six-axis classification as the graduation vehicle.
- **The honest problem.** The axes are **Claudine's taxonomy, not an observable
  provider property**. A research fleet can attest *evidence* ("provider has
  path-scoped write rules") but the mapping onto our 10 booleans is a Claudine
  engineering judgment — the same ownership argument that made
  `supports_interactive_inline_closure` and `allowed_env_keys` permanent facts.
  There is also a 10-vs-6 mismatch between the catalog field and the summary's
  proposal that nobody has reconciled.
- **Options.**
  - (i) Add the six-axis records to the sidecar and graduate — self-updating,
    but delegates a Claudine judgment to fleet agents and forces the 10→6
    reconciliation now.
  - (ii) **Keep `cli_sensitive_axes` facts permanently**; the widened schema's
    `permission_entities` / `rule_model` / `tool_visibility` records (already
    landed) serve as review *evidence* when a human updates the booleans —
    the `allowed_env_keys` pattern.
- **Recommendation: (ii) keep facts — drop this from v2.** Revisit only if the
  PolicyEngine six-axis work (the thing that un-deferred `sandbox` would also
  wait for) produces a taxonomy both sides agree on.

### 1c + 1d. `env_vars.effect` enum and `precedence.scope` enum (scheduled follow-ups)

- **What exists today.** Both were deliberately left free-form on the first
  pass ("we will enum it after the fleet lands" — the fleet has now landed).
  Example: claude's `env_vars[0].effect` is a two-sentence prose description.
- **Proposed.** A derivation pass over the landed data (all 9 docs) produces
  candidate enums; you ratify them; the sidecar tightens; the *next* refresh
  validates against them. No forced fleet — this piggybacks.
- **Tradeoffs.** Pure upside except analyst time; the free-form→enum path was
  the plan all along. The only decision is timing.
- **Recommendation: approve the derivation pass** (it can run in D-1.5 as an
  analysis task); tighten the sidecar at the next natural permissions refresh.

> **Executed 2026-07-05.** The derivation pass and backfill ran the same day as
> approval (docs-only task): `effect_category` was added as an **optional** enum
> beside the kept `effect` prose and backfilled across all 106 env_vars records,
> and `precedence[].scope` was tightened to the ratified enum with every scope
> token normalized in place across all 74 precedence records (per-record `tools`
> collisions resolved: claude/cli → `tool_visibility`, kilo/config_directories →
> `customization_resources`). Sidecar and `_fleet.md` capture instructions
> updated; all 9 docs validate clean against the tightened sidecar.
>
> Ratified `effect_category` variants (16): `sandbox_control`, `none`,
> `state_home_relocation`, `config_path_override`, `tool_surface`,
> `security_hardening`, `customization_lockdown`, `credential`,
> `threat_detection`, `policy_overlay`, `config_injection`,
> `config_source_toggle`, `approval_mode`, `network_control`,
> `workspace_trust`, `other`.
>
> Ratified `scope` variants (18): `rules`, `mcp`, `approval_mode`, `sandbox`,
> `tool_visibility`, `general_config`, `config_loading`, `agents`,
> `extensions`, `hooks`, `skills`, `slash_commands`,
> `customization_resources`, `security_controls`, `trust`, `workspace`,
> `provider_model`, `other`.

---

## 2. non-interactive-sessions (NIS) v2

The highest-value package: four catalog fields and the noise-prefix family all
graduate from this one topic, and it fixes the emptiest wave-1 field.

### 2a. `conflicting_flags` record shape (bare flag + condition)

- **What exists today.** `claudine_strategy.conflicting_flags` is a string
  array mixing bare flags with annotated prose. Verbatim examples of what the
  wave-1 skip-loudly coercion had to discard: claude `"--output-format json
  for live wrapping"`, codex `"no --json for live parsing"`, goose
  `"GOOSE_MODE=approve"`, opencode `"ACP parser on run stdout"`. Result: the
  catalog's `non_interactive_conflicting_flags` is honest but **empty for
  claude and codex** — every one of their entries was annotated.
- **Proposed addition.** Replace the string array with records:
  `{ flag: string (bare token, machine-checkable), when: string (the condition
  that made the annotation valuable), kind: enum(cli_flag, env_var, other) }[]`.
  This *keeps* the conditional knowledge (which is real and useful) instead of
  banning it — the annotations were information, just in the wrong field.
- **What it unlocks.** The coercion consumes `flag` where `kind == cli_flag`;
  claude/codex stop being empty; the loud-skip list shrinks toward zero.
- **Tradeoffs.** Slightly heavier authoring per record; `kind` enum is trivial.
  Alternative (bare-string mandate only, drop conditions) loses information —
  not recommended.
- **Recommendation: approve with the record shape.**

### 2b. Typed noise-prefix keys — stdout graduates, stderr does NOT

- **What exists today.** `io_contract.noise_handling` is prose. The catalog now
  has `stdout_noise_prefixes` / `stderr_noise_prefixes` as facts (values copied
  from the retired profile overrides — e.g. gemini's five stdout prefixes,
  opencode's five TUI glyph prefixes on stderr).
- **Proposed addition.** A typed `noise` key:
  `{ stream: enum(stdout, stderr), prefixes: string[], notes: string }[]`.
- **The critical asymmetry (Checkpoint B ruling, still binding).**
  `stdout_noise_prefixes` was ruled "facts now, research later — graduate when
  NIS gains a typed key." But `stderr_noise_prefixes` was ruled **curated facts
  permanently**, because OpenCode's stderr is simultaneously a noise-filtered
  surface AND a promoted lifecycle-evidence channel (`--print-logs` carries
  model/permission/cap/auth signals). An auto-scraped stderr list could
  silently eat wrapper-grade evidence. So: the typed key feeds the **stdout**
  coercion directly, and serves only as **review input** for the hand-ruled
  stderr list.
- **Tradeoffs.** Prefix lists drift with provider releases — exactly what
  research-feeding fixes for stdout. The stderr asymmetry costs a little
  explanation forever, but it is the safe design.
- **Recommendation: approve** (stdout graduation + stderr-as-evidence).

### 2c. `output_formats[]` enrichment: `selector.kind`, `stdin_supported`, `companion_flags`

- **What exists today.** NIS records carry `name/cli_value/stream/format`; the
  catalog's `output_formats` (facts) additionally carries the selector kind
  (flag-value vs bare flag vs transport flag vs default), stdin support, and —
  since Phase D step 3 — `companion_flags` (claude's Stream record holds
  `["--print", "--verbose"]`, and the derived `apply_structured_stream`
  default consumes it).
- **Proposed addition.** Bring the research records to parity:
  `selector: { kind: enum(flag_value, flag, transport_flag, default),
  flag: string }`, `stdin_supported: boolean`, `companion_flags: string[]`.
- **What it unlocks.** `output_formats` — the largest, most drift-prone
  structured facts field — graduates. This matters more since step 3: the
  wrapper's structured-stream argv is now *derived from these records*, so a
  provider changing its stream flag currently requires a human to notice and
  edit facts. Research-fed, it becomes a refresh-diff.
- **Tradeoffs.** The `selector.kind` enum is Claudine vocabulary being pushed
  into research — a mild ownership smell, but unlike 1b these are directly
  observable CLI properties (is it `--output-format stream-json` or a bare
  `--json`?), so fleet agents can attest them reliably. Freeze risk low: the
  four kinds cover every provider we've met, and codex's step-3 facts fix
  showed the record shape works.
- **Recommendation: approve.** Highest-value single item in this package.

### 2d. `invocation[]` subcommand split

- **What exists today.** NIS `invocation[]` carries command/stdin/prompt_arg;
  the catalog's `entrypoints` (facts) needs the split between a *subcommand*
  (`codex exec`, `goose run`) and *required flags* — not expressible today.
- **Proposed addition.** `subcommand: string` and `required_flags: string[]`
  fields on invocation records.
- **What it unlocks.** `entrypoints` graduates; the codex `exec`-insertion and
  goose `run`-positioning behavior code keeps reading typed catalog data that
  now tracks provider releases.
- **Tradeoffs.** Small, observable, low freeze risk.
- **Recommendation: approve.**

### 2e. Typed `model_required_in_non_tty`

- **What exists today.** Known only as `claudine_strategy` prose; the catalog
  bool is facts-fed (opencode `true`) and — since step 3 — gates the shared
  model-resolution prep stage.
- **Proposed addition.** A boolean key with a `notes` string (resolution order
  evidence).
- **Tradeoffs.** Trivially observable; the matrix already said "graduate
  opportunistically at the next refresh."
- **Recommendation: approve.**

### 2f. Framing vocabulary check (no new field)

Your option-b ruling makes NIS the future source of `stream_protocol` with
framing vocabulary (`ndjson`/`jsonl`/`json-rpc`). NIS already carries
`data_format` — the v2 pass should just confirm the sidecar constrains it to a
closed enum matching the ruled vocabulary, so the graduation (executed at its
own moment, per the ruling) has clean input. **Recommendation: approve
(bookkeeping).**

---

## 3. agent-models refresh mandates

No new fields — discipline changes to existing records, executed at the next
refresh. Retires override pins.

### 3a. One bare flag per `cli_flag` site + a `selects` role

- **What exists today.** Sites arrive compound and annotated — verbatim from
  the wave-1 loud skips: codex `"--model, -m"`, gemini/opencode
  `"--model / -m"`, goose `"--provider  (goose run)"`. Four `model_cli_flag`
  override pins exist solely because of this, and goose's has a second cause:
  its first cli_flag record is `--provider` (the aggregator provider/model
  pair), which a mechanical first-record rule would wrongly select.
- **Proposed.** Fleet-prompt mandate: `site` is exactly one bare flag token
  (short forms go in `example`/`notes`), PLUS a
  `selects: enum(model, provider, profile, effort, other)` field on
  `model_selection` records so the coercion filters `selects == model` instead
  of guessing by position.
- **What it unlocks.** All four `model_cli_flag` pins die at the refresh after
  this lands; the goose ambiguity is fixed *structurally*, not by pin.
- **Tradeoffs.** None real — this mirrors the already-ratified
  one-env-var-per-record mandate (Checkpoint A ruling 4, already in the fleet
  prompt and executing at the next refresh regardless).
- **Recommendation: approve.**

### 3b. `role` on env-var selection records

- **What exists today.** goose's `model_env_vars` pin exists because research
  correctly lists `GOOSE_PROVIDER` and `GOOSE_FAST_MODEL` beside `GOOSE_MODEL`
  and the coercion can't tell the model selector from the provider selector or
  the auxiliary-model knob.
- **Proposed.** `role: enum(model, provider, auxiliary_model, other)` on
  env-var records (or fold into 3a's `selects`).
- **What it unlocks — and honestly, what it doesn't.** Goose's env pin dies.
  The other four env pins (claude/codex/kimi/opencode/qwen) do **not** die:
  they pin *Claudine's wrapper conventions* (`KIMI_MODEL`, `CODEX_MODEL`, …)
  which differ from provider-native vars by design — those await the separate
  "wrapper env contract redesign" backlog item, not a schema change.
- **Recommendation: approve** (with the scope caveat above understood).

---

## 4. system-prompt v2

### 4a. Delivery site fields + interactive/non-interactive split

- **What exists today.** The topic's `claudine_delivery` carries the strategy
  *kinds* (aligning 1:1 with the catalog `SystemPromptSpec` variants) but not
  the flag/key/env-var *sites*, and has no interactive vs non-interactive
  split. The catalog field — append/replace × interactive/non-interactive
  delivery plus memory files — is facts-fed and is the most structurally
  complex spec in the catalog.
- **Proposed addition.** Per-mode delivery records:
  `{ mode: enum(append, replace), session: enum(interactive, non_interactive),
  strategy: <existing kinds enum>, site: string, notes: string }[]`.
- **What it unlocks.** `system_prompt` graduates — and this topic is where the
  summary-triage backlog shows the most churn (Kimi lost its replace surface
  across the product split; Goose's surfaces are version-hedged), i.e. exactly
  the drift research-feeding catches.
- **Tradeoffs.** Highest coercion complexity in the package: the schema must
  mirror a two-axis spec, and the current facts values are *battle-tested by
  every live wrapper run* — a bad graduation here has more blast radius than
  anywhere else. Mitigation: the regeneration diff review, plus the wrapper's
  system-prompt tests.
- **Recommendation: approve, but sequence LAST** (after 2x and 3x prove the v2
  cycle end-to-end). Keeping it facts another round is a reasonable "no".

---

## Explicitly NOT in this package

- **Rulings, not schemas** (coming to you in D-1.5 instead):
  `session_locations` semantics, the `memory_files` extraction rule, `acp`
  vocabulary reconciliation.
- **Deliberately permanent facts** (ruled at Checkpoint B): `allowed_env_keys`
  (security allowlist — never auto-widened), `stderr_noise_prefixes` (curated;
  see 2b), `supports_interactive_inline_closure`, `known_gaps`,
  `repo_home_root_files`, `billing_models`, `platform_kind`, `reasoning` (no
  topic carries it).
- **Coupled to other work:** `event_mapping` (moves with the Codex hooks `[S]`
  item), `resource_support` (moves with Phase D linking work), `sandbox`
  (deferred by your Checkpoint D ruling until a consumer exists).

## Decision sheet

Mark each; "approve" means the sidecar/fleet-prompt edits get drafted and the
topic queues for its next closeout refresh.

| # | Item | My recommendation | Your decision |
| --- | --- | --- | --- |
| 1a | permissions: typed YOLO switch sites | approve | |
| 1b | permissions: six-axis → `cli_sensitive_axes` | **reject — keep facts** | |
| 1c/1d | permissions: effect + scope enum derivation | approve (analysis in D-1.5) | approved (Ken, 2026-07-05) |
| 2a | NIS: `conflicting_flags` → `{flag, when, kind}` records | approve | |
| 2b | NIS: typed noise key (stdout graduates; stderr stays curated) | approve | |
| 2c | NIS: `output_formats` parity (selector/stdin/companions) | approve — highest value | |
| 2d | NIS: invocation subcommand split | approve | |
| 2e | NIS: typed `model_required_in_non_tty` | approve | |
| 2f | NIS: framing-vocab enum check | approve (bookkeeping) | |
| 3a | agent-models: bare-flag mandate + `selects` role | approve | |
| 3b | agent-models: env-var `role` | approve (goose-only payoff) | |
| 4a | system-prompt: delivery sites + session split | approve, sequence last | |
