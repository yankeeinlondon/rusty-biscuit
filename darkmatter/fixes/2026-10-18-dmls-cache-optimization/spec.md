---
area: darkmatter
status: implemented
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-18
review_iterations: 0
clarified: true
clarified_by: claude/opus
needs_rulings: false
implemented: true
implemented_by: "claude/opus"
created: 2026-10-18
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - dmls
related:
    - 2026-10-17-logging-improvements
    - 2026-10-17-dmls-index-store
    - 2026-10-11-dmls-request-timeouts
human_review: true
---

# DMLS Trigger-Registry Cache Optimization

Opening an ordinary Markdown document right after DMLS starts can stall
every schema-dependent request (semantic tokens, folding, links, highlight,
code actions, …) for over a second. The document's own trigger registry
costs almost nothing to prepare. The time goes into **re-preparing every
workspace scope's registry**, and that preparation is slow because the
trigger-payload memo misses on nearly every lookup.

This fix makes four changes:

1. The payload memo hits across workspace scopes that resolve a trigger
   identically.
2. A configuration notification that changes nothing does not invalidate
   registries.
3. A document's registry job runs ahead of queued workspace preparation, and
   duplicate workspace jobs merge into one.
4. Startup and the registry worker never prepare the same scopes at the same
   time.

## The Incident

On 2026-10-18, with the `dmls` built from `2026-10-17-dmls-index-store`,
opening the `review-3.md` document for `2026-10-17-dmls-index-store`
in the editor logged:

```text
WARN dmls::request_timing: slow request method="textDocument/semanticTokens/full" … elapsed_ms=1170
  overlay_build_ms=0 schema_assembly_ms=0 trigger_registry_ms=0 provider_chain_ms=0
  waited_for_file_list=false waited_for_index=false waited_for_trigger_registry=true
WARN dmls::request_timing: slow request method="textDocument/documentLink" … elapsed_ms=1153 … waited_for_trigger_registry=true
WARN dmls::request_timing: slow request method="textDocument/foldingRange" … elapsed_ms=1154 … waited_for_trigger_registry=true
WARN dmls::request_timing: slow request method="textDocument/documentHighlight" … outcome="cancelled" elapsed_ms=984 … waited_for_trigger_registry=true
WARN dmls::request_timing: slow request method="textDocument/codeAction" … outcome="cancelled" elapsed_ms=803 … waited_for_trigger_registry=true
INFO dmls::router: schema registry ready roots=5 tree_schemas=2 scoped_schemas=7 trigger_schemas=3 … elapsed_ms=1256
```

Every handler phase is 0 ms and nothing waited for the index store. The
requests waited for a trigger registry, and all of them were released within
2 ms of the registry line.

### Reproduction

A stdio client that sends `initialize`, `initialized`, `didOpen` for the
document above, then `semanticTokens/full`:

| After `initialized` the client sends | First `semanticTokens/full` |
| --- | --- |
| nothing | 163 ms, no registry wait |
| `workspace/didChangeConfiguration` with `{"settings": {}}` | **1,549 ms**, `slow request`, `waited_for_trigger_registry=true` |

Editors send that notification as a matter of course after `initialized`.

### Measurements

A `sample` of the process during the stall put 577 of 602 registry-worker
samples in:

```text
prepare_workspace_registries → prepare_registry → Preparation::run
  → scan_with_source → file_payload_memo::resolve → resolve_file_payload
  → resolve_yaml_schema_with_roots → to_json_schema (recursive union conversion)
```

A temporary timer on memo misses (not committed) recorded **32 misses
totaling 2,408 ms of CPU** in one startup. Every one came from the three
trigger schemas in `claudine/schemas/` (`claudine.yaml` 80–91 ms,
`sequence.yaml` 106–130 ms, `inline-compose.yaml` 0 ms). Each was resolved
once per Claudine workspace probe (`claudine`, `claudine/catalog-types`,
`claudine/cli`, `claudine/contract`, `claudine/gen`, `claudine/lib`), all
with the same two schema roots. Every probe appeared **twice**, once from
startup's preparation and once from the worker's. The document itself
(`darkmatter/…`) caused no misses.

## Findings

### F1. A configuration notification that changes nothing invalidates every registry

In the `dmls` package, [ServerState::update_config](../../dmls/src/router.rs)
applies configuration reloads. It clears contexts, calls
`invalidate_all_trigger_registries()`, and queues a workspace preparation
**before** comparing the effective configuration, and does so even when the
comparison then finds no change. After the editor's routine startup
`didChangeConfiguration`, every open document's registry is stale, so its
next schema-dependent request waits for a re-preparation.

