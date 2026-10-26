# DMLS

DMLS is Darkmatter's language server. Use this reference for protocol,
workspace, completion, hover, and diagnostic work.

## Contents

- [Architecture](#architecture)
- [Passive-analysis contract](#passive-analysis-contract)
- [File-resolution contexts](#file-resolution-contexts)
- [Features](#features)
- [Undeclared properties and unknown functions](#undeclared-properties-and-unknown-functions)
- [Call diagnostics](#call-diagnostics)
- [Rollout chronology](#rollout-chronology)
- [Verification](#verification)

## Architecture

The package area has two editor-facing crates:

- `dmls`: the Language Server Protocol server and workspace/document state.
- `zed-dmls`: the Zed extension that launches and configures DMLS.

DMLS owns protocol transport, document snapshots, workspace indexing,
capability negotiation, and publication. Darkmatter owns Markdown parsing,
frontmatter spans, SimplifiedSchema, expression parsing, schema selection, and
validation. Do not fork those authorities in the server.

## Passive-analysis contract

Editor analysis must be safe for every keystroke:

- No shell execution or interpolation effects.
- No remote fetch, credential lookup, or implicit network access.
- No file mutation.
- No ambient CWD/repository recapture for an already-open document.
- No target-specific rendering required to classify a problem.

Use source text plus captured document/workspace context. Keep incomplete syntax
recoverable where possible and publish typed diagnostics with stable ranges.

## File-resolution contexts

User-facing contract: `dmls/docs/file-references.md`. Code: `dmls/src/context.rs`.

- `RunOptions::new(snapshot)`: `main.rs` calls `RequestSnapshot::from_process()`
  once and hands it to the server (and to `--bench-index`). Nothing else in
  DMLS reads process `HOME`, environment, or CWD for resolution.
- `RepositoryContexts` (shared `Arc`, on `ServerState.contexts`) caches one
  `build_resolution_context` result per repository root (key from
  `biscuit_file::find_git_root`; a document in no repository keys its folder),
  failures included. `for_document(path)` derives with `for_source`;
  `for_untitled(roots)` needs exactly one repository across the folders.
- Every provider resolves through `DocumentContext.resolution`
  (`ctx.file_context()` is `Result<&FileResolutionContext, &ContextFailure>`,
  `ctx.resolve_reference(raw)`; no context is ever an `Option`, which the
  context guard enforces). A failed resolution
  means **no** resolution: never reintroduce a lexical join (the old
  `normalize_join` is deleted). `providers/diagnostics.rs` publishes the one
  `dm.context.build_failure` diagnostic. With a failed context the overlay
  assembles no schema at all (`SchemaOutcome::Ready(None)`): `$schema` and
  trigger payloads cannot resolve without one, and `DarkmatterSchemas::new`
  requires a context.
- User-facing docs: `darkmatter/docs/lsp/indexing.md` (startup order, file
  list, on-demand parsing, the index build and store, log lines) and
  `darkmatter/docs/lsp/configuration.md` (`.dmls.toml`, the `[workspace]`
  `root`/`include`/`exclude` keys). Update them with any behavior change here.
- Startup lists files and parses only open buffers. `discover_workspace`
  returns Markdown (`include`/`exclude`) and YAML (`exclude` only) paths,
  with `require_git(false)`, `.git` skipped, symlinks not followed, no
  contents read, and walk errors marking the list incomplete; it runs on a
  background thread (progress token `dmls/startup-index`) and reaches the
  dispatch thread as an `Internal` event on the router's `select!`, which owns
  every index mutation. `WorkspaceIndex` holds the list: every listed
  Markdown file is a graph document, unparsed until needed
  (`DocumentRecord::parsed == false`, root node only, anchors to it
  unchecked). Parse on demand with `prepare_targets` (fragment links, heading
  wiki links, the transclusion closure) or `ensure_parsed`; a disk change
  marks a parse stale (xxHash reuse on re-read, a failed read drops it); open
  buffers override disk until `close_buffer`. Diagnostics that depend on which paths
  exist are withheld while `WorkspaceGraph::listing()` is `Pending`
  (`router::listing_gates`).
- Startup (the overlay lock is not held across preparation I/O): the
  registry worker owns all preparation. `PendingRegistries::begin_startup`
  queues `RegistryJob::Startup(StartupPhase::Initial)` (a pass over
  `OverlayState::workspace_probes`: a probe document `.dmls-workspace` in
  each folder, catalog package and area inside it, and monorepo
  first-directory fallback area with a `schemas` folder, `fallback_areas`)
  while `router::spawn_startup` runs only the file-list walk at once (its
  `filter_entry` also records each directory holding `.git` as
  `DiscoveryReport::repositories`). `Internal::FileList` is installed as
  soon as it arrives (`ServerState::install_file_list`, logs `workspace
  files listed markdown yaml duplicates elapsed_ms`); the dispatch thread
  queues `StartupPhase::Discovered` only after **both** that and
  `Internal::StartupPhase{Initial}`, and only when
  `set_workspace_repositories` changed the set. After the last phase
  `Router::finish_startup` logs `schema registry ready` (or `… initialized
  with failures` when `failed_scopes > 0`; one line; `elapsed_ms` is
  probe-preparation time only) then sends `RegistryReady` and
  `RegistryPrepared{Startup}`. Startup counts as **one** outstanding job
  from server start to that line, so `flush_server`/`registry_jobs_until_idle`
  wait for it; shutdown (`PendingRegistries::shut_down`) closes the queue,
  drops startup, and zeroes the count. A re-list
  (`ServerState::relist`: rescan, config or ignore-file change) logs
  `workspace rescan complete` with the same fields, and queues
  `RegistryJob::Workspace` when its repository set changed (as does a
  watched `.git` change, `ServerState::note_repository_change`); a ready
  index build logs `index store revalidated markdown unchanged reparsed
  removed trigger elapsed_ms` (counts summed over its rounds, elapsed from the
  first waiter, including any file-list wait). Every workspace log line,
  including the store's (`index store opened`, `… unavailable, using
  memory`, `… recreated`, `… sweep`; `RootFacts`/`WorkspaceFacts::open` take
  the identifier), carries `workspace=` (`workspace::workspace_identifier`:
  the client's folder URIs, comma-joined, one line per server). Log string
  fields with `%` so they print unquoted. Never log file contents or heading
  text; `tests/l1/index_store_log_lines.rs` asserts each store line once.
- The index build (`index_build.rs`, router): references, rename,
  `willRenameFiles`, and `workspace/symbol` (`Trigger::for_method`; never
  `prepareRename`) are parked as `Waiter`s with their own `RequestTimings`
  while the store is not ready (`Router::needs_index_build`).
  `IndexBuildState` (`NotStarted`/`Building(Build)`/`Ready`/`Failed`) holds
  at most one `Build`, whose rounds each run one store pass on a thread
  (`FactsPlan::Full`, then `Paths(touched)`); the dispatch thread
  (`finish_index_round`) starts another round while anything changed during
  the last. A listed file the pass could not read, or found missing, fails
  the build (`RequestFailed`, file named) unless it changed meanwhile;
  cancelling the last waiter drops the build (its `Drop` sets the cancel
  flag; rows already written stay); shutdown answers waiters
  `ServerCancelled`. Progress token `dmls/index`. There is **no** production
  graph over every listed file (the full in-memory build is deleted;
  `tests/l1/full_index_removal_guard.rs` rejects its names in `src/`): the
  live snapshot holds open buffers and lazily parsed targets only, and a
  whole-workspace graph exists only as the test `Oracle`.
- `dmls --bench-index <dir>` (`bench.rs`): `bench_cold` (list, create and
  fill a store in a caller-given directory) and `bench_warm` (open,
  revalidate, then references, heading rename, and workspace symbols as the
  server answers them, plus the peak resident memory the warm half adds).
  The binary runs the warm half in a child process (internal
  `--bench-warm <store dir>`, not in the usage text) so the cold fill's peak
  does not mask it, over a temporary store directory it removes;
  `bench_index` runs both in one process (tests:
  `tests/l1/bench_index.rs`).
- Trigger registries (`registry_jobs.rs`, router): the dispatch thread only
  looks registries up (`TriggerRegistries::lookup`/`is_fresh`,
  `OverlayState::registry_demand`); it never prepares one. A document whose
  registry is not fresh gets one `RegistryJob::Document` on the registry
  worker (`spawn_registry_worker`); schema-dependent requests
  (`SCHEMA_DEPENDENT`) park as `Waiter`s on it (`Router::proceed` checks the
  index build, then the registry), `refresh_diagnostics` holds the
  document's publish until it installs, and refreshes schedule a workspace
  pass (`ServerState::prepare_workspace_registries`). `RegistryQueue` has a
  document lane, a startup-phase lane, and one workspace slot
  (`Idle`/`Queued`/`Running`/`RequestedAgain`, changed under one lock by both
  threads): documents always go first, then startup phases, then the
  workspace pass; schedules merge into one queued pass or one follow-up, and
  a running pass (`RegistryWorker::run_pass`, for both kinds) runs waiting
  document jobs before each probe. `outstanding` counts each queued document
  job, the queued/running pass and its follow-up, and startup (one). No two
  server preparations overlap (one worker thread;
  `work_counters::peak_concurrent_registry_scans` proves it in tests). A
  `didChangeConfiguration` with an unchanged effective config invalidates no
  registry and schedules nothing (`update_config`); `is_fresh` comparing the
  context re-prepares a document whose context rebuilt differently. Test
  seams: `StartupHooks::registry_gate` holds each job,
  `registry_probe_gate` (`LspFixture::start_with_registry_probe_gate`,
  `StartupEvent::WorkspaceProbePreparing`) holds each workspace-pass probe,
  `startup_probe_gate` (`start_with_startup_probe_gate`,
  `StartupEvent::StartupProbePreparing(phase)`) holds each startup probe
  preparation **after its reads, before its install** (so a change then
  leaves the scope stale; a fresh probe never waits),
  `StartupEvent::StartupPhaseFinished(phase)` marks each phase handled,
  `LspFixture::startup_events_until` reads the event sequence, and
  `LspFixture::registry_jobs_until_idle` returns the finished job kinds in
  order. Preparation is `begin` (lock) → `Preparation::run` (no lock) → `install`
  (lock; an older `seq` is dropped, a change since `begin` leaves it
  stale). `OverlayState::for_document` (no server) prepares inline; the
  router uses `for_document_timed`, lookup only. Cancelling a registry
  waiter keeps the job; shutdown answers waiters `ServerCancelled`.
- L1 sessions: `LspFixture::initialize` waits for the list
  (`StartupEvent::FileListInstalled`, which now **precedes** `RegistryReady`)
  and for the startup `RegistryPrepared` (`wait_for_startup_events`),
  `initialize_unlisted` for neither (a test that holds the walk must not
  `flush_server` before releasing it: startup is still outstanding); `start_with_file_list_gate` holds the walk and
  `start_with_index_round_gate` holds each build round after its store pass
  (`wait_for_event(IndexRoundRead)`, `release_index_round`,
  `open_index_round_gate`); `start_with_registry_gate` holds each registry
  job after startup (`wait_for_event(RegistryPreparing)`,
  `release_registry_job`, `open_registry_gate`), and
  `wait_for_registry_prepared`/`next_registry_prepared` return the thread a
  job ran on. `flush_server` is a `workspace/symbol` round trip that also
  waits for every queued registry job (`RegistryJobsOutstanding` events) and
  buffers what they published; `round_trip` does not wait for jobs. Since
  `workspace/symbol` runs the index build, a parse-counting test must not
  use either as a barrier, and its response lists a `File` symbol per
  document beside each heading. `work_counters::registry_scan_threads`
  names every thread a registry scan ran on.
  Startup's registry preparation publishes a skipped trigger file's
  (unversioned) warning before any open; wait for the versioned one.
- The graph takes `&dyn DocumentContexts` (`WorkspaceIndex::new(contexts)`):
  `arena::locate` takes the first planned candidate when it is a graph
  document (listed or open; no probe), else asks
  `context::resolve_reference`, else, only on a clean `NoMatch`, a later
  graph-document candidate (an unsaved buffer); an `Io` probe failure stays a
  broken path. An existing file outside the list is `EdgeTarget::File`, never
  a broken link; the list decides graph documents, never existence. A
  Markdown `File` target an open document needs (`prepare_targets`, hover,
  definition, anchor completion) is parsed as a dependency
  (`DocumentRecord::dependency`: headings and transclusions only; excluded
  from wiki candidates, path completion, workspace symbols, `transcluded_by`,
  and the index store; its folder joins the schema-location watches; tests in
  `tests/l1/referenced_targets.rs`). Tests use `context::test_support`
  (`abs`, `workspace_contexts`, `resolution_for`) because a context rejects a
  rootless `/w` path on Windows; integration tests use `FixedContext` or a
  real `RepositoryContexts`; empty graphs use `NoContexts`
  (`ContextFailure::NotProvided`).
- Wiki candidates come from a caller-supplied `wiki::WikiUniverse` (one slot
  per file, canonicalized against its wiki roots), not from the graph's
  documents: `build_with_roots`/`build_listed` take it, `assemble` appends any
  graph document it lacks (a dependency as an unmatchable slot), and every
  wiki matcher call indexes `graph.wiki_universe().docs()` by
  `wiki_slot(doc)` / `slot_document(slot)`, never by `DocumentId`. A graph
  over a few documents therefore judges ambiguity and rename spellings
  against the whole universe: an ambiguous link lists only in-graph
  candidates, and a unique match outside the graph is
  `WikiResolution::OutsideGraph` (no edge, no diagnostic).
  `WorkspaceIndex::rebuild` passes its own file list plus non-dependency
  documents; `WorkspaceGraph::build` (tests) passes its documents. The
  equivalence tests are `tests/l1/wiki_universe_equivalence.rs`
  (`Oracle::scoped`, `UniverseScope`).
- `index_store` (persistent per-file facts behind references, rename, and
  workspace symbols, which answer only from it): `FileFacts` is the
  R1 row (`from_index`/`into_index`, unstored fields empty), `keys` derives
  over-approximating candidate keys (joined key for `./`/`../`, joined plus
  file-name key for bare paths, file-name key for wiki and root-prefixed
  forms; both raw and Markdown-extension-elided final segments), and
  `IndexStore` is one root's `STRICT` SQLite store in WAL mode. Every row
  decode failure is `StoreFailure::Corrupt`, never an empty result. Each
  holder takes a shared `std::fs::File` lock on `<store>.lock`; the sweep and
  corruption repair need the exclusive lock. `RootFacts` is the infallible
  R12 ladder (recreate once at open, else `MemoryFacts` with one process-wide
  `warn`; after a mid-process switch, `take_switched()` tells the caller to
  revalidate). `StoreLocation::prepare` takes the `DMLS_CACHE_DIR` value and
  `user_cache_dir()` as parameters, so the module reads no environment.
  User-facing contract (location, lock and sweep, multiple processes, the
  ladder, sizing, network-filesystem limit): `docs/lsp/indexing.md` "The
  index store"; `DMLS_CACHE_DIR` and why it is no `.dmls.toml` key:
  `docs/lsp/configuration.md`; the request graph's external wiki universe:
  `docs/lsp/architecture.md`. Update those pages with any store change.
  Bump `FACTS_VERSION` for any extraction change; the golden test
  (`tests/l1/index_store_facts.rs`, regenerate with `DMLS_UPDATE_GOLDEN=1`)
  refuses to regenerate without the bump. Tests: `tests/l1/index_store_*.rs`
  and the ladder's unit tests in `src/index_store/source_tests.rs`.
- Store wiring: `WorkspaceFacts` (`index_store/workspace.rs`) holds one
  `RootFacts` per effective root plus the hash of every file read in this
  process. `revalidate` (full pass, deletes missing rows) and `refresh` (a
  few paths) read and hash in parallel and re-parse only rows that differ,
  reusing any supplied parse whose hash matches the bytes (open buffers, and
  for a refresh the index's own parses); `request_documents` overlays
  buffers and never writes. Ready =
  `mark_ready` after a round with no touched paths, same roots, every listed
  path verified; a store switch withdraws it. The router side is
  `router/facts.rs`: `StoreInputs` takes `DMLS_CACHE_DIR` and
  `biscuit_file::user_cache_dir_from(env, home)` from the request snapshot
  (no ambient read; the fixture snapshot's home puts test stores under the
  workspace), each build round runs a `FactsPlan` pass on its thread
  (`Full` first, then `Paths(touched)`; `Full` again after a switch), events
  after ready refresh at once (client watching) or wait for the next save
  (fallback; the save also hands over what `recheck_parsed` found changed),
  and closing a buffer counts as a change to its path. Every rename first
  refreshes its
  closed target and closed candidates (`refresh_rename_referrers`, before the
  graph is built: a stale span would make the provider drop the edit); in
  fallback mode a rename waits for its own full pass (`Router::revalidated`).
  Session tests: `tests/l1/index_store_live.rs`; set `DMLS_CACHE_DIR` in the
  snapshot env.
- Store-backed requests: `WorkspaceFacts::candidates_for(targets, listed)`
  (union over roots, listed paths only, once per path),
  `request_graph(targets, listed, overlay, universe, contexts)` (candidates +
  listed targets + overlay into `build_with_roots`, universe = the live
  snapshot's `shared_wiki_universe()`), and `symbol_documents(listed,
  overlay)` (stored headings, no graph). Router: `ServerState::request_graph`
  (overlay = open buffers + unlisted targets' `disk_parse`),
  `reference_targets` (document + `providers::reference_target_at`),
  `with_document_over(uri, graph, f)`. `Provider::workspace_symbols` takes
  `&[SymbolDocument]` (`(path, Vec<HeadingFact>)`);
  `symbols::graph_symbol_documents` adapts a graph (the oracle uses it).
  Equivalence with the oracle at every corpus probe:
  `tests/l1/index_store_requests.rs` (`Oracle::*_over(graph, ...)`); restart,
  two-root, nested-root sessions: `index_store_sessions.rs`; spawned binary:
  `index_store_stdio.rs`. A `../` link out of a workspace folder resolves only
  inside a repository (no store involved); give a cross-root fixture a `.git`.
- Invalidation: every watched path calls `invalidate_contexts`, which drops
  ancestor-keyed entries (plus folder keys inside the repository for a
  `.git/` path), then `relink`s the graph. Package manifests and `.git/config`
  are watched (`watch::context_input_globs`, from sniff's
  `PACKAGE_MANIFEST_FILE_NAMES`) but never indexed (`is_context_input`).
  Rescan mode diffs `scan_manifests` fingerprints; config reload clears all.
  The workspace registration also watches `**/*.yaml|yml` and the discovery
  inputs (`.gitignore`, `.ignore`, `.dmls.toml`: re-list). Schema-location
  watches (`schema_watch_registration` over `watch::WatchedFolder`s from
  `OverlayState::trigger_watch_folders`: `<parent>` + `<folder>/*` for a
  present folder; nearest existing ancestor + `<rel>` and `<rel>/*` for one
  absent at preparation, its `absent_base` recorded then so syncing reads
  nothing and a folder appearing changes the set) are re-synced after every
  message when the client has `relativePatternSupport`; without it, or on a
  rejected registration, `schema_watch_fallback` makes every save mark the
  registries stale. A rejected workspace registration switches to
  `ServerRescan`.
- The overlay schema cache key includes
  `darkmatter::markdown::compose::file_resolution_context_identity(context)`
  (the same exhaustive encoding the compose graph identity uses: source,
  `cwd`, repository/package/area roots, home, launch `@` scope, sorted
  environment, magic and vault roots, tree root and origin) plus the
  resolution's `generation()`, so the same text under another context (a
  different `^` root or `SCHEMAS_DIR`) re-assembles and a rebuilt context
  re-validates file values. Never hand-roll a context hash in DMLS. The
  trigger registry enters the key as its install `version`, and the
  registry's envelopes and payload files are left out of the cache's
  re-hashed dependencies (the registry cache tracks them).
- `dmls/tests/l1/schema_roots_parity.rs` is the `md` vs DMLS parity gate for
  schema roots, applied triggers, bare-name `$schema`, and `$path`
  definition errors; DMLS shows missing required properties only with
  `[schema] strict = true`.
- `context_build_count()` is a process-wide work counter; L1 tests assert
  deltas (nextest runs one test per process). `work_counters::work_counts()`
  does the same for DMLS file reads, content hashes, Markdown document
  parses, schema-root enumerations, and trigger-registry scans; route every
  new DMLS read/hash through its `read`/`read_to_string`/`hash_bytes`
  helpers. Timing that decides behavior reads a `clock::Clock`
  (`ManualClock` in tests). `log_capture::CapturedLogs` captures formatted
  `tracing` lines: `scoped` sees only the calling thread, `global` sees every
  thread (once per process). A test that runs `run_server` on its own thread
  wraps it in `tracing::dispatcher::with_default(&logs.dispatch(), ..)`
  instead (see `router/logging_tests.rs`); `initialized` never reaches
  dispatch, since `lsp_server`'s handshake consumes it.
- Trigger registries (`overlay/registry.rs`) are cached per document path
  and served with no I/O until invalidated. Root contents (listing, text,
  xxHash, classification) are shared across documents and fed to the
  library through `triggers::scan_with_source`, so a registry always equals
  `triggers::scan`. Invalidation enters only through
  `OverlayState::invalidate_trigger_inputs` (watched events, any YAML
  `didSave`, including an open buffer) and
  `invalidate_all_trigger_registries` (rescan, config reload, manifest
  change). A test that edits a schema file on disk must send one of those,
  or the edit is invisible by design. `Dependencies::searched` holds the
  normalized `TriggerRegistry::searched_locations()`; an event at a
  candidate, or at a folder above one (`candidate.starts_with(event)`),
  affects the scope. A failed preparation carries the replaced scope's
  `searched` and `folders` (`Preparation::carried`) so the broken file's
  folder stays watched. A directory event for `schemas/shared` is already
  covered by the folder rule (its parent holds the payload), so a test of the
  folder-above-a-candidate rule needs a nested absent folder
  (`schemas/shared/deep/` under an existing `schemas/shared/`). Protocol tests:
  `tests/l1/searched_candidates.rs`.
- Skipped (invalid) triggers: a scan with findings is `Outcome::Ready` and
  installs its valid subset, even an empty one; there is no last-good
  retention for them, and `failed_scopes` never counts them (only
  `failed_triggers`). `TriggerRegistries::reconcile_warnings` runs after
  every install: it folds every scope's served findings (stale scopes
  included, by sorted scope path, keyed by `InvalidTriggerFinding::identity`)
  and diffs them with the published set, pushing a
  `TriggerDiagnosticTransition { path, finding }` and one `warn` log
  (`ignoring invalid schema trigger …`, `trigger=`, `code=`) only for a new
  or changed finding, and a clear plus `info` (`invalid schema trigger no
  longer reported`) when no scope reports it. Embedded findings are logged,
  never published. Closed files get `diagnostics::frontmatter::invalid_trigger_diagnostic`
  (warning, line of a YAML error, related information = `cause_path`); an
  open trigger whose buffer envelope parses gets
  `SuggestionState::TriggerWarning` from `finding_for`, while a malformed
  buffer keeps the strict `TriggerError` (`dm.schema.prepare`) and wins;
  `didClose` republishes the warning (`closed_file_diagnostics`). Triggered
  schemas only produce document diagnostics in `[schema] strict = true`, so
  a session test of trigger properties sets it. Tests:
  `tests/l1/invalid_trigger_warnings.rs`, the registry unit tests.
- Whole-scope failures (root I/O, a non-UTF-8 root file → `RootFailed`, never
  last-good; a case-fold collision → `TriggerFailed`, last-good) log through
  one record per (workspace boundary, document folder): `warn` when new,
  `debug` on a repeat, `info` on recovery; a cold failure logs `trigger
  registry unavailable`. They publish no trigger-file diagnostic. A
  `TriggerFailed` case can't be built on a case-insensitive file system; the
  unit test substitutes `Prepared::result`.
- Request timing (`request_timing`): the router starts a `RequestTimings` on
  `ServerState` at each dequeue and emits exactly one `CompletionEvent` per
  request or notification (`slow request`/`slow notification` at `warn` from
  500 ms, else `request complete`/`notification complete` at `debug`). Wrap
  new measured work in `timings.time(Stage::…, ..)`, call
  `record_wait(Wait::…)` when a request blocks on the file list or the
  index build (`Wait::Index`), and call `OverlayState::for_document_timed` from dispatch paths.
  A request parked on a registry job uses `begin_registry_wait`; `proceed`
  and `answer_waiter` call `end_registry_wait`, so `trigger_registry_wait_ms`
  sums every parked interval (requeue, cancel, shutdown included).
- Slow-validation log (`validation_timing`): `refresh_diagnostics` opens
  `validation_timing::collect`; schema work inside it runs in
  `validation_timing::part` (each a library `ValidationTimings::record`,
  absorbed), so other providers never count. Parts today: the registry
  freshness check and lookup (`ValidationStage::Discover.time`),
  `schema::assemble` (DMLS extension merge and baseline setup timed as
  `Resolve`), and the diagnostics `validate_with_options`. `schema_for`
  calls `note_assembly`; a ready bundle served from the overlay cache is
  reported as compile `hit`. One event per refresh that reached
  `schema_for` (`slow schema validation` at `warn` when total >=
  `schema.slow_validation_ms`, default 200, else `schema validation
  complete` at `debug`). `schema_cache_key` excludes the threshold. Tests:
  `tests/l1/validation_timing_log.rs`.
- Payload memo (`triggers/assemble.rs::file_payload_memo`, process-wide,
  32-entry true LRU, successes only): keyed by what resolution reads, never
  the requester's folder — trigger path, payload value, schema roots, the
  requester's tree (`base_dir` + origin), `for_cwd(trigger folder)`, and
  `for_trusted_external_cwd(root)` per root; reuse also needs an unchanged
  fingerprint: dependency bytes, listings of roots, dependency folders, and
  every `ResolvedPayload::searched` candidate's folder, plus each candidate's
  kind (absent/file/dir/other). Only `NotFound`/`NotADirectory` are expected
  absence; any other read error yields no fingerprint (no hit, nothing
  stored). Tests inject that failure with the `#[cfg(test)]`
  `assemble::fail_observing(path)` guard, never permission bits. A test that
  a new nearer candidate misses must put it where no root or dependency
  folder listing changes (e.g. `shared/deep/` under an existing `shared/`),
  or it passes without the searched folding. Keying on the requester's
  context again makes every package folder miss (the original incident). Its
  tests are `payload_memo_tests.rs` (rely on nextest's process per test).
- Pass line: `run_pass` starts a `PayloadMemoTally` (thread-scoped, from
  `darkmatter::markdown::schemas::triggers`) for the pass, and `run_document`
  starts its own so an interrupting document job is excluded;
  `run_workspace` logs `trigger registries prepared` at `debug` with
  `prepare_workspace_probe`'s `ProbeOutcome` counts. Startup phases do not
  log it. Tests: `tests/l1/registry_observability.rs`
  (`LspFixture::start_with_registry_gate_and_clock` for a `ManualClock`).
  `RunOptions::with_clock` injects the clock. Never log text, frontmatter
  values, or payloads; `lsp_server`'s own debug trace already prints full
  payloads, so a leak test must scope to `dmls`/`darkmatter` records.
- Fixture traps: sniff recognizes a package only under a workspace manifest,
  and counts a declared member whose directory exists even without its own
  manifest. Inline `$schema` `file` is lazy (syntax only); use `file(eager)`
  to exercise resolution in validation.

## Features

DMLS projects Darkmatter's typed surfaces into LSP:

- Frontmatter and schema diagnostics with source spans.
- Schema-aware completion for keys, values, suggestions, literal-discriminated
  union arms, and named types (local names bare, `name@this` only for a
  built-in collision, imports with their target).
- Named-type references (`overlay/named_types.rs`,
  `providers/frontmatter/named_references.rs`, `diagnostics/named_types.rs`):
  hover and go-to-definition on bare, `@this`, and cross-file spellings;
  `UnknownNamedType`/`ConstraintConflict`/`NonDescendingCycle`/`ImportCycle`
  publish as `dm.schema.invalid_type_definition` on the reference's
  `ImportName` span with conflict origins as `relatedInformation` (a
  non-descending cycle is ranged on the first cycle reference this document
  wrote; the others are related information). Standalone schema files
  resolve their references through `resolve::expand_named_types`. Imported
  files are dependency edges, and a failed assembly records the files its
  error names so a fix clears it; local references add no edge.
- Type keywords come from `schema_type_descriptors()` plus each descriptor's
  `aliases`; `alphanumeric` options come from its `type_options`, offered only
  while `SchemaCursorRole::Constraint.leading_group` is `Some` (no `;` yet).
  Definition hover adds `schema_type_descriptor` text and, for
  `alphanumeric`, `PropertyAtom::alphanumeric_options()` with the
  `AsciiCharSet` `Display` rendering.
- Expression completion, hover, and parse diagnostics inside
  expression-typed frontmatter.
- Hover/documentation from typed descriptor catalogs.
- Hover shows the local narrowed type ("Type here") at body sites and
  inside expressions: `providers/dsl.rs::with_site_scope` builds
  `BodyGuards::scope_at` over `DeclaredTypes`, and `overlay/expressions.rs`
  `hover_markdown*` take a `site: &TypeScope` and refine it with
  `branch_scope`. Lifecycle/task `when` guards do not narrow the hover of
  frontmatter actions (call diagnostics in lifecycle stacks are narrowed;
  task bodies are not); see `dmls/docs/hover.md`.
- Lifecycle stacks: `overlay/event_views.rs::LifecycleHost::for_document`
  selects the document's host: the first host catalog whose
  `<stem>.lifecycle.json` loads
  (`schemas::catalog::host_lifecycle_profiles` →
  `lifecycle::host_profile::HostLifecycleProfile`; Claudine's
  `claudine/schemas/globals.lifecycle.json`), else `md_host`. Its registry's
  `resolve_key` (never a hand list) maps a root key to a `StackEvent`
  (event plus registry) and gives every `{{ … }}` span, `when` condition, and
  expression-typed value inside that stack the event's registry
  `BindingView`; `diagnostics/frontmatter.rs::GuardedStackCalls` parses with
  the host's parse options and calls the library's one walk,
  `lifecycle_call_checks`, with the host's registry, for stack spans and
  expression-typed stack values alike. Without a usable artifact the stacks
  are md's and `dm.lifecycle.incomplete_analysis` names each catalog's
  reason. DMLS never depends on a host crate; `md_host::` appears only in
  `event_views.rs` (the fallback and its tests). Completion offers only the
  globals the event may read; `err` where the event has none is
  `dm.expression.unavailable_global` (the view's reason, never `fm.err`).
  Host-catalog globals (`seq`, `loop`, `group`) are `ExecutionDependent` in
  every view (`validation_view`), and a host catalog that fails to load is the
  warning `dm.schema.host_catalog`. A schema problem's diagnostic carries the
  library's message without the declared description; guidance such as
  "use `start`" on Claudine's `composed` shows in hover only.
- Matching trigger payloads feed completion and hover as well as
  diagnostics. The editor's property shape is
  `EffectiveSchema::property_shape` (base → extensions → matching trigger
  payloads in registry order → authored `$schema`, whole top-level property
  replacement, as validation merges); `providers/frontmatter.rs::known_shape`
  and `overlay/schema.rs::effective_shape` both call it. Trigger payload
  shapes are retained at registry-scan time (`ResolvedPayload::shape`);
  `EffectiveSchema::simplified` stays the authored `$schema` alone, which is
  what "authored parameters" consumers (expression property-name completion,
  Claudine's prompt parameters) read. DMLS reads triggers from disk only; it
  registers no embedded bundle.
- Workspace dependency tracking for referenced schemas and documents.
- Recursive named types: every DMLS walk of a `PropertyDef` opens a
  `TypeExpr::Ref` through `providers::frontmatter::definitions(ctx)`, the
  bundle's `DefinitionLookup` (`overlay/schema.rs`: the effective schema's
  tables, then `SchemaBundle::extension_definitions`), with
  `query::resolved_alternatives` / `alternatives_for_def_in` /
  `alternative_accepts_in`; never `flattened_alternatives` or a table-less
  `alternative_accepts` on a shape that can hold a `Ref`. One definition per
  path step; `work_counters::record_definition_lookups`/`definition_lookups`
  record lookups for laziness tests (`tests/l1/recursive_schema_editor.rs`).
  `NonDescendingCycle` failures record every cycle file as a dependency
  (`overlay/mod.rs::error_dependencies`).

Completion and validation must call the same schema-arm selection and parser
authorities. A completion-only reconstruction is drift.

Frontmatter `$( … )` suffixes follow the same rule: completion, hover, and the
`dm.shell.invalid_suffix` diagnostic (`providers/dsl.rs`) read the library's
`FRONTMATTER_SHELL_SUFFIXES`, `describe_suffix`, and
`parse_frontmatter_shell_suffixes`, the grammar `md compose` parses with.
`frontmatter_shell_values` strips a quoted scalar's quotes before parsing,
because the YAML span includes them.

## Undeclared properties and unknown functions

`dm.expression.undeclared_property` is an advisory **Warning** on body
`{{ … }}` and on Expression-typed frontmatter values, with the same code,
severity, and wording `md compose` uses: the property is valid, of unknown
type, and `null` unless supplied at runtime. `dm.expression.unknown_function`
is an **Error** at both sites, ranged on the name and reported in every
branch, even without frontmatter. It comes from the library's passive
`expression::validate_expression` against `BindingView::baseline()`
(`overlay::expressions::unknown_function_calls`), the check
`validate_prepared` runs for preparation; never add a DMLS-side function list.

- **Expression-typed values.** `providers::frontmatter::expression_values`
  decodes each value with the library's `decode_scalar_node`, the decoder
  composition uses to anchor frontmatter failures, so block (`|`, `>`) and
  multi-line values are parsed as composition evaluates them and warnings land
  on exact identifier ranges. A value is kept only when the decoded text
  equals the parser's `FmEntry::scalar`. Never add a DMLS-side reconstruction.
  `rlsp-yaml-parser` starts a scalar's span after its tag and anchor, so
  tagged and anchored values need nothing extra. An alias value is checked
  against `FmEntry::alias_target` (the last scalar anchor before it) and
  diagnosed inside the anchor's scalar, which DMLS decodes where its own YAML
  tree found it: `FmEntry::alias_target_start` into the library's
  `decode_alias_definition`. Every alias of one definition shares its
  `alias_target` (`Arc<str>`), and `expression_values` decodes and checks each
  distinct start once per call, sharing the `Arc<DecodedScalar>`; a copy or
  decode per alias is O(aliases × length). Never resolve an alias by calling
  `decode_scalar_node` on the `*name` token: that searches the text before
  the alias, once per alias, and made a publication quadratic. The tree also
  sees through an anchor name repeated in a comment or string. The start is
  withheld for an anchor redefined before the alias, and the decoded text may
  disagree; then the `alias_target` text is still analyzed and every
  diagnostic lands on the alias token (`AuthoredExpression::AliasToken`):
  same codes and severities, identical diagnostics collapsed, and no
  quick-fix, since no replacement range exists there. Never skip an
  Expression-typed alias.
  `diagnostics()` computes the expression-value set once per publication and
  hands it to both `schema_problem_diagnostics` and `expression_diagnostics`.
  `distinct_expression_aliases_publish_without_searching_the_document` pins
  both through work counters (the library's `alias_search_work`, behind its
  `work-counters` feature, which dmls enables as a dev-dependency only).
  `many_aliases_of_one_long_expression_share_its_text_and_one_decode` pins the
  shared target (`Arc::ptr_eq`) and one decode (`ALIAS_TARGET_DECODES`).
  `expression_diagnostics` skips a value that is a whole literal token
  (`{{!data:v1:…}}`): it is data, and its encoded bytes cannot anchor a parse
  error (`a_literal_token_is_not_parsed_as_an_expression`).
  `expression_diagnostics` parses and reports each authored expression once,
  so an anchored value and its aliases never double-report; a malformed one
  whose property's union accepts it keeps its parse error for a later alias
  whose property rejects it (`EXPRESSION_DIAGNOSTIC_PARSES`,
  `a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias`).
  Hover/completion
  ignore the alias token.
- **Walk.** Diagnostics use the library's
  `expression::static_variable_reads`. It yields every `Variable` with its own
  span, classified by the same `AbsenceScope` rules the runtime uses (fallback
  primary, ternary condition plus its guarded root, direct argument to a
  `| null` parameter of a call that is not another call's argument). Never re-derive those rules in DMLS. The only deliberate
  difference is that the static walk visits both ternary branches.
- **`root_identifier` keeps its one-root contract.** Hover, definition, and
  graph indexing depend on it. Only the two diagnostic providers use the walk.
- **One classification.** `overlay::expressions::is_undeclared_property`
  classifies the first dotted segment through Darkmatter's binding model:
  `BindingView::baseline().names_document_property` (reserved namespaces from
  `reserved_root_descriptors()`, plus the `null` literal). DMLS supplies no
  host descriptors, so Claudine's `err`/`timing`/`group` are undeclared
  properties here, beneath lifecycle keys too, until R7c distributes host
  descriptors. Root names DMLS needs (`ctx` hover/completion, the `fm[...]`
  quick-fix) come from `context_root()`/`document_root()`, which read the same
  catalog. `undeclared_property.rs` fails on any root-name string literal, a
  host-global list, or an evaluation-session type in production `src/`.
  `providers::dsl::KnownRoots` adds frontmatter keys and schema properties. It
  computes `known_shape` once per diagnostics pass, and it is `None` for a
  frontmatter-less document, whose names are never reported undeclared.
- **Dash-separated keys.** A subtraction of only variables (`foo--bar`, `a- b`)
  that contains an undeclared operand, and whose whitespace-free source is a
  top-level key, gets one key-level finding. A literal operand
  (`iteration - 1`) stays arithmetic. A fix is attached only when the edited
  expression reparses with the replacement as one reference to the key: the
  bare key first, then `fm['key']` quoted to avoid the value's YAML quote.
  The fix rides in `Diagnostic.data` as `KeyReferenceFix`. The code action
  reads `data`, or recomputes the producing provider's diagnostics when a
  client drops `data`, and never parses the message.

## Call diagnostics

`dm.expression.incompatible_call` (**Error**) and
`dm.expression.uncertain_call` (**Warning**) show Darkmatter's design-time
argument categories. Darkmatter's library owns every rule, code, severity,
and message: DMLS calls `expression::expression_call_diagnostics(expr,
&TypeScope, strict)` (or `CallCheck::diagnostics` for lifecycle surfaces) and
only ranges each `ExpressionDiagnostic` (`diagnostics/call_check.rs::lsp_diagnostic`,
expected/actual types in `data`). Never add a conversion, union, category, or
severity rule in DMLS; fix the
library's `call_check.rs` instead, so composition's precheck and the editor
keep agreeing. User docs:
[`dmls/docs/diagnostics.md#call-diagnostics`](../../../darkmatter/dmls/docs/diagnostics.md#call-diagnostics).

- **Types.** Every site's `TypeScope` is built over
  `DeclaredTypes::new(effective.simplified)` (declaration-only), never the
  static-frontmatter layer (which hover's `with_site_scope` uses): a set
  `maybe: 3` does not remove `null`. Enclosing `::block` guards narrow
  through `BodyGuards::scope_at`; ternary and `&&` branches narrow inside the
  library walk.
- **Sites.** `providers/dsl.rs` checks body `{{ … }}` and every directive
  `when=` (false `::block` bodies included); `diagnostics/frontmatter.rs`
  checks expression-typed values, lifecycle stacks and guards, and plain
  frontmatter `{{ … }}` values. Fenced code and `{{{ … }}}` are skipped.
  Composition finds its own expressions (`compose/call_precheck.rs`); the two
  walks must cover the same surfaces.
- **Ranges.** The argument's span, or the call's for arity, ambiguity, or no
  matching overload. The message is the library's `CallFinding.message`.
- **Setting.** `ExpressionsConfig { strict }` in `config/mod.rs`:
  `[expressions] strict` in `.dmls.toml`, `dmls.expressions.strict` from
  `didChangeConfiguration` (DMLS never pulls `workspace/configuration`).
  Same serde validation and layering as `schema.strict`: editor over file
  over default; an invalid merge falls back to the file layer, not the
  previous effective config; a whole-settings `null` clears the overlay. It
  is the only call setting; no function-specific setting exists.
- **Tests.** `tests/l1/expression_call_diagnostics.rs` (worked-example rows
  under both settings, from the file and the editor, reloaded at runtime;
  ranges; verdicts), `tests/l1/call_check_agreement.rs` (a shared corpus
  composed by `md` and diagnosed by DMLS must reject the same documents),
  `config::tests::expressions_strict::*` (the input matrix), and the
  in-source `diagnostics::call_check::tests::diagnostics_never_enter_a_handler`
  (runs on the test thread because test function registration is
  thread-local).

## Rollout chronology

The DMLS stream was delivered in dependency order:

1. Span-aware validation and style diagnostics in the library.
2. Passive standalone schema classification.
3. Server transport and document/workspace state.
4. Schema selection, imports, completion, hover, and diagnostics.
5. Suggestion, literal-discriminant, expression, and meta-schema support.
6. Zed packaging and end-to-end editor integration.

This chronology is historical routing, not a second architecture. Current code
and public types are authoritative if an older phase note disagrees.

## Verification

Use package-scoped DMLS and Zed gates. Protocol behavior needs integration tests
that open real documents through the server's normal request path. Schema or
expression changes also require passive shipped-artifact corpus coverage in the
Darkmatter library, so editor tests are not the only proof of grammar behavior.

- `just test` runs the cross-platform L1 extension manifest and crate-shape
  contract alongside the DMLS protocol suite.
- `tests/l1/repository_contexts.rs` holds the per-repository context
  acceptance tests (one build, every feature, invalidation, failure
  diagnostic, untitled buffers). `LspFixture::start_with_snapshot` supplies
  a test's own `HOME`/environment/`@` roots; the default fixture snapshot uses
  a `HOME` inside the workspace.
- `just check-zed` compiles `zed-dmls` for Zed's `wasm32-wasip2` target and
  requires that target to have been provisioned explicitly.
- `just zed-verify` adds Zed's pinned official packager and artifact contract;
  CI runs this as an Ubuntu companion gate, not as L2.
- `just install-dmls` installs the binary and, when Zed's data directory
  exists, stages the extension (with the bundled `extension.wasm`) to a stable
  per-user directory and creates or repairs Zed's `installed/dmls` link.
- `just zed-doctor` diagnoses the host's native binary, stable dev-extension
  registration, manifest, and recent Zed log evidence without launching Zed.
