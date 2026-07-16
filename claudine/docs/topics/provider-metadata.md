---
categories: 
    - technical
    - type-safety
    - goal-alignment
---
# Provider Metadata

## Why Metadata Matters

Claudine normalizes many agentic CLIs into a single configuration model. Each
provider differs in dozens of material ways: binary name, config file paths,
stream protocol, hook event names, YOLO flag, reasoning controls, system-prompt
delivery, model-catalog source, and more. Without a type-strong, centralized
catalog these differences leak into scattered `match Provider { … }` blocks,
a maintenance burden that grows with every provider.

`ProviderInfo` solves this by being the **single authoritative record** for
every static fact about a provider. Each compiled `Provider` variant maps to
exactly one `&'static ProviderInfo` served from the central registry. Identity
fields are non-optional so a compile-time gap is impossible; genuinely dynamic
behavior (payload detection, MCP import/export, stream-parser construction, hook
registration) lives behind four trait objects on the same struct, so one
registry lookup returns both data and behavior.

The design goal: **all provider variation is driven by `ProviderInfo` metadata**.
When a feature must branch on provider identity it reads
`provider_info(provider).<field>` or calls a behavior trait — never a bare
`match Provider` outside the registry.

### Providers: the compiled set

The **compiled** `Provider` enum has **ten** variants (`PROVIDER_COUNT = 10`,
`PROVIDERS_DISPLAY_ORDER` in `lib/src/provider_id.rs`): Claude Code, Codex CLI,
Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and — added via the Phase H
provider ladder (2026-07-08) — Kilo, Pi, and Antigravity. The **research roster**
(`docs/providers.yaml`) is currently in sync with the enum (all ten wired); it is
designed to run *ahead* of the enum during onboarding (a new roster entry is
active-but-unwired until its variant lands) and carries a `skip_research: true`
flag for keep-identity-but-pause deprecations. Roo Code was removed in 2026-07.
Major provider version changes enter the roster as new entries rather than
mutating existing ones ("Major-version changes are new providers" — see
`features/2026-07-02-provider-metadata/spec.md`).

## Generated, Not Hand-Written

As of generator v1, each `lib/src/provider/<slug>/data.rs` is **generated** by
`claudine-gen` from four sources — the roster (`docs/providers.yaml`), per-provider
facts (`docs/providers/facts/<slug>.yaml`), the schema-enforced research fleets
(`docs/research/<topic>/`), and field-keyed overrides
(`docs/providers/overrides/<slug>.yaml`). Only `behavior.rs` (and the
parser/adapter/configurator/wrapper code) is hand-written. `data.rs` is **never**
hand-edited.

- Regenerate all providers + `catalog.json` + the field-list guards:
  `claudine providers generate --yes`.
- CI/drift path (byte-equality): `claudine providers generate --check`.
- One change = registry + `emit.rs` + regen-all + `catalog.json` + both
  field-list guards, together (the "one-change discipline").

The mapping registry (`gen`) declares the **compiled subset** of `ProviderInfo`;
`catalog.json` emits the full **superset** so docs/reporting tooling can consume
fields that are not compiled. This is the ratified answer to the spec's compiled-
subset boundary (Open Question 4).

### Per-provider module shape (`module-split.md`)

Each provider is a directory `lib/src/provider/<slug>/`:

- `data.rs` — the generated `&'static ProviderInfo` constant and its typed
  catalog constants.
- `behavior.rs` — the hand-written zero-sized struct implementing the four
  behavior traits.
- `mod.rs` — wiring.

The legacy `AgentCapabilities` tree and every transitional `legacy.rs` were
**retired in Phase C**; a guard (`provider_legacy_files_only_shrink`) keeps them
from returning.

## What We Capture

`ProviderInfo` carries **42 serialized fields** plus four non-serialized behavior
trait objects. The serialized set is the authoritative live surface — inspect it
with `claudine providers --describe --format json`, or read the committed
`docs/providers/catalog.json`. It is pinned on both sides by
`serialized_field_list_matches_catalog` (lib) and `registry_coverage` (gen), so
this prose intentionally does **not** re-list every field (that would drift).

The fields group into three families:

- **Identity** — `provider`, `display_name`, `slug`, `short_name`, `binary`,
  `agent_offset`, `cli_aliases`, `docs_url`, `usage_dashboard_url`,
  `sniff_binding`, `supports_skills`, `platform_kind`.
- **Behavior traits** (dynamic dispatch, `lib/src/provider/behavior.rs`) —
  `behavior` (`ProviderBehavior`: payload detection + parser construction),
  `mcp` (`McpBehavior`), `adapter` (`AdapterBehavior`), `configurator`
  (`ConfiguratorBehavior`). Each has default "not supported" impls so providers
  override only what they need.
- **Typed catalog data** — strongly-typed static facts, each backed by a module
  under `lib/src/provider/` with its own enum/struct: `stream_protocol`,
  `event_mapping`, `session_log_paths`, `config_paths`, `memory_files`,
  `output_formats`, `entrypoints`, `system_prompt`, `yolo`, `reasoning`,
  `known_gaps`, `acp`, `prompt_arg_conventions`, `unmapped_native_events`,
  `cli_sensitive_axes`, `repo_home_root_files`, `model_env_vars`,
  `model_catalog_source`, `expected_offerings`, `offering_sources`, `resume`,
  `model_cli_flag`, `non_interactive_conflicting_flags`, `billing_models`,
  `allowed_env_keys`, `display_policy`, `suppress_structured_stderr_on_success`,
  `supports_interactive_inline_closure`, `model_required_in_non_tty`.

