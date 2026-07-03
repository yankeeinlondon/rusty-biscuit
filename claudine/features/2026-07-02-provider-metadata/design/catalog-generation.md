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
variant. The order is therefore fixed, with compile checkpoints:

1. **Roster entry** (identity facts) — no code.
2. **Research fleets** for the new provider (topics that exist; gaps tracked).
3. **Hand wiring (small, manual):** add the `Provider` variant + `provider/<slug>/`
   module declaration + clap arm. Compile checkpoint: exhaustiveness errors enumerate
   every remaining structural site — this IS the TODO list (no separate artifact).
4. **`claudine providers generate <slug>`** — scaffolds `data.rs` (always overwritten)
   and `behavior.rs` stubs (**never** overwrites an existing `behavior.rs`; scaffold is
   one-shot). Facts file scaffolded with `TODO` markers for topic-less fields.
5. **Implement behavior** where the provider genuinely differs; graduation report
   (missing research topics per provider) tracks the rest.

The spec's "steps 1–3 are mechanical" is amended: step 3 above is manual but bounded
(three edits), and the compiler owns the checklist.

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
