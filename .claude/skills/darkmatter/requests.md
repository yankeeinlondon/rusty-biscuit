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
reference check, pre-flight, lifecycle, and composition share it.

- Reads are recorded at the operation boundary, never in `resolve_arg`:
  `functions/fs.rs::load_markdown` (the five Markdown readers), `file_exists_fn`
  (probed candidates on a miss), and contract imports via
  `DataRuntime::prepare_contract`. Record after resolution, before reading, so a
  failed read stays in the set.
- `invalidate_memo()` runs after every executed command attempt
  (`shell_expansion::execute_prepared_outcome`'s run closure,
  `MdShellRunner::execute`), never for a cached or skipped one, and never
  clears the read set.
- `read_set` is on `ComposeReport` (unioned by `merge`) and `ComposeFailure`;
  `ComposeRequest::read_set()` snapshots it.
