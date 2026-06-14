---
phase: 1
plan: "features/2026-06-05-pid/plan.md"
spec: "features/2026-06-05-pid/spec.md"
created: 2026-06-05
---

# Phase 1 — Current-State Mapping and PID Model

Validation checkpoint for the PID-capture plan. Inventories every spawn
path and every Claudine-controlled context surface that will be touched
by Phases 2–5, then records the chosen in-memory representation and
propagation boundary so the later phases have a single authoritative
reference.

## Spawn-Path Inventory

Rust's `std::process::Command::new(<provider binary>)` is invoked in
exactly four locations inside `claudine/cli`. All other Claudine code
that "runs a provider" delegates to one of these.

| # | Function | File | Wire mode | Used by |
|---|----------|------|-----------|---------|
| 1 | `run_child` | `claudine/cli/src/commands/wrap/exec/spawn.rs:67` | inherited stdio (interactive TUI) + filtered stdio | direct wrapper in `claudine/cli/src/commands/wrap/mod.rs:1329`, legacy composition branch in `claudine/cli/src/commands/wrap/composition/legacy_goose.rs:55/122` |
| 2 | `run_child_capture` | `claudine/cli/src/commands/wrap/exec/spawn.rs:373` | piped stdio captured into strings | legacy composition branch in `composition/legacy_goose.rs:85`, harness-orchestration no-stream fallback in `claudine/cli/src/commands/wrap/harness_orch.rs:624` |
| 3 | `run_child_stream_semantic` | `claudine/cli/src/commands/wrap/exec/spawn.rs:550` | piped + semantic parser + stderr bridge | direct wrapper in `wrap/mod.rs:1257`, harness-orchestration structured path in `harness_orch.rs:538`, composition structured path in `composition/structured.rs:95` |
| 4 | `run_kimi_wire_session` | `claudine/cli/src/commands/wrap/exec/wiring.rs:575` | Kimi JSON-RPC wire (stdin/stdout/stderr piped) | direct wrapper in `wrap/mod.rs:1226`, harness-orchestration in `harness_orch.rs:516`, composition structured path in `composition/structured.rs:72` |

### Owner pattern

All four entry points take an `&mut bool child_spawned` flag from
their caller. Today the flag is flipped to `true` immediately after a
successful `command.spawn()?`, and the existing tracing span records
`child.id()` via `Span::current().record("child_pid", …)` for
diagnostic purposes only.

### Call-site tree (phase-2 surface)

```
claudine cli::commands::wrap::run_provider_wrapper_inner (wrap/mod.rs)
├── wire_io::run_kimi_wire_session        (Kimi wire mode)
├── exec::run_child_stream_semantic       (structured streaming)
└── exec::run_child                       (legacy / fallback)

claudine cli::commands::wrap::harness_orch::run_harness_orchestration
├── wire_io::run_kimi_wire_session        (Kimi wire inside harness)
├── exec::run_child_stream_semantic       (structured harness attempt)
└── exec::run_child_capture               (capture fallback)

claudine cli::commands::wrap::composition::execute_without_harness
└── composition::legacy_goose::run_legacy_branch
    ├── exec::run_child                   (interactive inline)
    └── exec::run_child_capture           (non-interactive inline)

claudine cli::commands::wrap::composition::structured::run_structured_branch
├── wire_io::run_kimi_wire_session        (Kimi wire composition)
└── exec::run_child_stream_semantic       (structured composition)
```

### Phase 2 implication

- `CLAUDINE_PID` injection belongs in `env::build_child_env_with_launch`
  (`claudine/cli/src/commands/wrap/env.rs:72`), before the
  provider-specific overrides encoded by each `WrapperProfile`. That
  single insertion covers all four spawn sites because every spawn
  path receives its `env: &HashMap<OsString, OsString>` from one of
  the `EnvPlan` returned by `build_child_env` /
  `build_child_env_with_launch`.
- `AGENT_PID` capture belongs inside each of the four spawn functions
  immediately after `command.spawn()?`. The existing `child_spawned`
  flag and the existing `child.id()` tracing record show the exact
  line to extend.
