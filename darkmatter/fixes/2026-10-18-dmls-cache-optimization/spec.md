---
area: darkmatter
status: draft-spec
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
reviewed: false
clarified: false
implemented: false
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
   identically (F3, R1).
2. A configuration notification that changes nothing does not invalidate
   registries (F1, R2).
3. A document's registry job runs ahead of queued workspace preparation, and
   duplicate workspace jobs merge into one (F2, R3).
4. Startup and the registry worker never prepare the same scopes at the same
   time (F4, R4).

## The Incident

On 2026-10-18, with the `dmls` built from `2026-10-17-dmls-index-store`,
opening `darkmatter/features/2026-10-17-dmls-index-store/review-3.md` in the
editor logged:

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

`ServerState::update_config` (`dmls/src/router.rs`) clears contexts, calls
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

The registry worker (`spawn_registry_worker`, `dmls/src/registry_jobs.rs`)
takes jobs from one unbounded FIFO channel. A `RegistryJob::Workspace`
queued by F1 runs before the `RegistryJob::Document` a request is parked on,
however cheap the document's own job is. `schedule_workspace` does not merge
with a workspace job already queued, so several triggers (config, a YAML
save, a relist) queue several full passes.

### F3. The trigger-payload memo misses across workspace probes

`file_payload_memo` (`darkmatter/lib/src/markdown/schemas/triggers/assemble.rs`)
reuses an entry only when `(source, payload, schema_roots, folder_context)`
are all equal. `folder_context` is the requesting document's context with the
source path replaced by its folder, so every package folder is a distinct key
even though:

- `resolve_file_payload` resolves relative to the **trigger file's** folder
  (`base_dir = source.parent()`), and
- each lookup derives a context onto that folder or a schema root
  (`for_cwd(base_dir)`, `for_trusted_external_cwd(root)`), and `derive`
  recomputes the repository, package, and package-area scopes for the new
  `cwd`.

What survives the derivation and can still differ between two requesting
documents is the request-side state: `request_cwd` and its tree, the launch
`@` scope, an inherited validation failure, and the home and environment.
Within one repository and one DMLS process these are expected to agree for
every workspace probe, but nothing proves it yet.

The memo also holds `RECENT_PAYLOADS = 16` entries in insertion order. One
pass over this repository needs 18 distinct keys (6 probes × 3 triggers), so
the memo evicts its own entries within a single pass, and every later pass
(F1, a YAML save, every save in fallback watch mode, a rescan) pays the full
cost again.

### F4. Startup and the worker prepare the same scopes at the same time

`spawn_startup` runs `prepare_workspace_registries` on its own thread. A
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
- The preferred shape is a key built from a context **normalized onto the
  trigger's folder** (for example `context.for_cwd(source.parent())` with the
  source path cleared), so equality is decided by `FileResolutionContext`'s
  own `PartialEq` and does not drift when the context gains fields.
- The capacity covers at least one full workspace pass with headroom. It may
  be sized from the number of distinct trigger files rather than a fixed
  constant. Eviction remains least-recently-used, and a hit refreshes
  recency.
- The existing fingerprint check (dependency bytes and searched folder
  listings) is unchanged.

**Target:** on this repository, one cold workspace pass resolves each
trigger payload **once** (3 misses, not 18 or 32), and a second pass with no
file changes resolves **none**.

### R2. A configuration notification that changes nothing invalidates nothing (F1)

- `update_config` compares the effective configuration first. When it is
  unchanged, no trigger registry is invalidated and no workspace job is
  queued.
- Clearing contexts may stay (it is cheap). A context that rebuilds
  differently re-prepares through `is_fresh`'s context comparison, not
  through a blanket invalidation.
- When the configuration did change, invalidate only when the change can
  affect registries (schema roots, workspace roots or globs, anything
  `SchemaRoots::for_document` reads). Otherwise the existing behavior is
  kept.

### R3. Document jobs run first; workspace jobs merge (F2)

- The registry worker always takes a queued `Document` job before a queued
  `Workspace` job.
- At most one `Workspace` job is queued at a time; scheduling another while
  one is queued is a no-op. One that arrives while a workspace pass is
  **running** queues exactly one follow-up pass, so a change during the pass
  is not lost.
- A running workspace pass yields between probes when a document job is
  waiting: it finishes its current probe, runs the document job, and then
  continues. A request never waits for more than one probe's preparation
  plus its own.
- `outstanding` accounting and the startup observer events stay correct
  under merging. Tests that count jobs are updated to the merged count.

### R4. One preparation per scope at a time (F4)

- A scope whose preparation is already in flight is not prepared a second
  time concurrently. The second caller either waits for the in-flight result
  or skips the scope and relies on the in-flight result being installed.
  The choice is the implementer's, but it must not hold the overlay lock
  across preparation I/O (the existing rule in `overlay/registry.rs`).
- A change recorded while the first preparation ran still leaves the scope
  stale afterward (the existing `changes`/`epoch` replay), so skipping never
  installs an out-of-date registry.
