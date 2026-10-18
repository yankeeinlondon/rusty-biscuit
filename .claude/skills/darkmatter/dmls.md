# DMLS

DMLS is Darkmatter's language server. Use this reference for protocol,
workspace, completion, hover, and diagnostic work.

## Contents

- [Architecture](#architecture)
- [Passive-analysis contract](#passive-analysis-contract)
- [File-resolution contexts](#file-resolution-contexts)
- [Features](#features)
- [Undeclared properties and unknown functions](#undeclared-properties-and-unknown-functions)
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
  list, on-demand parsing, full index, log lines) and
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
- Startup order (one background thread, `router::spawn_startup`; the
  overlay lock is not held across preparation I/O): prepare
  each workspace scope's trigger registry
  (`OverlayState::prepare_workspace_registries` over `workspace_probes`: a
  probe document `.dmls-workspace` in each folder, catalog package and
  area inside it, and monorepo first-directory fallback area with a
  `schemas` folder, `fallback_areas`) → the file-list walk (its
  `filter_entry` also records each directory holding `.git` as
  `DiscoveryReport::repositories`) → the same preparation for those
  repositories (`OverlayState::set_workspace_repositories`) →
  `Internal::RegistryReady` (logs `schema registry ready`, or
  `… initialized with failures` when `failed_scopes > 0`; one line, after
  the walk, `elapsed_ms` excludes it) → `Internal::FileList` (logs
  `workspace files listed markdown yaml duplicates elapsed_ms`). A re-list
  (`ServerState::relist`: rescan, config or ignore-file change) logs
  `workspace rescan complete` with the same fields, and queues
  `RegistryJob::Workspace` when its repository set changed (as does a
  watched `.git` change, `ServerState::note_repository_change`); a built full index logs
  `full index built markdown headings links trigger elapsed_ms` (elapsed from
  the first waiter, including any file-list wait). Every workspace log line
  carries `workspace=` (`workspace::workspace_identifier`: the client's
  folder URIs, comma-joined, one line per server). Log string fields with `%`
  so they print unquoted.
- The full index (`full_index.rs`, router): references, rename,
  `willRenameFiles`, and `workspace/symbol` (`Trigger::for_method`; never
  `prepareRename`) are parked as `Waiter`s with their own `RequestTimings`
  while `WorkspaceIndex::is_full()` is false. `FullIndexState`
  (`NotStarted`/`Building(Build)`/`Ready`/`Failed`) holds at most one
  `Build`, which runs rounds on a thread
  (`parse_full_index_round`, no index lock); the dispatch thread reconciles
  each with `finish_full_build`, which drops parses of paths `touch`ed
  during the round and starts another round until nothing is unparsed. Any
  current unreadable file fails the build (`RequestFailed`, file named);
  cancelling the last waiter drops the build (its `Drop` sets the cancel
  flag); shutdown answers waiters `ServerCancelled`. Diagnostics refresh
  *before* waiters are answered, so a response stays a barrier. Progress
  token `dmls/full-index`. A new `WorkspaceIndex` mutator that changes a
  path's disk, buffer, or list state must call `touch`.
- Trigger registries (`registry_jobs.rs`, router): the dispatch thread only
  looks registries up (`TriggerRegistries::lookup`/`is_fresh`,
  `OverlayState::registry_demand`); it never prepares one. A document whose
  registry is not fresh gets one `RegistryJob::Document` on the registry
  worker (`spawn_registry_worker`); schema-dependent requests
  (`SCHEMA_DEPENDENT`) park as `Waiter`s on it (`Router::proceed` checks the
  full index, then the registry), `refresh_diagnostics` holds the
  document's publish until it installs, and refreshes queue
  `RegistryJob::Workspace` (`ServerState::prepare_workspace_registries`).
  Preparation is `begin` (lock) → `Preparation::run` (no lock) → `install`
  (lock; an older `seq` is dropped, a change since `begin` leaves it
  stale). `OverlayState::for_document` (no server) prepares inline; the
  router uses `for_document_timed`, lookup only. Cancelling a registry
  waiter keeps the job; shutdown answers waiters `ServerCancelled`.
- L1 sessions: `LspFixture::initialize` waits for the list
  (`StartupEvent::FileListInstalled`, which follows `RegistryReady`),
  `initialize_unlisted` for neither; `start_with_file_list_gate` holds the walk and
  `start_with_full_index_gate` holds each build round after its reads
  (`wait_for_event(FullIndexRoundRead)`, `release_full_index_round`,
  `open_full_index_gate`); `start_with_registry_gate` holds each registry
  job after startup (`wait_for_event(RegistryPreparing)`,
  `release_registry_job`, `open_registry_gate`), and
  `wait_for_registry_prepared`/`next_registry_prepared` return the thread a
  job ran on. `flush_server` is a `workspace/symbol` round trip that also
  waits for every queued registry job (`RegistryJobsOutstanding` events) and
  buffers what they published; `round_trip` does not wait for jobs. Since
  `workspace/symbol` builds the full index, a parse-counting test must not
  use either as a barrier, and its response lists a `File` symbol per
  document beside each heading. `work_counters::registry_scan_threads`
  names every thread a registry scan ran on.
  Startup's registry preparation publishes a failing trigger file's
  (unversioned) diagnostic before any open; wait for the versioned one.
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
  and `is_full`; its folder joins the schema-location watches; tests in
  `tests/l1/referenced_targets.rs`). Tests use `context::test_support`
  (`abs`, `workspace_contexts`, `resolution_for`) because a context rejects a
  rootless `/w` path on Windows; integration tests use `FixedContext` or a
  real `RepositoryContexts`; empty graphs use `NoContexts`
  (`ContextFailure::NotProvided`).
- Invalidation: every watched path calls `invalidate_contexts`, which drops
  ancestor-keyed entries (plus folder keys inside the repository for a
  `.git/` path), then `relink`s the graph. Package manifests and `.git/config`
  are watched (`watch::context_input_globs`, from sniff's
  `PACKAGE_MANIFEST_FILE_NAMES`) but never indexed (`is_context_input`).
  Rescan mode diffs `scan_manifests` fingerprints; config reload clears all.
  The workspace registration also watches `**/*.yaml|yml` and the discovery
  inputs (`.gitignore`, `.ignore`, `.dmls.toml`: re-list). Schema-location
  watches (`schema_watch_registration`, one `<parent>` + `<folder>/*` per
  folder in `OverlayState::trigger_watch_folders`) are re-synced after every
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
  or the edit is invisible by design. Failure logging and trigger-file
  diagnostics share one transition record per (workspace boundary,
  document folder): `warn` when new, `debug` on a repeat, `info` on
  recovery; a cold failure logs `trigger registry unavailable`. Root I/O
  errors (including a non-UTF-8 root file) never serve last-good data.
- Request timing (`request_timing`): the router starts a `RequestTimings` on
  `ServerState` at each dequeue and emits exactly one `CompletionEvent` per
  request or notification (`slow request`/`slow notification` at `warn` from
  500 ms, else `request complete`/`notification complete` at `debug`). Wrap
  new measured work in `timings.time(Stage::…, ..)`, call
  `record_wait(Wait::…)` when a request blocks on the file list or full
  index, and call `OverlayState::for_document_timed` from dispatch paths.
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
  `UnknownNamedType`/`ConstraintConflict`/`ImportCycle` publish as
  `dm.schema.invalid_type_definition` on the reference's `ImportName` span
  with conflict origins as `relatedInformation`. Standalone schema files
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
  primary, ternary condition plus its guarded root, direct `is_null`/`is_empty`
  argument). Never re-derive those rules in DMLS. The only deliberate
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