- Harness retry and structured/legacy/wire selection paths already
  pass `child_spawned: &mut bool` per attempt, so per-attempt PID
  capture is the natural shape; nothing should carry a stale PID
  across attempts.

## Context-Surface Inventory

Every Claudine-controlled surface that serializes `EventMeta` or
wrapper-supplied context extras. Provider-stream rows are emitted by
parser adapters and are explicitly out of scope for PID enrichment
unless they pass through one of these surfaces.

### Surfaces that emit Claudine-controlled `EventMeta`

| Surface | File | Role |
|---------|------|------|
| Dispatch JSONL log (canonical event) | `claudine/lib/src/dispatch/mod.rs:431` (`log_dispatch_event`) and `:408` (`write_dispatch_event_to`) | Adapter-driven events from provider hooks |
| Dispatch `prepare_meta_for_dispatch` | `claudine/lib/src/dispatch/mod.rs:445` | Stamps `EnvironmentContext`, `CLAUDINE_SESSION_ID`, `interactive`, `yolo` into every `EventMeta` |
| Wire-mode Kimi hook dispatcher | `claudine/cli/src/commands/wrap/exec/wiring.rs:394` (`dispatch_hook_request`) | Builds an `EventMeta` for each Kimi `HookRequest`, merges `request.context` into `extra` |
| Stream summary → `EventMeta` | `claudine/lib/src/stream/reporting.rs:25` (`summary_to_event_meta_with_context`) | Wrapper session lifecycle (`SessionEnd` synthetic) |
| Semantic-event → `EventMeta` | `claudine/lib/src/stream/reporting.rs:164` (`semantic_event_to_event_meta`) | Per-event semantic rows that Claudine controls (not raw provider lines) |
| Composition context extras (`composition_*`) | merged via `context_extra: Option<&HashMap<String, Value>>` in the two reporters above | `composition_file_ref`, `composition_mode`, `composition_source_path`, etc. |

### Reporting ingest / query surfaces

| Surface | File | Role |
|---------|------|------|
| Reporting schema v4 | `claudine/lib/src/reporting/schema.rs:5` (`SCHEMA_VERSION = "4"`) | `events` and `sessions` tables — no PID columns today |
| `PreparedEvent` row | `claudine/lib/src/reporting/ingest.rs:42` | Flattens `EventMeta` into SQL columns; `meta.extra` → `extra_json`, `meta.env` → `env_json` |
| `insert_event` / `upsert_session` | `claudine/lib/src/reporting/ingest.rs:488` (`insert_event`), session upsert below | Single ingest chokepoint for every JSONL line |
| Reporting DTOs | `claudine/lib/src/reporting/types.rs` | Public query types rendered by `claudine/logs/*` CLI |
| Query mappers | `claudine/lib/src/reporting/queries/{today,week,month,sessions,errors,tools,repos,trends,sync}.rs` | Project `events`/`sessions` into DTOs |
| CLI renderers | `claudine/cli/src/commands/logs/{today,week,month,sessions,errors,tools,repos,trends,sync}.rs` | Text + JSON output of those DTOs |

### Template / expression context

`DispatchRuntimeContext` (`claudine/lib/src/dispatch/mod.rs:29`) is
the wrapper-session-scoped cache for compiled Claudine config. It does
not itself carry per-event context; template and expression resolution
happens against `EventMeta` and the in-process environment inside
`prepare_meta_for_dispatch`. PIDs will reach templates and
expressions through `EventMeta` (and the `extra` map for the
wrapper-only `agent_pid` value), not through `DispatchRuntimeContext`.

### Phase 3/4 implication

- `claudine_pid` is a stable per-session value and belongs on
  `EnvironmentContext` so it is carried into every `EventMeta` via the
  existing `meta.env` projection, mirroring how `package_area` and
  `package` already flow from wrapper env to records.
- `agent_pid` is per-attempt and only known after a successful spawn.
  It cannot live on `EnvironmentContext` (which is captured at session
  start). It belongs on `EventMeta` as a typed optional field, with
  the wrapper setting it after spawn and the reporters projecting it
  into `extra` for downstream JSONL/SQLite ingest.