Contexts don't require that invalidation: `TriggerRegistries::is_fresh`
already rejects a scope prepared under a different context
(`scope.context == context.for_source(path)`), so a rebuilt context that
differs re-prepares by itself, and one that is equal stays fresh.

### F2. A document's job waits behind the whole workspace preparation

In the `dmls` package, [spawn_registry_worker](../../dmls/src/router.rs)
runs background registry preparation. It takes jobs from one unbounded FIFO channel. A `RegistryJob::Workspace`
queued by F1 runs before the `RegistryJob::Document` a request is parked on,
however cheap the document's own job is. `schedule_workspace` does not merge
with a workspace job already queued, so several triggers (config, a YAML
save, a relist) queue several full passes.

### F3. The trigger-payload memo misses across workspace probes

In the `darkmatter` library, [file_payload_memo](../../lib/src/markdown/schemas/triggers/assemble.rs)
caches resolved trigger payloads. It reuses an entry only when `(source, payload, schema_roots, folder_context)`
are all equal. `folder_context` is the requesting document's context with the
source path replaced by its folder, so every package folder is a distinct key
even though:

- `resolve_file_payload` resolves relative to the **trigger file's** folder
  (`base_dir = source.parent()`), and
- each lookup derives a context onto that folder or a schema root
  (`for_cwd(base_dir)`, `for_trusted_external_cwd(root)`), and `derive`
  recomputes the repository, package, and package-area scopes for the new
  `cwd`.

Derivation does not erase every document difference. It retains the request
boundary and tree, launch `@` scope, captured home and environment, repository
catalog, magic paths, vault roots, explicit tree controls, and inherited
validation failure. It also chooses whether to keep the current tree. In
the `dmls` package, [RepositoryContexts](../../dmls/src/context.rs) builds one context per
repository and derives all its documents from that context, so the launch
scope and request boundary agree across that repository's workspace probes.
That supports sharing for this incident, but does not justify sharing across
arbitrary library callers or across repositories.

The memo also holds `RECENT_PAYLOADS = 16` entries in insertion order. One
pass over this repository needs 18 distinct keys (6 probes × 3 triggers), so
the memo evicts its own entries within a single pass, and every later pass
(F1, a YAML save, every save in fallback watch mode, a rescan) pays the full
cost again.

### F4. Startup and the worker prepare the same scopes at the same time

In the `dmls` package, [spawn_startup](../../dmls/src/router.rs)
coordinates initial discovery and runs workspace registry preparation on its
own thread. A
workspace job queued meanwhile (F1) runs the same probes on the registry
worker in parallel. `TriggerRegistries::begin` starts a second
`Preparation` for a scope that already has one in flight, so both threads do
the full work, and the later install wins. CPU doubles; wall time does not
improve.

## Requirements

### R1. The payload memo is keyed by what resolution reads (F3)

- The memo key no longer includes the requesting document's folder. It
  includes only inputs that can change a resolved payload: the trigger
  path, the payload value, the schema roots, and the **request-side**
  context state that survives derivation onto the trigger's folder (the
  request boundary and its tree, launch `@` scope, home, environment, and
  inherited failure). The implementer confirms the exact field set by
  reading `FileResolutionContext::derive` and every context use in
  `resolve_yaml_schema_with_roots`. When in doubt, a field stays in the key:
  a miss costs time, a false hit serves a wrong schema.
- Use full `FileResolutionContext` equality on the contexts actually used by
  resolution, rather than hand-copying selected fields. The initial candidate
  key is the context derived with `for_cwd(source.parent())`, plus the ordered
  contexts derived **directly from the original context** with
  `for_trusted_external_cwd(root)` for each schema root. The first covers
  source-relative resolution; the others cover bare-name lookups in roots
  outside the repository. Keep the original context for execution.
- Confirm equivalence through imports, delegated schemas, and examples before
  dropping the original context from the key. In particular, deriving a
  trusted root from an already-normalized context can inherit a failure or
  tree choice that direct derivation would not have. If a resolution branch
  reads additional original state, retain that state or the original context
  for that branch. Never clear a validation failure or widen a trust boundary
  merely to obtain a hit. No new public context API is required by this fix.
- Sharing applies only to resolved payloads. Keep each document's registry,
  trigger matching context, `$path` pattern base, precedence, advisories, and
  diagnostic provenance unchanged. A memo hit returns the existing schema
  `Arc` identity as well as equivalent dependencies and shape.
