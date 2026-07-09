# Resume Prompt — Complete M-Antigravity Onboarding (→ Checkpoint H3)

Paste this into a fresh session to finish onboarding **Antigravity** as the 10th
compiled `Provider`. This is Phase H milestone #3 — the spec's **Goal-1
acceptance test** (a genuinely new provider through the full research→codegen→
behavior→smoke pipeline). **HOLD at ► CHECKPOINT H3 (Ken) when done.**

## Read first (in order)

1. Load the **`claudine`** agent skill.
2. `claudine/prompts/create-new-provider.md` — the canonical onboarding playbook.
   This prompt is the Antigravity-specific overlay; the playbook has the generic
   detail for every phase below.
3. `claudine/features/2026-07-02-provider-metadata/design/catalog-generation.md`
   — the authoritative onboarding state machine.
4. `m-pi-graduation.md` (bespoke non-cousin + live-binary smoke) and
   `m-kilo-graduation.md` (cousin) in this dir — the traps.

**Do NOT trust this prompt's claims about tooling/state — verify against the code
and the real binary first.** Every prior session found a doc contradicted by
reality. Fan out parallel Explore/survey subagents as needed.

## Antigravity specifics (critical)

- **Three surfaces.** Antigravity ships (a) a **headless CLI** `agy`, (b) a
  separate **Agent App** (GUI), and (c) a separate **Agentic Editor** (GUI). We
  wrap **`agy` only**.
- **GUI-launch hazard — do NOT launch the App or Editor.** The `antigravity`
  binary launches the desktop App; `agy` is the headless CLI. During an earlier
  signals research run (`-y`/yolo), the agent's "run the provider binary" step
  launched the desktop App and stole focus because the roster binary was wrong.
  It is now fixed. **Never** run `antigravity`, `open -a Antigravity`, or the
  `.app` bundle — for research OR the live smoke OR the wrapper. Only `agy`.
