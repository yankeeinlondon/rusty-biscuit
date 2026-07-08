# Supplemental Design: Provider Catalog Generation — Boundaries, Sources, and Lifecycle

> **Status:** draft for Ken's review. Refines spec.md "Architecture", "The generate
> step", "Codegen mechanics", "Research topics as typed contracts", "New provider
> onboarding flow", and rules on Open Questions 3, 4, and 5.
> Ratified inputs: F1 (shared types crate), F2 (hand-facts input), F6 (this doc's home).

## The field source matrix (resolves the drift-test paradox)

Every `ProviderInfo` field — current, table A, and table B — maps to **exactly one**
declared source. `data.rs` is wholly generated; the drift test covers all inputs; no
field is ever hand-edited in generated code.

| Source | File | Owner | Contents |
| --- | --- | --- | --- |
| **roster** | `docs/providers.yaml` | human | identity facts only: name, slug, binary, aliases, dirs, vendor, URLs, sniff binding |
| **research** | `docs/research/<topic>/<provider>.md` frontmatter | LLM fleet + evaluation | every field whose topic has landed (mapping registry declares topic→field) |
| **facts** | `docs/providers/facts/<slug>.yaml` | human | **topic-less values** — table-A fields whose research topic has not landed yet |
| **override** | `docs/providers/overrides/<slug>.yaml` | human | exceptional corrections that WIN over any other source |

Rules:

1. **Bootstrap:** the facts files are seeded once by a scraper that transcribes today's
   hand-written constants (one-time tool, then deleted). Day-one `data.rs` is a
   byte-equivalent regeneration from roster + facts. Partial generation is not allowed.
2. **Graduation:** when a field's research topic lands, the mapping registry re-points
   the field's source from facts → research, and generation **errors** if the facts file
   still carries that key (delete-on-graduate is enforced, not advisory).
3. **Collision = error, not precedence.** A value arriving from a source other than the
   field's declared one is a generation error. (Overrides are the single exception —
   they may override any field.) Example: URLs are roster-sourced; the agent-cli topic's
   homepage/docs URLs are *verification inputs* surfaced as drift warnings in the
   generate report, never merged.
4. **Override algebra:** whole-key replacement, scalars and lists alike — no deep merge
   in v1 (whole-list replacement is reviewable; deep merge is not). An override is
   validated against the same schema slice as the field it overrides and may not
   introduce a key with no matrix entry. Every override entry requires a `reason:`
   string; the generate report lists all active overrides (staleness lint: an override
   whose value now equals the research value is flagged for deletion).

## Gen crate dependency design (F1)

New leaf crate **`claudine/catalog-types`** (no deps beyond serde/strum):

- the coerced enums (`YoloSupport`, future `SandboxSupport`, `ConfigFormat`, …),
- the signal taxonomy types and shared vocab enums (`unit`, `zone`, `confidence` —
  canonical home, see design/signal-detection.md),
- the `DisplayPolicy` struct and `EventClass` enum (see design/render-components.md).

`claudine/gen` depends on catalog-types + darkmatter (schemas) + serde — still **no
dependency on the claudine lib**. The schema↔catalog compatibility check compares
sidecar `enum(...)` members against catalog-types variants via strum introspection, so
it fails before any research runs, as the spec requires. The claudine lib depends on
catalog-types too; generated `data.rs` references those types directly.

`--mapping` rendering: gen emits the mapping table as JSON; `claudine-cli` renders it
through renderable components. Gen stays render-free (no biscuit-terminal dep).

## Generate UX + drift lifecycle

- **Confirmation granularity:** per generated file (one `data.rs` per provider). The
  diff is shown; accept commits the regeneration.
- **Decline** is not a terminal state: declining a hunk requires, in the same session,
  either (a) scaffolding an override for the declined field (generator offers this), or
  (b) reverting the input that caused it. `generate` exits non-zero if declined drift
  remains unreconciled — inputs and committed output must always re-converge.
- **CI:** the generator runs in `--check` (report-only) mode — pure function of
  committed files, no keys, no network — and fails the build on drift. The drift test
  and `--check` are the same code path.