- Alternatively, startup's workspace preparation moves onto the registry
  worker as its first job, which removes the second thread entirely. This is
  acceptable if R3's ordering still lets a document job overtake it.

### R5. Make the cost visible

- At `debug`, one line per workspace pass:
  `trigger registries prepared probes=… prepared=… fresh=… payload_hits=… payload_misses=… elapsed_ms=… workspace=…`.
- The `slow request` line already reports `waited_for_trigger_registry`.
  Add `trigger_registry_wait_ms`, the time parked on a registry job, so
  queueing (F2) and preparation can be told apart from the log alone.
- Never log payload contents.

## Acceptance Criteria

1. With a client that sends `didChangeConfiguration` with unchanged settings
   right after `initialized`, the first schema-dependent request on a
   Markdown document outside `claudine/` does not wait for any workspace
   probe's preparation. On this repository it answers in **≤ 250 ms** on a
   quiet host (measured, recorded in the implementation log; not an L1 gate).
2. One cold startup on this repository records exactly 3 payload misses; a
   second workspace pass with no file changes records 0.
3. An unchanged `didChangeConfiguration` causes zero registry preparations.
4. While a workspace pass is running or queued, a document job queued after
   it is prepared before the pass's remaining probes.
5. No scope is prepared twice concurrently, and a change during a
   preparation still leaves that scope stale (existing replay tests pass
   unchanged).
6. Resolved registries, diagnostics, and trigger summaries are identical to
   today's on the existing trigger fixtures (`darkmatter/tests/fixtures/schema-triggers/`).

## Tests

Run through `just test` (nextest) in `darkmatter/`. Use the existing work
counters and injected gates, not timing.

| Requirement | Tier | Test |
| --- | --- | --- |
| R1 | L1 darkmatter | two documents in different package folders of one repository, same schema roots: the second resolution of a trigger payload is a memo hit (miss counter seam) |
| R1 | L1 darkmatter | a document in another repository, or with a different home or environment, is a miss (no false hit) |
| R1 | L1 darkmatter | 18 distinct keys followed by a repeat pass: the repeat pass has zero misses (capacity) |
| R1 | L1 darkmatter | a changed dependency file or a file added to a schema root still misses (fingerprint unchanged) |
| R2 | L1 dmls | an unchanged `didChangeConfiguration` leaves every registry fresh and queues no job; a change to a schema-root setting invalidates |
| R2 | L1 dmls | a context that rebuilds differently after a reload re-prepares on next need without a blanket invalidation |
| R3 | L1 dmls | with the worker gated, queue Workspace, Workspace, Document: the document job runs first and one workspace pass runs |
| R3 | L1 dmls | a document job queued during a gated workspace pass runs after the current probe, before the next |
| R3 | L1 dmls | a workspace schedule during a running pass yields exactly one follow-up pass |
| R4 | L1 dmls | startup preparation and a worker workspace job overlapping on a gate: each probe's `Preparation::run` executes once (registry-scan counter) |
| R4 | L1 dmls | a change recorded during the single in-flight preparation leaves the scope stale |
| R5 | L1 dmls | captured `tracing` subscriber shows the pass line and the `trigger_registry_wait_ms` field |
| Acc. 1 | L2 dmls | stdio session: `initialize`, `initialized`, `didChangeConfiguration`, `didOpen`, `semanticTokens/full` on a fixture with a costly trigger in another package; the request reports no registry wait (`waited_for_trigger_registry=false`) |

## Documentation

- `darkmatter/docs/lsp/indexing.md` (or the page that describes startup and
  trigger registries): job ordering, workspace-job merging, when a config
  reload invalidates registries, and what the payload memo is keyed on, with
  a Mermaid sequence for "request parked on a document job while a workspace
  pass runs".
- `darkmatter/docs/lsp/configuration.md`: a reload with unchanged settings
  is free; which settings re-prepare trigger registries.
- `.claude/skills/darkmatter/dmls.md`: the worker's priority rule and the
  memo key.
- The `file_payload_memo` and `update_config` doc comments, which describe
  the old keying and the "even when unchanged" invalidation.

## Out of Scope

- Making a single payload resolution cheaper (`to_json_schema`'s recursive
  union conversion, 80–130 ms for Claudine's triggers). With R1 it runs once
  per trigger per process; optimizing it is a separate fix if it still
  matters.
- Persisting resolved payloads across restarts.
- Lazily skipping workspace probes nobody has opened a document in.

## Open Questions

1. **R1 key fields.** Does any field that survives `derive` (request
   boundary, launch `@` scope) actually vary between workspace probes of one
   repository? If it does, R1's target of 3 misses is unreachable without a
   further argument that the varying field cannot change a payload. The
   implementer reports this before planning closes.
2. **R2 scope of "affects registries".** Is there a configuration key beyond
   schema roots and workspace discovery that changes a trigger registry? If
   that is hard to enumerate, invalidating on any effective change (but not
   on an unchanged one) is the fallback.
