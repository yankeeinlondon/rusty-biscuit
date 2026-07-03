# High-Level Approach: Completing the Provider-Metadata Spec

> Conceptual sequencing only — not executable. Each lettered track below gets its own
> executable plan when its turn comes. Authoritative detail: [spec.md](spec.md).

## Where we are (2026-07-02)

The research layer is proven end-to-end: sidecar SimplifiedSchemas, sequence prompts
with lifecycle verification (same-day skip, success gate), fleet runs across the
9-provider roster, and mechanical validation. Four topics are live (logging, models,
permissions, model-config — the last pending its quality fix), the identity grammar is
spike-validated and promoted into `unchained-ai` (`models::identity` +
gen family-fallback), and the drift-detection loop already produces a real work queue
(`requires_claudine_update` flags, Kimi 2.0 protocol break, transcript format changes).

## Track sequencing

```
A. Research contracts ──► B. Catalog & identity ──► C. Claudine generator ──► D. Code migration
        │                                                                          │
        └────► E. Wrapper reliability (early, parallel — research depends on it)   │
        └────► F. Signal catalog (after A's signals topic; feeds/uses E)           │
                                                            G. Rendering ◄────────┘
```

### A — Finish the research-contract layer (in flight)

Immediate: ~~local-runners fleet~~ (done 2026-07-02 — see
[spike-local-runners.md](spike-local-runners.md)), then the model-config fix
([model-config-plan.md](model-config-plan.md)). Then sidecars + verification pattern for
the remaining existing topics (usage, agent-cli, non-interactive-sessions) and the
planned ones (hooks, agent-skills, slash-commands, subagents, agent-permissions v2,
system-prompt, ACP, resume, MCP, signals). Each new
topic follows the now-standard recipe: schema designed backwards from catalog fields →
pilot → evaluate → fleet → evaluate again → targeted fixes. The
`requires_claudine_update` queue is triaged as topics land (Kimi 2.0 protocol fix rides
the non-interactive-sessions/signals topics).

**New topics (decided 2026-07-02).** Four additions, all green-field prompt authoring —
the legacy `docs/research/hooks/` and `docs/research/cross-referencing/` areas have
research outputs but no generating prompt, so they cannot be "migrated". They serve as
**evaluator variance-check inputs** (evaluators diff new claims against them; changes are
either drift worth documenting or errors worth catching) and are `git rm`'d at promotion
so a topic directory never carries two generations of truth (the roster filenames differ
from the legacy ones, so stale siblings would otherwise accumulate — observed with
`claude.md` vs `claude-code.md` in agent-logging).

