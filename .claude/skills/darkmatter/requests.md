# Composition Requests and File-Resolution Contexts

Detail behind the "Composition authority" section of [SKILL.md](SKILL.md).

Every entry point that resolves file references takes a `ComposeRequest`:
`compose_with`, `compose_preflight*`, the `collect_*` functions,
`transclusions_with_options`, `ReferenceGraphOptions::with_compose`,
`execute_directive`, and `execute_resolved_shell_values`. A request is
`ComposeRequest::prepare(options, &RequestSnapshot)` (which builds the context
through `build_resolution_context`) or `ComposeRequest::with_context(options,
built_or_derived_context)`. Preparation fixes the repository observation and
reuses it for the builder (one discovery per request), re-anchors a
`ComposeOptions::new()` context on the request directory, and makes `ctx.env`
the context's environment. Magic roots enter only through
`RequestSnapshot::with_magic_root*` (for `md`, the top-level repeatable
`--magic-root <DIR>`, applied in `main` by `request::with_magic_roots`); `ComposeOptions` has no magic paths and no
public context setter. Only binaries call `RequestSnapshot::from_process()`. DMLS builds one
context per repository (`dmls/src/context.rs`, see
[dmls.md](dmls.md#file-resolution-contexts)).
A request's environment is the environment of the one document it composes
(and everything that document transcludes). A host that runs several documents
in one process gives each its own request from its own captured snapshot
(Claudine: one per sequence step, group leg, and `proxy` target). Shell
children inherit the composing process unless the request sets
`ComposeOptions::with_shell_child_environment(ShellChildEnvironment)`
(`shell_expansion::types`): the supplied raw map replaces the inherited
environment, the executable is looked up on its `PATH` (`which::which_in`), and
`PWD` is the command's directory. It reaches `run_action` through
`execute_directive_outcome` (`::shell`, `::shell-block`, frontmatter `$( … )`),
is part of the options' graph identity, and is left out of the compose-cache
fingerprint, like `shell_working_directory`. See
`darkmatter/docs/topics/compose-requests.md` ("One environment per document").
A file source (root or child) outside the request's tree is admitted
trusted-external when only that derivation validates (`source_derivation_for`,
applied to the root in `ComposeRequest::assemble`). The schema stage validates
in the source's derived context (`source_file_resolution_context`), and
`resolve_ctx::document_file_context` recognizes that context by canonical
directory, not spelling (`/var` vs `/private/var`); re-deriving it with
`for_cwd` would drop a `~` tree root.
Unit tests use `crate::markdown::compose::test_request(options)` /
`test_request_in(options, context)`; `lib/tests/l1` uses
`crate::request_support::{request, request_at, context_at, cwd_context}`.

**The context is required, never `Option`.** `ComposeOptions` is only the
settings builder and holds no context. The pipeline runs on `ComposeRequest`,
which `Deref`s to `ComposeOptions`: stages take `&ComposeRequest`, context
views live on the request (`transclusion_options`,
`source_file_resolution_context`, `*_resolution_context`,
`with_accepted_source_file`, `extended_for`), and in-pipeline builder chains
use `request.derive(|o| …)`. `request.resolution_context()` is the
file-resolution context; `request.context()` (through `Deref`) is the
captured `ctx.*` `ComposeContext`, so never name a new request method
`context`. Internal inline passes use `Markdown::compose_with_options(request)`.
`DarkmatterSchemas::new(ctx)`, `CleanSchemaConfig::new(ctx)`,
`ResolutionContext::new(ctx)`, `ReferenceGraphOptions::with_compose(&request)`,
`FileTree::new(path, &request)`, `evaluate_condition_against(expr, data, &ctx)`,
`detect_schema(.., &ctx)`, `resolve_schema*(.., &ctx)`, and
`triggers::scan(&ctx)` all take one; none has a context-free form.
Validators are either context-bound (`ValidatorCache::validator_for(schema,
base, &ctx)`) or structural (`structural_validator_for`, `build_structural_validator`) for
callers with no request (coercion probes, examples, lint): an absolute path is
judged as resolved (it must exist; `match()` judges its full path through
`GlobReference::matches_without_context`, bare and absolute patterns only), and
every value that needs a context is judged by syntax only. Root-union coercion
relies on the absolute-path half to pick an arm by glob, so a `./`/`&`/`^`/`@`
pattern cannot steer coercion.

`match()` patterns are `GlobReference`s (`FileMatchGlobs` wraps one with the
file-name view); the grammar rejects a non-glob-reference pattern at definition
time (`file_match::definition_error`). A context-bound validator judges each
value from its own `cwd`: the document's folder for frontmatter, the caller's
origin for a caller-supplied value (`validate::CallerOrigins`: a top-level
property from the schema stage's `caller_input_records`, found through the
keyword's schema location; a value below a top-level property by its projected
absolute text, `with_supplied_values`, since a keyword sees no instance path;
`run_with_registry` then judges an authored copy of that exact absolute path
again, `without_supplied_values`). Never judge `match()` from the process directory or
from "any containing root". `find_files()` and `::file-links <glob>` call
`list_files` in the document's context and report skipped out-of-tree file
symlinks as `dm.glob.skipped_symlink` (`compose/glob_listing.rs`).

## Data runtime and read set

`compose::DataRuntime` (in `compose/expression/data/`) is the command-scoped memo and
read-set sink, shared the same way as the remote-fetch runtime: an
`Option<DataRuntime>` on `ComposeOptions` (`with_shared_data_runtime()`,
`with_data_runtime(rt)`), cloned into every `ResolutionContext` by both
`expression_resolution_context_in` builders; a root pipeline with none attached
makes its own, so separate library compositions never share. A standalone
`ResolutionContext::new` is isolated. `md compose`, `md graph`, and
`md validate refs` attach one before their first `prepare_request`, so the
reference check, pre-flight, lifecycle, and composition share it. Hosts read
the attached one with `ComposeOptions::data_runtime_handle()`; the
crate-internal `data_runtime()` returns it or a fresh unshared one, like
`remote_fetch_runtime()`. `DataRuntime` exposes `new`, `read_set`, `stats`,
`same_instance`, the observation log (`read_mark`, `observations_since`),
`memo_generation`, and `record_entry` (cache replay into the read set).

- **Observation log** (`runtime.rs::Observation`, `ObservationKind::{File,
  Url, Exists, MissingProbe, Listing, Glob}`): every input-reading function
  observes what it read, hashed at read with BLAKE3 (`RawSource::hash`
  keeps it on the raw memo entry, so a memo hit observes again; a contract's
  observations are kept per memo key). `observe` also adds the entry to the
  read set, except a `Glob` (a `find_files` pattern is not a path). A failed
  read observes `hash: None`. `capture_observations(f)` is a thread-local
  frame: the slot gate's post-call check and the slot evaluator use it so a
  concurrent document read on another thread never mixes in.
- **Slot read gate** (`expression/read_gate.rs`): `ResolutionContext::slot_gate`
  is set only by the schema-slot evaluator. `authorize_slot_read` runs at the
  content/fetch sites (`load_data` before `record` and the memo,
  `load_markdown`, URL `file_exists`) and the listing sites (`find_files` per
  pattern root, `find_*_index` per existing parent folder) before any I/O;
  `ResolutionContext::gate_schema_reads` wraps a schema resolution
  (`DataRuntime::prepare_contract`, `validate_schema`'s `validate`) in
  `simplified_schema::resolve::buffers::guarded`, so each schema file is
  judged before `buffers::read` opens it (a refused contract is not
  memoized). `functions::call_entry` then checks every
  observation a `reads`-registered function made before returning
  (`authorize_slot_observations`, a `Glob` judged by its pattern's roots; an
  unnameable input is refused). Frontmatter interpolation always excludes
  `$schema` (`with_schema_excluded`): only the slot resolver evaluates slots.
  `ExpressionError::SlotReadRefused` is never recovered (`try_frontmatter`
  re-raises it). With no gate nothing changes.

- **Memo shape** (`runtime.rs`): contracts are prepared once per document
  context and written text. Three layers share one generation: a raw source
  (`RawSource`: decoded text or the first invalid UTF-8 offset, size, BLAKE3
  hash) keyed by identity alone (canonical path or normalized URL); a parsed
  source keyed by `SourceKey { identity, format, delimiter }` (effective
  values, so defaults and aliases share); a bound result keyed by that plus
  the contract key (the unsorted `to_json_schema_with_definitions`
  serialization, the bare declaration, and the sorted import identities) and
  the effective header. So two delimiters read once and parse twice, and two
  contracts parse once and bind twice. `reads` counts raw loads only;
  `source_hits` counts parse-layer hits. Each layer is single-flighted
  against the generation of the entry below it (`single_flight` passes a
  stale generation on), so an in-flight read never fills the new maps after
  `invalidate_memo`. Entries are
  `Arc<OnceLock<Result<…, Arc<DataError>>>>`, filled without holding the map
  lock; a memoized error gets the current call site only at projection. The
  memo holds in every `CacheAccessMode`. `RemoteNotEnabled` (the reference
  check has no remote permission) is never stored.
- Failure identity survives the memo because the memo stores the typed
  `Arc<DataError>`. A source miss is `DataError::FileReference` (flattened to
  `ExpressionError::FileReference`); a contract import that cannot be read is
  `DataError::ContractImport` (kept inside `ExpressionError::Data`, worded and
  projected like a file reference, never a deferral); a data failure
  re-raised by `as_markdown` is `ExpressionError::NestedData`. Read them only
  through `ExpressionError::data_failure()` and `file_reference_diagnostic()`;
  never rebuild an import as `ExpressionError::FileReference`, which would
  make it recoverable (`provenance_tests.rs` beside `data/runtime.rs`).
  A contract import is not the data source, so `ContractImport` is never
  recovered by `try`; `nested.rs` marks a data failure from an `as_markdown`
  child as `NestedData` so an enclosing `try` never takes it for its own read
  (always fatal).
- `try` never writes its fallback into a memo layer or the read set: the memo
  keeps the source's result or failure under its usual keys (a later
  unguarded call fails from it; another `try` can pick a different fallback),
  an executed fallback's own reads are recorded, and an unexecuted one adds
  none, so a guarded read set equals the unguarded one. A deferral
  (`DataError::is_deferral`) is never memoized
  ([Recovering from read failures](../../../darkmatter/docs/topics/darkmatter-expressions.md#recovering-from-read-failures)).

- Reads are recorded at the operation boundary, never in `resolve_arg`:
  `functions/fs.rs::load_markdown` (the five Markdown readers), `file_exists_fn`,
  `load_data` (probed candidates on a miss, `record_probed_candidates`), the
  path functions (`observe_reference_probes`: candidates up to the first that
  exists), `has_skill` (each probed folder), `find_*_index` (the listed
  folder), and the schema files a contract (`DataRuntime::prepare_contract`,
  inside the memo's prepare closure) or `validate_schema_fn` read. Those two
  wrap the preparation in `simplified_schema::resolve::observed::collect` (and
  `buffers::recording` for the texts to hash) and record `Observed::read` on
  success and failure alike, plus every `Observed::searched` location not read
  (a missing import is a dependency); never reconstruct reads from a returned
  value or an error. Embedded schemas read nothing. Record after resolution,
  before reading, so a failed read stays in the set. `load_data` and
  `load_markdown` read bytes through `simplified_schema::resolve::buffers::read`
  (an editor buffer inside a `buffers::with` scope, else the file).
- `invalidate_memo()` runs after every executed command attempt
  (`shell_expansion::execute_prepared_outcome`'s run closure,
  `MdShellRunner::execute`), never for a cached or skipped one, and after
  every side effect the shared executor handed to the effect engine, failed or
  not (`StackExecutor::dispatch_side_effect` calls
  `LifecycleHost::side_effect_attempted`; `MdStackHost` invalidates, Claudine's
  host keeps the no-op default because its lifecycle expressions get a fresh
  runtime each). There is no end-of-event clear: a guarded-off or all-`info`
  stack keeps the memo. It swaps in empty source, bind, and contract maps (a regenerated
  imported schema must not be served stale); a generation counter keeps a
  load already in flight from filling the new maps. It never clears the read
  set.
- A body read-side call after a command is left pending by body
  interpolation and evaluated by the shell stages after the commands before it
  ran (compose.md "Reads after a command"), so it sees their invalidation; a
  structural read (page-block condition, collection source or callback) is
  decided before any body command. Tests: `lib/tests/l1/load_data_runtime.rs`.
- `read_set` is on `ComposeReport` (unioned by `merge`) and `ComposeFailure`;
  `ComposeRequest::read_set()` snapshots it.