- The capacity covers at least one full workspace pass with headroom. It may
  use a bounded fixed capacity; start with 32 entries, enough for the
  incident and the 18-distinct-input regression fixture. Count distinct full
  keys, not just trigger paths: roots and context can differ. Do not grow a
  process-wide cache without a bound, since expanded payloads can be large.
  Eviction becomes least-recently-used, and a hit refreshes recency. The current implementation is insertion-ordered, so this is an
  intentional behavior change.
- The existing fingerprint check (dependency bytes and searched folder
  listings) is unchanged.

**Target:** on this repository, one cold workspace pass resolves each
trigger payload **once per distinct resolution key** (3 misses for the
measured three-trigger input set, not 18 or 32), and a second forced scan with
no file changes resolves **none**. A fresh registry lookup bypasses the memo
entirely and cannot by itself demonstrate memo reuse.

### R2. A configuration notification that changes nothing invalidates nothing (F1)

- `update_config` compares the effective configuration first. When it is
  unchanged, no trigger registry is invalidated and no workspace job is
  queued.
- Clearing contexts may stay (it is cheap). A context that rebuilds
  differently re-prepares through `is_fresh`'s context comparison, not
  through a blanket invalidation.
- For this fix, any effective configuration change retains blanket trigger
  invalidation and schedules one merged workspace pass **after** applying the
  new configuration. This conservative choice avoids an incomplete list of
  relevant settings; narrowing invalidation further is out of scope.
- Keep existing graph relinking, configuration-error diagnostics, semantic
  token refresh decisions, and open-document diagnostics updates. An unchanged
  effective configuration can still rebuild a different repository context
  after files on disk changed; that document must re-prepare on next demand.
  The zero-preparation guarantee assumes unchanged context inputs too.
- Schema roots remain those of the five root kinds (`SchemaRootKind`:
  package, package area, tree, `SCHEMAS_DIR`, home) derived from the
  document's captured context; there is no new schema-root configuration setting. Home and
  environment remain captured at server startup. An unchanged notification
  does not mean zero work: configuration parsing and required context/graph
  refreshes may still occur.

### R3. Document jobs run first; workspace jobs merge (F2)

- The registry worker always takes a queued `Document` job before a queued
  `Workspace` job.
- At most one `Workspace` job is queued at a time; scheduling another while
  one is queued is a no-op. One that arrives while a workspace pass is
  **running** queues exactly one follow-up pass, so a change during the pass
  is not lost.
- A running workspace pass yields between probes when a document job is
  waiting: it finishes its current probe, runs the document job, and then
  continues. Post the document completion immediately so dispatch can resume
  its waiters; do not defer it until the workspace pass completes. An unrelated
  workspace pass adds at most its current probe's preparation to the wait.
  Earlier document jobs and the document's own preparation can still add time.
- Probe-list construction must also run off the dispatch thread. Check for
  document jobs before the first probe and between probes. Do not promise
  interruption inside a payload resolution or a filesystem operation.
- Track queued, running, and requested-again workspace work distinctly. Clear
  queued state atomically when a pass starts, not when dispatch later handles
  its completion. Otherwise a change between worker completion and dispatch
  handling can disappear. Merge all schedules received during one pass into
  one follow-up pass using the latest roots, contexts, and repository set.
- `outstanding` counts actual queued or running jobs, including one suspended
  workspace pass; it does not count each probe or each merged scheduling call.
  Startup phases need explicit accounting so observers cannot report startup
  complete between discovery and final preparation. Send one completion per
  executed job; update tests that previously counted duplicate schedules.

### R4. The registry worker owns all server preparation

Move both startup preparation phases onto the same worker used by document
and refresh jobs. This is the chosen design, rather than introducing a second
per-scope waiting mechanism. File discovery stays on its background thread;
neither that thread nor dispatch calls preparation directly. The worker
finishes a probe before starting another job, so two server jobs cannot run a
preparation concurrently. This is a DMLS guarantee, not a promise to serialize
all callers of the process-wide library memo.

Preserve startup's two phases: prepare the initially known workspace scopes,
then, once discovery has recorded the repositories it found, prepare any newly
known scopes. **File discovery starts at once and runs concurrently with the
initial phase** (it reads no schema files and does not use the worker); only
the second phase waits for both. Coordinate these phases through completion
messages; never block dispatch waiting for the worker. A document job can
overtake either phase.

Install and publish the file-list result once, as soon as discovery returns,
without waiting for any registry phase: the list's arrival is what releases
withheld path diagnostics and lets the index store start. This reverses
today's order, in which the file list is sent after the registry-ready event;
tests and docs that assert that order are updated. The existing one-time
`schema registry ready` summary and startup observer completion occur after
the final required preparation, so the summary still counts every startup
scope, with preparation elapsed time excluding the file walk and queue waits.
A later invalidation still makes affected scopes stale; startup completion
does not assert that no future refresh is needed.

