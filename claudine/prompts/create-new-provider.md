---
name: create-new-provider
description: Canonical playbook for onboarding a new agentic-CLI provider into the Claudine library + CLI.
---

# Create a New Provider for Claudine

You are onboarding a new agentic-CLI **provider** into Claudine (library + CLI).
This is the canonical, up-to-date playbook. Work top-to-bottom; each phase has a
compile/test checkpoint. Do **not** skip the live-binary smoke.

## Skills & authoritative references (read FIRST)

- Load the **`claudine`** agent skill (and `sniff`, `darkmatter`, `biscuit-file`
  as the task needs them).
- The onboarding **state machine** is the executable checklist:
  `claudine/features/2026-07-02-provider-metadata/design/catalog-generation.md`
  (section "Onboarding state machine"). It is authoritative over this prompt if
  they ever disagree — this prompt is the friendly summary.
- Architecture: `spec.md` in that same feature dir; the `claudine` skill's
  `architecture.md` + module map.
- Lessons from the last graduations (READ — they catalog the non-obvious traps):
  `m-kilo-graduation.md` (OpenCode-fork cousin) and `m-pi-graduation.md`
  (bespoke, non-cousin — and the live-binary smoke that caught four wrapper
  defects codegen could not).

## Rules of engagement

- **Verify, don't assume.** Every prior session in this workstream found a
  prompt/doc asserting something the code or the real binary contradicted. Trust
  a real `cargo check` / a real binary run over any doc or an IDE diagnostic.
- Never run `cargo fmt`. Never commit unless explicitly told. Subagents never
  commit.
- Provider identity source of truth is `docs/providers.yaml` (the **research
  roster**, which runs ahead of the compiled `Provider` enum). Generated
  `lib/src/provider/<slug>/data.rs` is NEVER hand-edited — only `behavior.rs`
  and the wrapper/parser/adapter/config are hand-written.

## Phase 0 — Identity & Wrap-ability

Claudine wraps a provider by spawning its **headless, non-interactive CLI** and
parsing its **structured stream** (JSON/NDJSON) on stdout. Before anything:

- Confirm the provider has such a CLI, and pin its **exact headless binary**.
- **GUI-surface hazard (the Antigravity lesson):** some providers ship a headless
  CLI *and* separate desktop surfaces (an editor, an "agent app", a `.app`
  bundle). The roster `binary` and every research/smoke invocation MUST target
  the **headless CLI only**. Never launch a GUI surface and never `open` a `.app`
  — with `-y`/yolo research the agent will otherwise foreground the desktop app
  and steal focus. If only a GUI surface exists, the provider is not wrappable
  as-is; stop and raise it.

## Phase 1 — Roster entry (`docs/providers.yaml`)

Add one `list:` entry (identity facts only). Required/typical fields, following
the existing entries:

- `name`, `file` (`<slug>.md`), `slug`, `binary` (the **headless CLI** binary —
  not an app launcher), `display_name`, `short_name`, `cli_aliases`, `docs_url`,
  `sniff_binding` (the `AiCli` variant name you add in Phase 3), `user_dir` /
  `repo_dir`, `vendor`, `model_provider`, `site`, `repo`.
- Landing an **active-but-unwired** entry is safe and expected (the Pi/Kilo
  pattern): `claudine-gen` generates only the *compiled* set, and
  `cross_validate_roster` treats an unwired active slug as informational
  (`unwired_active`), so `just test` stays green. (`skip_research: true` is only
  for deprecations/pauses.)

## Phase 2 — Research (incremental fleets)

The research topics live under `docs/research/<topic>/`, each driven by a
`_fleet.md` sequence over the roster. Each driver's `initialize` gate **skips**
any provider whose `last_updated` is fresher than 14 days (`date_delta`) and
**runs** any provider that is missing or stale — so a new provider sources across
every live topic while the existing roster is left alone.

- Run: `cd claudine && just run-fleet-research` (each topic fleet, `-y --codex`,
  three topics in parallel; `memory` is excluded on purpose). Override the
  researcher with `just run-fleet-research <agent>` (e.g. `--opencode`).
- To force a re-research of an existing doc, **backdate its `last_updated`**
  (never delete — deletion loses update-mode changelogs).
- **Yolo caution:** research runs `-y`. The `signals` fleet asks the agent to
  "run the provider binary" for live capture — this is exactly why the roster
  `binary` MUST be the headless CLI (Phase 0).
- Evaluate the produced docs (one subagent per doc is the proven pattern);
  `md schema validate '<doc>'` must return `true`.

## Phase 3 — `sniff::AiCli` variant (external `sniff` crate)

A genuinely new binary needs a cross-crate `sniff` change **before** generated
`data.rs` will compile. In `../sniff/lib`:

- Add `AiCli::<Variant>` (its binary name) + the `AI_CLI_INFO` row + the
  `serde_key` arm + the install const. The `AiCli::COUNT == AI_CLI_INFO.len()`
  guard must pass. Reusing another provider's binding mis-detects install status.

## Phase 4 — Hand wiring (compiler-forced via `[T; PROVIDER_COUNT]` arrays)

`Provider` is `#[non_exhaustive] #[repr(usize)]`; exhaustiveness is enforced by
fixed-length arrays, not `match` arms. Compiler-forced sites:

