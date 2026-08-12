# Supplemental Design: Functional Render Components & the Event Renderer

> **Status:** draft for Ken's review. Supersedes the spec's "Rendering Consistency"
> sketch with the functional-component direction ratified in brainstorm (2026-07-02):
> components organized by FUNCTION, not by provider; provider variance enters only as
> data. Elevated from trailing Phase 4 to an enabler that starts alongside Phase 1–2.

## The architecture in one paragraph

Provider variance enters the pipeline at exactly two points: the **parser** (input —
provider-specific, normalizes to `stream::protocol` events; exists today in
`stream/providers/`) and the **catalog** (render policy data — `DisplayPolicy`,
generated per provider). Between them everything is functional: a library of
renderable components in the claudine lib, each implementing `TerminalRenderable`
(and `BrowserRenderable` where mandated), each consuming normalized events plus policy
data — never matching on `Provider`. A Kimi thinking token and an OpenCode thinking
token render identically *by construction*.

```rust
// pseudo-code
let events = Parser::new(Provider::Codex).read_stream(&stream); // normalized
EventRenderer::new(provider_info).consume(&events);             // dispatch to components
```

## Ruling 1 — components consume policy, never Provider

`ToolUse::new(provider)` is acceptable constructor sugar, but the internal contract is
`ToolUse::with_policy(&DisplayPolicy)`: components read typed policy fields (glyphs,
truncation, verbosity tiers, noise prefixes) and contain **zero** `match provider`.
Rationale: otherwise decentralized dispatch regrows inside the render layer, outside
drift-guard reach. A new provider gets correct rendering by filling catalog data.

- `DisplayPolicy` struct + `EventClass` enum live in `claudine/catalog-types` (F1);
  the populated values are a generated catalog section (per-provider, overridable like
  any field — design/catalog-generation.md).
- **Noise-prefix double-ownership resolved:** `stdout/stderr_noise_prefixes` is
  DisplayPolicy data (single owner); spec table A's copy is re-pointed there.

## Ruling 2 — parser emits; EventRenderer dispatches

Parse and render stay separated because the normalized stream has three consumers:
the **EventRenderer** (this doc), the **signal engine** (design/signal-detection.md),
and **reporting** (JSONL). Baking render into the parser would force the other two to
re-parse or side-channel. The one-liner facade (`Parser::new(p).read_stream(...)`
wiring the default renderer) is preserved for ergonomics.

**The dispatch table is the load-bearing artifact:** an exhaustive mapping from every
`stream::protocol` event variant to a component or an explicit `Silent` entry, keyed
secondarily by `EventClass` policy. A completeness test fails when a new protocol
variant lacks a mapping — the render-path equivalent of enum exhaustiveness.

## Component inventory (initial)

Ken's functional set, plus candidates from the protocol's remaining vocabulary and
existing proto-components:

| Component | Input | Notes |
| --- | --- | --- |
| `AgentPrompt` / `SystemPrompt` | prompt reporting types | `prompt_reporting` module is the proto-component; first migration |
| `ToolUse` | ToolCall + ToolResult pair | spans two events — needs the span contract below |
| `ThinkingToken` | Reasoning spans | the hard streaming case |
| `FinalMessage` | terminal assistant output | kills the ×3 Codex duplication (design/pipeline-dry.md ws0) |
| `MetricsReport` | logs/report data | strongest early BrowserRenderable consumer |
| `McpCall` | MCP runtime events | |
| `HookEvent` | lifecycle events | |
| `StepProgress` | sequence step headers/transitions | |
| `FileChange`, `PlanUpdate`, `SubagentActivity` | protocol events | |
| `ErrorBlock` | typed errors + frontmatter excerpts | `report_block_error` is the proto-component |

## Ruling 3 — streaming contract

- **Stream-class events** render per-event: event → small component → render → emit.
  The IR tree's document shape is reserved for **report-class** output (MetricsReport,
  ErrorBlock appendices, describe/matrix tables).
- Components whose content arrives incrementally (`ThinkingToken`, `ToolUse`) get a
  three-phase span contract: `open() → append(chunk) → close()`, where `open`/`close`
  render frames and `append` renders deltas. This is an additive claudine-side trait
  (`StreamRenderable`) layered over `TerminalRenderable` — no change to the shared
  renderable crate in v1; promote upstream only if darkmatter/biscuit-terminal grow
  the same need.

## Ruling 4 — homes and the dual-target rule

- Components + `EventRenderer` live in **`lib/src/render/`**; the CLI keeps only sink
  wiring (which writer, TTY detection, color depth).
- `TerminalRenderable` is mandatory for every component. `BrowserRenderable` is
  **mandatory for report-class** components (MetricsReport, ErrorBlock, the
  providers/mapping tables) and opportunistic for stream-class ones.
  `MetricsReport`-in-browser is the proof case.

## Migration order

1. `FinalMessage` (retires the Codex triplication — lands with pipeline-dry ws0).
2. `AgentPrompt`/`SystemPrompt` (absorb `prompt_reporting`).
3. Live sink → `EventRenderer` + dispatch table (the big one; scattered `format!`
   branches retire incrementally per event class).
4. `MetricsReport` (+ browser target) and the providers/describe output
   (with AgentCapabilities retirement — design/module-split.md).

The CLI-crate drift guard lands after (design/pipeline-dry.md), locking in the wins.