- **`agy` is at `~/.local/bin/agy` and is on PATH** (~142 MB, headless).
- **Authentication is keyring-based — no headless API-key mode (verified via web
  research).** `agy` caches its OAuth token in the **OS-native keyring** (macOS
  login **Keychain**), not a dotfile; non-secret config is at
  `~/.gemini/antigravity-cli/settings.json`. A spawned process with a
  sanitized/different `HOME` (or no access to the unlocked Keychain) can't read
  the token → `agy` launches an interactive **Google OAuth browser flow**. There
  is **no officially supported API-key/headless auth** (`GEMINI_API_KEY` is
  ignored; `ANTIGRAVITY_API_KEY`/`ANTIGRAVITY_TOKEN` are community-reported and
  unverified — Google says "use the SDK" for automation — GitHub issues #78/#57).
  **Wrapper implication (must handle at graduation):** claudine sanitizes env for
  wrapped providers; the `agy` wrapper MUST **preserve `HOME`** (and the macOS
  Keychain session) or a wrapped run triggers OAuth and hangs. The **live-binary
  smoke must verify a non-interactive `agy` run reuses the cached login** (no
  browser) with claudine's env sanitization applied.
- Roster entry already present in `docs/providers.yaml`: `slug: antigravity`,
  `binary: agy`, `display_name: Antigravity`, `vendor: Google`,
  `model_provider: true`, `sniff_binding: Antigravity` (the AiCli variant you
  add in Phase 3), `site`/`repo` set. `user_dir`/`repo_dir`/`docs_url` are
  **provisional** — confirm/correct them from the research.

## State already in place (verify, don't assume)

- Darkmatter date functions `date_delta` / `older_than` / `newer_than` shipped
  (committed); `claudine context --expressions` reports the **Date Arithmetic**
  category.
- All 19 `docs/research/*/_fleet.md` drivers now gate on a **14-day** freshness
  window via `date_delta` (committed), with a two-branch `message` initialize.
- `just run-fleet-research [agent]` recipe exists (each topic fleet `-y --codex`,
  3 in parallel; excludes `memory`). Ken is sourcing Antigravity research with it
  (backdating `last_updated` to force any stale doc).
- Nine wired providers today: `PROVIDER_COUNT == 9`. Antigravity becomes
  `Provider` index 9 (the 10th), `PROVIDER_COUNT` 9→10.

## Step 0 — Confirm research is sourced

Before any wiring, verify Ken's fleet runs actually produced Antigravity docs:

- `docs/research/<topic>/antigravity.md` exists for the **18 live topics**
  (every topic dir except `memory`), each `md schema validate`-clean, with a
  recent `last_updated`.
- Evaluate the docs (one subagent per doc; cross-check against the App-vs-CLI
  surface distinction — research must describe **`agy`**, not the GUI app).
- If any topic is missing/stale, tell Ken which — he re-runs
  `just run-fleet-research` (backdating that doc's `last_updated`). Do not run
  the fleet yourself without his go (it spends codex quota).

## Steps 1–6 — the onboarding machine (Antigravity overlay)

Follow `create-new-provider.md` Phases 3–8. Antigravity-specific notes:

1. **`sniff::AiCli::Antigravity`** (external `../sniff/lib`) — binary **`agy`**
   (not `antigravity`!). Add the variant + `AI_CLI_INFO` row + `serde_key` arm +
   install const; the `AiCli::COUNT == AI_CLI_INFO.len()` guard must pass. Do
   this FIRST or generated `data.rs` won't compile.
2. **Hand wiring** — `provider_id.rs` (`Antigravity = 9` + `PROVIDER_COUNT` 9→10
   + display order + discriminant assert), `provider/registry.rs`
   (`&ANTIGRAVITY_INFO`), CLI `WRAPPER_REGISTRY` slot; then the manual sites
   (`emit::PROVIDER_VARIANTS` `("antigravity","Antigravity")`, `provider/mod.rs`,
   clap args/main/argv, `telemetry.rs` ×2, `stream::providers::for_provider`,
   `adapters/mod.rs`, `config/mod.rs`).
3. **Scaffold + facts** — `claudine providers generate antigravity --scaffold`,
   fill the `TODO(required)` facts (incl. the `acp:` sub-record) from research.
   Watch the `model_catalog_source` / `model_cli_flag` hard-stops. **Antigravity
   is a Google/Gemini surface** — check whether `expected_offerings` needs the
   aggregator merge-not-drop reconciliation (duplicate `default_models` ids).
4. **Generate** — `claudine providers generate --yes` (regen ALL + `catalog.json`
   + guards); confirm `--check` is clean.
5. **Behavior half** — determine `agy`'s headless stream shape from research +
   the live smoke. It is a distinct CLI (likely **bespoke** parser/adapter/
   config, Pi-style — do not assume it's a Gemini-CLI fork without evidence).
   Satisfy the consistency tests (hook-support invariant, `representative_payload_for`,
   `discover_agents_full` count, contract `support_matrix` + `auth_env_vars`,
   signals dormant test) and re-bless `dispatch-inventory.json`
   (`CLAUDINE_UPDATE_INVENTORY=1`). Rendering stays provider-neutral
   (`DisplayPolicy` facts, no `match provider`).
6. **Live-binary smoke against `agy`** (REQUIRED; **never** the App/Editor) —
   dry-run to see the spawned argv, then a real streamed run. Confirm the four
   things codegen can't: prompt delivery (positional/`--`/stdin), entrypoint flag
   vs the structured-output selector, system-prompt override (the default
   `apply_system_prompt` stub reports "unsupported"), and the resume selector.
   Land fixes in facts/wrapper; re-run `just test` / `just lint`; re-bless the
   inventory if imports shifted line numbers.

## Step 7 — Graduation report + HOLD

Write `m-antigravity-graduation.md` (retro: scaffold quality, generate UX,
behavior-half shape, live-smoke findings on `agy`, mechanical footprint,
verification status — this is Goal-1, so note that the end-to-end
research→production pipeline closed). Then **HOLD at ► CHECKPOINT H3 (Ken)**. Do
not roll into Phase I.

## Guards / gotchas

- One-change gen discipline: registry + `emit` + regen-ALL + `catalog.json` +
  both field-list guards, together (`claudine providers generate --yes` / `--check`).
- A genuinely-new binary needs the `sniff::AiCli` variant FIRST.
- Trust a real `cargo check` / a real `agy` run over rust-analyzer diagnostics
  (they show phantom E0004/E0308 already fixed).
- `just test` / `just lint` from `claudine/`. Ignore host flakes only on exact
  match (see the flake list in `m-pi-graduation.md` / the prior next-session
  prompt).
- Never `cargo fmt`; never commit unless Ken says so in the session; subagents
  never commit.