- `provider_id.rs`: variant `<X> = N` + bump `PROVIDER_COUNT` + `PROVIDERS_DISPLAY_ORDER` + the discriminant const-assert.
- `provider/registry.rs`: `&<X>_INFO`.
- CLI `WRAPPER_REGISTRY` slot.

Manual (NOT compiler-forced — easy to miss):

- `gen/src/emit.rs` `PROVIDER_VARIANTS` (`("<slug>","<Variant>")` — the one
  gen-side edit).
- `provider/mod.rs` (`mod <slug>`), clap (`args.rs` + `main.rs` `wrapper_command`
  + the `unreachable!` arm + `argv::WRAPPER_SUBCOMMANDS`), `telemetry.rs` (×2),
  `stream::providers::for_provider` arm, `adapters/mod.rs`, `config/mod.rs`.

## Phase 5 — Scaffold + generate

- `claudine providers generate <slug> --scaffold` writes `data.rs` (overwritten),
  `mod.rs` + a compiling `behavior.rs` stub (never overwrites), and a facts
  `TODO` skeleton — **including the mixed-source `acp:` sub-record**. Fill every
  `TODO(required)` fact from the research.
- **Hard-stops** (require a `docs/providers/overrides/<slug>.yaml` pin): a
  `model_catalog_source` from a dynamic-listing boolean, and a `model_cli_flag`
  from a compound `"--x / -y"` site. **Aggregator providers** hit a duplicate-id
  error on `expected_offerings` — reconcile the research by *merging* the
  duplicate `default_models` rows (never drop one).
- `claudine providers generate --yes` regenerates ALL providers + `catalog.json`
  + both field-list guards; `--check` is the CI/drift path. One change =
  registry + emit + regen-all + catalog + guards, together.

## Phase 6 — Behavior half

Reuse a cousin's parser/adapter/configurator when the provider is a fork
(Kilo reused OpenCode's verbatim); author bespoke when the wire format is new
(Pi's from-scratch NDJSON parser). The test suite surfaces each consistency site
as a *failing test*, one line each:

- **Hook-support invariant** (`hook_events_imply_configurator_hooks_supported`) —
  a `Hook`-level `event_mapping` requires a real configurator, never a stub.
- `discover_agents_full` count, `representative_payload_for` (return `None` when
  the wire shape is ambiguous), `quick_start` hook selection, contract
  `support_matrix` length + the `auth_env_vars` arm, the signals dormant test.
- Re-bless the dispatch census `docs/providers/dispatch-inventory.json`
  (`CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`).
  The scanner now covers **both** `lib/src` and `cli/src`.
- **Unified dispatch guard** (Phase I, `cli/tests/dispatch_inventory.rs` →
  `cli_dispatch_guard_holds_the_line`): the new per-provider
  `wrap/profile/<slug>.rs` and `permissions/providers/<slug>.rs` impl files are
  blanket-exempt automatically. But any **new conditional dispatch** the provider
  forces in a non-exempt file (a `match`/`matches!`/`==`/`!=` on the new variant —
  e.g. a wire quirk) fails the guard until you either migrate it to a
  `ProviderInfo` catalog field / behavior trait (preferred) or add a `keep` entry
  with a `reason` to `GUARD_ALLOWLIST`. Prefer the catalog: the whole point of the
  guard is to keep dispatch centralized.
- Rendering stays **provider-neutral** — fill `DisplayPolicy` facts, never
  `match provider` in render code.

## Phase 7 — Live-binary smoke (REQUIRED before graduation is final)

Codegen + tests validate the data seam and parser in the abstract; they cannot
catch argv-parser quirks or stub-default behavior. If the real **headless** CLI
is installable, do one dry-run (inspect the spawned argv) then one real streamed
run — **never a GUI surface** — and confirm the four things research cannot:

1. **Prompt delivery** — positional, after `--`, or **stdin**? (Pi rejected `--`
   and `-`-leading positionals; prompt is stdin-only in `-p` mode.)
2. **Entrypoint flag vs stream selector** — the non-interactive entrypoint flag
   is distinct from the `--output-format`/`--mode json` selector; conflating them
   opens the TUI headless.
3. **System-prompt override** — the default `apply_system_prompt` is a stub that
   reports "unsupported"; a provider that supports it must override the method.
4. **Resume selector** — the unattended-safe handle (verify a two-turn smoke
   appends to the same session, not a fork).

Land fixes in facts/wrapper, re-run `just test` / `just lint`, re-bless the
inventory if imports shifted line numbers.

## Phase 8 — Graduation report + HOLD

Write a graduation report (retro: scaffold quality, generate UX, behavior-half
shape, live-smoke findings, mechanical footprint, verification status). Then
**HOLD at the next Ken checkpoint** — do not roll into a subsequent milestone.

## Testing & known host flakes

- `just test` / `just lint` from `claudine/`; `just test-gen` for the generator.
- Ignore a host flake ONLY on exact match (see the current next-session prompt /
  graduation reports for the live list: `argv_normalization` passthrough
  handle-leak; the 3 `level2_tmux_*_chooser_detail`; sniff
  `detect_area_errors_when_not_in_repo`; etc.). Everything else is a real failure.