## Onboarding state machine (fixes the chicken-and-egg)

Generated code references `Provider::<New>`, which cannot exist before the enum
variant. The order is fixed, with compile checkpoints. **Amended against the
M-Kilo and M-Pi graduations (2026-07-07) — the real step list, not "three
edits":**

1. **Roster entry** (identity facts) — no code. Must carry `sniff_binding`,
   `docs_url`, and the other roster-sourced required fields; a missing one is a
   generation *error*, not a warning.
2. **`sniff::AiCli::<New>` variant** — an **external `sniff`-crate** change.
   `sniff_binding` compiles to `AiCli::<X>`; a genuinely new binary needs the
   variant (+ its `AI_CLI_INFO` row + `serde_key` arm) first, or the generated
   `data.rs` will not compile. Reusing another provider's binding mis-detects
   install status.
3. **Research fleets** for the new provider (topics that exist; gaps tracked).
4. **Hand wiring — compiler-walked, but via arrays, not match arms.** `Provider`
   is `#[non_exhaustive] #[repr(usize)]`, so exhaustiveness is enforced by
   fixed-length `[T; PROVIDER_COUNT]` tables, not `match` arms (a foreign-crate
   `match Provider` is forced to carry `_`, so it will **not** light up). The
   compiler-forced sites: `provider_id.rs` (variant + `PROVIDER_COUNT` +
   `PROVIDERS_DISPLAY_ORDER` + the discriminant const-assert),
   `provider/registry.rs` (`&<X>_INFO`), and the CLI `WRAPPER_REGISTRY` slot.
   Manual (not compiler-forced): `emit::PROVIDER_VARIANTS` (the slug→variant
   bridge — the one gen-side edit), the `provider/<slug>/` module decl, the clap
   subcommand (`args.rs` + `main.rs` `wrapper_command` + the `unreachable!` arm +
   `argv::WRAPPER_SUBCOMMANDS`), `telemetry.rs`'s two `Commands` matches, and any
   wildcard-bearing `match Provider` a wire-format reuse needs (e.g.
   `stream::providers::for_provider`).
