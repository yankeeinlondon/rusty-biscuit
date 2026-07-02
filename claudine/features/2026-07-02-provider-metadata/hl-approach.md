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

Immediate: local-runners fleet ([local-runner-plan.md](local-runner-plan.md)), then the
model-config fix ([model-config-plan.md](model-config-plan.md)). Then sidecars +
verification pattern for the remaining existing topics (usage, agent-cli,
non-interactive-sessions) and the planned ones (env-vars, resume, MCP, streaming,
signals). Each new topic follows the now-standard recipe: schema designed backwards from
catalog fields → pilot → evaluate → fleet. The `requires_claudine_update` queue is
triaged as topics land (Kimi 2.0 protocol fix rides the streaming/signals topics).

> TODO: we should also add topics for:
>
> - `hooks` - what event hooks does the agent support? what are the request and response hook schemas for each event? which events can stop execution of the running job versus those just used to extract information from a running process. How do the events an agent supports map to Claudine's canonical event model? What files are used to configure? What is the schema for these configuration files (or at least the "section" used for hook config)?
> - `agent-skills` - all agents support "agent skills" but there is some variance around config files, config file formats (json, yaml, markdown frontmatter, toml, etc.). Each Agent should also support the idea of "user scope" versus "repo scope" separation which allows skills to be defined across users or for a particular repo but there are often differences in terms of what metadata key/value pairs are recognized and or required. 
> - `slash-commands` - all agents support "slash commands" even if they call it something else (e.g., "prompts", etc.). Like "agent skills" there is important variance and in fact slash commands tends to have far greater variance than agent skills do. 
> - `agent-permissions` 
>     - what configuration files are used (user versus repo scoped); this should be OS specific
>     - what is the schema for permissions? is it a formal, informal, or observed schema?
>     - what are the "default permissions" for the given agent if nothing is specified?
>     - what are the default permissions for YOLO mode (if supported)?
>     - what CLI switches does the Agent provide to modify the permissions?
>     - what ENV variables -- if any -- would effect permissions?

QUESTION: what is the best set of seams to use to achieve full research coverage? Do any of the new topics above overlap in scope to existing research areas? Let's finalize this before we do any more research work.

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
