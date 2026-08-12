# M-Kilo Graduation Report (Phase H, milestone #1)

> Checkpoint **H1** artifact. Kilo Code onboarded as the 8th compiled `Provider`
> through the new metadata/scaffold process. This report is the process retro:
> scaffold quality, generate UX, report accuracy, and the behavior-half
> decisions — the inputs Ken reviews before M-Pi.

## Outcome

`Provider::Kilo` is live: generation is byte-clean for all 8 providers, the lib
and CLI compile, and `claudine kilo` wraps the Kilo Code CLI with structured
streaming. Kilo entered with **complete research coverage** — all 17 topic
docs, including every one the mapping registry consumes (acp, agent-cli,
agent-logging, agent-models, model-config, non-interactive-sessions, resume,
skills). The graduation report's "missing research topics" set is therefore
**empty** — the cleanest possible ladder rung, by design (M-Kilo is the
smallest-delta cousin; the process is the test).

## The scaffold/generate UX was BUILT this milestone (it did not exist)

The plan text says `generate kilo --scaffold`, but verify-first found **no
`--scaffold` flow existed**: `claudine-gen` iterated a hardcoded `PROVIDER_SLUGS`
const of exactly 7, mirrored by `emit::PROVIDER_VARIANTS`; `generate kilo` was a
silent no-op. So building the scaffold UX was M-Kilo's first deliverable. What
landed (`gen/src/scaffold.rs` + `--scaffold` on `generate`):

- **Roster/variant-driven discovery.** The duplicate `PROVIDER_SLUGS` const is
  retired; `provider_slugs()` derives from `emit::PROVIDER_VARIANTS` (the single
  authoritative wired-set, order = `Provider` discriminant order). `check` now
  cross-validates against the roster: a wired slug missing an active roster
  entry is a loud error, and researched-but-unwired roster slugs print an
  informational line (now just `pi`).
- **`generate <slug> --scaffold`** writes, never-overwrite: `provider/<slug>/
  mod.rs`, a compiling `behavior.rs` stub (4 trait impls, only
  `detect_from_payload` filled, adapter/configurator TODO), and — if absent — a
  `facts/<slug>.yaml` TODO skeleton over the 21 facts-sourced registry fields.
  Then it generates `data.rs` as normal.

**Scaffold-quality observations (for the retro):**

- The behavior.rs stub is genuinely useful — it compiled as-is and gave a clear
  fill-in surface. Good.
- The facts skeleton emits the 21 `DeclaredSource::Facts` fields but **not**
  `acp` (whose `server_mode` is research-declared while `client_supported`/
  `events_via_acp` are facts-fed). A fully onboarded provider still needs the
  `acp` facts sub-record by hand. **Follow-up candidate:** teach the skeleton
  to emit the `acp` facts keys too, or document the gap in the scaffold output.