5. **`claudine providers generate <slug> --scaffold`** — scaffolds `data.rs`
   (always overwritten), `mod.rs` + a compiling `behavior.rs` stub (**never**
   overwrites an existing `behavior.rs`), and a facts `TODO` skeleton for
   topic-less fields. **Gotcha:** fields whose research lands as freetext or a
   compound the coercion cannot consume — `model_catalog_source` (from a
   dynamic-listing boolean) and `model_cli_flag` (from a `"--x / -y"` compound
   site) — **hard-stop generation** until a field-keyed `overrides/<slug>.yaml`
   entry pins the value. This is a *required* step for any provider with dynamic
   listing, not optional, until the `agent-models` sidecar grows typed keys for
   them (tracked follow-up in the M-Kilo graduation report). **Aggregator
   providers (M-Pi):** a multi-provider front-end lists the same model id as the
   per-provider default for several providers, so `expected_offerings`
   (`default_models` → `DefaultModelsToExpectedOfferings`) hard-stops on a
   duplicate-id error. Per the collision-is-error rule, resolve it by *merging*
   the research — fold the duplicate `default_models` entries in
   `agent-models/<slug>.md` into one (record the second provider in the kept
   entry's notes), never by dropping a row. Expect `model_env_vars` and
   `non_interactive_conflicting_flags` to project empty for such a provider (no
   single bare model env var) — correct, no override.
6. **Implement behavior** where the provider genuinely differs. Consistency the
   test suite enforces — each is a one-line per-provider edit surfaced as a
   *failing test*, never a silent gap:
   - **Hook-support invariant** (`hook_events_imply_configurator_hooks_supported`):
     if the catalog's `event_mapping` declares any `Hook`-level event, the
     configurator MUST be real (`hooks_supported() == true` + a working
     `register`), never a no-op stub. (A `Hook`-level `event_mapping` is common
     when facts are copied from a parent — so an "easy cousin" still needs a real
     configurator.)
   - Catalog-consistency test sites, one line each: `discover_agents_full` count,
     `representative_payload_for` (payload detection — return `None` when the wire
     shape is ambiguous with another provider), `quick_start` hook selection,
     `claudine-contract` `support_matrix` length + the `auth_env_vars` arm, and
     the signals dormant-example test (swap to a still-unwired slug).
   - Re-bless `docs/providers/dispatch-inventory.json` (`CLAUDINE_UPDATE_INVENTORY=1`).
7. **Live-binary wrapper smoke — REQUIRED before graduation is final (M-Pi).**
   The compiler and test suite validate the data seam and the parser in the
   abstract, but they cannot catch argv-parser quirks or stub-default behavior.
   If a real provider binary is installable, do one dry-run (inspect the spawned
   argv) then one real streamed run, and confirm the four wrapper facts the
   research cannot reveal:
   - **Prompt delivery** — does the binary accept the prompt positionally, after
     `--`, or only on **stdin**? (Pi rejected both `--` and a `-`-leading
     positional; the prompt is stdin-only in `-p` mode — the Kilo-mirrored
     `AppendArgs(["--", prompt])` would have failed on every composed prompt.)
   - **Entrypoint flag vs stream selector** — the non-interactive entrypoint flag
     (`entrypoints.required_flags`, e.g. Pi's `-p`) is distinct from the
     structured-output selector (`--mode json`, sourced from `output_formats`);
     conflating them opens the TUI headless.
   - **System-prompt override** — the default `apply_system_prompt` is a stub that
     always reports "unsupported"; a provider that supports it must override the
     method (delegating to `apply_system_prompt_via_spec`) or the prompt is
     silently skipped with a warning. Confirm input tokens rise on a re-run.
   - **Resume selector** — the unattended-safe handle (Pi: `--session-id`, not the
     partial-UUID `--session` that can raise a cross-project fork prompt); verify a
     two-turn smoke appends to the same session file rather than forking.
   Land any corrections in the facts/wrapper and re-run `just test`; re-bless the
   inventory if imports shifted line numbers.

The spec's "steps 1–3 are mechanical" is retired: onboarding spans a roster
entry + a `sniff` variant + the wiring above + overrides + behavior + a handful
of per-provider test lines + one live-binary wrapper smoke. The **compiler** owns
the *structural* checklist; the **test suite** owns the *consistency* checklist;
the **live smoke** owns the argv/stub-default checklist neither can see.

## Offboarding state machine (removal & soft-deprecation)

Provider *removal* is the inverse of onboarding and is owned here, not by any
phase — a provider can be retired at any time. It is **not** the Phase-I CLI
drift guard (design/pipeline-dry.md), which only prevents *new* decentralized
dispatch from regrowing and deliberately exempts the clap Provider mapping and
the per-provider profile files. Adding **and removing** the `claudine <slug>`
subcommand + its switches is this machine's job (onboarding = add;
offboarding = remove); the guard never adds or removes a command.

Two retirement modes:

1. **Soft-deprecate (`skip_research: true`)** — keep the roster entry, the
   `Provider` variant, and **all** code + CLI wiring; only exclude the entry
   from research fleet fan-out. `claudine-gen` fails loudly if asked to generate
   a `skip_research` provider (its `data.rs` is frozen at last generation). Use
   for a keep-identity-but-pause hiatus, or an old major version kept for
   identity after its successor lands (Kimi v1→v2 precedent). **No CLI change** —
   `claudine <slug>` still wraps the (now research-frozen) binary.
2. **Full removal (the Roo precedent)** — tear down every site the onboarding
   footprint added. The **compiler** (the `[T; PROVIDER_COUNT]` arrays), the
   **gen drift test**, and the **consistency tests** walk the same structural +
   consistency checklists in reverse, so removal is compiler-walked exactly like
   onboarding; the manual (not-compiler-forced) sites are the same ones —
   `emit::PROVIDER_VARIANTS`, `signals::SIGNAL_SLUGS`, and the clap surface.

Full-removal teardown sites (inverse of the onboarding footprint):

- **Roster + inputs:** delete the `list:` entry, `facts/<slug>.yaml`,
  `overrides/<slug>.yaml`, and `docs/research/<topic>/<slug>.md` across every
  topic; remove `<slug>` from `signals::SIGNAL_SLUGS` (else the generated signals
  table is orphaned).
- **Enum / registry (compiler-forced):** remove the `Provider::<X>` variant,
  decrement `PROVIDER_COUNT`, drop it from `PROVIDERS_DISPLAY_ORDER` + the
  discriminant const-assert, and `provider/registry.rs` (`&<X>_INFO`).
  **Caveat:** removing a non-terminal variant *renumbers* every later
  discriminant — a wider blast radius than appending at the end (why Roo, at
  index 2, was removed deliberately). Prefer soft-deprecation if the only goal is
  to stop fleets.
- **CLI command + switches (the surface this note is about):** remove the clap
  subcommand in `args.rs`, the `main.rs` `wrapper_command` mapping arm **and** the
  `unreachable!` arm, the `argv::WRAPPER_SUBCOMMANDS` entry, both `telemetry.rs`
  arms, the `cli .../wrap/profile/<slug>.rs` file, and its `mod` / `use` /
  `static` / `WRAPPER_REGISTRY` slot.
- **Behavior + gen bridge:** delete `provider/<slug>/`,
  `stream/protocol/<slug>.rs`, `stream/providers/<slug>.rs` (+ the
  `for_provider` arm), `adapters/<slug>.rs` (+ `mod` + `*_ADAPTER`),
  `config/<slug>.rs` (+ `mod` + `use`); remove the `emit::PROVIDER_VARIANTS`
  entry (the one gen-side edit, mirror of onboarding).
- **sniff:** removing `AiCli::<X>` is *optional* — `sniff` detects agentic CLIs
  beyond Claudine's compiled set, so leave the variant unless nothing else
  references it (removing it renumbers `AiCli`, like the `Provider` caveat).
- **Tests + artifacts:** update `discover_agents_full` count,
  `representative_payload_for`, contract `support_matrix` length + the
  `auth_env_vars` arm, and the gen `provider_slugs` / wired-set tests; regenerate
  `data.rs` (all) + `catalog.json` + signals + families; re-bless
  `dispatch-inventory.json`; and re-seed the Phase-I CLI-guard inventory so the
  removed provider's exempted clap arm does not linger as stale drift.

## Compiled-subset mechanics (rules Open Question 4)

The runtime binary embeds **only** the compiled subset (`data.rs`). Gen additionally
emits a committed `docs/providers/catalog.json` — the full research projection
(all validated frontmatter, mapped and unmapped fields). Tooling that wants the
superset (`claudine providers --describe` enrichment, docs generation) reads
`catalog.json` from the repo when present and degrades gracefully when absent. The
claudine lib never reads research markdown at runtime.

## Rulings on Open Questions 3 and 5

- **OQ3 — `config_format`: per config-file entry.** Research reality already settled
  this: the model-config topic produces `config_files[]` records each carrying its own
  `format` (OpenCode alone spans jsonc/json/plist). The catalog field is
  `config_files: &[ConfigFileSpec]` (`PathTemplate` + format + scope), not a
  provider-level scalar. The topic doc's "per provider, Low effort" reading is retired.
- **OQ5 — the data/behavior litmus test (general, not just prompt delivery):** a fact
  is **data** if it is a pure selection among enumerable strategies plus string/scalar
  parameters, with no runtime control flow; it is **behavior** if delivering it
  requires sequencing, conditionals over runtime state, or side effects.
  Applied: `PromptDeliverySpec` *selection* (which strategy a provider uses) is a
  catalog enum field; the strategy *implementations* are permanent behavior-half code.
  This same test governs WrapperProfile migration (design/pipeline-dry.md).

## Confirmations folded in

- Overrides home `docs/providers/overrides/<slug>.yaml` — **confirmed** (spec OQ1).
- Generator home `claudine/gen` — **confirmed** (spec OQ2), with the catalog-types
  leaf crate added to the workspace.