- Phase 4 will add `claudine_pid` and nullable `agent_pid` columns to
  `events` and `sessions`, bumping `SCHEMA_VERSION` from `"4"` to
  `"5"`. Ingest will read the typed `EventMeta.agent_pid` and the
  `EventMeta.env.claudine_pid` fields rather than introducing a new
  fallback location.

## Canonical In-Memory Representation

Chosen shapes (to be implemented in Phase 2/3).

### `EnvironmentContext` (claudine/lib/src/events/environment.rs)

```rust
pub struct EnvironmentContext {
    // ...existing fields...
    /// Claudine's own process ID, captured once at wrapper startup.
    /// Always populated for wrapper-emitted records.
    #[serde(default)]
    pub claudine_pid: Option<u32>,
}
```

`u32` matches `std::process::Child::id()`'s return type on Unix and
Windows. `Option<_>` keeps deserialization of older JSONL lines
backwards compatible. Populated once at wrapper startup (Phase 2),
attached to every `EventMeta` via `meta.env`.

### `EventMeta` (claudine/lib/src/events/event_meta.rs)

```rust
pub struct EventMeta {
    // ...existing fields...
    /// Immediate child PID returned by the wrapper spawn operation.
    /// `None` until a successful provider spawn; absent from raw
    /// JSONL when unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_pid: Option<u32>,
}
```

`skip_serializing_if = "Option::is_none"` enforces the spec's "raw
structured logs omit `agent_pid` when unavailable" rule at the
serializer boundary rather than relying on every emitter.

### Wrapper-only context extras (no type changes)

The wrapper will set `meta.extra` keys `claudine_pid` and `agent_pid`
to mirror the typed fields for template/expression lookups, matching
the existing pattern used for `interactive`, `yolo`,
`composition_file_ref`, etc. The typed fields remain the source of
truth for JSONL and SQL ingest; the `extra` mirror exists only for
template expression bridging.

## Propagation Boundary

```
                       EnvironmentContext
                       ┌──────────────────────────────┐
                       │ claudine_pid: Option<u32>    │  set once at
                       │ (existing host/repo fields)  │  wrapper startup
                       └──────────────┬───────────────┘
                                      │
                                      ▼
   ┌───────────────── EnvironmentContext embedded in every EventMeta ─────────────────┐
   │                                                                                  │
   │  EventMeta                                                                       │
   │  ┌────────────────────────────────────────────────────────────┐                  │
   │  │ provider, event, timestamp, session_id, ...                │                  │
   │  │ env: EnvironmentContext  ◄── claudine_pid lives here       │                  │
   │  │ extra: HashMap<String, Value>                              │                  │
   │  │   ├── "interactive": "true" (existing)                     │                  │
   │  │   ├── "yolo": "false"       (existing)                     │                  │
   │  │   ├── "claudine_pid": <num> (mirror for templates)         │                  │
   │  │   └── "agent_pid": <num>    (mirror for templates)         │                  │
   │  │ agent_pid: Option<u32>      ◄── typed field, post-spawn    │                  │
   │  └────────────────────────────────────────────────────────────┘                  │
   │                                                                                  │
   └──────────────────────────────────────────────────────────────────────────────────┘

   DispatchRuntimeContext — NO PID STATE
   (per-session compiled config cache only; PIDs flow through EventMeta)
```

Boundary rules:

1. `claudine_pid` lives on `EnvironmentContext`. It is read-only after
   wrapper startup. It reaches `EventMeta` via the existing `meta.env`
   assignment in `prepare_meta_for_dispatch`.
2. `agent_pid` lives on `EventMeta`. It is `None` until a successful
   spawn, set by the wrapper immediately after `command.spawn()?`, and
   reset between harness attempts so a stale PID never leaks across
   retries.
3. The `extra` map carries stringly-typed mirrors (`"claudine_pid"`,
   `"agent_pid"`) so template and expression contexts see the values
   alongside the existing `"interactive"` / `"yolo"` / composition
   keys. Typed fields remain authoritative for JSONL and SQLite.
4. `DispatchRuntimeContext` is unchanged. PIDs are not configuration.
5. Provider-stream rows (raw `SemanticEvent`s emitted by provider
   adapters) are not enriched with PIDs solely because they occur
   during a wrapped session. Claudine-controlled reporters always
   stamp PIDs; provider-adapter rows do not.

