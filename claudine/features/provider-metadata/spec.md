# Provider Metadata Automation

> **Status:** DRAFT — brainstorming in progress. Open decisions are marked with `❓ OPEN`.

## Motivation

Claudine normalizes a set of agentic CLIs into one configuration model. The landscape changes
fast: providers ship new flags, output formats, and permission models monthly, and new
providers appear (Pi, Kilo Code are already in `docs/providers.yaml` but have no `Provider`
enum variant). Today, adding or updating a provider means hand-editing 4–6 Rust files
(provider module, wrapper profile, parser factory, adapter, clap variant) plus keeping the
legacy `AgentCapabilities` tree in agreement.

Meanwhile we already have a **research pipeline** that produces current, structured knowledge
per provider — `claudine sequence` over topic documents driven by `docs/providers.yaml`, each
writing prose to the body and **structured data to the frontmatter** of a per-provider
research document. That structured frontmatter is the raw material for automating the
metadata catalog, but today nothing connects it to `ProviderInfo`.

This feature specifies the pipeline that connects them: research → codegen → typed catalog →
rendering, so that adapting to the changing landscape becomes a repeatable, mostly-automated
workflow instead of an archaeology project.

## Goals

1. **Automate provider onboarding and updating** as far as practical. Full automation is not
   required; human review gates are expected. The measure of success: adding a new provider
   or absorbing a provider's breaking change touches *data* first and *code* only where
   genuine new behavior exists.