- Kilo didn't exercise the facts-skeleton path in anger (its facts were
  authored up front from research), so the skeleton is built + unit-tested but
  its ergonomics get their first real test at **M-Pi** (a non-cousin whose facts
  can't be copied from a sibling).

## Generate UX friction found (all resolvable, none blocking)

- **`model_catalog_source` needs a field-keyed override.** Kilo's agent-models
  research reports dynamic listing is available, but the coercion can't turn a
  boolean into a shell-command mechanism — it errors with an explicit "pin with
  an override" message. This is the documented opencode/codex/kimi pattern;
  `overrides/kilo.yaml` pins `kilo models`. Working as designed, but it means
  **every** provider with dynamic listing hits a hard generation stop until the
  override exists. Worth noting for M-Pi/Antigravity: the override is a required
  step, not optional. Longer-term the agent-models schema should grow a typed
  catalog-source key (tracked in the overrides' `reason:`).
- **`model_cli_flag` compound-site skip.** Research records `--model / -m`; the
  bare-token rule skips it, and the *research* value that survived was
  `--variant` (Kilo's reasoning-variant flag, not model selection). Pinned
  `--model` via override. Good that the skip is loud; mildly surprising that the
  surviving value was a semantically-wrong flag — the override caught it.

## Behavior-half decisions (Kilo as an OpenCode fork)

Kilo's CLI is an OpenCode fork, so behavior maximally reuses OpenCode's proven
infrastructure rather than duplicating it:

- **Rendering: zero per-provider code.** Kilo's event rendering is driven
  entirely by its generated `DisplayPolicy` facts — confirming the Phase-G
  thesis. No render-path change was needed.
- **Stream parser: reused.** `for_provider(Provider::Kilo, …)` returns the
  OpenCode NDJSON parser (Kilo's `--format json` is OpenCode-shaped).
- **Adapter: reused.** `provider_adapter()` returns `OPENCODE_ADAPTER` — Kilo
  events parse unchanged.
- **Detection: intentionally disabled.** `detect_from_payload → false`. Kilo's
  payloads are byte-identical in shape to OpenCode's, so raw-payload detection
  cannot uniquely identify Kilo; the wrapper path always knows the provider from
  the `claudine kilo` subcommand. (Enforced consistent via
  `representative_payload_for` returning `None`, like Goose/Qwen.)
- **Hooks: real, not stubbed.** Kilo's catalog declares `Hook`-level events
  (it has OpenCode's plugin bus), and the design invariant
  `hook_events_imply_configurator_hooks_supported` requires a matching
  configurator. So `KiloConfigurator` is a real plugin-bridge configurator
  (`@kilocode/plugin`, `~/.config/kilo/plugin/`), not a no-op — Kilo is a
  fully consistent hook provider (appears in `claudine init --quick`).
- **Wrapper contract.** `KiloWrapper`: `kilo run --auto --format json` +
  positional prompt. `--auto` is required — `kilo run` auto-rejects permissions
  by default, so without it a non-interactive run makes no progress;
  `--dangerously-skip-permissions` is the further `--yolo` escalation. Resume is
  `kilo run --session <id>` (scriptable, per session-resumption research).
- **MCP: not wired (documented gap).** Kilo has MCP (fork), but runtime MCP
  injection is a follow-up — `McpBehavior` uses defaults (unsupported). Matches
  the "smallest behavior delta" framing.

## Cross-crate dependency the plan understated

`sniff_binding` compiles to `AiCli::Kilo`, which required adding a **`Kilo`
variant to the external `sniff` crate** (`AiCli` enum + `AI_CLI_INFO` +
`serde_key`). The plan framed onboarding as claudine-only "3 hand edits"; in
practice a genuinely-new binary needs a `sniff::AiCli` variant first, or the
generated `data.rs` won't compile. Worth adding to the onboarding checklist.

## Facts judgment calls flagged for review (Kilo diverged from OpenCode)

Authored from Kilo research; all are **valid** catalog-types wire forms (they
generate), but are accuracy calls Ken may want to confirm:

- `acp.client_supported: false` — Kilo has *native* ACP (richer than OpenCode's
  none) but lacks `session/cancel` and needs client-side `request_permission`
  handling, so it is "not usable today". A case exists for `true` +
  `events_via_acp: [permission_request]` (cf. kimi). Left `false`; ACP is not
  wired for M-Kilo regardless.
- `billing_models: [subscription, per_token]` — Kilo Gateway also uses prepaid
  credits (402-on-empty). `prepaid_credits` is a valid variant; consider adding.
- `platform_kind: agent_aggregator` — Kilo aggregates 500+ models but also runs
  its own billing gateway; `vendor_platform` is defensible.
- `reasoning: not_documented` — Kilo has `--variant` (high/max/minimal) and
  `--thinking` (display-only), neither a clean `named_levels`/`binary_toggle`.
- `model_provider` omitted from the roster entry (unused by gen; ambiguous for
  an aggregator; kimi/pi omit it too).

## Onboarding footprint (the mechanical checklist, as walked)

Compiler-forced (`[T; PROVIDER_COUNT]` arrays, not match arms — `Provider` is
`#[non_exhaustive] #[repr(usize)]`): `provider_id.rs` (variant + `PROVIDER_COUNT`
7→8 + display-order + discriminant assert), `provider/registry.rs`
(`&KILO_INFO`), `cli wrap/profile/mod.rs` (`WRAPPER_REGISTRY` slot). Manual but
bounded: `provider/mod.rs` (`mod kilo`), clap (`args.rs`/`main.rs`/`argv`),
`telemetry.rs` (2 match arms), `emit::PROVIDER_VARIANTS`. Test updates:
`discover_agents_full` count 7→8, `representative_payload_for` (Kilo→None),
`quick_start` selection (+Kilo), signals `provider_recovers_wired` (dormant
example kilo→pi). Re-blessed `dispatch-inventory.json`.

## Verification status

- `claudine-gen check`: all 8 providers + catalog/signals/families **clean**
  (byte-identity held for the pre-existing 7).
- `cargo check -p claudine --all-targets` / `-p claudine-cli --all-targets`:
  clean.
- `just test`: **green** — lib 3319, contract 47, cli 1892 (1 handle-leak flake
  that retry-passed), gen 89/90, +19. Zero real failures. Fixed alongside the
  wiring: `discover_agents_full` count, `representative_payload_for`,
  `quick_start` selection, signals `provider_recovers_wired` (kilo→pi),
  contract `support_matrix`/`auth_env_vars` (+Kilo=KILO_API_KEY, len 7→8), and
  two gen tests that had used `kilo` as their unwired example (→`nonesuch`,
  M-Pi-proof).
- `just lint`: clean (clippy + error-transport guard) across all crates.
- End-to-end smoke: `claudine providers` renders the Kilo row (skills/commands/
  agents ✅, 13 hook events), `claudine kilo --help` is recognized, and
  `hooks --support` lists Kilo's native events — all from DisplayPolicy facts,
  no per-provider render code (Phase-G thesis confirmed).
- Known host flakes excepted per the phase brief: the 3
  `level2_tmux_*_chooser_detail` tests fail on clean HEAD (pre-existing, L2 only).

## Recommended follow-ups (not blocking H1)

1. Facts-skeleton should emit `acp` keys (or flag the gap) — tested for real at M-Pi.
2. Add `sniff::AiCli` variant to the documented onboarding checklist.
3. Kilo MCP runtime injection (fork parity) — deferred.
4. Reconsider the flagged facts accuracy calls (acp/billing/platform_kind).
5. **`agent-models` sidecar schema tightenings** (retire recurring hand-overrides
   across *all* providers, not just Kilo — the fleets ask the right questions but
   two answers land in a shape the coercion can't consume). **Capture side DONE
   (2026-07-07)** — `_fleet.md` prompt + `_schema.yaml` tightened, verified
   non-breaking (`claudine-gen check` clean, existing docs still validate,
   generated output byte-identical, overrides intact):
   - `dynamic_listing` gained `list_program` + `list_args` (structured shell
     mechanism → `ModelCatalogSource::ShellCommand`) and `rest_endpoint` (the
     REST case — e.g. Kimi's `GET /v1/models`, whose shell catalog source is
     correctly `none`), replacing "boolean + freetext `method`".
   - `model_selection` gained `aliases`, and the "one canonical site per record,
     no compound" rule (previously env-var-only) now covers `cli_flag` — the exact
     gap that let Kilo's `"--model / -m"` skip and a wrong alternate (`--variant`)
     survive.

   **Consumption side REMAINS** (retires the overrides — sequence before/with
   M-Pi or at Phase I): re-run the agent-models fleet to populate the new fields,
   teach the gen coercion (registry) to build `model_catalog_source`/
   `model_cli_flag` from them, regenerate `data.rs`, delete the now-redundant
   `overrides/*.yaml` model entries (kilo ×2, kimi/opencode/codex catalog-source).

   The dropped `fork_of` lineage idea is intentionally **not** here: reuse
   decisions came from Kilo's own per-topic research facts (which independently
   record the OpenCode-shaped wire format *and* the divergences), so lineage
   would add no machine-actionable signal — and would falsely imply reuse is safe
   where divergence is exactly what lineage can't express. This reinforces the
   fleet's per-provider-independence design.