Several fields that were sketched as "future work" in earlier drafts have since
landed (Phases D/F/G): `billing_models`, `model_cli_flag`, `resume`
(`ResumeSupport`), `supports_interactive_inline_closure`,
`model_required_in_non_tty`, `platform_kind`, the model-catalog fields
(`expected_offerings`/`offering_sources`/`model_catalog_source`), and — as a
**generated `DisplayPolicy` sub-record** — the former stdout/stderr noise prefixes
plus tool-result and event-suppression policy (single owner; zero `provider ==`
in render code).

### Related generated artifacts

- **`catalog.json`** (`docs/providers/`) — the serialized superset, `CATALOG_SCHEMA_VERSION`-stamped.
- **Signal detection tables** (`lib/src/signals/generated.rs`) — compiled from
  the `signals` research fleet; walked by one generic engine (see
  `design/signal-detection.md`).
- **Stream error vocabulary** (`lib/src/stream/providers/vocabulary.rs`) —
  compiled from the `agent-errors` research fleet. Provenance objects remain in
  research; generation projects only the ordered semantic kinds, needles, and
  numeric codes needed at runtime. Retired `error_vocabulary` facts keys are a
  hard source collision rather than a fallback.
- **Model catalog** (`unchained-ai/artifacts/models-catalog.json` + the vendored
  `families_generated.rs` slice) — model identity ground truth joined into
  `expected_offerings` (see `design/model-catalog-boundary.md`).
- **Dispatch inventory** (`docs/providers/dispatch-inventory.json`) — the
  mechanical census that seeds the drift guard (below).

## Ensuring a Single Source of Truth

### Generated-data drift tests

`claudine providers generate --check` regenerates every `data.rs` +
`catalog.json` in memory and byte-compares against the committed files, so any
hand edit or stale generator fails CI. The registry-covers-all-fields guard
(`gen/tests/registry_coverage.rs` and its lib twin
`serialized_field_list_matches_catalog`) pins the field list on both sides.

### Exhaustive invariant tests (`lib/src/provider/tests.rs`)

Registry completeness (every variant resolves to a `ProviderInfo` whose
`provider` matches the key; the array has `PROVIDER_COUNT` slots); non-empty
mandatory identity fields; sniff-binding round-trip; behavior trait objects
non-null; plus structural invariants (every provider has a config path; supported
events have native names; hook events imply `configurator.hooks_supported()`;
stream providers expose an event; ACP events imply ACP support).

### The unified `Provider`-dispatch drift guard (Phase I)

Decentralized `match Provider` / `matches!` / `== ` / `!=` dispatch is prevented
from regrowing by a single inventory-based, site-level guard in
**`claudine-cli/tests/dispatch_inventory.rs`**, covering **both** `lib/src` and
`cli/src`. (Phase I retired the lib crate's earlier regex-based
`no_unauthorized_match_provider_in_lib` guard and folded both crates into this one
mechanism.)

- **Mechanical inventory** — the scanner classifies every `Provider::<Variant>`
  occurrence into a pattern form (`match-provider`, `matches-macro`,
  `eq-comparison`, `ne-comparison`, `tuple-array`, `let-pattern`, `provider-arm`,
  `direct-ref`) and a `dispatch_class` (`conditional` = behavior varies by
  provider; `reference` = merely names one). `#[cfg(test)]` module bodies are
  blanked before scanning, and blanket-exempt files are tagged (`exempt_candidate`):
  the authoritative lib registry/identity/methods, the stream-parser factory, the
  per-provider `permissions/providers/*.rs` and CLI `wrap/profile/*.rs` impl files,
  the clap mapping in `main.rs`, and test paths. The census is committed at
  `docs/providers/dispatch-inventory.json` and byte-compared on every run
  (regenerate with `CLAUDINE_UPDATE_INVENTORY=1 …`).
- **The guard** (`cli_dispatch_guard_holds_the_line`) — every *conditional,
  non-exempt* site must be grandfathered in `GUARD_ALLOWLIST` with a workstream
  tag and a `reason`, matched line-independently by `(path, form, providers)`. A
  new decentralized dispatch site fails until migrated to a catalog field /
  behavior trait or consciously listed; a stale entry (matching no live site,
  e.g. after a provider removal) also fails. A burn-down summary by tag prints on
  every run.
- **End state (2026-07-08).** 18 governed sites, **all `keep`, zero pending
  migration** — every ws0-prep / ws3-profile / render migration completed in
  Phases C/D/G. The remainder are genuinely behavioral: Codex/OpenCode wire and
  stderr-bridge quirks, shadow-HOME mechanics, and Claude's canonical role as the
  native home for linked skills/commands/agents.

### WrapperProfile as a behavior shim

`WrapperProfile` (CLI layer) provides defaults derived from the central catalog
(`binary`, `agent_env`, `apply_yolo`, `apply_entrypoint`, `apply_output_format`,
`prompt_arg_conventions`, `apply_model`, `supports_resume`, …). After Phase D,
**static-fact overrides reached zero**: the remaining overrides are genuinely
behavioral (prompt-delivery mechanics, wire-RPC quirks). Adding a provider is
now catalog data + behavior-trait impls, not a spray of profile overrides.

## Remaining Gaps

Most gaps sketched in earlier drafts are closed. The tracked residue:

1. **Prompt-delivery *selection* enum** — the delivery *mechanics* stay
   behavior-half (the required `WrapperProfile::prompt_delivery`), but the
   enumerable mechanism vocabulary graduating to a `PromptDeliverySpec`
   *selection* field is a future item (Open Question 5 ruling: selection is data,
   mechanics stay code).
2. **Sandbox/container descriptor** — `apply_sandbox` is still a per-provider
   override; deferred until the permissions six-axis work provides a consumer
   (Checkpoint D ruling).
3. **Unmapped-research graduation** — the follow-up spec at
   `features/2026-07-06-more-struture/spec.md` covers research fields that are
   captured but not yet compiled.