2. **Substantially expand the static metadata** we capture (see [Metadata Expansion](#metadata-expansion)).
3. **DRY the codebase and make the module structure more cogent** — retire the parallel
   legacy `AgentCapabilities` tree, shrink the decentralized `match Provider` inventory, and
   migrate `WrapperProfile` overrides to catalog-driven defaults where the override is really
   a static fact.
4. **Metadata-driven rendering** — all provider-facing output rendered through
   `TerminalRenderable` (and, in future, `BrowserRenderable`) components, with per-provider
   *variance expressed as catalog metadata* consumed by shared components, not as
   per-provider rendering code.

All four goals are in scope for this spec, phased (see [Phasing](#phasing)).

## Non-Goals

- **Fully unattended updates.** Research documents are LLM-produced; nothing flows into
  compiled code without a human-reviewable diff. Two review gates exist: the research
  document commit and the generated-code commit.
- **`ContentPolicy` / research freshness windows.** Relevant context (research will
  eventually declare how long it stays valid) but specified separately.
- **Runtime-loaded metadata.** `ProviderInfo` stays `&'static` — compile-time data. This
  feature changes how that data is *authored and maintained*, not how it is *served*.

## Current State (summary)

Authoritative detail lives in `docs/topics/provider-metadata.md`. In brief:

- `ProviderInfo` (`lib/src/provider/mod.rs`): 36 fields — identity, 4 behavior trait
  objects, and typed catalog data — one `&'static` constant per provider, served from the
  `registry.rs` single-dispatch-site.
- A **legacy `AgentCapabilities` tree** (~80 fields) is maintained in parallel; tests enforce
  agreement. Duplication is a known burden (topic doc, "Improvement 3").
- **Drift guard**: a source-scan test forbids `match Provider` outside an allow-list in the
  lib crate. The CLI crate has no such guard and carries a long inventory of
  provider-specific dispatch (topic doc, "Decentralized Provider Info").
- **`WrapperProfile`** (CLI crate): defaults derive from the catalog, but ~14 methods are
  still overridden per provider with knowledge that is conceptually static data.
- **Research pipeline**: `claudine sequence <topic>.md` fans out over
  `docs/providers.yaml`, producing one document per provider per topic under
  `docs/research/<topic>/`. Topics today: agent-models, agent-permissions,
  non-interactive-sessions, usage, agent-logging, agent-cli. Planned: env vars, permissions,
  resume, MCP (config/security/events), CLI response streaming.
- Some research topics carry a `target_schema` (SimpleSchema) so their frontmatter is
  machine-validated (`md schema validate`); others (usage, agent-cli,
  non-interactive-sessions) do not yet.

## Architecture

**Decided:** research frontmatter feeds codegen directly. There is no hand-curated
intermediate "manifest" layer — the research documents *are* the data source, and the
committed generated Rust is the reviewable snapshot.

```
┌──────────────────────────────────────────────────────────────────────┐
│ 1. ROSTER + IDENTITY      docs/providers.yaml            (human)     │
│    every provider we WISH to support, plus its identity facts        │
│    (name, slug, binary, aliases, dirs, vendor, URLs, sniff binding)  │
├──────────────────────────────────────────────────────────────────────┤
│ 2. RESEARCH               docs/research/<topic>/<provider>.md  (LLM) │
│    1:1 per provider × topic, produced by `claudine sequence`;        │
│    prose in the body, structured facts in schema-validated           │
│    frontmatter. Committed and reviewed like code.                    │
├──────────────────────────────────────────────────────────────────────┤
│ 3. OVERRIDES              docs/providers/overrides/<slug>.yaml       │
│    small, human-owned corrections that WIN over research frontmatter │
│    during generation; survive research regeneration                  │
├──────────────────────────────────────────────────────────────────────┤
│ 4. CATALOG (generated)    lib/src/provider/<slug>/data.rs            │
│    deterministic codegen joins 1+2+3 → ProviderInfo data constants;  │
│    committed, diff-reviewed, drift-tested                            │
├──────────────────────────────────────────────────────────────────────┤
│ 5. BEHAVIOR (hand-written) lib/src/provider/<slug>/behavior.rs       │
│    behavior traits, stream parsers, adapters, configurators —        │
│    code that genuinely varies between providers                      │
└──────────────────────────────────────────────────────────────────────┘
```

### Why an overrides layer exists

Research documents are regenerated by LLM runs; a hand-edit to research frontmatter is
overwritten by the next `claudine sequence` pass. When research produces a wrong or stale
value, the correction must live somewhere durable. Overrides are expected to be *small and
exceptional* — every override is also a signal the research prompt needs improving, and the
generate report lists active overrides so they get revisited rather than fossilizing.

**Proposed home (pending confirmation):** `docs/providers/overrides/<slug>.yaml`, mirroring
the research frontmatter shape (topic-sectioned keys) so an override is written by copying
the wrong key and correcting its value. Human-owned; sequences never touch it. Alternatives
considered: a `curated:` section inside `providers.yaml` (mixes ownership in the roster
file), protected frontmatter keys in the research docs (relies on LLM compliance rather
than a mechanical guarantee), or no overrides layer at all (a persistent LLM blind spot
becomes an unfixable fact).

### The generate step

A new command — working name `claudine providers generate [<slug>]` — performs the join:

1. Read identity facts from `docs/providers.yaml`.
2. Read the frontmatter of every topic research document for the provider; validate each
   against its topic SimplifiedSchema and normalize values (`DarkmatterSchemas::validate` +
   `normalize_frontmatter` — see "Research topics as typed contracts"); schema violations
   fail loudly with file positions.
3. Apply overrides.
4. Map joined facts onto the typed catalog via a **mapping registry** (Rust, in the
   generator): topic frontmatter key → `ProviderInfo` field, with explicit string→enum
   coercions. An unmappable enum value (research reports a variant Rust doesn't know) is a
   loud generation error — that is exactly the "new variant needed" moment for a human.
5. Emit the *data half* of the per-provider module (`data.rs`) and present a **field-level
   diff with source attribution** (old value vs new value, which research doc supplied it);
   apply on interactive confirm. `--dry-run` reports only; non-TTY defaults to report-only.

Additional generate-report surfaces:

- Research frontmatter with no mapping → reported (extend the catalog or drop the field).
- Catalog fields with no research source and no identity/override source → reported (a
  missing research topic; this list is the backlog of topics to author).
- Any research doc with `requires_claudine_update: true` → surfaced prominently with its
  `reason` (these usually mean the *behavior half* needs human work, which no codegen
  covers).
- Active overrides → listed, with the research value they are suppressing.

### Codegen mechanics (decided: committed codegen)

- The generator is deterministic (no LLM) and emits stable, formatting-independent output so
  it never fights the `main`-is-formatting-authority rule.
- Generated `data.rs` files are committed and reviewed like any other code.
- A **drift test** asserts `generate(inputs) == committed file` — research frontmatter,
  providers.yaml, overrides, and generated code cannot silently diverge (same spirit as the
  existing facade-agreement tests).
- **Generator home (proposed, pending confirmation):** a small dedicated crate
  (`claudine/gen`, in-workspace) that depends on darkmatter (frontmatter parsing,
  SimpleSchema validation) + serde but **not** on the `claudine` lib — avoiding the
  bootstrap tangle of the CLI regenerating the source of the lib it links against (a
  stale/broken catalog could otherwise block building the tool that fixes it). Precedent:
  schematic's `define`/`gen` split. `claudine-cli` shells out to it so the user-facing UX
  stays `claudine providers generate`.

### Research topics as typed contracts (SimplifiedSchema)

Because frontmatter now feeds compiled code, every research topic MUST declare a schema, and
the schema becomes a deliberate design artifact. **No new schema technology is invented**:
the contract layer is Darkmatter's existing `SimplifiedSchema`
(`darkmatter::markdown::schemas::simplified`) — already used by the sequence docs'
`target_schema` and by claudine's compose-time `$schema` validation.

What SimplifiedSchema already gives this feature for free:

- **The type vocabulary** — 14 types (`string`, `date`, `boolean`, `boolish`, `enum`,
  `url`, `file`, inline object literals, …) with constraints (`required`, `default`,
  `enum` members, `pattern`, `min`/`max`, `file(match, eager)`, `url(scheme)`, `unique`).
  Rich enough to type every catalog-feeding frontmatter field identified so far.
- **Validation with positions** — the generator reuses `DarkmatterSchemas::validate` /
  `validate_with_positions` for its "fail loudly on schema violations" gate; the gen crate
  depends on darkmatter anyway, so this is zero new machinery and errors carry file
  positions.
- **Value normalization** — `normalize_frontmatter` coerces `boolish`/`numberlike`/date
  forms before the generator maps values, so LLM output quirks (`"true"` vs `true`) never
  reach the mapping layer.
- **Schema inference** — `DarkmatterSchemas::detect(sources)` infers a SimplifiedSchema
  from existing documents. Phase 0 uses it to *bootstrap* schemas for the topics that lack
  one (usage, agent-cli, non-interactive-sessions) from their already-written research
  docs, which humans then tighten — rather than authoring from scratch.
- **JSON Schema interop** — `to_json_schema` (Draft 2020-12) for any external tooling.

Contract discipline:

- Each topic's schema is designed *backwards from the catalog fields it feeds* (plus
  whatever extra fields serve docs/reporting).
- **Single-source per topic (proven by the logging spike):** the schema lives in a sidecar
  file (`docs/research/<topic>/_schema.yaml`) and target documents reference it with
  `$schema: ./_schema.yaml` (string-scalar file reference); the generator reads the same
  sidecar — so the contract is not trapped inside a prompt document and a schema change is
  one diff. Two grammar rules matter: the sidecar must wrap its properties under a root
  `$schema:` key (darkmatter's resolver otherwise classifies the file as raw JSON Schema,
  which validates vacuously), and nested shapes must be quoted inline-object literals.
  File-typed fields that must exist (e.g. the signal catalog's `evidence`) use
  `file(eager)` — bare `file` is lazy and skips existence checks.
- Schema changes to a topic are catalog-affecting changes and reviewed as such.
- **Schema↔catalog compatibility check:** the generator's mapping registry declares the
  expected SimplifiedSchema type for each catalog field it consumes, and generation
  verifies the topic schema is compatible — e.g. a frontmatter `enum(...)` feeding a Rust
  enum must have members that are a subset of the Rust variants. This catches contract
  drift *before any research runs*, earlier than the value-level check in step 4.
- The mapping registry doubles as documentation: `claudine providers generate --mapping`
  renders the topic→field mapping table (through renderable components, per Goal 4).

### New provider onboarding flow

1. Add entry (with identity facts) to `docs/providers.yaml` — the "wish to support" roster.
2. Run the research sequences — produces the topic docs for the new provider.
3. `claudine providers generate <slug>` — first run scaffolds: the generated `data.rs`,
   plus stubs for the behavior half (trait impl skeletons, parser stub, wrapper profile
   stub) and a TODO list of the structural wiring sites (clap variant, registry slot,
   parser factory arm).
4. Hand-implement behavior where the provider genuinely differs; compile-time
   exhaustiveness on the `Provider` enum walks you through the remaining sites.

Steps 1–3 are mechanical; step 4 is the irreducible engineering. Goal 1's success criterion:
step 4 shrinks release-over-release as more `WrapperProfile` behavior becomes catalog-driven.

Note: a roster entry without research/generation (Pi, Kilo today) is valid — the roster
deliberately runs ahead of code support. The generate report lists roster entries that have
research but no catalog module, as the graduation queue.

## Metadata Expansion

Two sources of new fields:

**A. Already identified in the topic doc** (promote from legacy tree / wrapper overrides):

| Field | Source today | Notes |
|-------|--------------|-------|
| `config_format` | legacy `AgentCapabilities` | per config file, not per provider? (❓ OPEN) |
| `billing_models` | legacy `BillingCapabilities` | |
| `model_cli_flag` | wrapper `apply_model` overrides | |
| `sandbox` (`SandboxSupport`) | wrapper `apply_sandbox` overrides | |
| `stdout/stderr_noise_prefixes` | wrapper overrides | |
| `resume` (`ResumeSpec`) | wrapper `build_resume_args` | pairs with the resume research topic |
| `supports_interactive_inline_closure` | wrapper override | |
| `prompt_delivery` (`PromptDeliverySpec`) | wrapper override | highest effort; may stay behavior |
| `structured_stream_flag` | wrapper `apply_structured_stream` | |
| `non_interactive_conflicting_flags` | wrapper overrides | |
| `allowed_env_keys` | wrapper overrides | pairs with the env-vars research topic |
| `suppress_structured_stderr_on_success` | wrapper override | |
| `model_required_in_non_tty` | hardcoded OpenCode check in `composition/select.rs` | |

**B. New, fed by research topics** (each topic's `target_schema` maps to a catalog section):

| Catalog section | Research topic | Example fields |
|------------------|---------------|----------------|
| `logging` | agent-logging | log directory per OS, format, has-desktop-app, desktop-log parity, schema URL |
| `models` | agent-models | catalog source, config schema, bespoke/local model support |
| `permissions` | agent-permissions | permission CLI params, config file paths (user/repo), agent-scoped permissions, policy-engine fit |
| `non_interactive` | non-interactive-sessions | output formats, schema URL/type, use-case detectability matrix (cap approaching/capped/no-funds/auth/…) |
| `usage` | usage | usage-data acquisition strategy (api/cli/pty-scrape), dashboard URL |
| `cli` | agent-cli | version, homepage/repo/docs URLs, full switch inventory (or a pointer to it) |
| `env_vars` | *(planned)* | recognized env vars, model env chain, sanitization allow-list |
| `resume` | *(planned)* | resume flags, session-ID injection pattern |
| `mcp` | *(planned)* | config location/format, security posture, event visibility |
| `streaming` | *(planned)* | protocol, event coverage, think-delimiter quirks |
| `signals` | *(planned — see [Signal Catalog](#signal-catalog--fine-grained-event-semantics))* | detection records: match/extract paths, units, timezones, version vocabularies, evidence fixtures |

❓ OPEN: how much of B belongs in compiled `ProviderInfo` vs staying research-only (consumed
by docs/reporting tooling but not compiled)? Proposal: research frontmatter is a superset;
the mapping registry declares the compiled subset; unmapped fields remain available to
tooling (drift reports, docs generation, `claudine providers --describe` enrichment).

## Signal Catalog — fine-grained event semantics

The metadata above is mostly **capability facts** (booleans, paths, flags). The
highest-value knowledge Claudine needs is a different class: **detection semantics** for
operationally critical signals — usage cap approaching, usage capped, no funds, invalid API
key, permission denied, model fallback, … For these, "provider X emits rate-limit events"
is nearly useless; what Claudine must know is:

> match `type == "rate_limit_event"`, extract `rate_limit_info.resetsAt`, it is
> **unix-seconds**, it is **UTC**, the status vocabulary is `allowed | allowed_warning |
> rejected` **since ~v2.x** with the older `approaching_limit | limited` vocabulary still
> in the wild, and the same signal appears in the session JSONL log with a different field
> spelling (`reset_at`).

Today this knowledge exists as hand-written Rust (e.g. `stream/protocol/claude.rs` carries
dual `resetsAt`/`reset_at` serde fields; `stream/logs/opencode/errors.rs` distinguishes
three separate 429 conditions) plus prose research. The signal catalog makes it a
first-class, research-fed, mechanically verified data surface.

### Normalized signal taxonomy (Claudine-owned)

Claudine defines the taxonomy — a fixed set of semantic signals with **typed normalized
payloads** — and research fills in per-provider mappings. Research never invents taxonomy.
Initial set (extends the use-case list already in the non-interactive-sessions topic):

`usage_cap_approaching`, `usage_capped`, `no_funds`, `auth_invalid`, `auth_kind_detected`,
`permission_denied_read`, `permission_denied_write`, `tokens_consumed`, `model_resolved`,
`model_fallback`, `human_input_requested`, `session_resumable`.

Each signal declares its normalized payload with **unit- and zone-explicit types**, e.g.:

```
usage_capped:
    window:    enum(five_hour, seven_day, seven_day_opus, monthly, unknown)
    lifts_at:  instant          # normalized to DateTime<Utc> internally
    remaining: quantity         # value + unit(percent | tokens | requests | usd)
```

### Detection records (research-fed, flat, SimplifiedSchema-friendly)

Per provider, per signal, per **source**, research produces flat detection records —
flat rows sidestep SimplifiedSchema's nested-object limits, diff cleanly, and render as
tables:

| Field | Type | Purpose |
|-------|------|---------|
| `signal` | `enum(<taxonomy>)` | which normalized signal |
| `source` | `enum(stream, session_log, app_log, sqlite, hook, stderr)` | where it is observed |
| `locator` | string | source-specific: event `type` path for streams, path template for logs, table/query for sqlite |
| `match` | string | discriminator: JSONPath-style path + expected value(s) |
| `distinguish` | string | how to tell it apart from near-identical events |
| `extract.<payload_field>` | string | extraction path for each normalized payload field |
| `unit` / `zone` | `enum(unix_seconds, unix_millis, iso8601, duration_secs, percent, tokens, usd, …)` / `enum(utc, local, embedded_offset, unspecified)` | **forced answers** to the units/timezone questions |
| `vocabulary` | string[] | observed enum values at this site |
| `since` / `until` | string | provider version range this record applies to (drift = multiple records) |
| `confidence` | `enum(source_code, observed, documented, inferred)` | how the fact was established |
| `evidence` | file | pointer to a captured fixture proving it |
| `detection` | `enum(declarative, bespoke)` | `bespoke` = detection stays hand-written in the behavior half, but semantics are still cataloged |

`zone: unspecified` is *allowed but surfaced* — it flows into `known_gaps` rather than
silently passing, so "we don't know the timezone" is a tracked fact instead of a latent bug.

### Runtime: records drive detection (decided)

Detection records are not documentation-only — they **drive runtime signal detection**:

- **Generate-time compilation.** The generator compiles `declarative` records into static
  detection tables in the generated data half: discriminator and extraction paths are
  parsed into a typed path representation at generate time (a malformed path is a
  generation error, never a runtime surprise), units/zones become enum values, version
  ranges become comparable bounds. Runtime never interprets raw YAML/strings — consistent
  with the `&'static` catalog posture.
- **One generic engine.** A single signal-detection engine in the lib walks incoming JSON
  values (stream lines, session-log records, app-log/sqlite rows) against the provider's
  detection tables and emits **normalized signal events** (the taxonomy's typed payloads,
  timestamps normalized to `DateTime<Utc>`, quantities to value+unit). Per-provider signal
  code shrinks to the `bespoke` escape hatch.
- **Bespoke joins the same sink.** Signals whose detection is too entangled for records
  (e.g. OpenCode's envelope-vs-responseBody layering) stay hand-written in the behavior
  half but emit through the same normalized signal sink, so consumers (lifecycle events,
  reporting, live sink rendering) see one uniform signal stream and never care which path
  produced it.
- **Layering with the typed parsers.** The engine complements — does not replace — the
  typed semantic parsers: parsers own the general event stream; the signal engine owns the
  operational-signal overlay. Where a typed parser already extracts a signal's fields
  (e.g. Claude's `RateLimit` model), the detection table can bind to the typed event
  instead of raw JSON; the records remain the contract either way.

### Evidence corpus and mechanical verification

Fine-grainedness is only trustworthy if it is **checked, not asserted**. Two mechanisms:

1. **Fixture corpus** — captured real payloads per provider/source under
   `docs/research/signals/fixtures/<provider>/`. Committed fixtures are curated and
   scrubbed (session IDs, paths, user content). Detection records cite fixtures as
   `evidence`.
2. **Replay check** — `claudine signals check` replays the corpus through the **production
   signal engine** (see "Runtime: records drive detection"): every record's `match` must
   fire on its evidence fixture and every `extract` path must produce a value of the
   declared unit/type. A wrong JSONPath from LLM research **fails mechanically** instead
   of surviving as plausible prose — and because the checker *is* the runtime engine,
   verified behavior is shipped behavior; there is no parallel interpreter to drift. This
   check is part of the research sequence's exit criteria (alongside `md schema validate`)
   and runs in CI. The same fixtures also serve the hand-written stream/log parsers as
   test vectors — one corpus, two consumers.

**Live harvest (the flywheel):** Claudine is the wrapper — it already sees every stream
event and can see the app logs. An opt-in harvest mode captures *unrecognized or
signal-adjacent* events (rate-limit-shaped, error-shaped, auth-shaped payloads that match
no current detection record) into a local corpus (`~/.claudine/harvest/`) for later
curation into fixtures. Real caps and auth failures happen naturally in daily use; harvest
turns them into evidence instead of losing them.

**Harvest scrubbing (decided):** all detectable confidential information is removed
**before recording** — scrubbing happens at capture time, not at curation time, so raw
sensitive payloads never touch disk. The capture-time scrubber redacts detected secrets
(API-key/token/bearer patterns), webhook URLs (reusing the existing `redact_webhook_urls`
posture), emails, home-directory paths, session identifiers, and free-text user/tool
content beyond what the signal shape needs (a capped event needs the envelope and
rate-limit fields, not the prompt text). Detection is best-effort by nature, so two gates
remain: capture-time scrub (automatic, conservative — when in doubt, drop the field) and
human curation review before any fixture is committed. The scrub rule catalog lives
alongside the protect regex catalog so both defensive surfaces evolve together.

### Research methodology upgrade (sequence prompt changes)

The current sequence files ask good questions but permit docs-only answers. For signal
topics the prompts additionally require:

- **Source-code-first for OSS providers.** Codex, Gemini CLI, Goose, Kimi, OpenCode, and
  Qwen are open source — the exact enum vocabularies, timestamp units, and field names are
  *in the repo*, not in the docs. Prompts direct research to the type/schema definitions in
  source and require file-path citations (permalink + version tag). `confidence:
  source_code` beats `documented` beats `inferred`.
- **SDK type definitions for closed providers.** Claude Code's `SDKMessage` union in
  `@anthropic-ai/claude-agent-sdk` is the de facto schema; the prompt names these known
  authoritative artifacts per provider.
- **Per-signal depth over per-topic breadth.** Signal research runs as its own sequence
  (`signals` topic) with one document per provider whose frontmatter is the detection
  records — not a paragraph inside a broad non-interactive-sessions doc. The broad topics
  keep the prose context; the signals topic owns the machine-readable records.
- **Unanswered ≠ omitted.** If research cannot establish a unit or zone, it must emit
  `unspecified` + `confidence: inferred` so the gap is tracked, never silently dropped.

## Rendering Consistency

Today, per-provider display variance is imperative code inside the live semantic sink and
wrap orchestration (e.g. Codex tool-result summary suppression, OpenCode `step_phase`
suppression — see the decentralized inventory in the topic doc). The direction:

1. **All provider-facing output goes through renderable components** (`Prose`,
   `Table`, `UnorderedList`, `CodeBlock`, … from biscuit-terminal/darkmatter). No bespoke
   `format!`-and-print paths for provider surfaces.
2. **Variance is metadata, not code.** Where providers legitimately differ in what should be
   shown (verbosity of tool results, suppressed event classes, noise prefixes), that policy
   becomes typed catalog data — e.g. a `DisplayPolicy` section: `tool_result_summary:
   Suppress | Show`, `info_event_suppression: &[EventClass]` — consumed by *one* shared
   rendering pipeline.
3. **Dual-target ready.** Components chosen/built must implement `TerminalRenderable` now
   and (where they already do) `BrowserRenderable`, so the browser surface inherits the same
   metadata-driven variance for free.
4. **The generator's own surfaces comply.** Generate reports, mapping tables, and diffs are
   rendered through the same components.

First milestone: an inventory pass over the wrap/live-sink render paths classifying each as
(a) already component-based, (b) mechanical migration, (c) needs a new component, (d) needs
a new `DisplayPolicy` metadata field.

## DRY / Module Cogency Workstreams

Ordered by leverage; several are prerequisites for codegen landing cleanly:

1. **Split per-provider modules into `data` (generated) + `behavior` (hand-written)** halves
   so codegen has a clean landing zone (`lib/src/provider/<slug>/{data.rs, behavior.rs}`).
2. **Retire the legacy `AgentCapabilities` tree** (topic doc improvement 3). Codegen should
   never have to emit the legacy tree, so consumer migration happens before or alongside
   the first generated fields.
3. **Migrate `WrapperProfile` static-fact overrides to catalog fields** (table A above),
   leaving genuine behavior (prompt delivery, wire-RPC) as overrides.
4. **Extend the drift guard to the CLI crate** (topic doc improvement 1) — with a curated
   allow-list; prevents the decentralized inventory from regrowing behind the automation.

## Phasing

- **Phase 0 — contracts.** Every research topic gets a SimplifiedSchema designed backwards
  from the catalog fields it feeds and moved to the proposed `_schema.yaml` sidecar.
  **Status:** agent-logging, agent-models, and agent-permissions all done — sidecars
  validated, sequence files carry the full pattern (sidecar `$schema` + fixed `update:` +
  `initialize` same-day skip + `success` verification stack), and all three fleets ran
  9/9 (27 schema-valid docs as of 2026-07-02; 18 flag `requires_claudine_update` with
  actionable reasons). Usage / agent-cli / non-interactive-sessions still need sidecars
  authored. New wrapper backlog item from the runs: a **model-mismatch guard** (observed
  `llm_call_start` model ≠ requested model should warn/abort — one OpenCode launch
  silently substituted GLM for the requested k2p7);
  identity facts consolidate into `providers.yaml`; render-path inventory (rendering
  workstream milestone 1). **Spiked 2026-07-01 on the logging topic, promoted, and pilot-verified** — see
  [spike-logging/findings.md](spike-logging/findings.md); the validated schema lives at
  `docs/research/agent-logging/_schema.yaml`, `_agent-logging.md` references it and fills
  the new record families, and a Codex-only pilot run (OpenCode + GLM-5.2, `--yolo`)
  produced a schema-valid document with correct per-site time semantics at
  `confidence: observed` and a concrete `requires_claudine_update` flag. Pilot-surfaced
  backlog: implement `grant:` frontmatter (currently a silent no-op; its absence derails
  non-YOLO research runs via `external_directory` auto-rejects), make the
  success-verification lifecycle stack standard in every research sequence (provider exit
  0 is not proof of completion), add `unix_nanos` to the shared `unit` enum, add a
  per-item selector to `claudine sequence`, and check why `env.AGENT`/`env.MODEL` did not
  reach the document. Key spike outcomes: (a) the
  flat-record shape (`surfaces[]`/`time_fields[]`/`record_types[]`) works in today's
  grammar — nested shapes must be quoted inline-object literals, and **the existing
  sequence `target_schema` blocks use the YAML-native nested form the parser rejects**, so
  Phase 0 must normalize them; (b) the realistic schema-authoring recipe is *evidence +
  prose → hand-draft → validate against real host data* (`detect()` bootstrap only helps
  when existing frontmatter is rich); (c) `unit`/`zone`/`confidence` enums are shared
  vocabulary with the signal catalog — define once; (d) real-log inspection belongs in the
  logging research prompt (the sequence already grants `state.user_dir` read), because
  evidence caught local-time filenames, version-suffixed live SQLite (`live_locked` — the
  `repo_home.rs` volatile-state knowledge becomes catalog data), and undocumented record
  types that docs-only research would miss.
- **Phase 1 — generator skeleton.** `claudine/gen` (or chosen home), mapping registry,
  drift test, diff+confirm UX. First generated field group is deliberately low-risk:
  identity + URLs + existing simple scalars.
- **Phase 2 — expansion + wrapper migration.** Add table-A fields to `ProviderInfo`,
  migrate the corresponding `WrapperProfile` overrides to catalog-driven defaults, extend
  generation to table-B sections as their research topics mature.
- **Phase 2s — signal catalog** (parallel to Phase 2). Define the normalized signal
  taxonomy + detection-record schema; author the `signals` research sequence with the
  source-code-first methodology; seed the fixture corpus from existing parser test data;
  build the generic signal engine + generate-time record compilation; land
  `claudine signals check` (a harness over the engine) and wire it into research exit
  criteria and CI; migrate existing hand-written signal extractions (Claude rate-limit,
  OpenCode 429 classification) onto the engine or the `bespoke` sink. Live harvest mode
  (with its capture-time scrubber) ships last — it needs the detection records to define
  "unrecognized".
- **Phase 3 — legacy retirement.** Migrate remaining `AgentCapabilities` consumers to the
  typed catalog; delete the legacy tree and its facade machinery.
- **Phase 4 — rendering.** `DisplayPolicy` catalog section; migrate live-sink/wrap render
  paths to shared metadata-driven components; CLI-crate drift guard lands here to lock in
  the wins.

Phases 2–4 can interleave; the ordering above is dependency-driven, not strictly serial.

## Open Questions

1. ~~**Corrections/overrides home**~~ — *provisionally decided:*
   `docs/providers/overrides/<slug>.yaml` (see "Why an overrides layer exists"). Ken to
   confirm.
2. ~~**Generator home**~~ — *provisionally decided:* dedicated `claudine/gen` crate (see
   Codegen mechanics). Ken to confirm.
3. **`config_format` granularity** — per provider or per config-file entry
   (`PathTemplate` + format pairs)?
4. **Compiled subset boundary** — confirm the "research is a superset; mapping registry
   declares the compiled subset" proposal.
5. **Prompt delivery** — does `PromptDeliverySpec` ever become data, or is it accepted as
   permanent behavior-half code? (Topic doc rates it highest-effort/highest-value.)
6. ~~**Declarative detection at runtime?**~~ — *decided:* records both document AND drive
   runtime detection, via generate-time compilation into static detection tables walked by
   one generic signal engine, with the `bespoke` escape hatch emitting into the same
   normalized signal sink (see "Runtime: records drive detection").
7. ~~**Harvest privacy posture**~~ — *decided:* capture-time scrubbing — all detectable
   confidential information is removed before recording, with human curation as the second
   gate (see "Harvest scrubbing").