## Unavailable-State Behavior

| Field | Before spawn | After failed spawn | After successful spawn | Across harness retries |
|-------|--------------|--------------------|------------------------|------------------------|
| `claudine_pid` (env) | populated | populated | populated | populated |
| `agent_pid` (typed)  | `None`     | `None`             | `Some(child_pid)`       | reset to `None`, then re-set on next successful spawn |
| `agent_pid` (extra)  | absent     | absent             | present (mirror)        | absent until next successful spawn, then present |

Serializer rules (enforced by `skip_serializing_if = "Option::is_none"`
on the typed field and explicit `extra.insert` only when `Some`):

- Raw JSONL: `agent_pid` key omitted entirely when unavailable.
- Reports/queries (DTOs, SQL): `agent_pid: null` is an explicit value
  meaning "no provider child PID was available for that row".
- `claudine_pid` is non-null on every wrapper-emitted record. Older
  JSONL lines that pre-date the field are tolerated via
  `#[serde(default)]` and surface as `null` in queries (Phase 4 will
  document the backfill behavior).

## Surfaces to be Touched by Phase 2–5

Consolidated punch list, derived from the inventories above. Phase 2
touches the wrapper layer; Phase 3 touches the shared event/context
model; Phase 4 touches reporting; Phase 5 closes the loop with
end-to-end tests and documentation.

### Phase 2 (wrapper env + spawn capture)

- `claudine/cli/src/commands/wrap/env.rs` — add `CLAUDINE_PID` to the
  `EnvPlan` returned by `build_child_env_with_launch`.
- `claudine/cli/src/commands/wrap/exec/spawn.rs` — capture
  `child.id()` after each successful `command.spawn()?` in `run_child`,
  `run_child_capture`, `run_child_stream_semantic`.
- `claudine/cli/src/commands/wrap/exec/wiring.rs` — capture
  `child.id()` after the successful spawn in `run_kimi_wire_session`.
- Test coverage: env-plan unit test for `CLAUDINE_PID`; a lightweight
  fake-provider spawn test that confirms `child.id()` is captured.

### Phase 3 (shared context propagation)

- `claudine/lib/src/events/environment.rs` — add `claudine_pid`.
- `claudine/lib/src/events/event_meta.rs` — add `agent_pid`.
- `claudine/lib/src/dispatch/mod.rs::prepare_meta_for_dispatch` —
  propagate both PIDs into `meta.extra` for template/expression
  consumption.
- `claudine/lib/src/stream/reporting.rs` —
  `summary_to_event_meta_with_context` and
  `semantic_event_to_event_meta` accept and propagate the PID fields.
- Composition / harness-orchestration call sites that build
  `EventMeta` or `context_extra` maps (see spawn-path tree).

### Phase 4 (reporting schema + ingest + queries + CLI)

- `claudine/lib/src/reporting/schema.rs` — bump `SCHEMA_VERSION` to
  `"5"`, add `claudine_pid INTEGER NOT NULL` (default 0 for backfill)
  and `agent_pid INTEGER NULL` to `events`; add nullable `agent_pid`
  to `sessions`.
- `claudine/lib/src/reporting/ingest.rs` — read the typed PID fields
  when building `PreparedEvent`; reset ingestion state on schema
  migration so derived cache rows are rebuilt with PID columns.
- `claudine/lib/src/reporting/types.rs` and
  `claudine/lib/src/reporting/queries/*.rs` — surface nullable
  `agent_pid` on DTOs.
- `claudine/cli/src/commands/logs/*.rs` — render nullable `agent_pid`
  in text and JSON output without mislabeling null as missing.

### Phase 5 (verification + documentation)

- End-to-end fake-provider test: child env contains `CLAUDINE_PID`,
  summary `EventMeta` carries both PIDs, failed-spawn JSONL omits
  `agent_pid`.
- Update wrapper env-var docs, reporting-fields docs, and any
  in-repo skill/topic doc that lists `EventMeta` fields.
- Comment sweep on every touched symbol whose behavior changed.