```mermaid
sequenceDiagram
    participant Dispatch
    participant Worker as Registry worker
    participant Discovery as File discovery
    par
        Dispatch->>Worker: Initial workspace preparation
    and
        Dispatch->>Discovery: List files and repositories
    end
    Discovery-->>Dispatch: Discovery result
    Note over Dispatch: Install and publish the file list now
    Worker-->>Dispatch: Initial phase complete
    Dispatch->>Worker: Prepare newly discovered scopes (after both)
    Dispatch->>Worker: Document needed by a request
    Note over Worker: Finish current probe, then prioritize document
    Worker-->>Dispatch: Document ready; resume its waiters
    Worker-->>Dispatch: Final startup phase complete
    Note over Dispatch: Registry-ready summary
```

- Keep preparation I/O outside the overlay lock, and preserve the existing
  change/epoch replay. A relevant change during preparation leaves the
  installed scope stale; waiting requests recheck demand and queue again.
- A pending document job reads current context when it executes, or detects an
  outdated captured context and reschedules before releasing waiters. A config
  change while it is queued must not strand requests behind a stale result.
- Cancellation removes only that request, retaining work needed by other
  waiters or diagnostics. Shutdown answers all waiters once, stops taking new
  jobs, and releases startup coordination without waiting for a completion
  that can no longer be delivered. Preserve dispatch responsiveness.

### R5. Make the cost visible

- At `debug`, one line per workspace pass:
  `trigger registries prepared probes=… prepared=… fresh=… payload_hits=… payload_misses=… elapsed_ms=… workspace=…`.
- The `slow request` line already reports `waited_for_trigger_registry`.
  Add `trigger_registry_wait_ms`, the time parked on a registry job, so
  total waiting can be distinguished from synchronous handler work. This
  duration includes both queueing and background preparation; it does not
  separate those two costs. Use the injected server clock. Sum separate parked
  intervals, stopping each at resume, cancellation, or shutdown; do not count
  intervening handler work or waits for the index. Emit it on the existing
  completion event, including canceled requests, without adding a second event.
- Count hits and misses attributable to that workspace pass, excluding document
  jobs that interrupt it; process-global counter deltas would mix those jobs.
  A hit means fingerprint-validated reuse, and a miss means an attempted
  uncached payload resolution. Fresh scopes perform neither. Include failed
  probes separately from fresh and successfully prepared probes. Preserve
  existing info-level startup summaries and failure/recovery transitions.
- Never log payload contents, captured environment values, or schema text.

## Acceptance Criteria

1. An unchanged startup configuration notification adds no workspace pass or
   blanket invalidation. A request for a fresh document registry proceeds
   immediately; a cold document can wait for its own job and the current
   workspace probe, but overtakes remaining workspace probes.
2. A cold scan of the measured three-trigger input set records 3 payload
   misses; a second forced scan with unchanged inputs records 0. Other roots
   supplied by a user's home or environment can legitimately add keys, so a
   universal count of 3 for every checkout is not required.
3. An unchanged configuration notification with unchanged context inputs
   causes zero additional preparations relative to the same startup without
   the notification. Already scheduled startup work may still finish.
4. Multiple queued workspace schedules merge, and schedules during a running
   pass produce one follow-up pass, including the worker-completion/dispatch
   boundary. Document completion releases waiters before remaining probes.
5. No two server preparations run concurrently. A relevant change during a
   preparation leaves that scope stale, and the next demand recovers. Existing
   change-replay and failure-transition tests continue to pass.
6. Resolved registries, diagnostics, and trigger summaries are identical on the
   existing fixtures under `darkmatter/tests/fixtures/schema-triggers/`,
   including package-specific payload selection and pattern matching.

Record one quick before/after sample of the reported stdio reproduction on
one quiet host in the implementation log. Retain **≤ 250 ms** as the original
incident's non-gating target, not a portable pass/fail threshold. Record any
remaining wait and its cause. No additional performance spike, host matrix,
repetition requirement, or statistical gate is needed: the existing profile
already identifies the expensive work.

## Tests

Run through `just test` (nextest) in `darkmatter/`. Use the existing work
counters and injected gates, not timing. Hermetic stdio transport tests are
L1: they require no terminal, browser, or live editor. Use temporary repository
fixtures and the existing `LspFixture` snapshot/context controls, with no host
home or environment dependency. Gates must have bounded failure cleanup so a
regression fails rather than hangs. Use the monorepo Test Toolkit. Run the
area's `just lint` after implementation; no new CI tier or scope is needed.