- `hooks` — per-event records: request/response payload schemas, `capability:
  enum(can_block, can_mutate, observe_only)`, config file + format + section (evidence
  for the spec's `config_format` granularity question), and a `canonical_event` mapping
  onto a **Claudine-owned enum of the 16 lifecycle events** (signal-catalog rule:
  research fills mappings, never invents taxonomy). Feeds `events`/`adapters` and makes
  the unified-hooks support matrix machine-checkable.
- `agent-skills`, `slash-commands`, `subagents` — **three separate topics, run as one
  wave** (separation wins: breadth is where research quality degrades, the evaluate→fix
  loop is per-document, and slash-commands has the greatest variance of the three).
  They share one vocabulary block (`scope` enum with per-OS paths, `format` enum,
  `metadata_keys[]` record shape), duplicated across the three sidecars with a comment
  naming the canonical source until darkmatter supports schema fragments — same pending
  situation as `unit`/`zone`/`confidence` shared with the signal catalog. Cross-kind
  resource-layout facts (`user_dir`/`repo_dir`, discovery order) are identity facts and
  live in `providers.yaml`, not in any of the three. Feeds the `linking` module's
  portability classification. "Shared scripts folders" from the legacy area rides
  inside `agent-skills` if it earns a record; otherwise dropped.
- `system-prompt` — append/replace support, provider-native prompt layers,
  config/memory files, agent/subagent prompt isolation, format recommendations, and
  the best non-mutating delivery strategy for Claudine's `SystemPromptSpec`.
- `ACP` — native-vs-adapter launch mode, protocol version/capabilities, reverse
  request handling, filesystem/terminal delegation, streaming-to-UI patterns, and
  Rust client integration guidance for Claudine's future ACP client/adapter layer.

**Not a new topic:** the agent-permissions bullet list (OS-specific config paths,
formal/informal schema classification, defaults-when-unspecified, YOLO defaults, CLI
switches, env vars) is a **v2 schema for the existing live `agent-permissions` topic**
— reuse the `platforms[]` per-OS pattern from local-runners and the standard
`has_official_schema: enum(formal,informal,none)` (+ `confidence` for observed facts).

**Env-vars ownership rule (decided):** domain topics own their domain's env vars
(model-config, local-runners, and permissions-v2 already carry `env_vars[]` records).
There is no standalone `env-vars` fleet topic in this pass; sanitization allow-lists
and allowed-key inventories should be generated consolidation views over the domain
topics. Never research the same env var in two places.

**Process amendment (from the local-runners cycle):** the recipe is pilot → evaluate →
fleet → **evaluate again** → targeted fixes. Every fresh generation introduced new
errors — including in previously-clean documents and once in a hand-written correction
— so the post-fleet adversarial pass is non-optional, and hand corrections get the same
verification as generated content.

### B — Model ground truth & identity (unchained-ai side)

Fresh catalog snapshot (needs API keys — run `gen` from a normal shell); wire the
identity parser through generation (family/version/variant fields, `latest_in_family`);
emit the JSON catalog artifact Claudine consumes; model-vs-offering identity and the
`catalog_id` / `resolves: pinned|family_latest` mapping fields in the agent-models
schema. ContentPolicy-style freshness stamping applies here (staleness is a correctness
input to "latest").

### C — The Claudine generator (`claudine/gen`)

The committed-codegen pipeline: mapping registry (research frontmatter key → typed
catalog field, with schema↔catalog compatibility checks), overrides layer, diff+confirm
UX, drift test (`generate(inputs) == committed`), first low-risk generated field group
(identity + URLs), then progressive expansion as topics mature. Scaffolding generation
for new-provider onboarding (Pi/Kilo graduation exercises it).

### D — Code migration (shrink hand-written provider variance)

WrapperProfile static-fact overrides → catalog fields (table A in the spec); legacy
`AgentCapabilities` retirement; per-provider module split into generated `data` +
hand-written `behavior`; CLI-crate drift guard to lock it in.

### E — Wrapper reliability (early + parallel; research fleets depend on it)

Model-mismatch guard (observed `llm_call_start` ≠ requested → abort; diagnosed, highest
priority), `grant:` frontmatter implementation, `select.rs` silent frontmatter-model
drop → warn, per-item selector for `claudine sequence`, and the standardized
success-verification pattern rolled into every sequence (done for the four live topics).

### F — Signal catalog (spec Phase 2s)

Normalized taxonomy → detection-record schema (`signals` research topic from track A) →
generate-time compilation into static tables + the generic engine → `claudine signals
check` (harness over the production engine) → fixture corpus (seeded from parser test
data + the live specimens already collected: usage-cap messages, rate-limit resets) →
capture-time-scrubbed harvest mode last. Migrates existing bespoke extractions (Claude
rate-limit, OpenCode 429 classification) onto the engine or the bespoke sink.

### G — Rendering consistency (after the metadata exists)

Render-path inventory → `DisplayPolicy` catalog section → migrate live-sink/wrap output
to shared `TerminalRenderable` components driven by metadata → `BrowserRenderable`
parity inherited for free.

## Skills & knowledge products (continuous)

Each research area that stabilizes gets distilled into an agent skill (`local-runners`
first, then `model-config`), and every such plan also adds a short summary + skill
pointer to the `claudine` skill's research section — the claudine skill carries the map,
topic skills carry the depth, and the research corpus stays the source of truth.

## Standing constraints

- Research model: OpenCode + `kimi-for-coding/k2p7`, fallback `minimax/MiniMax-M3` on
  cap; always verify the observed model matches the requested one until the
  model-mismatch guard exists.
- Human gates: schema/prompt changes checkpoint with Ken before fleet runs; generated
  code and research docs are reviewed via git like any other change.
- Two-plan rhythm: conceptual sequencing lives here; anything executable gets its own
  plan file with orchestration guidance and explicit context for the executing agent.