| Requirement | Tier | Test |
| --- | --- | --- |
| R1 | L1 darkmatter | two documents in different package folders of one repository, same schema roots: the second resolution of a trigger payload is a memo hit (miss counter seam) |
| R1 | L1 darkmatter | distinct repository/request boundaries, launch scopes, home/environment, explicit tree controls, or inherited failure never reuse incompatible payloads; cover a trusted external root and inline imports/examples as well as relative payload files |
| R1 | L1 darkmatter | 18 genuinely distinct full keys followed by a forced repeat scan: zero misses; a hit protects an entry from eviction and the least recently used entry is evicted at the bound |
| R1 | L1 darkmatter | a changed dependency file or a file added to a schema root still misses (fingerprint unchanged) |
| R2 | L1 dmls | an unchanged `didChangeConfiguration` leaves every registry fresh and queues no job; an effective config change (for example workspace discovery settings) invalidates; unrelated file/config diagnostics and refresh behavior remain intact |
| R2 | L1 dmls | a context that rebuilds differently after a reload re-prepares on next need without a blanket invalidation |
| R3 | L1 dmls | with the worker gated, queue Workspace, Workspace, Document: the document job runs first and one workspace pass runs |
| R3 | L1 dmls | a document job queued during a gated workspace pass runs after the current probe, before the next |
| R3 | L1 dmls | a workspace schedule during a running pass yields exactly one follow-up pass |
| R4 | L1 dmls | startup and refresh scheduled together: the shared worker never runs preparations concurrently; fresh probes are skipped, and discovered repositories are prepared before startup completion |
| R4 | L1 dmls | a change recorded during the single in-flight preparation leaves the scope stale |
| R5 | L1 dmls | captured `tracing` shows pass-local counts excluding interrupting document jobs; an injected clock proves summed registry waits, including resume/requeue, cancellation, and shutdown, on one completion event |
| Startup request ordering | L1 dmls | hermetic stdio session with the existing `LspFixture`: gate an unrelated workspace probe, send unchanged configuration, open a document and request semantic tokens; release the current probe and observe the response before the next workspace probe completes; allow `waited_for_trigger_registry=true` for a cold document |
| Concurrent discovery | L1 dmls | with the worker's initial phase gated, discovery completes and the file list is installed and published before the gate is released; the second phase starts only after both, and the registry-ready summary follows it and counts every startup scope |
| Worker completion boundary | L1 dmls | schedule a workspace refresh after the worker completes but before dispatch handles its completion; exactly one new pass runs |
| Lifecycle | L1 dmls | multiple waiters share a document job; cancel one and answer the other once; shutdown during either startup phase releases waiters and coordination; a context change during queued work cannot strand them |

## Documentation

- `darkmatter/docs/lsp/indexing.md`: job ordering, workspace-job merging, when a config
  reload invalidates registries, and what the payload memo is keyed on, with
  a Mermaid sequence for "request parked on a document job while a workspace
  pass runs".
- `darkmatter/docs/lsp/configuration.md`: an unchanged effective reload
  does not invalidate registries when context inputs are unchanged; every
  effective change retains conservative invalidation.
- `.claude/skills/darkmatter/dmls.md`: the worker's priority rule and the
  memo key.
- The `file_payload_memo` and `update_config` doc comments, which describe
  the old keying and the "even when unchanged" invalidation.

## Out of Scope

- Making a single payload resolution cheaper (`to_json_schema`'s recursive
  union conversion, 80–130 ms for Claudine's triggers). With payload sharing it runs once
  per distinct retained key while inputs are unchanged; optimizing it is a separate fix if it still
  matters.
- Persisting resolved payloads across restarts.
- Lazily skipping workspace probes nobody has opened a document in.

## Review Notes

The review chooses one worker for server preparation and conservative
invalidation on every effective configuration change. These choices avoid
cross-thread duplicate preparation and an incomplete dependency list without
changing schema-selection semantics. The previous draft called eviction
least-recently-used even though the code evicts by insertion order; the new
recency rule is now explicit. It also required a cold document to report no
registry wait, which conflicts with per-document preparation; the acceptance
criteria now prove queue ordering and notification behavior directly.

## Rulings

1. **Discovery runs concurrently with the initial registry phase** (author,
   2026-10-18). The walk does not depend on that phase, and the file list
   gates path diagnostics and the index store, so it is published as soon as
   it is ready rather than after the registry-ready event.

## Open Questions

None blocking. The context-key equivalence checks above are required
correctness work, not permission to broaden sharing if a check fails. Record
any necessary additional key inputs, and any departure from this design, in
the implementation log while leaving this reviewed snapshot intact.
